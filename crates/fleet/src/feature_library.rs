//! A project's feature library, served from Fleet's store. `docs/concepts/feature-catalog.md`.
//!
//! **Fleet files what it sees and a Job writes what only it knows.** The files a Job wrote, the pull requests it held,
//! the merges and the artifacts made for a feature are detected here (`filing`); the four sections, how to test it,
//! how it runs, how to observe it and its debt, are written by the Job that did the work (`writing`) and `all_done` is
//! held until it has (`holding`). `mapping` says which features own a file, `staleness` when a section no longer
//! matches the code, `brief` what an agent is handed first, `measuring` how the library is judged. The library lives on
//! this Mac; nothing here is shared with teammates.

pub(crate) mod brief;
mod filing;
mod holding;
pub(crate) mod mapping;
mod measuring;
mod reading;
mod seeding;
mod serving;
pub(crate) mod staleness;
mod writing;

pub(crate) use holding::{sections_owed_text, OWED};
pub(crate) use seeding::Seed;
