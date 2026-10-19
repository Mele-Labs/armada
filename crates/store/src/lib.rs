//! SQLite in WAL mode, migrations, retention — and **the only crate that
//! deserializes**. A Cargo-graph fact rather than a review item: the SQLite
//! dependency is scoped here alone, so nothing else *can* read a row. `ipc` is
//! the other place bytes enter the process, from the wire.
//!
//! **Query functions never return pre-filtered results**, but the parse
//! failures alongside them. v1 wrote `.filter_map(Result::ok)` after a store
//! call and dropped twenty-one real rows with no error anywhere — the bug was
//! at the call site, and a signature handing back a bare `Vec` invites it.
//!
//! **A corrupt store is refused and an empty one created**, on six checks in
//! `open.rs` that are each fatal: silently starting empty over a database that
//! exists means somebody loses work and is shown a clean slate.
//!
//! **Nothing here reads a clock.** Schema versioning is a migration list and a
//! version row, applied on open.
//!
//! **The Job tables are left, unread.** Sessions replaced Jobs; no migration
//! drops what a Job wrote, and nothing here reads or writes it. `retired`
//! holds the SQL that made them, which the frozen list still runs.

/// What Fleet told a session about the build it now runs, so it is told once.
mod build_notice;
mod devices;
mod error;
/// The session each Helm conversation resumes, one row per conversation.
mod helm_sessions;
/// The model and effort a person chose for a Helm conversation.
mod helm_tuning;
/// What a session Fleet hosts keeps beside its ledger row, and its thread.
mod hosted_sessions;
mod session_checks;
mod session_build;
mod session_plan;
mod session_rail;
mod session_limit;
mod session_loadout;
mod session_turn_usage;
mod usage_summary;
mod usage_rule_trips;
mod session_waiting;
mod sleep_night;
/// A repository's Autopilot and what it picked. #2092.
mod autopilot;
/// Kit's MCP servers, and each Manifest's word over one. `#1275`.
mod kit;
/// The 113 migrations the numbered list held, frozen.
mod legacy_migrations;
/// The SQL of migrations whose tables belong to Jobs, which are gone. Unread.
mod retired;
/// The Fleet limits a person saved, one row or none.
mod limits;
/// Main's CI on the forge, per repository, and the failed jobs of a red.
mod main_ci;
/// Which Session took each red main, and which one fixed it.
mod main_fix;
/// Commands a person always-allowed for a whole Manifest, kept here instead
/// of a commit on some Session's branch.
mod manifest_allowed;
/// Reads the `migrations/` directory; the build script includes it too.
#[cfg(test)]
mod migration_files;
/// What a migration is, and the table recording which names a file has had.
mod migrations;
/// The default model's step down while the plan's window is spent.
mod model_downshift;
mod open;
/// The span of ports a checkout's server run holds, and Fleet's own listener.
mod ports;
/// A person's preferences, one row per name, kept beside `limits`.
mod preferences;
/// `settings.json`, the Machine's settings file: parsed and written here, read
/// against `config`'s registry everywhere else.
pub mod settings_file;
/// What Fleet told a Session about one it follows, so it is told once.
mod linked_told;
/// A Session dispatched to wait on another, until its condition is met.
mod run_memory;
mod session_waits;
/// What Fleet told the owner of a pull request, so it is told once.
mod pull_notice;
mod repositories;
/// The shape of one retro item.
mod retro;
/// What an agent said about one pull request, kept for the commit it read.
mod review_runs;
mod walk_places;
mod row;
mod schema;
/// Every agent session a person runs and what each holds. `docs/concepts/session.md`.
mod session_ledger;
mod blank_sessions;
mod session_retro;
/// The Finding a rescue Scout brought back from a stranded slot. Since V106;
/// its verdict and items since V107.
mod slot_rescues;
/// What Fleet did about a stranded slot on its own: told its holder, started a Scout.
mod slot_strandings;
/// Every Studio a repository keeps, with its nodes and edges. `#1285`.
mod studio;
/// How long each of a repository's Checks has taken.
mod timings;
/// What Fleet saw happen, for the Cockpit's timeline.
mod timeline;

#[cfg(test)]
mod tests;

pub use devices::{Device, PushSubscription};
pub use error::{DatabaseFault, OpenError, ReadError, RowError, WriteError};
pub use hosted_sessions::{KeptHosting, KeptTerminalAsk};
pub use run_memory::KeptRun;
pub use session_waits::KeptWait;
pub use session_build::{KeptBuild, KeptTask, LandedPull, RoadAsk, RoadTimes, WorkTimes};
pub use session_turn_usage::KeptTurnUsage;
pub use usage_summary::{turn_units, DayUsage, Named, PriorUsage, UsageSplit, UsageSummaryRead, TOP_SESSIONS};
pub use usage_rule_trips::KeptTrip;
pub use session_plan::{KeptPlan, KeptPlanChange, KeptPlanGroup, KeptPlanTask, KeptTouch};
pub use session_rail::{KeptCite, KeptConcern, KeptRail, KeptReading, SETTLED_KEPT};
pub use session_waiting::KeptWaiting;
pub use session_checks::KeptSessionCheck;
pub use model_downshift::KeptDownshift;
pub use sleep_night::{SleepRow, SleepSwitch};
pub use autopilot::{KeptAutopilot, KeptPick};
pub use limits::SavedLimits;
pub use main_ci::{MainCi, MainFailedJob, MainMerge, MainState};
pub use main_fix::{MainFix, TakenBy, TakenHow};
pub use migrations::KNOWN_SCHEMA_VERSION;
pub use open::Store;
pub use ports::{PortClaim, PortClaimant};
pub use preferences::{Preferences, SavedPreferences, SHIPPED_THEME};
pub use retro::RetroLine;
pub use review_runs::KeptReviewRun;
pub use walk_places::{HeldReviewWalk, KeptWalkPlace};
pub use session_retro::{KeptRestart, KeptSessionLesson, KeptSessionRetro};
pub use session_ledger::{
    AttachmentState, Holder, HolderKind, KeptAttachment, KeptSession, SessionFigures,
    SessionSearch, SessionState,
};
pub use slot_rescues::{KeptRescue, RescueState, RescueVerdict};
pub use slot_strandings::KeptStranding;
pub use studio::{StudioError, Unreadable, UnreadableContent};
pub use timeline::{KeptTimeline, TimelineEntry, TimelineWindow};
