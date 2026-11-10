//! What calls the person, which of it he has not opened, and what began to wait on him since: `get_calls`,
//! `calls.changed` and `open_call`. `docs/practices/protocol/a-jobs-calls.md`.
//!
//! **A port of Electron Bridge's TypeScript, check for check**: `callsFromWaiting` and `touchedAt` in
//! `cockpit/waiting.ts`, the merge-line calls and the owner filter in `Dashboard.tsx` and `cockpit/owner.ts`,
//! `incomingOf` and `callOwnerOf` in `cockpit/deck/store.ts` (the unopened one for each owner is
//! `StandingCalls::unopened`), and `session-attention.ts` in the main process. A change to one side is a
//! change to the other until Electron reads this too.
//!
//! **Read off the rows Fleet already publishes**: every `session.changed` and `merge_line_view.changed` is
//! heard here, the Jobs kept slim in memory, and `calls.changed` published when the calls or the news moved.
//! Nothing is kept on disk: what he has opened and what has been told are Fleet's memory, and a restart seeds
//! from what stands.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    CallArrival, CallArrivalKind, CallTone, CallsChanged, Event, Instant, JobState, MainView, MergeLineViews, OpenStandingCall, SessionRecord,
    SessionTurn, StandingCall, StandingCallKind, StandingCalls, StopReason, WaitingItem, WaitingSource,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;

/// What holds a turn open on him: an agent mid-turn on one of these is waiting, not at work. `BLOCKING`.
fn holds_the_turn(source: WaitingSource) -> bool {
    use WaitingSource::*;
    matches!(
        source,
        AskCard | Permission | WindowAsk | Plan | SlotChoice | Delivery | FleetCall | BrowserAsk | Draft | Sketch | Document | TaskDone | RuleAct | OperatorSuggestion
    )
}

/// The sources that hold a turn on him, as a banner tells it. `BLOCKED`.
fn blocked(source: WaitingSource) -> bool {
    use WaitingSource::*;
    matches!(source, AskCard | Permission | WindowAsk | Plan | SlotChoice | Delivery | TaskDone | BrowserAsk)
}

fn tone_of(blocks: bool) -> CallTone {
    if blocks {
        CallTone::Blocked
    } else {
        CallTone::Waiting
    }
}

fn millis(at: &Instant) -> i64 {
    at.to_domain().epoch_millis().unwrap_or(0)
}

/// Why a Job stopped, as `stoppedSaid` says it. **Where Fleet was down it does not say from when to when**:
/// the clock a range is spelt in is the client's, set in its Settings.
fn stopped_said(reason: StopReason) -> &'static str {
    match reason {
        StopReason::TurnEnded => "Stopped: ended its turn with nothing running",
        StopReason::TasksOpen => "Stopped: ended its turn with tasks open",
        StopReason::ProcessGone => "Stopped: process ended",
        StopReason::FleetDown => "Stopped: Fleet was down",
    }
}

/// What a Job's mark says, as `jobStateOf` words it. **Four states have words Fleet does not hold** (a limit's
/// reset in the client's clock, what a Job waits to start after, the task a Job runs, who woke a turn), and
/// say their state plainly.
fn said_of(record: &SessionRecord, state: JobState) -> String {
    let said = said_without_repair(record, state);
    // Any state: the repair under way is one more clause on the line, in Fleet's own words.
    match &record.repairing {
        Some(repairing) => format!("{said} · Repairing: {}", repairing.said),
        None => said,
    }
}

fn said_without_repair(record: &SessionRecord, state: JobState) -> String {
    let stopped = |stopped: &ipc::SessionStopped| {
        // The repairing clause says it, so a running repair adds nothing here and a failed restart is not read beside it.
        if stopped.repair == Some(ipc::StopRepair::Running) && record.repairing.is_some() {
            return stopped_said(stopped.reason).to_string();
        }
        let repair = match stopped.repair {
            Some(ipc::StopRepair::Running) => Some("Repair Job running"),
            Some(ipc::StopRepair::Failed) => Some("Not repaired"),
            Some(ipc::StopRepair::Held) => Some("No repair: Fleet is unhealthy"),
            None => None,
        };
        let restart = match stopped.restart {
            ipc::StopRestart::Queued => Some("Restart queued"),
            ipc::StopRestart::Running => Some("Restarting"),
            ipc::StopRestart::Failed => Some("Restart failed"),
            ipc::StopRestart::None => None,
        };
        match repair.or(restart) {
            Some(more) => format!("{} · {more}", stopped_said(stopped.reason)),
            None => stopped_said(stopped.reason).to_string(),
        }
    };
    let failing = || record.attachments.iter().find(|one| one.kind == crate::pull_requesting::PR && one.state != ipc::AttachmentState::GivenBack && one.state != ipc::AttachmentState::Spent && one.detail.get("state").map(String::as_str) != Some("merged") && one.detail.get("checks").map(String::as_str) == Some("failed"));
    match state {
        JobState::Blank => "Blank: no slot, no branch".into(),
        JobState::Working => "Working".into(),
        JobState::Running => "Running".into(),
        JobState::Subagent => "Subagent working".into(),
        JobState::Stuck => "Stuck".into(),
        JobState::Stopped | JobState::StoppedAsks => record.stopped.as_ref().map(stopped).unwrap_or_default(),
        JobState::Waiting => "Waiting on you".into(),
        JobState::Replied => "Unread reply".into(),
        JobState::Done => "Done".into(),
        JobState::Failing => format!("Checks failed on #{}", failing().map(|one| one.target.as_str()).unwrap_or_default()),
        JobState::Idle => "Idle".into(),
        JobState::Quiet => "Not reachable".into(),
        JobState::Ended => "Ended".into(),
        JobState::Limited => "Held".into(),
        JobState::After => "Starts after another Job".into(),
        JobState::Draft => "Not started".into(),
        JobState::Held => record
            .held_open_by
            .iter()
            .map(|one| match one {
                ipc::HeldOpenBy::Watch { name } => format!("Held open by {name}"),
                ipc::HeldOpenBy::Job { name } => format!("Waiting on {name}"),
                ipc::HeldOpenBy::PullRequest { number, on } => match on {
                    ipc::PullRequestWait::Queue => format!("#{number} in the merge queue"),
                    ipc::PullRequestWait::Checks { auto_merge: true } => format!("#{number} to auto-merge once Checks pass"),
                    ipc::PullRequestWait::Checks { auto_merge: false } => format!("Checks running on #{number}"),
                    ipc::PullRequestWait::AutoMerge => format!("#{number} to auto-merge"),
                },
            })
            .collect::<Vec<_>>()
            .join(" · "),
        JobState::Paused => "Paused".into(),
        JobState::Unstarted => format!("Failed to start: {}", record.hosted.as_ref().and_then(|one| one.start_failed.as_ref()).map(|one| one.cause.as_str()).unwrap_or_default()),
    }
}

/// What a Job's row said when it was last published, as far as a call reads it.
struct Job {
    title: Option<String>,
    live: bool,
    /// Ended, or nothing listens: `session.dead`.
    dead: bool,
    paused: bool,
    working: bool,
    /// Held on its start card, since the Job began.
    start_card: Option<Instant>,
    start_failed: bool,
    last_turn_at: Option<Instant>,
    viewed_at: Option<i64>,
    items: Vec<WaitingItem>,
    state: Option<JobState>,
    stopped: Option<(StopReason, Instant)>,
    /// The Cockpit files the Job under needs-him: its row asks, or a pull request of its own alarms it.
    needs: bool,
    /// What its mark says, as `jobStateOf` words it.
    said: String,
    /// The pull requests it holds, as the ledger's `pr` rows name them: the number and the branch.
    pulls: Vec<(u64, String)>,
    /// When anything but the agent last said something to it: his words, a Poke, another Job's.
    told_at: Option<i64>,
}

impl Job {
    fn of(record: &SessionRecord) -> Job {
        let ended = record.state == ipc::SessionState::Ended;
        let hosted = record.hosted.as_ref().filter(|_| !ended);
        let quiet = record.hosted.is_none() && !record.terminal.as_ref().is_some_and(|one| one.listening);
        Job {
            title: record.title.clone(),
            live: record.state == ipc::SessionState::Live,
            dead: ended || quiet,
            paused: hosted.is_some_and(|one| one.paused.is_some()),
            working: record.hosted.as_ref().is_some_and(|one| matches!(one.turn, SessionTurn::Working { .. })),
            start_card: hosted.filter(|one| one.start.is_some()).map(|_| record.started_at.clone()),
            start_failed: hosted.is_some_and(|one| one.start_failed.is_some()),
            last_turn_at: record.last_turn_at.clone(),
            viewed_at: record.viewed_at.as_ref().map(millis),
            items: if ended { Vec::new() } else { record.waiting_for.clone() },
            state: record.mark.map(|mark| mark.state),
            stopped: record.stopped.as_ref().map(|stopped| (stopped.reason, stopped.at.clone())),
            needs: record.mark.is_some_and(|mark| crate::sessioning::marking::needs_him(record, mark.state)),
            said: record.mark.map(|mark| said_of(record, mark.state)).unwrap_or_default(),
            pulls: record
                .attachments
                .iter()
                .filter(|one| one.kind == crate::pull_requesting::PR && one.state != ipc::AttachmentState::GivenBack)
                .filter_map(|one| Some((one.target.parse().ok()?, one.detail.get("branch").cloned().unwrap_or_default())))
                .collect(),
            told_at: None,
        }
    }

    /// `session.title ?? session.address ?? session.id`: the title unless it is only space.
    fn name(&self, id: &str) -> String {
        self.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).unwrap_or(id).to_string()
    }

    /// He is waited on in the Job itself: a question, a permission, a plan, a start card, or a Job Fleet gave
    /// up restarting. Seeing its call is not enough, so it stays unopened until it does not ask: `needing`.
    fn asking(&self) -> bool {
        !self.paused && (matches!(self.state, Some(JobState::Waiting | JobState::StoppedAsks | JobState::Unstarted)) || self.start_card.is_some())
    }

    /// `waitingOf`'s own items: the start that failed, which has no item on the wire, then the wire's.
    fn waits(&self) -> Vec<WaitingItem> {
        let failed = self.start_failed.then(|| WaitingItem {
            approves: false,
            id: "start_failed".into(),
            text: "Start it again or report it".into(),
            since: self.last_turn_at.clone().unwrap_or_else(never),
            source: WaitingSource::FirstMessage,
            act: None,
            options: Vec::new(),
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
        });
        failed.into_iter().chain(self.items.iter().cloned()).collect()
    }
}

/// The instant Electron reads as none: nothing arrived at it.
fn never() -> Instant {
    Instant::carried("1970-01-01T00:00:00.000Z")
}

/// Whom a failure's call names: the Job that holds the pull request.
struct Owner<'a> {
    id: &'a str,
    job: &'a Job,
}

#[derive(Default)]
pub(crate) struct Keeper {
    jobs: BTreeMap<String, Job>,
    pub(crate) lines: MergeLineViews,
    /// The keys of the main and pull request calls he opened.
    opened: HashSet<String>,
    /// What each live Job waited on, and the Jobs that stood stopped, at the last reading: what is news is
    /// what is not here.
    waiting: HashMap<String, HashSet<String>>,
    stopped: HashSet<String>,
    seeded: bool,
    news: Vec<CallArrival>,
    pub(crate) told: HashMap<String, i64>,
    published: Vec<StandingCall>,
}

impl Keeper {
    /// What stands now is where things stand, and no news.
    pub(crate) fn seed(&mut self, records: &[SessionRecord], lines: MergeLineViews) {
        for record in records {
            self.hear(record);
        }
        self.news.clear();
        self.lines = lines;
        self.seeded = true;
    }

    /// A Job's row, published. **An item entering a live, unpaused Job's list, or the start card, or the Job
    /// turning stopped for good, is news once**: `SessionAttention.read`.
    pub(crate) fn hear(&mut self, record: &SessionRecord) {
        let id = record.id.as_str().to_string();
        let mut job = Job::of(record);
        job.told_at = self.told.get(&id).copied();
        let waits = if job.live { Self::waits_of(record) } else { Vec::new() };
        if self.seeded && job.live && !job.paused {
            let before = self.waiting.get(&id);
            for (item, blocks, text) in &waits {
                if before.is_none_or(|before| !before.contains(item)) {
                    self.news.push(CallArrival {
                        session_id: record.id.clone(),
                        title: record.title.clone().unwrap_or_else(|| "Job".into()),
                        text: text.clone(),
                        tone: tone_of(*blocks),
                        kind: CallArrivalKind::Needs,
                    });
                }
            }
        }
        let stopped = job.live && job.state == Some(JobState::StoppedAsks);
        if self.seeded && stopped && !self.stopped.contains(&id) {
            if let Some((reason, _)) = job.stopped {
                self.news.push(CallArrival {
                    session_id: record.id.clone(),
                    title: record.title.clone().unwrap_or_else(|| "Job".into()),
                    text: stopped_said(reason).into(),
                    tone: CallTone::Waiting,
                    kind: CallArrivalKind::Stopped,
                });
            }
        }
        if stopped {
            self.stopped.insert(id.clone());
        } else {
            self.stopped.remove(&id);
        }
        // What a paused Job held when it was parked stays read, so Resume does not tell it.
        let mut seen: HashSet<String> = waits.into_iter().map(|(item, _, _)| item).collect();
        if job.paused {
            seen.extend(self.waiting.remove(&id).unwrap_or_default());
        }
        self.waiting.insert(id.clone(), seen);
        self.jobs.insert(id, job);
    }

    /// What a live Job waits on as a banner tells it, in `waitsOf`'s order: the start card it is held on, then
    /// the list's own. Each as its id, whether it holds the turn, and its words.
    fn waits_of(record: &SessionRecord) -> Vec<(String, bool, String)> {
        let held = record.hosted.as_ref().is_some_and(|one| one.start.is_some());
        let start = held.then(|| ("start".to_string(), true, "Not started".to_string()));
        start.into_iter().chain(record.waiting_for.iter().map(|one| (one.id.clone(), blocked(one.source), one.text.clone()))).collect()
    }

    /// `pullOwner`: the first Job that holds pull request `number`, or a branch of that name. An ended Job
    /// made it too, so it is found.
    fn pull_owner(&self, number: u64, branch: Option<&str>) -> Option<Owner<'_>> {
        self.jobs
            .iter()
            .find(|(_, job)| job.pulls.iter().any(|(held, at)| *held == number || branch.is_some_and(|branch| at == branch)))
            .map(|(id, job)| Owner { id, job })
    }

    /// `isOnIt`: the Job is working, or something was said to it after the failure arose. A Job that is gone
    /// is not.
    fn on_it(owner: &Owner<'_>, since: Option<i64>) -> bool {
        let job = owner.job;
        if job.dead {
            return false;
        }
        if job.working {
            return true;
        }
        match (since, job.told_at) {
            (Some(since), Some(told)) => told > since,
            _ => false,
        }
    }

    /// The calls standing: every Job's, then the merge line's, oldest first.
    pub(crate) fn standing(&self, on_screen: &dyn Fn(&str) -> bool, now: i64) -> Vec<StandingCall> {
        let mut calls = Vec::new();
        for (id, job) in &self.jobs {
            self.job_calls(id, job, on_screen(id), now, &mut calls);
        }
        self.line_calls(&mut calls);
        calls.sort_by(|a, b| a.since.cmp(&b.since).then_with(|| a.key.cmp(&b.key)));
        calls
    }

    /// `callsFromWaiting` for one Job, and its stopped call.
    fn job_calls(&self, id: &str, job: &Job, here: bool, now: i64, calls: &mut Vec<StandingCall>) {
        if !job.live || job.dead {
            return;
        }
        // The last instant he was in it: now while it is on screen. He sending it a word is a visit Fleet keeps too.
        let touched = if here { now } else { job.viewed_at.unwrap_or(0) };
        let asking = job.asking();
        let opened = |since: &Instant| !asking && (here || job.viewed_at.is_some_and(|at| at >= millis(since)));
        let before = calls.len();
        // A paused Job is parked by him: nothing it waits on calls until he resumes it.
        let items = if job.paused { Vec::new() } else { job.waits() };
        for item in items {
            if job.working && !holds_the_turn(item.source) {
                continue;
            }
            // A page waits on his OK until he answers it, so being in its Job does not take it off his list.
            if item.source != WaitingSource::Document && touched != 0 && millis(&item.since) <= touched {
                continue;
            }
            calls.push(StandingCall {
                key: format!("session:{id}:{}", item.id),
                owner: format!("session:{id}"),
                kind: StandingCallKind::Job,
                session_id: Some(ipc::SessionId::carried(id)),
                title: job.name(id),
                text: item.text.clone(),
                tone: tone_of(blocked(item.source)),
                opened: opened(&item.since),
                since: item.since,
            });
        }
        // A Job that needs him is a call whether or not it raised an item that is still unseen: its row's mark, which
        // a visit does not take off the list while it asks. The key carries the turn, so the next one is a new call.
        // A Job held on its start card asks the same, though it is paused.
        if calls.len() == before && (job.needs || job.start_card.is_some()) {
            let since = job.last_turn_at.clone().unwrap_or_else(never);
            calls.push(StandingCall {
                key: format!("session:{id}:needs:{}", job.last_turn_at.as_ref().map(Instant::as_str).unwrap_or("")),
                owner: format!("session:{id}"),
                kind: StandingCallKind::Job,
                session_id: Some(ipc::SessionId::carried(id)),
                title: job.name(id),
                text: job.said.clone(),
                tone: CallTone::Waiting,
                opened: opened(&since),
                since,
            });
        }
    }

    /// Main when it is red and nothing newer runs, and a pull request of his that failed or cannot merge,
    /// each unless the Job that owns it is already on it.
    fn line_calls(&self, calls: &mut Vec<StandingCall>) {
        for view in &self.lines.lines {
            if let Some(MainView::Red { failed, checking, since, merge, .. }) = &view.main {
                if checking.is_empty() {
                    let owner = merge.as_ref().and_then(|merge| self.pull_owner(merge.number, merge.branch.as_deref()));
                    let text = [Some(failed.check.as_str()), failed.test.as_deref()].into_iter().flatten().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" · ");
                    let at = since.clone().unwrap_or_else(never);
                    self.line_call(calls, format!("main:{}", view.root), StandingCallKind::Main, "main is red".into(), text, owner, since.as_ref(), at);
                }
            }
            // A held checkout is told here and answered on the merge queue's checkout light.
            if let Some(held) = &view.checkout {
                let commits = |count: u32| format!("{count} {}", if count == 1 { "commit" } else { "commits" });
                let text = if !held.clashing.is_empty() {
                    "Uncommitted changes main also changed".to_string()
                } else if !held.uncommitted.is_empty() {
                    "Uncommitted changes".to_string()
                } else if held.ahead > 0 {
                    format!("{} not on {}", commits(held.ahead), held.base)
                } else if let Some(on) = &held.on {
                    format!("On {on}")
                } else {
                    String::new()
                };
                self.line_call(calls, format!("checkout:{}", view.root), StandingCallKind::Checkout, format!("{} behind {}", commits(held.behind), held.base), text, None, None, never());
            }
            for pull in view.pulls.iter().map(|listed| &listed.pull) {
                let failing = pull.ci == Some(ipc::HubPullCi::Failed) || pull.queue.as_ref().is_some_and(|queue| queue.state == ipc::HubQueueState::Unmergeable);
                // A form edit the forge would not auto-merge waits for a person.
                if !failing && pull.merge_by_hand {
                    let updated = self.lines.lines.iter().flat_map(|line| &line.pulls).find(|one| one.pull.number == pull.number).and_then(|one| one.pull.updated_at.as_ref());
                    let owner = self.pull_owner(pull.number, None);
                    self.line_call(calls, format!("pull:{}", pull.number), StandingCallKind::Pull, format!("armada.yml edit #{}", pull.number), "Needs merging by hand".into(), owner, updated, updated.cloned().unwrap_or_else(never));
                    continue;
                }
                if !failing {
                    continue;
                }
                let text = if pull.ci == Some(ipc::HubPullCi::Failed) { "checks failed" } else { "unmergeable" };
                // `failedAt` finds the number on the first line that lists it, as the call's key names no repository.
                let updated = self.lines.lines.iter().flat_map(|line| &line.pulls).find(|one| one.pull.number == pull.number).and_then(|one| one.pull.updated_at.as_ref());
                let at = updated.cloned().unwrap_or_else(never);
                let owner = self.pull_owner(pull.number, None);
                self.line_call(calls, format!("pull:{}", pull.number), StandingCallKind::Pull, format!("#{} {}", pull.number, pull.branch), text.into(), owner, updated, at);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn line_call(
        &self,
        calls: &mut Vec<StandingCall>,
        key: String,
        kind: StandingCallKind,
        title: String,
        text: String,
        owner: Option<Owner<'_>>,
        failed_at: Option<&Instant>,
        since: Instant,
    ) {
        if owner.as_ref().is_some_and(|owner| Self::on_it(owner, failed_at.map(millis))) {
            return;
        }
        let from = owner.as_ref().map(|owner| owner.id);
        calls.push(StandingCall {
            owner: from.map(|id| format!("session:{id}")).unwrap_or_else(|| key.clone()),
            session_id: from.map(ipc::SessionId::carried),
            opened: self.opened.contains(&key),
            key,
            kind,
            title,
            text,
            since,
            tone: CallTone::Waiting,
        });
    }

    /// The calls now, and what changed since the last publish, or none where neither did.
    pub(crate) fn changed(&mut self, on_screen: &dyn Fn(&str) -> bool, now: i64) -> Option<CallsChanged> {
        let calls = self.standing(on_screen, now);
        let arrived = std::mem::take(&mut self.news);
        if arrived.is_empty() && calls == self.published {
            return None;
        }
        self.published = calls.clone();
        Some(CallsChanged { calls, arrived })
    }

    /// He opened a main or pull request call. A key that names none standing opens nothing.
    pub(crate) fn open(&mut self, key: &str, on_screen: &dyn Fn(&str) -> bool, now: i64) {
        let standing = self.standing(on_screen, now);
        if standing.iter().any(|one| one.key == key && one.kind != StandingCallKind::Job) {
            self.opened.insert(key.to_string());
        }
    }
}

/// What Fleet keeps of the calls, in memory.
#[derive(Default)]
pub(crate) struct Kept(Mutex<Keeper>);

impl Kept {
    /// When anything but the agent last said something to the Job, off what the ledger read of its thread:
    /// `isOnIt` reads it. Noted as the row is built, so it is here before the row is heard.
    pub(crate) fn told_noted(&self, session: &str, at: Option<&str>) {
        let mut keeper = self.0.lock().expect("held across no panic");
        match at.map(|at| millis(&Instant::carried(at))) {
            Some(at) => keeper.told.insert(session.to_string(), at),
            None => keeper.told.remove(session),
        };
    }
}

#[cfg(test)]
impl Kept {
    /// What stands has been read, so what moves from here is news.
    pub(crate) fn seeded(&self) -> bool {
        self.0.lock().expect("held across no panic").seeded
    }
}

/// What the follower hears.
enum Heard {
    Job(Box<SessionRecord>),
    Lines(MergeLineViews),
}

/// Start telling the calls: seeded from what stands, then `calls.changed` on every row and merge line that
/// moves them. Production starts it beside the rail's, and a test starts it itself.
pub fn keep_the_calls<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<Heard>();
    fleet.events().tap(move |event| {
        let one = match event {
            // A row that no ledger read built carries no mark, and says nothing of what he is waited on for.
            Event::SessionChanged(record) if record.mark.is_some() => Heard::Job(Box::new(record.clone())),
            Event::MergeLineViewChanged(views) => Heard::Lines(views.clone()),
            _ => return,
        };
        let _ = tell.send(one);
    });
    tokio::spawn(async move {
        let records = Arc::clone(&fleet).list_sessions(None, None, None).await.map(|list| list.sessions).unwrap_or_default();
        let lines = crate::merge_lines::view(fleet.as_ref()).await;
        fleet.hosts().calls.0.lock().expect("held across no panic").seed(&records, lines);
        while let Some(one) = heard.recv().await {
            {
                let mut keeper = fleet.hosts().calls.0.lock().expect("held across no panic");
                match one {
                    Heard::Job(record) => keeper.hear(&record),
                    Heard::Lines(lines) => keeper.lines = lines,
                }
            }
            fleet.calls_told();
        }
    })
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
    /// `calls.changed`, when the calls or the news moved since it was last told.
    pub(crate) fn calls_told(&self) {
        let told = {
            let on_screen = |id: &str| self.hosts().on_screen.now(id);
            let mut keeper = self.hosts().calls.0.lock().expect("held across no panic");
            keeper.changed(&on_screen, self.now().epoch_millis().unwrap_or(0))
        };
        if let Some(told) = told {
            self.publish(Event::CallsChanged(told));
        }
    }

    fn calls_standing(&self) -> StandingCalls {
        let on_screen = |id: &str| self.hosts().on_screen.now(id);
        let keeper = self.hosts().calls.0.lock().expect("held across no panic");
        StandingCalls { calls: keeper.standing(&on_screen, self.now().epoch_millis().unwrap_or(0)) }
    }

    /// `open_call`: opened where it is a main or pull request call standing, then told.
    pub(crate) fn call_opened(&self, asked: &OpenStandingCall) -> StandingCalls {
        {
            let on_screen = |id: &str| self.hosts().on_screen.now(id);
            let mut keeper = self.hosts().calls.0.lock().expect("held across no panic");
            keeper.open(&asked.key, &on_screen, self.now().epoch_millis().unwrap_or(0));
        }
        self.calls_told();
        self.calls_standing()
    }

    /// `get_calls`.
    pub(crate) fn calls_read(&self) -> Result<StandingCalls, Refusal> {
        Ok(self.calls_standing())
    }
}
