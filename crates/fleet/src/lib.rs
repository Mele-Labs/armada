//! The daemon: Sessions, worktrees, Checks, delivery and the machine they run on.
//!
//! Each module says what it is for; there is no second copy of that here,
//! because the tour this replaces went stale a milestone at a time. What
//! follows binds the crate rather than one file.
//!
//! **Fleet reaches an agent only through `adapter-traits`** — never a vendor CLI directly,
//! anywhere.
//!
//! **The OS lifecycle is its own module, orthogonal to scheduling.** v1 mixed
//! the two and its `schedule.rs` alone reached 2,929 lines.
//!
//! **The harness renders and Fleet starts.** Nothing in `adapters` spawns, so
//! [`Detached`](detach::Detached) is the only way a process begins here, and
//! every confinement property is a value a test reads rather than a process a
//! test runs.

pub mod adrift;
/// The commands a person allowed, kept as a list they can read and take back.
pub mod allowed;
/// A form's edits to `armada.yml`, placed a key at a time and written through
/// `editing::save`.
pub mod amending;
/// A repository working its own backlog. #2092.
pub mod asks;
pub mod auto_gate;
pub mod autopilot;
/// Saving a workflow definition, and reading the workflow folders again.
pub mod authoring;
/// The backlog Fleet writes to: each copy kept in step, and an issue claimed in it.
pub mod backlog;
/// Fleet syncing each backlog while it runs, and claiming an issue in one.
pub mod backlog_syncing;
pub mod basing;
/// What a Session has built: the rules between what Fleet observed and the record on its row.
pub mod build_facts;
pub mod build_record;
/// Which URLs and issues a Studio or Session names, looked up for what they are.
pub mod enriching;
/// Where the build record meets the store.
pub mod build_keeping;
/// Which files a Session and its subagents touched, read off the tool calls Fleet hears.
pub mod file_touching;
/// A Session's changes marked by the person, checked against its plan and worded for the agent.
pub mod plan_marks;
/// The ticket a plan's group works, where the agent left it blank.
pub mod plan_issues;
/// A Session's plan: versions, the diff between them, task progress and groups worked.
pub mod plan_record;
/// Where a Session's plan meets the store.
pub mod plan_keeping;
pub mod plan_mirror;
pub mod plan_mirroring;
pub mod backlog_closing;
/// Where a Session's Loadout meets the store.
pub mod loadout_keeping;
/// Where a Session's turn usage meets the store.
pub mod turn_ingest;
pub mod usage_following;
pub mod job_time;
pub mod job_timing;
pub mod usage_rules;
pub mod usage_summarising;
pub mod time_summarising;
pub mod turn_usage_keeping;
pub mod loadout_reading;
/// A Session's place in its workflow, read from what Fleet observed, and the nudge for a step skipped.
pub mod rail;
/// What ends each step of a workflow, derived from its anchor, for the workflow's page.
pub mod rail_ends;
pub mod rail_lifecycle;
/// Where the rail meets the store.
pub mod rail_keeping;
/// What Fleet does when a Session's step moves.
pub mod linked_following;
pub mod after_following;
pub mod rail_following;
/// Telling sessions the build Fleet and Bridge now run, once per build.
pub mod build_notices;
/// The build Fleet runs on and the detached restart onto another. `scripts/restart-build`.
pub mod building;
/// Every build Fleet has run, kept for the Usage page.
pub mod builds_run;
/// The hours, builds and Jobs the Usage page reads time from.
pub mod time_reading;
/// What a repository runs in its checkout when the base moves there.
pub mod base_moves;
/// How long a Check may run.
pub mod budget;
/// The served checkout caught up whenever main moves.
pub mod behind_bay;
pub mod catching_up;
pub mod check_output;
pub mod checkouts;
/// Who wrote the same file as whom, and telling both.
pub mod clashing;
pub mod clock;
/// `api::Commands`, implemented over a real Fleet — the write half of the seam
/// `serving` holds the read half of. Three traits, three impl blocks, three
/// files, and no delegating signature between them.
pub mod commanding;
/// A copy of a person's own piece committed by a Job of its own: `commit_to_repository`.
pub mod committing;
pub mod configured;
pub mod daemon;
pub mod detach;
/// Whether the repository still has what `armada.yml` names. **A read of the
/// repository**, where `daemon::rereading` is a read of the file — a `run`
/// line naming a deleted script parses perfectly and says nothing.
pub mod drifting;
pub mod drone;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with. Save stops at the bytes.
pub mod editing;
/// Fetching a Job's base from the remote before a catch-up reads it.
pub mod fetching_the_base;
/// Walking the checkout for `search_files`, the `@` mention popup's read.
pub mod files;
/// A running Check's log, read for `observe_check_output` as it grows.
pub mod following;
pub mod framing;
pub mod group;
pub mod headroom;
pub mod judging;
/// The builds a restart keeps, and the swap back to the one before.
pub mod kept_builds;
/// The Cockpit's flight controls a person arranged, in `~/.armada`. `docs/concepts/kit.md`.
pub mod flight_controls;
/// Kit's MCP servers, resolved for one Manifest. `docs/concepts/kit.md`, `#1275`.
pub mod kit;
/// Fleet's local language model, shared by every reader.
pub mod language_model;
/// Kit's allowlist, in `~/.armada`. `docs/concepts/kit.md`.
pub mod kit_allowlist;
/// Kit's Add and Remove for a plugin or a skill, across the seam.
pub mod kit_pieces;
/// A Job's worktree is the pool slot it leased.
pub mod leasing;
/// Requests: an HTTP request filled from a repository's environments, and its collections.
pub mod requesting;
/// The Drones-at-once bound, memory share and disk floor a person saves, and
/// how a save reaches admission without a restart.
pub mod limits;
pub mod listener;
/// Main's CI, read off the forge on an interval.
pub mod main_ci;
pub mod main_fix;
pub mod main_hub;
/// A possible `armada.yml` per workspace, from Scan, and the Write that ends it.
pub mod manifest_proposal;
pub mod merge_lines;
pub mod act_failing;
pub mod model_downshift;
pub mod model_downshifting;
/// The models the harness offers, asked after Fleet is serving.
pub mod models_read;
pub mod phone;
/// The Judge's four operations Fleet serves: the one read-only call, the readings kept, and the person's acts.
pub mod judge_serving;
/// What a Judge is shown, and how its answer is read and pinned. Pure.
pub mod judge;
/// Each kind of artifact of a Job, read into the words a Judge is shown.
pub mod judge_content;
/// A reading between the store's rows and the wire, and the message a sent note is.
pub mod judge_keeping;
/// Kit against the shared home, and what this machine syncs. `docs/concepts/kit.md`.
pub mod home;
/// Mods: folders on this machine that change how Bridge looks.
pub mod mods;
pub mod mint;
/// What Fleet saw happen, kept for the Cockpit's timeline. Since 23.82.
pub mod timeline;
/// Who held one slot and what was checked out in it, for the Cockpit's Bay pane.
pub mod bay_history;
/// The lines of code a pinned note's spot was drawn from, found by the words on screen.
pub mod note_locating;
pub mod notice_loop;
/// The Operator: who does each piece of work that arrives. `armada-ehhu`.
pub mod on_your_words;
pub mod operator;
/// What a Job may do to the Jobs it started, decided on values.
pub mod started_jobs;
pub mod noticing;
/// Drone groups whose Fleet is gone, found by the mark Fleet put on them.
pub mod orphans;
pub mod session_limit;
/// What a Session's proposer is asked and how its answer reads, with the words the Job proposer shares.
pub mod session_proposer;
/// What the account writer is asked of a Session at work and how its answer reads.
pub mod session_account;
/// What the quick bar is offered beyond his saved lines: what the small model is asked, and how his sent lines rank.
pub mod session_quick;
pub mod peer;
/// A form's edit sent as a pull request, pending until the checkout has it.
pub mod pending_edit;
/// The machine's places for Checks, one line for every Job and repository. #1063.
pub mod places;
pub mod ports;
/// A person's Bridge preferences, five keys of settings.json in their older shape.
pub mod preferences;
pub mod preparing;
/// The probes Fleet can run on itself, and the Doctor modules it cannot.
/// **Not Doctor**, whose grid is ten modules and is not built.
pub mod probing;
/// The text of Copy debug report, read from what Fleet holds.
pub mod debug_reporting;
pub mod process;
/// The profiles a machine has joined, each with a directory of its own for its key.
pub mod profiles;
/// Starting a scout on a Studio, recording what it read, and stopping it.
pub mod promoting;
/// The prompts a person may override in settings.json: what ships, and what is in force.
pub mod prompts;
/// What happened to a pull request, told to the Sessions and Jobs that own it.
pub mod pull_notices;
/// Every comment on a pull request, filed for the Job that holds it, and a line when a reviewer speaks.
pub mod pull_comments;
pub mod pull_waits;
/// A pull request nobody reported, attached to the Session that worked its branch.
pub mod claiming;
pub mod pull_attaching;
/// Whose pull requests Fleet watches: the signed-in account's and the forge command line's.
pub mod my_pulls;
/// Who owns a pull request, from the Sessions ledger and the Jobs.
pub mod pull_owners;
/// Which Session or Job claimed a pull request, kept in the central store for the other machines.
pub mod pull_registry;
/// What a pull request's panel reads when it opens, and its comment box writes.
/// Since 23.83.
pub mod pull_request_panel;
pub mod pull_review_lists;
/// The agent runs about a pull request, kept per commit. `docs/concepts/pull-review.md`.
pub mod review_runs;
/// Natural Search: the person's words put to every open pull request. `docs/concepts/pull-review.md`.
pub mod review_searches;
/// Saved issue queries and the query writer. `docs/concepts/tracker.md`, *Saved queries*.
pub mod issue_queries;
/// A repository's projects: tasks a person wrote and issues he linked, kept on this Mac.
pub mod projects;
pub mod project_holds;
/// The review tools and explanations a repository has, and writing one.
pub mod review_tools;
/// A Walk run: where the change runs, and the walk played there. `docs/concepts/pull-review.md`, *Walk*.
pub mod review_walk_runs;
/// A pull request's change held up for a walk: a bay, a server, and giving them back.
pub mod review_walks;
/// Where a pull request's change runs, and the places Fleet remembers.
pub mod walk_places;
pub mod pull_request_notes;
/// A pull request by repository and number, and the acts a Session takes on one.
/// Since 23.48.
pub mod pull_request_acts;
pub mod pull_requesting;
/// A person's refresh of one pull request: everything the pull reading does for it, now.
pub mod pull_refreshing;
/// The issues a Session works on: named by the person, or closed by its pull requests.
pub mod session_issues;
/// What a Job holds, standing in the states of its lifecycles, and the readers that say where.
pub mod lifecycles;
/// The web links a Session's person and agent write, kept on its ledger.
pub mod session_links;
/// Reading a Link in: the source, fetched by Fleet. `#1293`.
pub mod reading_in;
/// Links already on a Studio, read as what their addresses name — `#1394`.
pub mod recognising;
/// Where one repository's records live, off the checkout — the per-repository
/// key and the one-time move of what an older Fleet wrote under it.
pub mod records;
pub mod redaction;
pub mod refusing;
/// A person's run of one Manifest entry in a Job's worktree, and Undo from
/// the snapshot taken before it. **A rehearsal, never a verdict.**
pub mod rehearsing;
/// A command Fleet runs on a cadence for one Job, and the agent's weighing of it.
pub mod watches;
/// The repositories one Fleet serves, and adding one by folder.
pub mod repositories;
/// Which profile a repository claims and syncs as: matched by its owner, or chosen.
pub mod repository_profiles;
pub mod rescuing;
pub mod stranding;
/// A Job's retro: what got in the way while it ran. `docs/concepts/retro.md`.
pub mod retro;
pub mod review;
pub mod runtime;
pub mod saving;
pub mod saying;
/// Scan: reading a repository nobody set up for Armada. **It writes nothing,
/// because a [`scanning::Tree`] has no write on it.**
pub mod scanning;
pub mod scoping;
/// Each Job's own scratch folder, which the slot guard reads as a slot.
pub mod scratch;
/// The read-only agent a person starts from a Studio. `#1292`.
pub mod scout;
pub mod scouting;
pub mod seeding;
/// A held checkout's uncommitted files sent up as a pull request.
pub mod sending_checkout;
pub mod servers;
/// A repository's app started in development: its `runs:`, drafted, saved and held. `docs/concepts/manifest.md`.
pub mod app_runs;
pub mod serving;
/// The session ledger: what a harness says of an agent session. `docs/concepts/session.md`.
pub mod session_host;
pub mod session_check_live;
/// A person's own `armada.yml` for a repository, at the place its tiles are kept. `docs/concepts/manifest.md`.
pub mod kit_manifests;
/// How a repository runs, remembered from what its Jobs did. `docs/concepts/kit.md`.
pub mod machine_note;
pub mod run_memory;
/// What Fleet learns from every Session of a repository — `docs/concepts/logbook.md`.
pub mod logbook;
/// What a person taught: a Logbook entry for the agent note, and the mods it changes.
pub mod teaching;
pub mod notice_mutes;
/// Folders and runs read from a person's agent CLI, and installing the mod into it.
pub mod session_import;
pub mod session_checking;
pub mod session_running;
/// A Job's terminal held by a keeper so it outlives Fleet. `docs/concepts/session/shared-terminal.md`.
pub mod terminal_keeping;
/// A shell action run from a flight control, in the slot a Session holds.
pub mod session_shell;
/// Pieces made into a package, read, and added to the person's own Kit.
pub mod sharing;
pub mod sessioning;
pub mod window_sign_in;
/// The Job browser's calls, held until Bridge answers them.
pub mod job_browser;
pub mod slot_holders;
pub mod slot_reclaiming;
/// A red main claimed in the central store, and the claim read back for the hub.
pub mod red_claim;
/// A red main whose pull request a Session merged is claimed for that Session, or for a fork of it.
pub mod session_red;
pub mod taking_red;
/// Signing in to an account with the forge's device flow, and enrolling the machine with the store.
pub mod signing_in;
pub mod slots;
/// The lock on the store, which names a holder past a quarter second.
pub mod store_lock;
/// The test map: collectors as plugs, laps written as edges, the map read for paths.
/// `docs/concepts/test-map.md`.
pub mod test_map;
/// Fleet feeding the test map from main moving and from an Autopilot bay.
pub mod test_map_feed;
/// A model reading the changed files no collector can measure into the test map.
pub mod test_map_reading;
/// The laps main's CI run recorded, fetched from the forge and kept as edges.
pub mod test_map_ci;
/// A Picture a person pastes onto a Studio, kept as a Note's frame is.
pub mod studio_pictures;
/// Runs a Studio holds, and what a node keeps of one past retention. `#1289`.
pub mod studio_runs;
/// Servers a Studio holds: started from one, and kept when they end. `#1345`.
pub mod studio_servers;
pub mod studio_sketches;
/// A repository's Studios, through the store and onto the stream. `#1285`.
pub mod studios;
pub mod subagents_resumed;
pub use worktrees_kept::keep_worktrees_warm;
/// settings.json: the file, its watch's re-read, its save, and the move of the
/// store's saved rows into it. `docs/concepts/machine.md`, *settings.json*.
pub mod settings;
pub mod sweeping;
/// What each served repository holds and where each agent has changed it, for the deck's map.
pub mod territory;
/// A repository's open issues, read from its tracker. #91.
pub mod tracking;
/// Where a path an agent named lies in a repository, and its name from its worktree's root.
pub mod tree_paths;
pub mod transcript;
/// The live dials settings.json moves, shipped and overlaid by saved.
pub mod tuning;
pub mod worktrees_kept;
/// The redactions the `Queries` and `Commands` impls call by hand. Split out to
/// keep those files, rather than their helpers, the thing that grows.
pub mod wire;
pub mod workflow_lifecycles;
pub mod workflow_page;
/// A change to a workflow an agent drafts for the owner to approve.
pub mod workflow_changes;
/// The lifecycles a repository has and the Workspaces (sets of repositories) that scope them, read
/// from the places a piece is kept.
pub mod piece_places;
/// Saving and deleting one Workspace, lifecycle, rule or autonomy default in Kit.
pub mod piece_saving;
pub mod workspaces;

#[cfg(any(test, feature = "test-support"))]
pub mod tests;

pub use adrift::Adrift;
pub use budget::CheckBudget;
pub use clock::{Clock, SystemClock};
pub use daemon::{Fittings, Fleet, Host, Reconciled, StartingIn};
pub use detach::Detached;
pub use drone::{environment, HostPaths};
pub use headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Short, Spare, TheMachine};
pub use judging::{CallFailed, Explaining, JudgeBudget};
pub use listener::claimed_listener_port;
pub use mint::{Mint, UlidMint};
pub use peer::PeerOf;
pub use places::{Asking, ChecksAtOnce, Place, Places, Room, OVERTAKEN_AT_MOST};
pub use ports::{detect_ceiling, BindConnectProbe, PortRange, PortsRefused};
pub use process::{holder_of, Holder, ProbeFailed, StartedAt};
pub use redaction::Redactor;
pub use rehearsing::verify_steps;
pub use kit_manifests::{kit_manifest_copy, KitManifest};
pub use run_memory::places::kit_manifest;
pub use runtime::{
    listener_address, machine_path, Presence, PublishError, Published, ReadError, RuntimeFile,
    Staleness, Vacancy, FILE_NAME,
};
pub use seeding::{CopyOnWrite, NotCloned, TheVolume};
pub use slots::Concurrency;
pub use noticing::Noticing;
pub use orphans::keep_ending_orphans;
pub use wire::left_out_workflow;
