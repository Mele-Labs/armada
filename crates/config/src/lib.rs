//! Manifest and WorkflowDef resolution: two files, checked against each other.
//!
//! Owns scan, propose, select and verify, plus Check and Command definitions.
//! A Manifest is an `armada.yml` at a workspace root, version-controlled with
//! the project it configures; the nearest one up the tree owns a path, and the
//! root owns what no Workspace claims. [`watches`](mod@watches) reads the
//! recipes a watch starts from. **Of Kit, only Workflows are read**:
//! [`catalogue`](mod@catalogue) merges them between Armada's and a repository's
//! own (#425). Every other tier is a constant, this Manifest, a step.
//!
//! **Everything either schema holds and this crate does not read is an unknown
//! key, and hard-fails.** [`manifest`](mod@manifest) and
//! [`workflow`](mod@workflow) each name the keys they take. That looks like an
//! unfinished parser and is not: a key nothing reads is a promise the file
//! makes and the system does not keep, so refusing it means each deferred
//! section lands with the code that honours it, and stays additive rather than
//! becoming a migration of every file already written.
//!
//! **Bytes enter here, and JSON does not.** Gate rule five scopes untyped JSON
//! reading to `store` and `ipc`; `armada.yml` and a WorkflowDef are YAML, walked
//! as an untyped value rather than deserialized into a struct —
//! [`yaml`](mod@yaml) holds why a derive could not carry the refusals.
//!
//! **Nothing here reads a clock.** No mtime check, no staleness window, no
//! cache expiry: *when* a Manifest is read again is `armada::watching`'s, and
//! [`live`](mod@live) says only which keys a re-read may move.

/// Editing `armada.yml` a key at a time, every other byte left where it was.
mod amending;
mod applying;
mod catalogue;
mod error;
mod fields;
mod gating;
mod judge;
mod lifecycle;
mod live;
mod loops;
mod manifest;
mod piece;
mod resolve;
mod resolving;
mod rule;
/// The review tools a pull request is asked with, from Armada, Kit and the repository.
pub mod review_tools;
mod roster;
mod runners;
mod scope;
/// The Machine's settings: `settings.json`'s keys, checked and resolved.
pub mod settings;
mod watches;
mod widening;
mod workflow;
/// A Workspace means a set of repositories here, and not the Manifest's monorepo unit.
mod workspace;
mod yaml;

#[cfg(test)]
mod tests;

pub use amending::{
    amend, splice, Amended, CheckEdit, CommandEdit, Edit, EvidenceEdit, NewCheck, NewCommand, NewEvidence,
    NewLink, NewNarrowing, NewPort, NewRun, NewRunner, NotAmended, PortEdit, RunEdit, Unplaceable,
};
pub use applying::{LifecycleFile, Lifecycles, Rank, Reach};
pub use catalogue::{
    carried, fit, Catalogue, LeftOut, ResolvedCatalogue, Unfit, WhyLeftOut, WorkflowFile, Written,
    CARRIED_AT,
};
pub use error::{Disagreement, Fault, LoadError, Refusal, ResolveError, UnknownCheck};
pub use fields::{workflow_fields, Field};
pub use gating::{gating, owner, Gate, GateWhy};
pub use lifecycle::{parse as parse_lifecycle, LifecycleError};
pub use live::{Adopted, Frozen, LiveKey, Moved, Reloads};
pub use piece::{LeftOut as PieceLeftOut, Written as PieceWritten};
pub use rule::{parse as parse_rule, RuleError, RuleFile, Rules};
pub use manifest::{
    AppRun, BadSeedPath, Check, Command, FromKit, Harness, kit_copy_fault, manifest_run_lines, KitCopy, KitFile, KitKind, KitName, KitRead, Link, Manifest, MergeBy,
    Preparation, Seed, Server, TrackerBlock, DEFAULT_CI_GATE,
};
pub use resolve::{ResolvedWorkflow, DEFAULT_ITERATION_CAP};
pub use roster::Roster;
pub use runners::{detected, shipped, RunnerDescription};
pub use watches::{
    parse as parse_watch_recipe, written as watch_recipe_written, LeftOut as WatchRecipeLeftOut,
    Recipe as WatchRecipe, RecipeSource as WatchRecipeSource, Recipes as WatchRecipes,
    ResolvedRecipe as ResolvedWatchRecipe, WhyLeftOut as WhyWatchRecipeLeftOut,
    Written as WatchRecipeWritten,
};
pub use widening::with_manifests_added;
pub use workflow::{head_of, names_no_lifecycle, EndFault, MechanicalCheck, NamesNoLifecycle, Step, WorkflowDef, WorkflowHead};
pub use workspace::{parse_workspace, WorkspaceError, WorkspaceFile, Workspaces};

// Re-exported, not re-declared. A Job carries its resolved workflow, so these
// are spelled in `core-model` where the record is — and every caller that
// already said `config::ResolvedStep` still means the same type. `GateVerdict`
// joined them when a step's routing became something a Job freezes.
pub use core_model::{
    AdvanceGate, ContextSource, Covers, DeclarePlanAt, DeclaredPaths, EvidenceScope, EvidenceType,
    FrozenWorkflow, GateVerdict, JudgeCheck, JudgeCriterion, PathPattern, ResolvedCheck,
    ResolvedStep, WorkflowSource,
};
