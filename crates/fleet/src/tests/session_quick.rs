//! The replies a Job's quick bar is offered beyond his saved lines: the answers a small model writes to the
//! agent's last message, and the lines he sends often across every Job. `get_session_quick`.
//!
//! **The model is the fake and the process is the stand-in**: what runs is Fleet's question, its reading of the
//! answer, the store, the settings file and the ranking.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{Instant, SaveSettings, SentFile, SessionId, SessionQuick, SessionRow, SessionVoice};
use store::settings_file::SettingValue;
use testkit::FakeJudge;

use super::session_host::{rig_fitted, Rig};
use crate::session_quick::{agent_spoke_last, alike, asks, learnt, question, read_asked, sent_in, QuickLook, Sent};

/// A fragment only the quick bar's question holds, so the fake answers it and nothing else.
const ASKED: &str = "Write the short replies he might send back";

const DAY: i64 = 86_400_000;
/// 2026-10-31T12:00:00Z.
const NOW: i64 = 1_793_448_000_000;

fn sent(text: &str, days_ago: i64, after_question: bool) -> Sent {
    Sent { text: text.to_string(), at_ms: NOW - days_ago * DAY, after_question }
}

fn twice(text: &str, days_ago: i64) -> Vec<Sent> {
    vec![sent(text, days_ago + 1, false), sent(text, days_ago, false)]
}

fn nothing_left_out() -> HashSet<String> {
    HashSet::new()
}

// ---- Ranking what he sends ----

#[test]
fn alike_lines_are_one_and_show_the_wording_he_used_last() {
    let lines = [sent("Looks good.", 3, false), sent("looks   good", 2, false), sent("Looks good!", 1, false)];
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Looks good!"]);
    assert_eq!(alike("  Looks   GOOD!!  "), alike("looks good"));
    assert_eq!(alike("?!"), "", "nothing but punctuation is no line");
}

#[test]
fn a_line_is_learnt_once_he_has_sent_it_twice() {
    let lines = [sent("Just get it done", 2, false), sent("Walk me through it", 2, false), sent("Walk me through it", 1, false)];
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Walk me through it"]);
}

#[test]
fn a_line_sent_lately_outranks_one_sent_more_often_long_ago() {
    let mut lines = vec![sent("Old habit", 60, false), sent("Old habit", 61, false), sent("Old habit", 62, false)];
    lines.extend(twice("New habit", 1));
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["New habit", "Old habit"]);
}

#[test]
fn between_lines_sent_as_lately_the_one_sent_more_often_ranks_first() {
    let mut lines = twice("Twice", 1);
    lines.extend([sent("Thrice", 2, false), sent("Thrice", 1, false), sent("Thrice", 1, false)]);
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Thrice", "Twice"]);
}

#[test]
fn a_sending_loses_half_its_weight_every_fourteen_days() {
    let times = |text: &str, n: usize, days_ago: i64| -> Vec<Sent> { (0..n).map(|_| sent(text, days_ago, false)).collect() };
    // Five sendings a fortnight old are worth 2.5, and two today 2.0.
    let mut lines = times("Fortnight old", 5, 14);
    lines.extend(times("Today", 2, 0));
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Fortnight old", "Today"]);
    // Seven sendings four weeks old are worth 1.75.
    let mut lines = times("Four weeks old", 7, 28);
    lines.extend(times("Today", 2, 0));
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Today", "Four weeks old"]);
}

#[test]
fn a_line_that_answered_a_question_ranks_higher_only_while_the_agent_asks_one() {
    let after = || vec![sent("Yes, go on", 2, true), sent("Yes, go on", 1, true)];
    let mut lines = after();
    lines.extend([sent("Thanks", 2, false), sent("Thanks", 1, false), sent("Thanks", 1, false)]);
    assert_eq!(learnt(&lines, &nothing_left_out(), true, NOW), ["Yes, go on", "Thanks"], "the agent asks: what answered a question first");
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW), ["Thanks", "Yes, go on"], "it does not: three sendings outweigh two");
}

#[test]
fn forgotten_and_saved_lines_are_left_out_however_they_are_spelled() {
    let mut lines = twice("Just get it done", 1);
    lines.extend(twice("Walk me through it", 1));
    lines.extend(twice("Keep going", 1));
    let left_out: HashSet<String> = [alike("just get it done."), alike("WALK me through it")].into_iter().collect();
    assert_eq!(learnt(&lines, &left_out, false, NOW), ["Keep going"]);
}

#[test]
fn no_more_than_six_are_learnt() {
    let lines: Vec<Sent> = (0..8).flat_map(|n| twice(&format!("Line {n}"), 1)).collect();
    assert_eq!(learnt(&lines, &nothing_left_out(), false, NOW).len(), 6);
}

// ---- What counts as a line he sent ----

fn said(from: SessionVoice, text: &str, at: &str) -> SessionRow {
    SessionRow::Message { id: format!("r{at}"), at: Instant::carried(at), from, text: text.to_string(), files: Vec::new(), tags: Vec::new(), notice: None, queued: false }
}

fn you(text: &str, at: &str) -> SessionRow {
    said(SessionVoice::You, text, at)
}

#[test]
fn only_one_line_of_sixty_characters_or_fewer_is_a_line_he_sent() {
    let sixty = "x".repeat(60);
    let sixty_one = "x".repeat(61);
    let mut with_file = you("Look at this", "2026-10-30T12:00:07Z");
    if let SessionRow::Message { files, .. } = &mut with_file {
        files.push(SentFile { id: "f1".into(), name: "a.png".into(), media_type: "image/png".into() });
    }
    let mut a_notice = you("Fleet says so", "2026-10-30T12:00:08Z");
    if let SessionRow::Message { notice, .. } = &mut a_notice {
        *notice = Some("slot_behind".into());
    }
    let rows = [
        you(&sixty, "2026-10-30T12:00:01Z"),
        you(&sixty_one, "2026-10-30T12:00:02Z"),
        you("two\nlines", "2026-10-30T12:00:03Z"),
        you("   ", "2026-10-30T12:00:04Z"),
        said(SessionVoice::Agent, "Done.", "2026-10-30T12:00:05Z"),
        said(SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() }, "A note", "2026-10-30T12:00:06Z"),
        with_file,
        a_notice,
        you("  Keep going  ", "2026-10-30T12:00:09Z"),
    ];
    let texts: Vec<String> = sent_in(&rows).into_iter().map(|one| one.text).collect();
    assert_eq!(texts, [sixty, "Keep going".to_string()]);
}

#[test]
fn a_line_is_marked_where_it_came_right_after_a_question_of_the_agents() {
    let rows = [
        said(SessionVoice::Agent, "Shall I carry on with the refactor?", "2026-10-30T12:00:01Z"),
        you("Yes, go on", "2026-10-30T12:00:02Z"),
        said(SessionVoice::Agent, "Done. The tests pass.", "2026-10-30T12:00:03Z"),
        you("Thanks", "2026-10-30T12:00:04Z"),
        you("Yes, go on", "2026-10-30T12:00:05Z"),
    ];
    let marks: Vec<(String, bool)> = sent_in(&rows).into_iter().map(|one| (one.text, one.after_question)).collect();
    assert_eq!(marks, [("Yes, go on".into(), true), ("Thanks".into(), false), ("Yes, go on".into(), false)]);
}

#[test]
fn a_message_asks_something_when_it_ends_with_a_question() {
    assert!(asks("Shall I carry on?"));
    assert!(asks("Shall I carry on? "));
    assert!(asks("Do you want **the long one**, or the short one?**"));
    assert!(asks("(Is that right?)"));
    assert!(asks("Did you mean \"main\"?\""));
    assert!(!asks("Is that right? I will carry on."));
    assert!(!asks("Done."));
}

#[test]
fn the_agent_spoke_last_only_when_its_message_is_the_threads_last() {
    let mut rows = vec![you("Hi", "2026-10-30T12:00:01Z"), said(SessionVoice::Agent, "Ready?", "2026-10-30T12:00:02Z")];
    assert_eq!(agent_spoke_last(&rows), Some(("r2026-10-30T12:00:02Z", "Ready?")));
    rows.push(SessionRow::Tool { id: "t1".into(), at: Instant::carried("2026-10-30T12:00:03Z"), text: "Reading a file".into(), subagent: None });
    assert!(agent_spoke_last(&rows).is_some(), "a call is no message");
    rows.push(you("Yes", "2026-10-30T12:00:04Z"));
    assert_eq!(agent_spoke_last(&rows), None);
}

// ---- The model's question and its answer ----

#[test]
fn the_question_carries_the_agents_message_from_its_end_and_his_own_lines() {
    let long = format!("{} Shall I carry on?", "a very long preamble ".repeat(200));
    let asked = question(&long, &["Just get it done".into(), "Walk me through it".into()]);
    assert!(asked.contains(ASKED), "{asked}");
    assert!(asked.contains("Shall I carry on?") && !asked.contains(&long), "the end of a long message, not the start of it");
    assert!(asked.contains("- Just get it done") && asked.contains("- Walk me through it"), "{asked}");
    assert!(question("Ready?", &[]).contains("(none yet)"));
}

#[test]
fn an_answer_is_read_into_at_most_three_lines_with_the_numbering_and_quotes_taken_off() {
    let reply = "Here are some replies:\n1. \"Yes, go on\"\n2) 'Not yet'\n- *Stop here*\n• Another one\n\n";
    assert_eq!(read_asked(reply), ["Yes, go on", "Not yet", "Stop here"]);
}

#[test]
fn an_answer_drops_a_repeat_a_word_for_nothing_and_a_line_too_long_for_a_chip() {
    let long = "y".repeat(61);
    let reply = format!("Yes, go on\nyes, go on.\nnone\n{long}\n7 is fine\nNot yet");
    assert_eq!(read_asked(&reply), ["Yes, go on", "7 is fine", "Not yet"]);
    assert!(read_asked("nothing").is_empty());
    assert!(read_asked("").is_empty());
}

// ---- What thread.look says ----

fn look(json: &str) -> QuickLook {
    let value: ipc::SettingValue = ipc::decode("a look", json.as_bytes()).expect("a value");
    QuickLook::read(Some(&crate::settings::file_value(&value)))
}

#[test]
fn a_look_nobody_saved_learns_and_shows_the_bar() {
    let none = QuickLook::read(None);
    assert!(none.learn && !none.hides("s1") && none.left_out.is_empty());
    assert_eq!(none, QuickLook::read(Some(&SettingValue::Text("not a look".into()))));
}

#[test]
fn a_look_names_what_to_hide_what_to_stop_learning_and_what_never_to_offer() {
    let read = look(r#"{"learn":false,"quickOff":false,"quickOffFor":["s2"],"quick":[{"id":"saved:1","text":"Keep going.","kind":"saved"},{"id":"learnt:1","text":"Ship it","kind":"learnt"}],"forgotten":["Walk me through it"]}"#);
    assert!(!read.learn);
    assert!(read.hides("s2") && !read.hides("s1"));
    let expected: HashSet<String> = ["keep going", "ship it", "walk me through it"].into_iter().map(String::from).collect();
    assert_eq!(read.left_out, expected);
    assert!(look(r#"{"quickOff":true}"#).hides("anything"));
}

// ---- Fleet's route, over a real store and settings file ----

fn rig_answering(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::answering(&[(ASKED, answer)]));
    let held = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| fittings.judge = held);
    (rig, judge)
}

fn questions(judge: &FakeJudge) -> usize {
    judge.asked().iter().filter(|one| one.contains(ASKED)).count()
}

async fn agent_says(rig: &Rig, id: &SessionId, text: &str) -> String {
    let row = rig.fleet.row_id(id.as_str());
    rig.fleet.row_put(id.as_str(), SessionRow::Message { id: row.clone(), at: rig.fleet.instant(), from: SessionVoice::Agent, text: text.into(), files: Vec::new(), tags: Vec::new(), notice: None, queued: false }).await;
    row
}

async fn quick(rig: &Rig, id: &SessionId) -> SessionQuick {
    rig.fleet.get_session_quick(id.clone()).await.expect("answered")
}

async fn save_look(rig: &Rig, json: &str) {
    let value: ipc::SettingValue = ipc::decode("a look", json.as_bytes()).expect("a value");
    let saved = rig.fleet.save_settings_now(SaveSettings { changes: [("thread.look".to_string(), Some(value))].into() }).await;
    assert!(saved.is_ok(), "{saved:?}");
}

#[tokio::test]
async fn the_agents_last_message_is_asked_about_once_and_the_answers_are_kept() {
    let (rig, judge) = rig_answering("1. \"Yes, carry on\"\n2. Not yet\n3. Stop here\n4. A fourth");
    let id = rig.start().await;
    agent_says(&rig, &id, "Shall I carry on with the refactor?").await;

    let first = quick(&rig, &id).await;
    assert_eq!(first.asked, ["Yes, carry on", "Not yet", "Stop here"]);
    assert_eq!(quick(&rig, &id).await, first);
    assert_eq!(questions(&judge), 1, "one ask for one message, however often the bar reads");
    let asked = judge.asked().into_iter().find(|one| one.contains(ASKED)).expect("asked");
    assert!(asked.contains("Shall I carry on with the refactor?"), "{asked}");
    assert_eq!(judge.models()[0], rig.fleet.proposing().expect("a proposer").model.as_str(), "on the proposer's model");

    agent_says(&rig, &id, "Or shall I stop here?").await;
    quick(&rig, &id).await;
    assert_eq!(questions(&judge), 2, "a new message is a new ask");
}

#[tokio::test]
async fn nothing_is_asked_while_the_agent_has_not_spoken_last() {
    let (rig, judge) = rig_answering("Yes");
    let id = rig.start().await;
    assert!(quick(&rig, &id).await.asked.is_empty(), "an empty thread");
    agent_says(&rig, &id, "Ready?").await;
    rig.send(&id, "Yes").await;
    assert!(quick(&rig, &id).await.asked.is_empty(), "he answered");
    assert_eq!(questions(&judge), 0);
}

#[tokio::test]
async fn nothing_is_asked_where_quick_replies_are_off_for_all_jobs_or_for_this_one() {
    let (rig, judge) = rig_answering("Yes, go on");
    let (id, other) = (rig.start().await, rig.start().await);
    agent_says(&rig, &id, "Ready?").await;
    agent_says(&rig, &other, "Ready?").await;

    save_look(&rig, r#"{"quickOff":true}"#).await;
    assert_eq!(quick(&rig, &id).await, SessionQuick::default());
    save_look(&rig, &format!(r#"{{"quickOffFor":["{}"]}}"#, id.as_str())).await;
    assert_eq!(quick(&rig, &id).await, SessionQuick::default());
    assert_eq!(questions(&judge), 0, "not for a Job whose bar is hidden");

    assert_eq!(quick(&rig, &other).await.asked, ["Yes, go on"], "its neighbour's bar is shown");
    assert_eq!(questions(&judge), 1);
}

#[tokio::test]
async fn a_call_that_fails_is_an_empty_list_and_is_not_asked_again() {
    let judge = Arc::new(FakeJudge::that_fails("the network"));
    let held = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| fittings.judge = held);
    let id = rig.start().await;
    agent_says(&rig, &id, "Ready?").await;
    assert_eq!(quick(&rig, &id).await, SessionQuick::default());
    quick(&rig, &id).await;
    assert_eq!(judge.asked().len(), 1);
}

#[tokio::test]
async fn an_unreadable_answer_is_an_empty_list_too() {
    let (rig, _judge) = rig_answering("none");
    let id = rig.start().await;
    agent_says(&rig, &id, "Ready?").await;
    assert!(quick(&rig, &id).await.asked.is_empty());
}

#[tokio::test]
async fn a_job_nobody_hosts_is_refused() {
    let (rig, _judge) = rig_answering("Yes");
    let refused = rig.fleet.get_session_quick(SessionId::carried("nope")).await.expect_err("no such job");
    assert_eq!((refused.error().code.as_str(), refused.status()), ("fleet.no_such_session", 422));
}

/// Each line, sent twice in turn across the Jobs, through the path a chip's press takes: a message to the Job.
async fn sends(rig: &Rig, jobs: &[&SessionId], lines: &[&str]) {
    for line in lines {
        for job in jobs {
            rig.send(job, line).await;
        }
    }
}

#[tokio::test]
async fn lines_he_sends_across_every_job_are_learnt_and_a_pressed_chip_counts_as_one() {
    let (rig, _judge) = rig_answering("none");
    let (a, b, c) = (rig.start().await, rig.start().await, rig.start().await);
    sends(&rig, &[&a, &b], &["Just get it done", "Walk me through it"]).await;
    rig.send(&c, "Only once").await;
    rig.send(&a, "Just get it done").await;

    let learnt = quick(&rig, &c).await.learnt;
    assert_eq!(learnt, ["Just get it done", "Walk me through it"], "sent in two Jobs and three times, then twice: a once-sent line is not here");
}

#[tokio::test]
async fn a_forgotten_line_and_a_saved_one_are_not_learnt_again() {
    let (rig, _judge) = rig_answering("none");
    let (a, b) = (rig.start().await, rig.start().await);
    sends(&rig, &[&a, &b], &["Just get it done", "Walk me through it", "Keep going"]).await;
    save_look(&rig, r#"{"forgotten":["just get it done"],"quick":[{"id":"saved:1","text":"Walk me through it","kind":"saved"}]}"#).await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Keep going"]);
}

#[tokio::test]
async fn with_learning_off_nothing_is_offered_and_no_thread_is_read() {
    let (rig, _judge) = rig_answering("none");
    let (a, b) = (rig.start().await, rig.start().await);
    sends(&rig, &[&a, &b], &["Just get it done"]).await;
    save_look(&rig, r#"{"learn":false}"#).await;
    assert!(quick(&rig, &a).await.learnt.is_empty());
    assert!(!rig.fleet.hosts().quick.sent_read(), "his lines were not read");

    save_look(&rig, r#"{"learn":true}"#).await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Just get it done"]);
    assert!(rig.fleet.hosts().quick.sent_read());
}

#[tokio::test]
async fn his_lines_are_read_off_the_threads_at_most_once_a_minute() {
    let (rig, _judge) = rig_answering("none");
    let (a, b) = (rig.start().await, rig.start().await);
    sends(&rig, &[&a, &b], &["Just get it done"]).await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Just get it done"]);

    sends(&rig, &[&a, &b], &["Walk me through it"]).await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Just get it done"], "the reading a moment ago stands");

    rig.fleet.hosts().quick.read_sent_every(Duration::ZERO);
    assert_eq!(quick(&rig, &a).await.learnt.len(), 2, "a minute on, the new line is in");
}

#[tokio::test]
async fn while_the_agent_asks_a_line_that_answered_a_question_ranks_first() {
    let (rig, _judge) = rig_answering("none");
    let (a, b) = (rig.start().await, rig.start().await);
    for job in [&a, &b] {
        agent_says(&rig, job, "Shall I carry on?").await;
        rig.send(job, "Yes, go on").await;
    }
    sends(&rig, &[&a, &b], &["Thanks"]).await;
    rig.send(&a, "Thanks").await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Thanks", "Yes, go on"], "he is not being asked");

    rig.fleet.hosts().quick.read_sent_every(Duration::ZERO);
    agent_says(&rig, &a, "Is that the one you meant?").await;
    assert_eq!(quick(&rig, &a).await.learnt, ["Yes, go on", "Thanks"], "he is");
}
