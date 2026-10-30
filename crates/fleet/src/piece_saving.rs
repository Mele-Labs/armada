//! `save_lifecycle_piece` and `delete_lifecycle_piece`: one Workspace, lifecycle, rule or autonomy
//! default, written in Kit's folder for it or taken out of it. The repository's own copy is
//! `commit_to_repository`'s, which goes by a Job and a pull request.
//!
//! **Fleet writes the file syntax; Bridge sends the DTO it read.** [`text`] turns the DTO into the
//! text of a piece file, and **the piece's own parser reads that text before anything is written**,
//! so a piece that is saved is one that loads, and a refusal is the parser's own sentence with every
//! fault of the piece. Nothing is created on the way to a refusal, not the folder and not a
//! temporary file.
//!
//! **Found by what a file declares, never by its name.** [`identity`] says why. A save replaces the
//! file that declares the kind, id and scope, in place, and only with `overwrite`; a new file is
//! named for the piece and never takes the name of a file that declares something else.
//!
//! **Read when asked, never cached**, as `piece_places` reads: the next read sees what was written.
//! Nothing outside Kit's folder is touched.

pub(crate) mod identity;
mod text;

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved,
    SaveLifecyclePiece, WireError, WireValue,
};

use self::identity::{declaring, folder_of, read, Identity};
use crate::daemon::Fleet;

/// The piece does not parse. **A 422 carrying the parser's own sentence**, and nothing is written.
pub(crate) const PIECE_UNFIT: &str = "fleet.piece_unfit";
/// Kit already holds the kind, id and scope and `overwrite` was not set. A 422 naming the file.
pub(crate) const PIECE_EXISTS: &str = "fleet.piece_exists";
/// Kit holds no piece of the kind, id and scope a delete named. A 422.
pub(crate) const PIECE_NOT_KEPT: &str = "fleet.piece_not_kept";
/// The folder or the file could not be written or removed. A 500.
pub(crate) const PIECE_UNWRITABLE: &str = "fleet.piece_unwritable";

/// Write `text` to `<path>.writing` and rename it over `path`, so a read sees the old file or the
/// new and never half of one. The staged name has no extension a read takes.
fn write_over(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut staged = path.as_os_str().to_owned();
    staged.push(".writing");
    let staged = PathBuf::from(staged);
    std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, path))
}

/// The kind of a piece and the text of the file a save would write for it.
pub(crate) fn written_as(piece: &LifecyclePiece) -> (LifecyclePieceKind, Result<String, String>) {
    match piece {
        LifecyclePiece::Lifecycle { lifecycle } => (LifecyclePieceKind::Lifecycle, text::lifecycle(lifecycle)),
        LifecyclePiece::Rule { rule } => (LifecyclePieceKind::Rule, text::rule(rule)),
        LifecyclePiece::Workspace { workspace } => (LifecyclePieceKind::Workspace, text::workspace(workspace)),
        LifecyclePiece::Autonomy { autonomy } => (LifecyclePieceKind::Autonomy, text::autonomy(autonomy)),
    }
}

/// A name in `folder` that no file holds: `<stem>.yml`, else `<stem>-2.yml` and on.
fn free_name(folder: &Path, declared: &Identity) -> PathBuf {
    let stem = declared.file_stem();
    let mut named = folder.join(format!("{stem}.yml"));
    let mut n = 1;
    while named.symlink_metadata().is_ok() {
        n += 1;
        named = folder.join(format!("{stem}-{n}.yml"));
    }
    named
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn piece_refusal(&self, code: &str, said: String, file: Option<&Path>) -> Refusal {
        let mut raised = WireError::raised(code, said, self.run_id());
        if let Some(file) = file {
            raised = raised.with_field("file", WireValue::Str(file.display().to_string()));
        }
        match code {
            PIECE_UNWRITABLE => Refusal::Fault(raised),
            _ => Refusal::Unacceptable(raised),
        }
    }

    fn piece_unwritable(&self, file: &Path, doing: &str, cause: std::io::Error) -> Refusal {
        self.piece_refusal(PIECE_UNWRITABLE, format!("{} could not be {doing}: {cause}", file.display()), Some(file))
    }

    /// Check one piece with its own parser, then write it in Kit's folder.
    pub(crate) fn lifecycle_piece_saved(&self, asked: SaveLifecyclePiece) -> Result<LifecyclePieceSaved, Refusal> {
        let (kind, written) = written_as(&asked.piece);
        let text = written.map_err(|why| {
            self.piece_refusal(PIECE_UNFIT, format!("The {} could not be written as a file: {why}", kind.as_wire()), None)
        })?;
        // The file is not named until the piece has said what it is.
        let unnamed = format!("({})", kind.as_wire());
        let declared = read(kind, Path::new(&unnamed), &text).map_err(|why| self.piece_refusal(PIECE_UNFIT, why, None))?;

        let folder = self.kit_pieces(folder_of(kind));
        let held = declaring(&folder, &declared);
        let (file, replaced) = match (held.first(), asked.overwrite) {
            (Some((file, _)), false) => {
                let said = format!(
                    "{} already declares {} and nothing was written; save again with overwrite to replace it",
                    file.display(),
                    declared.said()
                );
                return Err(self.piece_refusal(PIECE_EXISTS, said, Some(file)));
            }
            (Some((file, _)), true) => (file.clone(), true),
            (None, _) => (free_name(&folder, &declared), false),
        };
        write_over(&file, &text).map_err(|cause| self.piece_unwritable(&file, "written", cause))?;
        // A second file declaring it left both out of every read; the one that now says it stands alone.
        for (extra, _) in held.iter().skip(1) {
            std::fs::remove_file(extra).map_err(|cause| self.piece_unwritable(extra, "removed", cause))?;
        }
        Ok(LifecyclePieceSaved {
            kind,
            id: declared.id,
            scope: declared.scope,
            file: file.display().to_string(),
            replaced,
        })
    }

    /// Delete the file or files in Kit that declare one piece.
    pub(crate) fn lifecycle_piece_deleted(&self, asked: DeleteLifecyclePiece) -> Result<LifecyclePieceDeleted, Refusal> {
        let wanted = Identity::of(asked.kind, &asked.id, asked.scope);
        let held = declaring(&self.kit_pieces(folder_of(wanted.kind)), &wanted);
        let Some((first, _)) = held.first() else {
            return Err(self.piece_refusal(PIECE_NOT_KEPT, format!("Kit holds no {}.", wanted.said()), None));
        };
        for (file, _) in &held {
            std::fs::remove_file(file).map_err(|cause| self.piece_unwritable(file, "removed", cause))?;
        }
        Ok(LifecyclePieceDeleted { kind: wanted.kind, id: wanted.id, scope: wanted.scope, file: first.display().to_string() })
    }
}
