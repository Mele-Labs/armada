//! Triggers on the wire (23.58): a firing crosses as the JSON the TypeScript
//! side reads, absent where empty, and a 23.52 peer sees nothing new.

use crate::tests::{detail_of, job};
use crate::{
    decode, encode, Event, Instant, JobTrigger, JobTriggerChanged, SaveTrigger, StepId,
    TriggerFiringState, TriggerLevel, TriggerMoment, TriggerSkip, TriggerSkipReason,
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
    }
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
