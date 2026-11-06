//! The Operator's board on a real Fleet: the arrival a call is out on, those waiting their turn, what
//! it settled, how a person or his words reorder the wait, and a message about one item.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::Next;
use ipc::operator_board::MoveOnOperatorBoard;
use ipc::Event;
use tokio::sync::Semaphore;

use super::operator_acting::{asked, dispatch};
use super::session_host::{eventually, rig_placing, Rig};
use fleet::operator::{Answering, Arrival, Arrived, Calling, Source, Way};
use fleet::CallFailed;

/// A call whose rulings wait on a gate a test opens, and whose answers to the box do not.
struct Gated {
    gate: Semaphore,
    rulings: Mutex<Vec<String>>,
    talks: Mutex<Vec<String>>,
    asked: Mutex<Vec<String>>,
}

impl Gated {
    fn saying(rulings: &[&str], talks: &[&str]) -> Arc<Gated> {
        let backwards = |said: &[&str]| Mutex::new(said.iter().rev().map(|one| (*one).to_string()).collect::<Vec<_>>());
        Arc::new(Gated { gate: Semaphore::new(0), rulings: backwards(rulings), talks: backwards(talks), asked: Mutex::new(Vec::new()) })
    }

    /// What the box was asked, and what a ruling was asked.
    fn asked_of(&self, talk: bool) -> Vec<String> {
        self.asked.lock().expect("not poisoned").iter().filter(|one| one.contains("talking with the person") == talk).cloned().collect()
    }
}

impl Calling for Gated {
    fn ask(&self, question: String) -> Answering<'_> {
        let talk = question.contains("talking with the person");
        self.asked.lock().expect("not poisoned").push(question);
        let reply = if talk { self.talks.lock().expect("not poisoned").pop() } else { self.rulings.lock().expect("not poisoned").pop() };
        Box::pin(async move {
            if !talk {
                self.gate.acquire().await.expect("open").forget();
            }
            reply.ok_or(CallFailed::SaidNothing)
        })
    }
}

fn raised(items: usize) -> String {
    (1..=items).map(|at| format!("item: {at}\nway: 2\njob: none\nshared: none\nbecause: needs a person {at}\n")).collect::<Vec<_>>().join("\n")
}

fn said(first: &str, about: &str) -> String {
    format!("cites: none\njob: none\nstart: none\norder: none\nfirst: {first}\nsaid: {about}")
}

fn question(job: &str, manifest: &str, id: &str, title: &str, words: &str) -> Arrival {
    Arrival {
        id: id.to_string(),
        arrived: Arrived::Question,
        source: Source::Agent,
        session_id: job.to_string(),
        manifest_id: manifest.to_string(),
        subject: title.to_string(),
        words: words.to_string(),
        paths: Vec::new(),
        issues: Vec::new(),
        group: None,
        ways: vec![Way::Answer, Way::Raise],
        answers: Vec::new(),
        rule: None,
        open: Vec::new(),
    }
}

/// A question out in a call, and two more waiting behind it, in that order.
struct Waiting {
    rig: Rig,
    call: Arc<Gated>,
    manifest: String,
}

async fn waiting(rulings: &[&str], talks: &[&str]) -> Waiting {
    let rig = rig_placing(None);
    dispatch(&rig, false).await;
    let call = Gated::saying(rulings, talks);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let job = asked(&rig, "Keep the scrollback").await;
    let manifest = rig.fleet.store().lock().await.hosting(job.as_str()).expect("read").expect("hosted").manifest_id;
    let waiting = Waiting { rig, call, manifest };
    waiting.hand("question:first", "Keep them in the store?", "The first words").await;
    let call = waiting.call.clone();
    eventually(|| {
        let call = call.clone();
        async move { !call.asked_of(false).is_empty() }
    })
    .await;
    waiting.hand("question:second", "Which pane first?\nWith a second line", "The second words").await;
    waiting.waits_for(1).await;
    waiting.hand("question:third", "Delete the cache?", "The third words").await;
    waiting.waits_for(2).await;
    waiting
}

impl Waiting {
    async fn hand(&self, id: &str, title: &str, words: &str) {
        let job = self.rig.fleet.store().lock().await.hostings().expect("read").first().map(|one| one.session_id.clone()).expect("a Job");
        self.rig.fleet.operator_seat().hand(question(&job, &self.manifest, id, title, words), Duration::from_secs(30));
    }

    /// The handing runs in the background: until `count` wait behind the call.
    async fn waits_for(&self, count: usize) {
        let fleet = Arc::clone(&self.rig.fleet);
        eventually(|| {
            let fleet = Arc::clone(&fleet);
            async move { fleet.operator_board_wire().await.expect("read").next.len() == count }
        })
        .await;
    }

    async fn next(&self) -> Vec<String> {
        self.rig.fleet.operator_board_wire().await.expect("read").next.into_iter().map(|one| one.id).collect()
    }
}

fn code(refused: api::Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

#[tokio::test]
async fn the_board_reads_what_a_call_is_out_on_what_waits_behind_it_and_what_was_settled() {
    let rig = rig_placing(None);
    dispatch(&rig, false).await;
    let first = format!("{}\n", raised(1));
    let second = raised(2);
    let call = Gated::saying(&[&first, &second], &[]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    let mut watching = rig.fleet.events().subscribe();
    assert!(!rig.fleet.operator_board_wire().await.expect("read").now.is_some());

    let job = asked(&rig, "Keep the scrollback").await;
    let manifest = rig.fleet.store().lock().await.hosting(job.as_str()).expect("read").expect("hosted").manifest_id;
    let hand = |id: &str, title: &str| rig.fleet.operator_seat().hand(question(job.as_str(), &manifest, id, title, "words"), Duration::from_secs(30));
    hand("question:first", "Keep them in the store?");
    let waiting = Arc::clone(&call);
    eventually(|| {
        let waiting = waiting.clone();
        async move { !waiting.asked_of(false).is_empty() }
    })
    .await;
    let behind = |count: usize| {
        let fleet = Arc::clone(&rig.fleet);
        async move {
            eventually(|| {
                let fleet = Arc::clone(&fleet);
                async move { fleet.operator_board_wire().await.expect("read").next.len() == count }
            })
            .await
        }
    };
    hand("question:second", "Which pane first?\nWith a second line");
    behind(1).await;
    hand("question:third", "Delete the cache?");
    behind(2).await;

    let board = rig.fleet.operator_board_wire().await.expect("read");
    assert!(board.on);
    let now = board.now.expect("a call is out");
    assert_eq!((now.id.as_str(), now.kind, now.title.as_str(), now.session_id.as_deref()), ("question:first", ipc::operator_board::OperatorBoardKind::Question, "Keep them in the store?", Some(job.as_str())));
    assert!(now.repository.is_some() && now.waits.as_deref().is_some_and(|waits| waits.ends_with('s')), "{now:?}");
    assert_eq!(board.next.iter().map(|one| (one.id.as_str(), one.title.as_str())).collect::<Vec<_>>(), vec![("question:second", "Which pane first?"), ("question:third", "Delete the cache?")]);
    assert!(board.done.is_empty());

    call.gate.add_permits(2);
    let fleet = Arc::clone(&rig.fleet);
    eventually(|| {
        let fleet = Arc::clone(&fleet);
        async move { fleet.operator_board_wire().await.expect("read").done.len() == 3 }
    })
    .await;
    let board = rig.fleet.operator_board_wire().await.expect("read");
    assert!(board.now.is_none() && board.next.is_empty());
    assert_eq!(board.done.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), vec!["question:third", "question:second", "question:first"], "newest first");
    assert_eq!(board.done[2].outcome, "Raised: needs a person 1");
    assert_eq!(board.done[2].title, "Keep them in the store?");

    let told = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) if delivered.event.kind() == "operator.board_changed" => return delivered.event,
                Some(_) => continue,
                None => panic!("the stream ended"),
            }
        }
    })
    .await
    .expect("the board's change is told");
    assert!(matches!(told, Event::OperatorBoardChanged(_)));
}

#[tokio::test]
async fn a_person_moves_a_waiting_item_before_another_or_last_and_an_unknown_id_is_a_404() {
    let first = raised(1);
    let second = raised(2);
    let board = waiting(&[&first, &second], &[]).await;
    let (second_id, third_id) = ("question:second", "question:third");
    assert_eq!(board.next().await, [second_id, third_id]);

    let moved = board.rig.fleet.operator_board_moved(MoveOnOperatorBoard { id: third_id.into(), before: Some(second_id.into()) }).await.expect("moved");
    assert_eq!(moved.next.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [third_id, second_id], "before");
    let moved = board.rig.fleet.operator_board_moved(MoveOnOperatorBoard { id: third_id.into(), before: None }).await.expect("moved");
    assert_eq!(moved.next.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [second_id, third_id], "last");
    board.rig.fleet.operator_board_moved(MoveOnOperatorBoard { id: third_id.into(), before: Some(second_id.into()) }).await.expect("moved");

    for (id, before) in [("question:nobody", None), ("question:first", None), (third_id, Some("question:nobody")), (third_id, Some("question:first"))] {
        let refused = board.rig.fleet.operator_board_moved(MoveOnOperatorBoard { id: id.into(), before: before.map(str::to_string) }).await.expect_err("not waiting");
        assert_eq!(code(refused), (404, "fleet.operator_board_item_unknown".to_string()), "{id} before {before:?}");
    }
    assert_eq!(board.next().await, [third_id, second_id], "a refusal moves nothing");

    board.call.gate.add_permits(2);
    let call = board.call.clone();
    eventually(|| {
        let call = call.clone();
        async move { call.asked_of(false).len() == 2 }
    })
    .await;
    let second_call = &board.call.asked_of(false)[1];
    assert!(second_call.find("Delete the cache?") < second_call.find("Which pane first?"), "the next call is handed them in the order he left: {second_call}");
}

#[tokio::test]
async fn a_message_about_an_item_hands_the_call_its_title_and_text_and_is_kept_with_it() {
    let one = said("none", "It is waiting on the pane.");
    let two = said("none", "That one is settled.");
    let board = waiting(&[&raised(1), &raised(2)], &[&one, &two]).await;

    board.rig.fleet.operator_talked("why is this waiting?", Some("question:third")).await.expect("answered");
    board.rig.fleet.operator_talked("and this one?", Some("question:nobody")).await.expect("answered");

    let asked = board.call.asked_of(true);
    assert!(asked[0].contains("The person is talking about this item: question \"Delete the cache?\" (question:third), not yet ruled. Its words: The third words"), "{}", asked[0]);
    assert!(!asked[1].contains("talking about this item"), "an id it cannot find adds nothing");
    let talk = board.rig.fleet.operator_talk_wire().await.expect("read");
    assert_eq!(talk.lines.iter().map(|one| one.about.as_deref()).collect::<Vec<_>>(), [Some("question:third"), Some("question:third"), Some("question:nobody"), Some("question:nobody")]);
}

#[tokio::test]
async fn an_item_the_answer_names_to_be_taken_first_goes_first_and_one_it_was_not_shown_or_already_in_a_call_does_not() {
    let in_call = said("question:first", "Done.");
    let unknown = said("question:nobody", "Done.");
    let third = said("question:third", "Taking it first.");
    let board = waiting(&[&raised(1), &raised(2)], &[&in_call, &unknown, &third]).await;
    assert_eq!(board.next().await, ["question:second", "question:third"]);

    let said = board.rig.fleet.operator_talked("do the first one next", None).await.expect("answered");
    assert_eq!(said.moved, None, "it was in the call, and could not be moved");
    let said = board.rig.fleet.operator_talked("do the nobody one", None).await.expect("answered");
    assert_eq!(said.moved, None, "it was never shown");
    assert_eq!(board.next().await, ["question:second", "question:third"]);

    let said = board.rig.fleet.operator_talked("take the cache one first", None).await.expect("answered");
    assert_eq!(said.moved.as_deref(), Some("question:third"));
    assert_eq!(board.next().await, ["question:third", "question:second"]);
    let asked = board.call.asked_of(true);
    assert!(asked[0].contains("question:second") && asked[0].contains("waits its turn") && asked[0].contains("Being ruled now: question:first"), "{}", asked[0]);
}
