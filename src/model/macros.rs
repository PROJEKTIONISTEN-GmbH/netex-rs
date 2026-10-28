//! The declaration macros the typed model is written in.

/// Declares a typed element: a struct with named attributes and children in
/// schema order, plus what the declaration does not name, carried through.
///
/// Attributes are read into their fields by name and every other attribute is
/// kept in `extra_attributes`. Children are matched by local name in the NeTEx
/// namespace, `= "Name"` for a single name and `= choice` for a field whose
/// type lists its own names; every other child is kept in `unprocessed` at the
/// position it was read.
macro_rules! netex_element {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            attributes { $($(#[$attr_meta:meta])* pub $attr:ident : Option<$attr_ty:ty> = $attr_name:literal),* $(,)? }
            children {
                $(
                    $(#[$child_meta:meta])*
                    pub $child:ident : $child_ty:ty = $child_name:tt
                ),* $(,)?
            }
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct $name {
            $(
                $(#[$attr_meta])*
                pub $attr: Option<$attr_ty>,
            )*
            $(
                $(#[$child_meta])*
                pub $child: $child_ty,
            )*
            /// Attributes the declaration does not name, kept as written.
            pub extra_attributes: Vec<$crate::xml::Attribute>,
            /// Children the declaration does not name, kept as written, at the
            /// positions they were read.
            pub unprocessed: Vec<$crate::xml::Unprocessed>,
        }

        impl $crate::model::Element for $name {
            fn read<'i>(
                cursor: &mut $crate::xml::Cursor<'i>,
                child: &$crate::xml::Child<'i>,
            ) -> $crate::error::Result<Self> {
                let mut this = Self::default();
                for attribute in child.attributes() {
                    let (key, value) = attribute.map_err(|e| cursor.error_at(e.kind, child.offset))?;
                    match key {
                        $(
                            $attr_name => {
                                this.$attr = Some(
                                    <$attr_ty as $crate::model::AttributeValue>::from_text(&value)
                                        .ok_or_else(|| cursor.error_at(
                                            $crate::error::Kind::InvalidValue {
                                                datatype: <$attr_ty as $crate::model::AttributeValue>::DATATYPE,
                                                value: value.to_string(),
                                            },
                                            child.offset,
                                        ))?,
                                );
                            }
                        )*
                        other => this.extra_attributes.push($crate::xml::Attribute {
                            name: other.into(),
                            value: value.into_owned().into(),
                        }),
                    }
                }
                cursor.enter(child)?;
                if !child.empty {
                    let mut position = 0u32;
                    while let Some(next) = cursor.next_child()? {
                        netex_element!(@dispatch this, cursor, next, position; $($child : $child_ty = $child_name),*);
                        position += 1;
                    }
                }
                cursor.leave();
                Ok(this)
            }

            fn write(&self, name: &str, out: &mut $crate::xml::Emitter) {
                let candidates: Vec<Option<(&str, std::borrow::Cow<'_, str>)>> = Vec::from([
                    $(
                        self.$attr.as_ref().map(|value| {
                            ($attr_name, <$attr_ty as $crate::model::AttributeValue>::to_text(value))
                        }),
                    )*
                ]);
                let attributes: Vec<(&str, std::borrow::Cow<'_, str>)> =
                    candidates.into_iter().flatten().collect();
                let attributes: Vec<(&str, &str)> =
                    attributes.iter().map(|(key, value)| (*key, value.as_ref())).collect();
                let has_children = !self.unprocessed.is_empty()
                    $(|| netex_element!(@is_set self.$child, $child_name))*;
                if !has_children {
                    out.leaf(name, &attributes, &self.extra_attributes, "");
                    return;
                }
                out.start(name, &attributes, &self.extra_attributes);
                let mut merge = $crate::model::Merge::new(&self.unprocessed);
                $(
                    netex_element!(@write self.$child, $child_name, out, merge);
                )*
                merge.finish(out);
                out.end(name);
            }
        }

        impl $crate::model::Walk for $name {
            fn walk_unprocessed(
                &self,
                parent: &str,
                visit: &mut dyn FnMut(&str, &$crate::xml::Unprocessed),
            ) {
                for fragment in &self.unprocessed {
                    visit(parent, fragment);
                }
                $(
                    netex_element!(@walk self.$child, $child_name, visit);
                )*
            }
        }
    };

    (@walk $field:expr, choice, $visit:ident) => {
        $crate::model::Walk::walk_unprocessed(&$field, "", $visit)
    };
    (@walk $field:expr, $child_name:literal, $visit:ident) => {
        $crate::model::Walk::walk_unprocessed(&$field, $child_name, $visit)
    };

    (@dispatch $this:ident, $cursor:ident, $next:ident, $position:ident; $($child:ident : $child_ty:ty = $child_name:tt),*) => {
        if !$next.netex {
            $this.unprocessed.push($cursor.skip(&$next, $position)?);
        } $(else if netex_element!(@matches $child_ty, $next, $child_name) {
            netex_element!(@accept $this.$child, $cursor, $next, $child_name);
        })* else {
            $this.unprocessed.push($cursor.skip(&$next, $position)?);
        }
    };

    (@matches $child_ty:ty, $next:ident, choice) => {
        <<$child_ty as $crate::model::ChoiceSlot>::Item as $crate::model::Choice>::NAMES.contains(&$next.local)
    };
    (@matches $child_ty:ty, $next:ident, $child_name:literal) => {
        $next.local == $child_name
    };

    (@accept $field:expr, $cursor:ident, $next:ident, choice) => {
        $crate::model::ChoiceSlot::accept(&mut $field, $cursor, &$next)?
    };
    (@accept $field:expr, $cursor:ident, $next:ident, $child_name:literal) => {
        $crate::model::Slot::accept(&mut $field, $cursor, &$next)?
    };

    (@is_set $field:expr, choice) => {
        $crate::model::ChoiceSlot::is_set(&$field)
    };
    (@is_set $field:expr, $child_name:literal) => {
        $crate::model::Slot::is_set(&$field)
    };

    (@write $field:expr, choice, $out:ident, $merge:ident) => {
        $crate::model::ChoiceSlot::write_into(&$field, $out, &mut $merge)
    };
    (@write $field:expr, $child_name:literal, $out:ident, $merge:ident) => {
        $crate::model::Slot::write_into(&$field, $child_name, $out, &mut $merge)
    };
}

/// Declares a choice: an enum whose variants are the elements that may stand
/// at one position, each under its own name.
macro_rules! netex_choice {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident($ty:ty) = $element:literal
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant($ty),
            )*
        }

        impl $crate::model::Choice for $name {
            const NAMES: &'static [&'static str] = &[$($element),*];

            fn read<'i>(
                cursor: &mut $crate::xml::Cursor<'i>,
                child: &$crate::xml::Child<'i>,
            ) -> $crate::error::Result<Self> {
                match child.local {
                    $($element => Ok(Self::$variant(<$ty as $crate::model::Element>::read(cursor, child)?)),)*
                    other => Err(cursor.error_at(
                        $crate::error::Kind::Xml(format!("<{other}> is not one of {:?}", Self::NAMES)),
                        child.offset,
                    )),
                }
            }

            fn write(&self, out: &mut $crate::xml::Emitter) {
                match self {
                    $(Self::$variant(value) => $crate::model::Element::write(value, $element, out),)*
                }
            }
        }

        impl $crate::model::Walk for $name {
            fn walk_unprocessed(
                &self,
                _parent: &str,
                visit: &mut dyn FnMut(&str, &$crate::xml::Unprocessed),
            ) {
                match self {
                    $(Self::$variant(value) => $crate::model::Walk::walk_unprocessed(value, $element, visit),)*
                }
            }
        }
    };
}

/// Declares a container: an element holding a list of entities of a few
/// kinds, in document order, with any other child carried through in place.
///
/// The entities are boxed: an entity is hundreds of bytes and a carried-through
/// fragment forty, and a list of the one should not be sized for the other.
macro_rules! netex_container {
    (
        $(#[$meta:meta])*
        pub struct $name:ident of $item:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident($ty:ty) = $element:literal
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct $name {
            /// The entities, in document order.
            pub items: Vec<$item>,
            /// Attributes the declaration does not name, kept as written.
            pub extra_attributes: Vec<$crate::xml::Attribute>,
        }

        /// One entity of the container.
        #[derive(Debug, Clone, PartialEq)]
        pub enum $item {
            $(
                $(#[$variant_meta])*
                $variant(Box<$ty>),
            )*
            /// An entity the crate does not type, kept as written.
            Unprocessed($crate::xml::Unprocessed),
        }

        impl $crate::model::Element for $name {
            fn read<'i>(
                cursor: &mut $crate::xml::Cursor<'i>,
                child: &$crate::xml::Child<'i>,
            ) -> $crate::error::Result<Self> {
                let mut this = Self { items: Vec::new(), extra_attributes: cursor.extra_attributes(child, &[])? };
                cursor.enter(child)?;
                if !child.empty {
                    let mut position = 0u32;
                    while let Some(next) = cursor.next_child()? {
                        let item = if !next.netex {
                            $item::Unprocessed(cursor.skip(&next, position)?)
                        } else {
                            match next.local {
                                $($element => $item::$variant(Box::new(<$ty as $crate::model::Element>::read(cursor, &next)?)),)*
                                _ => $item::Unprocessed(cursor.skip(&next, position)?),
                            }
                        };
                        this.items.push(item);
                        position += 1;
                    }
                }
                cursor.leave();
                Ok(this)
            }

            fn write(&self, name: &str, out: &mut $crate::xml::Emitter) {
                if self.items.is_empty() {
                    out.leaf(name, &[], &self.extra_attributes, "");
                    return;
                }
                out.start(name, &[], &self.extra_attributes);
                for item in &self.items {
                    match item {
                        $($item::$variant(value) => $crate::model::Element::write(&**value, $element, out),)*
                        $item::Unprocessed(fragment) => out.raw(fragment),
                    }
                }
                out.end(name);
            }
        }

        impl $crate::model::Walk for $name {
            fn walk_unprocessed(
                &self,
                parent: &str,
                visit: &mut dyn FnMut(&str, &$crate::xml::Unprocessed),
            ) {
                for item in &self.items {
                    match item {
                        $($item::$variant(value) => $crate::model::Walk::walk_unprocessed(&**value, $element, visit),)*
                        $item::Unprocessed(fragment) => visit(parent, fragment),
                    }
                }
            }
        }
    };
}

/// Declares an enumeration: the tokens of an XML Schema `simpleType`, in schema
/// order, plus one variant that keeps any token the schemas do not list.
macro_rules! netex_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident as $xsd:literal {
            $($(#[$variant_meta:meta])* $variant:ident = $token:literal),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $(
                #[doc = concat!("The token `", $token, "`.")]
                $(#[$variant_meta])*
                $variant,
            )*
            /// A token the schemas this crate transcribes do not list.
            ///
            /// National profiles and later releases extend these lists. The token
            /// is kept as it was read and written back unchanged, so that one
            /// value the crate does not know does not cost the document that
            /// carries it; what to make of it is the application's decision.
            Unrecognised(Box<str>),
        }

        impl $name {
            /// Name of the XML Schema type this enumeration transcribes.
            pub const XSD_TYPE: &'static str = $xsd;

            /// Every token the schemas define, in schema order.
            pub const TOKENS: &'static [&'static str] = &[$($token),*];

            /// The value a token denotes.
            pub fn from_token(token: &str) -> Self {
                match token {
                    $($token => Self::$variant,)*
                    other => Self::Unrecognised(other.into()),
                }
            }

            /// The token the value is written as.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $token,)*
                    Self::Unrecognised(token) => token,
                }
            }
        }

        impl $crate::model::Leaf for $name {
            const DATATYPE: &'static str = $xsd;

            fn from_text(text: &str) -> Option<Self> {
                Some(Self::from_token(text))
            }

            fn to_text(&self) -> std::borrow::Cow<'_, str> {
                std::borrow::Cow::Borrowed(self.as_str())
            }
        }

        impl $crate::model::Element for $name {
            fn read<'i>(
                cursor: &mut $crate::xml::Cursor<'i>,
                child: &$crate::xml::Child<'i>,
            ) -> $crate::error::Result<Self> {
                $crate::model::read_leaf(cursor, child)
            }

            fn write(&self, name: &str, out: &mut $crate::xml::Emitter) {
                out.leaf(name, &[], &[], self.as_str());
            }
        }

        impl $crate::model::Walk for $name {
            fn walk_unprocessed(&self, _parent: &str, _visit: &mut dyn FnMut(&str, &$crate::xml::Unprocessed)) {}
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}
