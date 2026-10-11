//! The guided review of open retro items and the question about one, end to
//! end over the router, and the rules that hold a model's answer to the items
//! it was given. `docs/concepts/retro.md`, *Reviewing*.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Router;
use ipc::{AskLessonAnswer, LessonReview};
use testkit::FakeJudge;

use super::retro::{killed, running, sent, From};
use crate::retro::review_for_tests::read;
use crate::tests::tmp::TempDir;

/// A retro of two items, and the review the model gives is scripted by
/// `review`. The second answer is for a question about an item.
fn judge(review: &str) -> Arc<FakeJudge> {
    Arc::new(FakeJudge::answering(&[
        (
            "-----BEGIN RECORD-----",
            "{\"items\":[\
             {\"who\":\"fleet\",\"lands_in\":\"manifest\",\"title\":\"Browser tests wait a fixed time\",\
              \"what\":\"desktop_test timed out after 15 seconds on three attempts.\",\
              \"fix\":\"Raise the timeout in the desktop test config.\",\"evidence\":[\"act:1\"]},\
             {\"who\":\"fleet\",\"lands_in\":\"armada\",\"title\":\"Gate measured a stale main\",\
              \"what\":\"Two upstream commits counted as the Drone's work.\",\
              \"fix\":\"Fetch main before the gate measures.\",\"evidence\":[\"act:1\"]}]}",
        ),
        ("-----BEGIN ITEMS-----", review),
        (
            "-----BEGIN CITED ROWS-----",
            "  The record shows one act, a kill from outside Bridge. It shows no timeout.  ",
        ),
    ]))
}

async fn posted(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    sent(app, "POST", uri, body, From::Bridge).await
}

fn review(body: &[u8]) -> LessonReview {
    ipc::decode("a review", body).expect("a LessonReview")
}

/// **No open item, no model call**, and the answer is empty.
#[tokio::test]
async fn an_empty_open_set_answers_empty_without_a_model_call() {
    let home = TempDir::new();
    let judge = judge("{}");
    let (_fleet, app, _id) = running(&home, Arc::clone(&judge)).await;
    let before = judge.asked().len();

    let (status, body) = posted(&app, "/lessons/review", "").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered = review(&body);
    assert!(answered.entries.is_empty() && answered.set_aside.is_empty());
    assert_eq!(judge.asked().len(), before, "no call was made");
}

/// **One call reads every open item**, and an answer naming nothing the items
/// hold loses nothing: each open item is appended, `not ranked`, in the order
/// the list gives them, and the item's own parts and age reached the model.
#[tokio::test]
async fn one_call_reads_every_open_item_and_an_invented_id_loses_none() {
    let home = TempDir::new();
    let judge = judge(
        "Here you go: {\"entries\":[{\"lesson_id\":\"01INVENTED-9\",\"merged_ids\":[],\
         \"reason\":\"Made up.\"}],\"set_aside\":[{\"lesson_id\":\"01INVENTED-8\",\"why\":\"Old.\"}]}",
    );
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let (status, body) = posted(&app, "/lessons/review", "{}").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered = review(&body);
    let ids: Vec<&str> = answered
        .entries
        .iter()
        .map(|entry| entry.lesson_id.as_str())
        .collect();
    assert_eq!(ids, [format!("{id}-0"), format!("{id}-1")]);
    assert!(answered.entries.iter().all(|entry| entry.reason == "not ranked"));
    assert!(answered.set_aside.is_empty());
    assert!(!answered.model.is_empty());
    let asked: Vec<String> = judge
        .asked()
        .into_iter()
        .filter(|question| question.contains("-----BEGIN ITEMS-----"))
        .collect();
    let [question] = asked.as_slice() else {
        panic!("one review call: {asked:?}");
    };
    for carried in [
        "Browser tests wait a fixed time",
        "Fetch main before the gate measures.",
        "\"age_days\":0",
        "\"cites\":[\"act:1\"]",
    ] {
        assert!(question.contains(carried), "{carried}: {question}");
    }
}

/// An answer that will not read is a 500 that says so, and nothing is kept.
#[tokio::test]
async fn an_answer_that_will_not_read_is_a_failed_review() {
    let home = TempDir::new();
    let (fleet, app, id) = running(&home, judge("no json here")).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let (status, body) = posted(&app, "/lessons/review", "").await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(String::from_utf8_lossy(&body).contains("fleet.lesson_review_failed"));
}

/// **The question is answered from the item's record**: the model is handed
/// the item and the rows it cites and the person's words, and the reply is
/// trimmed.
#[tokio::test]
async fn a_question_is_answered_from_the_item_and_the_rows_it_cites() {
    let home = TempDir::new();
    let judge = judge("{}");
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");

    let (status, body) = posted(
        &app,
        &format!("/lessons/{id}-0/ask"),
        r#"{"question":"Did it time out?","history":[{"role":"person","text":"What is this?"},{"role":"fleet","text":"A timeout item."}]}"#,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let answered: AskLessonAnswer = ipc::decode("an answer", &body).expect("an answer");
    assert_eq!(
        answered.answer,
        "The record shows one act, a kill from outside Bridge. It shows no timeout."
    );
    let asked: Vec<String> = judge
        .asked()
        .into_iter()
        .filter(|question| question.contains("-----BEGIN CITED ROWS-----"))
        .collect();
    let [question] = asked.as_slice() else {
        panic!("one ask call: {asked:?}");
    };
    for carried in [
        "Browser tests wait a fixed time",
        "\"cite\":\"act:1\"",
        "A timeout item.",
        "The person asks: Did it time out?",
    ] {
        assert!(question.contains(carried), "{carried}: {question}");
    }
    assert!(
        !question.contains("\"cite\":\"act:2\""),
        "only the rows the item cites: {question}"
    );
}

/// An unknown id is a 404, and an empty or over-long question or history is
/// a 422, before any call is made.
#[tokio::test]
async fn an_unknown_id_and_a_question_past_its_bounds_are_refused() {
    let home = TempDir::new();
    let judge = judge("{}");
    let (fleet, app, id) = running(&home, Arc::clone(&judge)).await;
    killed(&app, &id, From::Anyone).await;
    fleet.reflect_next().await.expect("written");
    let before = judge.asked().len();

    let (status, body) = posted(&app, "/lessons/01NOBODY-0/ask", r#"{"question":"Why?"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.no_such_lesson"));

    let uri = format!("/lessons/{id}-0/ask");
    let long = "x".repeat(2_001);
    let turns = r#"{"role":"person","text":"a"},"#.repeat(21);
    for body in [
        r#"{"question":"   "}"#.to_string(),
        format!(r#"{{"question":"{long}"}}"#),
        format!(r#"{{"question":"Why?","history":[{}]}}"#, turns.trim_end_matches(',')),
    ] {
        let (status, answer) = posted(&app, &uri, &body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(String::from_utf8_lossy(&answer).contains("fleet.lesson_question_refused"));
    }
    assert_eq!(judge.asked().len(), before, "no call was made");
}

fn entry(lesson: &str, merged: &[&str], reason: &str) -> (String, Vec<String>, String) {
    (
        lesson.to_string(),
        merged.iter().map(|id| id.to_string()).collect(),
        reason.to_string(),
    )
}

fn shaped(said: &str, ids: &[&str]) -> (Vec<(String, Vec<String>, String)>, Vec<(String, String)>) {
    let (entries, set_aside) = read(said, ids).expect("reads");
    (
        entries
            .into_iter()
            .map(|e| (e.lesson_id, e.merged_ids, e.reason))
            .collect(),
        set_aside.into_iter().map(|a| (a.lesson_id, a.why)).collect(),
    )
}

/// **Order is the model's, an invented id is dropped, and an open id the model
/// left out is appended** to the end of the entries, in list order.
#[test]
fn the_models_order_stands_and_an_omitted_item_is_appended() {
    let said = r#"{"entries":[
        {"lesson_id":"c","merged_ids":[],"reason":"Recurs weekly."},
        {"lesson_id":"ghost","merged_ids":["b"],"reason":"Made up."},
        {"lesson_id":"a","merged_ids":[],"reason":"Cheap fix."}],"set_aside":[]}"#;

    let (entries, set_aside) = shaped(said, &["a", "b", "c", "d"]);

    assert_eq!(
        entries,
        [
            entry("c", &[], "Recurs weekly."),
            entry("a", &[], "Cheap fix."),
            entry("b", &[], "not ranked"),
            entry("d", &[], "not ranked"),
        ]
    );
    assert!(set_aside.is_empty());
}

/// **An id in both `merged_ids` and the entries is kept once**, as an entry,
/// and an id merged under two entries is merged under the first. An id
/// invented inside `merged_ids` is dropped.
#[test]
fn an_id_named_twice_is_kept_once() {
    let said = r#"{"entries":[
        {"lesson_id":"a","merged_ids":["b","a","ghost","c"],"reason":"Recurs."},
        {"lesson_id":"b","merged_ids":["c","d"],"reason":"Also worth it."},
        {"lesson_id":"a","merged_ids":["d"],"reason":"Again."}],"set_aside":[]}"#;

    let (entries, _) = shaped(said, &["a", "b", "c", "d"]);

    assert_eq!(
        entries,
        [
            entry("a", &["c"], "Recurs."),
            entry("b", &["d"], "Also worth it."),
        ]
    );
}

/// **An item is set aside only with a reason**, once, and never while it also
/// leads an entry or is merged. One set aside without a reason is appended.
#[test]
fn an_item_is_set_aside_only_with_a_reason_and_only_once() {
    let said = r#"{"entries":[{"lesson_id":"a","merged_ids":["b"],"reason":"Recurs."}],
        "set_aside":[
        {"lesson_id":"c","why":"The cited cause no longer applies."},
        {"lesson_id":"c","why":"Said twice."},
        {"lesson_id":"a","why":"Also an entry."},
        {"lesson_id":"b","why":"Also merged."},
        {"lesson_id":"d","why":"  "},
        {"lesson_id":"ghost","why":"Made up."}]}"#;

    let (entries, set_aside) = shaped(said, &["a", "b", "c", "d"]);

    assert_eq!(
        set_aside,
        [(
            "c".to_string(),
            "The cited cause no longer applies.".to_string()
        )]
    );
    assert_eq!(
        entries,
        [entry("a", &["b"], "Recurs."), entry("d", &[], "not ranked")]
    );
}

/// Prose around the JSON is passed over, a missing list reads as empty, and an
/// answer with no object is an error.
#[test]
fn only_the_json_object_is_read() {
    let (entries, _) = shaped("Sure.\n{\"entries\":[]}\nDone.", &["a"]);
    assert_eq!(entries, [entry("a", &[], "not ranked")]);
    assert!(read("nothing", &["a"]).is_err());
}
