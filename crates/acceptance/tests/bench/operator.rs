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
use fleet::operator::{Acting, Answering, Arrival, Arrived, Calling, Desk, Done, LiveJob, Logged, Operator, Quiet, Source, Stall, Way};
use fleet::CallFailed;

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
        },
        LiveJob {
            session_id: TERMINAL.into(),
            title: "Terminal Bugs".into(),
            manifest_id: REPOSITORY.into(),
            words: "The terminal pane loses its scrollback, and open terminals in a new tab".into(),
            paths: vec![format!("{PANES}terminal-pane.tsx"), format!("{PANES}pane-host.tsx")],
            issues: Vec::new(),
        },
        LiveJob {
            session_id: IN_TAB.into(),
            title: "Agent Opens Terminal in Tab".into(),
            manifest_id: REPOSITORY.into(),
            words: "When an agent opens a terminal, open it in a tab of the Job's panes".into(),
            paths: vec![format!("{PANES}terminal-pane.tsx")],
            issues: Vec::new(),
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
}

impl Acting for FakeFleet {
    fn go_ahead(&self, _session: &str) {}

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
}

/// An Operator over `call`, with `fleet` as its Desk and the [`Acting`] its rulings reach.
pub fn operator(call: &Arc<FakeCall>, fleet: &Arc<FakeFleet>) -> Arc<Operator> {
    Operator::new(call.clone(), fleet.clone(), fleet.clone())
}
