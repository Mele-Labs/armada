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
//! the checkout has its commit, or the forge says it merged and the checkout has
//! caught up to the base, or the forge says the pull request was closed
//! unmerged; each way the live keys are then read from the file again.
//!
//! **A raw save of the whole file stacks the same way**, as a commit of the text
//! as typed on the pending branch, so it goes into the same pull request.
//!
//! **A forge that would not turn auto-merge on is kept on the edit**, as the
//! plain fact [`Pending::merge_by_hand`] says. Nothing draws it yet: no surface
//! Fleet already has for a fact with no Job to hang it on can carry it without
//! a Bridge change, which is the owner's to ask for (#2396).

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
    /// What the forge said when it would not turn auto-merge on, in its own
    /// words. **Kept across a second save**: the pull request is the same one.
    pub(crate) auto_merge_refused: Option<String>,
}

impl Pending {
    /// The fact a person is owed when the pull request waits for them, or
    /// `None` while the forge is going to merge it itself.
    pub(crate) fn merge_by_hand(&self) -> Option<String> {
        self.auto_merge_refused.as_ref()?;
        let number = self.pull_request.rsplit('/').next().unwrap_or_default();
        Some(format!("armada.yml edit #{number} needs merging by hand"))
    }
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
        let carried = stacked
            .as_ref()
            .and_then(|pending| pending.auto_merge_refused.clone());
        let (parent, target, to) = self.parent_target_and_pull_request(&root, &path, base, stacked).await?;
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
        let name = file_name(file);
        let message = format!(
            "{name}: {}",
            edits.iter().map(Edit::said).collect::<Vec<_>>().join(", ")
        );
        self.sent_as_pending(served, amended.text(), message, parent, to, carried)
            .await?;
        self.live_keys_read_from(served, Some(amended.text()));
        Ok(edited(amended.text()))
    }

    /// Stack `text`, the file whole as a person typed it, on the pending edit's
    /// branch, or open a pull request for it where the pending one has merged.
    /// **Called with the turn held**, and `text` is not parsed: a raw save puts
    /// work in progress down, as it does on disk.
    pub(crate) async fn raw_text_through_a_pull_request(
        &self,
        served: &Served,
        base: String,
        text: String,
        held: (Pending, Stacking),
    ) -> Result<(), Refusal> {
        let file = served.manifest().path();
        let path = file.display().to_string();
        let root = served.root().to_string();
        let (pending, stacking) = held;
        if text == pending.text {
            return Ok(());
        }
        let stacked = (stacking == Stacking::Open).then_some(pending);
        let carried = stacked
            .as_ref()
            .and_then(|pending| pending.auto_merge_refused.clone());
        let (parent, _, to) = self
            .parent_target_and_pull_request(&root, &path, base, stacked)
            .await?;
        let message = format!("{}: edited as text", file_name(file));
        self.sent_as_pending(served, &text, message, parent, to, carried)
            .await?;
        self.live_keys_read_from(served, Some(&text));
        Ok(())
    }

    /// The commit an edit is made on, the text it is made against and where it
    /// goes: the pending edit's own where it is open, else the base as the
    /// remote holds it.
    async fn parent_target_and_pull_request(
        &self,
        root: &str,
        path: &str,
        base: String,
        stacked: Option<Pending>,
    ) -> Result<(String, String, PullRequestFor), Refusal> {
        Ok(match stacked {
            Some(pending) => (
                pending.commit,
                pending.text,
                PullRequestFor::Update {
                    branch: pending.branch,
                    pull_request: pending.pull_request,
                },
            ),
            None => {
                let (asked_base, asked_file) = (base.clone(), path.to_string());
                let on_the_base = self
                    .forge_asked(root, move |vcs: &V, root: &str| {
                        vcs.manifest_on_the_base(root, &asked_base, &asked_file)
                    })
                    .await
                    .map_err(|why| self.not_sent(path, why))?;
                (
                    on_the_base.commit,
                    on_the_base.text,
                    PullRequestFor::Open { base },
                )
            }
        })
    }

    /// Commit `text` on `parent`, push it, and keep it as the pending edit.
    async fn sent_as_pending(
        &self,
        served: &Served,
        text: &str,
        message: String,
        parent: String,
        to: PullRequestFor,
        carried: Option<String>,
    ) -> Result<(), Refusal> {
        let path = served.manifest().path().display().to_string();
        let root = served.root().to_string();
        let branch = match &to {
            PullRequestFor::Update { branch, .. } => branch.clone(),
            PullRequestFor::Open { .. } => format!("armada/manifest-{:08x}", rand::random::<u32>()),
        };
        let sending = ManifestEditToSend {
            file: path.clone(),
            text: text.to_string(),
            parent,
            message,
            branch: branch.clone(),
            to,
        };
        let sent = self
            .forge_asked(&root, move |vcs: &V, root: &str| {
                vcs.send_manifest_edit(root, &sending)
            })
            .await
            .map_err(|why| self.not_sent(&path, why))?;
        let pending = Pending {
            branch,
            commit: sent.commit,
            pull_request: sent.pull_request,
            text: text.to_string(),
            auto_merge_refused: sent.auto_merge.err().or(carried),
        };
        // The one place it is said until a surface draws it.
        if let Some(fact) = pending.merge_by_hand() {
            eprintln!("{fact}");
        }
        served.repository().pending_edit().set(Some(pending));
        Ok(())
    }

    /// Drop the pending edit once it is in the checkout, or can never be.
    /// Read with the reading of main.
    ///
    /// **Three ways it settles.** The checkout has the edit's own commit. The
    /// forge says it was closed unmerged. Or the forge says it merged and the
    /// checkout has the base's tip on the forge, read after the merge was
    /// seen, so the merge is in it whatever commit it made: a squash or a
    /// rebase puts a different commit on the base, and the first way never
    /// answers for those (#2394).
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
            .forge_asked(served.root(), {
                let base = base.clone();
                move |vcs: &V, root: &str| vcs.checkout_has(root, &base, &commit)
            })
            .await;
        if landed_here {
            held.set(None);
            self.live_keys_read_from(served, None);
            return;
        }
        // Dropped inside, where a closed pull request is read.
        let Some((_, Stacking::Merged)) = self.pending_edit_standing(served).await else {
            return;
        };
        let Some(tip) = self
            .forge_asked(served.root(), {
                let base = base.clone();
                move |vcs: &V, root: &str| vcs.base_head_on_the_forge(root, &base)
            })
            .await
        else {
            return;
        };
        let caught_up = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.checkout_has(root, &base, &tip)
            })
            .await;
        if caught_up {
            held.set(None);
            self.live_keys_read_from(served, None);
        }
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

/// The file's own name, which a commit message leads with.
fn file_name(file: &std::path::Path) -> String {
    file.file_name().map_or_else(
        || String::from("armada.yml"),
        |name| name.to_string_lossy().into_owned(),
    )
}
