//! The copy a Job started with, and what its rules planned: kept once, kept with the pairs that made
//! it, and settled once.

use crate::tests::{open, TempDir};
use crate::{rule_act_standing as standing, KeptCopy, KeptPiece, KeptSeen, NewRuleAct, PieceKind};

fn copy(text: &str) -> KeptCopy {
    KeptCopy {
        pieces: vec![
            KeptPiece { kind: PieceKind::Lifecycle, piece_id: "ticket".into(), path: "/kit/lifecycles/ticket.yml".into(), text: "a ticket".into() },
            KeptPiece { kind: PieceKind::Rule, piece_id: "moves".into(), path: "/kit/rules/moves.yml".into(), text: text.into() },
        ],
        autonomy: vec![("forge_write".into(), "acts".into())],
    }
}

fn act<'a>(rule: &'a str, position: u32, on: &'a str, state: &'a str) -> NewRuleAct<'a> {
    NewRuleAct { rule_id: rule, position, on: Some(("ticket", on)), entered: None, course: "asks", state, text: "Move it", said: "" }
}

#[test]
fn a_job_keeps_the_copy_it_started_with_and_a_second_freeze_changes_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.job_copy_of("old").expect("reads"), None, "a Job from before has none");
    assert!(store.freeze_job_copy("s1", "t0", &copy("the rule as first saved")).expect("freezes"));
    assert!(!store.freeze_job_copy("s1", "t1", &copy("the rule as saved later")).expect("freezes again"));
    assert_eq!(store.job_copy_of("s1").expect("reads"), Some(copy("the rule as first saved")));
    assert_eq!(store.job_copy_of("s2").expect("reads"), None);
}

#[test]
fn a_plan_is_kept_with_the_rules_that_ran_and_the_same_plan_kept_again_adds_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let ids = store
        .keep_rule_plan("s1", "occurrence", &["moves"], &[act("moves", 0, "ARM-41", standing::ASKED), act("moves", 0, "ARM-42", standing::ASKED)], "t0")
        .expect("keeps");
    assert_eq!(ids.len(), 2);
    assert_eq!(store.rules_that_ran("s1", "occurrence").expect("reads"), ["moves"]);
    assert!(store.rules_that_ran("s1", "another").expect("reads").is_empty());

    let again = store
        .keep_rule_plan("s1", "occurrence", &["moves"], &[act("moves", 0, "ARM-41", standing::ASKED)], "t1")
        .expect("keeps again");
    assert_eq!(again, [ids[0]], "the act that was there is the one named");
    let kept = store.rule_acts_of("s1").expect("reads");
    assert_eq!(kept.len(), 2);
    assert_eq!((kept[0].on.clone(), kept[0].raised_at.as_str()), (Some(("ticket".into(), "ARM-41".into())), "t0"));
}

#[test]
fn an_act_is_settled_once_and_a_try_is_counted_before_it_is_made() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.keep_rule_plan("s1", "o", &["moves"], &[act("moves", 0, "ARM-41", standing::PLANNED)], "t0").expect("keeps")[0];
    assert_eq!(store.begin_rule_attempt(id).expect("counts"), 1);
    assert_eq!(store.begin_rule_attempt(id).expect("counts"), 2);
    assert!(store.move_rule_act(id, &[standing::PLANNED, standing::ASKED], standing::DONE, "", "t1").expect("settles"));
    assert!(!store.move_rule_act(id, &[standing::PLANNED, standing::ASKED], standing::SKIPPED, "", "t2").expect("settles"), "it was done already");
    let kept = store.rule_act(id).expect("reads").expect("there");
    assert_eq!((kept.state.as_str(), kept.attempts, kept.settled_at.as_str()), (standing::DONE, 2, "t1"));
}

#[test]
fn an_act_that_failed_past_its_retries_starts_over_only_from_stuck() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.keep_rule_plan("s1", "o", &["moves"], &[act("moves", 0, "ARM-41", standing::PLANNED)], "t0").expect("keeps")[0];
    assert!(!store.restart_rule_act(id, standing::PLANNED).expect("restarts"), "it is not stuck");
    store.begin_rule_attempt(id).expect("counts");
    assert!(store.move_rule_act(id, &[standing::PLANNED], standing::STUCK, "the forge refused", "t1").expect("sticks"));
    assert!(store.restart_rule_act(id, standing::PLANNED).expect("restarts"));
    let kept = store.rule_act(id).expect("reads").expect("there");
    assert_eq!((kept.state.as_str(), kept.attempts, kept.said.as_str()), (standing::PLANNED, 0, ""));
}

#[test]
fn what_the_last_look_at_a_job_showed_is_kept_and_replaced() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.job_seen("s1").expect("reads"), None);
    store.keep_job_seen("s1", &KeptSeen { step: Some("plan".into()), events: 2, ..KeptSeen::default() }).expect("keeps");
    store.keep_job_seen("s1", &KeptSeen { step: None, pull_request_opened: true, events: 3, ..KeptSeen::default() }).expect("keeps");
    assert_eq!(
        store.job_seen("s1").expect("reads"),
        Some(KeptSeen { step: None, pull_request_opened: true, landed: false, ended: false, events: 3 })
    );
}
