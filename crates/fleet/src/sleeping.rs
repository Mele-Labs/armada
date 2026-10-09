//! Sleep mode: the owner turns it on before bed, and sessions and Jobs keep working until only he can
//! unblock them. `docs/concepts/session.md`.
//!
//! **What the night may decide.** A question a session asked, once it has waited out [`GRACE`] so the
//! agent could carry on by itself first: the option labelled "(Recommended)", else `mode: best`. **Never
//! a destructive or irreversible one, and never a permission.** Those are held under blocked.
//!
//! **An escalated Job** whose trigger words can carry past ([`ANSWERABLE`]) and whose title is not
//! destructive is told to decide for itself, by the owner's own act: a redirect to its Drone, or a
//! restart of the step with the same words where the Drone is gone. One whose trigger a second run
//! may well clear ([`RETRIED`]) has its step restarted, with no words. Any other escalation is held.
//!
//! **Each thing is left once.** A row's id is `<session>:<item>`, so a pass over an item already
//! decided or held changes nothing.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use core_model::{Actor, EscalationTrigger, JobStatus, TransitionReason};
use ipc::{
    AnswerWaiting, OverrideSleep, SendSessionMessage, SleepBlocked, SleepDecided, SleepLanded, SleepState,
    SleepWalk, WaitingItem, WaitingMode, WaitingSource, WireError,
};
use store::{SessionState, SleepRow};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::{Ending, Redirection, Steer};
use crate::session_host::waiting::mode_line;

/// How long an item waits before the night answers it.
pub const GRACE: Duration = Duration::from_secs(120);

/// How often the pass looks.
pub const EVERY: Duration = Duration::from_secs(10);

const SLEEP_NO_DECISION: &str = "fleet.sleep_no_decision";
const SLEEP_EMPTY: &str = "fleet.sleep_empty";

const RECOMMENDED: &str = "(recommended)";

/// Words that mark a question as one the night must not answer. **Conservative on purpose**: a word
/// found inside another ("enforce") holds the question, and a held question costs the owner a glance.
const DESTRUCTIVE: &[&str] = &[
    "delete", "drop", "remove", "force", "reset", "overwrite", "irreversible", "destructive", "discard",
    "destroy", "wipe", "purge", "erase", "truncate", "prune", "clobber", "unrecoverable", "permanent",
    "uninstall", "revert", "--hard", "close pr", "close the pr", "close pull", "close the pull", "close the issue",
];

/// Whole words that are too short to match inside another.
const DESTRUCTIVE_WORDS: &[&str] = &["rm", "kill", "undo"];

/// The escalations a person's words can carry a Job past: a Drone that stopped, went quiet, looped,
/// failed a gate, asked and went unanswered, or was refused a hatch, its scope or its evidence. The rest
/// (a dependency that failed, no worktree, a cap, a policy) are not cured by a sentence.
const ANSWERABLE: &[EscalationTrigger] = &[
    EscalationTrigger::Stalled,
    EscalationTrigger::Silent,
    EscalationTrigger::Thrashing,
    EscalationTrigger::NoReport,
    EscalationTrigger::Interrupted,
    EscalationTrigger::LoopCap,
    EscalationTrigger::GateFailure,
    EscalationTrigger::AskUnanswered,
    EscalationTrigger::HatchUnbidden,
    EscalationTrigger::ScopeRefused,
    EscalationTrigger::EvidenceTooLarge,
];

/// The escalations a plain second run may clear, with nothing to say: the Drone or run ended, was not
/// heard, would not start, or the gate or proposer gave no answer. The step is restarted, once a night
/// (the Job's `job:` row stops a second restart); an undecided gate is asked again instead, the work
/// being fine. `Unheard`, `WouldNotStart`, `NotPrepared` and `ProposerFailed` are Job-level, with no
/// stopped step to restart, so they usually end as a blocked row carrying the refusal.
const RETRIED: &[EscalationTrigger] = &[
    EscalationTrigger::DroneGone,
    EscalationTrigger::RunEnded,
    EscalationTrigger::Unheard,
    EscalationTrigger::WouldNotStart,
    EscalationTrigger::NotPrepared,
    EscalationTrigger::GateUndecided,
    EscalationTrigger::ProposerFailed,
];

/// How much of the agent's next words the review shows.
const CHOSE_CHARS: usize = 120;

/// Whether the item, read with its options, may be destructive or irreversible.
pub(crate) fn destructive(item: &WaitingItem) -> bool {
    let mut text = item.text.clone();
    for option in &item.options {
        text.push('\n');
        text.push_str(&option.label);
    }
    destructive_text(&text)
}

/// [`destructive`] over plain text.
fn destructive_text(text: &str) -> bool {
    let text = text.to_lowercase();
    DESTRUCTIVE.iter().any(|word| text.contains(word))
        || text
            .split(|one: char| !one.is_alphanumeric())
            .any(|word| DESTRUCTIVE_WORDS.contains(&word))
}

/// The index of the option labelled "(Recommended)", if one is.
fn recommended(item: &WaitingItem) -> Option<usize> {
    item.options
        .iter()
        .position(|one| one.label.to_lowercase().contains(RECOMMENDED))
}

fn wire_of(switch: &store::SleepSwitch, rows: Vec<SleepRow>) -> SleepState {
    let mut state = SleepState { on: switch.on, ..SleepState::default() };
    for row in rows {
        match row.kind.as_str() {
            "decided" => state.decided.push(SleepDecided {
                id: row.id,
                who: row.who,
                asked: row.text,
                chose: row.chose.unwrap_or_default(),
                corrected: row.corrected,
            }),
            "blocked" => state.blocked.push(SleepBlocked { id: row.id, who: row.who, text: row.text }),
            "landed" => state.landed.push(SleepLanded {
                id: row.id,
                who: row.who,
                title: row.text,
                pr: row.pr.unwrap_or_default(),
            }),
            "walk" => state.walks.push(SleepWalk { id: row.id, who: row.who, title: row.text }),
            _ => {}
        }
    }
    state
}

fn row(kind: &str, id: String, who: String, text: String) -> SleepRow {
    SleepRow { id, kind: kind.into(), who, text, session_id: None, chose: None, corrected: None, pr: None }
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
    /// `get_sleep`.
    pub(crate) async fn sleep_state(&self) -> Result<SleepState, Refusal> {
        let store = self.store().lock().await;
        let switch = store.sleep_switch().map_err(|why| self.ledger_fault(why))?;
        let rows = store.sleep_rows().map_err(|why| self.ledger_fault(why))?;
        Ok(wire_of(&switch, rows))
    }

    async fn sleep_published(&self) -> Result<SleepState, Refusal> {
        let state = self.sleep_state().await?;
        self.publish(ipc::Event::SleepChanged(state.clone()));
        Ok(state)
    }

    /// `set_sleep`. Setting it to what it already is changes nothing and publishes nothing.
    pub(crate) async fn switch_sleep(&self, on: bool) -> Result<SleepState, Refusal> {
        {
            let mut store = self.store().lock().await;
            let now = store.sleep_switch().map_err(|why| self.ledger_fault(why))?;
            if now.on == on {
                drop(store);
                return self.sleep_state().await;
            }
            if on {
                store.begin_sleep(self.now().as_str())
            } else {
                store.end_sleep()
            }
            .map_err(|why| self.ledger_fault(why))?;
        }
        self.sleep_published().await
    }

    /// `override_sleep`: the correction goes to the session as `Re: <asked>`, then the words.
    pub(crate) async fn override_a_decision(self: Arc<Self>, over: OverrideSleep) -> Result<SleepState, Refusal> {
        let words = over.text.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(SLEEP_EMPTY, "a correction needs words"));
        }
        let held = self.store().lock().await.sleep_row(&over.id).map_err(|why| self.ledger_fault(why))?;
        let Some((held, session)) = held.and_then(|one| {
            let session = one.session_id.clone()?;
            (one.kind == "decided").then_some((one, session))
        }) else {
            return Err(Refusal::IllegalMove(WireError::raised(
                SLEEP_NO_DECISION,
                "no decision of the night holds that id",
                self.run_id(),
            )));
        };
        Arc::clone(&self)
            .send_session_message(SendSessionMessage {
                session_id: ipc::SessionId::carried(session),
                text: format!("Re: {}\n{}", held.text, words),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await?;
        self.store()
            .lock()
            .await
            .correct_sleep_row(&over.id, &words)
            .map_err(|why| self.ledger_fault(why))?;
        self.sleep_published().await
    }

    /// Leave a row once, and say so on the stream where it was new.
    async fn leave(&self, one: SleepRow) -> Result<(), Refusal> {
        let new = self.store().lock().await.add_sleep_row(&one).map_err(|why| self.ledger_fault(why))?;
        if new {
            self.sleep_published().await?;
        }
        Ok(())
    }

    /// A pull request that merged: a landed row while the night is on.
    pub(crate) async fn sleep_landed(&self, who: String, title: String, url: &str) {
        let pr = match url.rsplit('/').next().filter(|last| last.chars().all(|c| c.is_ascii_digit())) {
            Some(number) if !number.is_empty() => format!("#{number}"),
            _ => url.to_string(),
        };
        let one = SleepRow { pr: Some(pr), ..row("landed", format!("landed:{url}"), who, title) };
        if self.sleeping().await {
            let _ = self.leave(one).await;
        }
    }

    /// An escalated Job the night will not answer: blocked. One it will answer is left to the pass.
    pub(crate) async fn sleep_escalated(&self, job: &core_model::Job) {
        if !self.sleeping().await {
            return;
        }
        let (id, title) = (job.id().as_str(), job.title().as_str());
        let answerable = match self.last_reason(job.id()).await {
            Ok(Some(TransitionReason::Escalation(trigger))) => ANSWERABLE.contains(&trigger) || RETRIED.contains(&trigger),
            _ => false,
        };
        if !answerable || destructive_text(title) {
            let text = format!("Escalated, waiting on you: {title}");
            let _ = self.leave(row("blocked", format!("job:{id}"), job.handle(), text)).await;
        }
    }

    /// Escalated Jobs, each once: told to decide for itself, or held.
    async fn sleep_jobs(&self) -> Result<(), Refusal> {
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        for job in loaded.jobs.iter().filter(|job| job.status() == JobStatus::Escalated) {
            let id = format!("job:{}", job.id().as_str());
            if self.store().lock().await.sleep_row(&id).map_err(|why| self.ledger_fault(why))?.is_some() {
                continue;
            }
            let Ok(Some(TransitionReason::Escalation(trigger))) = self.last_reason(job.id()).await else {
                continue;
            };
            let (who, title) = (job.handle(), job.title().as_str());
            let asked = format!("{title} ({})", trigger.as_wire());
            let retried = RETRIED.contains(&trigger);
            if !(ANSWERABLE.contains(&trigger) || retried) || destructive_text(title) {
                self.leave(row("blocked", id, who, format!("Escalated, waiting on you: {title}"))).await?;
                continue;
            }
            if retried {
                let (ran, said) = match trigger {
                    EscalationTrigger::GateUndecided => (self.rerun_gate(job.id()).await, "Ran the gate again"),
                    _ => (
                        self.restart_step_by(job.id(), None, Ending::Unheard, Actor::Fleet).await,
                        "Restarted the step",
                    ),
                };
                let done = match ran {
                    Ok(_) => row("decided", id, who, format!("{said}: {asked}")),
                    Err(why) => {
                        row("blocked", id, who, format!("Escalated, could not carry on ({why}): {title}"))
                    }
                };
                self.leave(done).await?;
                continue;
            }
            let Some(words) = Redirection::saying(&format!("{}\n\n{asked}", mode_line(WaitingMode::Best))) else {
                continue;
            };
            let by = Actor::Fleet;
            let sent = match self.steer(job.id(), Steer::Words(&words, by)).await {
                Err(Adrift::NoDroneToRedirect { .. }) => {
                    self.restart_step_by(job.id(), Some(&words), Ending::Unheard, by).await
                }
                other => other,
            };
            match sent {
                Ok(_) => self.leave(row("decided", id, who, asked)).await?,
                Err(why) => {
                    let text = format!("Escalated, could not carry on ({why}): {title}");
                    self.leave(row("blocked", id, who, text)).await?;
                }
            }
        }
        Ok(())
    }

    /// Fill `chose` on decisions made with `mode: best`: the first line of what the agent said next.
    async fn sleep_chose(self: &Arc<Self>) -> Result<(), Refusal> {
        let empty: Vec<SleepRow> = self
            .store()
            .lock()
            .await
            .sleep_rows()
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|one| one.kind == "decided" && one.chose.as_deref().map_or(true, str::is_empty))
            .collect();
        let mut filled = false;
        for one in empty {
            let Some(session) = one.session_id.clone() else { continue };
            let Ok(read) = Arc::clone(self).get_session(ipc::SessionId::carried(session)).await else {
                continue;
            };
            let Some(chose) = next_words(&read.rows) else { continue };
            let done = self
                .store()
                .lock()
                .await
                .fill_sleep_chose(&one.id, &chose)
                .map_err(|why| self.ledger_fault(why))?;
            filled |= done;
        }
        if filled {
            self.sleep_published().await?;
        }
        Ok(())
    }

    async fn sleeping(&self) -> bool {
        self.store().lock().await.sleep_switch().map(|one| one.on).unwrap_or(false)
    }

    /// One pass over every live session's waiting items. Nothing happens while sleep is off.
    pub async fn sleep_pass(self: Arc<Self>) -> Result<(), Refusal> {
        if !self.sleeping().await {
            return Ok(());
        }
        let sessions = {
            let store = self.store().lock().await;
            let found = store
                .find_sessions(&store::SessionSearch {
                    manifest_id: None,
                    state: Some(SessionState::Live),
                    text: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            let mut out = Vec::with_capacity(found.len());
            for session in &found {
                let who = session.title.clone().unwrap_or_else(|| session.id.clone());
                out.push((session.id.clone(), who, self.ledger_row(&store, session)?.waiting_for));
            }
            out
        };
        let now = self.now().epoch_millis();
        for (session, who, items) in sessions {
            for item in items {
                let id = format!("{session}:{}", item.id);
                if self.store().lock().await.sleep_row(&id).map_err(|why| self.ledger_fault(why))?.is_some() {
                    continue;
                }
                match item.source {
                    WaitingSource::Walk => {
                        self.leave(row("walk", id, who.clone(), item.text.clone())).await?;
                    }
                    WaitingSource::Permission => {
                        self.leave(row("blocked", id, who.clone(), item.text.clone())).await?;
                    }
                    WaitingSource::AskCard | WaitingSource::Agent if destructive(&item) => {
                        self.leave(row("blocked", id, who.clone(), item.text.clone())).await?;
                    }
                    WaitingSource::AskCard | WaitingSource::Agent => {
                        let since = ipc_millis(&item);
                        let waited = now.zip(since).map(|(now, since)| now.saturating_sub(since));
                        if waited.map_or(true, |ms| ms < GRACE.as_millis() as i64) {
                            continue;
                        }
                        let choice = recommended(&item);
                        let said = AnswerWaiting {
                            session_id: ipc::SessionId::carried(session.clone()),
                            item_id: item.id.clone(),
                            choice: choice.map(|at| at as u32),
                            text: None,
                            mode: choice.is_none().then_some(WaitingMode::Best),
                        };
                        if Arc::clone(&self).settle_waiting(said).await.is_err() {
                            continue;
                        }
                        let chose = choice.and_then(|at| item.options.get(at)).map(|one| one.label.clone());
                        self.leave(SleepRow {
                            session_id: Some(session.clone()),
                            chose,
                            ..row("decided", id, who.clone(), item.text.clone())
                        })
                        .await?;
                    }
                }
            }
        }
        self.sleep_jobs().await?;
        self.sleep_chose().await
    }
}

/// The first non-empty line of the agent's message after the last "decide this yourself" the night sent.
pub(crate) fn next_words(rows: &[ipc::SessionRow]) -> Option<String> {
    let line = mode_line(WaitingMode::Best);
    let from = rows.iter().rposition(|row| {
        matches!(row, ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, .. } if text.contains(line))
    })?;
    rows[from + 1..].iter().find_map(|row| match row {
        ipc::SessionRow::Message { from: ipc::SessionVoice::Agent, text, .. } => {
            let first = text.lines().map(str::trim).find(|one| !one.is_empty())?;
            Some(match first.char_indices().nth(CHOSE_CHARS) {
                Some((at, _)) => format!("{}…", first[..at].trim_end()),
                None => first.to_string(),
            })
        }
        _ => None,
    })
}

fn ipc_millis(item: &WaitingItem) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(item.since.as_str()).epoch_millis()
}
