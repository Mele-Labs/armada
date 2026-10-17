//! A Session's needs: what it says it needs on a file, and the order its landing takes among
//! everyone else's. `#1059`, `docs/capabilities/needs.md`.
//!
//! **A need is a row on the session ledger** (`ledger`). A session that declares through the
//! intake and a terminal's `armada need` (`served`) hold rows of the same table, so they stand in
//! ONE order, and `armada need`'s answer reads it. **Nothing expires by time**: a person gives a
//! stalled need back.

pub(crate) mod ledger;
mod served;
pub(crate) mod status;
