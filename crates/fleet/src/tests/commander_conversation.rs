//! A Job's Commander under a real Fleet, against a scripted host. `#2166`.
//!
//! **The host is scripted and everything above it is real**: the queue, the
//! shared turn, the thread, the stored session and the spend. What a session
//! says is the script's; what is asserted is what Fleet does with it.
//!
//! The wake is `#2167`'s (`tests::commanding_wakes`); `wake_commander` is the seam.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::DroneEvent;
use api::{Conversations, Refusal};
use core_model::JobId;
use ipc::{AskCommander, CommanderMessage, HelmText, Saw};
use testkit::FakeWorkProduct;
use tokio::sync::Semaphore;

use crate::allowance::{Allowance, Micros};
use crate::daemon::Fleet;
use crate::helm::{Carried, Carry, Carrying, Heard, Hosting, Seat};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

pub(super) type Hosted = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// A host that answers every turn with a call, a sentence and a turn's end, and
/// writes down what it was asked to carry. **Gated**, it holds each turn until
/// it is let go, so a case can send something while a turn is being written.
pub(super) struct Scripted {
    carried: Mutex<Vec<Carry>>,
    gate: Option<Semaphore>,
    entered: AtomicUsize,
    in_flight: AtomicUsize,
    most_at_once: AtomicUsize,
    cost_micros: u64,
    pub(super) steering: Mutex<Vec<(u32, String)>>,
}

impl Scripted {
    pub(super) fn answering() -> Arc<Scripted> {
        Arc::new(Scripted::new(None, 150_000))
    }

    fn holding_each_turn() -> Arc<Scripted> {
        Arc::new(Scripted::new(Some(Semaphore::new(0)), 150_000))
    }

    fn costing(cost_micros: u64) -> Arc<Scripted> {
        Arc::new(Scripted::new(None, cost_micros))
    }

    fn new(gate: Option<Semaphore>, cost_micros: u64) -> Scripted {
        Scripted {
            carried: Mutex::new(Vec::new()),
            gate,
            entered: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
            most_at_once: AtomicUsize::new(0),
            cost_micros,
            steering: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn carried(&self) -> Vec<Carry> {
        self.carried.lock().expect("not poisoned").clone()
    }

    fn let_go(&self, turns: usize) {
        self.gate.as_ref().expect("a gated host").add_permits(turns);
    }

    async fn until_entered(&self, turns: usize) {
        for _ in 0..500 {
            if self.entered.load(Ordering::SeqCst) >= turns {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the host was never asked to carry turn {turns}");
    }
}

impl Hosting for Scripted {
    fn carry<'a>(&'a self, carry: Carry, heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(async move {
            self.carried.lock().expect("not poisoned").push(carry);
            self.entered.fetch_add(1, Ordering::SeqCst);
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.most_at_once.fetch_max(now, Ordering::SeqCst);
            if let Some(gate) = &self.gate {
                gate.acquire().await.expect("open").forget();
            }
            heard.heard(&[
                DroneEvent::Started {
                    session: "commander-session".to_string(),
                    model: "stand-in".to_string(),
                    mcp_servers: 1,
                },
                DroneEvent::Called {
                    tool: "get_job".to_string(),
                    call: "toolu_1".to_string(),
                    detail: adapter_traits::CallDetail::of("the Job"),
                },
                DroneEvent::Said {
                    text: "Step one stopped on its Check; I raised nothing.".to_string(),
                    by: adapter_traits::Speaker::Drone,
                },
                DroneEvent::Ended {
                    turns: 2,
                    cost_micros: self.cost_micros,
                    refusals: 0,
                },
            ]);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Carried::Answered {
                session: "commander-session".to_string(),
            }
        }) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn model(&self) -> String {
        String::from("a-model")
    }

    fn running(&self) -> Vec<u32> {
        Vec::new()
    }

    fn running_commanders(&self) -> Vec<(u32, String)> {
        self.steering.lock().expect("not poisoned").clone()
    }
}

pub(super) fn hosted(home: &TempDir, host: &Arc<Scripted>) -> Arc<Hosted> {
    Arc::new(
        Fleet::assembled(fittings(home, FakeWorkProduct::changed(&["src/log.rs"])))
            .hosting_helm_on(Arc::clone(host) as Arc<dyn Hosting>),
    )
}

/// A Job at work, with the worktree its Drone would run in.
pub(super) async fn working(fleet: &Hosted, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    job.id().clone()
}

pub(super) async fn resolved(fleet: &Hosted, job: &JobId) -> api::Resolved {
    let record = fleet.load(job).await.expect("the Job reads");
    api::Resolved::of(ipc::JobId::from(job), record.handle())
}

async fn ask(fleet: &Arc<Hosted>, job: &JobId, text: &str) -> ipc::CommanderConversation {
    Arc::clone(fleet)
        .ask_commander(
            resolved(fleet, job).await,
            AskCommander {
                text: HelmText::said(text).expect("not blank"),
            },
        )
        .await
        .expect("the words are taken")
}

pub(super) async fn said(fleet: &Hosted, job: &JobId) -> Vec<CommanderMessage> {
    let observed = fleet
        .observe_commander(resolved(fleet, job).await)
        .await
        .expect("observed");
    observed.history
}

pub(super) async fn settled(fleet: &Hosted, job: &JobId) {
    for _ in 0..500 {
        let observed = fleet
            .observe_commander(resolved(fleet, job).await)
            .await
            .expect("observed");
        if !observed.replying {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Commander never settled");
}

fn asked_rows(thread: &[CommanderMessage]) -> Vec<&str> {
    thread
        .iter()
        .filter_map(|message| match message {
            CommanderMessage::Asked(asked) => Some(asked.text.as_str()),
            _ => None,
        })
        .collect()
}

pub(super) fn woke_rows(thread: &[CommanderMessage]) -> Vec<&str> {
    thread
        .iter()
        .filter_map(|message| match message {
            CommanderMessage::Woke(woke) => Some(woke.why.as_str()),
            _ => None,
        })
        .collect()
}

fn spoken(thread: &[CommanderMessage]) -> Vec<&str> {
    thread
        .iter()
        .filter_map(|message| match message {
            CommanderMessage::Row(shown) => match &shown.row().saw {
                Saw::Said { text } => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn the_owners_words_are_a_row_and_a_turn_in_the_jobs_worktree_that_is_answered() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;

    let taken = ask(&fleet, &job, "why did step one stop").await;
    assert!(taken.replying && !taken.resumes, "{taken:?}");
    settled(&fleet, &job).await;

    let thread = said(&fleet, &job).await;
    assert_eq!(asked_rows(&thread), ["why did step one stop"]);
    assert_eq!(
        spoken(&thread),
        ["Step one stopped on its Check; I raised nothing."],
        "the reply is on the thread, after the words it answers"
    );

    let carried = host.carried();
    assert_eq!(carried.len(), 1);
    let record = fleet.load(&job).await.expect("reads");
    let worktree = fleet
        .worktree_of(&record)
        .expect("reads the disk")
        .expect("the Job has a worktree");
    assert_eq!(
        carried[0].directory,
        worktree.path(),
        "hosted in the Job's worktree and never the owner's checkout"
    );
    assert_ne!(carried[0].directory, home.path().to_string_lossy());
    match &carried[0].seat {
        Seat::Commander(commander) => {
            assert_eq!(commander.job, job.as_str());
            assert_eq!(commander.model.as_str(), "the-commander-model");
        }
        other => panic!("not a Commander's seat: {other:?}"),
    }
    assert!(
        carried[0].turn.contains("You are the Commander of one Job"),
        "a new session is told the brief: {}",
        carried[0].turn
    );
    assert!(carried[0]
        .turn
        .contains("The owner wrote to you: why did step one stop"));
}

#[tokio::test]
async fn the_second_message_resumes_the_session_and_is_not_told_the_brief_again() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;

    ask(&fleet, &job, "first").await;
    settled(&fleet, &job).await;
    let second = ask(&fleet, &job, "and the second").await;
    assert!(second.resumes, "{second:?}");
    settled(&fleet, &job).await;

    let carried = host.carried();
    assert_eq!(carried[1].resuming.as_deref(), Some("commander-session"));
    assert!(!carried[1].turn.contains("You are the Commander"));
    let manifest = fleet
        .served_by_id(&job)
        .expect("served")
        .manifest()
        .id()
        .as_str()
        .to_string();
    let store = fleet.store().lock().await;
    assert_eq!(
        store
            .helm_session(&format!("commander:{}", job.as_str()))
            .expect("reads")
            .as_deref(),
        Some("commander-session"),
        "kept under the Job's key"
    );
    assert_eq!(
        store.helm_session(&manifest).expect("reads"),
        None,
        "and not under the repository's, which is Helm's"
    );
}

/// **A message sent mid-turn is on the thread at once and answered after.**
#[tokio::test]
async fn words_sent_mid_turn_wait_for_the_next_turn_and_never_interrupt_this_one() {
    let home = TempDir::new();
    let host = Scripted::holding_each_turn();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;

    ask(&fleet, &job, "first").await;
    host.until_entered(1).await;
    ask(&fleet, &job, "second").await;
    ask(&fleet, &job, "third").await;

    let during = said(&fleet, &job).await;
    assert_eq!(
        asked_rows(&during),
        ["first", "second", "third"],
        "all three are on the thread while the first is being answered"
    );
    assert_eq!(host.carried().len(), 1, "and nothing started a second turn");

    host.let_go(2);
    settled(&fleet, &job).await;

    let carried = host.carried();
    assert_eq!(carried.len(), 2, "the two that waited are one turn");
    assert!(carried[1].turn.contains("The owner wrote to you: second"));
    assert!(carried[1].turn.contains("The owner wrote to you: third"));
    assert_eq!(host.most_at_once.load(Ordering::SeqCst), 1, "one at a time");
}

/// **Wakes that arrive during a turn are one turn after it.** `#2167` decides
/// what is worth a wake; the queue is here.
#[tokio::test]
async fn wakes_during_a_turn_coalesce_into_the_one_turn_after_it() {
    let home = TempDir::new();
    let host = Scripted::holding_each_turn();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;

    Arc::clone(&fleet)
        .wake_commander(&job, "a Drone ended with nothing handed in")
        .await
        .expect("woken");
    host.until_entered(1).await;
    for why in [
        "the cost cap is nearly spent",
        "a Drone ended with nothing handed in",
        "the cost cap is nearly spent",
    ] {
        Arc::clone(&fleet)
            .wake_commander(&job, why)
            .await
            .expect("woken");
    }
    assert_eq!(host.carried().len(), 1, "a wake never interrupts");

    host.let_go(2);
    settled(&fleet, &job).await;

    let carried = host.carried();
    assert_eq!(carried.len(), 2, "three wakes are one turn: {}", carried.len());
    assert_eq!(
        carried[1]
            .turn
            .matches("Fleet woke you: the cost cap is nearly spent")
            .count(),
        1,
        "a reason said twice is said once: {}",
        carried[1].turn
    );
    assert!(carried[1]
        .turn
        .contains("Fleet woke you: a Drone ended with nothing handed in"));
    let thread = said(&fleet, &job).await;
    assert_eq!(
        woke_rows(&thread),
        [
            "a Drone ended with nothing handed in",
            "the cost cap is nearly spent",
            "a Drone ended with nothing handed in"
        ],
        "each turn names its reasons, once"
    );
}

/// **A Commander that cannot start leaves the escalation to the caller**, with
/// a line in the Job's log naming why.
#[tokio::test]
async fn a_commander_with_no_worktree_cannot_be_woken_and_the_job_log_says_why() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = fleet
        .propose(a_proposal("still at the approval gate"))
        .await
        .expect("a proposal");

    let refused = Arc::clone(&fleet)
        .wake_commander(job.id(), "a Drone ended with nothing handed in")
        .await
        .expect_err("nowhere to host it");
    assert!(refused.why.contains("no worktree"), "{refused:?}");

    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &job.handle(),
    ))
    .expect("the Job's log");
    assert!(
        log.contains("the Job's Commander could not be woken") && log.contains("no worktree"),
        "{log}"
    );
    assert!(host.carried().is_empty());

    let owner = Arc::clone(&fleet)
        .ask_commander(
            resolved(&fleet, job.id()).await,
            AskCommander {
                text: HelmText::said("hello").expect("not blank"),
            },
        )
        .await
        .expect_err("the owner is told too");
    assert!(matches!(owner, Refusal::IllegalMove(_)), "{owner:?}");
}

/// **The Commander's turns are the Job's cost and never its cap's.**
#[tokio::test]
async fn what_a_turn_costs_is_the_jobs_cost_and_none_of_its_caps() {
    let home = TempDir::new();
    let host = Scripted::costing(9_000_000);
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.allowance = Allowance::of(Micros::dollars(5), 300);
    let fleet = Arc::new(
        Fleet::assembled(fitted).hosting_helm_on(Arc::clone(&host) as Arc<dyn Hosting>),
    );
    let job = working(&fleet, &home, "bound the reader").await;

    ask(&fleet, &job, "how is it going").await;
    settled(&fleet, &job).await;

    let store = fleet.store().lock().await;
    assert_eq!(store.commander_cost_for(&job).expect("reads"), 9_000_000);
    let drones = store.spend_for(&job).expect("reads");
    assert_eq!(drones.cost_micros, 0, "no Drone spent it");
    drop(store);

    let record = fleet.load(&job).await.expect("reads");
    let spent = fleet.spend_of(&job).await.expect("reads");
    assert_eq!(
        fleet.allowance_for(&record).exceeded_by(&spent),
        None,
        "$9 of Commander against a $5 cap holds nothing back"
    );

    let detail = api::Queries::get_job(&*fleet, ipc::JobId::from(&job))
        .await
        .expect("the Job reads");
    let spend = detail.spend.expect("a spend is served");
    assert_eq!(
        spend.commander_cost_micros, 9_000_000,
        "the Job's cost shows it"
    );
    assert_eq!(spend.cost_micros, 0, "beside the Drones', not in it");
    assert_eq!(spend.cost_cap_micros, 5_000_000);
}
