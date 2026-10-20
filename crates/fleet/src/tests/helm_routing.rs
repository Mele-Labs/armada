//! Helm reading a thought for where it belongs, over the router: a match, no match, a question,
//! an id the model invented, a call that fails, and the row the thread keeps once the words were
//! delivered. `docs/concepts/helm.md`, *Routing a thought*.
//!
//! **The model is a script.** A fake answers each call with what the test wrote, so what is held
//! here is Fleet's half: who is offered to the model, what it is told, and what Fleet makes of an
//! answer.

use std::sync::Arc;
use std::time::Duration;

use api::Conversations;
use ipc::{
    SessionRecord, SessionRow,
    ThoughtKind, ThoughtReading, ThoughtSent, ThoughtTarget,
};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::helm::routing::{plainly_a_question, reading_of, Candidate, READ_BUDGET};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// A read measured 5.0 to 7.5 seconds on a live Fleet, most of it the CLI starting, and at eight
/// seconds the slower ones were read as questions. Bridge shows the turning mark for as long as it
/// takes, so the budget is generous and not tight.
#[test]
fn the_read_waits_for_a_slow_cli_start() {
    assert_eq!(READ_BUDGET, Duration::from_secs(20));
}

pub(super) fn a_live_record(id: &str, title: &str, manifest: &str, branch: Option<&str>) -> SessionRecord {
    let attachments = branch
        .map(|branch| {
            format!(r#"{{"kind":"branch","target":"{branch}","state":"standing","since":"2026-10-12T09:00:00.000Z","changed_at":"2026-10-12T09:00:00.000Z"}}"#)
        })
        .unwrap_or_default();
    let json = format!(
        r#"{{"id":"{id}","harness":"a-harness","origin":"bridge","manifest_id":"{manifest}","cwd":"/work",
        "title":"{title}","state":"live","started_at":"2026-10-12T09:00:00.000Z",
        "last_seen_at":"2026-10-12T09:40:00.000Z","usage":{{}},"attachments":[{attachments}],"waiting_for":[]}}"#
    );
    ipc::decode("a record", json.as_bytes()).expect("a SessionRecord")
}

fn candidates() -> Vec<Candidate> {
    vec![
        a_live_record(
            "01SESSAAAAAAAAAAAAAAAAAAAA",
            "Retry failed invoice charges with backoff",
            "billing",
            Some("armada/54-retry"),
        ),
        a_live_record("01SESSBBBBBBBBBBBBBBBBBBBB", "Pin the clock", "armada", None),
    ]
    .iter()
    .filter_map(Candidate::session)
    .collect()
}

#[test]
fn an_answer_is_held_to_the_candidates() {
    let named = |id: &str| format!(r#"{{"kind":"thought","id":"{id}"}}"#);
    let ThoughtReading::Thought {
        target: Some(target),
    } = reading_of(&named("01SESSAAAAAAAAAAAAAAAAAAAA"), &candidates())
    else {
        panic!("the first Session");
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
        reading_of(&named("01SESSAAAAAAAAAAAAAAAAAAAA"), &[]),
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
        "{\"id\":\"01SESSAAAAAAAAAAAAAAAAAAAA\"}",
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
                    kind: ThoughtKind::Session,
                    id: "01SESSAAAAAAAAAAAAAAAAAAAA".into(),
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

