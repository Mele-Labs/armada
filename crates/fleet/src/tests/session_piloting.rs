//! A session started on a Job a person takes over: the worktree handed to it,
//! the handoff row first in its thread, the bundle ahead of the person's first
//! message, and what closing it or ending the pilot does. Since 23.51.
//!
//! The stand-in process is `session_host`'s, so nothing here needs an agent.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal};
use core_model::{JobId, JobStatus};
use ipc::{
    CloseSession, GateAnswer, ManifestId, PilotFrom, PilotOutcome, SendSessionMessage, SessionGate,
    SessionId, SessionRecord, SessionRow, StartSession,
};
use store::{AttachmentState, Holder};

use crate::session_host::Heard;
use crate::tests::daemon::a_fleet;
use crate::tests::piloting::{changed, running, Fixture};
use crate::tests::session_host::{Shared, StandIn};
use crate::tests::tmp::TempDir;

struct Rig {
    fleet: Arc<Fixture>,
    stand_in: Arc<StandIn>,
    manifest: ManifestId,
    root: String,
    home: TempDir,
}

fn rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet(&home, changed()).hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    let one = fleet.repositories().first().expect("a served repository");
    let manifest = ManifestId::carried(one.manifest().id().as_str());
    let root = one.root().to_string();
    Rig {
        fleet: Arc::new(fleet),
        stand_in,
        manifest,
        root,
        home,
    }
}

impl Rig {
    /// A Job whose Drone was ended, which is the state a person most often
    /// takes over from.
    async fn escalated(&self) -> JobId {
        let id = running(&self.fleet, &self.home, "the reader drops lines").await;
        self.fleet.kill_drone(&id).await.expect("its Drone ends");
        id
    }

    async fn pilot(&self, job: &JobId) -> Result<SessionRecord, Refusal> {
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest.clone(),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: Some(PilotFrom {
                    job_id: ipc::JobId::from(job),
                    outcome: PilotOutcome::TakeOver,
                }),
                fork: None,
            })
            .await
    }

    async fn send(&self, id: &SessionId, text: &str) {
        Arc::clone(&self.fleet)
            .send_session_message(SendSessionMessage {
                session_id: id.clone(),
                text: text.into(),
                attachments: Vec::new(),
                mentions: Vec::new(),
            })
            .await
            .expect("taken");
    }

    async fn rows(&self, id: &SessionId) -> Vec<SessionRow> {
        Arc::clone(&self.fleet).get_session(id.clone()).await.expect("read").rows
    }

    async fn worktree_of(&self, job: &JobId) -> String {
        let bundle = self
            .fleet
            .handoff_bundle(ipc::JobId::from(job))
            .await
            .expect("a bundle");
        bundle.worktree.expect("a worktree").path
    }

    async fn gate(&self, id: &SessionId, path: &str) -> GateAnswer {
        let call = format!(
            r#"{{"session_id":"{}","cwd":"{}","tool_name":"Write","tool_input":{{"file_path":"{path}"}}}}"#,
            id.as_str(),
            self.root
        );
        let gate: SessionGate = ipc::decode("a gate", call.as_bytes()).expect("a gate");
        self.fleet.gate_session_call(gate).await.expect("answered")
    }
}

#[tokio::test]
async fn a_session_started_on_a_job_holds_its_worktree_with_the_handoff_first() {
    let rig = rig();
    let job = rig.escalated().await;
    let record = rig.pilot(&job).await.expect("started on the Job");

    assert_eq!(
        rig.fleet.load(&job).await.unwrap().status(),
        JobStatus::Piloted
    );
    let row = rig
        .fleet
        .summarised(&rig.fleet.load(&job).await.unwrap())
        .await
        .unwrap();
    assert_eq!(
        row.piloted.and_then(|pilot| pilot.session_id).as_deref(),
        Some(record.id.as_str()),
        "the Board names the session in the worktree"
    );
    assert_eq!(
        record.title.as_deref(),
        Some("the reader drops lines"),
        "named for the Job it took"
    );
    assert!(record.hosted.is_some());

    let rows = rig.rows(&record.id).await;
    let [SessionRow::Handoff {
        job_id,
        number,
        slot,
        branch,
        reason,
        ..
    }] = rows.as_slice()
    else {
        panic!("a handoff and nothing else: {rows:?}");
    };
    assert_eq!(job_id.as_str(), job.as_str());
    assert!(*number > 0 && slot.is_some() && !branch.is_empty());
    assert_eq!(reason, "take_over");

    let store = rig.fleet.store().lock().await;
    let held = store
        .attachments_of(&Holder::session(record.id.as_str()))
        .unwrap();
    for kind in ["slot", "branch"] {
        let one = held
            .iter()
            .find(|one| one.kind == kind)
            .unwrap_or_else(|| panic!("the session holds the {kind}"));
        assert_eq!(one.state, AttachmentState::Standing);
        assert_eq!(
            one.detail.get("handed").map(String::as_str),
            Some(format!("job {}", job.as_str()).as_str())
        );
    }
    let job_row = held.iter().find(|one| one.kind == "job").expect("the Job");
    assert_eq!(job_row.target, job.as_str());
    assert!(
        !job_row.detail.contains_key("looking"),
        "its own, not one it is looking at"
    );
    assert!(
        rig.stand_in.starts().is_empty(),
        "nothing runs until the person speaks"
    );
}

#[tokio::test]
async fn the_jobs_detail_carries_its_pilot_beside_the_boards_row() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;

    let piloted = rig
        .fleet
        .job_detail(ipc::JobId::from(&job))
        .await
        .expect("a detail")
        .job
        .piloted
        .expect("the detail's own row says who is piloting");
    assert_eq!(piloted.session_id.as_deref(), Some(id.as_str()));
    assert_eq!(piloted.exit, None);

    rig.fleet
        .close_as_superseded(&job, Some("landed by hand"))
        .await
        .expect("closed");
    let ended = rig
        .fleet
        .job_detail(ipc::JobId::from(&job))
        .await
        .unwrap()
        .job
        .piloted
        .expect("and still says so once it ended, since that is how an attestation is told from a pass");
    assert_eq!(ended.exit.as_deref(), Some("superseded"));
    assert_eq!(ended.note.as_deref(), Some("landed by hand"));
}

#[tokio::test]
async fn the_first_message_starts_the_agent_in_the_worktree_with_the_bundle_ahead_of_it() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;
    let worktree = rig.worktree_of(&job).await;

    rig.send(&id, "find why the reader drops lines").await;

    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 1);
    assert_eq!(
        starts[0].directory, worktree,
        "in the Job's own checkout, not the repository's root"
    );
    let sent = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("the reader drops lines"), "{}", sent[0]);
    assert!(sent[0].contains(&worktree), "the worktree is named");
    assert!(sent[0].contains("find why the reader drops lines"));
    assert!(
        sent[0].find("taken over").unwrap() < sent[0].find("find why").unwrap(),
        "the bundle goes first"
    );

    rig.send(&id, "and then?").await;
    let sent = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 2);
    assert!(
        !sent[1].contains("taken over"),
        "the transcript carries it from here: {}",
        sent[1]
    );
}

#[tokio::test]
async fn a_write_in_the_handed_worktree_takes_no_lease_and_one_in_the_root_is_pointed_back() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;
    let worktree = rig.worktree_of(&job).await;

    let inside = rig.gate(&id, &format!("{worktree}/src/log.rs")).await;
    assert!(
        !matches!(inside, GateAnswer::Hold { .. }),
        "no lease to wait for: {inside:?}"
    );
    let outside = rig.gate(&id, &format!("{}/src/log.rs", rig.root)).await;
    assert!(
        matches!(outside, GateAnswer::Hold { .. }),
        "the main checkout stays off limits: {outside:?}"
    );
    assert!(
        !rig.rows(&id)
            .await
            .iter()
            .any(|row| matches!(row, SessionRow::Lease { .. })),
        "nothing was leased"
    );
}

#[tokio::test]
async fn a_job_that_cannot_be_taken_over_makes_no_session() {
    let rig = rig();
    let proposed = rig
        .fleet
        .propose(crate::tests::daemon::a_proposal("not yet running"))
        .await
        .expect("proposed");

    let refused = rig
        .pilot(proposed.id())
        .await
        .expect_err("a proposal is not pilotable");
    let Refusal::IllegalMove(why) = refused else {
        panic!("a 409: {refused:?}");
    };
    assert_eq!(why.code, "fleet.not_pilotable");
    assert!(
        rig.fleet.store().lock().await.hostings().unwrap().is_empty(),
        "no session was written for a take over that did not happen"
    );
}

#[tokio::test]
async fn closing_the_session_gives_the_worktree_back_to_the_job_and_commits_nothing() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;

    let closed = rig
        .fleet
        .close_session(CloseSession {
            session_id: id.clone(),
        })
        .await
        .expect("closed");

    assert_eq!(closed.state, ipc::SessionState::Ended);
    assert_eq!(
        rig.fleet.load(&job).await.unwrap().status(),
        JobStatus::Piloted,
        "the Job is still the person's to finish"
    );
    assert!(
        rig.fleet.vcs().parked_slots().is_empty(),
        "the worktree is the Job's, so it is not committed to a session's name"
    );
    let store = rig.fleet.store().lock().await;
    let session = store
        .attachments_of(&Holder::session(id.as_str()))
        .unwrap();
    assert!(session
        .iter()
        .filter(|one| one.kind == "slot" || one.kind == "branch")
        .all(|one| one.state == AttachmentState::GivenBack));
    let job_held = store.attachments_of(&Holder::job(job.as_str())).unwrap();
    assert!(
        job_held
            .iter()
            .any(|one| one.kind == "slot" && one.state == AttachmentState::Standing),
        "the Job holds its slot again"
    );
}

#[tokio::test]
async fn ending_the_pilot_frees_the_session_of_the_worktree() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;
    rig.send(&id, "look at it").await;

    rig.fleet
        .close_as_superseded(&job, Some("landed by hand"))
        .await
        .expect("closed as superseded");

    assert!(
        rig.stand_in
            .process(0)
            .ended
            .load(std::sync::atomic::Ordering::SeqCst),
        "a process standing in the Job's worktree is let go"
    );
    let hosting = rig
        .fleet
        .store()
        .lock()
        .await
        .hosting(id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(hosting.lease_slot, None);

    rig.send(&id, "carry on elsewhere").await;
    let starts = rig.stand_in.starts();
    assert_eq!(starts.len(), 2);
    assert!(starts[1].resuming);
    assert_eq!(
        starts[1].directory, rig.root,
        "back in the repository, and its next write leases a slot of its own"
    );
}

#[tokio::test]
async fn the_commands_an_agent_names_reach_the_session_and_every_other() {
    let rig = rig();
    let job = rig.escalated().await;
    let id = rig.pilot(&job).await.unwrap().id;
    rig.send(&id, "hello").await;
    rig.stand_in.hears(
        0,
        Heard::Commands(vec![String::from("compact"), String::from("review")]),
    );

    let names = || async {
        Arc::clone(&rig.fleet)
            .get_session(id.clone())
            .await
            .unwrap()
            .session
            .hosted
            .unwrap()
            .commands
    };
    for _ in 0..500 {
        if !names().await.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(names().await, vec!["compact", "review"]);

    let blank = rig
        .fleet
        .start_session(StartSession {
            manifest_id: rig.manifest.clone(),
            title: None,
            model: None,
            effort: None,
            mode: None,
            pilot: None,
            fork: None,
        })
        .await
        .unwrap();
    assert_eq!(
        blank.hosted.unwrap().commands,
        vec!["compact", "review"],
        "a session whose agent has not started offers what the last one said"
    );
}
