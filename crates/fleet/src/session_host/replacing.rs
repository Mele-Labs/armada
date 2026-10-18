//! A Job hands on to a new one in its place, once the person has said yes on the Job's own question
//! card. `docs/concepts/session/a-handed-off-job.md`.
//!
//! **Fleet never starts the successor unasked.** The only way in is [`Fleet::handed_on`], called from
//! the permission door with the person's answer to a question that chose [`ipc::HANDOFF_YES`]. The
//! Job's call is held for the whole hand, so its agent cannot act between the yes and its end.
//!
//! **The order is the point.** The new Job is started first (a fork at the old one's latest message),
//! then the old Job's ledger is handed to it with `release_session_holds` and its target, then the old
//! Job is ended as `move_session` ends one. A new Job that cannot start leaves the old one as it was.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{CloseSession, ForkFrom, ManifestId, QuestionAnswer, ReleaseSessionHolds, RunOrNot, SessionId, StartSession};
use store::{AttachmentState, Holder, KeptAttachment};

use super::handing::said_of;
use super::serving::mode_of;
use crate::daemon::Fleet;

/// On the new Job's ledger, naming the Job it took the place of; `spent`, a fact that stays.
pub(crate) const REPLACES: &str = "replaces";
/// On the old Job's ledger, naming its successor; `spent`.
pub(crate) const REPLACED_BY: &str = "replaced_by";
/// On the old Job while the successor comes up: `standing`, settled when the hand ends either way.
const STARTING: &str = "handoff_starting";
/// On the old Job when the successor did not start: `standing` until the person next writes to it.
/// `detail.reason` says why. The target is the instant, so a second failure is a second row.
const FAILED: &str = "handoff_failed";

/// What a successor's agent is told it is: bare facts, and what came with it.
pub(crate) fn replace_note(old_title: Option<&str>, old: &str, when: &str) -> String {
    let title = old_title
        .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|title| !title.is_empty())
        .map(|title| format!("\"{title}\" "))
        .unwrap_or_default();
    format!(
        "This job took the place of {title}({old}) at {when}, and that job has ended. Your conversation up \
         to then is a copy of its. The slots, branches and pull requests it held were handed to you and are \
         yours: carry on in them. Nothing else it started is yours: its subagents, background tasks and \
         shells, windows and peer jobs stay its own, whatever your copy of the conversation says about \
         them. A task-notification about its background work belongs to it: do not act on it, relaunch \
         that work, or report it to the owner as yours. If it held no slot, your first write leases one."
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
    /// Whether the person chose to carry on in a new Job on a card.
    pub(crate) fn handoff_agreed(answers: &[QuestionAnswer]) -> bool {
        answers.iter().any(|one| one.chosen.iter().any(|label| label == ipc::HANDOFF_YES))
    }

    /// The person said yes to Job `old`'s card. Starts the successor, hands it the ledger and ends `old`;
    /// returns `decided` as it was, or a refusal the agent reads where the successor did not come up.
    pub(crate) async fn handed_on(&self, old: &str, decided: RunOrNot) -> RunOrNot {
        self.handoff_mark(old, STARTING, old, AttachmentState::Standing, Default::default()).await;
        let result = self.hand_off(old).await;
        self.handoff_settled(old, STARTING, old).await;
        match result {
            Ok(()) => decided,
            Err(why) => {
                let at = self.now().as_str().to_string();
                let detail = [(String::from("reason"), why.clone())].into();
                self.handoff_mark(old, FAILED, &at, AttachmentState::Standing, detail).await;
                let _ = self.published_hosted(old).await;
                RunOrNot::Deny {
                    message: String::from(
                        "The new job did not start, so nothing moved and you are still the job. Carry on here.",
                    ),
                }
            }
        }
    }

    async fn hand_off(&self, old: &str) -> Result<(), String> {
        let (session, hosting) = self.session_and_hosting(old).await.map_err(|why| said_of(&why))?;
        let at = self.latest_fork_point(old).await.map_err(|why| said_of(&why))?;
        let made = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(&hosting.manifest_id),
                title: session.title.clone(),
                model: hosting.model.clone(),
                effort: hosting.effort.clone(),
                mode: Some(mode_of(&hosting.mode)),
                root: None,
                fork: Some(ForkFrom { session_id: SessionId::carried(old), at: Some(at) }),
                request: None,
                after: None,
            })
            .await
            .map_err(|why| said_of(&why))?;
        let new = made.id.as_str().to_string();
        let handed = self
            .session_holds_released(ReleaseSessionHolds {
                session_id: SessionId::carried(old),
                to_session_id: Some(made.id.clone()),
                rows: None,
            })
            .await;
        if let Err(why) = handed {
            // The successor never held anything; it is let go and the old Job stands as it was.
            let _ = self.close_session(CloseSession { session_id: made.id }).await;
            return Err(said_of(&why));
        }
        let manifest = hosting.manifest_id.clone();
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            for (holder, kind, target) in [(new.as_str(), REPLACES, old), (old, REPLACED_BY, new.as_str())] {
                store
                    .attach(
                        &KeptAttachment {
                            holder: Holder::session(holder),
                            kind: kind.to_string(),
                            manifest_id: manifest.clone(),
                            target: target.to_string(),
                            state: AttachmentState::Spent,
                            detail: Default::default(),
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .map_err(|why| format!("{why:?}"))?;
            }
        }
        // Ended as a move ends a Job. What it still ran goes with the process; the hand left it no slot.
        self.close_session(CloseSession { session_id: SessionId::carried(old) }).await.map_err(|why| said_of(&why))?;
        let _ = self.published_hosted(&new).await;
        Ok(())
    }

    /// The failure fact is the person's to have seen: his next message to the Job puts it away.
    pub(crate) async fn handoff_failure_cleared(&self, session: &str) {
        let now = self.now().as_str().to_string();
        let cleared = self.store().lock().await.settle_held(&Holder::session(session), Some(FAILED), AttachmentState::GivenBack, &now);
        if cleared.unwrap_or(false) {
            let _ = self.published_hosted(session).await;
        }
    }

    async fn handoff_mark(
        &self,
        session: &str,
        kind: &str,
        target: &str,
        state: AttachmentState,
        detail: std::collections::BTreeMap<String, String>,
    ) {
        let Ok((row, _)) = self.session_and_hosting(session).await else { return };
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.attach(
            &KeptAttachment {
                holder: Holder::session(session),
                kind: kind.to_string(),
                manifest_id: row.manifest_id.unwrap_or_default(),
                target: target.to_string(),
                state,
                detail,
                since: now.clone(),
                changed_at: now,
            },
            false,
        );
        let _ = self.published_hosted(session).await;
    }

    async fn handoff_settled(&self, session: &str, kind: &str, target: &str) {
        let Ok((row, _)) = self.session_and_hosting(session).await else { return };
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.settle(
            &Holder::session(session),
            kind,
            &row.manifest_id.unwrap_or_default(),
            target,
            AttachmentState::GivenBack,
            &now,
        );
    }
}
