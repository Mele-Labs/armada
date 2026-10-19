//! Everything that talks to something outside Armada.
//!
//! The agent CLI, git2, the keychain, the model client — and the health probes
//! for each, because a probe is the smallest instance of what this crate already
//! owns. git2 is here for worktrees regardless, so asking whether git works adds
//! no dependency.
//!
//! **This is the only crate permitted to name a vendor.** Outside it, a vendor's
//! name is the boundary having leaked, and the gate refuses it.
//!
//! A dedicated health crate was rejected: no measurement backs the seam, and
//! grouping by surface rather than by capability is the shape that grew v1's
//! core to 38,470 lines.

//! # What is built so far
//!
//! Version control ([`GitVcs`], the work-product reader beside it,
//! [`reclaim`] — which deletes branches, and is the
//! only thing in the workspace that can — and delivery, which
//! brings a finished Job's branch up to its base, pushes it and opens it for
//! review), and the agent harness: [`HeadlessAgent`] renders a confined Drone into a program and
//! an argument list, [`transcript`](mod@transcript) reads what that Drone says
//! back, and [`mcp`](mod@mcp) writes the one-server configuration file the two
//! flags that confine it point at — and, in the repository rather than beside
//! the store, the one entry that tells an agent working there how to reach
//! Fleet at all.
//!
//! And the Judge: [`judge`](mod@judge) renders the same CLI as a one-shot call
//! — one turn, no tools, no MCP server and no directory — which is how the
//! semantic tier reaches a model without reaching a session.
//!
//! **The harness does not spawn.** Starting a process, detached, is `fleet`'s —
//! see `adapter_traits::AgentHarness` for the three things that split buys, the
//! first of which is that every confinement property here is a value a test
//! reads rather than a process a test has to start.
//!
//! [`IssueLookup`] is the same split again: it renders the call that
//! would resolve a bare issue link, and `fleet` is what has a network to run
//! it with.

mod acted;
pub mod agent_mod;
mod annotations;
mod background;
mod base;
mod basing;
mod build_standing;
mod checkout_send;
mod ci_workflows;
pub mod clash;
mod cloning;
mod commit;
mod concurrently;
/// A Helm conversation's process: one message, resumed by id.
mod conversing;
mod delivery;
mod error;
/// The setup a person already has, read to be shown. `#1491`.
mod existing_setup;
mod fetching_the_base;
mod filing;
/// Where a repository lives on its forge, and what a request's links name.
mod forge_address;
mod harness;
/// A session Bridge hosts: a live process, resumed by id. Since 23.49.
mod hosted_session;
mod inline_comments;
mod issue_lookup;
/// Reading a repository's open issues and writing back on one. #91.
mod issue_tracker;
/// The keys, addresses and filters of a Manifest's Jira `tracker:` block. #91.
mod jira;
mod jira_lookup;
mod jira_tracker;
mod judge;
mod keeping_current;
mod landing;
/// The pool of warm worktrees a repository leases out, shared by agents.
pub mod leasing;
mod main_ci;
mod manifest_edit;
mod mcp;
mod merge_queue;
mod merging_in;
mod merging_the_base_in;
/// The step onto a base, used by `merge_by: push`.
pub mod onto_base;
/// Which files a person's commits changed. `docs/concepts/pull-review.md`.
mod history;
mod outside;
mod pull_request_detail;
mod pull_request_diff;
mod pull_request_facts;
mod pull_request_patch;
mod pull_request_review;
mod pull_request_threads;
mod pull_request_writes;
mod pull_review_lists;
mod pull_watch;
/// What a Link's address names, and the call that fetches it. `#1293`.
mod reading_in;
mod reclaim;
mod remembering;
mod rerunning;
/// A scout's process: one ask, read tools only, inside one checkout.
mod scouting;
pub mod session_folders;
pub mod session_runs;
/// Signing in with the forge's device flow.
mod sign_in;
/// The tracked files of a checkout that hold some words, for a pinned note.
pub mod source_search;
/// A worktree before a person's run, and putting back what the run changed.
pub mod snapshot;
/// The central store over HTTP, signed with this machine's key.
mod store;
mod store_device;
mod store_signing;
/// The directories of a repository's base and the paths an agent has changed in a slot.
pub mod territory;
/// A terminal session's thread, drawn from the transcript its agent CLI keeps.
pub mod terminal_thread;
pub mod turn_lines;
mod tracker_providers;
mod subagent_lines;
mod transcript;
/// The repository's Trigger files as `main` holds them.
mod trigger_files;
mod under_review;
mod watching;
mod work_product;
mod worktree;

#[cfg(test)]
mod tests;

pub use annotations::annotation_places;
pub use build_standing::{branch_in_build, fetch_branch, fetch_main, position_against_main, BranchInBuild};
pub use ci_workflows::ActionsWorkflows;
pub use concurrently::concurrently;
pub use conversing::{
    door_tools, path_written, wrote_the_checkout, ConversationRefused, Conversing,
    CHANGES_THE_CHECKOUT,
};
pub use error::{CommitWorkError, CreateWorktreeError, ReadWorkProductError};
pub use existing_setup::{ExistingSetup, Home};
pub use forge_address::{linked_in, remote_of, ForgeAddress};
pub use harness::HeadlessAgent;
pub use hosted_session::{check_line, reads_only, CheckLine, HostedLaunch, HostedRefused, HOSTED_HARNESS, MOD_INSTALLED_MANIFEST, MOD_MANIFEST, SESSION_RULES};
pub use issue_lookup::IssueLookup;
pub use outside::command_reach;
pub use issue_tracker::IssueTracker;
pub use tracker_providers::TrackersByProvider;
pub use mcp::{publish_the_agents_door, Published, REPOSITORY_CONFIG};
pub use reading_in::{
    bounded, fetching, forge_facts, forge_node, milestone_read, source_of, text_of_a_page,
    text_of_a_session, AnIssue, Fetch, MilestoneRead, Source, StaysALink, Taken, FETCH_SECONDS,
    FORGE_HOST, MOST_ISSUES, SESSIONS,
};
pub use remembering::{personal_settings, remember_the_rule, Remembered, PERSONAL_SETTINGS};
pub use scouting::{
    checkout_as_it_stands, denied_to_a_scout, files_a_search_showed, no_servers, CheckoutRead,
    Looked, ScoutRefused, Scouting, Shown,
};
pub use subagent_lines::{subagent_calls, subagent_parent, SubagentCall};
pub use transcript::{background_form, init_commands, inside_a_subagent, sent_message, viewed_image, written_document};
pub use trigger_files::triggers_on_base;
pub use sign_in::ForgeSignIn;
pub use store::HttpStore;
pub use store_device::{device_id, remember_device, DeviceKey, ID_FILE, KEY_FILE};
pub use worktree::GitVcs;
