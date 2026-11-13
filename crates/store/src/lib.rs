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
mod macs;
mod error;
mod fleet_outages;
/// What a session Fleet hosts keeps beside its ledger row, and its thread.
mod hosted_sessions;
mod session_checks;
mod session_gates;
mod stop_nudges;
mod job_repairs;
mod job_values;
mod session_build;
mod session_plan;
mod session_focus;
mod session_viewed;
mod session_rail;
mod session_limit;
mod session_snooze;
mod session_open_turn;
mod session_loadout;
mod fleet_builds;
mod job_times;
mod process_stops;
mod session_slot_waits;
mod time_summary;
mod session_turn_usage;
mod usage_summary;
mod usage_spent;
mod usage_rule_trips;
mod waiting_compactions;
/// A job's word that a Fleet notice was wrong, and the notices its replies are read against.
mod fleet_disputes;
mod bug_reports;
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
/// The changes to a workflow an agent drafted for the owner to approve.
mod workflow_changes;
mod workflow_drafts;
/// The versions a workflow has had in one place, and what a copy of one of Armada's began from.
mod workflow_versions;
/// What this Mac syncs with the shared home.
mod home_kinds;
mod home_repositories;
/// How the person chose to land work, per repository and kind of work.
mod delivery_picks;
mod word_asks;
/// The Jobs a person renamed, which the proposer names later Jobs from.
mod job_renames;
/// The runs of a saved request, by the owner or a Job's agent, kept after the Job ends.
mod request_runs;
/// What a Job holds standing in the states of its lifecycles, and the moves between them.
mod lifecycle_instances;
/// The lifecycles and rules a Job started with, and the acts those rules planned for it.
mod job_copies;
mod job_copy_edits;
mod rule_acts;
mod plan_beads;
/// A Job's Judge readings, one an artifact, with their notes.
mod judge_readings;
mod fleet_calls;
/// The refreshes of the preview Fleet offered a Job, one a moved tip for the build.
mod preview_offers;
/// The branches Jobs offer for the owner's preview, and the Refresh row he skipped.
mod preview_offered;
mod notice_mutes;
/// The issue queries a person saved, per repository.
mod issue_queries;
/// The projects a person keeps, across repositories: tasks he wrote and linked issues, in order.
mod projects;
/// What Fleet learned for a project, which the person did not set.
mod project_learned;
/// Where each project's file is kept, and the committed Workspace files imported as projects.
mod project_places;
mod session_init;
mod session_waiting;
/// A repository's Autopilot and what it picked. #2092.
mod autopilot;
mod autopilot_decisions;
mod operator_dispatches;
mod parked_ideas;
/// What his other Macs share with this one, kept whole.
mod other_macs;
/// The Operator's Log of rulings. `armada-ehhu`.
mod operator_rulings;
mod operator_orders;
mod operator_switches;
mod operator_turn;
mod operator_suggestions;
/// The pairs of Jobs the Operator keeps in touch. `armada-ehhu`.
mod operator_in_touch;
/// The line for a slot, kept across a restart. `armada-ehhu`.
mod slot_line;
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
/// A test's fresh store, copied from one migrated once.
mod migrated_copy;
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
/// Each write-clash notice Fleet sent a Job, so a restart does not send it again.
mod clash_told;
/// A Job's terminals: their numbers, names, and the shell its agent last saw in each.
mod job_shells;
/// A Session dispatched to wait on another, until its condition is met.
mod run_memory;
mod session_waits;
/// What Fleet learnt from every Session of a repository.
mod logbook;
/// What Fleet told the owner of a pull request, so it is told once.
mod pull_notice;
/// Which comments on a pull request a Job was told of, and at what edit.
mod pull_comment;
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

pub use devices::{Device, PushSubscription, ThisMac};
pub use macs::{MacSaid, MacState, PairedMac};
pub use error::{DatabaseFault, OpenError, ReadError, RowError, WriteError};
pub use hosted_sessions::{KeptHosting, KeptTerminalAsk};
pub use run_memory::{KeptPlace, KeptRun};
pub use logbook::{
    logbook_words, LogEntry, LogEvidence, LogHistory, LogHow, LogKind, LogRewrite, LogSection, LogState, LogTopic,
};
pub use stop_nudges::KeptStop;
pub use session_waits::KeptWait;
pub use session_open_turn::KeptOpenTurn;
pub use session_build::{KeptBuild, KeptTask, LandedPull, RoadAsk, RoadTimes, WorkTimes};
pub use fleet_builds::{KeptFleetBuild, KeptBuildSplit, KeptPullRequestWait, NewBuild};
pub use job_times::{KeptCheckSpan, KeptJobTime, KeptTurnTimed};
pub use session_slot_waits::KeptSlotWait;
pub use time_summary::{BuildFigures, HourFigures, HoursRead, Medians, SlowJob, SplitMs, TimeRead, Usual, DEFAULT_HOURS, DEFAULT_SLOWEST, OWNER_ASKING_TOOLS};
pub use session_turn_usage::{KeptBackground, KeptOpenCall, KeptReading, KeptToolResult, KeptTurnSpent, KeptTurnTimes, KeptTurnUsage};
pub use usage_spent::{SpentSplit, SpentWindowRead, UsageSpentRead, SPENT_BY, SPENT_ON, SPENT_WINDOWS};
pub use usage_summary::{turn_units, DayUsage, Named, PriorUsage, UsageSplit, UsageSummaryRead, TOP_SESSIONS, UNIT_SCALE};
pub use usage_rule_trips::KeptTrip;
pub use waiting_compactions::{KeptCompaction, NextRequest};
pub use fleet_disputes::{KeptDispute, KeptNotice};
pub use bug_reports::{KeptBugFix, KeptBugReport};
pub use session_focus::KeptFocus;
pub use session_plan::{KeptPlan, KeptPlanChange, KeptPlanGroup, KeptPlanTask, KeptTouch};
pub use session_rail::{KeptRail, KeptStarted};
pub use session_tells::KeptTell;
pub use workflow_changes::{
    KeptWorkflowChange, NewWorkflowChange, APPROVED as CHANGE_APPROVED, CANNOT as CHANGE_CANNOT, DISMISSED as CHANGE_DISMISSED, DRAFTING as CHANGE_DRAFTING,
    FAILED as CHANGE_FAILED, READY as CHANGE_READY,
};
pub use workflow_drafts::{DraftKey, KeptDraft, KeptDraftChange, NewDraftChange, BY_AGENT as DRAFT_BY_AGENT, BY_YOU as DRAFT_BY_YOU};
pub use workflow_versions::{KeptBase, KeptVersion, NewVersion, WorkflowPlace, ORIGIN_FOUND as VERSION_FOUND, ORIGIN_PUBLISHED as VERSION_PUBLISHED};
pub use session_later::{KeptHanded, KeptLater};
pub use job_shells::KeptJobTerminal;
pub use watches::{KeptWatch, KeptWatchPart, KeptWatchReading, KeptWatchSetAside, KeptWatchTaken, KeptWatchVerdict, KeptWatchWhole, READINGS_KEPT};
pub use notice_mutes::LiveMutes;
pub use word_asks::KeptWordAsk;
pub use job_renames::KeptRename;
pub use job_repairs::KeptRepair;
pub use job_values::KeptValue;
pub use request_runs::{KeptRequestRun, NewRun, RunsAsked, StatusWanted, RUNS_KEPT};
pub use home_kinds::KeptHomeItem;
pub use home_repositories::KeptHomeRepository;
pub use lifecycle_instances::{KeptInstance, KeptMove, KeptRead};
pub use job_copies::{KeptCopy, KeptPiece, PieceKind};
pub use job_copy_edits::{KeptCopyMark, KeptWorkflowCopy};
pub use rule_acts::{standing as rule_act_standing, KeptRuleAct, KeptSeen, NewRuleAct};
pub use plan_beads::KeptBead;
pub use judge_readings::{KeptJudgeNote, KeptJudgeReading};
pub use fleet_calls::KeptCall;
pub use preview_offers::KeptPreviewOffer;
pub use preview_offered::KeptPreviewOffered;
pub use issue_queries::KeptIssueQuery;
pub use projects::{
    Edited, KeptAsked, KeptHolder, KeptProject, KeptProjectRow, KeptProjects, KeptRowItem, KeptTaskIssue, KeptTaskPull, KeptTaskState, LOOSE as LOOSE_PROJECT,
};
pub use session_init::KeptInit;
pub use session_waiting::{KeptApproval, KeptClosed, KeptWaiting};
pub use session_checks::KeptSessionCheck;
pub use session_gates::{KeptGate, KeptGateChecks, KeptGateHold, CHECKS_GREEN, PLAN_APPROVED, WALK_APPROVED};
pub use test_map::{StaleTest, TestsReaching};
pub use model_downshift::KeptDownshift;
pub use autopilot::{KeptAutopilot, KeptPick};
pub use autopilot_decisions::KeptDecision;
pub use operator_dispatches::KeptDispatch;
pub use parked_ideas::KeptIdea;
pub use other_macs::{FromMac, KeptFromMac, KeptProjectAct};
pub use operator_rulings::KeptRuling;
pub use operator_orders::{KeptOrder, KeptTalk};
pub use operator_switches::KeptSwitch;
pub use operator_turn::{KeptPage, KeptTurn};
pub use operator_suggestions::KeptSuggestion;
pub use slot_line::KeptInLine;
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
