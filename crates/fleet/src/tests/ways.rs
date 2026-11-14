//! How the owner works with Jobs reaches a Job by mechanism: the note every Job starts with and every
//! helper is briefed with, the note told to a running Job when he changes it, a question with no options
//! sent back when he answers by buttons, `all_done` held until a picture is shown when he looks, and the
//! block a plan step ends with. Each is Fleet's, so none rests on a Job choosing to read a list.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{Sessions, Settings};
use config::ResolvedWorkflow;
use core_model::WorkflowId;
use ipc::{
    AllDone, AnswerWith, LookQuestions, LookResults, Preferences, ReadResults, SavePreference, SessionId, SetWaitingFor,
    TakeIn, WaitingAct, WaitingActKind, WaitingInput, WaitingOption,
};
use store::{AttachmentState, Holder, KeptAttachment};

use super::rule_acting::notices_in;
use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::proposing::workflow_words;
use crate::ways;

fn prefs(take_in: TakeIn, answer_with: AnswerWith) -> Preferences {
    Preferences { take_in, answer_with, ..Preferences::default() }
}

async fn save(rig: &Rig, name: &str, word: &str) {
    rig.fleet
        .save_preference_now(SavePreference { name: name.into(), value: false, text: Some(word.into()) })
        .await
        .unwrap_or_else(|why| panic!("{name} {word}: {why:?}"));
}

/// A Job with a process running, as one is once it has been written to.
async fn running(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    eventually(|| async { rig.stand_in.process_of(id.as_str()).is_some() }).await;
    id
}

fn question(id: &str, options: usize) -> WaitingInput {
    WaitingInput {
        id: id.into(),
        text: "Which way?".into(),
        act: None,
        options: (0..options).map(|n| WaitingOption { label: format!("Way {n}"), description: None, picture: None }).collect(),
        approves: false,
        steps: Vec::new(),
    }
}

fn listing(id: &SessionId, items: Vec<WaitingInput>) -> SetWaitingFor {
    SetWaitingFor { items, session_id: Some(id.clone()) }
}

fn done(id: &SessionId) -> AllDone {
    AllDone { session_id: Some(id.clone()), summary: None }
}

async fn shown(rig: &Rig, id: &SessionId, target: &str, detail: &[(&str, &str)]) {
    let now = rig.fleet.now().as_str().to_string();
    let artifact = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "artifact".into(),
        manifest_id: rig.manifest.as_str().to_string(),
        target: target.into(),
        state: AttachmentState::Standing,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect::<BTreeMap<_, _>>(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&artifact, false).expect("kept");
}

// ------------------------------------------------------------------ the note's words

#[test]
fn the_note_at_every_default_is_the_header_and_the_read_line() {
    assert_eq!(
        ways::note(&Preferences::default()),
        "How the owner works with Jobs, from his Settings. Fleet checks it.\n\
         Results: a short note in the thread for small work. A page under .armada/documents/, conclusion first, for work with more than one decision."
    );
}

#[test]
fn the_note_joins_a_take_in_part_and_an_answer_part_in_his_words() {
    let read = |results| Preferences { read_results: results, ..Preferences::default() };
    assert!(ways::note(&read(ReadResults::AlwaysPage)).ends_with("Results: every result is a page under .armada/documents/, conclusion first."));
    assert!(ways::note(&read(ReadResults::ThreadOnly)).ends_with("Results: in the thread as text. No separate pages."));

    let look = |results, questions| Preferences { take_in: TakeIn::Look, look_results: results, look_questions: questions, ..Preferences::default() };
    let every = ways::note(&look(LookResults::PictureEvery, LookQuestions::PicturesMix));
    assert!(every.contains("Results: lead every result with a picture through show_sketch, a mock for a screen and a diagram for anything else, with two lines of text under it. all_done is refused until a picture is shown. The result picture rides on the done card, so it asks no approval of its own."), "{every}");
    assert!(every.ends_with("Questions with options that look different: draw each option as a mock and offer them as pictures, with the parts he might take from one and not another named by data-part."), "{every}");
    let screens = ways::note(&look(LookResults::PictureScreens, LookQuestions::PicturesWhole));
    assert!(screens.contains("Results: lead a result that changes a screen with a mock through show_sketch. Other results may be text."), "{screens}");
    assert!(screens.ends_with("offer them as pictures."), "the whole, with no parts clause: {screens}");
    let walk = ways::note(&look(LookResults::WalkVisible, LookQuestions::PicturesMix));
    assert!(walk.contains("Results: open anything he would see as a walk with show_window, and a diagram through show_sketch for the rest. all_done is refused until a picture is shown. The result picture rides on the done card, so it asks no approval of its own."), "{walk}");

    let listen = ways::note(&prefs(TakeIn::Listen, AnswerWith::Type));
    assert!(listen.ends_with("Results: Bridge reads your all_done summary aloud. Write it as two or three spoken sentences, saying what changed and what waits on him. No paths, code or tables in it."), "{listen}");
    assert!(!listen.contains("Questions with options"), "the questions line is Look's alone: {listen}");

    let talk = ways::note(&prefs(TakeIn::Read, AnswerWith::Talk));
    assert!(talk.ends_with("Answers: he answers by voice. Ask questions he can answer in a few spoken words."), "{talk}");
    let buttons = ways::note(&prefs(TakeIn::Listen, AnswerWith::Buttons));
    assert!(buttons.ends_with("Answers: ask every question with two to four options, your pick first, a cost on each, one decision a question. A question without options is sent back."), "{buttons}");
}

#[test]
fn the_plan_block_follows_how_he_takes_work_in_and_adds_the_buttons_line() {
    assert_eq!(ways::plan_block(&Preferences::default()), "", "every default adds none");
    assert_eq!(ways::plan_block(&prefs(TakeIn::Look, AnswerWith::Type)), "Lead with pictures: sketch the shape with show_sketch before you ask.");
    assert_eq!(
        ways::plan_block(&prefs(TakeIn::Listen, AnswerWith::Type)),
        "Open with a short version he can hear: two or three plain sentences in your reply before any detail."
    );
    assert_eq!(
        ways::plan_block(&prefs(TakeIn::Read, AnswerWith::Talk)),
        "Put a page first: the approach as a page under .armada/documents/, conclusion first, then ask."
    );
    assert_eq!(
        ways::plan_block(&prefs(TakeIn::Look, AnswerWith::Buttons)),
        "Lead with pictures: sketch the shape with show_sketch before you ask. \
         Settle this with him as a quiz: one decision a question through AskUserQuestion, two to four options, your pick first, a cost on each, until the shape is agreed."
    );
    let thread_only = Preferences { answer_with: AnswerWith::Buttons, read_results: ReadResults::ThreadOnly, ..Preferences::default() };
    assert!(!ways::plan_block(&thread_only).contains("Put a page first"), "he said no separate pages");
}

// ------------------------------------------------------------------ 1: starting rules

#[tokio::test]
async fn a_jobs_start_note_carries_the_note_of_his_preferences() {
    let rig = rig_placing(None);
    save(&rig, "answer_with", "buttons").await;
    let id = running(&rig).await;

    let start = rig.stand_in.starts().into_iter().find(|one| one.session == id.as_str()).expect("started");
    let told = start.note.expect("a note");
    assert!(told.contains(&rig.fleet.ways_note()), "{told}");
    assert!(told.contains("Answers: ask every question with two to four options"), "{told}");
}

#[tokio::test]
async fn a_helpers_brief_carries_the_same_note() {
    let rig = rig_placing(None);
    assert_eq!(rig.fleet.logbook_for_subagent("no-such-session", "work in apps/desktop/src").await, ways::note(&Preferences::default()));

    save(&rig, "take_in", "listen").await;
    let handed = rig.fleet.logbook_for_subagent("no-such-session", "work in apps/desktop/src").await;
    assert!(handed.contains("Results: Bridge reads your all_done summary aloud."), "{handed}");
}

// ------------------------------------------------------------------ 2: a change reaches running Jobs

#[tokio::test]
async fn a_saved_change_to_how_he_works_is_told_to_a_running_job_as_a_fleet_notice() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let before = rig.fleet.get_preferences().await.expect("read");

    save(&rig, "answer_with", "talk").await;
    let now = rig.fleet.get_preferences().await.expect("read");
    Arc::clone(&rig.fleet).preferences_saved(before, now.clone()).await;

    let told: Vec<String> = notices_in(&rig, &id).await.into_iter().filter(|(word, _)| word.as_deref() == Some("taught")).map(|(_, text)| text).collect();
    assert_eq!(told, [ways::note(&now)], "the whole new note, once");
    assert!(told[0].contains("Answers: he answers by voice."));
    let sent = rig.stand_in.process_of(id.as_str()).expect("running").sent.lock().unwrap().join("\n");
    assert!(sent.contains("he answers by voice"), "it reached the process: {sent}");
}

#[tokio::test]
async fn a_save_that_leaves_the_note_as_it_was_tells_nobody() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let before = rig.fleet.get_preferences().await.expect("read");

    save(&rig, "listen_voice", "Daniel").await;
    let now = rig.fleet.get_preferences().await.expect("read");
    Arc::clone(&rig.fleet).preferences_saved(before, now).await;

    assert!(notices_in(&rig, &id).await.iter().all(|(word, _)| word.as_deref() != Some("taught")));
}

// ------------------------------------------------------------------ 3: the buttons check

#[tokio::test]
async fn a_question_with_no_options_or_more_than_four_is_refused_by_name_where_he_answers_by_buttons() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.set_waiting(None, listing(&id, vec![question("which", 0)])).await.expect("he types by default, so nothing is checked");
    save(&rig, "answer_with", "buttons").await;

    let none = rig.fleet.set_waiting(None, listing(&id, vec![question("which", 0)])).await.unwrap_err();
    assert_eq!((none.status(), none.error().code.as_str()), (422, "fleet.session_waiting_options"));
    assert!(none.error().message.contains("\"which\"") && none.error().message.contains("has no options"), "{}", none.error().message);
    assert!(none.error().message.contains("two to four options, your pick first and a cost on each"), "{}", none.error().message);

    let many = rig.fleet.set_waiting(None, listing(&id, vec![question("which", 5)])).await.unwrap_err();
    assert_eq!(many.error().code, "fleet.session_waiting_options");
    assert!(many.error().message.contains("has 5 options"), "{}", many.error().message);

    for options in [2, 3, 4] {
        rig.fleet.set_waiting(None, listing(&id, vec![question("which", options)])).await.expect("two to four is a question he can press");
    }
}

#[tokio::test]
async fn an_item_that_is_not_a_question_needs_no_options_where_he_answers_by_buttons() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    save(&rig, "answer_with", "buttons").await;
    let run = WaitingInput { act: Some(WaitingAct { kind: WaitingActKind::Run, target: "scripts/restart".into() }), ..question("restart", 0) };
    let approval = WaitingInput { approves: true, ..question("plan", 0) };
    let procedure = WaitingInput { steps: vec!["Open Settings.".into()], ..question("todo", 0) };
    rig.fleet.set_waiting(None, listing(&id, vec![run, approval, procedure])).await.expect("a run, an approval and a procedure are not questions");
}

// ------------------------------------------------------------------ 4: the look check

#[tokio::test]
async fn all_done_is_refused_by_name_until_a_picture_is_shown_where_he_looks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    save(&rig, "take_in", "look").await;

    let refused = rig.fleet.all_done(None, done(&id)).await.unwrap_err();
    assert_eq!((refused.status(), refused.error().code.as_str()), (409, "fleet.picture_owed"));
    assert!(refused.error().message.starts_with("all_done refused. "), "{}", refused.error().message);
    assert!(refused.error().message.contains("Lead the result with a picture"), "{}", refused.error().message);

    shown(&rig, &id, "notes.md", &[("form", "file"), ("by", "agent"), ("shown", "t1")]).await;
    assert!(rig.fleet.all_done(None, done(&id)).await.is_err(), "a text file is no picture");

    shown(&rig, &id, "/tmp/after.png", &[("form", "file"), ("by", "agent"), ("shown", "t2")]).await;
    assert!(rig.fleet.all_done(None, done(&id)).await.expect("an image shown is a picture").done_since.is_some());
}

/// Marks the Job a Side Job, as `start_side_job` keeps it.
pub(super) async fn made_side_job(rig: &Rig, id: &SessionId) {
    let mut store = rig.fleet.store().lock().await;
    let mut hosting = store.hosting(id.as_str()).unwrap().expect("hosted");
    hosting.side_job = true;
    store.keep_hosting(&hosting).unwrap();
}

#[tokio::test]
async fn a_side_job_that_answered_owes_no_picture_until_it_records_a_plan() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    save(&rig, "take_in", "look").await;
    made_side_job(&rig, &id).await;
    assert!(rig.fleet.all_done(None, done(&id)).await.expect("an answer needs no picture").done_since.is_some());

    let id = rig.start().await;
    made_side_job(&rig, &id).await;
    super::session_rail::approved_plan(&rig, &id).await;
    let refused = rig.fleet.all_done(None, done(&id)).await.unwrap_err();
    assert_eq!(refused.error().code.as_str(), "fleet.picture_owed", "a Side Job that built something shows it");
}

#[tokio::test]
async fn a_sketch_or_a_window_is_a_picture_and_the_other_looks_are_not_held() {
    let rig = rig_placing(None);
    save(&rig, "take_in", "look").await;
    for form in ["sketch", "window"] {
        let id = rig.start().await;
        shown(&rig, &id, "a target", &[("form", form)]).await;
        rig.fleet.all_done(None, done(&id)).await.unwrap_or_else(|why| panic!("{form}: {why:?}"));
    }

    save(&rig, "look_results", "picture_screens").await;
    let id = rig.start().await;
    rig.fleet.all_done(None, done(&id)).await.expect("screens only: other results may be text, and it is not checked");

    save(&rig, "take_in", "read").await;
    save(&rig, "look_results", "picture_every").await;
    let id = rig.start().await;
    rig.fleet.all_done(None, done(&id)).await.expect("reading him asks for no picture");
}

// ------------------------------------------------------------------ 5: plan talks in workflows

fn workflow() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: feature\nname: feature\nsteps:\n  \
         - id: propose\n    label: Propose\n    delivers: false\n    advance_gate: auto\n    gate: plan_approved\n    \
         instructions: |\n      Say what you will build.\n  \
         - id: shape\n    label: Shape\n    delivers: false\n    advance_gate: auto\n    gate: sketch_approved\n  \
         - id: plan\n    label: Plan\n    delivers: false\n    advance_gate: auto\n    evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      - { type: plan_recorded }\n  \
         - id: build\n    label: Build\n    delivers: false\n    advance_gate: auto\n    instructions: Write the code.\n  \
         - id: walk\n    label: Walk\n    delivers: false\n    advance_gate: auto\n    gate: walk_approved\n  \
         - id: land\n    label: Land\n    delivers: true\n    advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = config::Manifest::parse(std::path::Path::new("fixture-armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\n")
        .expect("the fixture manifest parses");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"));
    [(resolved.id().clone(), resolved)].into_iter().collect()
}

#[test]
fn a_step_that_settles_a_plan_with_him_ends_with_the_block_and_one_that_does_not_is_left_alone() {
    let held = workflow();
    let id = held.keys().next().expect("one workflow").as_str().to_string();
    let block = ways::plan_block(&prefs(TakeIn::Look, AnswerWith::Type));
    let words = workflow_words(&held, Some(&id), false, &block).expect("told");
    let lines: Vec<&str> = words.lines().skip(1).collect();

    assert_eq!(lines[0], format!("1. propose, \"Propose\", gate plan_approved: Say what you will build. {block}"), "gated on his plan");
    assert_eq!(lines[1], format!("2. shape, \"Shape\", gate sketch_approved: {block}"), "gated on his sketch, with no instructions of its own");
    assert!(lines[2].starts_with("3. plan, \"Plan\", gate none: ") && lines[2].ends_with(&block), "a step that records the plan: {}", lines[2]);
    assert_eq!(lines[3], "4. build, \"Build\", gate none: Write the code.", "no gate and no plan");
    assert_eq!(lines[4], "5. walk, \"Walk\", gate walk_approved", "a walk is not a plan talk");
    assert_eq!(lines[5], "6. land, \"Land\", gate none");
}

#[test]
fn with_no_block_every_step_reads_as_it_did() {
    let held = workflow();
    let id = held.keys().next().expect("one workflow").as_str().to_string();
    let words = workflow_words(&held, Some(&id), false, &ways::plan_block(&Preferences::default())).expect("told");
    assert!(words.contains("1. propose, \"Propose\", gate plan_approved: Say what you will build.\n"), "{words}");
    assert!(words.contains("2. shape, \"Shape\", gate sketch_approved\n"), "{words}");
}
