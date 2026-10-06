//! Pressing Accept on a Kit retro item that carries a `change`: it is applied to
//! Kit's allowlist, once, and the answer says what was applied. An item with no
//! change is only saved. `docs/concepts/retro.md`.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::Router;
use core_model::{Change, LandsIn, Whose};
use ipc::{KitAllowedSource, Lesson, RecordRefusal, RetroChange};
use store::{Reflected, RetroLine};
use testkit::FakeJudge;

use super::retro::{killed, running, sent, From};
use crate::kit_allowlist::{parse, FILE};
use crate::tests::tmp::TempDir;

const COMMAND: &str = "grep -a -c";

fn kit_item(said: &str, change: Option<&str>) -> RetroLine {
    RetroLine {
        whose: Whose::Drone,
        title: Some(String::from("The Drone was refused grep")),
        what: Some(said.to_string()),
        fix: Some(String::from("Allow grep on check logs.")),
        said: said.to_string(),
        evidence: vec![String::from("refusal:1")],
        lands_in: Some(LandsIn::Kit),
        change: change.map(|command| Change::AllowCommand {
            command: command.to_string(),
        }),
    }
}

/// A Fleet whose Job has ended with a retro of three Kit items kept: one with a
/// change, one with none, and one with a change a person will disagree with.
async fn retro_kept(home: &TempDir) -> (Arc<super::retro::Fixture>, Router, Vec<String>) {
    let (fleet, app, id) = running(home, Arc::new(FakeJudge::saying("{\"items\":[]}"))).await;
    killed(&app, &id, From::Anyone).await;
    let job = core_model::JobId::carried(core_model::Ulid::carried(id.clone()));
    fleet
        .store()
        .lock()
        .await
        .record_retro(
            &job,
            &Reflected::Written {
                model: String::from("a-model"),
                items: vec![
                    kit_item("grep was refused on a check log", Some(COMMAND)),
                    kit_item("a skill was missing", None),
                    kit_item("rg was refused", Some("rg --files")),
                ],
            },
            &fleet.now(),
        )
        .expect("kept");
    let ids = (0..3).map(|place| format!("{id}-{place}")).collect();
    (fleet, app, ids)
}

async fn pressed(app: &Router, id: &str, act: &str) -> Lesson {
    let (status, body) = sent(
        app,
        "POST",
        &format!("/lessons/{id}/{act}"),
        "",
        From::Bridge,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    ipc::decode("a lesson", &body).expect("a Lesson")
}

fn the_file(home: &TempDir) -> std::path::PathBuf {
    home.path().join("kit").join(FILE)
}

fn on_disk(home: &TempDir) -> String {
    std::fs::read_to_string(the_file(home)).unwrap_or_default()
}

/// **Accept on a Kit item with a change appends it, sets `accepted`, and answers
/// with what was applied**, naming the item as where it came from.
#[tokio::test]
async fn accepting_a_kit_item_with_a_change_appends_it_and_says_what_was_applied() {
    let home = TempDir::new();
    let (_fleet, app, ids) = retro_kept(&home).await;
    assert_eq!(on_disk(&home), "", "nothing yet");

    let accepted = pressed(&app, &ids[0], "agree").await;

    assert_eq!(accepted.state.as_wire(), "accepted");
    let applied = RetroChange::AllowCommand {
        command: String::from(COMMAND),
    };
    assert_eq!(accepted.change, Some(applied.clone()));
    assert_eq!(accepted.applied, Some(applied), "what was applied");
    assert_eq!(
        parse(&on_disk(&home)),
        vec![ipc::KitAllowedCommand {
            run: String::from(COMMAND),
            from: KitAllowedSource::RetroItem {
                lesson_id: ids[0].clone()
            },
        }]
    );
}

/// **Agreeing twice applies once**: the second press answers with where the item
/// stands, and the allowlist holds one line. A person who took the line out in
/// between is not given it back by pressing again.
#[tokio::test]
async fn agreeing_twice_applies_once() {
    let home = TempDir::new();
    let (fleet, app, ids) = retro_kept(&home).await;

    let first = pressed(&app, &ids[0], "agree").await;
    let second = pressed(&app, &ids[0], "agree").await;

    assert_eq!(first, second, "the standing state, again");
    assert_eq!(parse(&on_disk(&home)).len(), 1, "{}", on_disk(&home));

    fleet
        .remove_from_kit_allowlist(COMMAND)
        .await
        .expect("a person takes it out");
    let third = pressed(&app, &ids[0], "agree").await;
    assert_eq!(third.state.as_wire(), "accepted");
    assert_eq!(on_disk(&home).contains(COMMAND), false, "not given back");
}

/// **A Kit item with no change still only saves**, and says nothing was applied.
#[tokio::test]
async fn accepting_a_kit_item_without_a_change_only_saves_it() {
    let home = TempDir::new();
    let (_fleet, app, ids) = retro_kept(&home).await;

    let kept = pressed(&app, &ids[1], "agree").await;

    assert_eq!(kept.state.as_wire(), "accepted");
    assert_eq!(kept.change, None);
    assert_eq!(kept.applied, None, "nothing was applied");
    assert_eq!(on_disk(&home), "", "and nothing was written");
}

/// **Disagree is unchanged**: it discards the item and applies nothing, though
/// the item carries a change.
#[tokio::test]
async fn disagreeing_applies_nothing() {
    let home = TempDir::new();
    let (_fleet, app, ids) = retro_kept(&home).await;

    let discarded = pressed(&app, &ids[2], "disagree").await;

    assert_eq!(discarded.state.as_wire(), "discarded");
    assert_eq!(discarded.applied, None);
    assert_eq!(on_disk(&home), "");
    assert!(
        discarded.change.is_some(),
        "the item still says what it would have changed"
    );
}

/// **A listing reads `applied` back**, so a screen drawn later says what Accept
/// did without asking again.
#[tokio::test]
async fn the_listing_reads_what_was_applied_back() {
    let home = TempDir::new();
    let (_fleet, app, ids) = retro_kept(&home).await;
    pressed(&app, &ids[0], "agree").await;

    let (status, body) = sent(&app, "GET", "/lessons?state=accepted", "", From::Bridge).await;

    assert_eq!(status, StatusCode::OK);
    let lessons: ipc::Lessons = ipc::decode("the lessons", &body).expect("Lessons");
    let [one] = lessons.lessons.as_slice() else {
        panic!("one accepted item: {lessons:?}");
    };
    assert!(one.applied.is_some(), "{one:?}");
}

fn refusal(cite: &str, tool: &str, tried: Option<&str>) -> RecordRefusal {
    RecordRefusal {
        cite: cite.to_string(),
        at: ipc::Instant::carried("2026-10-05T09:00:00.000Z"),
        step: None,
        tool: tool.to_string(),
        tried: tried.map(str::to_string),
        because: None,
    }
}

/// What the model answered: one Kit item with this `change` written in it.
fn said(change: &str) -> String {
    format!(
        "{{\"items\":[{{\"who\":\"drone\",\"lands_in\":\"kit\",\"title\":\"Grep was refused\",\
         \"what\":\"The Drone asked for grep on a check log.\",\"fix\":\"Allow grep.\",\
         \"evidence\":[\"refusal:1\"]{change}}}]}}"
    )
}

fn read_change(change: &str, refusals: &[RecordRefusal]) -> Option<Change> {
    let known = BTreeSet::from([String::from("refusal:1")]);
    let items = crate::retro::read(&said(change), &known, refusals).expect("it reads");
    assert_eq!(
        items.len(),
        1,
        "the item stays whatever became of its change"
    );
    items.into_iter().next().and_then(|item| item.change)
}

const TRIED: &str = "grep -a -c arc-dispatch .armada/checks/x/implement.1.dry.0.log";

/// **Fleet copies the command off the refusal the item cites**, and the model
/// never writes one. A shorter start of it may be named, and only a start of it.
#[test]
fn the_command_is_copied_off_the_refusal_the_item_cites() {
    let record = [refusal("refusal:1", "Bash", Some(TRIED))];
    let whole = r#","change":{"kind":"allow_command","refusal":"refusal:1"}"#;
    assert_eq!(
        read_change(whole, &record),
        Some(Change::AllowCommand {
            command: String::from(TRIED)
        })
    );
    let cut = r#","change":{"kind":"allow_command","refusal":"refusal:1","command":"grep -a -c"}"#;
    assert_eq!(
        read_change(cut, &record),
        Some(Change::AllowCommand {
            command: String::from("grep -a -c")
        })
    );
}

/// **A change the cited row does not hold is dropped**, and the item stays.
#[test]
fn a_change_the_cited_row_does_not_hold_is_dropped() {
    let record = [refusal("refusal:1", "Bash", Some(TRIED))];
    let ask = |refusal: &str, command: &str| {
        format!(r#","change":{{"kind":"allow_command","refusal":"{refusal}"{command}}}"#,)
    };
    // A command the model wrote, which the refusal never tried.
    assert_eq!(
        read_change(
            &ask("refusal:1", r#","command":"curl example.test""#),
            &record
        ),
        None
    );
    // The right words, in the wrong order, are not a start of it.
    assert_eq!(
        read_change(&ask("refusal:1", r#","command":"grep -c -a""#), &record),
        None
    );
    // A row the record does not have.
    assert_eq!(read_change(&ask("refusal:9", ""), &record), None);
    // A row that is not a shell call.
    let other = [refusal("refusal:1", "Write", Some(TRIED))];
    assert_eq!(read_change(&ask("refusal:1", ""), &other), None);
    // A row that kept no argument.
    let none = [refusal("refusal:1", "Bash", None)];
    assert_eq!(read_change(&ask("refusal:1", ""), &none), None);
    // A row cut for length holds a command nobody read in full.
    let long = [refusal("refusal:1", "Bash", Some("grep -a -c arc…"))];
    assert_eq!(read_change(&ask("refusal:1", ""), &long), None);
    // A refused command that chains another has no whole to allow.
    let chained = [refusal("refusal:1", "Bash", Some("grep a x && curl y"))];
    assert_eq!(read_change(&ask("refusal:1", ""), &chained), None);
    assert_eq!(
        read_change(&ask("refusal:1", r#","command":"grep a""#), &chained),
        Some(Change::AllowCommand {
            command: String::from("grep a")
        }),
        "a start of it before the chain is still one of its own cuts"
    );
}

/// **Only a Kit item carries a change.**
#[test]
fn an_item_that_lands_elsewhere_carries_no_change() {
    let record = [refusal("refusal:1", "Bash", Some(TRIED))];
    let known = BTreeSet::from([String::from("refusal:1")]);
    let text = said(r#","change":{"kind":"allow_command","refusal":"refusal:1"}"#)
        .replace("\"lands_in\":\"kit\"", "\"lands_in\":\"manifest\"");
    let items = crate::retro::read(&text, &known, &record).expect("it reads");
    assert_eq!(items[0].change, None);
}
