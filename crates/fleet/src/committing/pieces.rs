//! Commit to repository for a lifecycle, a rule and autonomy defaults: the person's own file in Kit
//! copied into `.armada/<folder>/`, or the repository's file taken out of it. **A Project is never
//! committed**: it is kept on this Mac, in a synced folder or in the cloud.
//!
//! **A piece is found by what its file declares** (`crate::piece_saving::identity`), in Kit and on
//! the branch, and the file on the branch that declares it is the one replaced whatever it is
//! called. **A file on the branch that declares something else is never written over.**
//!
//! **Where a piece may be committed follows its scope.** A piece scoped within a Project is read by
//! every member repository from every other member's `.armada/`, so it may be committed to any
//! repository the Project holds and to no other. One scoped within a repository is that repository's.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::ScopeId;
use ipc::{LifecyclePieceKind, PieceScope, PieceToCommit};

use super::{Copy, NOT_YOURS, REFUSED};
use crate::daemon::Fleet;
use crate::piece_saving::identity::{declaring, folder_of, Identity};
use crate::run_memory::JobTree;

/// The piece a commit names, where it is one of the three kinds a repository can hold apart from
/// the rest of its `.armada/`.
fn identity_of(piece: &PieceToCommit) -> Option<Identity> {
    match piece {
        PieceToCommit::Lifecycle { id, scope } => Some(Identity::of(LifecyclePieceKind::Lifecycle, id, scope.clone())),
        PieceToCommit::LifecycleRule { id, scope } => Some(Identity::of(LifecyclePieceKind::Rule, id, scope.clone())),
        PieceToCommit::Autonomy { id, scope } => Some(Identity::of(LifecyclePieceKind::Autonomy, id, scope.clone())),
        PieceToCommit::FlightControls { .. }
        | PieceToCommit::Workflow { .. }
        | PieceToCommit::ReviewTool { .. }
        | PieceToCommit::Manifest
        | PieceToCommit::Requests { .. } => None,
    }
}

/// What the older kinds are called in the sentence that refuses to remove one.
fn older_word(piece: &PieceToCommit) -> &'static str {
    match piece {
        PieceToCommit::FlightControls { .. } => "tiles",
        PieceToCommit::Workflow { .. } => "a workflow",
        PieceToCommit::ReviewTool { .. } => "a review tool",
        PieceToCommit::Manifest => "armada.yml",
        PieceToCommit::Requests { .. } => "requests",
        PieceToCommit::Lifecycle { .. }
        | PieceToCommit::LifecycleRule { .. }
        | PieceToCommit::Autonomy { .. } => "this",
    }
}

/// The relative paths of the files under `tree` that declare `wanted`, and the text of each.
fn declared_in(tree: &Path, wanted: &Identity) -> Vec<(String, String)> {
    let folder = tree.join(".armada").join(folder_of(wanted.kind));
    declaring(&folder, wanted)
        .into_iter()
        .filter_map(|(path, text)| Some((path.strip_prefix(tree).ok()?.to_string_lossy().to_string(), text)))
        .collect()
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
    /// Whether `root` is where `wanted` may be committed. **A refusal names the repository it
    /// belongs in.**
    fn piece_placed_in(&self, root: &str, wanted: &Identity) -> Result<(), Refusal> {
        let PieceScope::Within { id: scoped } = &wanted.scope else { return Ok(()) };
        let projects = self.projects_held();
        if let Some(project) = ScopeId::new(scoped).ok().and_then(|id| projects.get(&id).cloned()) {
            let member = self.repositories().hosted().into_iter().any(|one| one.root() == root && project.holds(one.manifest().id()));
            return match member {
                true => Ok(()),
                false => Err(self.commit_refusal(
                    REFUSED,
                    format!("The {} is scoped to the Project `{scoped}`, which does not hold this repository. Commit it to one that does.", wanted.said()),
                )),
            };
        }
        match self.repositories().serving(scoped) {
            Some(one) if one.root() == root => Ok(()),
            Some(one) => Err(self.commit_refusal(
                REFUSED,
                format!("The {} is scoped to the repository {}. Commit it there.", wanted.said(), one.root()),
            )),
            None => Err(self.commit_refusal(
                REFUSED,
                format!("The {} is scoped within `{scoped}`, which is neither a Project nor a repository Fleet serves.", wanted.said()),
            )),
        }
    }

    /// Kit's file that declares `wanted`, as its file is written, into `.armada/<folder>/` under its
    /// own name, or in place of the file on the branch that declares it.
    pub(super) fn piece_copied(&self, root: &str, tree: &JobTree, wanted: Identity, overwrite: bool) -> Result<Copy, Refusal> {
        let folder = folder_of(wanted.kind);
        let kit = declaring(&self.kit_pieces(folder), &wanted);
        let Some((kit_file, text)) = kit.into_iter().next() else {
            return Err(self.commit_refusal(NOT_YOURS, format!("Your Kit holds no {}.", wanted.said())));
        };
        let Some(name) = kit_file.file_name().map(|name| name.to_string_lossy().to_string()) else {
            return Err(self.commit_refusal(NOT_YOURS, format!("Your Kit holds no {}.", wanted.said())));
        };
        self.piece_placed_in(root, &wanted)?;

        let in_tree = Path::new(&tree.path);
        let named = format!(".armada/{folder}/{name}");
        let held = declared_in(in_tree, &wanted).into_iter().next().map(|(path, _)| path);
        if held.is_none() && in_tree.join(&named).exists() {
            let said = format!("{named} on {} holds something else, so the {} was not written over it.", tree.branch, wanted.said());
            return Err(self.commit_refusal(super::EXISTS, said));
        }
        let path = self.free_or_overwritten(held, named, &wanted.id, tree, overwrite)?;
        let named = wanted.named();
        Ok(Copy { files: vec![(path, text)], removals: Vec::new(), message: format!("Add the {named}"), title: format!("Commit the {named}") })
    }

    /// The file on the branch that declares the piece, deleted. **Only the three kinds above can be
    /// removed**, and removing what the repository does not hold is refused.
    pub(super) fn piece_removed(&self, tree: &JobTree, piece: &PieceToCommit) -> Result<Copy, Refusal> {
        let Some(wanted) = identity_of(piece) else {
            return Err(self.commit_refusal(
                REFUSED,
                format!("Removing {} from a repository is not built. Only a lifecycle, rule or autonomy defaults can be removed.", older_word(piece)),
            ));
        };
        let removals: Vec<String> = declared_in(Path::new(&tree.path), &wanted).into_iter().map(|(path, _)| path).collect();
        if removals.is_empty() {
            return Err(self.commit_refusal(REFUSED, format!("{} holds no {} to remove.", tree.branch, wanted.said())));
        }
        let named = wanted.named();
        Ok(Copy { files: Vec::new(), removals, message: format!("Remove the {named}"), title: format!("Remove the {named}") })
    }
}
