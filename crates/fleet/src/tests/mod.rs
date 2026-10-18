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
mod agent_starts;
mod session_after;
mod asking_in_window;
mod sketching;
mod building;
mod checkouts_apart;
mod daemon;
mod declaring;
mod detach;
mod door_per_repository;
mod drifting;
mod editing;
mod helm_session;
mod helm_routing;
mod helm_telling;
mod helm_deciding;
mod helm_door;
mod helm_permission;
mod helm_record;
mod helm_studio;
mod host;
mod http;
mod flight_controls;
mod kit_allowlist;
mod listener;
mod manifest_proposals;
mod mods;
mod tracker_per_manifest;
mod orphans;
mod session_limit;
mod peer;
mod ports;
mod preferences;
mod process;
mod reading_a_request;
mod reading_in;
// The rest are `mods.inc`, one a file here: `build.rs` writes it.
include!("mods.inc");
mod serving;
mod session_auto;
mod session_auto_reads;
mod session_reach_folders;
mod session_background;
mod session_background_work;
mod session_bays;
mod session_fork;
mod session_handoff;
mod session_ending_slots;
mod session_release_holds;
mod session_release_slot;
mod session_gate;
mod session_gate_leases;
mod session_activity;
mod session_annotation_done;
mod session_done;
mod session_finish_up;
mod session_host;
mod session_plan;
mod session_proposal;
mod session_rail;
mod session_pause;
mod session_queue;
mod session_slot_asking;
mod session_slot_choice;
mod session_question;
mod reply_asks;
mod session_standing;
pub(crate) mod slot_leases;
mod slot_reclaim;
mod session_waiting;
mod sleeping;
mod session_running;
mod session_shell;
mod sessioning;
mod loadout;
mod loadout_item;
mod turn_ingest;
mod usage_watch;
mod turn_usage;
mod session_notes;
mod terminal_session;
mod walk_question;
mod walk_steps;
mod worktrees_kept;
mod studio_pictures;
mod studio_runs;
mod studio_sketches;
mod studios;
// `pub(crate)`, not `mod`: `crate::records::migrating`'s own tests are not a
// descendant of this module and need the same temporary directory every
// fixture here already uses, rather than a second one invented beside it.
pub(crate) mod tmp;
mod bay_history;
mod verify_runs;
mod watching;
mod workflow_promise;
mod workspace_loading;
mod workspace_ports;
mod workspace_runs;
mod workspace_verify;
mod getting_an_issue;
mod mention_lines;
