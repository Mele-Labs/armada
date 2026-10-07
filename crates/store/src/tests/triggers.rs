//! Frozen Triggers and their firings survive a reopen, and freezing replaces.

use core_model::{
    FrozenTrigger, OnTriggerFailure, StepId, Timestamp, TriggerFiring, TriggerResolution,
    TriggerSkipped, TriggerSource, TriggerState, TriggerWhen,
};

use crate::tests::{job_id, open, top_level, TempDir};

fn frozen(name: &str, resolution: TriggerResolution) -> FrozenTrigger {
    FrozenTrigger {
        name: name.to_string(),
        when: TriggerWhen::PrOpened,
        step: StepId::new("handoff"),
        source: TriggerSource::Machine,
        resolution,
        on_failure: OnTriggerFailure {
            block: true,
            repair: false,
        },
    }
}

#[test]
fn a_frozen_set_reads_back_whole_and_a_second_freeze_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01TRIGGERS");
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01TRIGGERS");
    let first = vec![
        frozen(
            "deploy",
            TriggerResolution::Command {
                name: "deploy_qa".to_string(),
                asks_first: true,
            },
        ),
        frozen(
            "gone",
            TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo {
                command: "x".to_string(),
            }),
        ),
    ];
    store.freeze_triggers(&id, &first).expect("frozen");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.frozen_triggers(&id).expect("read"), first);
    let second = vec![frozen(
        "tidy",
        TriggerResolution::Skill {
            name: "tidy-up".to_string(),
        },
    )];
    store.freeze_triggers(&id, &second).expect("frozen again");
    assert_eq!(store.frozen_triggers(&id).expect("read"), second);
}

#[test]
fn a_firing_opens_running_and_is_settled_in_place() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01FIRING"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01FIRING");
    let at = |s: &str| Timestamp::from_rfc3339(format!("2026-10-07T09:00:{s}.000Z"));
    let one = frozen(
        "deploy",
        TriggerResolution::Command {
            name: "deploy_qa".to_string(),
            asks_first: false,
        },
    );

    let running = TriggerFiring::running(&one, at("01"));
    let row = store.open_firing(&id, &running).expect("opened");
    assert_eq!(
        store.trigger_firings(&id).expect("read")[0].state,
        TriggerState::Running
    );
    let ended = running.ended(Some(3), at("02"));
    store.settle_firing(row, &ended).expect("settled");
    store
        .open_firing(
            &id,
            &TriggerFiring::skipped(
                &one,
                TriggerSkipped::SkillNotRun {
                    skill: "s".to_string(),
                },
                at("03"),
            ),
        )
        .expect("skipped");

    let read = store.trigger_firings(&id).expect("read");
    assert_eq!(read[0], ended);
    assert_eq!(read[0].state, TriggerState::Failed);
    assert_eq!(read[1].state, TriggerState::Skipped);
    assert_eq!(
        read[1].skipped,
        Some(TriggerSkipped::SkillNotRun {
            skill: "s".to_string()
        })
    );
}
