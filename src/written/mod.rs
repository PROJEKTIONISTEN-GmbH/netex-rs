//! A document as it is written, byte for byte: where each element starts and ends and where the
//! values of its attributes stand ([`scan`]), the objects of its frames with what they refer to and
//! hold ([`outline`]), and changes applied as the document is written out again — elements left
//! out, values replaced, every other byte as it was ([`edit`]). What [`crate::merge`] rewrites a
//! delivery by, and what a tool that cuts a delivery (the `netex` command's `extract`) needs.

pub mod edit;
pub mod outline;
pub mod scan;
