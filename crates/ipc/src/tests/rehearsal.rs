//! A person's run, as the stream and the run history spell it.

use core_model::Timestamp;

use crate::{
    decode, encode, ChangeKind, ChangedFile, CheckoutRunDiff, CheckoutRunMessage,
    CheckoutRunOpened, CheckoutRunRecord, Cursor, Delivered, DiffAgainst, Event, Instant, Missed,
    OutputClosed, OutputEnded, OutputLines, ProtocolId, RunDiffReading, StreamMessage,
};

fn at(text: &str) -> Instant {
    Instant::from(&Timestamp::from_rfc3339(text))
}

/// #1064: a declared seed is spelled by `state`, as a Verify step is.
#[test]
fn a_declared_seed_is_spelled_by_state() {
    use crate::{DeclaredSeed, SeedWarmth};

    let declared = DeclaredSeed {
        paths: vec!["target".to_string()],
        warmed_by: vec!["warm_build".to_string()],
        warmth: SeedWarmth::Warming {
            commit: "a787".to_string(),
        },
    };
    let spelled = encode(&declared).expect("plain data");
    assert_eq!(
        spelled,
        r#"{"paths":["target"],"warmed_by":["warm_build"],"warmth":{"state":"warming","commit":"a787"}}"#
    );
    assert_eq!(
        decode::<DeclaredSeed>("a seed", spelled.as_bytes()).expect("reads back"),
        declared
    );
}

fn a_record() -> CheckoutRunRecord {
    CheckoutRunRecord {
        id: "01RUN".to_string(),
        name: "fmt".to_string(),
        workspace: None,
        command: "cargo fmt --all".to_string(),
        required: Vec::new(),
        started_at: at("2026-09-11T10:00:00.000Z"),
        ended_at: at("2026-09-11T10:00:04.000Z"),
        duration_ms: 4_000,
        exit_code: Some(0),
        expect_exit_code: 0,
        ended: "passed".to_string(),
        stopped: false,
        changed: vec![ChangedFile {
            path: "src/lib.rs".to_string(),
            change: ChangeKind::Modified,
        }],
        changed_unreadable: None,
        undoable: true,
        undone_at: None,
        log: ".armada/runs/main/01RUN/output.log".to_string(),
        requester: crate::Requester::outside(),
    }
}

fn delivered(event: Event) -> String {
    encode(&StreamMessage::Event(Delivered {
        cursor: Cursor::at(3),
        event,
    }))
    .expect("plain data")
}

/// The checkout run socket speaks the four messages the other sockets do.
#[test]
fn a_runs_socket_speaks_the_four_messages_the_other_sockets_do() {
    let spelled = |message: CheckoutRunMessage| encode(&message).expect("plain data");
    let opened = spelled(CheckoutRunMessage::Opened(CheckoutRunOpened {
        protocol_id: ProtocolId::current(),
        id: "01RUN".to_string(),
        name: "test".to_string(),
        path: ".armada/runs/main/01RUN/output.log".to_string(),
        live: true,
        skipped: 0,
    }));
    assert!(opened.contains("\"message\":\"opened\""), "{opened}");
    let lines = spelled(CheckoutRunMessage::Lines(OutputLines {
        lines: vec!["Compiling ipc".to_string()],
    }));
    assert!(lines.contains("\"message\":\"lines\""), "{lines}");
    let missed = spelled(CheckoutRunMessage::Missed(Missed { dropped: 4 }));
    assert!(missed.contains("\"dropped\":4"), "{missed}");
    let closed = spelled(CheckoutRunMessage::Closed(OutputClosed {
        because: OutputEnded::Finished,
    }));
    assert!(closed.contains("\"because\":\"finished\""), "{closed}");
}

/// `/events` carries a run's end and never its output.
#[test]
fn a_runs_end_travels_under_the_name_the_inventory_declares() {
    let finished = delivered(Event::CheckoutRunFinished(a_record()));
    assert!(
        finished.contains("\"kind\":\"checkout_run.finished\""),
        "{finished}"
    );
    let back: StreamMessage = decode("a stream message", finished.as_bytes()).expect("it reads");
    let StreamMessage::Event(Delivered {
        event: Event::CheckoutRunFinished(record),
        ..
    }) = back
    else {
        panic!("not the end of a run");
    };
    assert_eq!(record, a_record());
}

/// A checkout run's diff says what it was measured against, and a snapshot
/// that is gone is its own state rather than an empty patch.
#[test]
fn a_checkout_runs_diff_names_its_base_and_a_gone_snapshot_is_a_state() {
    let read = CheckoutRunDiff {
        id: "01RUN".to_string(),
        against: DiffAgainst::RunSnapshot,
        reading: RunDiffReading::Read {
            files: a_record().changed,
            patch: Some("+formatted\n".to_string()),
        },
    };
    let json = encode(&read).expect("plain data");
    assert!(json.contains("\"against\":\"run_snapshot\""), "{json}");
    assert!(json.contains("\"state\":\"read\""), "{json}");
    let back: CheckoutRunDiff = decode("a run's diff", json.as_bytes()).expect("it reads");
    assert_eq!(back, read);

    let gone = CheckoutRunDiff {
        reading: RunDiffReading::Gone {
            why: "no snapshot".to_string(),
        },
        ..read
    };
    let json = encode(&gone).expect("plain data");
    assert!(json.contains("\"state\":\"gone\""), "{json}");
    assert!(!json.contains("\"patch\""), "{json}");
    let back: CheckoutRunDiff = decode("a run's diff", json.as_bytes()).expect("it reads");
    assert_eq!(back, gone);
}

/// **A Verify step spells its state beside its name**, one level deep, and
/// reads back whichever state it is in.
#[test]
fn a_verify_step_carries_its_state_flat_and_reads_back() {
    use crate::{CheckoutVerify, VerifyGroup, VerifyStep, VerifyStepState};
    let verify = CheckoutVerify {
        id: "01VERIFY".to_string(),
        started_at: at("2026-09-12T10:00:00.000Z"),
        ended_at: None,
        workspace: Some("apps/web".to_string()),
        requester: crate::Requester::outside(),
        steps: vec![
            VerifyStep {
                group: VerifyGroup::Setup,
                name: "bootstrap".to_string(),
                run: "pnpm install".to_string(),
                state: VerifyStepState::Running {
                    run_id: "01RUN".to_string(),
                },
            },
            VerifyStep {
                group: VerifyGroup::Checks,
                name: "test".to_string(),
                run: "cargo test".to_string(),
                state: VerifyStepState::Waiting,
            },
            VerifyStep {
                group: VerifyGroup::Checks,
                name: "lint".to_string(),
                run: "cargo clippy".to_string(),
                state: VerifyStepState::NotRun {
                    why: "Verify was stopped during `test`".to_string(),
                },
            },
        ],
    };
    let sent = encode(&verify).expect("encodes");
    assert!(sent.contains(r#""group":"setup","name":"bootstrap","run":"pnpm install","state":"running","run_id":"01RUN""#), "{sent}");
    assert!(sent.contains(r#""state":"waiting""#), "{sent}");
    let back: CheckoutVerify = decode("a Verify", sent.as_bytes()).expect("reads back");
    assert_eq!(back, verify);
}

/// **A record written before 23.40 reads as outside**, which is what it
/// was: nothing in Armada asked for it.
#[test]
fn a_run_record_from_before_the_requester_reads_as_outside() {
    let mut value: serde_json::Value =
        serde_json::from_str(&encode(&a_record()).expect("plain data")).expect("json");
    value
        .as_object_mut()
        .expect("an object")
        .remove("requester");
    let older = value.to_string();
    let read: CheckoutRunRecord = decode("an older run", older.as_bytes()).expect("it reads");
    assert_eq!(read.requester, crate::Requester::outside());
    assert!(encode(&read)
        .expect("plain data")
        .contains(r#""requester":{"kind":"outside"}"#));
}
