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
mod serving;
mod session;
mod session_fork;
mod session_host;
mod session_piloting;
mod sessioning;
mod settling;
mod terminal_session;
mod showing;
mod showing_again;
mod silence;
mod slot_pool;
mod snapshotting;
mod standing_rules;
mod starting;
mod starting_empty;
mod step_baseline;
mod step_gate;
mod stuck;
mod studio_pictures;
mod studio_runs;
mod studio_servers;
mod studio_sketches;
mod studios;
mod sub_dispatch;
mod superseding;
mod terms;
mod walk_notes;
mod walking;
mod work_plan;
// `pub(crate)`, not `mod`: `crate::records::migrating`'s own tests are not a
// descendant of this module and need the same temporary directory every
// fixture here already uses, rather than a second one invented beside it.
pub(crate) mod tmp;
