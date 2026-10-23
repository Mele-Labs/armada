//! A Job Fleet opens for one act of its own: a hosted session with a slot leased and its branch
//! cut, whose process never starts unless a message is sent to it. Commit to repository's, so the
//! piece rides in a pull request of its own (the owner, 21 Oct 2026), and Promote's, which tells
//! the Job to see its pull request merged. `crate::committing`, `crate::mods`.
//!
//! What both do in the Job's tree is here: the files written and committed alone, the branch
//! pushed with its pull request opened, and that pull request put on the Job's row.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, CommitTime, Committed, Delivery, Opened, Pushed, Review, Vcs, WorkProduct, Worktree};
use api::Refusal;
use ipc::{CloseSession, ManifestId, SessionId, StartSession};
use store::{AttachmentState, Holder, KeptAttachment};

use super::gate::Taken;
use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::run_memory::JobTree;

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
    /// A Job titled `title` in `served`, holding a slot and its own branch. The inner `Err` says
    /// why no slot was leased, and the Job is closed again.
    pub(crate) async fn job_of_its_own(
        &self,
        served: &Served,
        title: String,
    ) -> Result<Result<(SessionId, JobTree), String>, Refusal> {
        let start = StartSession {
            manifest_id: ManifestId::carried(served.manifest().id().as_str()),
            title: Some(title),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        };
        let id = self.started_hosted(start, false).await?.id;
        let leased = match self.take_lease(id.as_str(), served).await {
            // No process runs, so there is nothing to move.
            Taken::Moving(moving) => {
                self.hosts().of(id.as_str()).state().moving = None;
                Ok(JobTree { path: moving.directory, branch: moving.branch })
            }
            Taken::Held { directory, .. } => match self.session_and_hosting(id.as_str()).await?.1.lease_branch {
                Some(branch) => Ok(JobTree { path: directory, branch }),
                None => Err(format!("The slot at {directory} has no branch.")),
            },
            Taken::Full => Err(format!("Every slot of {} is leased.", served.root())),
            Taken::Refused(why) => Err(why),
            Taken::Gone => Err(String::from("The Job ended before a slot was leased.")),
        };
        match leased {
            Ok(tree) => Ok(Ok((id, tree))),
            Err(why) => {
                let _ = self.closed_and_parked(CloseSession { session_id: id }).await;
                Ok(Err(why))
            }
        }
    }

    /// Write each file in the Job's tree and commit those paths alone on its branch with `message`.
    /// `None` where the branch already held them as written; the `Err` says what failed.
    pub(crate) async fn committed_in(&self, tree: &JobTree, files: &[(String, String)], message: &str) -> Result<Option<String>, String> {
        let (vcs, path, branch) = (Arc::clone(self.vcs()), tree.path.clone(), tree.branch.clone());
        let (files, message) = (files.to_vec(), message.to_string());
        let secs = self.now().epoch_millis().unwrap_or_default().div_euclid(1_000);
        tokio::task::spawn_blocking(move || {
            for (at, text) in &files {
                write_over(&Path::new(&path).join(at), text).map_err(|why| format!("{at} could not be written: {why}"))?;
            }
            let paths: Vec<&str> = files.iter().map(|(at, _)| at.as_str()).collect();
            match vcs.commit_paths(&Worktree::at(path.clone(), branch.clone()), &paths, &message, CommitTime::seconds_since_epoch(secs)) {
                Ok(Committed::Made { commit }) => Ok(Some(commit)),
                Ok(Committed::NothingToCommit) => Ok(None),
                Err(why) => Err(format!("{} could not be committed on {branch}: {why}", paths.join(", "))),
            }
        })
        .await
        .map_err(|why| why.to_string())
        .and_then(|done| done)
    }

    /// Push the Job's branch and open its pull request into the base, through the forge every
    /// Job's delivery uses. The address, or why there is none.
    pub(crate) async fn delivered_for_review(&self, served: &Served, tree: &JobTree, review: Review) -> Result<String, String> {
        let (vcs, branch) = (Arc::clone(self.vcs()), tree.branch.clone());
        let worktree = Worktree::at(tree.path.clone(), tree.branch.clone());
        let declared = served.manifest().base().map(str::to_string);
        let done = tokio::task::spawn_blocking(move || {
            match vcs.push(&worktree) {
                Ok(Pushed::ToTheRemote { .. }) => {}
                Ok(Pushed::NoRemote) => return Err(format!("The repository has no remote, so {branch} was not pushed and no pull request was opened.")),
                Err(why) => return Err(format!("{branch} was not pushed, so no pull request was opened: {}", why.said)),
            }
            let base = match vcs.base(&worktree, declared.as_deref()) {
                Ok(Some(base)) => base,
                Ok(None) => return Err(format!("{branch} is pushed. The repository names no base, so no pull request was opened.")),
                Err(why) => return Err(format!("{branch} is pushed. The base did not read, so no pull request was opened: {}", why.said)),
            };
            match vcs.open_for_review(&worktree, &base, &review) {
                Ok(Opened::PullRequest { url } | Opened::AlreadyOpen { url }) => Ok(url),
                Ok(Opened::NothingPushed) => Err(format!("Nothing of {branch} reached the remote, so no pull request was opened.")),
                Ok(Opened::NoTool { why }) => Err(format!("{branch} is pushed. No pull request was opened: {why}")),
                Err(why) => Err(format!("{branch} is pushed. The pull request did not open: {}", why.said)),
            }
        })
        .await;
        done.unwrap_or_else(|why| Err(format!("{} is committed. Pushing it stopped: {why}", tree.branch)))
    }

    /// The pull request on the Job's ledger row, as a claim puts it, so the pull reading, the merge
    /// and Fleet's reports follow it from now rather than from the pull reading's next pass.
    pub(crate) async fn pull_attached(&self, served: &Served, id: &str, branch: &str, url: &str) {
        let Some(number) = crate::pull_requesting::number_of(url) else { return };
        let now = self.now().as_str().to_string();
        let kept = KeptAttachment {
            holder: Holder::session(id),
            kind: crate::pull_requesting::PR.into(),
            manifest_id: served.manifest().id().as_str().to_string(),
            target: number.to_string(),
            state: AttachmentState::Standing,
            detail: [
                ("state".to_string(), "open".to_string()),
                ("branch".to_string(), branch.to_string()),
                ("address".to_string(), url.to_string()),
            ]
            .into(),
            since: now.clone(),
            changed_at: now,
        };
        let _ = self.store().lock().await.attach(&kept, false);
        let _ = self.pull_request_registered(served.root(), number, &crate::red_claim::Taker::Session(id.to_string())).await;
        let _ = self.published_hosted(id).await;
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
