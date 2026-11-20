//! A Job's offer for the preview is a Ready to try card in its thread: what the offer gave it, each of his
//! answers and what follows, and each state it settles in. `crates/ipc/operations/answer_ready_to_try.toml`.

use std::fs;
use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    AnswerReadyToTry, AnswerSketch, ArtifactForm, Instant, JobThreadRead, NoteVerdict, OfferApproved, OfferPreview, PreviewOffered, ReadyAnswer,
    ReadyState, SessionId, ShowSketch, SketchHow, ThreadItem, ThreadReadyToTry, WithdrawPreview,
};

use super::build_notices::{a_rig, Rig};
use super::preview_offering::{a_job_offering, a_previewable_repository, base, branch_with, built_on_main_holding, checked, code, holding, step};

const BRANCH: &str = "armada/one";

/// An offer of the Job's own branch with a look, and whatever else is given.
fn offered(session: &str) -> OfferPreview {
    OfferPreview {
        session_id: Some(SessionId::carried(session)),
        branch: None,
        about: Some("Keeps one thing".into()),
        look: Some("The card sits under the reply".into()),
        steps: Some(vec![step("Open the Job", None)]),
        approved: None,
        pictures: None,
        words: None,
    }
}

async fn offer(rig: &Rig, offer: OfferPreview) -> Result<PreviewOffered, Refusal> {
    let offered = Arc::clone(&rig.fleet).offer_preview(None, offer).await;
    checked(rig).await;
    offered
}

async fn cards(rig: &Rig, session: &str) -> Vec<ThreadReadyToTry> {
    rig.fleet.ready_cards_of(session).await
}

async fn card(rig: &Rig, session: &str, tip: &str) -> ThreadReadyToTry {
    cards(rig, session).await.into_iter().find(|one| one.tip == tip).expect("a card for that tip")
}

async fn answer(rig: &Rig, session: &str, tip: &str, answer: ReadyAnswer) -> Result<ThreadReadyToTry, Refusal> {
    let asked = AnswerReadyToTry { session_id: SessionId::carried(session), branch: BRANCH.into(), tip: tip.into(), answer };
    api::Commands::answer_ready_to_try(Arc::clone(&rig.fleet), asked).await
}

/// A Job holding the branch, offered once with a look and steps, and its tip.
async fn an_offered_job(rig: &Rig) -> (String, String) {
    a_previewable_repository(rig);
    let (session, tip) = a_job_offering(rig, BRANCH, "crates/x/one.rs", "1").await;
    offer(rig, offered(&session)).await.expect("offered");
    (session, tip)
}

/// An instant `seconds` from Fleet's now.
fn from_now(rig: &Rig, seconds: i64) -> Instant {
    let now = rig.fleet.now().epoch_millis().expect("a clock");
    Instant::carried(&crate::clock::rfc3339_utc(now + seconds * 1000))
}

#[tokio::test]
async fn an_offer_is_an_open_card_in_its_jobs_thread_at_the_offer() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);

    let one = card(&rig, &session, &tip).await;
    assert_eq!(one.id, format!("try:{BRANCH}@{tip}"));
    assert_eq!((one.state, one.in_build, one.verdict), (ReadyState::Open, true, None));
    assert_eq!((one.about.as_str(), one.look.as_deref(), one.steps.len()), ("Keeps one thing", Some("The card sits under the reply"), 1));
    assert_eq!(one.at, one.offered_at, "drawn at the offer");

    let thread = Arc::clone(&rig.fleet)
        .job_thread_served(JobThreadRead { session_id: SessionId::carried(session.as_str()), before: None, limit: None, upto: None })
        .await
        .expect("the thread");
    let drawn: Vec<&ThreadReadyToTry> = thread.items.iter().filter_map(|item| match item {
        ThreadItem::ReadyToTry(card) => Some(card),
        _ => None,
    }).collect();
    assert_eq!(drawn, [&one], "the thread draws the card");
}

#[tokio::test]
async fn a_card_whose_tip_is_on_main_under_the_running_build_is_in_the_build() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    assert!(!card(&rig, &session, &tip).await.in_build, "no preview runs yet");

    built_on_main_holding(&rig, &tip);

    let one = card(&rig, &session, &tip).await;
    assert!(one.in_build, "the build stands on a main that holds the tip, though no list names the branch");
}

#[tokio::test]
async fn a_new_build_moves_the_cards_of_a_job_whose_branch_is_on_the_main_it_stands_on() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    rig.fleet.build_notes_published().await;

    built_on_main_holding(&rig, &tip);

    let moved = rig.fleet.build_notes_published().await;
    assert!(moved.contains(&session), "no note lists the branch, yet its card now reads in_build: {moved:?}");
}

#[tokio::test]
async fn an_offer_carries_his_words_its_pictures_and_what_he_approved_and_a_re_offer_keeps_them() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, BRANCH, "crates/x/one.rs", "1").await;
    let cwd = rig.fleet.store().lock().await.session(&session).unwrap().expect("the Job").cwd;
    fs::create_dir_all(std::path::Path::new(&cwd).join("shots")).unwrap();
    fs::write(std::path::Path::new(&cwd).join("shots/built.png"), b"\x89PNG").unwrap();
    fs::write(std::path::Path::new(&cwd).join("shots/notes.txt"), "words").unwrap();
    let id = SessionId::carried(session.as_str());
    let drawing: ShowSketch = ipc::decode(
        "a sketch",
        format!(r#"{{"title":"Card","question":"Does this read right?","session_id":"{session}","scene":{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Card"}}],"edges":[]}}}}"#).as_bytes(),
    )
    .unwrap();
    let shown = rig.fleet.show_sketch(None, drawing).await.expect("drawn");
    let drawn = shown.attachments.iter().find(|one| one.target == "sketch:Card").and_then(|one| one.detail.get("drawn")).cloned().expect("drawn");
    Arc::clone(&rig.fleet)
        .answer_sketch(AnswerSketch { session_id: id.clone(), sketch: "sketch:Card".into(), drawn, how: SketchHow::Approved, words: None })
        .await
        .expect("approved");

    let with = |approved: Option<OfferApproved>, pictures: Option<Vec<String>>, words: Option<&str>| OfferPreview {
        approved,
        pictures,
        words: words.map(str::to_string),
        ..offered(&session)
    };
    let named = |artifact: &str, revision| Some(OfferApproved { artifact: artifact.into(), revision });
    assert_eq!(code(offer(&rig, with(named("sketch:Nothing", None), None, None)).await), "fleet.artifact_not_held");
    assert_eq!(code(offer(&rig, with(named("Card", Some(7)), None, None)).await), "fleet.preview_approved_revision_unknown");
    assert_eq!(code(offer(&rig, with(None, Some(vec!["shots/notes.txt".into()]), None)).await), "fleet.preview_pictures_invalid", "not a picture");
    assert_eq!(code(offer(&rig, with(None, Some(vec!["shots/gone.png".into()]), None)).await), "fleet.preview_pictures_invalid", "no such file");
    assert_eq!(code(offer(&rig, with(None, Some(vec!["shots/built.png".into(); 5]), None)).await), "fleet.preview_pictures_invalid", "five is one over");
    assert_eq!(code(offer(&rig, with(None, None, Some(&"x".repeat(2001)))).await), "fleet.preview_words_invalid");
    assert!(cards(&rig, &session).await.is_empty(), "a refused offer raises no card");

    let tip = offer(&rig, with(named("Card", None), Some(vec!["shots/built.png".into()]), Some("Put the shots first"))).await.expect("offered").tip;
    let one = card(&rig, &session, &tip).await;
    let approved = one.approved.clone().expect("what he approved");
    assert_eq!((approved.artifact.as_str(), approved.title.as_str(), approved.form, approved.revision), ("sketch:Card", "Card", ArtifactForm::Sketch, 1), "named by its title");
    assert!(approved.approved_at.is_some(), "when he approved it");
    let picture = format!("{cwd}/shots/built.png");
    assert_eq!((one.pictures.clone(), one.words.as_deref()), (vec![picture.clone()], Some("Put the shots first")));
    let read = rig.fleet.repository_file_served(id.clone(), picture).await.expect("the picture reads");
    assert!(matches!(read, ipc::RepositoryFile::Picture { .. }));

    let newer = branch_with(&rig, BRANCH, BRANCH, "crates/x/more.rs", "2");
    offer(&rig, offered(&session)).await.expect("offered again");
    let again = card(&rig, &session, &newer).await;
    assert_eq!((again.approved, again.pictures, again.words), (one.approved, one.pictures, one.words), "a re-offer that leaves them out keeps them");
    offer(&rig, with(None, Some(Vec::new()), Some(""))).await.expect("cleared");
    let cleared = card(&rig, &session, &newer).await;
    assert_eq!((cleared.pictures.len(), cleared.words), (0, None), "an empty list and empty words clear them");
}

#[tokio::test]
async fn looks_good_is_filed_once_as_a_decision_of_the_features_the_job_changed() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);
    let link = store::KeptFeatureLink {
        feature_id: "feature-1".into(),
        kind: store::feature_link_kind::JOB.into(),
        target: session.clone(),
        job: session.clone(),
        title: String::new(),
        gist: String::new(),
        gist_by: String::new(),
        state: String::new(),
        via: "trailer".into(),
        at: "2026-11-18T09:00:00.000Z".into(),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).unwrap();

    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");
    let _ = answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await;

    let filed = rig.fleet.store().lock().await.feature_decisions("feature-1").unwrap();
    assert_eq!(filed.iter().map(|one| (one.question.as_str(), one.choice.as_str(), one.job.as_str())).collect::<Vec<_>>(), [("Keeps one thing", "Looks good", session.as_str())]);
}

#[tokio::test]
async fn looks_good_keeps_the_verdict_tells_the_job_to_land_and_answers_the_card() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);

    let answered = answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");

    assert_eq!((answered.state, answered.verdict), (ReadyState::Answered, Some(NoteVerdict::Good)));
    let note = rig.fleet.build_notes_read().await.notes.into_iter().next().expect("the note");
    assert_eq!((note.looked_at, note.verdict), (true, Some(NoteVerdict::Good)), "the note says what the card says");
    let told = rig.thread_of(&session).await;
    assert_eq!(
        told.last().map(String::as_str),
        Some(format!("The owner tried {BRANCH} at {} in his preview and says it looks good: \"Keeps one thing\". Land it.", &tip[..7]).as_str())
    );
    let again = answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await;
    assert_eq!((code(again), rig.thread_of(&session).await.len()), ("fleet.ready_to_try_settled".to_string(), told.len()), "an answered card takes no other");
}

/// Two groups gated `you_approved` around one that is not, so the first open gate is told from the next.
async fn planned_on_his_approve(rig: &Rig, session: &str) {
    let gated = |title: &str, gate| ipc::PlanGroupInput {
        gate,
        ..super::session_plan::group(None, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))])
    };
    let groups = vec![gated("Walk", ipc::PlanGate::YouApproved), gated("Build", ipc::PlanGate::None), gated("Sign off", ipc::PlanGate::YouApproved)];
    rig.fleet.record_plan(None, super::session_plan::plan(&SessionId::carried(session), groups), false).await.expect("recorded");
    // The clock reads milliseconds: let the answer land after the plan.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
}

async fn closed_gates(rig: &Rig, session: &str) -> Vec<Option<String>> {
    let record = Arc::clone(&rig.fleet).get_session(SessionId::carried(session)).await.expect("the Job").session;
    record.plan.expect("a plan").groups.iter().map(|group| group.gate_closed.as_ref().map(|one| one.fact.clone())).collect()
}

#[tokio::test]
async fn looks_good_on_a_card_closes_the_first_open_you_approved_gate_once_and_the_next_card_the_next() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);
    planned_on_his_approve(&rig, &session).await;
    assert!(rig.fleet.gates_kept(&session).await.is_empty(), "nothing is approved yet");

    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");

    // The answer sweeps the gates itself: no row of the Job moved for the keeper to hear.
    let facts = closed_gates(&rig, &session).await;
    assert_eq!(facts.iter().map(Option::is_some).collect::<Vec<_>>(), [true, false, false], "the first open you_approved gate, in plan order, and no other");
    assert!(facts[0].as_deref().is_some_and(|fact| fact.contains("Keeps one thing")), "{facts:?}");
    assert!(rig.fleet.gates_kept(&session).await.is_empty(), "one answer closes one gate, once");

    let newer = branch_with(&rig, BRANCH, BRANCH, "crates/x/more.rs", "2");
    offer(&rig, offered(&session)).await.expect("offered again");
    holding(&rig, &[(BRANCH, &newer)]);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    answer(&rig, &session, &newer, ReadyAnswer::LooksGood).await.expect("answered");
    let next = closed_gates(&rig, &session).await;
    assert_eq!(next.iter().map(Option::is_some).collect::<Vec<_>>(), [true, false, true], "the next card closes the next gate");
}

#[tokio::test]
async fn needs_rework_and_a_reminder_on_a_card_close_no_you_approved_gate() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);
    planned_on_his_approve(&rig, &session).await;

    answer(&rig, &session, &tip, ReadyAnswer::Remind { until: from_now(&rig, 3600) }).await.expect("later");
    answer(&rig, &session, &tip, ReadyAnswer::NeedsRework { words: "Move the card".into() }).await.expect("rework");

    assert!(rig.fleet.gates_kept(&session).await.is_empty());
    assert_eq!(closed_gates(&rig, &session).await, [None, None, None]);
}

#[tokio::test]
async fn a_looks_good_given_before_the_plan_closes_no_you_approved_gate() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    holding(&rig, &[(BRANCH, &tip)]);
    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    planned_on_his_approve(&rig, &session).await;

    assert!(rig.fleet.gates_kept(&session).await.is_empty(), "what he approved before there was a plan is not its gate's");
}

#[tokio::test]
async fn needs_rework_sends_his_words_and_keeps_the_verdict_and_empty_words_change_nothing() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    let before = rig.thread_of(&session).await.len();

    let empty = answer(&rig, &session, &tip, ReadyAnswer::NeedsRework { words: " \n".into() }).await;
    assert_eq!(code(empty), "fleet.build_note_words_empty");
    assert_eq!((card(&rig, &session, &tip).await.state, rig.thread_of(&session).await.len()), (ReadyState::Open, before), "nothing changed");

    let answered = answer(&rig, &session, &tip, ReadyAnswer::NeedsRework { words: "The pictures are too small".into() }).await.expect("answered");

    assert_eq!((answered.state, answered.verdict), (ReadyState::Answered, Some(NoteVerdict::NeedsWork)));
    assert_eq!(
        rig.thread_of(&session).await.last().map(String::as_str),
        Some("On your preview note \"Keeps one thing\": The pictures are too small"),
        "his words reach the Job as send_build_note_feedback sends them"
    );
}

#[tokio::test]
async fn remind_puts_the_card_off_until_its_time_and_it_comes_back_drawn_then() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;

    assert_eq!(code(answer(&rig, &session, &tip, ReadyAnswer::Remind { until: from_now(&rig, -60) }).await), "fleet.ready_to_try_remind_past");
    assert_eq!(code(answer(&rig, &session, &tip, ReadyAnswer::Remind { until: Instant::carried("tomorrow") }).await), "fleet.ready_to_try_remind_past");

    let until = from_now(&rig, 3600);
    let later = answer(&rig, &session, &tip, ReadyAnswer::Remind { until: until.clone() }).await.expect("put off");
    assert_eq!((later.state, later.later_until.as_ref(), later.verdict), (ReadyState::Later, Some(&until), None));
    rig.fleet.ready_cards_come_back().await;
    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Later, "not before its time");

    // Its time comes: the reminder is kept at an instant that has passed.
    let came = from_now(&rig, -1);
    rig.fleet.store().lock().await.keep_build_note_later(BRANCH, &tip, came.as_str()).unwrap();
    rig.fleet.ready_cards_come_back().await;

    let back = card(&rig, &session, &tip).await;
    assert_eq!((back.state, back.later_until, &back.at), (ReadyState::Open, None, &came), "open again, drawn when it came back");
    assert!(back.at > back.offered_at);
    let looked = answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("a card that came back is answered");
    assert_eq!((looked.state, &looked.at), (ReadyState::Answered, &came), "it stays where it came back");
}

#[tokio::test]
async fn a_withdrawn_offer_withdraws_its_card_and_an_offer_made_again_opens_it() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;

    Arc::clone(&rig.fleet).withdraw_preview(None, WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: None }).await.expect("withdrawn");

    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Withdrawn);
    let refused = answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect_err("withdrawn");
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, "fleet.ready_to_try_settled"));

    offer(&rig, offered(&session)).await.expect("offered again");
    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Open, "the same tip offered again stands");
}

#[tokio::test]
async fn a_new_tip_replaces_the_card_before_it_and_an_unknown_card_is_not_found() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    let newer = branch_with(&rig, BRANCH, BRANCH, "crates/x/more.rs", "2");

    offer(&rig, offered(&session)).await.expect("offered again");

    let states: Vec<(String, ReadyState)> = cards(&rig, &session).await.into_iter().map(|one| (one.tip, one.state)).collect();
    assert_eq!(states, [(tip.clone(), ReadyState::Replaced), (newer.clone(), ReadyState::Open)]);
    assert_eq!(code(answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await), "fleet.ready_to_try_settled");
    let unknown = answer(&rig, &session, &"0".repeat(40), ReadyAnswer::LooksGood).await.expect_err("no such card");
    assert_eq!((unknown.status(), unknown.error().code.as_str()), (404, "fleet.ready_to_try_unknown"));
    let elsewhere = answer(&rig, &rig.a_hosted_session().await, &newer, ReadyAnswer::LooksGood).await.expect_err("another Job's thread");
    assert_eq!(elsewhere.status(), 404, "a card is answered from the thread that holds it");
}

#[tokio::test]
async fn a_mark_on_the_build_note_answers_its_card() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    let mark = ipc::MarkBuildNoteLookedAt { branch: BRANCH.into(), tip: tip.clone(), verdict: Some(NoteVerdict::Good) };

    api::Commands::mark_build_note_looked_at(Arc::clone(&rig.fleet), mark).await.expect("marked");

    let one = card(&rig, &session, &tip).await;
    assert_eq!((one.state, one.verdict), (ReadyState::Answered, Some(NoteVerdict::Good)));
}

/// The Job's Needs you items for its cards.
async fn tries(rig: &Rig, session: &str) -> Vec<ipc::WaitingItem> {
    let items = rig.fleet.waiting_now(session).await.expect("the Job's list");
    items.into_iter().filter(|one| one.source == ipc::WaitingSource::ReadyToTry).collect()
}

#[tokio::test]
async fn an_open_card_waits_in_needs_you_until_answered_and_comes_back_with_its_reminder() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    let id = format!("try:{session}");

    let raised = tries(&rig, &session).await;
    let [item] = raised.as_slice() else { panic!("one item for the Job: {raised:?}") };
    assert_eq!((item.id.as_str(), item.text.as_str(), item.artifact.as_deref()), (id.as_str(), "Ready to try", Some(id.as_str())));
    let said = ipc::AnswerWaiting { session_id: SessionId::carried(session.as_str()), item_id: id.clone(), choice: None, text: Some("Fine".into()), mode: None, fixed: None };
    let refused = Arc::clone(&rig.fleet).settle_waiting(said).await;
    assert_eq!(code(refused), "fleet.session_waiting_empty", "answered on its card, not here");

    answer(&rig, &session, &tip, ReadyAnswer::Remind { until: from_now(&rig, 3600) }).await.expect("put off");
    assert!(tries(&rig, &session).await.is_empty(), "a reminder holds it");
    rig.fleet.store().lock().await.keep_build_note_later(BRANCH, &tip, from_now(&rig, -1).as_str()).unwrap();
    rig.fleet.ready_cards_come_back().await;
    assert_eq!(tries(&rig, &session).await.len(), 1, "back when the reminder comes due");

    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");
    assert!(tries(&rig, &session).await.is_empty(), "an answer takes it off");
}

#[tokio::test]
async fn a_withdrawn_or_replaced_card_waits_on_nobody() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    let newer = branch_with(&rig, BRANCH, BRANCH, "crates/x/more.rs", "2");
    offer(&rig, offered(&session)).await.expect("offered again");

    let ids: Vec<String> = tries(&rig, &session).await.into_iter().map(|one| one.id).collect();
    assert_eq!(ids, [format!("try:{session}")], "the Job's one item stands for the newer card {newer}; the replaced card {tip} waits on nobody");

    Arc::clone(&rig.fleet).withdraw_preview(None, WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: None }).await.expect("withdrawn");
    assert!(tries(&rig, &session).await.is_empty());
}

#[tokio::test]
async fn two_open_cards_of_one_job_raise_one_item_and_answering_both_leaves_none() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    rig.holds_branch(&session, "armada/two").await;
    let second = branch_with(&rig, "armada/two", &base(&rig), "crates/x/two.rs", "2");
    offer(&rig, OfferPreview { branch: Some("armada/two".into()), ..offered(&session) }).await.expect("offered the second");
    assert_eq!(cards(&rig, &session).await.iter().filter(|one| one.state == ReadyState::Open).count(), 2, "two open cards");

    let raised = tries(&rig, &session).await;
    let [item] = raised.as_slice() else { panic!("one item for two cards: {raised:?}") };
    let id = format!("try:{session}");
    assert_eq!((item.id.as_str(), item.text.as_str(), item.artifact.as_deref()), (id.as_str(), "Ready to try", Some(id.as_str())));

    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("first answered");
    assert_eq!(tries(&rig, &session).await.len(), 1, "one card still open");
    let asked = AnswerReadyToTry { session_id: SessionId::carried(session.as_str()), branch: "armada/two".into(), tip: second, answer: ReadyAnswer::LooksGood };
    api::Commands::answer_ready_to_try(Arc::clone(&rig.fleet), asked).await.expect("second answered");
    assert!(tries(&rig, &session).await.is_empty(), "both answered leaves none");
}

#[tokio::test]
async fn dismissing_the_ready_to_try_row_puts_away_only_the_cards_open_then() {
    let rig = a_rig();
    let (session, _tip) = an_offered_job(&rig).await;
    let id = format!("try:{session}");

    let dismissal = ipc::DismissWaiting { session_id: SessionId::carried(session.as_str()), item_id: id.clone(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismissal).await.expect("dismissed");
    assert!(tries(&rig, &session).await.is_empty(), "the card open then is put away");

    rig.holds_branch(&session, "armada/two").await;
    branch_with(&rig, "armada/two", &base(&rig), "crates/x/two.rs", "2");
    offer(&rig, OfferPreview { branch: Some("armada/two".into()), ..offered(&session) }).await.expect("offered a new card");
    let raised = tries(&rig, &session).await;
    let [item] = raised.as_slice() else { panic!("the new card raises the row again: {raised:?}") };
    assert_eq!(item.id, id);
}

#[tokio::test]
async fn a_card_whose_offer_left_the_list_is_withdrawn_by_fleet() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    assert_eq!(tries(&rig, &session).await.len(), 1);

    // The branch moved, so Fleet drops the offer; no Job calls anything.
    branch_with(&rig, BRANCH, BRANCH, "crates/x/more.rs", "2");
    rig.fleet.previews_watched().await;
    checked(&rig).await;

    assert!(tries(&rig, &session).await.is_empty(), "the Job's row goes with the offer");
    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Withdrawn);
}

#[tokio::test]
async fn withdrawing_a_branch_with_open_cards_and_no_offer_takes_the_cards_back() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    rig.fleet.store().lock().await.remove_preview_offered(&session, BRANCH).unwrap();
    assert_eq!(tries(&rig, &session).await.len(), 1, "the card outlived its offer");

    let withdraw = || WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: Some(BRANCH.into()) };
    Arc::clone(&rig.fleet).withdraw_preview(None, withdraw()).await.expect("withdrawn with no offer standing");
    assert!(tries(&rig, &session).await.is_empty());
    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Withdrawn);

    let again = Arc::clone(&rig.fleet).withdraw_preview(None, withdraw()).await;
    assert_eq!(code(again), "fleet.preview_not_offered", "neither an offer nor an open card");
}

#[tokio::test]
async fn an_answered_card_stays_answered_when_its_branch_is_withdrawn() {
    let rig = a_rig();
    let (session, tip) = an_offered_job(&rig).await;
    answer(&rig, &session, &tip, ReadyAnswer::LooksGood).await.expect("answered");

    Arc::clone(&rig.fleet).withdraw_preview(None, WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: None }).await.expect("the offer withdrawn");

    assert_eq!(card(&rig, &session, &tip).await.state, ReadyState::Answered);
}
