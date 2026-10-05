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

mod basing;
mod ci_workflows;
mod cloning;
mod commit;
mod conversing;
mod delivery;
mod edited;
mod existing_setup;
mod filing;
mod git_guard_gaps;
mod harness;
mod held_off;
mod issue_lookup;
mod judge;
mod keeping_current;
mod landing;
mod leasing;
mod leasing_jobs;
mod leasing_parking;
mod leasing_rescue;
mod leasing_shape;
mod mcp;
mod merging_after_a_kill;
mod merging_by_push;
mod reading_in;
mod reclaim;
mod remembering;
pub mod repo;
mod rerunning;
mod scouting;
mod snapshot;
mod standing;
mod transcript;
mod under_review;
mod work_product;
mod worktree;
