//! The account of a Session at work, written by a small model. `docs/concepts/session/working-on.md`.
//!
//! **What the model reads is what Fleet detected**: the person's last message, the calls of the turn
//! as `now` words and the agent's own texts. It writes `done`, `now` and `next` for that message in
//! the person's words, and the line is kept on [`Activity`](super::activity::Activity) beside `now`.
//! Memory only, as the rest of the row is.
//!
//! **Kept until his next message.** A turn ending clears nothing, and a turn Fleet's own notice
//! begins clears nothing: only a message from him that starts a turn does, with the calls and texts
//! the account was written from.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{SessionAccount, SessionLive, SessionPlan};

use super::activity::Publish;
use crate::daemon::Fleet;
use crate::judging::watched;
use crate::session_account::{read, TurnSoFar, CALLS_AT_MOST, SAID_MOST};

/// What a model call is made from, and which message it answers.
pub(crate) struct Reading {
    pub for_message: String,
    pub turn: TurnSoFar,
}

/// What an account is written from, and the account.
#[derive(Default)]
pub(crate) struct Writing {
    /// The last message the person sent: its row id and his words.
    his: Option<(String, String)>,
    /// That message has not started a turn yet, so the turn it starts is a new one.
    his_unstarted: bool,
    calls: Vec<String>,
    said: Vec<String>,
    /// Something the model reads is not yet in an account.
    dirty: bool,
    account: Option<SessionAccount>,
    live: Option<SessionLive>,
}

impl Writing {
    /// The person sent a message.
    pub(crate) fn person_said(&mut self, row: &str, words: &str) {
        self.his = Some((row.to_string(), words.to_string()));
        self.his_unstarted = true;
    }

    /// A call of the Session's own, as `now` words.
    pub(crate) fn call_seen(&mut self, now: &str) {
        if self.calls.last().map(String::as_str) != Some(now) {
            self.calls.push(now.to_string());
            let over = self.calls.len().saturating_sub(CALLS_AT_MOST);
            self.calls.drain(..over);
            self.dirty = true;
        }
    }

    /// The agent said something.
    pub(crate) fn said_seen(&mut self, text: &str) {
        self.said.push(text.trim().to_string());
        let over = self.said.len().saturating_sub(SAID_MOST);
        self.said.drain(..over);
        self.dirty = true;
    }

    /// A turn began. **Only a message from him that starts it clears the account**, the live page and
    /// what they were read from.
    pub(crate) fn turn_began(&mut self) {
        if !std::mem::take(&mut self.his_unstarted) {
            return;
        }
        self.calls.clear();
        self.said.clear();
        self.dirty = false;
        self.account = None;
        self.live = None;
    }

    /// Something was read that no account has. How long to wait before writing one, or `None`.
    pub(crate) fn wanted(&mut self) -> Option<std::time::Duration> {
        (self.dirty && self.his.is_some()).then_some(std::time::Duration::ZERO)
    }

    /// The model is about to be asked.
    pub(crate) fn begin(&mut self, groups: Vec<(String, String)>) -> Option<Reading> {
        let (row, words) = self.his.clone()?;
        Some(Reading {
            for_message: row,
            turn: TurnSoFar { message: words, calls: self.calls.clone(), said: self.said.clone(), groups },
        })
    }

    /// The model answered. Whether the account changed: an answer for a message he has since
    /// replaced, or for a Session that ended, is dropped.
    pub(crate) fn finished(&mut self, account: SessionAccount) -> bool {
        if self.his.as_ref().map(|(row, _)| row) != Some(&account.for_message) {
            return false;
        }
        let changed = self.account.as_ref() != Some(&account);
        self.account = Some(account);
        changed
    }

    pub(crate) fn account(&self) -> Option<SessionAccount> {
        self.account.clone()
    }

    pub(crate) fn live(&self) -> Option<SessionLive> {
        self.live.clone()
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
    /// What the model reads changed: an account is written in a task of its own.
    pub(super) fn account_wanted(self: &Arc<Self>, id: &str) {
        let Some(wait) = self.hosts().of(id).state().activity.writing.wanted() else { return };
        let (fleet, id) = (Arc::clone(self), id.to_string());
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            fleet.account_written(&id).await;
        });
    }

    async fn account_written(self: &Arc<Self>, id: &str) {
        let plan = self.plan_of_session(id).await;
        let offered = group_choice(plan.as_ref());
        let Some(reading) = self.hosts().of(id).state().activity.writing.begin(offered.choices.clone()) else { return };
        let asked = self.account_asked(&reading.turn).await;
        let written = match asked {
            Ok(written) => written,
            Err(why) => return eprintln!("job {id}: its account was not written: {why}"),
        };
        let account = SessionAccount {
            for_message: reading.for_message,
            done: written.done,
            now: written.now,
            next: written.next,
            group: offered.working.or(written.group),
            at: self.instant(),
        };
        let step = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            match state.activity.writing.finished(account) {
                true => state.activity.changed(self.hosts().activity_every()),
                false => Publish::Not,
            }
        };
        self.publish_step(id.to_string(), step);
    }

    /// One call, one account. Whatever goes wrong is words for the log.
    async fn account_asked(&self, turn: &TurnSoFar) -> Result<crate::session_account::Written, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let named = self.machine_settings().now().get(keys::MODELS_ACCOUNT);
        let model = Model::named_at(&named, None).map_err(|_| String::from("no model is configured for `models.account`"))?;
        let ask = Ask::put(model, &turn.question(), proposing.environment.clone())
            .map_err(|_| String::from("there was nothing to ask"))?;
        let reply = watched(proposing.client.as_ref(), &ask, proposing.budget, &|_| {}, std::future::pending::<()>())
            .await
            .map_err(|why| why.to_string())?;
        read(&reply, &turn.groups).ok_or_else(|| String::from("the answer had no line to keep"))
    }

    /// The Session's plan as the row carries it.
    async fn plan_of_session(&self, id: &str) -> Option<SessionPlan> {
        let kept = self.store().lock().await.session_plan(id).ok().flatten()?;
        Some(crate::plan_record::wire(&kept))
    }
}

/// Which plan group an account names.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GroupChoice {
    /// The group holding a task at work, which is the answer where there is one.
    pub working: Option<String>,
    /// The approved plan's groups, for the model to pick among where no task is at work.
    pub choices: Vec<(String, String)>,
}

/// The group the Session is on. **None for a plan the person has not approved**: it is a proposal and
/// not the work.
pub(crate) fn group_choice(plan: Option<&SessionPlan>) -> GroupChoice {
    let Some(plan) = plan.filter(|plan| plan.approved) else { return GroupChoice::default() };
    let working = plan
        .groups
        .iter()
        .find(|group| group.tasks.iter().any(|task| task.state == ipc::PlanTaskState::Working))
        .map(|group| group.id.clone());
    let choices = match working {
        Some(_) => Vec::new(),
        None => plan.groups.iter().map(|group| (group.id.clone(), group.title.clone())).collect(),
    };
    GroupChoice { working, choices }
}
