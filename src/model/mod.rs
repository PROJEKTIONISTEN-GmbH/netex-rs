//! The typed model: the parts of NeTEx the crate reads into Rust types.
//!
//! Every type here is declared with the element's children in the order the
//! schema gives them (drafted from the schemas with `tools/schema_table.py
//! --declare`). A child the declaration does not name is not lost: it is kept
//! as an [`Unprocessed`] fragment at the position it was read, and written
//! back there. An attribute the declaration does not name is kept the same way.
//!
//! The rule for what is typed: the framework, the parts of Part 1 (network,
//! stops) and Part 2 (timetable) a routing engine reads, and everything the
//! German national delivery uses. Fares (Part 3) and new modes (Part 5) are
//! carried through untyped.

#[macro_use]
mod macros;
mod calendar;
mod enumerations;
mod framework;
mod publication;
mod resource;
mod service;
mod site;
mod timetable;
mod types;

use std::borrow::Cow;

pub use calendar::*;
pub use enumerations::*;
pub use framework::*;
pub use publication::*;
pub use resource::*;
pub use service::*;
pub use site::*;
pub use timetable::*;
pub use types::*;

use crate::error::{Kind, Result};
use crate::xml::{Child, Cursor, Emitter, Unprocessed};

/// A type that is read from one element and written as one.
///
/// The element's name is the caller's: the same type is written as `<Name>`
/// and `<ShortName>`, and a reference type as `<LineRef>` and `<OperatorRef>`.
pub trait Element: Sized + std::fmt::Debug {
    /// Reads the element whose start tag is `child`, through to its end tag.
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self>;

    /// Writes the element as `<name>`.
    fn write(&self, name: &str, out: &mut Emitter);
}

impl<T: Element> Element for Box<T> {
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self> {
        T::read(cursor, child).map(Box::new)
    }

    fn write(&self, name: &str, out: &mut Emitter) {
        T::write(self, name, out);
    }
}

impl<T: Walk> Walk for Box<T> {
    fn walk_unprocessed(&self, parent: &str, visit: &mut dyn FnMut(&str, &Unprocessed)) {
        T::walk_unprocessed(self, parent, visit);
    }
}

/// An element with character data and no children or attributes.
pub trait Leaf: Sized + std::fmt::Debug {
    /// The XML Schema datatype the value is read as, for errors.
    const DATATYPE: &'static str;

    /// The value a lexical form denotes, or `None` if it is not one.
    fn from_text(text: &str) -> Option<Self>;

    /// The lexical form the value is written as.
    fn to_text(&self) -> Cow<'_, str>;
}

/// Reads a leaf element into a [`Leaf`] type.
pub(crate) fn read_leaf<'i, T: Leaf>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<T> {
    let text = cursor.leaf_text(child)?;
    T::from_text(&text).ok_or_else(|| {
        cursor.error_at(
            Kind::InvalidValue {
                datatype: T::DATATYPE,
                value: text.into_owned(),
            },
            child.offset,
        )
    })
}

/// Implements [`Element`] and [`Walk`] for a [`Leaf`] type.
macro_rules! leaf_element {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Element for $ty {
                fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self> {
                    read_leaf(cursor, child)
                }

                fn write(&self, name: &str, out: &mut Emitter) {
                    out.leaf(name, &[], &[], &Leaf::to_text(self));
                }
            }

            impl Walk for $ty {
                fn walk_unprocessed(&self, _parent: &str, _visit: &mut dyn FnMut(&str, &Unprocessed)) {}
            }
        )*
    };
}

leaf_element!(
    Box<str>,
    bool,
    u32,
    i64,
    f64,
    DateTime,
    Date,
    Time,
    Duration
);

/// Walks what a typed value carries through untyped.
///
/// Every element and attribute the model does not name is kept, not lost; this
/// is how to see how much of a document that is. The inventory of the German
/// delivery counts with it.
pub trait Walk {
    /// Calls `visit` with the parent element's name and each fragment carried
    /// through under it, at any depth.
    fn walk_unprocessed(&self, parent: &str, visit: &mut dyn FnMut(&str, &Unprocessed));
}

impl<T: Walk> Walk for Option<T> {
    fn walk_unprocessed(&self, parent: &str, visit: &mut dyn FnMut(&str, &Unprocessed)) {
        if let Some(value) = self {
            value.walk_unprocessed(parent, visit);
        }
    }
}

impl<T: Walk> Walk for Vec<T> {
    fn walk_unprocessed(&self, parent: &str, visit: &mut dyn FnMut(&str, &Unprocessed)) {
        for value in self {
            value.walk_unprocessed(parent, visit);
        }
    }
}

/// A choice between elements of different names at one position.
pub trait Choice: Sized + std::fmt::Debug {
    /// The element names the choice accepts.
    const NAMES: &'static [&'static str];

    /// Reads whichever alternative `child` is.
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self>;

    /// Writes the alternative held, under its own name.
    fn write(&self, out: &mut Emitter);
}

/// A field of a typed element: how it takes a child and writes it back.
pub trait Slot: Default {
    /// The element type held.
    type Item;

    /// Takes a child that belongs to this field.
    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()>;

    /// Whether the field holds anything.
    fn is_set(&self) -> bool;

    /// Writes what the field holds, each item as one child.
    fn write_into(&self, name: &str, out: &mut Emitter, merge: &mut Merge<'_>);
}

impl<T: Element> Slot for Option<T> {
    type Item = T;

    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()> {
        if self.is_some() {
            return Err(cursor.error_at(
                Kind::Duplicate {
                    name: child.local.to_owned(),
                },
                child.offset,
            ));
        }
        *self = Some(T::read(cursor, child)?);
        Ok(())
    }

    fn is_set(&self) -> bool {
        self.is_some()
    }

    fn write_into(&self, name: &str, out: &mut Emitter, merge: &mut Merge<'_>) {
        if let Some(value) = self {
            merge.before_child(out);
            value.write(name, out);
        }
    }
}

impl<T: Element> Slot for Vec<T> {
    type Item = T;

    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()> {
        self.push(T::read(cursor, child)?);
        Ok(())
    }

    fn is_set(&self) -> bool {
        !self.is_empty()
    }

    fn write_into(&self, name: &str, out: &mut Emitter, merge: &mut Merge<'_>) {
        for value in self {
            merge.before_child(out);
            value.write(name, out);
        }
    }
}

/// A choice field: like [`Slot`], for alternatives that carry their own names.
pub trait ChoiceSlot: Default {
    /// The choice type held.
    type Item: Choice;

    /// Takes a child that belongs to this field.
    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()>;

    /// Whether the field holds anything.
    fn is_set(&self) -> bool;

    /// Writes what the field holds.
    fn write_into(&self, out: &mut Emitter, merge: &mut Merge<'_>);
}

impl<T: Choice> ChoiceSlot for Option<T> {
    type Item = T;

    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()> {
        if self.is_some() {
            return Err(cursor.error_at(
                Kind::Duplicate {
                    name: child.local.to_owned(),
                },
                child.offset,
            ));
        }
        *self = Some(T::read(cursor, child)?);
        Ok(())
    }

    fn is_set(&self) -> bool {
        self.is_some()
    }

    fn write_into(&self, out: &mut Emitter, merge: &mut Merge<'_>) {
        if let Some(value) = self {
            merge.before_child(out);
            value.write(out);
        }
    }
}

impl<T: Choice> ChoiceSlot for Vec<T> {
    type Item = T;

    fn accept<'i>(&mut self, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<()> {
        self.push(T::read(cursor, child)?);
        Ok(())
    }

    fn is_set(&self) -> bool {
        !self.is_empty()
    }

    fn write_into(&self, out: &mut Emitter, merge: &mut Merge<'_>) {
        for value in self {
            merge.before_child(out);
            value.write(out);
        }
    }
}

/// Puts carried-through fragments back among the typed children of an element.
///
/// Children are written in declaration order, which for a document that was
/// valid is the order they were read in; each fragment remembers the index it
/// had among its siblings and is written when that index comes round.
#[derive(Debug)]
pub struct Merge<'a> {
    pending: std::slice::Iter<'a, Unprocessed>,
    next: Option<&'a Unprocessed>,
    index: u32,
}

impl<'a> Merge<'a> {
    /// A merge over the fragments of one element.
    pub fn new(unprocessed: &'a [Unprocessed]) -> Self {
        let mut pending = unprocessed.iter();
        let next = pending.next();
        Merge {
            pending,
            next,
            index: 0,
        }
    }

    /// Writes every fragment due before the next typed child, then counts the child.
    pub fn before_child(&mut self, out: &mut Emitter) {
        while let Some(fragment) = self.next {
            if fragment.position > self.index {
                break;
            }
            out.raw(fragment);
            self.index += 1;
            self.next = self.pending.next();
        }
        self.index += 1;
    }

    /// Writes the fragments left after the last typed child.
    pub fn finish(&mut self, out: &mut Emitter) {
        while let Some(fragment) = self.next {
            out.raw(fragment);
            self.next = self.pending.next();
        }
    }
}

/// A value carried in an attribute.
pub trait AttributeValue: Sized {
    /// The XML Schema datatype the value is read as, for errors.
    const DATATYPE: &'static str;

    /// The value a lexical form denotes, or `None` if it is not one.
    fn from_text(text: &str) -> Option<Self>;

    /// The lexical form the value is written as.
    fn to_text(&self) -> Cow<'_, str>;
}

impl AttributeValue for Box<str> {
    const DATATYPE: &'static str = "xsd:string";

    fn from_text(text: &str) -> Option<Self> {
        Some(text.into())
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl AttributeValue for u32 {
    const DATATYPE: &'static str = "xsd:integer";

    fn from_text(text: &str) -> Option<Self> {
        text.parse().ok()
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

impl AttributeValue for bool {
    const DATATYPE: &'static str = "xsd:boolean";

    fn from_text(text: &str) -> Option<Self> {
        <bool as Leaf>::from_text(text)
    }

    fn to_text(&self) -> Cow<'_, str> {
        <bool as Leaf>::to_text(self)
    }
}
