//! `commit_to_repository`: **Commit to repository** copies one piece a person keeps for a
//! repository into `.armada/` on the branch of the Job they are in (their `armada.yml` at its
//! root), so it lands with that Job's pull request. `docs/concepts/kit.md`, *Commit to repository*.
//!
//! **A copy, never a move**: the person's own piece is read and left where it is kept. **Never the
//! checkout**: the files are written in the Job's tree and committed alone on its branch, as run
//! memory commits `armada.yml`. With no Job named that holds a slot in the repository it is
//! refused, since Fleet does not guess among several.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, CommitTime, Committed, Delivery, Vcs, WorkProduct, Worktree};
use api::Refusal;
use config::KitRead;
use core_model::WorkflowSource;
use ipc::{
    CommitToRepository, CommittedToRepository, FlightControl, FlightControlsRead,
    PieceToCommit, ReviewToolScope, SessionId, WireError,
};

use crate::daemon::Fleet;
use crate::flight_controls::repository::{check_repository, parse_repository, IN_REPOSITORY};
use crate::flight_controls::read_at;
use crate::run_memory::JobTree;

/// No Job named that holds a branch in the repository. A 422.
pub(crate) const NO_JOB: &str = "fleet.commit_no_job";
/// The piece is not one of the person's own. A 422.
pub(crate) const NOT_YOURS: &str = "fleet.commit_not_yours";
/// The Job's branch already holds the id and `overwrite` was not asked for. A 422.
pub(crate) const EXISTS: &str = "fleet.commit_exists";
/// What the branch would hold breaks a rule, or the branch's file will not read. A 422.
pub(crate) const REFUSED: &str = "fleet.commit_refused";
/// A file could not be written or git refused the commit. A 500.
pub(crate) const UNWRITABLE: &str = "fleet.commit_unwritable";

/// The repository's own folders, relative to its root.
const WORKFLOWS: &str = ".armada/workflows";
const REVIEW_TOOLS: &str = ".armada/review-tools";
/// The Manifest, at the workspace root.
const MANIFEST: &str = "armada.yml";

/// What one piece comes to on the branch: each file's path in the tree and its text.
struct Copy {
    files: Vec<(String, String)>,
    message: String,
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
    /// The tree and branch of the Job `id` names, where it is in `root` and holds a slot.
    pub(crate) async fn job_tree_in(&self, root: &str, id: Option<&SessionId>) -> Option<JobTree> {
        let (_, hosting) = self.session_and_hosting(id?.as_str()).await.ok()?;
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id))).ok()?;
        match (served.root() == root, hosting.lease_slot, hosting.lease_branch) {
            (true, Some(slot), Some(branch)) => Some(JobTree { path: adapter_traits::slot_path(root, slot), branch }),
            _ => None,
        }
    }

    fn commit_refusal(&self, code: &str, said: String) -> Refusal {
        let raised = WireError::raised(code, said, self.run_id());
        match code {
            UNWRITABLE => Refusal::Fault(raised),
            _ => Refusal::Unacceptable(raised),
        }
    }

    /// `commit_to_repository`: copy the piece into the Job's tree and commit it on its branch.
    pub(crate) async fn committed_to_repository(
        &self,
        asked: CommitToRepository,
    ) -> Result<CommittedToRepository, Refusal> {
        let root = self.run_repository(&asked.repository)?;
        let Some(tree) = self.job_tree_in(&root, asked.session_id.as_ref()).await else {
            let said = match &asked.session_id {
                None => String::from("No Job is named. Commit from a Job open in this repository."),
                Some(id) => format!("Job {} holds no branch in {root}. Commit from a Job open in this repository.", id.as_str()),
            };
            return Err(self.commit_refusal(NO_JOB, said));
        };
        let copy = match &asked.piece {
            PieceToCommit::FlightControls { job, deck } => self.tiles_copied(&root, &tree, job, deck, asked.overwrite).await?,
            PieceToCommit::Workflow { workflow_id } => self.workflow_copied(&root, &tree, workflow_id.as_str(), asked.overwrite)?,
            PieceToCommit::ReviewTool { id } => self.review_tool_copied(&tree, id, asked.overwrite)?,
            PieceToCommit::Manifest => self.manifest_copied(&root, &tree, asked.overwrite).await?,
        };
        let paths: Vec<String> = copy.files.iter().map(|(path, _)| path.clone()).collect();
        let commit = self.copy_committed(&tree, copy).await?;
        Ok(CommittedToRepository { repository: root, branch: tree.branch, paths, commit })
    }

    /// The chosen tiles from the person's own, added to the tiles the branch already holds.
    async fn tiles_copied(
        &self,
        root: &str,
        tree: &JobTree,
        job: &[String],
        deck: &[String],
        overwrite: bool,
    ) -> Result<Copy, Refusal> {
        if job.is_empty() && deck.is_empty() {
            return Err(self.commit_refusal(NOT_YOURS, String::from("No tiles are chosen.")));
        }
        let yours = match self.repository_flight_controls_read(root.to_string(), None).await?.read {
            FlightControlsRead::Read { controls } => controls,
            FlightControlsRead::NotRead { why } => return Err(self.commit_refusal(NOT_YOURS, format!("Your tiles for this repository did not read: {why}"))),
        };
        let at = Path::new(&tree.path).join(IN_REPOSITORY);
        let mut theirs = match read_at(&at, parse_repository).await {
            FlightControlsRead::Read { controls } => controls,
            FlightControlsRead::NotRead { why } => return Err(self.commit_refusal(REFUSED, why)),
        };
        let mut added = Vec::new();
        for (list, chosen, from, onto) in [("job", job, &yours.job, &mut theirs.job), ("deck", deck, &yours.deck, &mut theirs.deck)] {
            for id in chosen {
                let Some(tile) = from.iter().find(|tile| &tile.id == id) else {
                    return Err(self.commit_refusal(NOT_YOURS, format!("Your tiles for this repository have no `{id}` in {list}.")));
                };
                placed(onto, tile.clone(), overwrite).map_err(|()| {
                    self.commit_refusal(EXISTS, format!("{IN_REPOSITORY} on {} already holds `{id}` in {list}. Commit with overwrite to replace it.", tree.branch))
                })?;
                added.push(id.as_str());
            }
        }
        check_repository(&theirs).map_err(|why| self.commit_refusal(REFUSED, why))?;
        let text = toml::to_string(&theirs).map_err(|why| self.commit_refusal(UNWRITABLE, format!("{IN_REPOSITORY} could not be written: {why}")))?;
        let noun = if added.len() == 1 { "tile" } else { "tiles" };
        Ok(Copy { files: vec![(IN_REPOSITORY.to_string(), text)], message: format!("Add the {noun} {}", added.join(", ")) })
    }

    /// A workflow from the person's Kit, as its file is written, into `.armada/workflows/`.
    fn workflow_copied(&self, root: &str, tree: &JobTree, id: &str, overwrite: bool) -> Result<Copy, Refusal> {
        let not_yours = || self.commit_refusal(NOT_YOURS, format!("Your Kit holds no workflow `{id}` this repository runs."));
        let served = self.repositories().served().into_iter().find(|one| one.root() == root).ok_or_else(not_yours)?;
        let files = served.files();
        let kit = files.iter().find(|file| file.source() == WorkflowSource::Kit && file.id().as_str() == id).ok_or_else(not_yours)?;
        let name = kit.path().file_name().map(|name| name.to_string_lossy().to_string()).ok_or_else(not_yours)?;
        // The file on the branch that defines this id is the one replaced, whatever it is called.
        let in_tree = Path::new(&tree.path);
        let catalogued = self.locating().workflows(in_tree, served.manifest(), &served.workspaces());
        let defined = catalogued
            .files
            .iter()
            .filter(|file| file.source() == WorkflowSource::Repository && file.id().as_str() == id)
            .find_map(|file| file.path().strip_prefix(in_tree).ok().map(|path| path.to_string_lossy().to_string()));
        let named = format!("{WORKFLOWS}/{name}");
        let held = defined.or_else(|| in_tree.join(&named).exists().then(|| named.clone()));
        let path = self.free_or_overwritten(held, named, id, tree, overwrite)?;
        Ok(Copy { files: vec![(path, kit.text().to_string())], message: format!("Add the workflow {id}") })
    }

    /// A review tool from the person's Kit, as its file is written, into `.armada/review-tools/`.
    fn review_tool_copied(&self, tree: &JobTree, id: &str, overwrite: bool) -> Result<Copy, Refusal> {
        let kit = PathBuf::from(&self.host().kit_home).join(crate::review_tools::FOLDER);
        let Some((_, text)) = crate::review_tools::review_tool_defined(&kit, ReviewToolScope::Kit, id) else {
            return Err(self.commit_refusal(NOT_YOURS, format!("Your Kit holds no review tool `{id}`.")));
        };
        let in_tree = Path::new(&tree.path);
        let held = self
            .review_tool_held(&in_tree.join(REVIEW_TOOLS), ReviewToolScope::Repository, id)
            .and_then(|path| path.strip_prefix(in_tree).ok().map(|path| path.to_string_lossy().to_string()));
        let path = self.free_or_overwritten(held, format!("{REVIEW_TOOLS}/{id}.md"), id, tree, overwrite)?;
        Ok(Copy { files: vec![(path, text)], message: format!("Add the review tool {id}") })
    }

    /// The person's own `armada.yml` for the repository, from the place their Kit keeps it, as the
    /// file at the root of the Job's tree. Kit's copy of a repository that has an `armada.yml` of its
    /// own names only what it changes, so one that is not a whole Manifest is refused.
    async fn manifest_copied(&self, root: &str, tree: &JobTree, overwrite: bool) -> Result<Copy, Refusal> {
        let (label, read) = self.kit_manifest_read(root).await?;
        let text = match read {
            KitRead::Text(text) => text,
            KitRead::Absent => return Err(self.commit_refusal(NOT_YOURS, String::from("Your Kit holds no armada.yml for this repository."))),
            KitRead::Unread(why) => return Err(self.commit_refusal(NOT_YOURS, format!("Your armada.yml for this repository did not read: {why}"))),
        };
        if Path::new(&tree.path).join(MANIFEST).exists() && !overwrite {
            return Err(self.commit_refusal(EXISTS, format!("{MANIFEST} on {} already exists. Commit with overwrite to replace it.", tree.branch)));
        }
        config::Manifest::parse(Path::new(MANIFEST), &text)
            .map_err(|why| self.commit_refusal(REFUSED, format!("{} is not a whole Manifest: {why}", label.display())))?;
        Ok(Copy { files: vec![(MANIFEST.to_string(), text)], message: format!("Add {MANIFEST}") })
    }

    /// The path a copy is written to: the file the branch already holds it in, only with
    /// `overwrite`, and else `named`.
    fn free_or_overwritten(&self, held: Option<String>, named: String, id: &str, tree: &JobTree, overwrite: bool) -> Result<String, Refusal> {
        match (held, overwrite) {
            (Some(path), false) => Err(self.commit_refusal(EXISTS, format!("{path} on {} already holds `{id}`. Commit with overwrite to replace it.", tree.branch))),
            (Some(path), true) => Ok(path),
            (None, _) => Ok(named),
        }
    }

    /// Write each file in the Job's tree and commit those paths alone on its branch. `None` where
    /// the branch already held them as written.
    async fn copy_committed(&self, tree: &JobTree, copy: Copy) -> Result<Option<String>, Refusal> {
        let (vcs, path, branch) = (Arc::clone(self.vcs()), tree.path.clone(), tree.branch.clone());
        let secs = self.now().epoch_millis().unwrap_or_default().div_euclid(1_000);
        let done = tokio::task::spawn_blocking(move || {
            for (at, text) in &copy.files {
                write_over(&Path::new(&path).join(at), text).map_err(|why| format!("{at} could not be written: {why}"))?;
            }
            let paths: Vec<&str> = copy.files.iter().map(|(at, _)| at.as_str()).collect();
            match vcs.commit_paths(&Worktree::at(path.clone(), branch.clone()), &paths, &copy.message, CommitTime::seconds_since_epoch(secs)) {
                Ok(Committed::Made { commit }) => Ok(Some(commit)),
                Ok(Committed::NothingToCommit) => Ok(None),
                Err(why) => Err(format!("{} could not be committed on {branch}: {why}", paths.join(", "))),
            }
        })
        .await
        .map_err(|why| why.to_string())
        .and_then(|done| done);
        done.map_err(|why| self.commit_refusal(UNWRITABLE, why))
    }
}

/// Put `tile` into `list`: in place of one of the same id only with `overwrite`, else at the end.
fn placed(list: &mut Vec<FlightControl>, tile: FlightControl, overwrite: bool) -> Result<(), ()> {
    match (list.iter().position(|held| held.id == tile.id), overwrite) {
        (Some(_), false) => Err(()),
        (Some(at), true) => {
            list[at] = tile;
            Ok(())
        }
        (None, _) => {
            list.push(tile);
            Ok(())
        }
    }
}

/// Write `text` to `<path>.writing` and rename it over `path`.
fn write_over(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut staged = path.as_os_str().to_owned();
    staged.push(".writing");
    let staged = PathBuf::from(staged);
    std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, path))
}
