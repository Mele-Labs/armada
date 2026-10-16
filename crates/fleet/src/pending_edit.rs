//! A form's edit to `armada.yml` that went out as a pull request and has not
//! reached the checkout Fleet serves.
//!
//! **One per served repository, and it stacks.** A second save while the pull
//! request is open commits on top of its branch, so it is one pull request and
//! not a queue of them.
//!
//! **While it is pending the live keys follow its text and the file on disk is
//! left alone**, so the edit applies at once and nothing is left uncommitted in
//! a checkout that is only ever fast-forwarded (14 Oct 2026). It is dropped when
//! the checkout has its commit, or the forge says the pull request was closed
//! unmerged; either way the live keys are then read from the file again.

use std::io;
use std::sync::{Mutex, PoisonError};

use adapter_traits::{
    AgentHarness, Delivery, Landing, ManifestEditToSend, NotDelivered, PullRequestFor, Vcs,
    WorkProduct,
};
use api::Refusal;
use config::{amend, Edit};
use ipc::{Instant, ManifestEdited};

use crate::adrift::Adrift;
use crate::amending::declared_in;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// What the pull request holds of the Manifest, and where it is.
#[derive(Clone, Debug)]
pub(crate) struct Pending {
    pub(crate) branch: String,
    /// The tip of the branch, which a second edit is made on.
    pub(crate) commit: String,
    pub(crate) pull_request: String,
    /// The Manifest whole, as the branch holds it.
    pub(crate) text: String,
}

/// Whether a second edit may be made on a pending one's branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stacking {
    /// The pull request is open, or the forge would not say.
    Open,
    /// Merged, and the checkout has not caught up to it yet: its text is still
    /// the truth, and the branch is not one to push to.
    Merged,
}

/// The pending edit of one repository.
#[derive(Default)]
pub(crate) struct Held {
    standing: Mutex<Option<Pending>>,
    /// One save, or one settling, at a time: both read the pending edit and
    /// then ask the forge, and neither may be answered from before the other.
    turns: tokio::sync::Mutex<()>,
}

impl Held {
    pub(crate) fn standing(&self) -> Option<Pending> {
        self.standing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The text a reading of the Manifest is drawn from, where one is pending.
    pub(crate) fn text(&self) -> Option<String> {
        self.standing().map(|pending| pending.text)
    }

    fn set(&self, pending: Option<Pending>) {
        *self.standing.lock().unwrap_or_else(PoisonError::into_inner) = pending;
    }

    pub(crate) async fn turn(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.turns.lock().await
    }
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
    /// The pending edit, after dropping one the forge says was closed unmerged.
    /// **Called with the turn held.**
    pub(crate) async fn pending_edit_standing(
        &self,
        served: &Served,
    ) -> Option<(Pending, Stacking)> {
        let held = served.repository().pending_edit();
        let pending = held.standing()?;
        let url = pending.pull_request.clone();
        let landed = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.landed(root, &url)
            })
            .await;
        match landed.landing {
            Landing::ClosedUnmerged { .. } => {
                held.set(None);
                self.live_keys_read_from(served, None);
                None
            }
            Landing::Merged { .. } => Some((pending, Stacking::Merged)),
            Landing::Open { .. } | Landing::Unknown => Some((pending, Stacking::Open)),
        }
    }

    /// Send `edits` as a commit on the pending edit's branch, or on a new one
    /// off the base. **Re-applied to the text the target holds**, which is not
    /// always the text the form was drawn from.
    pub(crate) async fn edit_through_a_pull_request(
        &self,
        served: &Served,
        base: String,
        edits: Vec<Edit>,
        held: Option<(Pending, Stacking)>,
    ) -> Result<ManifestEdited, Refusal> {
        let file = served.manifest().path();
        let path = file.display().to_string();
        let root = served.root().to_string();
        let stacked = held
            .filter(|(_, stacking)| *stacking == Stacking::Open)
            .map(|(pending, _)| pending);
        let (parent, target, to) = match stacked {
            Some(pending) => (
                pending.commit,
                pending.text,
                PullRequestFor::Update {
                    branch: pending.branch,
                    pull_request: pending.pull_request,
                },
            ),
            None => {
                let (asked_base, asked_file) = (base.clone(), path.clone());
                let on_the_base = self
                    .forge_asked(&root, move |vcs: &V, root: &str| {
                        vcs.manifest_on_the_base(root, &asked_base, &asked_file)
                    })
                    .await
                    .map_err(|why| self.not_sent(&path, why))?;
                (
                    on_the_base.commit,
                    on_the_base.text,
                    PullRequestFor::Open { base },
                )
            }
        };
        let amended = amend(file, &target, &edits).map_err(|why| self.not_amended(why))?;
        let edited = |text: &str| ManifestEdited {
            path: path.clone(),
            at: Instant::from(&self.now()),
            text: text.to_string(),
            declared: Some(declared_in(amended.manifest())),
        };
        if amended.text() == target {
            return Ok(edited(amended.text()));
        }
        let branch = match &to {
            PullRequestFor::Update { branch, .. } => branch.clone(),
            PullRequestFor::Open { .. } => format!("armada/manifest-{:08x}", rand::random::<u32>()),
        };
        let name = file.file_name().map_or_else(
            || String::from("armada.yml"),
            |name| name.to_string_lossy().into_owned(),
        );
        let sending = ManifestEditToSend {
            file: path.clone(),
            text: amended.text().to_string(),
            parent,
            message: format!(
                "{name}: {}",
                edits.iter().map(Edit::said).collect::<Vec<_>>().join(", ")
            ),
            branch: branch.clone(),
            to,
        };
        let sent = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.send_manifest_edit(root, &sending)
            })
            .await
            .map_err(|why| self.not_sent(&path, why))?;
        // **`sent.auto_merge` is not read**: a forge that would not turn it on
        // leaves the branch out and the pull request open for a person to merge.
        served.repository().pending_edit().set(Some(Pending {
            branch,
            commit: sent.commit,
            pull_request: sent.pull_request,
            text: amended.text().to_string(),
        }));
        self.live_keys_read_from(served, Some(amended.text()));
        Ok(edited(amended.text()))
    }

    /// Drop the pending edit once the checkout has its commit, or the forge
    /// says the pull request was closed unmerged. Read with the reading of main.
    pub(crate) async fn pending_edit_settled(&self, served: &Served) {
        let held = served.repository().pending_edit();
        let _turn = held.turn().await;
        let Some(pending) = held.standing() else {
            return;
        };
        let commit = pending.commit.clone();
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return;
        };
        let landed_here = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.checkout_has(root, &base, &commit)
            })
            .await;
        if landed_here {
            held.set(None);
            self.live_keys_read_from(served, None);
            return;
        }
        // Dropped inside, where a closed pull request is read.
        self.pending_edit_standing(served).await;
    }

    /// Have the live keys read from `text`, or from the file where it is `None`.
    fn live_keys_read_from(&self, served: &Served, text: Option<&str>) {
        self.locating().reload_from(served.root(), text);
    }

    fn not_sent(&self, path: &str, why: NotDelivered) -> Refusal {
        self.refusal(Adrift::ManifestUnwritable {
            path: path.to_string(),
            cause: io::Error::other(why.said()),
        })
    }
}
