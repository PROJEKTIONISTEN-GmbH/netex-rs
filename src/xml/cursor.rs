//! A cursor over the tokens of a document.

use std::borrow::Cow;

use quick_xml::events::attributes::Attribute as RawAttribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Reader;
use quick_xml::XmlVersion;

use super::{Attribute, Limits, Unprocessed};
use crate::error::{Error, Kind, Limit, Location, Result};

/// The start of a child element, handed to whoever reads it.
#[derive(Debug)]
pub struct Child<'i> {
    /// The element's local name.
    pub local: &'i str,
    /// The element's namespace prefix, if it has one.
    pub prefix: Option<&'i str>,
    /// Whether the element is a NeTEx element: unprefixed under the default
    /// NeTEx namespace, or prefixed with the prefix bound to it.
    pub netex: bool,
    /// The start tag.
    pub start: BytesStart<'i>,
    /// Whether the start tag was an empty-element tag.
    pub empty: bool,
    /// Byte offset of the start tag's `<`.
    pub offset: usize,
    /// The attributes, parsed once from the start tag.
    attributes: Attributes<'i>,
    /// The namespace bindings the start tag declares, when it declares any
    /// that matter: what the NeTEx namespace is bound to inside this element.
    scope: Option<Scope<'i>>,
}

/// The attributes of a start tag as written — qualified name and raw value —
/// parsed once: the cursor needs them for the namespace scope, the element's
/// reader for its fields, and parsing a tag twice was a fifth of the time of
/// a typed read. A NeTEx element has four attributes at most in practice
/// (`id`, `version`, `ref`, `order`), so those are kept inline and only a tag
/// with more allocates.
#[derive(Debug, Clone)]
struct Attributes<'i> {
    inline: [(&'i str, Cow<'i, str>); Attributes::INLINE],
    len: usize,
    spilled: Vec<(&'i str, Cow<'i, str>)>,
}

impl<'i> Attributes<'i> {
    const INLINE: usize = 4;

    fn new() -> Self {
        Attributes {
            inline: std::array::from_fn(|_| ("", Cow::Borrowed(""))),
            len: 0,
            spilled: Vec::new(),
        }
    }

    fn push(&mut self, key: &'i str, raw_value: Cow<'i, str>) {
        if self.len < Self::INLINE {
            self.inline[self.len] = (key, raw_value);
        } else {
            self.spilled.push((key, raw_value));
        }
        self.len += 1;
    }

    fn iter(&self) -> impl Iterator<Item = (&'i str, &Cow<'i, str>)> + '_ {
        self.inline[..self.len.min(Self::INLINE)]
            .iter()
            .chain(&self.spilled)
            .map(|(key, raw_value)| (*key, raw_value))
    }
}

/// What the NeTEx namespace is bound to at some point of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Scope<'i> {
    /// Whether the default namespace is the NeTEx namespace.
    default_is_netex: bool,
    /// The prefix bound to the NeTEx namespace, if one is.
    netex_prefix: Option<&'i str>,
}

impl<'i> Child<'i> {
    /// The element's attributes: qualified name and normalised, unescaped value.
    ///
    /// The names borrow the document; the values are unescaped and, as XML
    /// requires of attribute values, have their line breaks and tabs folded to
    /// spaces — borrowed too, unless unescaping changed them.
    pub fn attributes(&self) -> impl Iterator<Item = Result<(&'i str, Cow<'i, str>)>> + '_ {
        self.attributes.iter().map(|(key, raw_value)| {
            // Only a reference and a line break or tab change a value; one without any is the value.
            if !raw_value
                .bytes()
                .any(|byte| matches!(byte, b'&' | b'\t' | b'\n' | b'\r'))
            {
                return Ok((key, raw_value.clone()));
            }
            let attribute = RawAttribute {
                key: QName(key),
                value: raw_value.clone(),
            };
            let value = attribute
                .normalized_value(XmlVersion::Explicit1_0)
                .map_err(|e| {
                    Error::new(Kind::Entity {
                        name: e.to_string(),
                    })
                })?;
            Ok((key, value))
        })
    }
}

/// What a leaf-or-list element holds: character data, or a first child.
// A child carries its attributes inline and is the larger variant by far;
// the value is matched on the spot and never stored, so boxing it would buy
// an allocation and nothing else.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Content<'i> {
    /// Character data, up to the end tag, which has been read.
    Text(Cow<'i, str>),
    /// The first child element; the rest follow with [`Cursor::next_child`].
    Child(Child<'i>),
}

/// A cursor over the events of one document held in memory.
///
/// Typed readers pull children from it one at a time; whatever they do not
/// read they ask it to skip, and it hands back the skipped source text.
///
/// A copy of a cursor goes on from where the cursor stands, on its own: a
/// reader that may give up on an element reads it with the cursor and, where
/// it gives up, goes on from a copy taken at the element's start tag.
#[derive(Clone)]
pub struct Cursor<'i> {
    input: &'i str,
    reader: Reader<&'i [u8]>,
    limits: Limits,
    /// The elements from the root down to the one being read.
    path: Vec<&'i str>,
    /// What the NeTEx namespace is bound to where the cursor is.
    scope: Scope<'i>,
    /// For each element on the path, the scope to restore when it is left, if
    /// the element changed it.
    scopes: Vec<Option<Scope<'i>>>,
    /// The names of the attributes of the tag being checked, kept from one
    /// tag to the next so that checking one allocates nothing.
    names: Vec<&'i str>,
}

impl std::fmt::Debug for Cursor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor")
            .field("offset", &self.reader.buffer_position())
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// The NeTEx namespace.
pub const NETEX_NAMESPACE: &str = "http://www.netex.org.uk/netex";

impl<'i> Cursor<'i> {
    /// A cursor at the start of `input`.
    pub fn new(input: &'i str, limits: Limits) -> Self {
        let input = super::without_bom(input);
        let mut reader = Reader::from_str(input);
        let config = reader.config_mut();
        config.check_end_names = true;
        config.trim_text_start = false;
        config.trim_text_end = false;
        Cursor {
            input,
            reader,
            limits,
            path: Vec::with_capacity(32),
            scope: Scope::default(),
            scopes: Vec::with_capacity(32),
            names: Vec::new(),
        }
    }

    /// The document being read.
    pub fn input(&self) -> &'i str {
        self.input
    }

    /// The limits the cursor enforces.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Byte offset of the next token.
    pub fn offset(&self) -> usize {
        usize::try_from(self.reader.buffer_position()).unwrap_or(usize::MAX)
    }

    /// Where the cursor is, for an error.
    pub fn location(&self) -> Location {
        self.location_at(self.offset())
    }

    fn location_at(&self, offset: usize) -> Location {
        Location::at(self.input, offset, self.path.join("/"))
    }

    /// An error at the cursor's position.
    pub fn error(&self, kind: Kind) -> Error {
        Error::at(kind, self.location())
    }

    /// An error at `offset`.
    pub fn error_at(&self, kind: Kind, offset: usize) -> Error {
        Error::at(kind, self.location_at(offset))
    }

    fn read_event(&mut self) -> Result<(usize, Event<'i>)> {
        let offset = self.offset();
        let event = self
            .reader
            .read_event()
            .map_err(|e| Error::at(Kind::Xml(e.to_string()), self.location_at(offset)))?;
        Ok((offset, event))
    }

    /// Reads past the prolog to the root element and returns it.
    ///
    /// The document is checked for characters XML 1.0 forbids, the XML
    /// declaration for its place and its encoding, a DTD is refused, and the
    /// root's namespace declarations decide which elements are NeTEx elements.
    pub fn root(&mut self) -> Result<Child<'i>> {
        if let Some((offset, character)) = forbidden_character(self.input) {
            return Err(self.error_at(Kind::Xml(forbidden(character)), offset));
        }
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::Decl(_) if offset > 0 => return Err(self.misplaced_declaration(offset)),
                Event::Decl(decl) => {
                    if let Some(Ok(encoding)) = decl.encoding() {
                        if !encoding.eq_ignore_ascii_case("utf-8") {
                            return Err(self.error_at(
                                Kind::Encoding {
                                    declared: encoding.into_owned(),
                                },
                                offset,
                            ));
                        }
                    }
                }
                Event::DocType(doctype) => {
                    let name = doctype.split_whitespace().next().unwrap_or("").to_owned();
                    return Err(self.error_at(Kind::Entity { name }, offset));
                }
                Event::Start(start) => return self.child(start, false, offset),
                Event::Empty(start) => return self.child(start, true, offset),
                Event::Text(text) => {
                    if !text.trim().is_empty() {
                        return Err(self.error_at(Kind::UnexpectedText, offset));
                    }
                }
                Event::GeneralRef(reference) => {
                    return Err(self.error_at(
                        Kind::Entity {
                            name: reference.as_ref().to_owned(),
                        },
                        offset,
                    ));
                }
                Event::Eof => {
                    return Err(self.error_at(
                        Kind::NotNetex {
                            root: "(no element)".to_owned(),
                            expected: None,
                        },
                        offset,
                    ));
                }
                Event::CData(_) => {
                    return Err(self.error_at(
                        Kind::Xml("character data before the root element".to_owned()),
                        offset,
                    ))
                }
                Event::End(_) => {
                    return Err(self.error_at(
                        Kind::Xml("an end tag before the root element".to_owned()),
                        offset,
                    ))
                }
                Event::Comment(_) | Event::PI(_) => {}
            }
        }
    }

    /// The error of an attribute written right after the one before.
    fn unspaced(&self, key: &str, offset: usize) -> Error {
        self.error_at(
            Kind::Xml(format!("no white space before the attribute {key}")),
            offset,
        )
    }

    /// The error of an XML declaration anywhere but at the start.
    fn misplaced_declaration(&self, offset: usize) -> Error {
        self.error_at(
            Kind::Xml("an XML declaration other than at the start of the document".to_owned()),
            offset,
        )
    }

    /// Checks the attributes of a tag the cursor skips as it checks those of
    /// a child it reads: written as XML has them, none twice, every reference
    /// in a value one the cursor resolves.
    fn check_tag(&mut self, start: &BytesStart<'i>, offset: usize) -> Result<()> {
        self.names.clear();
        let mut parsed = start.attributes();
        parsed.with_checks(false);
        for attribute in parsed {
            let attribute =
                attribute.map_err(|e| self.error_at(Kind::Xml(e.to_string()), offset))?;
            let key = self.reborrow(attribute.key.as_ref());
            if !spaced(start, key) {
                return Err(self.unspaced(key, offset));
            }
            if self.names.contains(&key) {
                return Err(self.error_at(
                    Kind::Xml(format!("the attribute {key} occurs twice")),
                    offset,
                ));
            }
            self.names.push(key);
            check_value(&attribute.value).map_err(|kind| self.error_at(kind, offset))?;
        }
        Ok(())
    }

    fn child(&mut self, start: BytesStart<'i>, empty: bool, offset: usize) -> Result<Child<'i>> {
        let name = start.name();
        let (local, prefix) = name.decompose();
        // Borrow the names from the input rather than from the event, so they
        // outlive it: the event's buffer is the input itself.
        let local = self.reborrow(local.as_ref());
        let prefix = prefix.map(|p| self.reborrow(p.as_ref()));

        // Parse the attributes once, keeping name and raw value, and notice
        // whether the element rebinds the NeTEx namespace: on the root it
        // always does; below it hardly ever.
        let mut attributes = Attributes::new();
        let mut scope: Option<Scope<'i>> = None;
        // An attribute stated twice is refused here, against the attributes kept so far, rather than
        // by quick-xml's own check, which keeps the names of every tag's attributes in a list of its
        // own — an allocation for every element with an attribute.
        let mut parsed = start.attributes();
        parsed.with_checks(false);
        for attribute in parsed {
            let attribute =
                attribute.map_err(|e| self.error_at(Kind::Xml(e.to_string()), offset))?;
            let key = self.reborrow(attribute.key.as_ref());
            if !spaced(&start, key) {
                return Err(self.unspaced(key, offset));
            }
            if let Some((earlier, _)) = attributes.iter().find(|(earlier, _)| *earlier == key) {
                let tag = start.as_ref().as_ptr() as usize;
                let at = |name: &str| name.as_ptr() as usize - tag;
                return Err(self.error_at(
                    Kind::Xml(format!(
                        "position {}: duplicated attribute, previous declaration at position {}",
                        at(key),
                        at(earlier)
                    )),
                    offset,
                ));
            }
            check_value(&attribute.value).map_err(|kind| self.error_at(kind, offset))?;
            // A value taken from the tag borrows the input, like a name.
            let raw_value: Cow<'i, str> = match attribute.value {
                Cow::Borrowed(value) => Cow::Borrowed(self.reborrow(value)),
                Cow::Owned(value) => Cow::Owned(value),
            };
            if key == "xmlns" {
                let inner = scope.get_or_insert(self.scope);
                inner.default_is_netex = raw_value == NETEX_NAMESPACE;
            } else if let Some(bound) = key.strip_prefix("xmlns:") {
                let inner = scope.get_or_insert(self.scope);
                if raw_value == NETEX_NAMESPACE {
                    inner.netex_prefix = Some(bound);
                } else if inner.netex_prefix == Some(bound) {
                    inner.netex_prefix = None;
                }
            }
            attributes.push(key, raw_value);
            if attributes.len > self.limits.max_attributes {
                return Err(self.error_at(
                    Kind::Limit(Limit::Attributes(self.limits.max_attributes)),
                    offset,
                ));
            }
        }
        let effective = scope.unwrap_or(self.scope);
        let netex = match prefix {
            None => effective.default_is_netex,
            Some(prefix) => effective.netex_prefix == Some(prefix),
        };
        Ok(Child {
            local,
            prefix,
            netex,
            start,
            empty,
            offset,
            attributes,
            scope,
        })
    }

    /// The same text, as a slice of the input.
    ///
    /// Names taken from an event borrow the event; the event borrows the input.
    /// Re-slicing the input at the same place gives the name the input's
    /// lifetime, which the path stack needs.
    fn reborrow(&self, text: &str) -> &'i str {
        let base = self.input.as_ptr() as usize;
        let here = text.as_ptr() as usize;
        debug_assert!(here >= base && here + text.len() <= base + self.input.len());
        let at = here - base;
        &self.input[at..at + text.len()]
    }

    /// Enters an element: its children are what the cursor reads next.
    pub fn enter(&mut self, child: &Child<'i>) -> Result<()> {
        if self.path.len() >= self.limits.max_depth {
            return Err(self.error_at(
                Kind::Limit(Limit::Depth(self.limits.max_depth)),
                child.offset,
            ));
        }
        self.path.push(child.local);
        match child.scope {
            Some(scope) => {
                self.scopes
                    .push(Some(std::mem::replace(&mut self.scope, scope)));
            }
            None => self.scopes.push(None),
        }
        Ok(())
    }

    /// Leaves the element entered last.
    pub fn leave(&mut self) {
        self.path.pop();
        if let Some(Some(scope)) = self.scopes.pop() {
            self.scope = scope;
        }
    }

    /// The next child of the element being read, or `None` at its end.
    ///
    /// Whitespace between children is skipped; other character data is an
    /// error, since no typed NeTEx element has mixed content.
    pub fn next_child(&mut self) -> Result<Option<Child<'i>>> {
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::Start(start) => return self.child(start, false, offset).map(Some),
                Event::Empty(start) => return self.child(start, true, offset).map(Some),
                Event::End(_) => return Ok(None),
                Event::Text(text) => {
                    if !text.trim().is_empty() {
                        return Err(self.error_at(Kind::UnexpectedText, offset));
                    }
                }
                Event::CData(_) => return Err(self.error_at(Kind::UnexpectedText, offset)),
                Event::GeneralRef(reference) => {
                    return Err(self.error_at(
                        Kind::Entity {
                            name: reference.as_ref().to_owned(),
                        },
                        offset,
                    ))
                }
                Event::Eof => {
                    return Err(
                        self.error_at(Kind::Xml("unexpected end of document".to_owned()), offset)
                    )
                }
                Event::Decl(_) => return Err(self.misplaced_declaration(offset)),
                Event::Comment(_) | Event::PI(_) => {}
                Event::DocType(doctype) => {
                    let name = doctype.split_whitespace().next().unwrap_or("").to_owned();
                    return Err(self.error_at(Kind::Entity { name }, offset));
                }
            }
        }
    }

    /// Reads the character data of a leaf element, up to its end tag.
    ///
    /// Entity and character references are resolved — the five predefined
    /// entities and numeric references only. A child element is an error.
    pub fn text(&mut self, child: &Child<'i>) -> Result<Cow<'i, str>> {
        if child.empty {
            return Ok(Cow::Borrowed(""));
        }
        let mut gathered: Option<Cow<'i, str>> = None;
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::End(_) => return Ok(gathered.unwrap_or(Cow::Borrowed(""))),
                Event::Text(text) => {
                    let piece = text.xml10_content();
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::CData(data) => {
                    let piece = data.xml10_content();
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::GeneralRef(reference) => {
                    let resolved = self.resolve_reference(&reference, offset)?;
                    let mut buffer = [0u8; 4];
                    let piece = Cow::Owned(resolved.encode_utf8(&mut buffer).to_owned());
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::Start(_) | Event::Empty(_) => {
                    return Err(self.error_at(
                        Kind::Xml(format!(
                            "<{}> holds an element where character data was expected",
                            child.local
                        )),
                        offset,
                    ))
                }
                Event::Comment(_) | Event::PI(_) => {}
                Event::Decl(_) => return Err(self.misplaced_declaration(offset)),
                Event::DocType(_) | Event::Eof => {
                    return Err(
                        self.error_at(Kind::Xml("unexpected end of document".to_owned()), offset)
                    )
                }
            }
        }
    }

    /// Reads the character data of a leaf element as a typed leaf is read: an
    /// attribute on it is refused, but for a namespace declaration, which
    /// carries no content.
    pub fn leaf_text(&mut self, child: &Child<'i>) -> Result<Cow<'i, str>> {
        for attribute in child.attributes() {
            let (name, _) = attribute.map_err(|e| self.error_at(e.kind, child.offset))?;
            if name == "xmlns" || name.starts_with("xmlns:") {
                continue;
            }
            return Err(self.error_at(
                Kind::UnexpectedAttribute {
                    name: name.to_owned(),
                },
                child.offset,
            ));
        }
        self.text(child)
    }

    /// Reads what an element holds when it may hold either character data or
    /// child elements — a `MultilingualString`, which is a text or a list of
    /// translations.
    pub fn content(&mut self, child: &Child<'i>) -> Result<Content<'i>> {
        if child.empty {
            return Ok(Content::Text(Cow::Borrowed("")));
        }
        let mut gathered: Option<Cow<'i, str>> = None;
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::End(_) => return Ok(Content::Text(gathered.unwrap_or(Cow::Borrowed("")))),
                Event::Start(start) | Event::Empty(start) => {
                    if gathered
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty())
                    {
                        return Err(self.error_at(Kind::UnexpectedText, offset));
                    }
                    let empty = matches!(self.input.as_bytes().get(offset..self.offset()), Some(tag) if tag.ends_with(b"/>"));
                    return self.child(start, empty, offset).map(Content::Child);
                }
                Event::Text(text) => {
                    let piece = text.xml10_content();
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::CData(data) => {
                    let piece = data.xml10_content();
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::GeneralRef(reference) => {
                    let resolved = self.resolve_reference(&reference, offset)?;
                    let mut buffer = [0u8; 4];
                    let piece = Cow::Owned(resolved.encode_utf8(&mut buffer).to_owned());
                    self.gather(&mut gathered, piece, offset)?;
                }
                Event::Comment(_) | Event::PI(_) => {}
                Event::Decl(_) => return Err(self.misplaced_declaration(offset)),
                Event::DocType(_) | Event::Eof => {
                    return Err(
                        self.error_at(Kind::Xml("unexpected end of document".to_owned()), offset)
                    )
                }
            }
        }
    }

    /// The character a predefined entity or a character reference denotes.
    fn resolve_reference(
        &self,
        reference: &quick_xml::events::BytesRef<'_>,
        offset: usize,
    ) -> Result<char> {
        if let Ok(Some(character)) = reference.resolve_char_ref() {
            return if is_xml_char(character) {
                Ok(character)
            } else {
                Err(self.error_at(Kind::Xml(forbidden(character)), offset))
            };
        }
        match reference.as_ref() {
            "amp" => Ok('&'),
            "lt" => Ok('<'),
            "gt" => Ok('>'),
            "quot" => Ok('"'),
            "apos" => Ok('\''),
            other => Err(self.error_at(
                Kind::Entity {
                    name: other.to_owned(),
                },
                offset,
            )),
        }
    }

    fn gather(
        &mut self,
        gathered: &mut Option<Cow<'i, str>>,
        piece: Cow<'i, str>,
        offset: usize,
    ) -> Result<()> {
        let length = gathered.as_ref().map_or(0, |g| g.len()) + piece.len();
        if length > self.limits.max_text {
            return Err(self.error_at(Kind::Limit(Limit::Text(self.limits.max_text)), offset));
        }
        match gathered {
            None => *gathered = Some(piece),
            Some(existing) => existing.to_mut().push_str(&piece),
        }
        Ok(())
    }

    /// Skips an element the caller does not read, returning its source text.
    pub fn skip(&mut self, child: &Child<'i>, position: u32) -> Result<Unprocessed> {
        let end = if child.empty {
            self.offset()
        } else {
            self.skip_to_end(child)?
        };
        let length = end - child.offset;
        if length > self.limits.max_fragment {
            return Err(self.error_at(
                Kind::Limit(Limit::Fragment(self.limits.max_fragment)),
                child.offset,
            ));
        }
        Ok(Unprocessed {
            name: child.local.into(),
            position,
            xml: self.input[child.offset..end].into(),
        })
    }

    /// Skips an element the caller does not want, keeping nothing of it.
    ///
    /// The element is tokenized — it must still be well-formed, within the
    /// depth limit, and free of entities the cursor does not resolve — but
    /// nothing is copied; this is what the lean mode does with every element
    /// outside its selection.
    pub fn skip_over(&mut self, child: &Child<'i>) -> Result<()> {
        if !child.empty {
            self.skip_to_end(child)?;
        }
        Ok(())
    }

    /// Reads to the end tag of `child`, whose start tag has been read, and
    /// returns the offset just past it.
    fn skip_to_end(&mut self, child: &Child<'i>) -> Result<usize> {
        let base = self.path.len();
        let mut depth = 1usize;
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::Start(start) => {
                    depth += 1;
                    if base + depth > self.limits.max_depth {
                        return Err(
                            self.error_at(Kind::Limit(Limit::Depth(self.limits.max_depth)), offset)
                        );
                    }
                    self.check_tag(&start, offset)?;
                }
                Event::Empty(start) => self.check_tag(&start, offset)?,
                Event::End(_) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(self.offset());
                    }
                }
                Event::GeneralRef(reference) => {
                    self.resolve_reference(&reference, offset)?;
                }
                Event::Eof => {
                    return Err(self.error_at(
                        Kind::Xml(format!("<{}> is never closed", child.local)),
                        offset,
                    ))
                }
                Event::DocType(doctype) => {
                    let name = doctype.split_whitespace().next().unwrap_or("").to_owned();
                    return Err(self.error_at(Kind::Entity { name }, offset));
                }
                Event::Decl(_) => return Err(self.misplaced_declaration(offset)),
                Event::Text(_) | Event::CData(_) | Event::Comment(_) | Event::PI(_) => {}
            }
        }
    }

    /// Reads the rest of the document after the root element: only whitespace,
    /// comments and processing instructions may follow.
    pub fn finish(&mut self) -> Result<()> {
        loop {
            let (offset, event) = self.read_event()?;
            match event {
                Event::Eof => return Ok(()),
                Event::Text(text) if text.trim().is_empty() => {}
                Event::Comment(_) | Event::PI(_) => {}
                _ => {
                    return Err(self.error_at(
                        Kind::Xml("content after the root element".to_owned()),
                        offset,
                    ))
                }
            }
        }
    }

    /// The attributes of `child` the caller did not read, kept as written.
    pub fn extra_attributes(&self, child: &Child<'i>, known: &[&str]) -> Result<Vec<Attribute>> {
        let mut extra = Vec::new();
        for attribute in child.attributes() {
            let (key, value) = attribute.map_err(|e| self.error_at(e.kind, child.offset))?;
            if known.contains(&key) {
                continue;
            }
            extra.push(Attribute {
                name: key.into(),
                value: value.into_owned().into(),
            });
        }
        Ok(extra)
    }
}

/// Whether XML 1.0 allows `character` in a document: tab, line feed, carriage
/// return, and every character from the space on but U+FFFE and U+FFFF.
pub(crate) fn is_xml_char(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\r')
        || (character >= ' ' && !matches!(character, '\u{fffe}' | '\u{ffff}'))
}

/// The first character of `text` XML 1.0 forbids, and where it stands.
///
/// Every byte of a document is looked at once, in blocks a vector unit takes
/// whole; only a block holding a byte below the space or the lead byte of
/// U+FFFE and U+FFFF is looked into.
fn forbidden_character(text: &str) -> Option<(usize, char)> {
    const BLOCK: usize = 64;
    let bytes = text.as_bytes();
    let suspect = |block: &[u8]| {
        block
            .iter()
            .fold(false, |any, &byte| any | (byte < 0x20) | (byte == 0xef))
    };
    for (number, block) in bytes.chunks(BLOCK).enumerate() {
        if !suspect(block) {
            continue;
        }
        // A character may begin in the block before; start at a boundary.
        let mut start = number * BLOCK;
        while !text.is_char_boundary(start) {
            start -= 1;
        }
        let end = ((number + 1) * BLOCK).min(bytes.len());
        if let Some((at, character)) = text[start..]
            .char_indices()
            .take_while(|(at, _)| start + at < end)
            .find(|&(_, character)| !is_xml_char(character))
        {
            return Some((start + at, character));
        }
    }
    None
}

/// The message of a character XML 1.0 forbids.
fn forbidden(character: char) -> String {
    format!(
        "U+{:04X} is a character XML 1.0 forbids",
        u32::from(character)
    )
}

/// Checks an attribute's value as written: no `<`, and every reference one of
/// the five entities XML predefines or a character reference to a character
/// XML allows.
fn check_value(raw: &str) -> std::result::Result<(), Kind> {
    if raw.contains('<') {
        return Err(Kind::Xml("a `<` in an attribute value".to_owned()));
    }
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        let after = &rest[at + 1..];
        let Some(end) = after.find(';') else {
            return Err(Kind::Xml(
                "a `&` in an attribute value that begins no reference".to_owned(),
            ));
        };
        let name = &after[..end];
        let character = match name.strip_prefix('#') {
            Some(number) => {
                let code = match number.strip_prefix('x') {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse().ok(),
                };
                let Some(character) = code.and_then(char::from_u32) else {
                    return Err(Kind::Xml(format!("&{name}; is no character reference")));
                };
                Some(character)
            }
            None => None,
        };
        match character {
            Some(character) if !is_xml_char(character) => {
                return Err(Kind::Xml(forbidden(character)))
            }
            Some(_) => {}
            None if matches!(name, "amp" | "lt" | "gt" | "quot" | "apos") => {}
            None => {
                return Err(Kind::Entity {
                    name: name.to_owned(),
                })
            }
        }
        rest = &after[end + 1..];
    }
    Ok(())
}

/// Whether white space stands before the attribute named `key` in `start`,
/// as XML has it between a tag's name and attributes: `key` a slice of the
/// tag.
fn spaced(start: &BytesStart<'_>, key: &str) -> bool {
    let tag = start.as_ref().as_bytes();
    (key.as_ptr() as usize)
        .checked_sub(tag.as_ptr() as usize + 1)
        .and_then(|before| tag.get(before))
        .is_some_and(u8::is_ascii_whitespace)
}
