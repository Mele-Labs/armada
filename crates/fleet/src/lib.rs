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
mod amending;
/// A repository working its own backlog. #2092.
pub mod autopilot;
/// Saving a workflow definition, and reading the workflow folders again.
mod authoring;
pub mod basing;
/// What a Session has built: the rules between what Fleet observed and the record on its row.
mod build_facts;
mod build_record;
/// Which URLs and issues a Studio or Session names, looked up for what they are.
mod enriching;
/// Where the build record meets the store.
mod build_keeping;
/// Which files a Session and its subagents touched, read off the tool calls Fleet hears.
mod file_touching;
/// A Session's changes marked by the person, checked against its plan and worded for the agent.
mod plan_marks;
/// A Session's plan: versions, the diff between them, task progress and groups worked.
mod plan_record;
/// Where a Session's plan meets the store.
mod plan_keeping;
/// Where a Session's Loadout meets the store.
mod loadout_keeping;
/// Where a Session's turn usage meets the store.
mod turn_ingest;
mod usage_watch;
pub mod usage_rules;
mod usage_summarising;
mod turn_usage_keeping;
mod loadout_reading;
/// A Session's place in its workflow, read from what Fleet observed, and the nudge for a step skipped.
mod rail;
/// Where the rail and the judges meet the store, and a concern as a waiting item.
mod rail_keeping;
/// What Fleet does when a Session's step moves or shows a step skipped.
pub mod linked_watching;
pub mod after_watching;
pub mod rail_watching;
/// Telling sessions the build Fleet and Bridge now run, once per build.
mod build_notices;
/// The build Fleet runs on and the detached restart onto another. `scripts/restart-build`.
mod building;
/// How long a Check may run.
mod budget;
/// The served checkout caught up whenever main moves.
mod behind_bay;
mod catching_up;
mod check_output;
mod checkouts;
/// Who wrote the same file as whom, and telling both.
mod clashing;
pub mod clock;
/// `api::Commands`, implemented over a real Fleet — the write half of the seam
/// `serving` holds the read half of. Three traits, three impl blocks, three
/// files, and no delegating signature between them.
pub mod commanding;
/// A copy of a person's own piece committed by a Job of its own: `commit_to_repository`.
mod committing;
mod configured;
pub mod daemon;
pub mod detach;
/// Whether the repository still has what `armada.yml` names. **A read of the
/// repository**, where `daemon::rereading` is a read of the file — a `run`
/// line naming a deleted script parses perfectly and says nothing.
mod drifting;
pub mod drone;
/// The Manifest file itself, read and written — the half of Journey 9's
/// *Editing* a person acts with. Save stops at the bytes.
mod editing;
/// Fetching a Job's base from the remote before a catch-up reads it.
mod fetching_the_base;
/// Walking the checkout for `search_files`, the `@` mention popup's read.
mod files;
/// A running Check's log, read for `observe_check_output` as it grows.
mod following;
mod framing;
mod group;
pub mod headroom;
pub mod helm;
pub mod judging;
/// The Cockpit's flight controls a person arranged, in `~/.armada`. `docs/concepts/kit.md`.
mod flight_controls;
/// Kit's MCP servers, resolved for one Manifest. `docs/concepts/kit.md`, `#1275`.
pub mod kit;
/// Fleet's local language model, shared by every reader.
pub mod language_model;
/// Kit's allowlist, in `~/.armada`. `docs/concepts/kit.md`.
pub mod kit_allowlist;
/// Kit's Add and Remove for a plugin or a skill, across the seam.
pub mod kit_pieces;
/// A Job's worktree is the pool slot it leased.
mod leasing;
/// Requests: an HTTP request filled from a repository's environments, and its collections.
pub mod requesting;
/// The Drones-at-once bound, memory share and disk floor a person saves, and
/// how a save reaches admission without a restart.
pub mod limits;
pub mod listener;
/// Main's CI, read off the forge on an interval.
pub mod main_ci;
mod main_fix;
pub(crate) mod main_hub;
/// A possible `armada.yml` per workspace, from Scan, and the Write that ends it.
pub mod manifest_proposal;
pub mod merge_lines;
mod model_downshift;
mod model_downshifting;
/// The models the harness offers, asked after Fleet is serving.
pub mod models_read;
pub mod phone;
/// Mods: folders on this machine that change how Bridge looks.
pub mod mods;
pub mod mint;
/// What Fleet saw happen, kept for the Cockpit's timeline. Since 23.82.
pub mod timeline;
/// Who held one slot and what was checked out in it, for the Cockpit's Bay pane.
mod bay_history;
/// The lines of code a pinned note's spot was drawn from, found by the words on screen.
pub mod note_locating;
pub mod notice_loop;
pub mod noticing;
/// Drone groups whose Fleet is gone, found by the mark Fleet put on them.
pub mod orphans;
mod session_limit;
/// What a Session's proposer is asked and how its answer reads, with the words the Job proposer shares.
mod session_proposer;
pub mod peer;
/// A form's edit sent as a pull request, pending until the checkout has it.
mod pending_edit;
/// The machine's places for Checks, one line for every Job and repository. #1063.
pub mod places;
pub mod ports;
/// A person's Bridge preferences, five keys of settings.json in their older shape.
mod preferences;
pub mod preparing;
/// The probes Fleet can run on itself, and the Doctor modules it cannot.
/// **Not Doctor**, whose grid is ten modules and is not built.
mod probing;
pub mod process;
/// The profiles a machine has joined, each with a directory of its own for its key.
pub mod profiles;
/// Starting a scout on a Studio, recording what it read, and stopping it.
mod promoting;
/// The prompts a person may override in settings.json: what ships, and what is in force.
pub mod prompts;
/// What happened to a pull request, told to the Sessions and Jobs that own it.
mod pull_notices;
mod pull_waits;
/// A pull request nobody reported, attached to the Session that worked its branch.
mod claiming;
mod pull_attaching;
/// Whose pull requests Fleet watches: the signed-in account's and the forge command line's.
mod my_pulls;
/// Who owns a pull request, from the Sessions ledger and the Jobs.
mod pull_owners;
/// Which Session or Job claimed a pull request, kept in the central store for the other machines.
mod pull_registry;
/// What a pull request's panel reads when it opens, and its comment box writes.
/// Since 23.83.
mod pull_request_panel;
mod pull_review_lists;
/// The agent runs about a pull request, kept per commit. `docs/concepts/pull-review.md`.
mod review_runs;
/// Natural Search: the person's words put to every open pull request. `docs/concepts/pull-review.md`.
mod review_searches;
/// The review tools and explanations a repository has, and writing one.
mod review_tools;
/// A Walk run: where the change runs, and the walk played there. `docs/concepts/pull-review.md`, *Walk*.
mod review_walk_runs;
/// A pull request's change held up for a walk: a bay, a server, and giving them back.
mod review_walks;
/// Where a pull request's change runs, and the places Fleet remembers.
mod walk_places;
/// A pull request by repository and number, and the acts a Session takes on one.
/// Since 23.48.
mod pull_requesting;
/// A person's refresh of one pull request: everything the pull watch does for it, now.
mod pull_refreshing;
/// The issues a Session works on: named by the person, or closed by its pull requests.
mod session_issues;
/// Reading a Link in: the source, fetched by Fleet. `#1293`.
mod reading_in;
/// Links already on a Studio, read as what their addresses name — `#1394`.
mod recognising;
/// Where one repository's records live, off the checkout — the per-repository
/// key and the one-time move of what an older Fleet wrote under it.
pub mod records;
pub mod redaction;
mod refusing;
/// A person's run of one Manifest entry in a Job's worktree, and Undo from
/// the snapshot taken before it. **A rehearsal, never a verdict.**
mod rehearsing;
/// The repositories one Fleet serves, and adding one by folder.
pub mod repositories;
/// Which profile a repository claims and syncs as: matched by its owner, or chosen.
pub mod repository_profiles;
mod rescuing;
mod stranding;
/// A Job's retro: what got in the way while it ran. `docs/concepts/retro.md`.
mod retro;
mod review;
pub mod runtime;
mod saving;
pub mod saying;
/// Scan: reading a repository nobody set up for Armada. **It writes nothing,
/// because a [`scanning::Tree`] has no write on it.**
pub mod scanning;
mod scoping;
/// The read-only agent a person starts from a Studio. `#1292`.
pub mod scout;
mod scouting;
pub mod seeding;
/// A held checkout's uncommitted files sent up as a pull request.
mod sending_checkout;
mod servers;
pub mod serving;
/// The session ledger: what a harness says of an agent session. `docs/concepts/session.md`.
pub mod session_host;
mod session_check_live;
/// A person's own `armada.yml` for a repository, at the place its tiles are kept. `docs/concepts/manifest.md`.
pub mod kit_manifests;
/// How a repository runs, remembered from what its Jobs did. `docs/concepts/kit.md`.
mod machine_note;
mod run_memory;
mod notice_mutes;
/// Folders and runs read from a person's agent CLI, and installing the mod into it.
mod session_import;
mod session_checking;
mod session_running;
/// A shell action run from a flight control, in the slot a Session holds.
mod session_shell;
/// Pieces made into a package, read, and added to the person's own Kit.
mod sharing;
mod sessioning;
mod window_sign_in;
mod slot_holders;
mod slot_reclaiming;
/// A red main claimed in the central store, and the claim read back for the hub.
pub mod red_claim;
/// A red main whose pull request a Session merged is claimed for that Session, or for a fork of it.
mod session_red;
mod taking_red;
/// Signing in to an account with the forge's device flow, and enrolling the machine with the store.
pub mod signing_in;
pub mod slots;
mod sleeping;
/// The lock on the store, which names a holder past a quarter second.
mod store_lock;
/// The test map: collectors as plugs, laps written as edges, the map read for paths.
/// `docs/concepts/test-map.md`.
pub mod test_map;
/// Fleet feeding the test map from main moving and from a sleeping bay.
mod test_map_feed;
/// The laps main's CI run recorded, fetched from the forge and kept as edges.
mod test_map_ci;
/// A Picture a person pastes onto a Studio, kept as a Note's frame is.
mod studio_pictures;
/// Runs a Studio holds, and what a node keeps of one past retention. `#1289`.
pub mod studio_runs;
/// Servers a Studio holds: started from one, and kept when they end. `#1345`.
pub mod studio_servers;
mod studio_sketches;
/// A repository's Studios, through the store and onto the stream. `#1285`.
mod studios;
mod subagents_resumed;
pub use worktrees_kept::keep_worktrees_warm;
/// settings.json: the file, its watch's re-read, its save, and the move of the
/// store's saved rows into it. `docs/concepts/machine.md`, *settings.json*.
pub mod settings;
pub mod sweeping;
/// What each served repository holds and where each agent has changed it, for the deck's map.
mod territory;
/// A repository's open issues, read from its tracker. #91.
mod tracking;
pub mod transcript;
mod trigger_authoring;
mod trigger_wire;
/// The live dials settings.json moves, shipped and overlaid by saved.
pub mod tuning;
mod worktrees_kept;
/// The redactions the `Queries` and `Commands` impls call by hand. Split out to
/// keep those files, rather than their helpers, the thing that grows.
mod wire;
pub mod workspaces;

#[cfg(test)]
mod tests;

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
pub use trigger_wire::{trigger_definition, trigger_list};
pub use wire::left_out_workflow;
