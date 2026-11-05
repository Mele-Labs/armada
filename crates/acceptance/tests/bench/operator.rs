//! The Operator's apparatus: the three Jobs of the 3 Nov report, a model call that answers what
//! the test wrote, and a Fleet that keeps the Log and acts on Jobs in memory.
//!
//! The call is in-process rather than `FakeJudge`'s shell, so a late call is late on tokio's
//! paused clock and the test waits no real 20 seconds. The real call is `fleet`'s own
//! `operator::KeptStarted`, proved in `fleet`'s `tests::operator`.

use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_model::Timestamp;
use fleet::operator::{
    Acting, Answering, Arrival, Arrived, Board, Calling, Choice, Desk, Done, Earlier, Filed, Group, Kind, LiveJob, LogRead, Logged, Operator, Order, Orders, Present, Question,
    Quiet, Repository, Ruling, Source, Stall, Way,
};
use fleet::CallFailed;
use ipc::operator_positions::{OperatorPosition, OperatorRole};
use ipc::operator_talk::OperatorOrderForm;

pub const REPOSITORY: &str = "01ARMADAREPOSITORY";
pub const NOW: &str = "2026-11-03T15:30:00.000Z";

/// Where the three Jobs met, as the write clash Fleet told of that afternoon.
pub const PANES: &str = "apps/desktop/src/renderer/src/job-panes/";

/// The three Jobs, oldest first. Terminal Bugs started the third and handed it a group of its own plan.
pub const TABS: &str = "01JOBPRREVIEWTABS";
pub const TERMINAL: &str = "01JOBTERMINALBUGS";
pub const IN_TAB: &str = "01JOBTERMINALINTAB";

pub fn live() -> Vec<LiveJob> {
    vec![
        LiveJob {
            session_id: TABS.into(),
            title: "Tmux-like PR Review Tabs".into(),
            manifest_id: REPOSITORY.into(),
            words: "Give pull request review tabs like tmux: a tab per open review, keys to move between them".into(),
            paths: vec![format!("{PANES}tabs.tsx"), format!("{PANES}pane-host.tsx")],
            issues: Vec::new(),
            running: Vec::new(),
            started_by: Vec::new(),
        },
        LiveJob {
            session_id: TERMINAL.into(),
            title: "Terminal Bugs".into(),
            manifest_id: REPOSITORY.into(),
            words: "The terminal pane loses its scrollback, and open terminals in a new tab".into(),
            paths: vec![format!("{PANES}terminal-pane.tsx"), format!("{PANES}pane-host.tsx")],
            issues: Vec::new(),
            running: Vec::new(),
            started_by: Vec::new(),
        },
        LiveJob {
            session_id: IN_TAB.into(),
            title: "Agent Opens Terminal in Tab".into(),
            manifest_id: REPOSITORY.into(),
            words: "When an agent opens a terminal, open it in a tab of the Job's panes".into(),
            paths: vec![format!("{PANES}terminal-pane.tsx")],
            issues: Vec::new(),
            running: Vec::new(),
            started_by: Vec::new(),
        },
    ]
}

/// The start of one of [`live`]'s Jobs, as Fleet hands it to the Operator.
pub fn started(session: &str, source: Source) -> Arrival {
    let job = live().into_iter().find(|one| one.session_id == session).expect("one of the three");
    Arrival {
        id: format!("start:{session}"),
        arrived: Arrived::Start,
        source,
        session_id: job.session_id,
        manifest_id: job.manifest_id,
        subject: job.title,
        words: job.words,
        paths: job.paths,
        issues: job.issues,
        group: None,
        ways: Way::STARTING.to_vec(),
        answers: Vec::new(),
        rule: None,
        open: Vec::new(),
    }
}

/// The issues open in [`REPOSITORY`]'s beads as the sources arrive.
pub const SCROLLBACK_ISSUE: &str = "armada-a3f2";
pub fn open_issues() -> Vec<Filed> {
    vec![
        Filed { reference: SCROLLBACK_ISSUE.into(), title: "Terminal scrollback is lost when the pane redraws".into() },
        Filed { reference: "armada-b71c".into(), title: "Usage meter shows yesterday's tokens".into() },
    ]
}

/// The question the 3 Nov report's Jobs each asked, as an agent's options put it.
pub const ASKED: &str = "Should a terminal an agent opens keep its scrollback per tab, or share one?";
pub const PER_TAB: &str = "Keep it per tab (recommended)";

/// [`ASKED`], from one of [`live`]'s Jobs working on `group`, while the person is `present`.
pub fn asked(session: &str, group: Group, present: Present, earlier: Option<Earlier>) -> Question {
    let job = live().into_iter().find(|one| one.session_id == session).expect("one of the three");
    Question {
        session_id: job.session_id,
        manifest_id: job.manifest_id,
        title: job.title,
        item_id: "ask:1".into(),
        text: ASKED.into(),
        options: vec![PER_TAB.into(), "Share one scrollback".into()],
        kind: Kind::Recommended(0),
        group,
        earlier,
        present,
    }
}

/// One of [`live`]'s Jobs gone quiet as `quiet` says, with its plan's tasks `left`.
pub fn stall(session: &str, quiet: Quiet, left: &[&str], nudged: bool) -> Stall {
    let job = live().into_iter().find(|one| one.session_id == session).expect("one of the three");
    Stall {
        session_id: job.session_id,
        manifest_id: job.manifest_id,
        title: job.title,
        quiet,
        plan: Some(job.words),
        done: vec!["Find where the scrollback is dropped".into()],
        left: left.iter().map(|one| one.to_string()).collect(),
        nudged,
    }
}

/// What one call does: answer after a while, or fail after a while.
pub enum Reply {
    Says(String, Duration),
    Fails(Duration),
}

/// A model call that answers each call with the next of its replies, and keeps what it was asked.
pub struct FakeCall {
    replies: Mutex<VecDeque<Reply>>,
    asked: Mutex<Vec<String>>,
}

impl FakeCall {
    pub fn replying(replies: Vec<Reply>) -> Arc<FakeCall> {
        Arc::new(FakeCall { replies: Mutex::new(replies.into()), asked: Mutex::new(Vec::new()) })
    }

    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl Calling for FakeCall {
    fn ask(&self, question: String) -> Answering<'_> {
        self.asked.lock().expect("not poisoned").push(question);
        let reply = self.replies.lock().expect("not poisoned").pop_front();
        Box::pin(async move {
            match reply {
                Some(Reply::Says(answer, after)) => {
                    tokio::time::sleep(after).await;
                    Ok(answer)
                }
                Some(Reply::Fails(after)) => {
                    tokio::time::sleep(after).await;
                    Err(CallFailed::Refused { code: Some(1), tail: String::new(), said: "the service is overloaded".into() })
                }
                None => Err(CallFailed::SaidNothing),
            }
        })
    }
}

/// Fleet, as far as the Operator reaches it: the live Jobs, the Log, the day's ceiling and what
/// becomes of each Job.
pub struct FakeFleet {
    live: Vec<LiveJob>,
    log: Mutex<Vec<Logged>>,
    ceiling: u32,
    ended: Mutex<Vec<(String, String)>>,
    in_touch: Mutex<Vec<(String, String)>>,
    nudged: Mutex<Vec<(String, String)>>,
    raised: Mutex<Vec<(String, String)>>,
    started_anew: Mutex<Vec<String>>,
    answered: Mutex<Vec<(String, String, u32, String)>>,
    answered_first: Mutex<bool>,
    went_ahead: Mutex<Vec<String>>,
    sourced: Mutex<Vec<Sourced>>,
    answer_stands: Mutex<OperatorPosition>,
    suggested: Mutex<Vec<(String, Way)>>,
    orders: Mutex<Orders>,
    log_read: Mutex<Vec<LogRead>>,
}

/// What one ruling on a source did: the arrival, the way, the other Job and the issue it named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sourced {
    pub arrival: String,
    pub way: Way,
    pub other: Option<String>,
    pub issue: Option<String>,
}

impl FakeFleet {
    pub fn with(live: Vec<LiveJob>, ceiling: u32) -> Arc<FakeFleet> {
        Arc::new(FakeFleet {
            live,
            log: Mutex::new(Vec::new()),
            ceiling,
            ended: Mutex::new(Vec::new()),
            in_touch: Mutex::new(Vec::new()),
            nudged: Mutex::new(Vec::new()),
            raised: Mutex::new(Vec::new()),
            started_anew: Mutex::new(Vec::new()),
            answered: Mutex::new(Vec::new()),
            answered_first: Mutex::new(false),
            went_ahead: Mutex::new(Vec::new()),
            sourced: Mutex::new(Vec::new()),
            answer_stands: Mutex::new(OperatorPosition::On),
            suggested: Mutex::new(Vec::new()),
            orders: Mutex::new(Orders::default()),
            log_read: Mutex::new(Vec::new()),
        })
    }

    /// Each Job told to carry on, and the next step it was told.
    pub fn nudged(&self) -> Vec<(String, String)> {
        self.nudged.lock().expect("not poisoned").clone()
    }

    /// Each Job raised to the person, and the Operator's read.
    pub fn raised(&self) -> Vec<(String, String)> {
        self.raised.lock().expect("not poisoned").clone()
    }

    /// Each Job whose rest was started as a new Job.
    pub fn started_anew(&self) -> Vec<String> {
        self.started_anew.lock().expect("not poisoned").clone()
    }

    /// Each item answered as the person's press would: the Job, the item, the option and its label.
    /// Where the Answer role stands from here on. The others stand On.
    /// A row of the Log the box may cite.
    pub fn logged_earlier(&self, row: LogRead) {
        self.log_read.lock().expect("not poisoned").push(row);
    }

    /// The person pressed: `form` stands, as `fleet`'s `operator_order_confirmed` keeps it.
    pub fn order_stands(&self, form: OperatorOrderForm) {
        let mut orders = self.orders.lock().expect("not poisoned");
        let id = orders.0.len() as i64 + 1;
        orders.0.push(Order { id, form, until: None });
    }

    pub fn answer_stands(&self, position: OperatorPosition) {
        *self.answer_stands.lock().expect("not poisoned") = position;
    }

    /// What a role at Suggest would do: the Job and the way.
    pub fn suggested(&self) -> Vec<(String, Way)> {
        self.suggested.lock().expect("not poisoned").clone()
    }

    pub fn answered(&self) -> Vec<(String, String, u32, String)> {
        self.answered.lock().expect("not poisoned").clone()
    }

    /// Each Job let start as its own.
    pub fn went_ahead(&self) -> Vec<String> {
        self.went_ahead.lock().expect("not poisoned").clone()
    }

    /// What each ruling on a report, an issue, a red main, a review request or a Later item did.
    pub fn sourced(&self) -> Vec<Sourced> {
        self.sourced.lock().expect("not poisoned").clone()
    }

    /// The person answers every item himself, ahead of any ruling.
    pub fn answer_first(&self) {
        *self.answered_first.lock().expect("not poisoned") = true;
    }

    /// The Log, oldest first.
    pub fn log(&self) -> Vec<Logged> {
        self.log.lock().expect("not poisoned").clone()
    }

    /// The Jobs still working once every ruling was acted on.
    pub fn carrying_on(&self) -> BTreeSet<String> {
        let ended = self.ended.lock().expect("not poisoned");
        self.live
            .iter()
            .map(|one| one.session_id.clone())
            .filter(|id| !ended.iter().any(|(gone, _)| gone == id))
            .collect()
    }

    /// Each pair of Jobs told of the other, in the order the pair was named.
    pub fn in_touch(&self) -> Vec<(String, String)> {
        self.in_touch.lock().expect("not poisoned").clone()
    }

    /// Each Job that ended into another, and that other.
    pub fn ended_into(&self) -> Vec<(String, String)> {
        self.ended.lock().expect("not poisoned").clone()
    }
}

impl Desk for FakeFleet {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339(NOW)
    }

    fn day_began(&self) -> Done<'_, Timestamp> {
        Box::pin(async move { Timestamp::from_rfc3339(format!("{}T00:00:00.000Z", &NOW[..10])) })
    }

    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move { self.live.clone() })
    }

    fn orders(&self) -> Done<'_, Orders> {
        Box::pin(async move { self.orders.lock().expect("not poisoned").clone() })
    }

    fn board(&self) -> Done<'_, Board> {
        Box::pin(async move {
            Board {
                log: self.log_read.lock().expect("not poisoned").clone(),
                repositories: vec![Repository { id: REPOSITORY.to_string(), name: "armada".to_string() }],
                ..Board::default()
            }
        })
    }

    fn daily_calls(&self) -> u32 {
        self.ceiling
    }

    fn calls_since(&self, since: Timestamp) -> Done<'_, u32> {
        Box::pin(async move {
            let log = self.log.lock().expect("not poisoned");
            let calls: BTreeSet<&str> = log
                .iter()
                .filter(|row| row.at.as_str() >= since.as_str())
                .filter_map(|row| row.call.as_deref())
                .collect();
            calls.len() as u32
        })
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move { self.log.lock().expect("not poisoned").push(row) })
    }

    fn settled(&self, _arrival: &Arrival) -> Done<'_, Option<String>> {
        let first = *self.answered_first.lock().expect("not poisoned");
        Box::pin(async move { first.then(|| "the person answered it before the ruling came".to_string()) })
    }
}

impl Acting for FakeFleet {
    fn position(&self, role: Option<OperatorRole>, manifest: &str) -> OperatorPosition {
        if role.is_some_and(|role| self.orders.lock().expect("not poisoned").role_off(role, manifest, &Timestamp::from_rfc3339(NOW))) {
            return OperatorPosition::Off;
        }
        match role {
            Some(OperatorRole::Answer) => *self.answer_stands.lock().expect("not poisoned"),
            _ => OperatorPosition::On,
        }
    }

    fn suggest(&self, arrival: &Arrival, ruling: &Ruling) {
        self.suggested.lock().expect("not poisoned").push((arrival.session_id.clone(), ruling.way));
    }

    fn go_ahead(&self, session: &str) {
        self.went_ahead.lock().expect("not poisoned").push(session.into());
    }

    fn hand_over(&self, session: &str, _group: Option<&str>, into: &str, _shared: &[String]) {
        self.ended.lock().expect("not poisoned").push((session.into(), into.into()));
    }

    fn merge(&self, ending: &str, into: &str, _shared: &[String]) {
        self.ended.lock().expect("not poisoned").push((ending.into(), into.into()));
    }

    fn keep_in_touch(&self, one: &str, other: &str, _shared: &[String]) {
        self.in_touch.lock().expect("not poisoned").push((one.into(), other.into()));
    }

    fn nudge(&self, session: &str, step: &str) {
        self.nudged.lock().expect("not poisoned").push((session.into(), step.into()));
    }

    fn start_rest(&self, session: &str, _seen: &str) {
        self.started_anew.lock().expect("not poisoned").push(session.into());
    }

    fn raise(&self, session: &str, read: &str) {
        self.raised.lock().expect("not poisoned").push((session.into(), read.into()));
    }

    fn answer(&self, session: &str, item: &str, choice: &Choice, _because: &str) {
        self.answered.lock().expect("not poisoned").push((session.into(), item.into(), choice.at, choice.label.clone()));
    }

    fn sourced(&self, arrival: &Arrival, ruling: &Ruling) {
        let done = Sourced { arrival: arrival.id.clone(), way: ruling.way, other: ruling.other.clone(), issue: ruling.issue.clone() };
        self.sourced.lock().expect("not poisoned").push(done);
    }
}

/// An Operator over `call`, with `fleet` as its Desk and the [`Acting`] its rulings reach.
pub fn operator(call: &Arc<FakeCall>, fleet: &Arc<FakeFleet>) -> Arc<Operator> {
    Operator::new(call.clone(), fleet.clone(), fleet.clone())
}
