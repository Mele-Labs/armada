//! A Job hands on to a new one in its place, once the person has said yes on the Job's own question
//! card, or presses Retry on the band of a hand that failed. `docs/concepts/session/a-handed-off-job.md`.
//!
//! **Fleet never starts the successor unasked.** The ways in are [`Fleet::handed_on`], called from
//! the permission door with the person's answer to a question that chose [`ipc::HANDOFF_YES`], and
//! [`Fleet::handoff_retried`], the person's press on a standing `handoff_failed` row. The Job's call
//! is held for the whole hand, so its agent cannot act between the yes and its end.
//!
//! **The successor is a fresh Job, not a fork.** A fork would carry the whole thread and start as
//! heavy as the Job it replaces. What it starts from is [`Fleet::handoff_of`] (ids, where the work
//! is, the last of the thread) and the brief the Job wrote in the card, both kept on its `replaces`
//! row and told to every process of it in the system prompt.
//!
//! **The order is the point.** The new Job is started first, then the old Job's ledger is handed to
//! it with `release_session_holds` and its target, then the two are linked, then the old Job is ended
//! as `move_session` ends one. A failure at any step puts back what moved and closes the new Job, so
//! the old Job stands as it was and there are never two live ones.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AskingToRun, CloseSession, ManifestId, QuestionAnswer, ReleaseSessionHolds, RetryHandoff, RunOrNot, SessionId,
    SessionRecord, StartSession,
};
use store::{AttachmentState, Holder, KeptAttachment};

use super::handing::said_of;
use super::serving::mode_of;
use crate::daemon::Fleet;

/// On the new Job's ledger, naming the Job it took the place of; `spent`, a fact that stays.
/// `detail.context` is what the new Job starts from.
pub(crate) const REPLACES: &str = "replaces";
/// On the old Job's ledger, naming its successor; `spent`.
pub(crate) const REPLACED_BY: &str = "replaced_by";
/// On the old Job while the successor comes up: `standing`, settled when the hand ends either way.
const STARTING: &str = "handoff_starting";
/// On the old Job when the successor did not start: `standing` until the person next writes to it or
/// retries. `detail.reason` says why, `detail.brief` keeps the brief for a retry. The target is the
/// instant, so a second failure is a second row.
const FAILED: &str = "handoff_failed";
const REASON: &str = "reason";
const BRIEF: &str = "brief";
const CONTEXT: &str = "context";

const NOT_FAILED: &str = "fleet.handoff_not_failed";
const STILL_RUNNING: &str = "fleet.handoff_running";
const HANDOFF_FAILED: &str = "fleet.handoff_failed";

/// How much of a brief is kept: a brief is where the work stands, not the work.
const BRIEF_CHARS: usize = 6_000;

/// What a successor's agent is told it is: bare facts, and what came with it.
pub(crate) fn replace_note(old_title: Option<&str>, old: &str, when: &str, context: &str) -> String {
    let title = old_title
        .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|title| !title.is_empty())
        .map(|title| format!("\"{title}\" "))
        .unwrap_or_default();
    format!(
        "This job took the place of {title}({old}) at {when}, and that job has ended. You are a new job: \
         you start without its conversation, and what it left for you is below. The slots, branches and \
         pull requests it held were handed to you and are yours: carry on in them. Nothing else it started \
         is yours: its subagents, background tasks and shells, windows and peer jobs stay its own. A \
         task-notification about its background work belongs to it: do not act on it, relaunch that work, \
         or report it to the owner as yours. If it held no slot, your first write leases one.\n\n{context}"
    )
}

// The step a test has broken by arrangement (`start` or `close`), where the real faults cannot be
// made. Never set outside a test.
#[cfg(test)]
thread_local! {
    pub(crate) static FAULT: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
fn faulted(at: &str) -> Option<String> {
    (FAULT.with(|one| one.get()) == Some(at)).then(|| format!("{at} refused by arrangement"))
}

#[cfg(not(test))]
fn faulted(_: &str) -> Option<String> {
    None
}

/// A reason for the band: a bare fact, lowercase at its start.
fn reason_of(what: &str, why: &str) -> String {
    let mut chars = why.trim().chars();
    let tail = match chars.next() {
        Some(first) => format!("{}{}", first.to_lowercase(), chars.as_str()),
        None => String::new(),
    };
    format!("{what}: {tail}")
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

    /// The brief the Job wrote on its card: the description of the option that carries on. Read off the
    /// question Fleet holds, so nothing the agent calls says it.
    pub(crate) fn handoff_brief(asking: &AskingToRun) -> Option<String> {
        let said = asking
            .questions()
            .into_iter()
            .flat_map(|question| question.options)
            .find(|option| option.label == ipc::HANDOFF_YES)?
            .description;
        let said = said.trim();
        (!said.is_empty()).then(|| said.chars().take(BRIEF_CHARS).collect())
    }

    /// The person said yes to Job `old`'s card. Starts the successor, hands it the ledger and ends `old`;
    /// returns `decided` as it was, or a refusal the agent reads where the successor did not come up.
    pub(crate) async fn handed_on(&self, old: &str, decided: RunOrNot, brief: Option<String>) -> RunOrNot {
        match self.handing_on(old, brief).await {
            Ok(_) => decided,
            Err(_) => RunOrNot::Deny {
                message: String::from(
                    "The new job did not start, so nothing moved and you are still the job. Carry on here.",
                ),
            },
        }
    }

    /// `retry_handoff`: the person's press on a hand that failed. The same steps as the yes, with the
    /// brief the failed row kept; his press is the consent, and no card is raised. Answers the new Job.
    pub(crate) async fn handoff_retried(&self, retry: RetryHandoff) -> Result<SessionRecord, Refusal> {
        let old = retry.session_id.as_str().to_string();
        self.session_and_hosting(&old).await?;
        let held = self
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(&old))
            .map_err(|why| self.ledger_fault(why))?;
        let standing = |kind: &str| held.iter().find(|one| one.kind == kind && one.state == AttachmentState::Standing);
        if standing(STARTING).is_some() {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                STILL_RUNNING,
                format!("job {old} is handing on already"),
                self.run_id(),
            )));
        }
        let Some(failed) = standing(FAILED) else {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                NOT_FAILED,
                format!("job {old} has no failed hand off to retry"),
                self.run_id(),
            )));
        };
        let brief = failed.detail.get(BRIEF).cloned();
        match self.handing_on(&old, brief).await {
            Ok(new) => self.published_hosted(&new).await,
            Err(why) => Err(self.hosted_refusal(HANDOFF_FAILED, &why)),
        }
    }

    /// One hand, from the first mark to the last: `Ok` is the new Job's id; `Err` is the reason, which
    /// stands on the old Job as a fresh `handoff_failed` row in place of any earlier one.
    async fn handing_on(&self, old: &str, brief: Option<String>) -> Result<String, String> {
        self.handoff_mark(old, STARTING, old, AttachmentState::Standing, Default::default()).await;
        let result = self.hand_off(old, brief.as_deref()).await;
        self.handoff_settled(old, STARTING, old).await;
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.settle_held(&Holder::session(old), Some(FAILED), AttachmentState::GivenBack, &now);
        if let Err(why) = &result {
            let mut detail: std::collections::BTreeMap<String, String> = [(String::from(REASON), why.clone())].into();
            if let Some(brief) = &brief {
                detail.insert(String::from(BRIEF), brief.clone());
            }
            self.handoff_mark(old, FAILED, &now, AttachmentState::Standing, detail).await;
        }
        let _ = self.published_hosted(old).await;
        result
    }

    async fn hand_off(&self, old: &str, brief: Option<&str>) -> Result<String, String> {
        let (session, hosting) = self
            .session_and_hosting(old)
            .await
            .map_err(|why| reason_of("job not found", &said_of(&why)))?;
        // Composed before anything moves, so it lists what the old Job holds.
        let mut context = self.handoff_of(old).await.map_err(|why| reason_of("thread not read", &said_of(&why)))?;
        if let Some(brief) = brief {
            let title = session.title.clone().unwrap_or_else(|| old.to_string());
            context.push_str(&format!("\n\nBrief from {title}:\n{brief}"));
        }
        if let Some(why) = faulted("start") {
            return Err(reason_of("new job did not start", &why));
        }
        let made = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(&hosting.manifest_id),
                title: session.title.clone(),
                model: hosting.model.clone(),
                effort: hosting.effort.clone(),
                mode: Some(mode_of(&hosting.mode)),
                root: None,
                fork: None,
                request: None,
                after: None,
            })
            .await
            .map_err(|why| reason_of("new job did not start", &said_of(&why)))?;
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
            return Err(reason_of("ledger did not move", &said_of(&why)));
        }
        if let Err(why) = self.linked_and_ended(old, &new, &hosting.manifest_id, context).await {
            self.handoff_undone(old, &new).await;
            return Err(why);
        }
        let _ = self.published_hosted(&new).await;
        Ok(new)
    }

    /// The links, then the old Job's end. `replaces` carries what the new Job starts from.
    async fn linked_and_ended(&self, old: &str, new: &str, manifest: &str, context: String) -> Result<(), String> {
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let with_context: std::collections::BTreeMap<String, String> = [(String::from(CONTEXT), context)].into();
            for (holder, kind, target, detail) in
                [(new, REPLACES, old, with_context), (old, REPLACED_BY, new, Default::default())]
            {
                store
                    .attach(
                        &KeptAttachment {
                            holder: Holder::session(holder),
                            kind: kind.to_string(),
                            manifest_id: manifest.to_string(),
                            target: target.to_string(),
                            state: AttachmentState::Spent,
                            detail,
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .map_err(|why| reason_of("jobs not linked", &format!("{why:?}")))?;
            }
        }
        if let Some(why) = faulted("close") {
            return Err(reason_of("old job did not end", &why));
        }
        // Ended as a move ends a Job. What it still ran goes with the process; the hand left it no slot.
        self.close_session(CloseSession { session_id: SessionId::carried(old) })
            .await
            .map_err(|why| reason_of("old job did not end", &said_of(&why)))?;
        Ok(())
    }

    /// Puts back what a hand that failed moved: the ledger goes to the old Job again, the links are
    /// given back (Bridge draws only spent ones) and the new Job is closed. Best effort, in that order,
    /// so the last step still leaves one live Job where an earlier one cannot be undone.
    async fn handoff_undone(&self, old: &str, new: &str) {
        let _ = self
            .session_holds_released(ReleaseSessionHolds {
                session_id: SessionId::carried(new),
                to_session_id: Some(SessionId::carried(old)),
                rows: None,
            })
            .await;
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let manifest = store.session(old).ok().flatten().and_then(|one| one.manifest_id).unwrap_or_default();
            for (holder, kind, target) in [(new, REPLACES, old), (old, REPLACED_BY, new)] {
                let _ = store.settle(&Holder::session(holder), kind, &manifest, target, AttachmentState::GivenBack, &now);
            }
        }
        let _ = self.close_session(CloseSession { session_id: SessionId::carried(new) }).await;
    }

    /// What every process of the new Job `id` is told it is: what it took the place of, and what the
    /// old Job left it. Read at each start, so the note is the same after a restart.
    pub(crate) async fn successor_note_of(&self, id: &str) -> Option<String> {
        let store = self.store().lock().await;
        let held = store.attachments_of(&Holder::session(id)).ok()?;
        let link = held.into_iter().find(|one| one.kind == REPLACES && one.state == AttachmentState::Spent)?;
        let title = store.session(&link.target).ok().flatten().and_then(|one| one.title);
        let context = link.detail.get(CONTEXT).map(String::as_str).unwrap_or_default();
        Some(replace_note(title.as_deref(), &link.target, &link.since, context))
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
