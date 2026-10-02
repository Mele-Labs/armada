//! `armada land` — the merge line, ported from `scripts/land`.
//! `docs/capabilities/merge-line.md` is the design; `docs/practices/running-locally.md`,
//! *Landing a branch*, is how it is run.
//!
//! Stages 1-3 are the state kept on disk, the pure gate comparisons, and the
//! infrastructure a turn runs on. This stage is the orchestration that
//! calls all of it and the CLI verb that reaches it — start at
//! [`preflight::preflight`], [`enqueue::land`], [`status::status`] and
//! [`runner_loop::run_runner`], then [`turn::take_turn`] and [`gating::foundations`]
//! for what a turn does.
//!
//! State stays JSON, decoded and encoded only through
//! [`ipc::decode`]/[`ipc::encode`] (gate rule five) — [`shell::gh_view`]
//! reads `gh`'s own JSON answer through the same doorway. A turn ends in
//! [`onto_main`], which pushes the base itself.

// The state on disk is `adapters`', so Fleet reads the same files with the
// same types without depending on this binary. `docs/capabilities/merge-line.md`.
pub use adapters::land_state::{codec, dir, outcome, queue};

mod armada_cli;
mod batch;
mod blame;
mod caches;
mod enqueue;
pub mod env;
pub mod gate;
mod gating;
pub mod git;
pub mod lock;
mod logs;
mod merge_in;
mod onto_main;
mod preflight;
mod prepare;
mod repo;
pub mod runner;
mod runner_loop;
mod say;
mod shell;
pub mod stamp;
mod status;
mod stop;
mod turn;
mod withdraw;
pub mod worktree;

pub use codec::{ReadStateError, WriteStateError};
pub use dir::{key, StateDir, StateDirError};
pub use enqueue::{land, Queued};
pub use env::Env;
pub use gate::{
    a_report, already_red_on_base, failing_lines, finding_counts, foundations_delta, not_installed,
    FoundationsComparison,
};
pub use git::{GitFailed, GitFailure};
pub use lock::{LockError, TurnLock};
pub use outcome::{
    merge_outcome, read_outcome, MergeOutcomeError, Outcome, OutcomePatch, OutcomeState, Place,
};
pub use preflight::{preflight, Preflighted};
pub use queue::{nonce, queued, read_queue_entry, write_queue_entry, QueueEntry, QueuedError};
pub use runner::{ensure_runner, spawn_detached, EnsureRunnerError};
pub use runner_loop::run_runner;
pub use stamp::{read_stamp, write_stamp, PreflightStamp};
pub use status::{status, UNKNOWN};
pub use stop::Refused;
pub use withdraw::{withdraw, Withdrawn};
pub use worktree::{
    drop_worktree, land_root, main_tree, reused, reused_keeping, LandRootError, LandWorktree,
    LogError, MainTreeError, ReuseError,
};
