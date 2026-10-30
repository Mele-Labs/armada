//! What a piece file declares, and which files in a folder declare it.
//!
//! **A piece is (kind, id, scope), found by what its file says and not by its name.** `ticket` for
//! every repository and `ticket` within one repository are two files that rank against each other,
//! and a file may be called anything the folder's extensions allow. A file that does not parse
//! declares nothing, which is also how the read side treats it: it is left out.

use std::path::{Path, PathBuf};

use core_model::LifecycleScope;
use ipc::{LifecyclePieceKind, PieceScope};

use crate::piece_places::{files_in, AUTONOMY, LIFECYCLES, RULES, WORKSPACES};

/// The folder a kind of piece is kept in, in Kit and under a repository's `.armada/`.
pub(crate) fn folder_of(kind: LifecyclePieceKind) -> &'static str {
    match kind {
        LifecyclePieceKind::Lifecycle => LIFECYCLES,
        LifecyclePieceKind::Rule => RULES,
        LifecyclePieceKind::Workspace => WORKSPACES,
        LifecyclePieceKind::Autonomy => AUTONOMY,
    }
}

/// What the person reads a kind of piece as.
fn word_of(kind: LifecyclePieceKind) -> &'static str {
    match kind {
        LifecyclePieceKind::Lifecycle => "lifecycle",
        LifecyclePieceKind::Rule => "rule",
        LifecyclePieceKind::Workspace => "Workspace",
        LifecyclePieceKind::Autonomy => "autonomy defaults",
    }
}

fn scope_of(scope: &LifecycleScope) -> PieceScope {
    match scope {
        LifecycleScope::Everywhere => PieceScope::Everywhere,
        LifecycleScope::Within(id) => PieceScope::Within { id: id.as_str().to_string() },
    }
}

/// One piece: its kind, its id, and where it applies. **A Workspace has no scope**, so it is always
/// `Everywhere` here and a scope asked for with one is dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub kind: LifecyclePieceKind,
    pub id: String,
    pub scope: PieceScope,
}

impl Identity {
    pub(crate) fn of(kind: LifecyclePieceKind, id: &str, scope: PieceScope) -> Identity {
        let scope = match (kind, scope) {
            (LifecyclePieceKind::Workspace, _) => PieceScope::Everywhere,
            (_, PieceScope::Within { id }) => PieceScope::Within { id: id.trim().to_string() },
            (_, everywhere) => everywhere,
        };
        Identity { kind, id: id.trim().to_string(), scope }
    }

    /// `rule `ticket_to_review` within `my-store``, as a sentence names it.
    pub(crate) fn said(&self) -> String {
        let word = word_of(self.kind);
        match &self.scope {
            PieceScope::Everywhere => format!("{word} `{}`", self.id),
            PieceScope::Within { id } => format!("{word} `{}` within `{id}`", self.id),
        }
    }

    /// `rule ticket_to_review within my-store`, as a commit's message names it.
    pub(crate) fn named(&self) -> String {
        let word = word_of(self.kind);
        match &self.scope {
            PieceScope::Everywhere => format!("{word} {}", self.id),
            PieceScope::Within { id } => format!("{word} {} within {id}", self.id),
        }
    }

    /// The file name a new file for it takes: `<id>.yml`, and `<id>.<scope>.yml` within a scope.
    /// **Anything but letters, digits, `-` and `_` becomes `-`**, since a Workspace's id and a scope
    /// are any text and a file's name is not.
    pub(crate) fn file_stem(&self) -> String {
        let plain = |text: &str| -> String {
            text.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect()
        };
        match &self.scope {
            PieceScope::Everywhere => plain(&self.id),
            PieceScope::Within { id } => format!("{}.{}", plain(&self.id), plain(id)),
        }
    }
}

/// What the text of a file declares as `kind`, or the parser's own sentence for why it declares
/// nothing: every fault of the file, with its key. `path` is only named in that sentence.
pub(crate) fn read(kind: LifecyclePieceKind, path: &Path, text: &str) -> Result<Identity, String> {
    let said = |why: &dyn std::fmt::Display| why.to_string();
    match kind {
        LifecyclePieceKind::Lifecycle => config::parse_lifecycle(path, text)
            .map(|one| Identity::of(kind, one.id().as_str(), scope_of(one.scope())))
            .map_err(|why| said(&why)),
        LifecyclePieceKind::Rule => config::parse_rule(path, text)
            .map(|one| Identity::of(kind, one.id().as_str(), scope_of(one.scope())))
            .map_err(|why| said(&why)),
        LifecyclePieceKind::Workspace => config::parse_workspace(path, text)
            .map(|one| Identity::of(kind, one.id().as_str(), PieceScope::Everywhere))
            .map_err(|why| said(&why)),
        LifecyclePieceKind::Autonomy => config::parse_autonomy(path, text)
            .map(|one| Identity::of(kind, one.id().as_str(), scope_of(one.scope())))
            .map_err(|why| said(&why)),
    }
}

/// Every file in `folder` that declares `wanted`, with its text, in the order the folder lists them.
pub(crate) fn declaring(folder: &Path, wanted: &Identity) -> Vec<(PathBuf, String)> {
    files_in(folder, config::PieceWritten::in_kit)
        .into_iter()
        .filter(|file| read(wanted.kind, file.path(), file.text()).is_ok_and(|declared| &declared == wanted))
        .map(|file| (file.path().to_path_buf(), file.text().to_string()))
        .collect()
}
