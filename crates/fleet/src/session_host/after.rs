//! A Session that waits on another, and starts on its own. `docs/concepts/session.md`, *A Session that
//! waits on another*.
//!
//! **A waiting Session is a blank Session and a wait.** It exists and is listed, holds its brief, takes
//! no slot and runs no process. [`crate::after_following`] reads what Fleet observes of the Session waited
//! on and calls [`Fleet::wait_met`] when the condition holds: the waiting Session then starts on that
//! Session's handoff followed by the brief. **The person may approve a waiting Session and pick what it
//! starts on** (`change_waiting`): approved (`held` false) it starts at once when released, on the workflow,
//! model and effort its hosting holds. **A Session an agent started waits the same way and is
//! released the same way, except that its first message is then held for the person**, as every start
//! an agent makes is.
//!
//! **The Session waited on is told**, in its thread and as a message that wakes it if it is idle: what
//! the new one needs of it, and that it may change the condition and where the new one starts from
//! (`change_waiting`). Each change is a row in both threads.
//!
//! **A hand on for context moves every wait to the new Session** ([`Fleet::waits_followed`]), and the
//! new Session is told as the old one was. It holds what the old one held but no plan, and a plan of
//! its own numbers from `g1` again, so a wait on a plan part becomes a wait on its end, which its agent
//! may point at a part of its own.
//!
//! **A wait that can no longer be met goes to the person**: the Session it waits on ended with no
//! successor, and what it waits for will not come. It leaves its wait held on its start card with the
//! reason, even where he approved it to start alone, since what he approved has not happened.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{
    ChangeWaiting, ReadSessionWait, SessionLink, SessionRecord, SessionRow, SessionVoice, SessionWait, StartAfter, StartBase,
    StartCondition, WaitBy, WaitPull, WaitReading, WaiterEvent, WaiterSide,
};
use store::{AttachmentState, Holder, KeptWait};

use crate::daemon::Fleet;
use crate::judging::watched;
use crate::session_proposer::{unread_wait, PlanPart, WaitBrief};

mod dropping;

const WAIT_UNKNOWN: &str = "fleet.wait_session_unknown";
const WAIT_ON_ITSELF: &str = "fleet.wait_on_itself";
const WAIT_PART_UNKNOWN: &str = "fleet.wait_part_unknown";
const WAIT_OVER: &str = "fleet.wait_over";
const WAIT_NOT_YOURS: &str = "fleet.wait_not_yours";
const WAIT_UNPLACED: &str = "fleet.wait_unplaced";

// ---- The wire and the store, each in its own spelling ----------------------------------------------------

/// The condition as the store keeps it: its kind and the plan part it names.
pub(crate) fn condition_kept(condition: &StartCondition) -> (&'static str, String) {
    match condition {
        StartCondition::Ends => ("ends", String::new()),
        StartCondition::GroupDone { group } => ("group_done", group.clone()),
        StartCondition::TaskDone { task } => ("task_done", task.clone()),
        StartCondition::PrPushed => ("pr_pushed", String::new()),
        StartCondition::PrLands => ("pr_lands", String::new()),
    }
}

pub(crate) fn condition_of(wait: &KeptWait) -> StartCondition {
    match wait.condition.as_str() {
        "group_done" => StartCondition::GroupDone { group: wait.part.clone() },
        "task_done" => StartCondition::TaskDone { task: wait.part.clone() },
        "pr_pushed" => StartCondition::PrPushed,
        "pr_lands" => StartCondition::PrLands,
        _ => StartCondition::Ends,
    }
}

pub(crate) fn base_kept(base: StartBase) -> &'static str {
    match base {
        StartBase::Main => "main",
        StartBase::Branch => "branch",
    }
}

pub(crate) fn base_of(wait: &KeptWait) -> StartBase {
    if wait.base == "branch" {
        StartBase::Branch
    } else {
        StartBase::Main
    }
}

/// The store's word for a wait the Operator set: a hold until another Job pushes. On the wire it is
/// not the person's, so it reads as the agent's.
pub(crate) const BY_OPERATOR: &str = "operator";

fn by_kept(by: WaitBy) -> &'static str {
    match by {
        WaitBy::You => "you",
        WaitBy::Agent => "agent",
    }
}

fn by_of(wait: &KeptWait) -> WaitBy {
    if wait.by == "agent" || wait.by == BY_OPERATOR {
        WaitBy::Agent
    } else {
        WaitBy::You
    }
}

pub(crate) fn pull_of(wait: &KeptWait) -> Option<WaitPull> {
    let number = wait.pull_number.as_deref()?.parse().ok()?;
    Some(WaitPull { number, branch: wait.pull_branch.clone().unwrap_or_default() })
}

/// The wait as the record carries it.
pub(crate) fn wait_wire(wait: &KeptWait, on_title: Option<String>, part_title: Option<String>) -> SessionWait {
    SessionWait {
        on: SessionLink { id: ipc::SessionId::carried(&wait.on_session), title: on_title.unwrap_or_else(|| wait.on_session.clone()) },
        condition: condition_of(wait),
        base: base_of(wait),
        needs: wait.needs.clone(),
        brief: wait.brief.clone(),
        pull: pull_of(wait),
        part_title,
        by: by_of(wait),
        held: wait.held,
        workflow: None,
        model: None,
        effort: None,
    }
}

/// The title of a group or task of a plan.
pub(crate) fn plan_part_title(plan: &ipc::SessionPlan, id: &str) -> Option<String> {
    plan.groups.iter().find_map(|group| {
        if group.id == id {
            return Some(group.title.clone());
        }
        group.tasks.iter().find(|task| task.id == id).map(|task| task.text.clone())
    })
}

// ---- What the rows and the messages say ------------------------------------------------------------------

fn part_title(parts: &[PlanPart], id: &str) -> String {
    parts.iter().find(|one| one.id == id).map(|one| one.title.clone()).unwrap_or_else(|| id.to_string())
}

/// The condition as a row says it: what it is, then the part or the pull request it names.
fn row_label(wait: &KeptWait, parts: &[PlanPart]) -> String {
    let pull = pull_of(wait).map(|one| format!(", #{}", one.number)).unwrap_or_default();
    match wait.condition.as_str() {
        "group_done" | "task_done" => format!("A plan part is done, {}", part_title(parts, &wait.part)),
        "pr_pushed" => format!("Its pull request is pushed{pull}"),
        "pr_lands" => format!("Its pull request lands{pull}"),
        _ => String::from("It ends"),
    }
}

fn base_label(wait: &KeptWait) -> &'static str {
    if base_of(wait) == StartBase::Branch {
        "from its PR branch"
    } else {
        "from main"
    }
}

/// What a start condition waits for, as a sentence's clause: `Cockpit Map ends`.
fn when_said(wait: &KeptWait, on: &str, parts: &[PlanPart]) -> String {
    match wait.condition.as_str() {
        "group_done" | "task_done" => format!("{} is done in {on}", part_title(parts, &wait.part)),
        "pr_pushed" => match pull_of(wait) {
            Some(pull) => format!("pull request #{} is pushed", pull.number),
            None => format!("{on} pushes a pull request"),
        },
        "pr_lands" => match pull_of(wait) {
            Some(pull) => format!("pull request #{} lands", pull.number),
            None => format!("a pull request of {on} lands"),
        },
        _ => format!("{on} ends"),
    }
}

/// The two ends of a change as the rows say them: the condition, and where it starts from where that moved too.
fn change_said(was: &KeptWait, now: &KeptWait, parts: &[PlanPart]) -> (String, String) {
    let (from, to) = (row_label(was, parts), row_label(now, parts));
    if base_of(was) == base_of(now) {
        return (from, to);
    }
    if from == to {
        return (format!("{from}, {}", base_label(was)), format!("{to}, {}", base_label(now)));
    }
    (format!("{from}, {}", base_label(was)), format!("{to}, {}", base_label(now)))
}

/// A group is done when it holds tasks and every one is; a task when it is. A task handed in without an edit reads done.
pub(crate) fn plan_part_done(plan: &ipc::SessionPlan, wait: &KeptWait) -> bool {
    let done = |task: &ipc::SessionPlanTask| matches!(task.state, ipc::PlanTaskState::Done | ipc::PlanTaskState::HandedIn);
    match wait.condition.as_str() {
        "group_done" => plan
            .groups
            .iter()
            .find(|group| group.id == wait.part)
            .is_some_and(|group| !group.tasks.is_empty() && group.tasks.iter().all(done)),
        "task_done" => plan
            .groups
            .iter()
            .flat_map(|group| group.tasks.iter())
            .any(|task| task.id == wait.part && done(task)),
        _ => false,
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
    /// The groups and tasks of a Session's plan, each group followed by its tasks.
    async fn plan_parts(&self, session: &str) -> Vec<PlanPart> {
        let plan = self.store().lock().await.session_plan(session).ok().flatten();
        let Some(plan) = plan else { return Vec::new() };
        crate::plan_record::wire(&plan)
            .groups
            .into_iter()
            .flat_map(|group| {
                let tasks: Vec<PlanPart> =
                    group.tasks.into_iter().map(|task| PlanPart { id: task.id, title: task.text, group: false }).collect();
                std::iter::once(PlanPart { id: group.id, title: group.title, group: true }).chain(tasks)
            })
            .collect()
    }

    /// The title a Session goes by, else its id.
    async fn session_title(&self, session: &str) -> String {
        let found = self.store().lock().await.session(session).ok().flatten();
        found.and_then(|one| one.title).unwrap_or_else(|| session.to_string())
    }

    /// The Session `after` names, checked: hosted, live, not the waiting one, and holding the plan part it names.
    pub(super) async fn after_checked(&self, waiting: &str, after: &StartAfter) -> Result<(), Refusal> {
        let session = self.after_known(waiting, after).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(after.session.as_str()));
        }
        Ok(())
    }

    /// [`Self::after_checked`], for a wait that stands: **the Session it waits on may have ended**, and a
    /// person changing it is how it gets out of a wait that cannot now be met.
    async fn after_known(&self, waiting: &str, after: &StartAfter) -> Result<store::KeptSession, Refusal> {
        let on = after.session.as_str();
        if on == waiting {
            return Err(self.hosted_refusal(WAIT_ON_ITSELF, "a Job cannot wait on itself"));
        }
        let (session, _) = self.session_and_hosting(on).await.map_err(|_| {
            self.hosted_refusal(WAIT_UNKNOWN, &format!("no Job is named {on}, and only a Job Fleet hosts can be waited on"))
        })?;
        let (kind, part) = condition_kept(&after.condition);
        if !part.is_empty() {
            let parts = self.plan_parts(on).await;
            let wanted = kind == "group_done";
            if !parts.iter().any(|one| one.id == part && one.group == wanted) {
                let held: Vec<String> = parts.iter().filter(|one| one.group == wanted).map(|one| one.id.clone()).collect();
                return Err(self.hosted_refusal(
                    WAIT_PART_UNKNOWN,
                    &format!("the plan of {on} holds no {} `{part}`; it holds: {}", if wanted { "group" } else { "task" }, held.join(", ")),
                ));
            }
        }
        Ok(session)
    }

    /// Keep the wait of a Session just made, tell the Session it waits on, and publish both.
    /// **The waiting Session's record carries the wait from here on**, which is what [`crate::after_following`] reads.
    pub(crate) async fn wait_begun(
        &self,
        waiting: &str,
        after: StartAfter,
        brief: String,
        held: bool,
        by: WaitBy,
        tell: bool,
    ) -> Result<SessionRecord, Refusal> {
        self.wait_kept(waiting, after, brief, held, by_kept(by), tell).await
    }

    /// [`Self::wait_begun`] for the Operator's hold of a Job on its start card, which stays held for
    /// the person once the wait ends. Kept as the Operator's, so a hold whose condition can no longer
    /// be met is known to be its to lift.
    pub(crate) async fn held_by_operator(&self, waiting: &str, after: StartAfter, brief: String) -> Result<SessionRecord, Refusal> {
        self.wait_kept(waiting, after, brief, true, BY_OPERATOR, true).await
    }

    async fn wait_kept(
        &self,
        waiting: &str,
        after: StartAfter,
        brief: String,
        held: bool,
        by: &str,
        tell: bool,
    ) -> Result<SessionRecord, Refusal> {
        self.after_checked(waiting, &after).await?;
        let (kind, part) = condition_kept(&after.condition);
        let on = after.session.as_str().to_string();
        let wait = KeptWait {
            session_id: waiting.to_string(),
            on_session: on.clone(),
            condition: kind.to_string(),
            part,
            base: base_kept(after.base).to_string(),
            needs: after.needs.trim().to_string(),
            brief,
            held,
            // The Session waited on is told by [`crate::after_following`], which sees the wait once it is published.
            told: !tell,
            by: by.to_string(),
            pull_number: None,
            pull_branch: None,
            created_at: self.now().as_str().to_string(),
            started_at: None,
        };
        self.store().lock().await.keep_wait(&wait).map_err(|why| self.ledger_fault(why))?;
        self.published_hosted(waiting).await
    }

    /// A hand on: every open wait on `old` now waits on `new`, which is told of each. Linked once `old`
    /// has ended, as its parent's link is: a hand undone before then closes `new`.
    pub(crate) async fn waits_followed(self: &Arc<Self>, old: &str, new: &str) {
        let parts = self.plan_parts(old).await;
        let mut followed: Vec<(KeptWait, KeptWait)> = Vec::new();
        {
            let mut store = self.store().lock().await;
            for was in store.waits_on(old).unwrap_or_default() {
                // A Session closed while it waited never starts.
                if !store.session(&was.session_id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Live) {
                    continue;
                }
                let mut now = was.clone();
                now.on_session = new.to_string();
                now.told = false;
                if !now.part.is_empty() {
                    now.condition = String::from("ends");
                    now.part = String::new();
                }
                if store.keep_wait(&now).is_ok() {
                    followed.push((was, now));
                }
            }
        }
        for (was, now) in followed {
            let mut also = String::new();
            if !was.part.is_empty() {
                also = format!(" It waited on {} in the plan of the Job this one took over from.", part_title(&parts, &was.part));
                let said = change_said(&was, &now, &parts);
                let (waiting_title, new_title) = (self.waiting_title(&now).await, self.session_title(new).await);
                self.row_put(new, self.changed_row(new, &now.session_id, &waiting_title, WaiterSide::Waited, &said, None)).await;
                self.row_put(&now.session_id, self.changed_row(&now.session_id, new, &new_title, WaiterSide::Waiting, &said, None)).await;
            }
            self.waiter_told(&now.session_id, &also).await;
            // The waiting Session's record carries the wait, so the watcher reads it against the new Session.
            let _ = self.published_hosted(&now.session_id).await;
        }
        let _ = self.published_hosted(new).await;
    }

    /// At Fleet's start: a wait still on a Session that was handed on follows to the last of the chain.
    /// A Fleet that stopped partway through a hand left it there, as did one from before waits followed.
    pub(crate) async fn waits_left_followed(self: &Arc<Self>) {
        let moves: Vec<(String, String)> = {
            let store = self.store().lock().await;
            let open = store.open_waits().unwrap_or_default();
            let on: std::collections::BTreeSet<String> = open.into_iter().map(|one| one.on_session).collect();
            on.into_iter().filter_map(|on| handed_to(&store, &on).map(|last| (on, last))).collect()
        };
        for (old, new) in moves {
            self.waits_followed(&old, &new).await;
        }
    }

    /// A `changed` row for `thread`, naming `session`. `by` is `None` for a change Fleet made.
    fn changed_row(&self, thread: &str, session: &str, title: &str, side: WaiterSide, said: &(String, String), by: Option<WaitBy>) -> SessionRow {
        SessionRow::Waiter {
            id: self.row_id(thread),
            at: self.instant(),
            session: SessionLink { id: ipc::SessionId::carried(session), title: title.to_string() },
            event: WaiterEvent::Changed,
            side,
            says: None,
            from: Some(said.0.clone()),
            to: Some(said.1.clone()),
            by,
        }
    }

    /// The Session waited on is told what the new one needs, in its thread and as a message. **Once**: the
    /// first call to claim it tells, and a Fleet that restarts finds it told.
    pub(crate) async fn waiter_told_of_wait(self: &Arc<Self>, waiting: &str) {
        self.waiter_told(waiting, "").await;
    }

    /// [`Self::waiter_told_of_wait`], with `also` said after when and where from.
    async fn waiter_told(self: &Arc<Self>, waiting: &str, also: &str) {
        let kept = self.store().lock().await.wait_of(waiting).ok().flatten();
        let Some(wait) = kept.filter(|one| one.started_at.is_none() && !one.told) else { return };
        // A word would start a Job that itself still waits, ahead of what it waits for: it is told once it has started.
        let on_waits = self.store().lock().await.wait_of(&wait.on_session).ok().flatten().is_some_and(|one| one.started_at.is_none());
        if on_waits {
            return;
        }
        if !matches!(self.store().lock().await.claim_wait_told(waiting), Ok(true)) {
            return;
        }
        let wait = &wait;
        let parts = self.plan_parts(&wait.on_session).await;
        let waiting_title = self.waiting_title(wait).await;
        let mut text = format!(
            "Job \"{waiting_title}\" ({}) now waits on this Job. It starts when {}, from {}.",
            wait.session_id,
            when_said(wait, "this Job", &parts),
            if base_of(wait) == StartBase::Branch { "the branch of your pull request" } else { "main" },
        );
        text.push_str(also);
        if !wait.needs.is_empty() {
            text.push_str(&format!(" It needs: {}.", wait.needs));
        }
        text.push_str(" You can change when it starts, or where from, with change_waiting.");
        let row = SessionRow::Waiter {
            id: self.row_id(&wait.on_session),
            at: self.instant(),
            session: SessionLink { id: ipc::SessionId::carried(&wait.session_id), title: waiting_title },
            event: WaiterEvent::Waits,
            side: WaiterSide::Waited,
            says: Some(if wait.needs.is_empty() { first_line(&wait.brief) } else { wait.needs.clone() }),
            from: None,
            to: None,
            by: None,
        };
        self.told_or_kept(&wait.on_session, &text, row).await;
        let _ = self.published_hosted(&wait.on_session).await;
    }

    /// News with `row` in its place, or the row alone where there is nobody to tell: **a Job that ended
    /// takes no word**, and its thread still says what became of what waited on it.
    async fn told_or_kept(self: &Arc<Self>, session: &str, text: &str, row: SessionRow) {
        if !matches!(self.told_news_by_fleet_as(session, text, row.clone()).await, Ok(true)) {
            self.row_put(session, row).await;
        }
    }

    /// The wait as the wire carries it, with the titles it names read now.
    async fn wait_wired(&self, wait: &KeptWait) -> SessionWait {
        let on_title = self.session_title(&wait.on_session).await;
        let part = self.plan_parts(&wait.on_session).await.into_iter().find(|one| one.id == wait.part).map(|one| one.title);
        let mut wire = wait_wire(wait, Some(on_title), part);
        let hosting = self.store().lock().await.hosting(&wait.session_id).ok().flatten();
        if let Some(hosting) = hosting {
            (wire.workflow, wire.model, wire.effort) = (hosting.workflow, hosting.model, hosting.effort);
        }
        wire
    }

    /// The person's approval and picks on a waiting Job: the model and effort it starts on are its hosting's,
    /// as `tune_session` keeps them, and a met wait starts on them. A pick of the person's is no longer the
    /// proposer's. `""` clears one.
    async fn start_picks_kept(self: &Arc<Self>, waiting: &str, change: &ChangeWaiting) -> Result<(), Refusal> {
        let clean = |one: &Option<String>| one.as_ref().map(|one| one.trim().to_string()).map(|one| Some(one).filter(|one| !one.is_empty()));
        if let Some(workflow) = clean(&change.workflow) {
            self.workflow_set(ipc::SetSessionWorkflow { session_id: ipc::SessionId::carried(waiting), workflow }).await?;
        }
        let (model, effort) = (clean(&change.model), clean(&change.effort));
        if model.is_none() && effort.is_none() {
            return Ok(());
        }
        let mut store = self.store().lock().await;
        let Some(mut hosting) = store.hosting(waiting).map_err(|why| self.ledger_fault(why))? else { return Ok(()) };
        let (model_set, effort_set) = (model.is_some(), effort.is_some());
        if let Some(model) = model {
            hosting.model = model;
        }
        if let Some(effort) = effort {
            hosting.effort = effort;
        }
        store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
        if model_set {
            self.person_changed(&mut store, waiting, ipc::ProposedField::Model);
        }
        if effort_set {
            self.person_changed(&mut store, waiting, ipc::ProposedField::Effort);
        }
        Ok(())
    }

    /// What a waiting Session is called: its title, else the first line of its brief.
    async fn waiting_title(&self, wait: &KeptWait) -> String {
        let found = self.store().lock().await.session(&wait.session_id).ok().flatten();
        found.and_then(|one| one.title).filter(|one| !one.is_empty()).unwrap_or_else(|| first_line(&wait.brief))
    }

    /// The condition is met: the waiting Session starts on the handoff followed by its brief. **Once**:
    /// the first call to claim the wait starts it, and any other finds it started.
    pub(crate) async fn wait_met(self: &Arc<Self>, wait: KeptWait, met: String) {
        let at = self.now().as_str().to_string();
        let claimed = self.store().lock().await.claim_wait_start(&wait.session_id, &at);
        if !matches!(claimed, Ok(true)) {
            return;
        }
        let waiting = wait.session_id.clone();
        let on = wait.on_session.clone();
        let on_title = self.session_title(&on).await;
        let from = self.starts_from(&wait).await;
        let handoff = self.handoff_of(&on).await.unwrap_or_else(|_| format!("Handoff from {on_title} ({on})"));
        let message = format!("{handoff}\n\nWhy you start now: {met}.\nYou start from {from}.\n\nYour brief:\n{}", wait.brief);
        if wait.held {
            if let Err(why) = self.held_with(&waiting, &message).await {
                self.session_warned(&waiting, "the start it waited for was not held for the person", format!("{why:?}"));
            }
        } else if let Err(why) = self
            .waited_brief_from(&waiting, SessionVoice::Session { id: on.clone(), title: on_title }, &message)
            .await
        {
            eprintln!("job {waiting}: the start it waited for was not sent: {why:?}");
        }
        let title = self.waiting_title(&wait).await;
        self.row_put(
            &on,
            SessionRow::Waiter {
                id: self.row_id(&on),
                at: self.instant(),
                session: SessionLink { id: ipc::SessionId::carried(&waiting), title },
                event: WaiterEvent::Started,
                side: WaiterSide::Waited,
                says: Some(format!("From {from}")),
                from: None,
                to: None,
                by: None,
            },
        )
        .await;
        let _ = self.published_hosted(&waiting).await;
        let _ = self.published_hosted(&on).await;
    }

    /// Where the waiting Session starts from, in words: main, or the branch the Session waited on works on.
    async fn starts_from(&self, wait: &KeptWait) -> String {
        if base_of(wait) == StartBase::Main {
            return if wait.condition == "pr_lands" { String::from("main, after the merge") } else { String::from("main") };
        }
        if let Some(pull) = pull_of(wait).filter(|one| !one.branch.is_empty()) {
            return format!("the branch {} (pull request #{})", pull.branch, pull.number);
        }
        let held = self.store().lock().await.attachments_of(&Holder::session(&wait.on_session)).unwrap_or_default();
        let branch = held
            .iter()
            .filter(|one| one.kind == "branch" && one.state == AttachmentState::Standing)
            .max_by(|a, b| a.since.cmp(&b.since))
            .map(|one| one.target.clone());
        match branch {
            Some(branch) => format!("the branch {branch}"),
            None => String::from("main, since the Job you waited on has no branch"),
        }
    }

    /// `read_session_wait`: what the proposer reads of a brief that tags a Session. **Never refused for
    /// what it could not read**: a call that fails or answers badly is the defaults, as the Session
    /// proposer's is, and says why on Fleet's standard error.
    pub(crate) async fn wait_read(self: &Arc<Self>, read: ReadSessionWait) -> Result<WaitReading, Refusal> {
        let on = read.session_id.as_str().to_string();
        let request = read.request.trim().to_string();
        if request.is_empty() {
            return Ok(unread_wait());
        }
        let _ = self.session_and_hosting(&on).await.map_err(|_| {
            self.hosted_refusal(WAIT_UNKNOWN, &format!("no Job is named {on}, and only a Job Fleet hosts can be waited on"))
        })?;
        let parts = self.plan_parts(&on).await;
        let title = self.session_title(&on).await;
        let brief = WaitBrief::about(&request, &title, &parts);
        let proposing = match self.proposing() {
            Ok(proposing) => proposing,
            Err(why) => {
                eprintln!("a wait was not read: {why:?}");
                return Ok(unread_wait());
            }
        };
        let ask = match Ask::put(proposing.model.clone(), brief.question(), proposing.environment.clone()) {
            Ok(ask) => ask,
            Err(_) => return Ok(unread_wait()),
        };
        match watched(proposing.client.as_ref(), &ask, proposing.budget, &|_| {}, std::future::pending::<()>()).await {
            Ok(reply) => Ok(brief.read(&reply)),
            Err(why) => {
                eprintln!("job {on}: the wait was not read, so it is the defaults: {why}");
                Ok(unread_wait())
            }
        }
    }

    /// `change_waiting`. **The agent of the Session waited on may change a Session that waits on it and
    /// no other; the person may change any.** Both threads get a row, and the Session waited on is
    /// told in words when the person changed it.
    pub(crate) async fn waiting_changed(
        self: &Arc<Self>,
        caller: Option<Caller>,
        change: ChangeWaiting,
    ) -> Result<SessionWait, Refusal> {
        let waiting = change.waiting_session.as_str().to_string();
        let was = self.open_wait_of(&waiting).await?;
        let by = match caller.as_ref() {
            None => WaitBy::You,
            Some(caller) => {
                let me = self.session_holding(caller).ok_or_else(|| {
                    self.hosted_refusal(WAIT_UNPLACED, "only a Job's own agent changes when a Job waiting on it starts, and this call comes from none")
                })?;
                if me != was.on_session {
                    return Err(self.hosted_refusal(WAIT_NOT_YOURS, &format!("job {waiting} does not wait on you, so its start is not yours to change")));
                }
                WaitBy::Agent
            }
        };
        self.wait_changed_by(waiting, was, by, None, change).await
    }

    /// The wait of a Job that has not started, or the refusal that it does not wait.
    pub(crate) async fn open_wait_of(&self, waiting: &str) -> Result<KeptWait, Refusal> {
        let kept = self.store().lock().await.wait_of(waiting).map_err(|why| self.ledger_fault(why))?;
        kept.filter(|one| one.started_at.is_none()).ok_or_else(|| self.wait_over(waiting))
    }

    fn wait_over(&self, waiting: &str) -> Refusal {
        Refusal::IllegalMove(ipc::WireError::raised(
            WAIT_OVER,
            format!("job {waiting} does not wait: it started already, or never waited"),
            self.run_id(),
        ))
    }

    /// `change` made to the wait `was` of `waiting` by `by`. **`starter` is the Job that started `waiting`
    /// where it is the one changing it**, which is told to the Job waited on in words, as the person's is.
    pub(crate) async fn wait_changed_by(
        self: &Arc<Self>,
        waiting: String,
        was: KeptWait,
        by: WaitBy,
        starter: Option<&str>,
        change: ChangeWaiting,
    ) -> Result<SessionWait, Refusal> {
        let picks = change.held.is_some() || change.workflow.is_some() || change.model.is_some() || change.effort.is_some();
        if picks && by == WaitBy::Agent {
            return Err(self.hosted_refusal(WAIT_NOT_YOURS, &format!("whether job {waiting} starts at once, and what it starts on, are the person's to set")));
        }
        let mut now = was.clone();
        if let Some(held) = change.held {
            now.held = held;
        }
        if let Some(condition) = &change.condition {
            self.after_known(&waiting, &StartAfter { session: ipc::SessionId::carried(&was.on_session), condition: condition.clone(), base: base_of(&was), needs: String::new() }).await?;
            let (kind, part) = condition_kept(condition);
            now.condition = kind.to_string();
            now.part = part;
            if !matches!(kind, "pr_pushed" | "pr_lands") {
                now.pull_number = None;
                now.pull_branch = None;
            }
        }
        if let Some(base) = change.base {
            now.base = base_kept(base).to_string();
        }
        self.start_picks_kept(&waiting, &change).await?;
        if now == was {
            if picks {
                let _ = self.published_hosted(&waiting).await;
            }
            return Ok(self.wait_wired(&was).await);
        }
        if now.held != was.held && now.condition == was.condition && now.part == was.part && now.base == was.base {
            // Approving is not a change of when it starts: it is kept and shown, and the Job waited on hears nothing.
            self.store().lock().await.keep_wait(&now).map_err(|why| self.ledger_fault(why))?;
            let _ = self.published_hosted(&waiting).await;
            return Ok(self.wait_wired(&now).await);
        }
        now.by = by_kept(by).to_string();
        self.store().lock().await.keep_wait(&now).map_err(|why| self.ledger_fault(why))?;

        let parts = self.plan_parts(&now.on_session).await;
        let (from, to) = change_said(&was, &now, &parts);
        let (waiting_title, on_title) = (self.waiting_title(&now).await, self.session_title(&now.on_session).await);
        let said = (from.clone(), to.clone());
        let for_waited_on = self.changed_row(&now.on_session, &waiting, &waiting_title, WaiterSide::Waited, &said, Some(by));
        let starter = starter.filter(|one| *one != now.on_session);
        if by == WaitBy::You {
            let text = format!("The person changed when job \"{waiting_title}\" ({waiting}) starts: {from} -> {to}.");
            self.told_or_kept(&now.on_session, &text, for_waited_on).await;
        } else if let Some(starter) = starter {
            let starter_title = self.session_title(starter).await;
            let text = format!("Job \"{starter_title}\" ({starter}) changed when job \"{waiting_title}\" ({waiting}), which it started, starts: {from} -> {to}.");
            self.told_or_kept(&now.on_session, &text, for_waited_on).await;
        } else {
            self.row_put(&now.on_session, for_waited_on).await;
        }
        self.row_put(&waiting, self.changed_row(&waiting, &now.on_session, &on_title, WaiterSide::Waiting, &said, Some(by))).await;
        let _ = self.published_hosted(&now.on_session).await;
        // The waiting Session's record carries the wait, so the watcher reads the new condition against what it sees.
        let _ = self.published_hosted(&waiting).await;
        Ok(self.wait_wired(&now).await)
    }

    /// `start_waiting_now`: the person starts a waiting Session at once, whatever its condition.
    pub(crate) async fn waiting_started_now(self: &Arc<Self>, session: &str) -> Result<SessionRecord, Refusal> {
        let kept = self.store().lock().await.wait_of(session).map_err(|why| self.ledger_fault(why))?;
        let Some(wait) = kept.filter(|one| one.started_at.is_none()) else {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                WAIT_OVER,
                format!("job {session} does not wait: it started already, or never waited"),
                self.run_id(),
            )));
        };
        self.wait_met(wait, String::from("You started it")).await;
        self.published_hosted(session).await
    }
}

/// The Session that last took the place of `on`, down the whole chain of hands. `None` for one never
/// handed on.
fn handed_to(store: &store::Store, on: &str) -> Option<String> {
    let mut last = None;
    let mut at = on.to_string();
    while let Some(next) = store
        .attachments_of(&Holder::session(&at))
        .ok()?
        .into_iter()
        .find(|one| one.kind == super::replacing::REPLACED_BY && one.state == AttachmentState::Spent)
    {
        at = next.target.clone();
        last = Some(next.target);
    }
    last
}

/// The first non-blank line of some words, cut to a title's length.
pub(crate) fn first_line(words: &str) -> String {
    super::serving::title_of(words).unwrap_or_default()
}
