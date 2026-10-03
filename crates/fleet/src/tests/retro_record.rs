//! A retro's record, read off a real Job. `docs/concepts/retro.md`.
//!
//! **The fixture is Job 3 on the owner's Fleet**, captured read-only on
//! 3 Oct 2026: `get_job`, `get_job_events` and `get_evidence` as they answered,
//! the transcript rows a record reads from its two Drones that have any, and
//! the log lines a record pairs. Its record holds an `out_of_bounds` failure
//! the gate's own table never kept, a Judge `not_met`, a person agreeing with
//! it and then restarting the step, a command a person was asked to allow, and
//! a Drone saying it never saw a test pass. Each is asserted below.

use std::fs;

use core_model::JobId;
use ipc::{CheckRunBy, DroneId, JobDetail, JobEvidence, JobHistory, Saw, TranscriptRow, Voice};

use crate::retro::gathering::{annotations_on, transcripts_of};
use crate::retro::record::{assembled, cites, Sources};
use crate::tests::tmp::TempDir;
use crate::transcript::{log_of, transcript_of};

const HANDLE: &str = "3-make-branch-inputs-searchable-comboboxes-w";
const JOB: &str = "01M3ZJHFA30023EPTPC0MM1N8D";
const PLANNED_BY: &str = "01M3ZJMDQ1004E5TZ5WNM5H4E1";
const IMPLEMENTED_BY: &str = "01M3ZXV34M0050JPQD7YFE1WG8";

fn decoded<T: serde::de::DeserializeOwned>(what: &'static str, json: &str) -> T {
    ipc::decode(what, json.as_bytes()).unwrap_or_else(|why| panic!("{what}: {why}"))
}

/// Job 3's records laid out where Fleet keeps them, under `home`.
fn laid_out(home: &TempDir) -> String {
    let root = home.path().to_string_lossy().to_string();
    let put = |path: std::path::PathBuf, text: &str| {
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(path, text).expect("written");
    };
    put(log_of(&root, HANDLE), include_str!("retro_job_3/log.jsonl"));
    for (drone, rows) in [
        (
            PLANNED_BY,
            include_str!("retro_job_3/01M3ZJMDQ1004E5TZ5WNM5H4E1.jsonl"),
        ),
        (
            IMPLEMENTED_BY,
            include_str!("retro_job_3/01M3ZXV34M0050JPQD7YFE1WG8.jsonl"),
        ),
    ] {
        put(
            transcript_of(&root, HANDLE, &ipc::DroneId::carried(drone).to_domain()),
            rows,
        );
    }
    root
}

fn job_3(home: &TempDir) -> ipc::RetroRecord {
    let root = laid_out(home);
    let detail: JobDetail = decoded("Job 3's detail", include_str!("retro_job_3/detail.json"));
    let history: JobHistory = decoded("Job 3's history", include_str!("retro_job_3/events.json"));
    let evidence: JobEvidence = decoded(
        "Job 3's evidence",
        include_str!("retro_job_3/evidence.json"),
    );
    let drones = [
        DroneId::carried(PLANNED_BY),
        DroneId::carried(IMPLEMENTED_BY),
    ];
    let transcripts = transcripts_of(&root, HANDLE, &drones);
    let log = crate::journal::read_from(&root, HANDLE, 0).notes;
    assembled(&Sources {
        detail: &detail,
        history: &history,
        evidence: &evidence,
        transcripts: &transcripts,
        log: &log,
        notes: &[],
    })
}

/// **What the gate's own table never kept is still on the record.** The plan
/// step's first run failed `out_of_bounds` for reaching `armada.yml`, which only
/// the transcript holds; the three `desktop_test` failures are the table's.
#[test]
fn job_3s_failed_checks_include_the_one_only_its_transcript_kept() {
    let home = TempDir::new();
    let record = job_3(&home);

    let bounds = record
        .failed_checks
        .iter()
        .find(|check| check.name == "out_of_bounds")
        .expect("the out_of_bounds failure");
    assert_eq!(bounds.step.as_ref().map(|step| step.as_str()), Some("plan"));
    assert_eq!(bounds.run, CheckRunBy::Gate);
    assert!(
        bounds
            .produced
            .as_deref()
            .is_some_and(|said| said.contains("armada.yml")),
        "{bounds:?}"
    );
    let desktop: Vec<Option<u32>> = record
        .failed_checks
        .iter()
        .filter(|check| check.name == "desktop_test" && check.run == CheckRunBy::Gate)
        .map(|check| check.attempt)
        .collect();
    assert_eq!(desktop, vec![Some(1), Some(2), Some(3)], "each once");
    assert!(
        record
            .failed_checks
            .iter()
            .any(|check| check.run == CheckRunBy::Drone),
        "and the Drone's own runs, off the log: {:?}",
        record.failed_checks
    );
}

/// A Judge's `not_met`, a person asked about it and agreeing, and the restart
/// after it with what they said — the three in the order they happened.
#[test]
fn job_3s_judge_refusal_the_answer_and_the_restart_are_each_on_the_record() {
    let home = TempDir::new();
    let record = job_3(&home);

    let refused = &record.not_met[0];
    assert_eq!(refused.criterion, "addresses_the_request");
    assert_eq!(refused.step.as_str(), "plan");
    assert_eq!(refused.attempt, 1);

    let judge = record
        .asked
        .iter()
        .find(|asked| asked.about.contains("addresses_the_request"))
        .expect("the judge question");
    assert_eq!(
        judge.waited_ms,
        Some(77_322),
        "03:36:56.829 to 03:38:14.151"
    );
    assert!(judge
        .answer
        .as_deref()
        .is_some_and(|said| said.contains("agreed")));

    let restarted = record
        .restarts
        .iter()
        .find(|act| act.said.is_some())
        .expect("a restart with what the person said");
    assert!(restarted
        .said
        .as_deref()
        .is_some_and(|said| said.contains("Base branch should also offer creating a new branch")));
}

/// **The command a person was asked to allow, and how long the Drone waited.**
#[test]
fn job_3s_command_ask_is_paired_with_its_answer() {
    let home = TempDir::new();
    let record = job_3(&home);

    let command = record
        .asked
        .iter()
        .find(|asked| asked.about.contains("grep -a -c"))
        .expect("the command ask");
    assert_eq!(command.answer.as_deref(), Some("allow for this job"));
    assert_eq!(command.waited_ms, Some(30_198));
}

/// **What the Drone said after its submission is on the record**: it never saw
/// the test it changed pass, and no field of the submission carried that.
#[test]
fn job_3s_drone_saying_it_never_saw_the_test_pass_is_on_the_record() {
    let home = TempDir::new();
    let record = job_3(&home);

    assert!(
        record
            .said_after
            .iter()
            .any(|said| said.said.contains("never saw the arc-dispatch test pass")),
        "{:?}",
        record.said_after
    );
    assert!(record
        .not_done
        .iter()
        .any(|said| said.step.as_ref().map(|s| s.as_str()) == Some("plan")));
}

/// The waits at `awaiting_*` and the acts a person made, and **every row's
/// cite is unique**, since a retro item names rows by it.
#[test]
fn job_3s_waits_and_acts_are_counted_and_every_cite_is_unique() {
    let home = TempDir::new();
    let record = job_3(&home);

    let review = record
        .waited
        .iter()
        .find(|waited| waited.status.as_wire() == "awaiting_review")
        .expect("the wait for the person's answer");
    assert!(review.ms.is_some_and(|ms| ms > 70_000), "{review:?}");
    assert!(
        record
            .waited
            .last()
            .is_some_and(|last| last.until.is_none()),
        "the Job is still awaiting repair: {:?}",
        record.waited
    );
    assert!(record.acts.iter().all(|act| act.actor.as_wire() == "human"));
    assert!(!record.acts.is_empty());

    let counted = record.failed_checks.len()
        + record.not_met.len()
        + record.not_done.len()
        + record.said_after.len()
        + record.restarts.len()
        + record.asked.len()
        + record.waited.len()
        + record.acts.len();
    assert_eq!(cites(&record).len(), counted);
}

/// **A refusal names the tool; the call it answered names the argument.** The
/// join `scripts/job` was written for.
#[test]
fn a_refusal_is_joined_to_what_the_drone_tried() {
    let row = |saw: Saw| TranscriptRow {
        ts: ipc::Instant::carried("2026-10-03T03:51:40.000Z"),
        step: Some(ipc::StepId::carried("implement")),
        by: Voice::Drone,
        drone_id: None,
        saw,
    };
    let transcripts = vec![vec![
        row(Saw::Called {
            tool: "Bash".to_string(),
            call: "toolu_1".to_string(),
            detail: "grep -a FAIL implement.1.dry.0.log".to_string(),
            truncated: false,
            detail_length: None,
            whole: None,
        }),
        row(Saw::Refused {
            tool: "Bash".to_string(),
            call: "toolu_1".to_string(),
            because: String::new(),
        }),
    ]];
    let detail: JobDetail = decoded("Job 3's detail", include_str!("retro_job_3/detail.json"));
    let history: JobHistory = decoded("Job 3's history", include_str!("retro_job_3/events.json"));
    let evidence: JobEvidence = decoded(
        "Job 3's evidence",
        include_str!("retro_job_3/evidence.json"),
    );

    let record = assembled(&Sources {
        detail: &detail,
        history: &history,
        evidence: &evidence,
        transcripts: &transcripts,
        log: &[],
        notes: &[],
    });

    let [refused] = record.refusals.as_slice() else {
        panic!("one refusal: {:?}", record.refusals);
    };
    assert_eq!(refused.cite, "refusal:1");
    assert_eq!(
        refused.tried.as_deref(),
        Some("grep -a FAIL implement.1.dry.0.log")
    );
    assert_eq!(refused.because, None, "empty is the harness saying nothing");
}

/// **An annotation is linked by the Job it names, and by nothing else.** A
/// note left at the same minute with no Job named is not this Job's.
#[test]
fn an_annotation_is_linked_by_the_job_it_names_and_by_nothing_else() {
    let home = TempDir::new();
    let dir = home.path().join(".armada").join("annotations");
    fs::create_dir_all(&dir).expect("a directory");
    let note = |id: &str, open: Option<&str>| {
        let open = open
            .map(|job| format!(r#","openJobId":"{job}""#))
            .unwrap_or_default();
        format!(
            r#"{{"id":"{id}","status":"open","text":"which part is it on","screen":"Job",
                "selector":"button.armada-wf-card","createdAt":"2026-10-03T03:57:21.464Z"{open}}}"#
        )
    };
    fs::write(dir.join("a.json"), note("a", Some(JOB))).expect("written");
    fs::write(dir.join("b.json"), note("b", None)).expect("written");
    fs::write(dir.join("c.json"), note("c", Some("01SOMEOTHERJOB"))).expect("written");

    let linked = annotations_on(
        &home.path().to_string_lossy(),
        &JobId::carried(core_model::Ulid::carried(JOB)),
    );

    let ids: Vec<&str> = linked.iter().map(|note| note.id.as_str()).collect();
    assert_eq!(ids, vec!["a"]);
    assert_eq!(linked[0].screen.as_deref(), Some("Job"));
}
