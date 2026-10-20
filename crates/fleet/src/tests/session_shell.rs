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
        RunSessionShell { session_id: self.session.clone(), command: command.into(), repository_tile: None }
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

    let nobody = RunSessionShell { session_id: SessionId::carried("nobody"), command: "true".into(), repository_tile: None };
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

/// A shell tile the repository committed asks before it runs on this Mac, and asks again when its
/// command changes. A person's own tile never asks.
mod asking {
    use super::*;
    use ipc::{ApproveRepositoryTile, RepositoryTile, TileAsk};

    fn root(rig: &Rig) -> String {
        rig.fleet.repositories().first().expect("a repository").root().to_string()
    }

    /// The repository commits a shell tile `specs` running `command`.
    fn commits(rig: &Rig, command: &str) {
        let file = std::path::Path::new(&root(rig)).join(".armada/flight-controls.toml");
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
        std::fs::write(&file, format!("version = 1\n[[job]]\nid = \"specs\"\nkind = \"shell\"\ntext = {command:?}\n")).expect("written");
    }

    fn pressed(rig: &Rig, command: &str) -> RunSessionShell {
        let repository_tile = Some(RepositoryTile { repository: root(rig), id: "specs".into() });
        RunSessionShell { repository_tile, ..rig.run(command) }
    }

    async fn asked(rig: &Rig) -> TileAsk {
        let read = rig.fleet.repository_flight_controls_read(root(rig), None).await.expect("read");
        assert_eq!(read.asking.len(), 1);
        read.asking.into_iter().next().expect("one")
    }

    fn approving(rig: &Rig, command_hash: &str) -> ApproveRepositoryTile {
        ApproveRepositoryTile { repository: root(rig), session_id: None, tile_id: "specs".into(), command_hash: command_hash.into() }
    }

    #[tokio::test]
    async fn a_repository_tile_that_is_not_approved_is_refused_with_its_command_and_nothing_runs() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        let marker = rig._home.path().join("ran");
        let command = format!("touch {}", marker.display());
        commits(&rig, &command);
        assert!(!asked(&rig).await.approved, "the listing says it has not been approved");

        let api::Refusal::Unacceptable(error) = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, &command)).await.expect_err("asks") else {
            panic!("a 422");
        };

        assert_eq!(error.code, "fleet.tile_not_approved");
        assert_eq!(error.fields.get("command"), Some(&ipc::WireValue::Str(command)));
        assert_eq!(error.fields.get("command_hash"), Some(&ipc::WireValue::Str(asked(&rig).await.command_hash)));
        assert!(!marker.exists(), "nothing ran");
    }

    #[tokio::test]
    async fn an_approved_repository_tile_runs_and_the_listing_says_so() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        commits(&rig, "echo approved");
        let ask = asked(&rig).await;

        let after = rig.fleet.repository_tile_approved(approving(&rig, &ask.command_hash)).await.expect("approved");
        assert!(after.asking[0].approved);

        let ran = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, "echo approved")).await.expect("runs");
        assert_eq!((ran.exit, ran.output.trim()), (Some(0), "approved"));
        let kept = rig._home.path().join("kit/approved-tiles");
        assert!(kept.is_dir(), "kept in Kit's folder on this Mac");
    }

    #[tokio::test]
    async fn a_press_runs_the_repositorys_command_never_the_one_it_sent() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        let marker = rig._home.path().join("sent-ran");
        let sent = format!("touch {}", marker.display());
        commits(&rig, "echo committed");

        let api::Refusal::Unacceptable(error) = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, &sent)).await.expect_err("asks") else {
            panic!("a 422");
        };
        assert_eq!(error.code, "fleet.tile_not_approved");
        assert_eq!(error.fields.get("command"), Some(&ipc::WireValue::Str("echo committed".into())));
        assert!(error.message.contains("echo committed") && !error.message.contains(&sent), "the refusal shows the repository's command");

        let ask = asked(&rig).await;
        rig.fleet.repository_tile_approved(approving(&rig, &ask.command_hash)).await.expect("approved");
        let ran = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, &sent)).await.expect("runs");

        assert_eq!((ran.exit, ran.output.trim()), (Some(0), "committed"));
        assert!(!marker.exists(), "the sent command never ran");
    }

    #[tokio::test]
    async fn a_tile_the_repository_does_not_hold_is_refused_and_nothing_runs() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        let marker = rig._home.path().join("ran");
        let sent = format!("touch {}", marker.display());
        commits(&rig, "echo committed");
        let missing = Some(RepositoryTile { repository: root(&rig), id: "other".into() });

        let refused = Arc::clone(&rig.fleet)
            .run_session_shell(RunSessionShell { repository_tile: missing, ..rig.run(&sent) })
            .await
            .expect_err("no such tile");

        assert!(matches!(refused, api::Refusal::Unacceptable(ref error) if error.code == "fleet.tile_not_in_repository"));
        assert!(!marker.exists(), "nothing ran");
    }

    #[tokio::test]
    async fn a_changed_command_asks_again() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        commits(&rig, "echo first");
        let ask = asked(&rig).await;
        rig.fleet.repository_tile_approved(approving(&rig, &ask.command_hash)).await.expect("approved");

        commits(&rig, "echo second");

        assert!(!asked(&rig).await.approved, "the listing asks again");
        let refused = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, "echo second")).await.expect_err("asks");
        assert!(matches!(refused, api::Refusal::Unacceptable(ref error) if error.code == "fleet.tile_not_approved"));
    }

    #[tokio::test]
    async fn approving_a_stale_hash_is_refused_and_keeps_nothing() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        commits(&rig, "echo first");
        let stale = asked(&rig).await.command_hash;
        commits(&rig, "echo second");

        let refused = rig.fleet.repository_tile_approved(approving(&rig, &stale)).await.expect_err("stale");

        assert!(matches!(refused, api::Refusal::Unacceptable(ref error) if error.code == "fleet.tile_command_changed"));
        assert!(!asked(&rig).await.approved);
        let pressed = Arc::clone(&rig.fleet).run_session_shell(pressed(&rig, "echo second")).await;
        assert!(pressed.is_err(), "still refused");
    }

    #[tokio::test]
    async fn a_persons_own_tile_never_asks() {
        let rig = rig().await;
        rig.holds_the_slot().await;
        commits(&rig, "echo mine");

        let ran = rig.ran("echo mine").await;

        assert_eq!((ran.exit, ran.output.trim()), (Some(0), "mine"));
    }
}
