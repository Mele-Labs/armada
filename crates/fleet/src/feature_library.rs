//! A project's feature library, served from Fleet's store. `docs/concepts/feature-catalog.md`.
//!
//! **Fleet files what it sees and a Job writes what only it knows.** The files a Job wrote, the pull requests it held,
//! the merges and the artifacts made for a feature are detected here (`filing`); the four sections, how to test it,
//! how it runs, how to observe it and its debt, are written by the Job that did the work (`writing`) and `all_done` is
//! held until it has (`holding`). `mapping` says which features own a file, `staleness` when a section no longer
//! matches the code, `floor_plan` which modules it lights, `brief` what an agent is handed first, `measuring` how the library is judged. The library lives on
//! this Mac; nothing here is shared with teammates.

pub(crate) mod brief;
mod changing;
mod consulting;
pub(crate) mod committing;
mod filing;
mod floor_plan;
mod handoff;
mod holding;
pub(crate) mod librarian;
pub(crate) mod mapping;
mod measuring;
mod modules;
mod pending;
mod project_pages;
mod reading;
mod seeding;
mod serving;
pub(crate) mod staleness;
mod writing;
mod glossary;
pub(crate) mod since_paragraph;

pub(crate) use holding::{sections_owed_text, OWED};
pub(crate) use reading::{features_of_note, slug_held};
pub(crate) use pending::pending_facts;
#[cfg(test)]
pub(crate) use seeding::Seed;
pub(crate) use seeding::PROJECT as PROJECT_UNKNOWN;
#[cfg(test)]
pub(crate) use floor_plan::{is_contract, plan as floor_plan_of_layouts};
#[cfg(test)]
pub(crate) use modules::read as read_modules;
#[cfg(test)]
pub(crate) use project_pages::first_sentence;
