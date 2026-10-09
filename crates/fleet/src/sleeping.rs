//! Sleep mode: the owner turns it on before bed, and sessions and Jobs keep working until only he can
//! unblock them. `docs/concepts/session.md`.
//!
//! **What the night may decide.** A question a session asked, once it has waited out [`GRACE`] so the
//! agent could carry on by itself first: the option labelled "(Recommended)", else `mode: best`. **Never
//! a destructive or irreversible one, and never a permission.** Those are held under blocked, as is an
//! escalated Job: no route answers an escalation in words, so there is nothing to decide with.
//!
//! **Each thing is left once.** A row's id is `<session>:<item>`, so a pass over an item already
//! decided or held changes nothing.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    AnswerWaiting, OverrideSleep, SendSessionMessage, SleepBlocked, SleepDecided, SleepLanded, SleepState,
    SleepWalk, WaitingItem, WaitingMode, WaitingSource, WireError,
};
use store::{SessionState, SleepRow};

use crate::daemon::Fleet;

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

/// Whether the item, read with its options, may be destructive or irreversible.
pub(crate) fn destructive(item: &WaitingItem) -> bool {
    let mut text = item.text.to_lowercase();
    for option in &item.options {
        text.push('\n');
        text.push_str(&option.label.to_lowercase());
    }
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

    /// An escalated Job: blocked, because nothing answers an escalation in words.
    pub(crate) async fn sleep_escalated(&self, id: &str, who: String, title: &str) {
        if self.sleeping().await {
            let text = format!("Escalated, waiting on you: {title}");
            let _ = self.leave(row("blocked", format!("job:{id}"), who, text)).await;
        }
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
        Ok(())
    }
}

fn ipc_millis(item: &WaitingItem) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(item.since.as_str()).epoch_millis()
}
