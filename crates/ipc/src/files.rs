//! Searching the checkout a request dispatches into, for the `@` mention popup
//! a person opens while typing a request or a brief — `search_files` in
//! `crates/ipc/operations/`.
//!
//! **Named by `?manifest_id=`, not in the body.** A Fleet serves several
//! repositories; an absent one reads the checkout Fleet was started in.
//! `?root=` names a folder Fleet does not serve yet instead, and `?folders=true`
//! asks for its folders rather than its files.

use serde::{Deserialize, Serialize};

/// What was found, narrowed against a person's typed text.
///
/// **Capped, not exhaustive.** `crates/fleet/src/files.rs` stops once it has
/// enough, so a repository nobody has sized cannot turn one keystroke into an
/// unbounded walk. Paths are relative to the repository root, forward-slashed,
/// and never `.git`, `node_modules` or `target` — see that module's own notes
/// for what a fuller answer would still have to settle.
///
/// **`folders` answers `?folders=true`, and then `paths` is empty.** Each is
/// relative to the root, `/`-separated, with no trailing `/`, and the root
/// itself is never one. It is left out where the answer is files, and an older
/// answer without it reads as none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesFound {
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<String>,
}
