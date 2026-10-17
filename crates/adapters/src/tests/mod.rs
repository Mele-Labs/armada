//! The cases that need git's own opinion, and the ones that need no process at
//! all.
//!
//! See [`repo`] for why the version-control cases run against a real repository
//! while everything above this crate runs against a fake.
//!
//! `harness` and `transcript` are the other half and are the opposite shape:
//! **nothing there starts a process.** What a Drone is confined to is a value
//! the harness renders, and what a Drone said is a line somebody already
//! captured — so the whole confinement posture is asserted without a
//! credential, a network, or an agent.

mod background_task;
mod basing;
mod build_standing;
mod deleting_merged_branch;
mod catching_up_to_the_remote;
mod check_line;
mod checkout_send;
mod ci_workflows;
mod clash;
mod cloning;
mod commit;
mod concurrently;
mod conversing;
mod delivery;
mod edited;
mod existing_setup;
mod filing;
mod forge_address;
mod git_guard_gaps;
mod harness;
mod held_off;
mod hosted_session;
mod issue_lookup;
mod jira;
mod judge;
mod keeping_current;
mod landing;
mod leasing;
mod leasing_existing_branch;
mod leasing_history;
mod leasing_jobs;
mod leasing_review;
mod slot_watching;
mod subagent_lines;
mod leasing_last_session;
mod leasing_parking;
mod leasing_reclaiming;
mod leasing_rescue;
mod leasing_sessions;
mod leasing_settled;
mod leasing_shape;
mod leasing_trim;
mod manifest_edit;
mod mcp;
mod merging_after_a_kill;
mod merging_in;
mod merging_by_push;
mod pull_request_detail;
mod pull_request_facts;
mod pull_request_patch;
mod pull_request_review;
mod pull_request_threads;
mod pull_review_lists;
mod reading_in;
mod reclaim;
mod remembering;
mod repairing;
pub mod repo;
mod rerunning;
mod scouting;
mod snapshot;
mod standing;
mod sign_in;
mod store;
mod store_vector;
mod territory;
mod terminal_follow;
mod terminal_thread;
mod transcript;
mod transcript_acts;
mod trigger_files;
mod under_review;
mod work_product;
mod worktree;
