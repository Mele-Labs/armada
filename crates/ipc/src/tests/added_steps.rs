//! Steps added to one Job on the wire (23.68): each crosses as the JSON the
//! TypeScript side reads, absent where empty, and a 23.58 peer sees nothing new.

use crate::tests::{detail_of, job};
use crate::{
    decode, encode, AddStep, AddedPlaced, AddedRuns, AddedStep, ApproveDispatch, Event, Instant,
    JobAdditionChanged, JobId, KeptFrom, SaveTrigger, StepId, TriggerFiringState, TriggerMoment,
    TriggerScope,
};

fn a_passed_script() -> AddedStep {
    AddedStep {
        id: "a1".into(),
        runs: AddedRuns::Script {
            command: "fmt".into(),
        },
        when: TriggerMoment::StepPasses,
        step: StepId::carried("implement"),
        block: false,
        repair: true,
        placed: AddedPlaced::Running,
        added_at: Instant::carried("2026-10-07T10:00:00.000Z"),
        state: TriggerFiringState::Passed,
        skipped: None,
        exit_code: Some(0),
        started_at: Some(Instant::carried("2026-10-07T10:00:01.000Z")),
        ended_at: Some(Instant::carried("2026-10-07T10:00:02.000Z")),
        log_at: Some(Instant::carried("2026-10-07T10:00:02.000Z")),
        kept: Some(TriggerScope::Machine),
        repair_record: None,
    }
}

/// **This string is the TypeScript side's fixture too**: `apps/desktop/src/main/
/// added-steps.test.ts` parses the same text, so a field renamed on either side
/// fails in one of the two.
pub const ADDED: &str = r#"{"id":"a1","runs":{"kind":"script","command":"fmt"},"when":"step_passes","step":"implement","block":false,"repair":true,"placed":"running","added_at":"2026-10-07T10:00:00.000Z","state":"passed","exit_code":0,"started_at":"2026-10-07T10:00:01.000Z","ended_at":"2026-10-07T10:00:02.000Z","log_at":"2026-10-07T10:00:02.000Z","kept":"machine"}"#;

#[test]
fn an_added_step_is_the_json_bridge_decodes() {
    assert_eq!(encode(&a_passed_script()).expect("encodes"), ADDED);
    let back: AddedStep = decode("an added step", ADDED.as_bytes()).expect("decodes");
    assert_eq!(back, a_passed_script());
}

#[test]
fn a_step_to_add_reads_its_switches_as_off_and_tells_the_three_kinds_apart() {
    let said = br#"{"runs":{"kind":"skill","skill":"tidy-up"},"when":"pr_opened","step":"summarise","repair":true,"later":1}"#;
    let add: AddStep = decode("a step to add", said).expect("decodes");
    assert!(!add.block && add.repair);
    assert_eq!(
        add.runs,
        AddedRuns::Skill {
            skill: "tidy-up".into()
        }
    );
    for text in [
        r#"{"runs":{"kind":"script","command":"fmt"},"when":"step_starts","step":"a"}"#,
        r#"{"runs":{"kind":"drone","brief":"read it twice"},"when":"step_passes","step":"a"}"#,
    ] {
        let add: AddStep = decode("a step to add", text.as_bytes()).expect("decodes");
        assert_eq!(encode(&add).expect("encodes").contains("\"kind\""), true);
    }
}

#[test]
fn an_approval_without_additions_carries_no_key_and_one_with_them_round_trips() {
    assert!(!encode(&ApproveDispatch::default())
        .expect("encodes")
        .contains("additions"));
    let said = br#"{"additions":[{"runs":{"kind":"script","command":"fmt"},"when":"step_passes","step":"implement","block":true}]}"#;
    let body: ApproveDispatch = decode("an approval", said).expect("decodes");
    let [one] = body.additions.as_deref().expect("held") else {
        panic!("one addition");
    };
    assert!(one.block && !one.repair);
    let text = encode(&body).expect("encodes");
    let back: ApproveDispatch = decode("an approval", text.as_bytes()).expect("decodes");
    assert_eq!(back, body);
}

#[test]
fn a_detail_without_additions_carries_no_key_and_one_with_them_round_trips() {
    let mut detail = detail_of(&job(), &[]);
    assert!(!encode(&detail).expect("encodes").contains("additions"));
    detail.additions.push(a_passed_script());
    let text = encode(&detail).expect("encodes");
    assert!(text.contains(r#""additions":[{"id":"a1""#), "{text}");
    let back: crate::JobDetail = decode("a detail", text.as_bytes()).expect("decodes");
    assert_eq!(back, detail);
}

#[test]
fn the_event_names_the_job_and_carries_the_row_whole() {
    let event = Event::JobAdditionChanged(JobAdditionChanged {
        job_id: job().id().into(),
        addition: a_passed_script(),
        removed: false,
        at: Instant::carried("2026-10-07T10:00:02.000Z"),
    });
    assert_eq!(event.kind(), "job.addition_changed");
    assert!(
        event.about().0.is_some(),
        "a Board filters on the job it names"
    );
    let text = encode(&event).expect("encodes");
    assert!(!text.contains("removed"), "{text}");
    let back: Event = decode("an event", text.as_bytes()).expect("decodes");
    assert_eq!(back, event);
}

#[test]
fn a_save_names_the_addition_it_keeps_and_without_one_reads_as_every_save_did() {
    let said = br#"{"scope":"machine","definition":"name: x\n"}"#;
    let save: SaveTrigger = decode("a save", said).expect("decodes");
    assert_eq!(save.kept_from, None);
    let kept = SaveTrigger {
        kept_from: Some(KeptFrom {
            job_id: JobId::carried("01JOB"),
            addition_id: "a1".into(),
        }),
        ..save
    };
    let text = encode(&kept).expect("encodes");
    assert!(
        text.contains(r#""kept_from":{"job_id":"01JOB","addition_id":"a1"}"#),
        "{text}"
    );
}
