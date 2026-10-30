//! `commit_to_repository`: **Commit to repository** copies one piece a person keeps for a
//! repository into `.armada/` (their `armada.yml` at its root) by a Job of its own: its own slot
//! and branch, one commit, and a pull request for that alone, landed as every Job's is (the owner,
//! 21 Oct 2026). `docs/concepts/kit.md`, *Commit to repository*.
//!
//! **A copy, never a move**: the person's own piece is read and left where it is kept. **Every
//! refusal is decided against the checkout, before the Job is made**, so a refused commit leaves
//! no Job behind; the checkout is read and never written. **A pull request that does not open
//! loses nothing**: the Job keeps the commit on its branch, and the stream says why.

mod pieces;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Review, Vcs, WorkProduct};
use api::Refusal;
use config::KitRead;
use core_model::WorkflowSource;
use ipc::{
    BackgroundAct, CommitToRepository, CommittedToRepository, FlightControl, FlightControlsRead,
    LifecyclePieceKind, PieceScope, PieceToCommit, ReviewToolScope, SessionId, WireError,
};

use crate::daemon::Fleet;
use crate::flight_controls::repository::{check_repository, parse_repository, IN_REPOSITORY};
use crate::flight_controls::read_at;
use crate::piece_saving::identity::Identity;
use crate::repositories::Served;
use crate::requesting::{self, NotCopied};
use crate::run_memory::JobTree;

/// The piece is not one of the person's own. A 422.
pub(crate) const NOT_YOURS: &str = "fleet.commit_not_yours";
/// The repository's base already holds the id and `overwrite` was not asked for, or holds the
/// piece as written. A 422.
pub(crate) const EXISTS: &str = "fleet.commit_exists";
/// What the branch would hold breaks a rule, the base's file will not read, or no Job could be
/// made. A 422.
pub(crate) const REFUSED: &str = "fleet.commit_refused";
/// A file could not be written or git refused the commit. A 500.
pub(crate) const UNWRITABLE: &str = "fleet.commit_unwritable";
/// The commit is on the Job's branch and the pull request would not open. A 500, told as `act.failed`.
pub(crate) const PULL_REQUEST_NOT_OPENED: &str = "fleet.commit_pull_request_not_opened";

/// The repository's own folders, relative to its root.
const WORKFLOWS: &str = ".armada/workflows";
const REVIEW_TOOLS: &str = ".armada/review-tools";
/// The Manifest, at the workspace root.
const MANIFEST: &str = "armada.yml";

/// What one piece comes to on the branch: each file's path in the tree and its text, the paths it
/// deletes, the commit's message, and the title the Job and its pull request carry.
struct Copy {
    files: Vec<(String, String)>,
    removals: Vec<String>,
    message: String,
    title: String,
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
            UNWRITABLE | PULL_REQUEST_NOT_OPENED => Refusal::Fault(raised),
            _ => Refusal::Unacceptable(raised),
        }
    }

    /// `commit_to_repository`: the piece read against the checkout, then a Job of its own that
    /// commits it on its branch, pushes it and opens its pull request. **Answered once the Job and
    /// its branch exist**; the commit, the push and the pull request finish in a task of their own,
    /// and a failure there is `act.failed` for that Job.
    pub(crate) async fn committed_to_repository(
        self: &Arc<Self>,
        asked: CommitToRepository,
    ) -> Result<CommittedToRepository, Refusal> {
        let root = self.run_repository(&asked.repository)?;
        let served = self.repositories().served().into_iter().find(|one| one.root() == root).ok_or_else(|| {
            self.commit_refusal(REFUSED, format!("Fleet does not serve {root}."))
        })?;
        let main = JobTree { path: root.clone(), branch: served.manifest().base().unwrap_or("main").to_string() };
        let copy = match &asked.piece {
            _ if asked.remove => self.piece_removed(&main, &asked.piece)?,
            PieceToCommit::FlightControls { job, deck } => self.tiles_copied(&root, &main, job, deck, asked.overwrite).await?,
            PieceToCommit::Workflow { workflow_id } => self.workflow_copied(&root, &main, workflow_id.as_str(), asked.overwrite)?,
            PieceToCommit::ReviewTool { id } => self.review_tool_copied(&main, id, asked.overwrite)?,
            PieceToCommit::Manifest => self.manifest_copied(&root, &main, asked.overwrite).await?,
            PieceToCommit::Requests { collections, environments } => {
                self.requests_copied(&root, &main, collections, environments, asked.overwrite).await?
            }
            PieceToCommit::Lifecycle { id, scope } => {
                self.piece_copied(&root, &main, Identity::of(LifecyclePieceKind::Lifecycle, id, scope.clone()), asked.overwrite)?
            }
            PieceToCommit::LifecycleRule { id, scope } => {
                self.piece_copied(&root, &main, Identity::of(LifecyclePieceKind::Rule, id, scope.clone()), asked.overwrite)?
            }
            PieceToCommit::Workspace { id } => {
                self.piece_copied(&root, &main, Identity::of(LifecyclePieceKind::Workspace, id, PieceScope::Everywhere), asked.overwrite)?
            }
            PieceToCommit::Autonomy { id, scope } => {
                self.piece_copied(&root, &main, Identity::of(LifecyclePieceKind::Autonomy, id, scope.clone()), asked.overwrite)?
            }
        };
        let paths: Vec<String> = copy.files.iter().map(|(path, _)| path.clone()).chain(copy.removals.iter().cloned()).collect();
        let held = |(at, text): &(String, String)| std::fs::read_to_string(Path::new(&root).join(at)).is_ok_and(|on| &on == text);
        if copy.removals.is_empty() && copy.files.iter().all(held) {
            return Err(self.commit_refusal(EXISTS, format!("{} on {} already holds this as written.", paths.join(", "), main.branch)));
        }
        let (session_id, tree) = match self.job_of_its_own(&served, copy.title.clone()).await? {
            Ok(job) => job,
            Err(why) => return Err(self.commit_refusal(REFUSED, format!("No Job could be made for this commit. {why}"))),
        };
        let reply = CommittedToRepository { repository: root, session_id: session_id.clone(), branch: tree.branch.clone(), paths: paths.clone() };
        let (fleet, manifest) = (Arc::clone(self), ipc::ManifestId::carried(served.manifest().id().as_str()));
        tokio::spawn(async move {
            if let Err(refusal) = fleet.copy_finished(&served, &session_id, &tree, copy, &paths).await {
                fleet.act_failed(BackgroundAct::CommitToRepository, Some(manifest), Some(session_id), refusal);
            }
        });
        Ok(reply)
    }

    /// What follows the reply: the commit on the Job's branch, its push and its pull request. A pull
    /// request that does not open is a failure of this act, **the commit and the push standing**.
    async fn copy_finished(&self, served: &Served, session_id: &SessionId, tree: &JobTree, copy: Copy, paths: &[String]) -> Result<(), Refusal> {
        self.committed_in_removing(tree, &copy.files, &copy.removals, &copy.message).await.map_err(|why| self.commit_refusal(UNWRITABLE, why))?.ok_or_else(|| {
            self.commit_refusal(UNWRITABLE, format!("git found nothing to commit for {} on {}.", paths.join(", "), tree.branch))
        })?;
        let body = format!("{}.\n\nMade by Commit to repository, as a Job of its own: {}.", copy.message, paths.join(", "));
        match self.delivered_for_review(served, tree, Review::assembled(copy.title, body)).await {
            Ok(url) => {
                self.pull_attached(served, session_id.as_str(), &tree.branch, &url).await;
                Ok(())
            }
            Err(why) => Err(self.commit_refusal(PULL_REQUEST_NOT_OPENED, why)),
        }
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
        let (mut added, mut named) = (Vec::new(), Vec::new());
        for (list, chosen, from, onto) in [("job", job, &yours.job, &mut theirs.job), ("deck", deck, &yours.deck, &mut theirs.deck)] {
            for id in chosen {
                let Some(tile) = from.iter().find(|tile| &tile.id == id) else {
                    return Err(self.commit_refusal(NOT_YOURS, format!("Your tiles for this repository have no `{id}` in {list}.")));
                };
                placed(onto, tile.clone(), overwrite).map_err(|()| {
                    self.commit_refusal(EXISTS, format!("{IN_REPOSITORY} on {} already holds `{id}` in {list}. Commit with overwrite to replace it.", tree.branch))
                })?;
                added.push(id.as_str());
                named.push(tile.name.clone().unwrap_or_else(|| id.clone()));
            }
        }
        check_repository(&theirs).map_err(|why| self.commit_refusal(REFUSED, why))?;
        let text = toml::to_string(&theirs).map_err(|why| self.commit_refusal(UNWRITABLE, format!("{IN_REPOSITORY} could not be written: {why}")))?;
        let noun = if added.len() == 1 { "tile" } else { "tiles" };
        Ok(Copy {
            files: vec![(IN_REPOSITORY.to_string(), text)],
            removals: Vec::new(),
            message: format!("Add the {noun} {}", added.join(", ")),
            title: format!("Commit the {noun} {}", named.join(", ")),
        })
    }

    /// The chosen collections and environments from the person's own, added to the branch's
    /// `.armada/requests.toml`. A secret goes as its name: the file has nowhere to put a value.
    async fn requests_copied(
        &self,
        root: &str,
        tree: &JobTree,
        collections: &[String],
        environments: &[String],
        overwrite: bool,
    ) -> Result<Copy, Refusal> {
        let yours = self
            .own_requests_file(root)
            .await
            .map_err(|why| self.commit_refusal(NOT_YOURS, format!("Your requests for this repository did not read: {why}")))?;
        let theirs = requesting::read_file(&Path::new(&tree.path).join(requesting::IN_REPOSITORY))
            .await
            .map_err(|why| self.commit_refusal(REFUSED, why))?;
        let held = requesting::IN_REPOSITORY;
        let copied = requesting::copied(theirs, &yours, collections, environments, overwrite).map_err(|why| match why {
            NotCopied::NotYours(why) => self.commit_refusal(NOT_YOURS, why),
            NotCopied::Exists(what) => self.commit_refusal(
                EXISTS,
                format!("{held} on {} already holds the {what}. Commit with overwrite to replace it.", tree.branch),
            ),
        })?;
        requesting::check(&copied).map_err(|why| self.commit_refusal(REFUSED, why))?;
        let text = requesting::written(&copied)
            .map_err(|why| self.commit_refusal(UNWRITABLE, format!("{held} could not be written: {why}")))?;
        let named: Vec<&str> = collections.iter().chain(environments).map(String::as_str).collect();
        Ok(Copy {
            files: vec![(held.to_string(), text)],
            removals: Vec::new(),
            message: format!("Add the requests {}", named.join(", ")),
            title: format!("Commit the requests {}", named.join(", ")),
        })
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
        Ok(Copy { files: vec![(path, kit.text().to_string())], removals: Vec::new(), message: format!("Add the workflow {id}"), title: format!("Commit the workflow {id}") })
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
        Ok(Copy { files: vec![(path, text)], removals: Vec::new(), message: format!("Add the review tool {id}"), title: format!("Commit the review tool {id}") })
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
        Ok(Copy { files: vec![(MANIFEST.to_string(), text)], removals: Vec::new(), message: format!("Add {MANIFEST}"), title: format!("Commit {MANIFEST}") })
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
