//! The claim, written before the code: **the Operator's own session puts cards in front of him and nothing else
//! does, his press on one reaches that session as a message, and what arrives is told to the session while it runs.**
//! A card put Later and a resting Job's item are off the page until they are due. `crates/ipc/operations/raise_card.toml`.
//!
//! **A pressed card is never lost**: until the session settles it, it is in the page's `working` or back first in
//! `turn`, with why. `crates/ipc/operations/report_card.toml`.

use std::sync::Arc;

use api::{HostedSessions, Projects, Refusal, Sessions};
use ipc::operator_talk::TalkToOperator;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorFeedKind, OperatorLater};
use ipc::{CardOption, CloseSession, Instant, NewProject, OperatorActs, OperatorCardKind, ProjectMember, RaiseCard, ReportCard, ReviseCard, SessionId, SetOperatorActs, SetWaitingFor, SettleCard, WaitingAct, WaitingActKind, WaitingInput, WaitingOption};

use super::operator_dispatches::the_dispatch;
use super::operator_session::{live, switched};
use super::peer::Placing;
use super::planted::Held;
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::operator::{keep_the_raises, Arrival, Arrived, Source};

/// The first process the stand-in starts is pid 1000, and this connection is the one it holds.
const OPERATOR_PORT: u16 = 50999;
const HOST_PORT: u16 = 12345;

fn rig() -> Rig {
    let peers = Placing::nothing();
    peers.holding(1000, OPERATOR_PORT, HOST_PORT);
    rig_placing(Some(peers))
}

/// [`rig`] on a clock the test moves.
fn rig_held() -> (Rig, Arc<Held>) {
    let peers = Placing::nothing();
    peers.holding(1000, OPERATOR_PORT, HOST_PORT);
    let clock = Arc::new(Held::started());
    let held = Arc::clone(&clock);
    let rig = rig_fitted(move |fittings| {
        fittings.peers = peers;
        fittings.clock = held;
    });
    (rig, clock)
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

async fn page(rig: &Rig) -> ipc::operator_page::OperatorPage {
    rig.fleet.operator_page_wire().await.expect("read")
}

fn question(id: &str, text: &str, options: &[&str]) -> WaitingInput {
    WaitingInput {
        approves: false,
        id: id.into(),
        text: text.into(),
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.into(), on: None }),
        options: options.iter().map(|label| WaitingOption { label: (*label).into(), description: None, picture: None }).collect(),
        steps: Vec::new(),
        artifact: None,
    }
}

/// `job`'s list, as its agent sets it whole.
async fn waiting(rig: &Rig, job: &SessionId, items: Vec<WaitingInput>) {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(job.clone()) }).await.expect("set");
}

fn the_operator() -> Option<api::Caller> {
    Some(api::Caller::at(format!("127.0.0.1:{OPERATOR_PORT}").parse().unwrap()))
}

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

/// The Operator's session, started and its process up.
async fn started(rig: &Rig) -> String {
    switched(rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = live(rig).await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    id
}

fn heard(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

/// His words to the Operator, about `about` where it names a card, with nothing sent beside them.
pub(super) fn said(words: &str, about: Option<&str>) -> TalkToOperator {
    TalkToOperator { words: words.into(), about: about.map(str::to_string), attachments: Vec::new(), mentions: Vec::new() }
}

fn option(label: &str, cost: &str) -> CardOption {
    CardOption { label: label.into(), cost: cost.into() }
}

fn card(options: Vec<CardOption>) -> RaiseCard {
    RaiseCard { kind: OperatorCardKind::Deal, title: "Start the pane Job?".into(), read: "Nothing else touches the pane.".into(), options, sessions: Vec::new(), repository: None, talk_of: None }
}

fn two() -> Vec<CardOption> {
    vec![option("Start it", "Takes a slot for about an hour"), option("Wait", "The pane waits until you say")]
}

fn press(card: &str, label: &str) -> OperatorAnswerPress {
    OperatorAnswerPress { id: card.into(), answer: OperatorAnswer::Yes, then: None, option: Some(label.into()), project: None }
}

#[tokio::test]
async fn only_the_operators_own_session_raises_revises_and_settles_a_card_and_each_card_is_one_decision() {
    let rig = rig();
    started(&rig).await;

    let refused = rig.fleet.card_raised(a_job(), card(two())).await.expect_err("a Job is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    let refused = rig.fleet.card_raised(None, card(two())).await.expect_err("nobody is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");

    let five: Vec<CardOption> = (1..=5).map(|n| option(&format!("Option {n}"), "A cost")).collect();
    for options in [vec![option("Only one", "A cost")], five, vec![option("A", "x"), option("A", "y")], vec![option("A", "x"), option("B", " ")]] {
        let refused = rig.fleet.card_raised(the_operator(), card(options)).await.expect_err("not a card");
        assert_eq!(code(refused).1, "fleet.card_refused");
    }
    assert!(page(&rig).await.turn.is_empty(), "nothing was kept");

    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    let shown = page(&rig).await.turn.remove(0);
    assert_eq!(shown.id, raised);
    assert_eq!(shown.card.as_ref().map(|one| one.kind), Some(OperatorCardKind::Deal));
    assert_eq!(shown.item.text, "Start the pane Job?");
    assert_eq!(shown.item.options.iter().map(|one| (one.label.as_str(), one.description.as_deref())).collect::<Vec<_>>(), [("Start it", Some("Takes a slot for about an hour")), ("Wait", Some("The pane waits until you say"))]);
    assert_eq!(shown.read.as_ref().map(|one| (one.read.as_str(), one.pick.as_str())), Some(("Nothing else touches the pane.", "Start it")), "the first is the pick");

    let revise = ReviseCard { card: raised.clone(), read: Some("A Job touches it too.".into()), options: Some(vec![option("Wait", "Nothing runs"), option("Start it", "Clashes")]) };
    let refused = rig.fleet.card_revised(a_job(), revise.clone()).await.expect_err("not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    rig.fleet.card_revised(the_operator(), revise).await.expect("revised");
    let shown = page(&rig).await.turn.remove(0);
    assert_eq!(shown.read.as_ref().map(|one| (one.read.as_str(), one.pick.as_str())), Some(("A Job touches it too.", "Wait")));

    let refused = rig.fleet.card_settled(a_job(), SettleCard { card: raised.clone(), said: "Done.".into() }).await.expect_err("not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    rig.fleet.card_settled(the_operator(), SettleCard { card: raised.clone(), said: "Started it myself.".into() }).await.expect("settled");
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "settled: {:?}", after.turn);
    let feed = after.feed.iter().find(|one| one.kind == OperatorFeedKind::Settled).expect("the feed says it");
    assert_eq!((feed.title.as_str(), feed.said.as_deref()), ("Start the pane Job?", Some("Started it myself.")));
    let again = rig.fleet.card_settled(the_operator(), SettleCard { card: raised, said: "Again.".into() }).await.expect_err("settled already");
    assert_eq!(code(again).1, "fleet.card_unknown");
}

#[tokio::test]
async fn a_press_picks_the_card_and_tells_the_operator_which_option_and_is_refused_with_no_session() {
    let rig = rig();
    let id = started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;

    let refused = rig.fleet.operator_answered(press(&raised, "Somewhere else")).await.expect_err("not an option");
    assert_eq!(code(refused).1, "fleet.operator_answer_not_offered");
    assert_eq!(page(&rig).await.turn.len(), 1, "nothing changed");

    let answered = rig.fleet.operator_answered(press(&raised, "Wait")).await.expect("pressed");
    assert!(answered.page.turn.is_empty(), "off the front");
    assert_eq!(answered.page.working.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [raised.as_str()], "worked on");
    eventually(|| async { heard(&rig, "He chose") >= 1 }).await;
    assert_eq!(heard(&rig, "Wait"), 1, "the Operator is told which option");
    assert_eq!(heard(&rig, &raised), 1, "and which card");

    let second = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    rig.fleet.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(&id) }).await.expect("closed");
    let refused = rig.fleet.operator_answered(press(&second, "Start it")).await.expect_err("no session");
    assert_eq!(code(refused).1, "fleet.operator_session_not_running");
    assert_eq!(page(&rig).await.turn.len(), 1, "the card stands: nothing was lost");
}

#[tokio::test]
async fn later_sends_a_card_off_the_page_and_tells_the_operator_when_and_his_words_about_a_card_arrive_tagged() {
    let rig = rig();
    let id = started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;

    let later = rig.fleet.operator_turn_later(OperatorLater { id: raised.clone(), until: Some(Instant::carried("2099-01-01T00:00:00.000Z")) }).await.expect("later");
    assert!(later.turn.is_empty(), "a card put Later is off the page until it is due: {:?}", later.turn);
    eventually(|| async { heard(&rig, "back with Later") >= 1 }).await;
    assert_eq!(heard(&rig, "2099-01-01"), 1, "told until when");

    rig.fleet.operator_talked(said("Why that one?", Some(&raised))).await.expect("said");
    eventually(|| async { heard(&rig, "Why that one?") >= 1 }).await;
    assert_eq!(heard(&rig, &format!("About card {raised}")), 1, "tagged with the card");
    rig.fleet.operator_talked(said("Anything else?", None)).await.expect("said");
    eventually(|| async { heard(&rig, "Anything else?") >= 1 }).await;

    rig.fleet.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(&id) }).await.expect("closed");
    let refused: Refusal = rig.fleet.operator_talked(said("Anyone?", Some(&raised))).await.expect_err("nothing would hear it");
    assert_eq!(code(refused).1, "fleet.operator_session_not_running");
}

#[tokio::test]
async fn the_operators_acts_are_a_value_per_project_that_is_unset_until_he_says() {
    let rig = rig();
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Shop".into(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();

    let listed = rig.fleet.operator_acts_listed().await.expect("listed");
    let one = listed.projects.iter().find(|one| one.project == project).expect("listed");
    assert_eq!((one.name.as_str(), one.acts), ("Shop", None), "unset is a state of its own");

    let set = rig.fleet.operator_acts_set(SetOperatorActs { project: project.clone(), acts: Some(OperatorActs::AskBeforeStarting) }).await.expect("set");
    assert_eq!(set.projects.iter().find(|one| one.project == project).and_then(|one| one.acts), Some(OperatorActs::AskBeforeStarting));
    assert_eq!(page(&rig).await.operator_acts.iter().find(|one| one.project == project).and_then(|one| one.acts), Some(OperatorActs::AskBeforeStarting), "the page serves it");

    let unset = rig.fleet.operator_acts_set(SetOperatorActs { project: project.clone(), acts: None }).await.expect("unset");
    assert_eq!(unset.projects.iter().find(|one| one.project == project).and_then(|one| one.acts), None);

    let unknown = rig.fleet.operator_acts_set(SetOperatorActs { project: "no-such".into(), acts: Some(OperatorActs::OnlyWhenTold) }).await.expect_err("no such project");
    assert_eq!(unknown.status(), 404);
}

fn arrival(id: &str, subject: &str) -> Arrival {
    Arrival {
        id: id.into(),
        arrived: Arrived::Report,
        source: Source::Agent,
        session_id: String::new(),
        manifest_id: String::new(),
        subject: subject.into(),
        words: "The scrollback is lost on a resize.".into(),
    }
}

#[tokio::test]
async fn while_the_operators_session_runs_what_arrives_is_told_to_it_once_and_fleet_rules_nothing() {
    let rig = rig();
    let id = started(&rig).await;

    assert!(rig.fleet.hand_once(arrival("report:1", "Scrollback is lost")).await, "handed");
    assert!(rig.fleet.hand_once(arrival("report:1", "Scrollback is lost")).await, "handed again");
    eventually(|| async { heard(&rig, "Scrollback is lost") >= 1 }).await;
    assert_eq!(heard(&rig, "Scrollback is lost"), 1, "told once");
    assert!(page(&rig).await.feed.iter().any(|one| one.kind == OperatorFeedKind::Told && one.title == "Scrollback is lost"), "and on the feed");

    // His dispatch is the session's to start, so nothing starts and nothing is held in Fleet's own hold.
    let mut plain = the_dispatch(&rig);
    plain.then.clear();
    let held = Arc::clone(&rig.fleet).dispatch_job(plain).await.expect("dispatched");
    assert!(held.session.is_none() && held.operator.is_some(), "{held:?}");
    eventually(|| async { heard(&rig, "Read crates/reader") + heard(&rig, "Scrollback is lost on redraw") >= 1 }).await;

    // One carrying sends to follow starts as he asked: the sends need the Job they were set up for.
    let followed = Arc::clone(&rig.fleet).dispatch_job(the_dispatch(&rig)).await.expect("dispatched");
    assert!(followed.session.is_some(), "starts with its sends: {followed:?}");

    // The session gone, the dispatch starts as he asked.
    rig.fleet.closed_and_parked(CloseSession { session_id: ipc::SessionId::carried(&id) }).await.expect("closed");
    let started = Arc::clone(&rig.fleet).dispatch_job(the_dispatch(&rig)).await.expect("dispatched");
    assert!(started.session.is_some(), "fails open: {started:?}");
}

#[tokio::test]
async fn while_the_operators_session_runs_no_jobs_item_is_queued_into_your_turn() {
    let rig = rig();
    started(&rig).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("a", "First?", &["Yes", "No"])]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert!(page(&rig).await.turn.is_empty(), "the session reads what waits on Jobs itself");
}

fn report(card: &str, step: Option<&str>, failed: Option<&str>) -> ReportCard {
    ReportCard { card: card.into(), step: step.map(str::to_string), failed: failed.map(str::to_string) }
}

fn titled(title: &str) -> RaiseCard {
    RaiseCard { title: title.into(), ..card(two()) }
}

#[tokio::test]
async fn a_press_moves_the_card_from_turn_to_working_and_the_next_card_comes_to_the_front() {
    let rig = rig();
    started(&rig).await;
    let first = rig.fleet.card_raised(the_operator(), titled("Start the pane Job?")).await.expect("raised").card;
    let second = rig.fleet.card_raised(the_operator(), titled("Merge #12?")).await.expect("raised").card;
    assert_eq!(page(&rig).await.turn.first().map(|one| one.id.clone()), Some(first.clone()));

    let answered = rig.fleet.operator_answered(press(&first, "Start it")).await.expect("pressed");
    assert_eq!(answered.page.turn.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [second.as_str()], "the next card is in front at once");
    let working = answered.page.working.first().expect("working");
    assert_eq!(working.id, first);
    let card = working.card.as_ref().expect("a card");
    assert_eq!(card.picked.as_ref().map(|one| one.option.as_str()), Some("Start it"));
    assert!(card.steps.is_empty() && card.returned.is_none(), "{card:?}");

    let again = rig.fleet.operator_answered(press(&first, "Wait")).await.expect_err("picked already");
    assert_eq!(code(again).1, "fleet.operator_answer_not_offered");

    rig.fleet.operator_answered(press(&second, "Wait")).await.expect("pressed");
    let after = page(&rig).await;
    assert!(after.turn.is_empty());
    assert_eq!(after.working.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [second.as_str(), first.as_str()], "newest pick first");
}

#[tokio::test]
async fn report_card_step_appends_to_the_working_card() {
    let rig = rig();
    started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    rig.fleet.operator_answered(press(&raised, "Start it")).await.expect("pressed");

    let reported = rig.fleet.card_reported(the_operator(), report(&raised, Some("Started Build the pane"), None)).await.expect("reported");
    assert_eq!(reported.card, raised);
    rig.fleet.card_reported(the_operator(), report(&raised, Some("Opened #41"), None)).await.expect("reported");
    let after = page(&rig).await;
    let steps = &after.working.first().and_then(|one| one.card.as_ref()).expect("working").steps;
    assert_eq!(steps.iter().map(|one| one.said.as_str()).collect::<Vec<_>>(), ["Started Build the pane", "Opened #41"]);
    assert!(after.turn.is_empty(), "still worked on");
}

#[tokio::test]
async fn report_card_failed_returns_the_card_to_the_front_of_turn_with_why_until_he_presses_again() {
    let rig = rig();
    started(&rig).await;
    let failing = rig.fleet.card_raised(the_operator(), titled("Start the pane Job?")).await.expect("raised").card;
    let other = rig.fleet.card_raised(the_operator(), titled("Merge #12?")).await.expect("raised").card;
    rig.fleet.operator_answered(press(&failing, "Start it")).await.expect("pressed");

    rig.fleet.card_reported(the_operator(), report(&failing, None, Some("No slot is free"))).await.expect("reported");
    let after = page(&rig).await;
    assert!(after.working.is_empty(), "out of working");
    assert_eq!(after.turn.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [failing.as_str(), other.as_str()], "ahead of every other card");
    let returned = after.turn[0].card.as_ref().and_then(|one| one.returned.as_ref()).expect("returned");
    assert_eq!(returned.why, "No slot is free");

    let refused = rig.fleet.card_reported(the_operator(), report(&failing, Some("Started"), None)).await.expect_err("not worked on now");
    assert_eq!(code(refused).1, "fleet.card_refused");

    let answered = rig.fleet.operator_answered(press(&failing, "Wait")).await.expect("pressed again");
    let picked = answered.page.working.first().and_then(|one| one.card.as_ref()).expect("working");
    assert_eq!((picked.picked.as_ref().map(|one| one.option.as_str()), picked.returned.is_none()), (Some("Wait"), true), "picked anew");
}

#[tokio::test]
async fn a_picked_card_unsettled_for_15_minutes_returns_to_the_front_and_the_operator_is_told() {
    let (rig, clock) = rig_held();
    started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    rig.fleet.operator_answered(press(&raised, "Start it")).await.expect("pressed");

    clock.on(14 * 60);
    rig.fleet.operator_session_kept().await;
    assert_eq!(page(&rig).await.working.len(), 1, "not yet");

    clock.on(2 * 60);
    rig.fleet.operator_session_kept().await;
    let after = page(&rig).await;
    assert!(after.working.is_empty());
    let back = after.turn.first().expect("back in front");
    assert_eq!(back.id, raised);
    assert_eq!(back.card.as_ref().and_then(|one| one.returned.as_ref()).map(|one| one.why.as_str()), Some("Nothing settled in 15 min"));
    eventually(|| async { heard(&rig, "is back in front of him") >= 1 }).await;
    assert_eq!(heard(&rig, &format!("Card {raised}")), 1, "naming the card");

    rig.fleet.operator_session_kept().await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(heard(&rig, "is back in front of him"), 1, "told once");
}

#[tokio::test]
async fn settle_card_on_a_picked_card_clears_working_and_the_feed_says_it() {
    let rig = rig();
    started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    rig.fleet.operator_answered(press(&raised, "Start it")).await.expect("pressed");
    rig.fleet.card_settled(the_operator(), SettleCard { card: raised.clone(), said: "Started Build the pane.".into() }).await.expect("settled");
    let after = page(&rig).await;
    assert!(after.working.is_empty() && after.turn.is_empty(), "{after:?}");
    assert!(after.feed.iter().any(|one| one.kind == OperatorFeedKind::Settled && one.said.as_deref() == Some("Started Build the pane.")));
}

#[tokio::test]
async fn only_the_operator_reports_on_a_card_and_a_report_is_one_of_step_or_failed_on_a_picked_card() {
    let rig = rig();
    started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;

    let refused = rig.fleet.card_reported(a_job(), report(&raised, Some("Started"), None)).await.expect_err("a Job is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    let refused = rig.fleet.card_reported(the_operator(), report(&raised, Some("Started"), None)).await.expect_err("not picked");
    assert_eq!(code(refused).1, "fleet.card_refused");
    let refused = rig.fleet.card_reported(the_operator(), report("turn:9999", Some("Started"), None)).await.expect_err("no such card");
    assert_eq!(code(refused), (404, "fleet.card_unknown".to_string()));

    rig.fleet.operator_answered(press(&raised, "Start it")).await.expect("pressed");
    for (step, failed) in [(Some("Started"), Some("No slot")), (None, None), (Some("  "), None)] {
        let refused = rig.fleet.card_reported(the_operator(), report(&raised, step, failed)).await.expect_err("one of the two");
        assert_eq!(code(refused).1, "fleet.card_refused");
    }
    let after = page(&rig).await;
    assert!(after.working[0].card.as_ref().is_some_and(|one| one.steps.is_empty() && one.returned.is_none()), "nothing changed");
}

// Proves: a picture and a tag he sends to the Operator reach its session and stay on his row, about a card or none.
#[tokio::test]
async fn a_picture_and_a_tag_he_sends_the_operator_reach_its_session_about_a_card_or_none() {
    let rig = rig();
    let id = started(&rig).await;
    let raised = rig.fleet.card_raised(the_operator(), card(two())).await.expect("raised").card;
    let with_files = |words: &str, about: Option<&str>| TalkToOperator {
        attachments: vec![ipc::SessionUpload { name: "shot.png".into(), media_type: "image/png".into(), data: "aGVsbG8=".into() }],
        mentions: vec![ipc::SessionTag { kind: ipc::TagKind::Branch, id: "armada/other".into(), title: "armada/other".into(), url: None }],
        ..said(words, about)
    };

    for (words, about) in [("This toast is wrong", None), ("Like this one", Some(raised.as_str()))] {
        rig.fleet.operator_talked(with_files(words, about)).await.expect("said");
        eventually(|| async { heard(&rig, words) >= 1 }).await;
        let rows = rig.rows(&SessionId::carried(&id)).await;
        let (files, tags) = rows
            .iter()
            .rev()
            .find_map(|row| match row {
                ipc::SessionRow::Message { from: ipc::SessionVoice::You, files, tags, .. } => Some((files.clone(), tags.clone())),
                _ => None,
            })
            .expect("his message is on the Operator's thread");
        assert_eq!(files.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["shot.png"], "the picture rides with {words:?}");
        assert_eq!(tags.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["armada/other"], "the tag rides with {words:?}");
    }
    assert_eq!(heard(&rig, &format!("About card {raised}")), 1, "the talk about the card still names it");
}
