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
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Ask, Delivery, Model, Vcs, WorkProduct};
use config::settings as keys;
use ipc::{SessionAccount, SessionLive, SessionPlan};

use super::activity::Publish;
use crate::daemon::Fleet;
use crate::judging::watched;
use crate::session_account::{read, TurnSoFar, CALLS_AT_MOST, SAID_MOST};

/// The most often the model is asked to write an account, a Session at a time.
pub(crate) const WRITE_EVERY: Duration = Duration::from_secs(5);
/// The longest a first change waits for the rest of its burst, where the model has not been asked lately.
const SETTLE: Duration = Duration::from_secs(1);

/// What a model call is made from, and which message it answers.
pub(crate) struct Reading {
    pub for_message: String,
    /// How many times the account was cleared when it began, so an answer that crossed a clear is dropped.
    pub epoch: u32,
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
    /// A task is waiting to ask the model. At most one, and none while a call is out.
    scheduled: bool,
    /// The model is being asked.
    asking: bool,
    /// When the model was last asked.
    asked_at: Option<Instant>,
    /// Counts the times a message of his cleared the account.
    epoch: u32,
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

    /// A page was shown or served. Whether the live view changed.
    pub(crate) fn live_seen(&mut self, url: &str, title: &str) -> bool {
        let live = SessionLive { url: url.to_string(), title: title.to_string() };
        let changed = self.live.as_ref() != Some(&live);
        self.live = Some(live);
        changed
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
        self.epoch += 1;
        self.account = None;
        self.live = None;
    }

    /// Something was read that no account has. **How long to wait before asking the model, or
    /// `None`** where a task is already waiting or a call is out: a burst of changes is one call,
    /// made once the wait is up and reading all of it, and the model is asked at most once in `every`.
    pub(crate) fn wanted(&mut self, every: Duration) -> Option<Duration> {
        if !self.dirty || self.his.is_none() || self.scheduled || self.asking {
            return None;
        }
        self.scheduled = true;
        Some(match self.asked_at.map(|at| at.elapsed()) {
            Some(since) if since < every => every - since,
            _ => every.min(SETTLE),
        })
    }

    /// The wait is over and the model is about to be asked. `None` where there is nothing to ask.
    pub(crate) fn begin(&mut self, groups: Vec<(String, String)>) -> Option<Reading> {
        self.scheduled = false;
        if !std::mem::take(&mut self.dirty) {
            return None;
        }
        let (row, words) = self.his.clone()?;
        self.asking = true;
        self.asked_at = Some(Instant::now());
        Some(Reading {
            for_message: row,
            epoch: self.epoch,
            turn: TurnSoFar { message: words, calls: self.calls.clone(), said: self.said.clone(), groups },
        })
    }

    /// The call is over, with `account` where it wrote one. An account for a message he has since
    /// replaced, or that crossed a clear, is dropped. Whether the account changed, and how long to
    /// wait before asking again where more was read meanwhile.
    pub(crate) fn finished(
        &mut self,
        epoch: u32,
        account: Option<SessionAccount>,
        every: Duration,
    ) -> (bool, Option<Duration>) {
        self.asking = false;
        let mut changed = false;
        if let Some(account) = account {
            let current = self.his.as_ref().map(|(row, _)| row) == Some(&account.for_message) && self.epoch == epoch;
            if current && self.account.as_ref() != Some(&account) {
                self.account = Some(account);
                changed = true;
            }
        }
        (changed, self.wanted(every))
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
    /// What the model reads changed: an account is written in a task of its own, once the wait is up.
    pub(super) fn account_wanted(self: &Arc<Self>, id: &str) {
        let every = self.hosts().account_every();
        let wait = self.hosts().of(id).state().activity.writing.wanted(every);
        if let Some(wait) = wait {
            self.account_after(id, wait);
        }
    }

    fn account_after(self: &Arc<Self>, id: &str, wait: Duration) {
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
        let (for_message, epoch) = (reading.for_message.clone(), reading.epoch);
        let account = match self.account_asked(&reading.turn).await {
            Ok(written) => Some(SessionAccount {
                for_message,
                done: written.done,
                now: written.now,
                next: written.next,
                group: offered.working.or(written.group),
                at: self.instant(),
            }),
            Err(why) => {
                self.session_warned(id, "its account was not written", why);
                None
            }
        };
        let every = self.hosts().account_every();
        let (step, again) = {
            let runtime = self.hosts().of(id);
            let mut state = runtime.state();
            let (changed, again) = state.activity.writing.finished(epoch, account, every);
            let step = match changed {
                true => state.activity.changed(self.hosts().activity_every()),
                false => Publish::Not,
            };
            (step, again)
        };
        self.publish_step(id.to_string(), step);
        if let Some(wait) = again {
            self.account_after(id, wait);
        }
    }

    /// A page the Session showed or served is its live view. What the row's publish needs, which a
    /// caller that publishes the row itself may leave.
    pub(crate) fn live_page_seen(&self, id: &str, url: &str, title: &str) -> Publish {
        let runtime = self.hosts().of(id);
        let mut state = runtime.state();
        match state.activity.writing.live_seen(url, title) {
            true => state.activity.changed(self.hosts().activity_every()),
            false => Publish::Not,
        }
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
