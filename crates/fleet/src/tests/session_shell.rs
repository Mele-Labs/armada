//! A shell action run from a flight control, through a real Fleet and a real shell, in a slot
//! worktree the ledger says the Session holds. `docs/concepts/kit.md`.

use std::sync::Arc;
use std::time::Duration;

use api::Sessions;
use ipc::{RunSessionShell, SessionFact, SessionId, SessionReport};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

type Running = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const SLOT: u32 = 3;

struct Rig {
    fleet: Arc<Running>,
    session: SessionId,
    /// Where the slot's worktree is, whether or not it has been made.
    slot: std::path::PathBuf,
    _home: TempDir,
}

async fn rig() -> Rig {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&[])));
    let session = SessionId::carried("s1");
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: session.clone(),
            fact: SessionFact::Started {
                cwd: home.path().to_string_lossy().into(),
                title: None,
                origin: ipc::SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("kept");
    let root = fleet.repositories().first().expect("a repository").root().to_string();
    let slot = adapter_traits::slot_path(&root, SLOT).into();
    Rig { fleet, session, slot, _home: home }
}

impl Rig {
    /// The ledger says the Session holds the slot.
    async fn holds_the_slot(&self) {
        std::fs::create_dir_all(&self.slot).expect("a worktree");
        let at = self.fleet.now().as_str().to_string();
        let manifest = self.fleet.repositories().first().unwrap().manifest().id().as_str().to_string();
        let row = KeptAttachment {
            holder: Holder::session(self.session.as_str()),
            kind: "slot".into(),
            manifest_id: manifest,
            target: SLOT.to_string(),
            state: AttachmentState::Standing,
            detail: Default::default(),
            since: at.clone(),
            changed_at: at,
        };
        self.fleet.store().lock().await.attach(&row, false).unwrap();
    }

    fn run(&self, command: &str) -> RunSessionShell {
        RunSessionShell { session_id: self.session.clone(), command: command.into() }
    }

    async fn ran(&self, command: &str) -> ipc::SessionShellRan {
        Arc::clone(&self.fleet).run_session_shell(self.run(command)).await.expect("ran")
    }
}

fn said(refusal: api::Refusal) -> String {
    match refusal {
        api::Refusal::Unacceptable(error) => error.message,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// **A Session with no slot is refused, and nothing runs**: its directory is a checkout, and a
/// tile pressed with nothing to press it on must not touch one.
#[tokio::test]
async fn a_session_that_holds_no_slot_is_refused_and_nothing_runs() {
    let rig = rig().await;
    let marker = rig._home.path().join("ran");

    let refused = Arc::clone(&rig.fleet)
        .run_session_shell(rig.run(&format!("touch {}", marker.display())))
        .await
        .expect_err("no slot");

    assert!(said(refused).contains("no worktree slot"));
    assert!(!marker.exists(), "the command did not run in the Session's directory or anywhere");

    let nobody = RunSessionShell { session_id: SessionId::carried("nobody"), command: "true".into() };
    assert!(Arc::clone(&rig.fleet).run_session_shell(nobody).await.is_err());
    rig.holds_the_slot().await;
    assert!(Arc::clone(&rig.fleet).run_session_shell(rig.run("  ")).await.is_err(), "a blank line");
}

#[tokio::test]
async fn a_line_runs_in_the_slot_and_answers_its_output_and_exit() {
    let rig = rig().await;
    rig.holds_the_slot().await;

    let ran = rig.ran("echo out; echo oops >&2; pwd; exit 3").await;

    assert_eq!(ran.exit, Some(3));
    assert!(!ran.timed_out && !ran.truncated);
    let here = std::fs::canonicalize(&rig.slot).unwrap();
    assert!(ran.output.contains(&*here.to_string_lossy()), "ran in the slot: {:?}", ran.output);
    let (out, oops) = (ran.output.find("out").unwrap(), ran.output.find("oops").unwrap());
    assert!(out < oops, "one stream, in the order printed: {:?}", ran.output);

    assert_eq!(rig.ran("true").await.exit, Some(0));
}

#[tokio::test]
async fn output_past_the_cap_keeps_its_end() {
    let rig = rig().await;
    rig.holds_the_slot().await;

    let ran = rig.ran("i=0; while [ $i -lt 20000 ]; do echo line-$i; i=$((i+1)); done").await;

    assert!(ran.truncated);
    assert!(ran.output.len() <= crate::session_shell::TAIL);
    assert!(ran.output.trim_end().ends_with("line-19999"), "the end is what is kept");
    assert!(!ran.output.contains("line-0\n"), "the start is what is dropped");
}

#[tokio::test]
async fn a_line_that_runs_too_long_is_stopped() {
    let rig = rig().await;
    rig.holds_the_slot().await;

    let ran = Arc::clone(&rig.fleet)
        .shell_action_run_within(rig.run("echo begun; sleep 30"), Duration::from_secs(1))
        .await
        .expect("answered");

    assert!(ran.timed_out);
    assert_eq!(ran.exit, None);
    assert!(ran.output.contains("begun"));
}
