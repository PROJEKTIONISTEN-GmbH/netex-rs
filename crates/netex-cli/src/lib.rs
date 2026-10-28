//! The `netex` command: a checking report for NeTEx deliveries, their
//! statistics, their conversion to GTFS, regional cuts of them and the merge
//! of several into one — each command a module here, the merge netex-rs's own
//! (`netex_rs::merge`), the binary in `main.rs` parsing the command line around
//! them.

pub mod area;
pub mod extract;
pub mod stats;
pub mod validate;
