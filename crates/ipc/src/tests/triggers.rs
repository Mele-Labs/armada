//! Triggers on the wire (23.58): a firing crosses as the JSON the TypeScript
//! side reads, absent where empty, and a 23.52 peer sees nothing new.

use crate::tests::{detail_of, job};
use crate::{
    decode, encode, ChooseTriggerFix, Event, HoldAct, HoldSettled, Instant, JobAlert, JobAlertKind,
    JobTrigger, JobTriggerChanged, SaveTrigger, StepId, TriggerFiringState, TriggerFixChoice, TriggerFixChosen, TriggerLevel, TriggerMoment,
    TriggerPullRequest, TriggerRepair, TriggerSkip, TriggerSkipReason,
};

fn a_failed_firing() -> JobTrigger {
    JobTrigger {
        name: "tidy".into(),
        when: TriggerMoment::StepPasses,
        step: StepId::carried("implement"),
        level: TriggerLevel::Machine,
        state: TriggerFiringState::Failed,
        skipped: None,
        exit_code: Some(1),
        started_at: Some(Instant::carried("2026-10-07T10:00:00.000Z")),
        ended_at: Some(Instant::carried("2026-10-07T10:00:02.000Z")),
        log_at: Some(Instant::carried("2026-10-07T10:00:02.000Z")),
        repair: None,
        blocks: false,
    }
}

fn a_held_fix() -> JobTrigger {
    JobTrigger {
        state: TriggerFiringState::FixReady,
        exit_code: Some(1),
        ended_at: None,
        repair: Some(TriggerRepair {
            attempt: 2,
            branch: Some("armada/repair-tidy-1".into()),
            files: vec!["src/lib.rs".into(), "Cargo.toml".into()],
            choice: None,
            pull_request: None,
        }),
        ..a_failed_firing()
    }
}

/// **The TypeScript side's fixture too**, as `FIRED` is.
pub const FIX_READY: &str = r#"{"name":"tidy","when":"step_passes","step":"implement","level":"machine","state":"fix_ready","exit_code":1,"started_at":"2026-10-07T10:00:00.000Z","log_at":"2026-10-07T10:00:02.000Z","repair":{"attempt":2,"branch":"armada/repair-tidy-1","files":["src/lib.rs","Cargo.toml"]}}"#;

#[test]
fn a_held_fix_is_the_json_bridge_decodes() {
    assert_eq!(encode(&a_held_fix()).expect("encodes"), FIX_READY);
    let back: JobTrigger = decode("a trigger", FIX_READY.as_bytes()).expect("decodes");
    assert_eq!(back, a_held_fix());
}

#[test]
fn a_placed_fix_carries_the_choice_and_the_number_of_the_pull_request() {
    let placed = JobTrigger {
        state: TriggerFiringState::Passed,
        repair: Some(TriggerRepair {
            choice: Some(TriggerFixChoice::NewPr),
            pull_request: Some(TriggerPullRequest::at("https://forge.test/o/r/pull/412")),
            ..a_held_fix().repair.expect("a repair")
        }),
        ..a_held_fix()
    };
    let text = encode(&placed).expect("encodes");
    assert!(
        text.contains(
            r#""choice":"new_pr","pull_request":{"url":"https://forge.test/o/r/pull/412","number":412}"#
        ),
        "{text}"
    );
    let back: JobTrigger = decode("a trigger", text.as_bytes()).expect("decodes");
    assert_eq!(back, placed);
    assert_eq!(
        TriggerPullRequest::at("https://example.test/pr").number,
        None
    );
}

#[test]
fn the_choice_is_a_body_and_the_answer_says_where_the_firing_stands() {
    let body: ChooseTriggerFix =
        decode("a choice", br#"{"trigger":"tidy","choice":"this_branch"}"#).expect("decodes");
    assert_eq!(body.choice, TriggerFixChoice::ThisBranch);
    let said = encode(&TriggerFixChosen {
        state: TriggerFiringState::Rerunning,
        pull_request: None,
    })
    .expect("encodes");
    assert_eq!(said, r#"{"state":"rerunning"}"#);
}

#[test]
fn a_fleet_before_the_repair_sends_no_repair_and_is_read_all_the_same() {
    let back: JobTrigger = decode("a trigger", FIRED.as_bytes()).expect("decodes");
    assert_eq!(back.repair, None);
}

/// **This string is the TypeScript side's fixture too**: `apps/desktop/src/main/
/// triggers.test.ts` parses the same text, so a field renamed on either side
/// fails in one of the two.
pub const FIRED: &str = r#"{"name":"tidy","when":"step_passes","step":"implement","level":"machine","state":"failed","exit_code":1,"started_at":"2026-10-07T10:00:00.000Z","ended_at":"2026-10-07T10:00:02.000Z","log_at":"2026-10-07T10:00:02.000Z"}"#;

#[test]
fn a_fired_trigger_is_the_json_bridge_decodes() {
    assert_eq!(encode(&a_failed_firing()).expect("encodes"), FIRED);
    let back: JobTrigger = decode("a trigger", FIRED.as_bytes()).expect("decodes");
    assert_eq!(back, a_failed_firing());
}

#[test]
fn a_pending_one_has_no_times_and_a_skipped_one_says_why() {
    let pending = JobTrigger {
        state: TriggerFiringState::Pending,
        exit_code: None,
        started_at: None,
        ended_at: None,
        log_at: None,
        repair: None,
        blocks: false,
        ..a_failed_firing()
    };
    let text = encode(&pending).expect("encodes");
    assert_eq!(
        text,
        r#"{"name":"tidy","when":"step_passes","step":"implement","level":"machine","state":"pending"}"#
    );
    let skipped = JobTrigger {
        state: TriggerFiringState::Skipped,
        skipped: Some(TriggerSkip {
            reason: TriggerSkipReason::NotInThisRepo,
            name: "fmt".into(),
            said: "skipped: `fmt` is not in this repo".into(),
        }),
        ..pending
    };
    let text = encode(&skipped).expect("encodes");
    assert!(
        text.contains(r#""skipped":{"reason":"not_in_this_repo","name":"fmt""#),
        "{text}"
    );
    let back: JobTrigger = decode("a trigger", text.as_bytes()).expect("decodes");
    assert_eq!(back, skipped);
}

#[test]
fn a_detail_without_triggers_carries_no_key_and_one_with_them_round_trips() {
    let mut detail = detail_of(&job(), &[]);
    assert!(!encode(&detail).expect("encodes").contains("triggers"));
    detail.triggers.push(a_failed_firing());
    let text = encode(&detail).expect("encodes");
    assert!(text.contains(r#""triggers":[{"name":"tidy""#), "{text}");
    let back: crate::JobDetail = decode("a detail", text.as_bytes()).expect("decodes");
    assert_eq!(back, detail);
}

#[test]
fn the_event_names_the_job_and_carries_the_row_whole() {
    let event = Event::JobTriggerChanged(JobTriggerChanged {
        job_id: job().id().into(),
        trigger: a_failed_firing(),
        at: Instant::carried("2026-10-07T10:00:02.000Z"),
    });
    assert_eq!(event.kind(), "job.trigger_changed");
    assert!(
        event.about().0.is_some(),
        "a Board filters on the job it names"
    );
    let text = encode(&event).expect("encodes");
    let back: Event = decode("an event", text.as_bytes()).expect("decodes");
    assert_eq!(back, event);
}

#[test]
fn a_save_without_overwrite_reads_as_false_and_a_peer_ignores_what_it_does_not_know() {
    let said = br#"{"scope":"machine","definition":"name: x\n","later":1}"#;
    let save: SaveTrigger = decode("a save", said).expect("decodes");
    assert!(!save.overwrite);
}

/// **The TypeScript side's fixture too**, as `FIRED` is.
pub const HELD: &str = r#"{"name":"deploy","when":"pr_opened","step":"summarise","level":"machine","state":"held","exit_code":1,"started_at":"2026-10-07T10:00:00.000Z","ended_at":"2026-10-07T10:00:02.000Z","log_at":"2026-10-07T10:00:02.000Z","blocks":true}"#;

fn a_held_firing() -> JobTrigger {
    JobTrigger {
        name: "deploy".into(),
        when: TriggerMoment::PrOpened,
        step: StepId::carried("summarise"),
        state: TriggerFiringState::Held,
        blocks: true,
        ..a_failed_firing()
    }
}

#[test]
fn a_held_trigger_says_it_blocks_and_one_that_does_not_leaves_it_out() {
    assert_eq!(encode(&a_held_firing()).expect("encodes"), HELD);
    let back: JobTrigger = decode("a trigger", HELD.as_bytes()).expect("decodes");
    assert_eq!(back, a_held_firing());
    assert!(!encode(&a_failed_firing()).expect("encodes").contains("blocks"));
    let before: JobTrigger = decode("a trigger", FIRED.as_bytes()).expect("decodes");
    assert!(!before.blocks, "a Fleet before 23.63 sends none");
}

#[test]
fn the_bell_on_a_row_names_the_trigger_and_where_it_fired() {
    let alert = JobAlert {
        kind: JobAlertKind::Held,
        trigger: "deploy".into(),
        when: TriggerMoment::PrOpened,
        step: StepId::carried("summarise"),
    };
    let text = encode(&alert).expect("encodes");
    assert_eq!(
        text,
        r#"{"kind":"held","trigger":"deploy","when":"pr_opened","step":"summarise"}"#
    );
    let back: JobAlert = decode("an alert", text.as_bytes()).expect("decodes");
    assert_eq!(back, alert);
    let mut row = crate::JobSummary::from(&job());
    assert!(!encode(&row).expect("encodes").contains("alert"));
    row.alert = Some(alert);
    let text = encode(&row).expect("encodes");
    let back: crate::JobSummary = decode("a row", text.as_bytes()).expect("decodes");
    assert_eq!(back, row);
    for kind in [JobAlertKind::FixReady, JobAlertKind::Failed] {
        assert!(encode(&kind).expect("encodes").starts_with('"'));
    }
}

#[test]
fn the_two_acts_name_a_trigger_or_an_added_step_and_the_answer_says_where_it_stands() {
    let body: HoldAct = decode("a hold", br#"{"trigger":"deploy"}"#).expect("decodes");
    assert_eq!((body.trigger.as_deref(), body.addition), (Some("deploy"), None));
    let body: HoldAct = decode("a hold", br#"{"addition":"a1"}"#).expect("decodes");
    assert_eq!(body.addition.as_deref(), Some("a1"));
    let said = encode(&HoldSettled {
        state: TriggerFiringState::Skipped,
        released: true,
    })
    .expect("encodes");
    assert_eq!(said, r#"{"state":"skipped","released":true}"#);
    let back: TriggerSkip = decode("a skip", br#"{"reason":"by_owner","name":"","said":"skipped by you while it held the Job"}"#)
        .expect("decodes");
    assert_eq!(back.reason, TriggerSkipReason::ByOwner);
}
