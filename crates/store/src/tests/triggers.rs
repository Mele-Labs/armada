//! Frozen Triggers and their firings survive a reopen, and freezing replaces.

use core_model::{
    FixChoice, FrozenTrigger, OnTriggerFailure, RepairRecord, StepId, Timestamp, TriggerFiring,
    TriggerResolution, TriggerSkipped, TriggerSource, TriggerState, TriggerWhen,
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
    assert_eq!(read[0].state, TriggerState::Held, "it blocks, and it failed");
    assert_eq!(read[1].state, TriggerState::Skipped);
    assert_eq!(
        read[1].skipped,
        Some(TriggerSkipped::SkillNotRun {
            skill: "s".to_string()
        })
    );
}

#[test]
fn a_repair_is_kept_on_the_firing_and_what_waits_on_a_person_is_listed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01REPAIR"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01REPAIR");
    let at = |s: &str| Timestamp::from_rfc3339(format!("2026-10-07T09:00:{s}.000Z"));
    let mut one = frozen(
        "deploy",
        TriggerResolution::Command {
            name: "deploy_qa".to_string(),
            asks_first: false,
        },
    );
    one.on_failure.repair = true;
    let running = TriggerFiring::running(&one, at("01"));
    let row = store.open_firing(&id, &running).expect("opened");
    let failed = running.ended(Some(1), at("02"));
    assert_eq!(failed.state, TriggerState::Repairing);
    store.settle_firing(row, &failed).expect("settled");
    assert_eq!(store.unfinished_repairs().expect("read").len(), 1);

    let mut repair = RepairRecord {
        tries: 1,
        branch: Some("armada/repair-1".to_string()),
        files: vec!["src/a.rs".to_string(), "Cargo.toml".to_string()],
        settled_at: Some(at("03")),
        ..RepairRecord::default()
    };
    store
        .settle_repair(row, TriggerState::FixReady, &repair, None)
        .expect("kept");
    drop(store);
    let mut store = open(&dir);
    let read = store.firings_with_ids(&id).expect("read");
    assert_eq!((read[0].0, read[0].1.state), (row, TriggerState::FixReady));
    assert_eq!(read[0].1.repair, repair);
    assert!(store.unfinished_repairs().expect("read").is_empty());
    let waiting = store.repairs_waiting_on_a_person().expect("read");
    assert_eq!(waiting.len(), 1, "a fix with no choice waits on him");

    repair.choice = Some(FixChoice::NewPr);
    store
        .settle_repair(row, TriggerState::FixReady, &repair, None)
        .expect("kept");
    assert!(store
        .repairs_waiting_on_a_person()
        .expect("read")
        .is_empty());
    assert_eq!(store.chosen_fixes().expect("read").len(), 1);

    repair.pull_request = Some("https://forge/pull/9".to_string());
    store
        .settle_repair(row, TriggerState::Passed, &repair, Some(&at("04")))
        .expect("kept");
    assert_eq!(store.trigger_firings(&id).expect("read")[0].repair, repair);

    store
        .settle_repair(row, TriggerState::Failed, &repair, Some(&at("05")))
        .expect("kept");
    let waiting = store.repairs_waiting_on_a_person().expect("read");
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].0, id);

    // A later firing of the same Trigger that passes clears it.
    let later = TriggerFiring::running(&one, at("06"));
    let again = store.open_firing(&id, &later).expect("opened");
    store
        .settle_firing(again, &later.ended(Some(0), at("07")))
        .expect("settled");
    assert!(store
        .repairs_waiting_on_a_person()
        .expect("read")
        .is_empty());

    // So does giving the Job's disk back.
    store
        .settle_firing(
            again,
            &TriggerFiring::running(&one, at("08")).ended(Some(1), at("09")),
        )
        .expect("settled");
    store
        .settle_repair(
            again,
            TriggerState::Failed,
            &RepairRecord {
                tries: 2,
                ..RepairRecord::default()
            },
            Some(&at("10")),
        )
        .expect("kept");
    assert_eq!(store.repairs_waiting_on_a_person().expect("read").len(), 1);
    store.retain_job(&id, &at("11")).expect("retained");
    assert!(store
        .repairs_waiting_on_a_person()
        .expect("read")
        .is_empty());
}

#[test]
fn a_hold_is_listed_until_it_is_settled_and_a_released_one_is_passed_by_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01HOLDING"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01HOLDING");
    let at = |s: &str| Timestamp::from_rfc3339(format!("2026-10-07T09:00:{s}.000Z"));
    let mut one = frozen(
        "begin",
        TriggerResolution::Command {
            name: "deploy_qa".to_string(),
            asks_first: false,
        },
    );
    one.when = TriggerWhen::StepStarts;
    let held = TriggerFiring::running(&one, at("01")).ended(Some(1), at("02"));
    assert_eq!(held.state, TriggerState::Held);
    store.open_firing(&id, &held).expect("opened");

    let holding = store.holding_firings(&id).expect("read");
    assert_eq!(holding.len(), 1);
    assert_eq!(holding[0].1.state, TriggerState::Held);
    assert_eq!(store.alerting_firings(&id).expect("read").len(), 1);

    // A later firing of the same Trigger at the same place supersedes it.
    let passed = TriggerFiring::running(&one, at("03")).ended(Some(0), at("04"));
    store.open_firing(&id, &passed).expect("opened");
    assert!(store.holding_firings(&id).expect("read").is_empty());
}

#[test]
fn skipping_a_hold_keeps_who_and_the_next_entry_passes_it_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01SKIPPED"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01SKIPPED");
    let at = |s: &str| Timestamp::from_rfc3339(format!("2026-10-07T09:00:{s}.000Z"));
    let mut one = frozen(
        "begin",
        TriggerResolution::Command {
            name: "deploy_qa".to_string(),
            asks_first: false,
        },
    );
    one.when = TriggerWhen::StepStarts;
    let held = TriggerFiring::running(&one, at("01")).ended(Some(1), at("02"));
    let firing = store.open_firing(&id, &held).expect("opened");

    let skipped = held.skipped_by_the_owner(at("05"));
    store.settle_hold(firing, &skipped, true).expect("settled");
    drop(store);

    let mut store = open(&dir);
    let read = store.trigger_firings(&id).expect("read");
    assert_eq!(read[0].state, TriggerState::Skipped);
    assert_eq!(read[0].skipped, Some(TriggerSkipped::ByOwner));
    assert!(store.holding_firings(&id).expect("read").is_empty());
    let step = StepId::new("handoff");
    assert!(store
        .take_released_hold(&id, TriggerWhen::StepStarts, &step)
        .expect("taken"));
    assert!(
        !store
            .take_released_hold(&id, TriggerWhen::StepStarts, &step)
            .expect("taken"),
        "the entry that asked is the one that passed it"
    );
    assert!(!store
        .take_released_hold(&id, TriggerWhen::StepPasses, &step)
        .expect("taken"));
}
