//! Helm reading a thought for where it belongs, over the router: a match, no match, a question,
//! an id the model invented, a call that fails, and the row the thread keeps once the words were
//! delivered. `docs/concepts/helm.md`, *Routing a thought*.
//!
//! **The model is a script.** A fake answers each call with what the test wrote, so what is held
//! here is Fleet's half: who is offered to the model, what it is told, and what Fleet makes of an
//! answer.

use std::sync::Arc;
use std::time::Duration;

use api::{Conversations, Next, Sessions, Subscription};
use axum::http::StatusCode;
use ipc::{
    ReadThought, SessionFact, SessionId, SessionOrigin, SessionRecord, SessionReport, SessionRow,
    ThoughtKind, ThoughtReading, ThoughtSent, ThoughtTarget,
};
use testkit::{FakeJudge, FakeWorkProduct};

use super::retro::{running, sent, From, Fixture};
use crate::daemon::Fleet;
use crate::helm::routing::{plainly_a_question, reading_of, Candidate, READ_BUDGET};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

async fn session(fleet: &Fixture, id: &str, title: &str) {
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Started {
                cwd: "/somewhere/no/repository/serves".into(),
                title: Some(title.into()),
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("a session is started");
}

/// The words as Bridge sends them, and what Fleet reads them as.
async fn read(app: &axum::Router, words: &str) -> ThoughtReading {
    let body = ipc::encode(&ReadThought {
        text: words.into(),
    })
    .expect("plain data");
    let (status, answer) = sent(app, "POST", "/helm/read_thought", &body, From::Bridge).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    ipc::decode("a reading", &answer).expect("a ThoughtReading")
}

/// The calls the router put to the model, which only a read makes.
fn asked(judge: &FakeJudge) -> Vec<String> {
    judge
        .asked()
        .into_iter()
        .filter(|question| question.contains("-----BEGIN CANDIDATES-----"))
        .collect()
}

#[tokio::test]
async fn a_thought_naming_a_live_session_answers_that_session() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        r#"{"kind":"thought","id":"01LISTVIEWSAAAAAAAAAAAAAAA"}"#,
    ));
    let (fleet, app, _job) = running(&home, Arc::clone(&judge)).await;
    session(&fleet, "01LISTVIEWSAAAAAAAAAAAAAAA", "Sessions list views").await;

    let reading = read(&app, "the sessions page should say which ones wait on a merge").await;

    let ThoughtReading::Thought {
        target: Some(target),
    } = reading
    else {
        panic!("a thought with a target: {reading:?}");
    };
    assert_eq!(target.kind, ThoughtKind::Session);
    assert_eq!(target.id, "01LISTVIEWSAAAAAAAAAAAAAAA");
    assert_eq!(target.title, "Sessions list views");
    let [question] = asked(&judge).try_into().expect("one call");
    assert!(
        question.contains("the sessions page should say which ones wait on a merge"),
        "the words were put to the model: {question}"
    );
}

/// Every candidate reached the model: the live Session and the running Job, whatever repository
/// Helm points at, and the Job with what it must meet. An ended Session did not.
#[tokio::test]
async fn the_model_is_shown_every_live_session_and_running_job_and_no_ended_one() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"kind":"thought"}"#));
    let (fleet, app, job) = running(&home, Arc::clone(&judge)).await;
    session(&fleet, "01LIVEAAAAAAAAAAAAAAAAAAAA", "Release notes script").await;
    session(&fleet, "01ENDEDBBBBBBBBBBBBBBBBBBB", "Store migration spike").await;
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: SessionId::carried("01ENDEDBBBBBBBBBBBBBBBBBBB"),
            fact: SessionFact::Ended {
                reason: "closed".into(),
            },
        })
        .await
        .expect("ended");

    read(&app, "group the release notes by pull request").await;

    let [question] = asked(&judge).try_into().expect("one call");
    for shown in [
        "Release notes script",
        "01LIVEAAAAAAAAAAAAAAAAAAAA",
        "fix the off-by-one in the log reader",
        job.as_str(),
        "the symptom is gone",
    ] {
        assert!(question.contains(shown), "{shown}: {question}");
    }
    assert!(
        !question.contains("Store migration spike"),
        "an ended Session is no candidate: {question}"
    );
}

#[tokio::test]
async fn no_match_is_a_thought_with_no_target() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"kind":"thought"}"#));
    let (_fleet, app, _job) = running(&home, Arc::clone(&judge)).await;

    assert_eq!(
        read(&app, "the merge line page should group by repository").await,
        ThoughtReading::Thought { target: None }
    );
}

#[tokio::test]
async fn a_question_is_a_question() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"kind":"question"}"#));
    let (_fleet, app, _job) = running(&home, Arc::clone(&judge)).await;

    assert_eq!(
        read(&app, "look at job 12 and tell me what it expected").await,
        ThoughtReading::Question
    );
    assert_eq!(asked(&judge).len(), 1, "the model decided");
}

/// An id the model invented names no one, so the thought stands without a target.
#[tokio::test]
async fn an_id_that_is_no_candidates_is_no_match() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(
        r#"{"kind":"thought","id":"01INVENTEDNOTACANDIDATE"}"#,
    ));
    let (fleet, app, _job) = running(&home, judge).await;
    session(&fleet, "01LIVEAAAAAAAAAAAAAAAAAAAA", "Release notes script").await;

    assert_eq!(
        read(&app, "group the release notes by pull request").await,
        ThoughtReading::Thought { target: None }
    );
}

/// Whatever goes wrong, the words go to Helm.
#[tokio::test]
async fn a_call_that_fails_is_a_question() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::that_fails("the network is down"));
    let (fleet, app, _job) = running(&home, judge).await;
    session(&fleet, "01LIVEAAAAAAAAAAAAAAAAAAAA", "Release notes script").await;

    assert_eq!(
        read(&app, "group the release notes by pull request").await,
        ThoughtReading::Question
    );
}

#[tokio::test]
async fn an_answer_that_is_not_the_json_asked_for_is_a_question() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("I think this one is for the release notes"));
    let (_fleet, app, _job) = running(&home, judge).await;

    assert_eq!(
        read(&app, "group the release notes by pull request").await,
        ThoughtReading::Question
    );
}

#[tokio::test]
async fn blank_words_and_a_plain_wh_question_make_no_call() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(r#"{"kind":"thought"}"#));
    let (_fleet, app, _job) = running(&home, Arc::clone(&judge)).await;

    assert_eq!(read(&app, "   ").await, ThoughtReading::Question);
    assert_eq!(read(&app, "why did Job 12 fail?").await, ThoughtReading::Question);
    assert!(asked(&judge).is_empty(), "no call was made");
}

/// A read measured 5.0 to 7.5 seconds on a live Fleet, most of it the CLI starting, and at eight
/// seconds the slower ones were read as questions. Bridge shows the turning mark for as long as it
/// takes, so the budget is generous and not tight.
#[test]
fn the_read_waits_for_a_slow_cli_start() {
    assert_eq!(READ_BUDGET, Duration::from_secs(20));
}

fn candidates() -> Vec<Candidate> {
    vec![
        Candidate::job(
            "01JOBAAAAAAAAAAAAAAAAAAAAA",
            "Retry failed invoice charges with backoff",
            "billing",
            Some("armada/54-retry"),
            &["A failed charge is retried three times"],
            Some("Add a retry queue"),
        ),
        Candidate::job("01JOBBBBBBBBBBBBBBBBBBBBBB", "Pin the clock", "armada", None, &[], None),
    ]
}

#[test]
fn an_answer_is_held_to_the_candidates() {
    let named = |id: &str| format!(r#"{{"kind":"thought","id":"{id}"}}"#);
    let ThoughtReading::Thought {
        target: Some(target),
    } = reading_of(&named("01JOBAAAAAAAAAAAAAAAAAAAAA"), &candidates())
    else {
        panic!("the first Job");
    };
    assert_eq!(target.title, "Retry failed invoice charges with backoff");
    assert_eq!(
        target.manifest_id.as_ref().map(|id| id.as_str()),
        Some("billing")
    );
    assert_eq!(
        reading_of(&named("01NOONE"), &candidates()),
        ThoughtReading::Thought { target: None }
    );
    assert_eq!(
        reading_of(&named("01JOBAAAAAAAAAAAAAAAAAAAAA"), &[]),
        ThoughtReading::Thought { target: None },
        "nothing running offers nothing to name"
    );
}

#[test]
fn an_answer_with_prose_around_the_json_still_reads_and_anything_else_is_a_question() {
    assert_eq!(
        reading_of("Sure: {\"kind\":\"thought\"} done", &candidates()),
        ThoughtReading::Thought { target: None }
    );
    for unreadable in [
        "",
        "thought",
        "{\"kind\":\"banana\"}",
        "{\"id\":\"01JOBAAAAAAAAAAAAAAAAAAAAA\"}",
        "} {",
    ] {
        assert_eq!(
            reading_of(unreadable, &candidates()),
            ThoughtReading::Question,
            "{unreadable}"
        );
    }
}

/// **Narrow on purpose**: a request in a question's clothes still goes to the model.
#[test]
fn only_a_wh_word_and_a_question_mark_together_skip_the_model() {
    for plain in ["why did Job 12 fail?", "  What is this stuck on? ", "how many Drones are running?"] {
        assert!(plainly_a_question(plain), "{plain}");
    }
    for not in [
        "can you make the migration spike also cover the jobs table?",
        "why did Job 12 fail",
        "the sessions page should say which ones wait on a merge",
        "what if the page grouped them? make it so",
    ] {
        assert!(!plainly_a_question(not), "{not}");
    }
}

async fn published(watching: &mut Subscription) -> Vec<ipc::Event> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(50), watching.next()).await
    {
        seen.push(delivered.event);
    }
    seen
}

/// The words delivered are a row of the thread that outlives the window, an event of Helm's own,
/// and nothing said to Helm: the thread holds no `asked`.
#[tokio::test]
async fn words_delivered_are_a_row_in_the_thread_and_an_event_and_never_asked_of_helm() {
    let home = TempDir::new();
    let (fleet, app, _job) = running(&home, Arc::new(FakeJudge::saying("{}"))).await;
    let mut watching = fleet.events().subscribe();
    let target = ThoughtTarget {
        kind: ThoughtKind::Session,
        id: "01LISTVIEWSAAAAAAAAAAAAAAA".into(),
        title: "Sessions list views".into(),
        manifest_id: None,
        slot: Some(3),
    };
    let body = ipc::encode(&ThoughtSent {
        text: "say which ones wait on a merge".into(),
        target: target.clone(),
        attachments: Vec::new(),
        mentions: Vec::new(),
    })
    .expect("plain data");

    let (status, answer) = sent(&app, "POST", "/helm/thought_sent", &body, From::Bridge).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let record: SessionRecord = ipc::decode("a record", &answer).expect("the Helm Session");
    assert!(record.helm);
    let thread = Arc::clone(&fleet).get_helm_session(None).await.expect("read");
    let rows = thread.session.expect("a Helm Session was made for the row").rows;
    let [SessionRow::Sent { text, target: went, .. }] = &rows[..] else {
        panic!("one sent row and nothing asked of Helm: {rows:?}");
    };
    assert_eq!(text, "say which ones wait on a merge");
    assert_eq!(*went, target);
    let events = published(&mut watching).await;
    let routed: Vec<&ipc::HelmThoughtRouted> = events
        .iter()
        .filter_map(|event| match event {
            ipc::Event::HelmThoughtRouted(routed) => Some(routed),
            _ => None,
        })
        .collect();
    let [routed] = routed.as_slice() else {
        panic!("one routed event: {events:?}");
    };
    assert_eq!(routed.to, ThoughtKind::Session);
    assert_eq!(routed.id, "01LISTVIEWSAAAAAAAAAAAAAAA");
}

/// A Fleet restarted keeps the row: the thread is in the store.
#[tokio::test]
async fn the_row_is_still_there_after_a_restart() {
    let home = TempDir::new();
    let fleet = Arc::new(Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[]))));
    fleet
        .record_thought_sent(
            ThoughtSent {
                text: "back off longer".into(),
                target: ThoughtTarget {
                    kind: ThoughtKind::Job,
                    id: "01JOBAAAAAAAAAAAAAAAAAAAAA".into(),
                    title: "Retry failed invoice charges".into(),
                    manifest_id: None,
                    slot: None,
                },
                attachments: Vec::new(),
                mentions: Vec::new(),
            },
            None,
        )
        .await
        .expect("recorded");
    drop(fleet);

    let again = Arc::new(Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[]))));

    let rows = again
        .get_helm_session(None)
        .await
        .expect("read")
        .session
        .expect("the Helm Session outlives the Fleet")
        .rows;
    assert!(
        rows.iter()
            .any(|row| matches!(row, SessionRow::Sent { text, .. } if text == "back off longer")),
        "{rows:?}"
    );
}

/// Helm's own Session is where a question goes: it is never offered as a place a thought belongs.
#[tokio::test]
async fn the_helm_session_is_no_candidate_for_a_thought() {
    let home = TempDir::new();
    let (fleet, _app, _job) = running(&home, Arc::new(FakeJudge::saying("{}"))).await;
    let helm = fleet
        .record_thought_sent(
            ThoughtSent {
                text: "x".into(),
                target: ThoughtTarget {
                    kind: ThoughtKind::Job,
                    id: "01JOB".into(),
                    title: "A Job".into(),
                    manifest_id: None,
                    slot: None,
                },
                attachments: Vec::new(),
                mentions: Vec::new(),
            },
            None,
        )
        .await
        .expect("the Helm Session is made for the row");
    assert!(helm.helm && helm.title.as_deref() == Some("Helm"));
    assert!(Candidate::session(&helm).is_none(), "titled, live and still no candidate");
    let mut ordinary = helm.clone();
    ordinary.helm = false;
    assert!(Candidate::session(&ordinary).is_some(), "the same row unmarked is one");
}
