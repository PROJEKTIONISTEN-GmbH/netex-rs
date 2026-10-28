//! What can go wrong, and where.

use std::fmt;

/// The result of an operation of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Where in a document something happened.
///
/// A reader of a line file a hundred megabytes long is owed more than "invalid":
/// this names the line, the byte offset, and the elements from the root down to
/// the one being read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// 1-based line.
    pub line: u32,
    /// 0-based byte offset into the document.
    pub offset: usize,
    /// The elements from the root down, `/`-separated: `PublicationDelivery/dataObjects/CompositeFrame`.
    pub path: String,
}

impl Location {
    /// The location `offset` bytes into `text`, under `path`.
    pub fn at(text: &str, offset: usize, path: String) -> Self {
        let offset = offset.min(text.len());
        let line = memchr::memchr_iter(b'\n', &text.as_bytes()[..offset]).count() + 1;
        Location {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            offset,
            path,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {} (byte {}) at {}",
            self.line, self.offset, self.path
        )
    }
}

/// Why a document could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    /// The document is not well-formed XML.
    Xml(String),
    /// The document is well-formed but is not a NeTEx document: its root is not
    /// `<PublicationDelivery>` in the NeTEx namespace.
    NotNetex {
        /// The root element found.
        root: String,
    },
    /// An element the crate types held character data where NeTEx has elements.
    UnexpectedText,
    /// An element the crate types held an attribute it does not model.
    ///
    /// Attributes on a typed leaf are refused rather than dropped, so that no
    /// content is lost quietly; the elements that carry attributes are typed
    /// with them.
    UnexpectedAttribute {
        /// The attribute.
        name: String,
    },
    /// An element that may occur once under its parent occurred again.
    Duplicate {
        /// The element.
        name: String,
    },
    /// A lexical value did not match the XML Schema datatype it is declared as.
    InvalidValue {
        /// The datatype, e.g. `xsd:boolean`.
        datatype: &'static str,
        /// The offending lexical form.
        value: String,
    },
    /// The document declares an entity or a DTD.
    ///
    /// Entities are never expanded and DTDs never fetched — that is what keeps
    /// an "XML bomb" from being one — so a document that relies on either cannot
    /// be read correctly and is refused.
    Entity {
        /// The entity name, or the DTD's first token.
        name: String,
    },
    /// The document is declared in an encoding this crate does not read.
    ///
    /// Documents are read as UTF-8, which is what every published NeTEx document
    /// uses; a declaration naming anything else is refused rather than decoded
    /// wrongly.
    Encoding {
        /// The declared encoding.
        declared: String,
    },
    /// The document exceeds a limit set for reading it.
    Limit(Limit),
    /// The reader could not use the input.
    Io(String),
}

/// A limit a document exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Limit {
    /// Elements nested deeper than [`Limits::max_depth`](crate::Limits::max_depth).
    Depth(usize),
    /// A run of character data longer than [`Limits::max_text`](crate::Limits::max_text) bytes.
    Text(usize),
    /// An element with more attributes than [`Limits::max_attributes`](crate::Limits::max_attributes).
    Attributes(usize),
    /// A carried-through fragment larger than [`Limits::max_fragment`](crate::Limits::max_fragment) bytes.
    Fragment(usize),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Xml(message) => write!(f, "not well-formed XML: {message}"),
            Kind::NotNetex { root } => {
                write!(
                    f,
                    "not a NeTEx document: the root element is <{root}>, not <PublicationDelivery>"
                )
            }
            Kind::UnexpectedText => f.write_str("character data where NeTEx has elements"),
            Kind::UnexpectedAttribute { name } => {
                write!(f, "attribute {name} on an element that carries none")
            }
            Kind::Duplicate { name } => write!(f, "<{name}> occurs more than once"),
            Kind::InvalidValue { datatype, value } => {
                write!(f, "{value:?} is not a valid {datatype}")
            }
            Kind::Entity { name } => write!(
                f,
                "the document declares or uses an entity ({name}); entities are not expanded"
            ),
            Kind::Encoding { declared } => {
                write!(
                    f,
                    "the document is declared as {declared}; only UTF-8 is read"
                )
            }
            Kind::Limit(limit) => match limit {
                Limit::Depth(n) => write!(f, "elements nested deeper than {n}"),
                Limit::Text(n) => write!(f, "character data longer than {n} bytes"),
                Limit::Attributes(n) => write!(f, "an element with more than {n} attributes"),
                Limit::Fragment(n) => write!(f, "a carried-through fragment larger than {n} bytes"),
            },
            Kind::Io(message) => write!(f, "cannot read the input: {message}"),
        }
    }
}

/// An error, with where in the document it arose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// What went wrong.
    pub kind: Kind,
    /// Where. `None` for errors that have no place in a document, such as an
    /// input that could not be read at all.
    pub location: Option<Location>,
    /// Which document of a dataset, when the error arose reading one.
    pub document: Option<String>,
}

impl Error {
    /// An error with no place in a document.
    pub fn new(kind: Kind) -> Self {
        Error {
            kind,
            location: None,
            document: None,
        }
    }

    /// An error at a place in a document.
    pub fn at(kind: Kind, location: Location) -> Self {
        Error {
            kind,
            location: Some(location),
            document: None,
        }
    }

    /// The same error, naming the document of a dataset it arose in.
    pub fn in_document(mut self, name: &str) -> Self {
        self.document = Some(name.to_owned());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(document) = &self.document {
            write!(f, "{document}: ")?;
        }
        match &self.location {
            Some(location) => write!(f, "{}: {}", location, self.kind),
            None => self.kind.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
