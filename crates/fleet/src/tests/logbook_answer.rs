//! Distil's answer, read back and held to the record: claims citing nothing the record holds, or
//! naming an entry not offered, are dropped; a habit needs the person's own message; one item the
//! model spelt wrong costs only that item. `docs/concepts/logbook.md`.

use std::collections::BTreeSet;

use store::{LogKind, LogSection};

use crate::logbook::answer::{hard, read, Known, Ran};

fn ran(n: usize, command: &str, failed: bool) -> Ran {
    Ran { cite: format!("ran:{n}"), command: command.to_string(), failed, exit: failed.then_some(1), said: String::new() }
}

fn known_read(said: &str) -> crate::logbook::answer::Distilled {
    let cites: BTreeSet<String> = ["tool:1", "correction:1", "ran:1", "ran:2"].iter().map(|one| one.to_string()).collect();
    let ids = BTreeSet::from([4, 5]);
    let ran = [ran(1, "pnpm test", true), ran(2, "pnpm -F desktop test", false)];
    read(said, &Known { cites: &cites, ids: &ids, ran: &ran }).expect("reads")
}

#[test]
fn a_logbook_answer_keeps_only_what_the_record_holds_up() {
    let said = r#"Here it is:
```json
{"confirmed":[{"id":4,"evidence":["tool:1"]},{"id":9,"evidence":["tool:1"]},{"id":5,"evidence":["tool:7"]}],
 "contradicted":[{"id":5,"evidence":["tool:1"]},{"id":5,"text":"Chrome is not in /Applications.","evidence":["tool:1"]}],
 "tripped":[{"id":4,"evidence":["ran:1"]}, "nonsense"],
 "new":[
  {"section":"repo","kind":"gotcha","text":"Run `pnpm -F desktop test`; `pnpm test` fails at the root.","evidence":["ran:1","ran:2"]},
  {"section":"repo","kind":"gotcha","text":"Uncited.","evidence":["tool:99"]},
  {"section":"habits","kind":"habit","text":"Ask before landing.","evidence":["tool:1"]},
  {"section":"habits","kind":"habit","text":"Never run cargo fmt.","evidence":["correction:1"]},
  {"section":"machine","kind":"gotcha","text":"A machine gotcha is the wrong kind.","evidence":["tool:1"]},
  {"section":"repo","kind":"convention","text":"one\ntwo\nthree","evidence":["tool:1"]},
  {"section":"repo","kind":"spell","text":"Misspelt kind.","evidence":["tool:1"]}
 ]}
```"#;
    let read = known_read(said);
    assert_eq!(read.confirmed.iter().map(|one| one.id).collect::<Vec<_>>(), vec![4], "an unoffered id and an unheld cite go");
    assert_eq!(read.contradicted.len(), 1, "a contradiction with no text goes");
    assert_eq!(read.contradicted[0].text.as_deref(), Some("Chrome is not in /Applications."));
    assert_eq!(read.tripped.len(), 1, "an item that will not read costs only itself");
    let learnt: Vec<(LogSection, LogKind, bool)> = read.learnt.iter().map(|one| (one.section, one.kind, one.hard)).collect();
    assert_eq!(learnt, vec![(LogSection::Repo, LogKind::Gotcha, true), (LogSection::Habits, LogKind::Habit, false)]);
}

#[test]
fn a_logbook_answer_with_no_object_is_refused() {
    let cites = BTreeSet::new();
    let ids = BTreeSet::new();
    assert!(read("nothing to learn", &Known { cites: &cites, ids: &ids, ran: &[] }).is_err());
}

#[test]
fn logbook_hard_evidence_is_a_failure_then_a_different_form_that_worked() {
    let rows = [ran(1, "pnpm test", true), ran(2, "pnpm test", false), ran(3, "pnpm -F desktop test", false)];
    let cites = |names: &[&str]| names.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    assert!(hard(&cites(&["ran:1", "ran:3"]), &rows));
    assert!(!hard(&cites(&["ran:1", "ran:2"]), &rows), "the same command again is a retry");
    assert!(!hard(&cites(&["ran:3", "ran:1"]), &[ran(1, "a", false), ran(2, "x", false), ran(3, "b", true)]), "worked before it failed");
    assert!(!hard(&cites(&["ran:1"]), &rows));
}

#[test]
fn the_question_asks_for_one_canonical_form_of_a_command() {
    let question = crate::logbook::QUESTION;
    for left_out in ["redirect", "/tmp", "--reporter="] {
        assert!(question.contains(left_out), "the question never says to leave out `{left_out}`");
    }
    assert!(question.contains("one canonical form"), "the question never asks for one canonical form of a command");
}
