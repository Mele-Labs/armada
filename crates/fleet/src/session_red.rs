//! A red main whose pull request a Session merged goes back to that Session.
//! `docs/capabilities/merge-line.md`, *When a Session's pull request turns main red*.
//!
//! **Fleet takes the claim itself.** A Job's red is sent back by `main_fix`; this is the same
//! pickup for a pull request nobody's Job opened, which the pull watch placed with the Session
//! that held it (`MainMerge::session`). The red is claimed for that Session first
//! (`Taker::Session`), then the Session is told as Fleet tells it anything (`told_by_fleet`),
//! which wakes it. A Session that cannot be told, because it ended or was refused, is forked,
//! the claim moves to the fork, and the fork is told in its place.
//!
//! **Once per red per Session**: the take is a `main_red` row on the Session's own ledger, kept
//! even where no central store applies. A red held elsewhere, held by a newer run, or already
//! taken by a Job here is left to the red band's buttons, as `main_fix::pick_up` leaves it.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use core_model::Timestamp;
use ipc::{ForkFrom, ManifestId, SessionId, SessionRecord, StartSession};
use store::{AttachmentState, Holder, KeptAttachment, MainCi};

use crate::daemon::Fleet;
use crate::main_ci::{MainChange, MainChanged};
use crate::main_fix::brief_of;
use crate::red_claim::Taker;
use crate::repositories::Served;

/// On a Session's ledger: it was handed this red. The target is when main first read red.
pub(crate) const MAIN_RED: &str = "main_red";

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

fn woken_words(main: &MainCi, number: u64, facts: &str) -> String {
    format!(
        "Main went red at {} after your pull request #{number} merged. Fleet has claimed the red \
         for you. Fix it.\n\n{facts}",
        short(main.red_commit())
    )
}

fn forked_words(main: &MainCi, number: u64, parent: &str, facts: &str) -> String {
    format!(
        "Main went red at {} after pull request #{number}, which session {parent} merged. Fleet \
         has claimed the red for this fork. Fix it.\n\n{facts}",
        short(main.red_commit())
    )
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
    /// What main's change asks of Fleet where a Session merged the pull request: each red is
    /// handed to it. **Nothing here fails the turn**; a refusal is the band's two buttons.
    /// Needs the `Arc` that waking a Session does.
    pub(crate) async fn red_given_to_sessions(self: &Arc<Self>, changes: &[MainChanged]) {
        for changed in changes {
            if matches!(changed.change, MainChange::WentRed | MainChange::BackToRed) {
                self.red_given_to_session(&changed.now).await;
            }
        }
    }

    async fn red_given_to_session(self: &Arc<Self>, main: &MainCi) {
        // A newer run may have fixed it: BackToRed picks it up when that run ends red.
        if main.held() {
            return;
        }
        let (Some(merge), Some(red_at)) = (main.merge.as_ref(), main.red_at.as_ref()) else {
            return;
        };
        // A Job's pull request is `main_fix::pick_up`'s.
        let (None, Some(id)) = (&merge.job, merge.session.clone()) else {
            return;
        };
        let known = {
            let store = self.store().lock().await;
            let taken = store.attachments_of(&Holder::session(&id)).is_ok_and(|held| {
                held.iter()
                    .any(|one| one.kind == MAIN_RED && one.target == red_at.as_str())
            });
            !taken && store.session(&id).ok().flatten().is_some()
        };
        if !known || self.taking(main).await.is_some() {
            return;
        }
        let Some(served) = self
            .repositories()
            .served()
            .into_iter()
            .find(|served| served.root() == main.repository)
        else {
            return;
        };
        // Held elsewhere, or the store silent: left to the band's buttons, which say why.
        if self.red_is_free(main, false).await.is_err() {
            return;
        }
        if self
            .claim_red(main, &Taker::Session(id.clone()), false)
            .await
            .is_err()
        {
            return;
        }
        let facts = self.with_the_log(&served, main, brief_of(main)).await;
        let said = woken_words(main, merge.number, &facts);
        if matches!(self.told_by_fleet(&id, &said).await, Ok(true)) {
            self.kept_main_red(&id, &served, red_at, main, "woke", None)
                .await;
            return;
        }
        let Some(fork) = self.forked_for_red(&served, &id, merge.number).await else {
            self.release_red(main).await;
            return;
        };
        // The claim made for the old Session moves to its fork, which is ours to replace.
        if self
            .claim_red(main, &Taker::Session(fork.clone()), true)
            .await
            .is_err()
        {
            self.release_red(main).await;
        }
        let said = forked_words(main, merge.number, &id, &facts);
        let _ = self.told_by_fleet(&fork, &said).await;
        self.kept_main_red(&id, &served, red_at, main, "forked", Some(&fork))
            .await;
        self.kept_main_red(&fork, &served, red_at, main, "took", None)
            .await;
    }

    /// A fork of `old`: of its conversation as it ended, or where it is live, of its latest
    /// message of its own. `None` where it cannot be forked either way.
    async fn forked_for_red(&self, served: &Served, old: &str, number: u64) -> Option<String> {
        let fork = |at: Option<String>| {
            self.start_session(StartSession {
                manifest_id: ManifestId::carried(served.manifest().id().as_str()),
                title: Some(format!("Fix main after #{number}")),
                model: None,
                effort: None,
                mode: None,
                root: None,
                pilot: None,
                fork: Some(ForkFrom {
                    session_id: SessionId::carried(old),
                    at,
                }),
            })
        };
        let made: Result<SessionRecord, _> = match fork(None).await {
            Ok(made) => Ok(made),
            Err(_) => match self.latest_fork_point(old).await {
                Ok(at) => fork(Some(at)).await,
                Err(why) => Err(why),
            },
        };
        made.ok().map(|made| made.id.as_str().to_string())
    }

    /// Record on `session`'s ledger that it was handed this red.
    async fn kept_main_red(
        &self,
        session: &str,
        served: &Served,
        red_at: &Timestamp,
        main: &MainCi,
        how: &str,
        fork: Option<&str>,
    ) {
        let now = self.now().as_str().to_string();
        let mut detail = BTreeMap::from([
            ("how".to_string(), how.to_string()),
            ("commit".to_string(), main.red_commit().to_string()),
        ]);
        if let Some(merge) = main.merge.as_ref() {
            detail.insert("merge".to_string(), merge.number.to_string());
        }
        if let Some(fork) = fork {
            detail.insert("fork".to_string(), fork.to_string());
        }
        // `spent`: a fact that stays, holding nothing to give back.
        let _ = self.store().lock().await.attach(
            &KeptAttachment {
                holder: Holder::session(session),
                kind: MAIN_RED.to_string(),
                manifest_id: served.manifest().id().as_str().to_string(),
                target: red_at.as_str().to_string(),
                state: AttachmentState::Spent,
                detail,
                since: now.clone(),
                changed_at: now,
            },
            false,
        );
    }
}
