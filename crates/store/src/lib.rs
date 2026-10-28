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
mod session_focus;
mod session_rail;
mod session_limit;
mod session_loadout;
mod session_turn_usage;
mod usage_summary;
mod usage_rule_trips;
/// A job's word that a Fleet notice was wrong, and the notices its replies are read against.
mod fleet_disputes;
mod session_tells;
mod session_later;
mod watches;
/// The sentences read from a reply that he marked Not a question.
mod reply_misreads;
/// The sentences read from a reply that he confirmed with thumbs up.
mod reply_confirmed;
/// The follow-ups read from a reply that he marked Not deferred.
mod later_misreads;
/// The follow-ups read from a reply that he confirmed as deferred.
mod later_confirmed;
/// The sentences of a reply a disputed walk or done reminder misread.
mod claim_misreads;
/// The nudges Fleet sent about a workflow's steps, kept against the step.
mod workflow_nudges;
/// The changes to a workflow an agent drafted for the owner to approve.
mod workflow_changes;
/// What this Mac syncs with the shared home.
mod home_kinds;
mod home_repositories;
/// How the person chose to land work, per repository and kind of work.
mod delivery_picks;
mod word_asks;
/// The Jobs a person renamed, which the proposer names later Jobs from.
mod job_renames;
mod job_issues;
mod plan_beads;
/// A Job's adversarial reviews of its plan, one a version.
mod adversarial_reviews;
mod fleet_calls;
/// The refreshes of the preview Fleet offered a Job, one a moved tip for the build.
mod preview_offers;
mod notice_mutes;
/// The issue queries a person saved, per repository.
mod issue_queries;
/// The projects a person keeps, across repositories: tasks he wrote and linked issues, in order.
mod projects;
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
/// The plan's last reading, kept so a restarted Fleet can serve it.
mod plan_reading;
/// The span of ports a checkout's server run holds, and Fleet's own listener.
mod ports;
/// A person's preferences, one row per name, kept beside `limits`.
mod preferences;
/// `settings.json`, the Machine's settings file: parsed and written here, read
/// against `config`'s registry everywhere else.
pub mod settings_file;
/// What Fleet told a Session about one it follows, so it is told once.
mod linked_told;
/// The shell each Job's agent last typed into or read.
mod job_shells;
/// A Session dispatched to wait on another, until its condition is met.
mod run_memory;
mod session_waits;
/// What Fleet learnt from every Session of a repository.
mod logbook;
/// What Fleet told the owner of a pull request, so it is told once.
mod pull_notice;
mod repositories;
/// The shape of one retro item.
mod retro;
/// What an agent said about one pull request, kept for the commit it read.
mod review_runs;
mod walk_places;
/// The owner's notes on a pull request's lines, and which Job each was sent to.
mod pull_request_notes;
mod row;
mod schema;
/// Every agent session a person runs and what each holds. `docs/concepts/session.md`.
mod session_ledger;
mod blank_sessions;
mod attachment_versions;
mod session_retro;
/// The Finding a rescue Scout brought back from a stranded slot. Since V106;
/// its verdict and items since V107.
mod slot_rescues;
/// What Fleet did about a stranded slot on its own: told its holder, started a Scout.
mod slot_strandings;
/// Every Studio a repository keeps, with its nodes and edges. `#1285`.
mod studio;
/// Which tests reach each file in a repository. `docs/concepts/test-map.md`.
mod test_map;
/// How long each of a repository's Checks has taken.
mod timings;
/// What Fleet saw happen, for the Cockpit's timeline.
mod timeline;

#[cfg(test)]
mod tests;

pub use devices::{Device, PushSubscription};
pub use error::{DatabaseFault, OpenError, ReadError, RowError, WriteError};
pub use hosted_sessions::{KeptHosting, KeptTerminalAsk};
pub use run_memory::{KeptPlace, KeptRun};
pub use logbook::{
    logbook_words, LogEntry, LogEvidence, LogHistory, LogHow, LogKind, LogRewrite, LogSection, LogState,
};
pub use session_waits::KeptWait;
pub use session_build::{KeptBuild, KeptTask, LandedPull, RoadAsk, RoadTimes, WorkTimes};
pub use session_turn_usage::KeptTurnUsage;
pub use usage_summary::{turn_units, DayUsage, Named, PriorUsage, UsageSplit, UsageSummaryRead, TOP_SESSIONS};
pub use usage_rule_trips::KeptTrip;
pub use fleet_disputes::{KeptDispute, KeptNotice};
pub use session_focus::KeptFocus;
pub use session_plan::{KeptPlan, KeptPlanChange, KeptPlanGroup, KeptPlanTask, KeptTouch};
pub use session_rail::{KeptCite, KeptConcern, KeptRail, KeptReading, KeptStarted, SETTLED_KEPT};
pub use session_tells::KeptTell;
pub use workflow_nudges::KeptWorkflowNudge;
pub use workflow_changes::{
    KeptWorkflowChange, NewWorkflowChange, APPROVED as CHANGE_APPROVED, CANNOT as CHANGE_CANNOT, DISMISSED as CHANGE_DISMISSED, DRAFTING as CHANGE_DRAFTING,
    FAILED as CHANGE_FAILED, READY as CHANGE_READY,
};
pub use session_later::KeptLater;
pub use watches::{KeptWatch, KeptWatchPart, KeptWatchReading, KeptWatchSetAside, KeptWatchTaken, KeptWatchVerdict, KeptWatchWhole, READINGS_KEPT};
pub use notice_mutes::LiveMutes;
pub use word_asks::KeptWordAsk;
pub use job_renames::KeptRename;
pub use home_kinds::KeptHomeItem;
pub use home_repositories::KeptHomeRepository;
pub use job_issues::KeptJobIssue;
pub use plan_beads::KeptBead;
pub use adversarial_reviews::{KeptAdversarialFinding, KeptAdversarialReview};
pub use fleet_calls::KeptCall;
pub use preview_offers::KeptPreviewOffer;
pub use issue_queries::KeptIssueQuery;
pub use projects::{KeptAsked, KeptHolder, KeptProject, KeptProjectRow, KeptProjects, KeptRowItem, KeptTaskState, LOOSE as LOOSE_PROJECT};
pub use session_waiting::KeptWaiting;
pub use session_checks::KeptSessionCheck;
pub use test_map::{StaleTest, TestsReaching};
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
pub use pull_request_notes::KeptPullRequestNote;
pub use session_retro::{KeptRestart, KeptSessionLesson, KeptSessionRetro};
pub use attachment_versions::KeptAttachmentVersion;
pub use session_ledger::{
    AttachmentState, Holder, HolderKind, KeptAttachment, KeptSession, SessionFigures,
    SessionSearch, SessionState,
};
pub use slot_rescues::{KeptRescue, RescueState, RescueVerdict};
pub use slot_strandings::KeptStranding;
pub use studio::{StudioError, Unreadable, UnreadableContent};
pub use timeline::{KeptTimeline, TimelineEntry, TimelineWindow};
