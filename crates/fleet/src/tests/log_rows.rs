//! A Job's log rows on its resources reading: kind, bytes, and whether a
//! writer in the Job's tree holds the file. `crate::resources::logs`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use core_model::{DroneId, Job, StepId, Timestamp, Ulid};
use ipc::{LogFile, LogKind};
use testkit::FakeWorkProduct;

use crate::asked::briefs_dir;
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder};
use crate::resources::logs::writing;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::tmp::TempDir;
use crate::transcript::transcripts_dir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const DRONE: &str = "01DRONE00000000000000001";

fn a_fleet(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&["src/log.rs"])))
}

fn records_root(fleet: &Fixture, job: &Job) -> String {
    fleet
        .served_by(job)
        .expect("the Job's repository")
        .records_root()
        .to_string()
}

/// A recorded process whose *child* opens `transcript`, writes one row, and
/// closes it once `closed` exists — staying alive in the tree either way.
fn a_drone_holding(transcript: &Path, closed: &Path) -> Child {
    use std::os::unix::process::CommandExt;
    Command::new("sh")
        .args([
            "-c",
            "( exec 3>>\"$1\"; printf 'row\\n' >&3; \
               while [ ! -e \"$2\" ]; do sleep 0.05; done; \
               exec 3>&-; exec sleep 30 ) & wait",
            "sh",
        ])
        .arg(transcript)
        .arg(closed)
        .process_group(0)
        .spawn()
        .expect("sh runs")
}

async fn record(fleet: &Fixture, job: &Job, pid: u32) {
    let Ok(Holder::Held(started)) = holder_of(pid) else {
        panic!("the spawned process is alive");
    };
    fleet
        .store()
        .lock()
        .await
        .record_drone_process(&store::DroneProcess {
            job_id: job.id().clone(),
            step_id: StepId::new("fix"),
            drone_id: DroneId::carried(Ulid::carried(DRONE.to_string())),
            pid,
            started_at: started.as_str().to_string(),
            spawned_at: Timestamp::from_rfc3339("2026-10-01T00:00:00.000Z"),
        })
        .expect("the Drone's process is recorded, as a spawn would");
}

async fn the_transcript(fleet: &Fixture, job: &Job) -> LogFile {
    let held = fleet.job_resources(job).await.expect("a reading");
    held.logs
        .into_iter()
        .find(|log| log.kind == LogKind::Transcript)
        .expect("the transcript is listed")
}

fn until(what: &str, mut done: impl FnMut() -> bool) {
    let by = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < by, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The claim: a transcript held open by a child of the recorded process reads
/// `being written`; once that child closes it — still alive, still in the
/// tree — it does not, and its bytes are measured.
#[tokio::test]
async fn a_transcript_a_child_holds_open_is_being_written_until_it_closes() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("a drone is writing"))
        .await
        .expect("a Job");
    let dir = transcripts_dir(&records_root(&fleet, &job), &job.handle());
    std::fs::create_dir_all(&dir).expect("the transcripts directory");
    let transcript = dir.join(format!("{DRONE}.jsonl"));
    let closed = home.path().join("closed");
    let mut drone = a_drone_holding(&transcript, &closed);
    until("the child opened the transcript and wrote its row", || {
        std::fs::metadata(&transcript).is_ok_and(|held| held.len() == 4)
    });
    record(&fleet, &job, drone.id()).await;

    let open = the_transcript(&fleet, &job).await;
    assert_eq!(
        open.being_written,
        Some(true),
        "a writer in the tree holds it"
    );
    assert_eq!(
        open.path,
        format!(".armada/transcripts/{}/{DRONE}.jsonl", job.handle())
    );

    std::fs::write(&closed, "").expect("tell the child to close it");
    let by = Instant::now() + Duration::from_secs(10);
    let shut = loop {
        let now = the_transcript(&fleet, &job).await;
        if now.being_written == Some(false) || Instant::now() > by {
            break now;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", drone.id())])
        .status();
    let _ = drone.wait();

    assert_eq!(
        shut.being_written,
        Some(false),
        "the process is still in the tree and no longer holds the file"
    );
    assert_eq!(shut.bytes, Some(4), "measured, `row\\n`");
}

/// The claim: a file Fleet lists and cannot `stat` carries no size and no
/// writer answer, rather than a zero or a `false`; a kept brief beside it is
/// measured and held by nobody.
#[tokio::test]
async fn a_log_that_cannot_be_measured_has_no_size() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("a transcript went missing"))
        .await
        .expect("a Job");
    let root = records_root(&fleet, &job);
    let dir = transcripts_dir(&root, &job.handle());
    std::fs::create_dir_all(&dir).expect("the transcripts directory");
    let dangling: PathBuf = dir.join(format!("{DRONE}.jsonl"));
    std::os::unix::fs::symlink(home.path().join("nowhere"), &dangling).expect("a link to nothing");
    let briefs = briefs_dir(&root, &job.handle());
    std::fs::create_dir_all(&briefs).expect("the briefs directory");
    std::fs::write(briefs.join("fix-1-tests.txt"), "the question").expect("a kept brief");

    let held = fleet.job_resources(&job).await.expect("a reading");

    let transcript = held
        .logs
        .iter()
        .find(|log| log.kind == LogKind::Transcript)
        .expect("listed even though it will not stat");
    assert_eq!(transcript.bytes, None, "absent, never zero");
    assert_eq!(transcript.being_written, None, "nothing was learned");
    let brief = held
        .logs
        .iter()
        .find(|log| log.kind == LogKind::Brief)
        .expect("the kept brief");
    assert_eq!(brief.bytes, Some(12));
    assert_eq!(brief.being_written, Some(false));
}

/// `lsof -w -a -p <pids> -Fai -- <files>` as it prints on darwin: one process
/// holding one file for writing and one for reading.
#[test]
fn only_a_file_open_for_writing_is_being_written() {
    let said = "p23649\nf3\naw\ni123260191\nf4\nar\ni123260192\np501\nf9\nau\ni7\n";

    let held = writing(said);

    assert!(held.contains(&123_260_191) && held.contains(&7));
    assert!(!held.contains(&123_260_192), "a reader is not a writer");
}
