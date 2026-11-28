//! A project's feature library, served from Fleet's store. `docs/concepts/feature-catalog.md`.
//!
//! **Fleet files what it sees and a Job writes what only it knows.** The files a Job wrote, the pull requests it held,
//! the merges and the artifacts made for a feature are detected here (`filing`); the four sections, how to test it,
//! how it runs, how to observe it and its debt, are written by the Job that did the work (`writing`) and `all_done` is
//! held until it has (`holding`). `mapping` says which features own a file, `staleness` when a section no longer
//! matches the code, `floor_plan` which modules it lights, `brief` what an agent is handed first, `measuring` how the library is judged. The library lives on
//! this Mac; nothing here is shared with teammates.

pub(crate) mod asking;
mod asking_rank;
mod asking_reply;
pub(crate) mod catalog_rules;
pub(crate) mod brief;
mod changing;
mod consulting;
pub(crate) mod committing;
pub(crate) mod deciding;
mod filing;
pub(crate) mod doc_drift;
mod doc_checking;
mod floor_files;
mod floor_plan;
mod handoff;
mod health;
mod holding;
pub(crate) mod librarian;
pub(crate) mod librarian_steps;
pub(crate) mod mapping;
mod measuring;
pub(crate) mod note_claims;
pub(crate) mod meaning;
mod modules;
mod of_job;
mod pending;
mod project_pages;
pub(crate) mod ranking;
mod reading;
mod retiring;
mod seeding;
mod serving;
mod starting;
pub(crate) mod sub_projects;
pub(crate) mod staleness;
pub(crate) mod upkeep;
mod writing;
mod glossary;
mod gisting;
mod librarian_gists;
pub(crate) mod since_paragraph;
mod whos_on_what;

pub(crate) use holding::{sections_owed_text, OWED};
pub(crate) use reading::{features_of_note, slug_held};
pub(crate) use pending::pending_facts;
#[cfg(test)]
pub(crate) use seeding::Seed;
pub(crate) use seeding::PROJECT as PROJECT_UNKNOWN;
#[cfg(test)]
pub(crate) use floor_plan::{is_contract, plan as floor_plan_of_layouts, read as read_floor};
#[cfg(test)]
pub(crate) use modules::read as read_modules;
pub(crate) use floor_plan::read as layout_with_files;
pub(crate) use modules::{json_scan, Dep as ModuleDep, Kind as ModuleKind, Layout as ModuleLayout, Module, SKIPPED as SKIPPED_FOLDERS};
#[cfg(test)]
pub(crate) use project_pages::first_sentence;
