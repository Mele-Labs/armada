//! The Operator's apparatus: the three live Jobs a report may be handed to, a model call that
//! answers what the test wrote, and a Fleet that keeps the Log and acts on what it is handed in memory.
//!
//! The call is in-process rather than `FakeJudge`'s shell, so a late call is late on tokio's
//! paused clock and the test waits no real 20 seconds. The real call is `fleet`'s own
//! `operator::KeptStarted`, proved in `fleet`'s `tests::operator`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_model::Timestamp;
use fleet::operator::{carried, Acted, Acting, Answering, Arrival, Board, Calling, Desk, Done, Filed, LiveJob, LogRead, Logged, Operator, Order, Orders, Repository, Ruling, Way};
use fleet::CallFailed;
use ipc::operator_positions::{OperatorPosition, OperatorRole};
use ipc::operator_talk::OperatorOrderForm;

pub const REPOSITORY: &str = "01ARMADAREPOSITORY";
pub const NOW: &str = "2026-11-03T15:30:00.000Z";

/// Where the three Jobs met, as the write clash Fleet told of that afternoon.
pub const PANES: &str = "apps/desktop/src/renderer/src/job-panes/";

/// The three Jobs, oldest first.
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

/// The issues open in [`REPOSITORY`]'s beads as the sources arrive.
pub const SCROLLBACK_ISSUE: &str = "armada-a3f2";
pub fn open_issues() -> Vec<Filed> {
    vec![
        Filed { reference: SCROLLBACK_ISSUE.into(), title: "Terminal scrollback is lost when the pane redraws".into() },
        Filed { reference: "armada-b71c".into(), title: "Usage meter shows yesterday's tokens".into() },
    ]
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

/// Fleet, as far as the Operator reaches it: the live Jobs, the Log and what
/// becomes of each Job.
pub struct FakeFleet {
    live: Vec<LiveJob>,
    log: Mutex<Vec<Logged>>,
    sourced: Mutex<Vec<Sourced>>,
    stands: Mutex<Vec<(OperatorRole, OperatorPosition)>>,
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
    pub fn with(live: Vec<LiveJob>) -> Arc<FakeFleet> {
        Arc::new(FakeFleet {
            live,
            log: Mutex::new(Vec::new()),
            sourced: Mutex::new(Vec::new()),
            stands: Mutex::new(Vec::new()),
            suggested: Mutex::new(Vec::new()),
            orders: Mutex::new(Orders::default()),
            log_read: Mutex::new(Vec::new()),
        })
    }

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

    /// Where `role` stands from here on. The others stand On.
    pub fn role_stands(&self, role: OperatorRole, position: OperatorPosition) {
        self.stands.lock().expect("not poisoned").push((role, position));
    }

    /// What a role at Suggest would do: the arrival and the way.
    pub fn suggested(&self) -> Vec<(String, Way)> {
        self.suggested.lock().expect("not poisoned").clone()
    }

    /// What each ruling on a report, an issue, a red main, a note or an error did.
    pub fn sourced(&self) -> Vec<Sourced> {
        self.sourced.lock().expect("not poisoned").clone()
    }

    /// The Log, oldest first.
    pub fn log(&self) -> Vec<Logged> {
        self.log.lock().expect("not poisoned").clone()
    }
}

impl Desk for FakeFleet {
    fn now(&self) -> Timestamp {
        Timestamp::from_rfc3339(NOW)
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

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move { self.log.lock().expect("not poisoned").push(row) })
    }

}

impl Acting for FakeFleet {
    fn position(&self, role: Option<OperatorRole>, manifest: &str) -> OperatorPosition {
        if role.is_some_and(|role| self.orders.lock().expect("not poisoned").role_off(role, manifest, &Timestamp::from_rfc3339(NOW))) {
            return OperatorPosition::ReadOnly;
        }
        let stands = self.stands.lock().expect("not poisoned");
        role.and_then(|role| stands.iter().rev().find(|(one, _)| *one == role).map(|(_, position)| *position)).unwrap_or(OperatorPosition::Decide)
    }

    fn suggest(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        self.suggested.lock().expect("not poisoned").push((arrival.id.clone(), ruling.way));
        carried()
    }

    fn sourced(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        let done = Sourced { arrival: arrival.id.clone(), way: ruling.way, other: ruling.other.clone(), issue: ruling.issue.clone() };
        self.sourced.lock().expect("not poisoned").push(done);
        carried()
    }
}

/// An Operator over `call`, with `fleet` as its Desk and the [`Acting`] its rulings reach.
pub fn operator(call: &Arc<FakeCall>, fleet: &Arc<FakeFleet>) -> Arc<Operator> {
    Operator::new(call.clone(), fleet.clone(), fleet.clone())
}
