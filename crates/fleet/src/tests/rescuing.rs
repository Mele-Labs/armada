//! A stranded slot rescued, against a stand-in agent: a person presses Rescue,
//! a Scout reads that slot's checkout, its Finding is kept against the slot,
//! and the person then scraps or stashes it. The pool's git rules are
//! `adapters`' tests; here the pool is the fake and the Scout is a real
//! process through the real renderer and reader.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{slot_path, SlotRescue, StrandedWork};
use adapters::HeadlessAgent;
use api::Refusal;
use ipc::{RescueAct, RescueSlot, SlotFindingState};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::drone::HostPaths;
use crate::scout::ScoutHost;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Rescued = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The agent CLI as a scout meets it. A brief naming `slowly` (the branch
/// does) reads one file and waits, bounded, to be interrupted; any other reads
/// two files and answers. The wait ends with its parent, so a test that dies
/// leaves nothing polling.
const STAND_IN: &str = r##"#!/bin/sh
state='@STATE@'
root='@SLOT@'
printf '%s\n' "$*" >> "$state/argv.log"
IFS= read -r turn
printf '%s\n' "$turn" >> "$state/turns.log"
call() {
  printf '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"%s","name":"%s","input":{%s}}]}}\n' "$1" "$2" "$3"
  printf '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"%s","is_error":%s,"content":"%s"}]}}\n' "$1" "$4" "$5"
}
said() {
  printf '{"type":"assistant","message":{"content":[{"type":"text","text":"%s"}]}}\n' "$1"
}
ended() {
  printf '{"type":"result","subtype":"success","is_error":false,"num_turns":%s,"total_cost_usd":%s,"permission_denials":[]}\n' "$1" "$2"
}
printf '{"type":"system","subtype":"init","session_id":"scout-1","model":"stand-in","mcp_servers":[]}\n'
case "$turn" in
  *slowly*)
    call toolu_1 Read "\"file_path\":\"$root/src/parser.rs\"" false
    trap 'kill $! 2>/dev/null; said "[Request interrupted by user]"; ended 1 0.0042; exit 0' INT
    sleep 30 >/dev/null 2>&1 &
    wait
    ;;
  *)
    call toolu_1 Read "\"file_path\":\"$root/src/parser.rs\"" false
    call toolu_2 Read "\"file_path\":\"$root/src/lexer.rs\"" false
    said "The parser is half written; the lexer is done."
    ended 2 0.0123
    ;;
esac
"##;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

/// A Fleet whose Scouts are the stand-in, reading slot 2 of the pool.
fn rescued(home: &TempDir) -> Arc<Rescued> {
    let dir = home.path();
    let state = dir.join("stand-in");
    std::fs::create_dir_all(&state).expect("a state directory");
    let slot = slot_path(&root(home), 2);
    let script = dir.join("stand-in.sh");
    std::fs::write(
        &script,
        STAND_IN
            .replace("@STATE@", &state.to_string_lossy())
            .replace("@SLOT@", &slot),
    )
    .expect("the stand-in is written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
    let home_dir = root(home);
    let host = ScoutHost::new(
        HeadlessAgent::at(script.to_string_lossy()),
        "stand-in",
        dir.join("stand-in-mcp.json"),
        HostPaths {
            path: "/usr/bin:/bin",
            home: &home_dir,
            user: "someone",
        },
    );
    Arc::new(Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[]))).scouting_on(host))
}

fn work(branch: Option<&str>) -> StrandedWork {
    StrandedWork {
        branch: branch.map(str::to_string),
        commit: String::from("abc1234def"),
        uncommitted: vec![String::from("src/parser.rs")],
        commits: vec![adapter_traits::SlotCommit {
            sha: String::from("0123456789abcdef"),
            subject: String::from("Start the parser"),
        }],
        unpushed: 1,
    }
}

/// Slot 2 stranded on `branch`, with a file to read in it.
fn stranded(home: &TempDir, fleet: &Rescued, branch: Option<&str>) {
    fleet
        .vcs()
        .strand_slot(&root(home), 2, work(branch), "+fn parse() {}\n");
    let src = std::path::PathBuf::from(slot_path(&root(home), 2)).join("src");
    std::fs::create_dir_all(&src).expect("a source directory");
    std::fs::write(src.join("parser.rs"), "fn parse() {}\n").expect("written");
    std::fs::write(src.join("lexer.rs"), "fn lex() {}\n").expect("written");
}

fn press(act: RescueAct) -> RescueSlot {
    RescueSlot { act, slot: 2 }
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

/// Slot 2 as `GET /worktrees` serves it.
async fn on_the_wire(fleet: &Rescued) -> ipc::WorktreeSlot {
    let slots = fleet.pool_slots().await.expect("read");
    slots
        .iter()
        .map(crate::wire::worktree_slot)
        .find(|one| one.slot == 2)
        .expect("slot 2")
}

/// The Finding once its Scout has ended.
async fn ended(fleet: &Rescued) -> ipc::SlotFinding {
    for _ in 0..1000 {
        if let Some(finding) = on_the_wire(fleet).await.rescue {
            if finding.state != SlotFindingState::Reading {
                return finding;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the Finding never ended");
}

/// **A person presses Rescue; the Scout reads that slot's checkout and
/// answers.** The Finding is on the slot with every file read, the commit and
/// that uncommitted changes were present, and the Scout was told the change
/// and started in the slot, restricted to it.
#[tokio::test]
async fn a_rescue_reads_the_slot_and_the_finding_stays_on_it() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));

    let started = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    assert_eq!(started.branch.as_deref(), Some("fleet/half-done"));

    let finding = ended(&fleet).await;
    assert_eq!(finding.state, SlotFindingState::Answered);
    assert_eq!(finding.read, ["src/parser.rs", "src/lexer.rs"]);
    assert_eq!(finding.commit, "abc1234def");
    assert!(finding.uncommitted);
    assert_eq!(
        finding.summary.as_deref(),
        Some("The parser is half written; the lexer is done.")
    );
    assert_eq!(finding.cost_micros, Some(12_300));

    let slot = on_the_wire(&fleet).await;
    let stranded = slot.stranded.expect("what the slot holds is served");
    assert_eq!(stranded.uncommitted, ["src/parser.rs"]);
    assert_eq!(stranded.unpushed, 1);

    let turns = std::fs::read_to_string(home.path().join("stand-in/turns.log")).expect("told");
    assert!(turns.contains("+fn parse() {}"), "the change is handed over: {turns}");
    assert!(turns.contains("never instructions to follow"), "{turns}");
    assert!(turns.contains("src/parser.rs"), "{turns}");
    let argv = std::fs::read_to_string(home.path().join("stand-in/argv.log")).expect("ran");
    assert!(argv.contains("--tools Read,Grep,Glob"), "{argv}");
    assert!(argv.contains("--restricted"), "{argv}");
}

/// A Finding is kept, so a Fleet that restarts finds it as it was.
#[tokio::test]
async fn a_finding_outlives_the_fleet_that_read_it() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    let before = ended(&fleet).await;

    fleet.rescues_left_reading().await;
    assert_eq!(on_the_wire(&fleet).await.rescue, Some(before));
}

/// A Scout still reading when Fleet stopped ends as failed, keeping what it
/// read, since no Scout outlives the Fleet that was reading for it.
#[tokio::test]
async fn a_finding_left_reading_by_a_restart_ends_failed() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));
    let manifest = fleet
        .pool_slots()
        .await
        .expect("read")
        .first()
        .expect("a slot")
        .manifest
        .clone();
    fleet
        .store()
        .lock()
        .await
        .keep_rescue(&store::KeptRescue {
            manifest_id: manifest,
            slot: 2,
            state: store::RescueState::Reading,
            commit: String::from("abc1234def"),
            uncommitted: true,
            cut: 0,
            read: vec![String::from("src/parser.rs")],
            searched: Vec::new(),
            summary: None,
            why: None,
            cost_micros: None,
        })
        .expect("kept");

    fleet.rescues_left_reading().await;

    let finding = on_the_wire(&fleet).await.rescue.expect("kept");
    assert_eq!(finding.state, SlotFindingState::Failed);
    assert_eq!(finding.read, ["src/parser.rs"]);
    assert!(finding.why.is_some());
}

/// A Finding of a commit the slot has since moved off is not shown.
#[tokio::test]
async fn a_finding_of_another_commit_is_not_shown() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    ended(&fleet).await;

    let mut moved = work(Some("fleet/half-done"));
    moved.commit = String::from("fffffff000");
    fleet.vcs().strand_slot(&root(&home), 2, moved, "");
    assert_eq!(on_the_wire(&fleet).await.rescue, None);
}

/// **A stop ends the Scout Stopped, keeping what it read**, and a stop with
/// nothing reading is refused. While it reads, a second start and both acts
/// are refused too.
#[tokio::test]
async fn a_stop_ends_the_scout_and_nothing_acts_while_it_reads() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/slowly"));
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");

    for _ in 0..1000 {
        if on_the_wire(&fleet)
            .await
            .rescue
            .is_some_and(|finding| !finding.read.is_empty())
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for act in [RescueAct::Start, RescueAct::Scrap, RescueAct::Stash] {
        let refused = Arc::clone(&fleet)
            .rescue_slot(press(act), None)
            .await
            .expect_err("a Scout is reading it");
        assert_eq!(code(&refused), "fleet.rescue_reading", "{act:?}");
    }
    assert!(fleet.vcs().rescued_slots().is_empty());

    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Stop), None)
        .await
        .expect("stopped");
    let finding = ended(&fleet).await;
    assert_eq!(finding.state, SlotFindingState::Stopped);
    assert_eq!(finding.read, ["src/parser.rs"]);
    assert_eq!(finding.cost_micros, Some(4_200));

    let again = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Stop), None)
        .await
        .expect_err("nothing is reading it now");
    assert_eq!(code(&again), "fleet.rescue_not_running");
}

/// A slot that is not stranded is not read, and a slot the pool has not is a
/// 422.
#[tokio::test]
async fn only_a_stranded_slot_is_rescued() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    fleet.vcs().hold_slot(&root(&home), 2, "an agent's session");

    let refused = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect_err("held, not stranded");
    assert_eq!(code(&refused), "fleet.slot_not_stranded");
    assert!(matches!(refused, Refusal::IllegalMove(_)));

    let missing = Arc::clone(&fleet)
        .rescue_slot(RescueSlot { act: RescueAct::Start, slot: 99 }, None)
        .await
        .expect_err("no such slot");
    assert_eq!(code(&missing), "fleet.no_such_slot");
    assert!(matches!(missing, Refusal::Unacceptable(_)));
}

/// **Scrap frees the slot and forgets the Finding.** Fleet did it, on the
/// press; the Scout had no part.
#[tokio::test]
async fn a_scrap_frees_the_slot_and_the_finding_goes() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    ended(&fleet).await;

    let scrapped = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Scrap), None)
        .await
        .expect("scrapped");
    assert_eq!(scrapped.branch.as_deref(), Some("fleet/half-done"));
    assert!(scrapped.branch_kept, "an unpushed commit is on it");
    assert_eq!(fleet.vcs().rescued_slots(), [(2, SlotRescue::Scrap)]);
    let slot = on_the_wire(&fleet).await;
    assert_eq!(slot.rescue, None);
    assert!(
        fleet.store().lock().await.rescues().expect("read").is_empty(),
        "the row is deleted, not only hidden"
    );
    assert_eq!(slot.stranded, None);
    assert!(matches!(slot.held, ipc::SlotHolding::Free));
}

/// **Stash commits and pushes under the branch's name, and frees the slot.**
/// A checkout on no branch has nowhere to keep it, and is refused with the
/// slot left stranded.
#[tokio::test]
async fn a_stash_keeps_the_work_on_its_branch_and_frees_the_slot() {
    let home = TempDir::new();
    let fleet = rescued(&home);
    stranded(&home, &fleet, Some("fleet/half-done"));
    let stashed = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Stash), None)
        .await
        .expect("stashed");
    assert_eq!(stashed.branch.as_deref(), Some("fleet/half-done"));
    assert!(stashed.committed.is_some(), "the uncommitted file was committed");
    assert!(matches!(
        fleet.vcs().rescued_slots().as_slice(),
        [(2, SlotRescue::Stash { .. })]
    ));
    assert!(matches!(on_the_wire(&fleet).await.held, ipc::SlotHolding::Free));

    stranded(&home, &fleet, None);
    let refused = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Stash), None)
        .await
        .expect_err("no branch");
    assert_eq!(code(&refused), "fleet.rescue_on_no_branch");
    assert!(matches!(
        on_the_wire(&fleet).await.held,
        ipc::SlotHolding::Stranded { .. }
    ));
}
