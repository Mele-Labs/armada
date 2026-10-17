//! Which manifests gate a Job, as `Job::gate_manifests` records them.
//!
//! **A repository with no workspaces records none.** The root gates every Job
//! there and always has, so an empty list means "the root alone", and a Job
//! written before workspaces gated reads the same. Where the repository has
//! workspaces the list names every manifest that gates, the root included.
//!
//! **An empty set of paths gates nothing**, decided 6 Oct 2026: a Job that
//! changed no files runs no Checks. `config::gating` answers that; nothing here
//! hands it to `Covers::reach`, which reads no paths as always.
//! `docs/concepts/manifest.md`, *Workspace gating*.

