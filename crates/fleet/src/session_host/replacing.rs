//! A Job hands on to a new one in its place, once the person has said yes on the Job's own question
//! card, presses Retry on the band of a hand that failed, or set Large context to carry on
//! automatically. `docs/concepts/session/a-handed-off-job.md`.
//!
//! **Fleet never starts the successor unasked.** The ways in are [`Fleet::handed_on`], called from
//! the permission door with the person's answer to a question that chose [`ipc::HANDOFF_YES`], or
//! from his message answering such a question kept for a walk (`handoff_answered_later`),
//! [`Fleet::handoff_retried`], the person's press on a standing `handoff_failed` row, and
//! [`Fleet::handed_on_by_setting`], the Large context setting at carry on automatically. **The hand runs
//! on a task of its own, never on the Job's call.** Ending the old Job ends the process the call came
//! from; the connection goes with it, and the server drops a hand run on that call partway, leaving
//! a `handoff_starting` that never settles and a successor with no link (19 Oct 2026). The call is
//! answered first, telling the agent the work moved and to stop.
//!
//! **The successor is a fresh Job, not a fork.** A fork would carry the whole thread and start as
//! heavy as the Job it replaces. What it starts from is [`Fleet::handoff_of`] (ids, where the work
//! is, the last of the thread) and the brief the Job wrote in the card, both kept on its `replaces`
//! row and told to every process of it in the system prompt.
//!
//! **The order is the point.** The new Job is started first, then the two are linked and published,
//! so Bridge folds the old Job behind the new one at once and the new Job's first process is told the
//! handoff. Then the old Job's ledger is handed on with `release_session_holds` and its target, the
//! old Job is ended as `move_session` ends one, and the new Job is told to carry on, so the person
//! need not write first. A failure at any step puts back what moved and closes the new Job, so the
//! old Job stands as it was and there are never two live ones. A Fleet that stopped mid-hand does
//! the same at its next start: [`Fleet::handoffs_healed`].

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    CloseSession, ManifestId, ReleaseSessionHolds, RetryHandoff, RunOrNot, SessionId,
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
pub(crate) const STARTING: &str = "handoff_starting";
/// On the old Job when the successor did not start: `standing` until the person next writes to it or
/// retries. `detail.reason` says why, `detail.brief` keeps the brief for a retry. The target is the
/// instant, so a second failure is a second row.
const FAILED: &str = "handoff_failed";
const REASON: &str = "reason";
const BRIEF: &str = "brief";
const CONTEXT: &str = "context";
/// On `handoff_starting` once the successor is made: its id, which the boot pass reads.
const NEW: &str = "new";

/// The reason a hand that a stopped Fleet left standing is given at the next start.
const STOPPED: &str = "fleet stopped during the hand off";
/// How long after a hand began a successor an older Fleet made, which wrote no id, was started.
const MADE_WITHIN_MS: i64 = 60_000;

/// What the old Job's agent reads as the answer to its card.
const MOVED: &str = "The work moved to a new job, which carries on from your brief. Stop here: end your turn now and make no more tool calls.";
/// The new Job's first turn, so it carries on without the person writing first.
const CARRY_ON: &str = "Carry on with the work of the job you took the place of. What it left you, with its brief if it wrote one, is in your instructions.";

const NOT_FAILED: &str = "fleet.handoff_not_failed";
const STILL_RUNNING: &str = "fleet.handoff_running";
const HANDOFF_FAILED: &str = "fleet.handoff_failed";

/// How much of a brief is kept: a brief is where the work stands, not the work.
const BRIEF_CHARS: usize = 6_000;

/// What a Job is told when the Large context setting carries it on with no card: no card carries a
/// brief, so it is asked for one.
pub(crate) const BRIEF_ASKED: &str = "Reply with your brief only: where the work is, what is open, what you hold. Make no tool calls. Your context is past the limit, and this job carries on in a new one from your reply.";
/// How long the hand waits for that reply before it carries on with the composed context alone.
const BRIEF_WAIT: std::time::Duration = std::time::Duration::from_secs(180);
const BRIEF_POLL: std::time::Duration = std::time::Duration::from_millis(200);

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

/// A mark's detail that keeps the brief, so a hand that fails, or a Fleet that stops, keeps it for Retry.
fn brief_of(brief: &Option<String>) -> BTreeMap<String, String> {
    brief.iter().map(|brief| (String::from(BRIEF), brief.clone())).collect()
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
    /// The brief the Job wrote on its card: the description of the option that carries on. Read off the
    /// question Fleet holds, so nothing the agent calls says it.
    pub(crate) fn handoff_brief(questions: &[ipc::AskedQuestion]) -> Option<String> {
        let said = questions
            .iter()
            .flat_map(|question| question.options.iter())
            .find(|option| option.label == ipc::HANDOFF_YES)?
            .description
            .trim();
        (!said.is_empty()).then(|| said.chars().take(BRIEF_CHARS).collect())
    }

    /// The person said yes to Job `old`'s card. Marks the hand running, starts it on a task of its own
    /// and answers the call at once: the hand ends the process this call came from.
    pub(crate) async fn handed_on(self: &Arc<Self>, old: &str, brief: Option<String>) -> RunOrNot {
        self.handoff_mark(old, STARTING, old, AttachmentState::Standing, brief_of(&brief)).await;
        let (fleet, old) = (Arc::clone(self), old.to_string());
        tokio::spawn(async move { fleet.handing_on(&old, brief).await });
        RunOrNot::Deny { message: String::from(MOVED) }
    }

    /// [`Self::handed_on`] on a task of its own, for a caller with no call to answer. **Not async, and
    /// the task boxed**: the hand starts a Job, whose messages come back through the caller, and an
    /// awaited hand would make the caller's future contain itself.
    pub(crate) fn handed_on_apart(self: &Arc<Self>, old: &str, brief: Option<String>) {
        let (fleet, old) = (Arc::clone(self), old.to_string());
        let hand: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> = Box::pin(async move {
            fleet.handed_on(&old, brief).await;
        });
        tokio::spawn(hand);
    }

    /// The Context rule tripped with the Large context setting at carry on automatically, the owner's
    /// standing consent: the same hand as his yes, on a task of its own, with the brief the Job
    /// replies to [`BRIEF_ASKED`]. Nothing else starts a successor unasked.
    pub(crate) async fn handed_on_by_setting(self: &Arc<Self>, old: &str) {
        self.handoff_mark(old, STARTING, old, AttachmentState::Standing, BTreeMap::new()).await;
        let (fleet, old) = (Arc::clone(self), old.to_string());
        tokio::spawn(async move {
            let brief = fleet.brief_replied(&old).await;
            let _ = fleet.handing_on(&old, brief).await;
        });
    }

    /// What the Job replied to [`BRIEF_ASKED`], once its turn is over: its messages after the ask.
    /// `None` where it was not told, or said nothing inside [`BRIEF_WAIT`].
    async fn brief_replied(self: &Arc<Self>, old: &str) -> Option<String> {
        // The brief is waited for, so only a Session that has the ask is waited on.
        if !matches!(self.noticed_by_fleet(old, BRIEF_ASKED, ipc::Notice::ContextHandoff).await, Ok(told) if told.sent()) {
            return None;
        }
        let until = tokio::time::Instant::now() + BRIEF_WAIT;
        while tokio::time::Instant::now() < until {
            tokio::time::sleep(BRIEF_POLL).await;
            if !matches!(self.hosts().of(old).state().turn, ipc::SessionTurn::Idle) {
                continue;
            }
            let rows = self.rows_of(old).await.ok()?;
            let asked = rows.iter().rposition(|row| matches!(row, ipc::SessionRow::Message { text, .. } if text == BRIEF_ASKED))?;
            let said: Vec<&str> = rows[asked + 1..]
                .iter()
                .filter_map(|row| match row {
                    ipc::SessionRow::Message { text, from: ipc::SessionVoice::Agent, .. } => Some(text.trim()),
                    _ => None,
                })
                .filter(|text| !text.is_empty())
                .collect();
            if !said.is_empty() {
                return Some(said.join("\n\n").chars().take(BRIEF_CHARS).collect());
            }
        }
        None
    }

    /// `retry_handoff`: the person's press on a hand that failed. The same steps as the yes, with the
    /// brief the failed row kept; his press is the consent, and no card is raised. Answers the new Job.
    pub(crate) async fn handoff_retried(self: Arc<Self>, retry: RetryHandoff) -> Result<SessionRecord, Refusal> {
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
        self.handoff_mark(&old, STARTING, &old, AttachmentState::Standing, brief_of(&brief)).await;
        // On a task of its own, as the yes is, so a request dropped partway does not drop the hand.
        let (fleet, from) = (Arc::clone(&self), old.clone());
        let handed = tokio::spawn(async move { fleet.handing_on(&from, brief).await })
            .await
            .unwrap_or_else(|_| Err(String::from("hand off did not finish")));
        match handed {
            Ok(new) => self.published_hosted(&new).await,
            Err(why) => Err(self.hosted_refusal(HANDOFF_FAILED, &why)),
        }
    }

    /// One hand, from the mark its caller made to the last: `Ok` is the new Job's id; `Err` is the
    /// reason, which stands on the old Job as a fresh `handoff_failed` row in place of any earlier one.
    async fn handing_on(self: &Arc<Self>, old: &str, brief: Option<String>) -> Result<String, String> {
        // Queued as the hand starts, so what the old Job learnt is distilled even where the hand fails.
        self.logbook_distils().queue(old);
        let result = self.hand_off(old, brief.as_deref()).await;
        self.handoff_ended(old, result.as_ref().err().map(String::as_str), brief.as_deref()).await;
        result
    }

    /// The `handoff_starting` mark settled, and the failure, if there was one, standing in place of any
    /// earlier one.
    async fn handoff_ended(&self, old: &str, failed: Option<&str>, brief: Option<&str>) {
        self.handoff_settled(old, STARTING, old).await;
        let now = self.now().as_str().to_string();
        let _ = self.store().lock().await.settle_held(&Holder::session(old), Some(FAILED), AttachmentState::GivenBack, &now);
        if let Some(why) = failed {
            let mut detail = brief_of(&brief.map(String::from));
            detail.insert(String::from(REASON), why.to_string());
            self.handoff_mark(old, FAILED, &now, AttachmentState::Standing, detail).await;
        }
        let _ = self.published_hosted(old).await;
    }

    async fn hand_off(self: &Arc<Self>, old: &str, brief: Option<&str>) -> Result<String, String> {
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
        let mut marked = brief_of(&brief.map(String::from));
        marked.insert(String::from(NEW), new.clone());
        self.handoff_mark(old, STARTING, old, AttachmentState::Standing, marked).await;
        if let Err(why) = self.linked(old, &new, &hosting.manifest_id, context).await {
            self.handoff_undone(old, &new).await;
            return Err(why);
        }
        let _ = self.published_hosted(old).await;
        let _ = self.published_hosted(&new).await;
        let handed = self
            .session_holds_released(ReleaseSessionHolds {
                session_id: SessionId::carried(old),
                to_session_id: Some(made.id.clone()),
                rows: None,
            })
            .await;
        if let Err(why) = handed {
            self.handoff_undone(old, &new).await;
            return Err(reason_of("ledger did not move", &said_of(&why)));
        }
        if let Err(why) = self.old_ended(old).await {
            self.handoff_undone(old, &new).await;
            return Err(why);
        }
        // Its first process reads the links, so it is told the handoff and the brief.
        let _ = self.told_by_fleet(&new, CARRY_ON, ipc::Notice::ContextHandoff).await;
        let _ = self.published_hosted(&new).await;
        Ok(new)
    }

    /// The two links. `replaces` carries what the new Job starts from.
    async fn linked(&self, old: &str, new: &str, manifest: &str, context: String) -> Result<(), String> {
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
        Ok(())
    }

    async fn old_ended(&self, old: &str) -> Result<(), String> {
        if let Some(why) = faulted("close") {
            return Err(reason_of("old job did not end", &why));
        }
        // Ended as a move ends a Job. What it still ran goes with the process; the hand left it no slot.
        self.closed_and_parked(CloseSession { session_id: SessionId::carried(old) })
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
        let _ = self.closed_and_parked(CloseSession { session_id: SessionId::carried(new) }).await;
    }

    /// At Fleet's start: a `handoff_starting` still standing is a hand a Fleet that stopped left
    /// partway. Where the old Job had ended, the new one carries on and is told to. Otherwise what
    /// moved is put back, the new Job is closed and the old one stands with `handoff_failed`, for its
    /// Retry. Either way one Job is live. How many were settled.
    pub async fn handoffs_healed(self: &Arc<Self>) -> usize {
        let standing: Vec<KeptAttachment> = {
            let store = self.store().lock().await;
            let manifests: std::collections::BTreeSet<String> =
                store.hostings().unwrap_or_default().into_iter().map(|one| one.manifest_id).collect();
            manifests.iter().flat_map(|manifest| store.standing_of_kind(STARTING, manifest).unwrap_or_default()).collect()
        };
        for mark in &standing {
            let old = mark.holder.id.as_str();
            let new = self.successor_left(mark).await;
            let ended = self.session_and_hosting(old).await.is_ok_and(|(row, _)| row.state == store::SessionState::Ended);
            match (ended, &new) {
                (true, Some(new)) => {
                    self.handoff_ended(old, None, None).await;
                    let _ = self.told_by_fleet(new, CARRY_ON, ipc::Notice::ContextHandoff).await;
                }
                _ => {
                    if let Some(new) = &new {
                        self.handoff_undone(old, new).await;
                    }
                    self.handoff_ended(old, Some(STOPPED), mark.detail.get(BRIEF).map(String::as_str)).await;
                }
            }
        }
        standing.len()
    }

    /// The successor a hand left partway made, if it made one that is still live: named on the mark, or
    /// by the old Job's link, or, for a Fleet before marks named it, the Job of the same repository and
    /// title started within a minute of the mark.
    async fn successor_left(&self, mark: &KeptAttachment) -> Option<String> {
        let old = mark.holder.id.as_str();
        let store = self.store().lock().await;
        let live = |id: &str| store.session(id).ok().flatten().is_some_and(|row| row.state != store::SessionState::Ended);
        let linked = || {
            store.attachments_of(&Holder::session(old)).ok()?.into_iter().find(|one| one.kind == REPLACED_BY && one.state == AttachmentState::Spent).map(|one| one.target)
        };
        if let Some(new) = mark.detail.get(NEW).cloned().or_else(linked) {
            return live(&new).then_some(new);
        }
        let title = store.session(old).ok().flatten()?.title;
        let since = core_model::Timestamp::from_rfc3339(mark.since.clone()).epoch_millis()?;
        store
            .hostings()
            .ok()?
            .into_iter()
            .filter(|one| one.manifest_id == mark.manifest_id && one.session_id != old)
            .filter_map(|one| store.session(&one.session_id).ok().flatten())
            .filter(|row| row.state != store::SessionState::Ended && row.title == title)
            .filter_map(|row| {
                let at = core_model::Timestamp::from_rfc3339(row.started_at.clone()).epoch_millis()?;
                (at >= since && at - since <= MADE_WITHIN_MS).then_some((at, row.id))
            })
            .min()
            .map(|(_, id)| id)
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
