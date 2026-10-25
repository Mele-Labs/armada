//! What this crate proves about itself: `corrupt` shows a damaged file is
//! refused rather than opened empty, `migrate` and `named_migrations` show what
//! a migration does to a file an earlier one wrote, and the rest each hold one
//! table's reads and writes across a reopen.

mod corrupt;
mod devices;
mod helm_sessions;
mod helm_tuning;
mod delivery_picks;
mod home_kinds;
mod home_repositories;
mod kit;
mod lessons;
mod limits;
mod main_ci;
mod main_fix;
mod manifest_allowed;
mod run_memory;
mod logbook;
mod migrate;
mod named_migrations;
mod slot_strandings;
mod plan_reading;
mod ports;
mod preferences;
mod rebuilding;
mod session_build;
mod repositories;
mod review_runs;
mod walk_places;
mod session_checks;
mod session_ledger;
mod session_loadout;
mod session_turn_usage;
mod usage_rule_trips;
mod fleet_disputes;
mod usage_summary;
mod session_plan;
mod session_focus;
mod session_rail;
mod session_tells;
mod session_later;
mod notice_mutes;
mod word_asks;
mod job_renames;
mod job_issues;
mod plan_beads;
mod fleet_calls;
mod issue_queries;
mod settings_file;
mod studio;
mod studio_authors;
mod studio_files;
mod studio_forge;
mod studio_runs;
mod studio_sketches;
mod studio_unfrozen;
mod studio_zones;
mod test_map;
mod timeline;
mod timings;
mod watches;
mod tmp;

use core_model::{Timestamp, Ulid};

use crate::Store;
pub use tmp::TempDir;

pub fn ulid(value: &str) -> Ulid {
    Ulid::carried(value)
}

pub fn at(value: &str) -> Timestamp {
    Timestamp::from_rfc3339(value)
}

pub fn open(dir: &TempDir) -> Store {
    Store::open(&dir.db()).expect("a fresh store opens")
}
