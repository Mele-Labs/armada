//! What this crate proves about itself.
//!
//! **Every module below carries its own header, saying what it proves and why
//! its cases are shaped that way. There is deliberately no index of them
//! here.** The one this replaces described about half of them and numbered
//! them, and the numbering had drifted far enough that two modules were both
//! "the eleventh" — a copy of every header, checked by nothing, is the thing
//! that can be wrong about a suite while every test in it passes.
//!
//! What no single module can say is which of them are one subject:
//!
//! - `process`, `runtime` and `detach` are one. A Fleet that outlives the app
//!   must be findable, and its runtime file must let a reader tell a live Fleet
//!   from a pid that used to be one.
//! - `asked_run` is `gate` asked from the other side, before a step is spent.
//! - `peer` is the primitive `concurrency` rests on: a call attributed by the
//!   connection it arrived on rather than by which Job was admitted first.

// The rest are `mods.inc`, one a file here: `build.rs` writes it.
include!("mods.inc");

pub(crate) mod repositories;
pub(crate) mod seeding;
pub(crate) mod servers;
// `pub(crate)`, not `mod`: `crate::records::migrating`'s own tests are not a
// descendant of this module and need the same temporary directory every
// fixture here already uses, rather than a second one invented beside it.
pub(crate) mod tmp;
mod tools;
mod transcript;
mod tuning;
mod unattended;
mod under_review;
mod underway;
mod verify_runs;
mod waiting_checks;
mod watching;
mod wave_rounds;
mod waves;
mod widening;
mod workflow_promise;
mod workspace_gate;
mod workspace_loading;
mod workspace_ports;
mod workspace_runs;
mod workspace_verify;
