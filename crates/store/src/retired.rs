//! The SQL of the migrations whose tables belong to Jobs, which are gone.
//!
//! **Nothing reads or writes these tables.** The list in `legacy_migrations.rs`
//! is frozen and a database's version row says it ran these, so the statements
//! stay and the tables are made empty and left. No migration drops them.

// The statements are split by position only, to keep each file short to read
// by range; every constant is re-exported here so `crate::retired::Vnn` holds.
mod first;
mod fourth;
mod second;
mod third;

pub(crate) use first::*;
pub(crate) use fourth::*;
pub(crate) use second::*;
pub(crate) use third::*;
