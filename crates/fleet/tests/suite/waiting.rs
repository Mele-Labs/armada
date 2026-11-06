//! Waiting on what another task does: bounded, and failing by naming it.
//!
//! **A poll whose bound runs out must say so.** One that fell through instead
//! handed the failure to the next assertion, which named something else — a
//! scout that never read under load was reported as one that forgot what it
//! read. `#1494`.

