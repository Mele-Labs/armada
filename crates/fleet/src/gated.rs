//! A repository's manifests read against one change: which of them gate it,
//! what each owns of it, and what each came to.
//!
//! **Only where the repository has workspaces.** One with none has no
//! [`Gated`], and a Check there runs by its own `when` as it always did. Where
//! workspaces exist the changed paths are read, never `Job::gate_manifests`
//! alone, and an empty diff gates nothing. `docs/concepts/manifest.md`,
//! *Workspace gating*.

