//! A run read from the file when it starts, rather than from the Manifest Fleet booted with: one
//! saved while Fleet is up starts at once, on ports that never move one already in use.
//! **Every start here is a real process**, for `crate::tests::servers`' reason.

use std::collections::BTreeMap;
use std::sync::Arc;

use ipc::StartedBy;
use store::{PortClaim, PortClaimant};

use crate::app_runs::draft::{DraftPort, DraftProcess, DraftSource, RunDraft};
use crate::app_runs::{RunPhase, RunTarget, RunView};
use crate::checkouts::Checkout;
use crate::ports::spread_for_tests as spread;
use crate::servers::Holder;
use crate::tests::app_runs::{a_fleet_with_folders, answers, code, git, settled};
use crate::tests::tmp::TempDir;

fn names(of: &[&str]) -> Vec<String> {
    of.iter().map(|one| one.to_string()).collect()
}

fn ports(of: &[(&str, u16)]) -> BTreeMap<String, u16> {
    of.iter().map(|(name, port)| (name.to_string(), *port)).collect()
}

#[test]
fn a_booted_name_keeps_its_offset_and_a_new_one_goes_after_them_whatever_its_name_sorts_to() {
    let held = names(&["api", "other", "web"]);
    // Nothing differs: the ports are the ones the boot gave them.
    let same = spread(&held, &held, &BTreeMap::new(), 100);
    assert_eq!(same, ports(&[("api", 100), ("other", 101), ("web", 102)]));
    // `aaa` sorts before `api`, and still goes after the three.
    let more = spread(&held, &names(&["aaa", "api", "other", "web"]), &BTreeMap::new(), 100);
    assert_eq!(more, ports(&[("aaa", 103), ("api", 100), ("other", 101), ("web", 102)]));
    // Two new ones go in sorted order after them.
    let two = spread(&held, &names(&["api", "mid", "other", "web", "zed"]), &BTreeMap::new(), 100);
    assert_eq!((two["mid"], two["zed"]), (103, 104));
    // One the boot knew and the file dropped leaves its port unused.
    let fewer = spread(&held, &names(&["api", "web"]), &BTreeMap::new(), 100);
    assert_eq!(fewer, ports(&[("api", 100), ("web", 102)]));
}

#[test]
fn a_port_a_live_server_is_on_stays_its_own_and_is_never_handed_to_another_name() {
    let held = names(&["api", "web"]);
    // `late` was started on 102, which is where the next new name would have gone.
    let live = ports(&[("late", 102)]);
    let placed = spread(&held, &names(&["api", "late", "later", "web"]), &live, 100);
    assert_eq!((placed["late"], placed["later"]), (102, 103), "{placed:?}");
    assert_eq!((placed["api"], placed["web"]), (100, 101));
    // A live server whose name the file has since dropped still holds its port.
    let placed = spread(&held, &names(&["api", "web", "zed"]), &ports(&[("gone", 102)]), 100);
    assert_eq!(placed["zed"], 103);
}

fn a_new_server(name: &str) -> DraftProcess {
    DraftProcess {
        name: name.to_string(),
        serve: format!("python3 -m http.server ${{port.{name}}} --bind 127.0.0.1"),
        dir: None,
        port: Some(DraftPort { name: name.to_string(), env: None }),
        from: DraftSource::RunMemory,
    }
}

fn a_new_run(name: &str, servers: &[&str]) -> RunDraft {
    RunDraft {
        name: name.to_string(),
        label: None,
        processes: servers.iter().map(|one| a_new_server(one)).collect(),
    }
}

/// Wait for `port` to answer: a server with no `ready` is up as soon as it starts, before it binds.
async fn answering(port: u16) {
    let up = async {
        while !answers(&format!("http://127.0.0.1:{port}/")) {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), up).await.expect("the server answers");
}

fn live_ports(view: &RunView) -> Vec<u16> {
    view.processes.iter().filter_map(|one| one.port()).collect()
}

#[tokio::test]
async fn a_run_saved_while_fleet_is_up_starts_at_once_on_ports_that_leave_the_running_ones_alone() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);
    Arc::clone(&fleet).run_started(&target, "app").await.expect("the booted run starts");
    let app = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    let root = fleet.first().root().to_string();
    let booted_width = fleet.store().lock().await.port_span_for_checkout(&root).expect("read").expect("claimed").width;

    // `aaa` sorts before every port the boot knew, so a naive re-index would move all of theirs.
    let saved = fleet.run_saved(None, a_new_run("extra", &["aaa"])).await.expect("it saves");
    assert!(saved.saved_in.file.ends_with(".yml"), "{saved:?}");
    let (_, fresh) = Arc::clone(&fleet).run_started(&target, "extra").await.expect("it starts with no restart");

    assert!(fresh);
    let extra = settled(&fleet, "extra", |view| view.phase() == RunPhase::Running).await;
    let port = extra.processes[0].port().expect("a port");
    assert!(!live_ports(&app).contains(&port), "{port} is not one the running run is on");
    answering(port).await;
    let app_now = fleet.run_read(&target, "app").await.expect("reads");
    assert_eq!(live_ports(&app_now), live_ports(&app), "the running run kept its ports");
    for port in live_ports(&app) {
        answering(port).await;
    }
    let widened = fleet.store().lock().await.port_span_for_checkout(&root).expect("read").expect("claimed");
    assert!(widened.width > booted_width && (widened.base..widened.base + widened.width).contains(&port), "{widened:?}");
    // An event of the new server names its run, though Fleet booted without it.
    let state = extra.processes[0].server.clone().expect("an instance");
    assert_eq!(fleet.runs_moved_by(&state).iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["extra"]);

    fleet.run_stopped(&target, "extra").await.expect("it stops");
    fleet.run_stopped(&target, "app").await.expect("it stops");
}

#[tokio::test]
async fn a_span_with_no_room_after_it_moves_when_nothing_is_up_and_refuses_when_something_is() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let root = fleet.first().root().to_string();
    let held = fleet.first().manifest().clone();
    let many: String = (0..6).map(|n| format!("  p{n}: {{}}\n")).collect();
    let text = format!("version: 1\nid: 01FIXTUREMANIFEST\nports:\n  api: {{}}\n  web: {{}}\n  other: {{}}\n{many}");
    let fresh = config::Manifest::parse(std::path::Path::new("armada.yml"), &text).expect("a Manifest");

    // The booted claim, and a neighbour right after it.
    fleet.checkout_ports(&fleet.first(), &root).await;
    let claim = fleet.store().lock().await.port_span_for_checkout(&root).expect("read").expect("claimed");
    let neighbour = PortClaim {
        claimant: PortClaimant::Checkout(String::from("/elsewhere")),
        base: claim.base + claim.width,
        width: 2,
        claimed_at: fleet.now(),
    };
    fleet.store().lock().await.claim_port_span(&neighbour).expect("a neighbour");

    // Something up pins the span where it is, so there is nowhere to grow.
    let (other, _) = Arc::clone(&fleet)
        .hold_server(Checkout::main(fleet.first()), "other", StartedBy::Person, None)
        .await
        .expect("a server");
    let refused = fleet.checkout_ports_declared(&root, &held, &fresh).await.expect_err("no room");
    assert!(matches!(refused, crate::ports::PortsRefused::RangeExhausted { .. }), "{refused}");
    assert!(fleet.servers().any_live(&Holder::Checkout(root.clone())));
    fleet.stopped_server(&other.id).await.expect("it stops");

    // Nothing up: the span goes where there is room, every port inside it and clear of the neighbour.
    let placed = fleet.checkout_ports_declared(&root, &held, &fresh).await.expect("it moves");
    let moved = fleet.store().lock().await.port_span_for_checkout(&root).expect("read").expect("claimed");
    assert!(placed.values().all(|port| (moved.base..moved.base + moved.width).contains(port)), "{placed:?} in {moved:?}");
    assert!(
        moved.base + moved.width <= neighbour.base || moved.base >= neighbour.base + neighbour.width,
        "{moved:?} clear of {neighbour:?}"
    );
    assert_eq!(placed.len(), 9);
}

const OWN_FILE: &str = "version: 1\nid: 01FIXTUREMANIFEST\nports:\n  jobport: {}\ncommands:\n  jobsrv:\n    serve: python3 -m http.server ${port.jobport} --bind 127.0.0.1\nruns:\n  jobrun:\n    commands: [jobsrv]\n";

#[tokio::test]
async fn a_job_runs_the_server_its_own_worktree_adds_and_main_does_not_have_it() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let root = std::path::PathBuf::from(fleet.first().root());
    git(&root, &["init", "-q"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "start"]);
    let slots = TempDir::new();
    let slot = slots.path().join("slot-1");
    git(&root, &["worktree", "add", "-q", "-b", "job-branch", &slot.to_string_lossy()]);
    let jobs = Checkout::of_a_job(fleet.first(), &slot.to_string_lossy()).expect("a worktree");

    // Without a file of its own, the worktree runs what the repository declares and not the server.
    let before = Arc::clone(&fleet).run_started_on(jobs.clone(), "jobrun").await.expect_err("not declared yet");
    assert_eq!(code(before), "fleet.not_an_app_run");

    std::fs::write(slot.join("armada.yml"), OWN_FILE).expect("a Job adds a server");
    let (started, fresh) = Arc::clone(&fleet).run_started_on(jobs.clone(), "jobrun").await.expect("its own file's run starts");
    assert!(fresh);
    assert_eq!(started.checkout, jobs.path());
    let port = started.processes[0].port().expect("a port");
    answering(port).await;
    let listed = fleet.runs_listed(&RunTarget::Main(None)).await.expect("listed");
    assert!(listed.iter().all(|one| one.name != "jobrun"), "main does not declare the Job's run");

    fleet.run_stopped_on(&jobs, "jobrun").await.expect("it stops");
}

#[tokio::test]
async fn a_manifest_that_will_not_read_refuses_the_run_and_starts_nothing() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    std::fs::write(home.path().join("armada.yml"), "version: 1\nruns: [not, a, map]\n").expect("broken since boot");

    let started = Arc::clone(&fleet).run_started(&RunTarget::Main(None), "app").await.expect_err("refused");
    assert_eq!(code(started), "fleet.app_run_manifest_unreadable");
    let listed = fleet.runs_listed(&RunTarget::Main(None)).await.expect_err("refused");
    assert_eq!(code(listed), "fleet.app_run_manifest_unreadable");
    assert!(fleet.servers().every().is_empty(), "nothing was started");
}
