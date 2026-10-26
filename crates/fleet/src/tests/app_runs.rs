//! A repository's `runs:` started, read, stopped and restarted with no Job — against real
//! processes, for `crate::tests::servers`' reason: whether a port answers and is free again is what
//! a fake cannot stand in for. **Every case stands on the fixture that file builds.**

use std::sync::Arc;
use std::time::Duration;

use api::Refusal;
use ipc::{Event, ServerPhase, ServerState, StartedBy};

use crate::app_runs::{ProcessState, RunPhase, RunTarget, RunView};
use crate::checkouts::Checkout;
use crate::ports::{BindConnectProbe, PortProbe};
use crate::tests::servers::{a_fleet_holding, next_event, Fixture};
use crate::tests::tmp::TempDir;

/// `api` and `web` each serve their own folder on their own port, so what a process answers says
/// which folder it started in. `broken` falls over at once. `hasty` is a run that has it in it,
/// and `other` is a server no run lists.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  api: {}
  web: {}
  other: {}
commands:
  api:
    serve: python3 -m http.server ${port.api} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.api}/
    dir: backend
  web:
    serve: python3 -m http.server ${port.web} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.web}/
    dir: frontend
  other:
    serve: python3 -m http.server ${port.other} --bind 127.0.0.1
  broken:
    serve: "/bin/sh -c 'exit 3'"
  fmt:
    run: /usr/bin/true
runs:
  app:
    label: The app
    commands: [api, web]
  hasty:
    commands: [web, broken]
"#;

fn a_fleet_with_folders(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    let fleet = a_fleet_holding(home, events, MANIFEST, MANIFEST);
    let root = std::path::PathBuf::from(fleet.first().root());
    for folder in ["backend", "frontend"] {
        std::fs::create_dir_all(root.join(folder)).expect("a folder under the checkout");
    }
    std::fs::write(root.join("backend/marker.txt"), "backend").expect("a marker");
    fleet
}

fn code(refusal: Refusal) -> String {
    match refusal {
        Refusal::Unacceptable(why)
        | Refusal::IllegalMove(why)
        | Refusal::NotFound(why)
        | Refusal::Fault(why) => why.code,
    }
}

/// Whether `url` answers 2xx, by `curl` — the same probe the fixture's own `ready` uses.
fn answers(url: &str) -> bool {
    std::process::Command::new("curl")
        .args(["-sf", "-o", "/dev/null", url])
        .status()
        .is_ok_and(|status| status.success())
}

/// The run as it stands once `settled` says so, or a panic naming how it stood instead.
async fn settled(
    fleet: &Fixture,
    name: &str,
    settled: impl Fn(&RunView) -> bool,
) -> RunView {
    let target = RunTarget::Main(None);
    let waiting = async {
        loop {
            let view = fleet.run_read(&target, name).await.expect("the run reads");
            if settled(&view) {
                return view;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    match tokio::time::timeout(Duration::from_secs(30), waiting).await {
        Ok(view) => view,
        Err(_) => panic!(
            "the run never settled: {:?}",
            fleet.run_read(&target, name).await.map(|view| view.processes)
        ),
    }
}

fn states(view: &RunView) -> Vec<ProcessState> {
    view.processes.iter().map(|one| one.state).collect()
}

#[tokio::test]
async fn a_run_starts_each_server_on_its_own_port_in_its_own_folder() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);

    let (started, fresh) = Arc::clone(&fleet).run_started(&RunTarget::Main(None), "app").await.expect("it starts");
    assert!(fresh);
    assert_eq!((started.name.as_str(), started.label.as_str()), ("app", "The app"));
    let names: Vec<&str> = started.processes.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["api", "web"], "the file's order");

    let running = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    let ports: Vec<u16> = running.processes.iter().map(|one| one.port().expect("a port")).collect();
    assert_ne!(ports[0], ports[1], "each on a port of its own");
    // `backend/marker.txt` is there and `frontend/marker.txt` is not: each answered from its folder.
    assert!(answers(&format!("http://127.0.0.1:{}/marker.txt", ports[0])), "api started in backend");
    assert!(!answers(&format!("http://127.0.0.1:{}/marker.txt", ports[1])), "web started in frontend");

    fleet.run_stopped(&RunTarget::Main(None), "app").await.expect("it stops");
}

#[tokio::test]
async fn a_second_start_is_answered_with_the_run_already_up() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);

    let (first, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    let (again, fresh) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it answers");

    assert!(!fresh, "nothing was started by the second call");
    let ids = |view: &RunView| view.processes.iter().map(|one| one.server.as_ref().map(|s| s.id.clone())).collect::<Vec<_>>();
    assert_eq!(ids(&first), ids(&again), "the same instances");

    fleet.run_stopped(&target, "app").await.expect("it stops");
}

#[tokio::test]
async fn stopping_a_run_stops_all_of_it_and_nothing_else_the_checkout_holds() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);
    let (other, _) = Arc::clone(&fleet)
        .hold_server(Checkout::main(fleet.first()), "other", StartedBy::Person, None)
        .await
        .expect("a server no run lists");
    Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    let running = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    let ports: Vec<u16> = running.processes.iter().filter_map(|one| one.port()).collect();

    let stopped = fleet.run_stopped(&target, "app").await.expect("it stops");

    assert_eq!(stopped.phase(), RunPhase::Stopped);
    assert_eq!(states(&stopped), [ProcessState::Exited, ProcessState::Exited]);
    assert!(ports.iter().all(|port| BindConnectProbe.free(*port)), "its ports are free");
    assert_ne!(held_phase(&fleet), ServerPhase::Exited, "the server no run lists is still up");
    fleet.stopped_server(&other.id).await.expect("it stops");
}

fn held_phase(fleet: &Fixture) -> ServerPhase {
    fleet
        .servers()
        .instance(&crate::servers::Holder::Checkout(fleet.first().root().to_string()), "other")
        .map(|state| state.phase)
        .expect("held")
}

#[tokio::test]
async fn restarting_a_run_brings_every_process_up_again_as_new_instances() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);
    let (first, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;

    let again = Arc::clone(&fleet).run_restarted(&target, "app").await.expect("it restarts");

    let id = |view: &RunView, at: usize| view.processes[at].server.as_ref().map(|s| s.id.clone());
    assert_ne!(id(&first, 0), id(&again, 0));
    assert_ne!(id(&first, 1), id(&again, 1));
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    fleet.run_stopped(&target, "app").await.expect("it stops");
}

#[tokio::test]
async fn a_process_that_falls_over_fails_the_run_and_the_rest_stays_readable() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);
    Arc::clone(&fleet).run_started(&target, "hasty").await.expect("it starts");

    let view = settled(&fleet, "hasty", |view| states(view) == [ProcessState::Ready, ProcessState::Failed]).await;

    assert_eq!(view.phase(), RunPhase::Failed);
    let broken = view.processes[1].server.as_ref().expect("its instance");
    assert!(!broken.stopped && broken.exit_code == Some(3), "it fell over: {broken:?}");
    fleet.run_stopped(&target, "hasty").await.expect("it stops");
}

#[tokio::test]
async fn every_run_is_listed_in_the_files_order_and_a_server_event_names_the_runs_it_moved() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let target = RunTarget::Main(None);
    let listed = fleet.runs_listed(&target).await.expect("listed");
    assert_eq!(listed.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["app", "hasty"]);
    assert!(listed.iter().all(|one| one.phase() == RunPhase::Stopped));

    let mut watching = events.subscribe();
    Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    let serving = next_event(&mut watching, |event| matches!(event, Event::ServerServing(state) if state.name == "web")).await;
    let Event::ServerServing(state) = serving else { unreachable!("filtered") };

    let moved = fleet.runs_moved_by(&state);
    let names: Vec<&str> = moved.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["app", "hasty"], "`web` is in both runs");
    let other = ServerState { name: String::from("other"), ..state };
    assert!(fleet.runs_moved_by(&other).is_empty(), "a server no run lists moves no run");
    fleet.run_stopped(&target, "app").await.expect("it stops");
}

#[tokio::test]
async fn a_run_nothing_declares_and_a_job_nobody_holds_are_refused_with_their_codes() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);

    let unknown = Arc::clone(&fleet).run_started(&RunTarget::Main(None), "nothing").await.expect_err("refused");
    assert_eq!(code(unknown), "fleet.not_an_app_run");
    let no_job = Arc::clone(&fleet).run_started(&RunTarget::Job(String::from("nobody")), "app").await.expect_err("refused");
    assert_eq!(code(no_job), "fleet.no_job_to_run");
}

#[test]
fn a_server_that_exits_on_its_own_has_failed_and_one_that_was_stopped_has_not() {
    let state = |phase, stopped| ServerState {
        id: String::from("01SERVER"),
        name: String::from("web"),
        manifest_id: None,
        checkout: ipc::ServerCheckout { path: String::from("/work/shop"), branch: None, commit: None, behind: None },
        phase,
        serve: String::from("serve"),
        ports: Vec::new(),
        links: Vec::new(),
        started_by: StartedBy::Person,
        session_id: None,
        started_at: ipc::Instant::carried(String::from("2026-10-25T00:00:00Z")),
        serving_since: None,
        ended_at: None,
        exit_code: None,
        ended: None,
        stopped,
        log: String::from("log"),
    };
    assert_eq!(ProcessState::of(&state(ServerPhase::Starting, false)), ProcessState::Starting);
    assert_eq!(ProcessState::of(&state(ServerPhase::Serving, false)), ProcessState::Ready);
    assert_eq!(ProcessState::of(&state(ServerPhase::Exited, true)), ProcessState::Exited);
    assert_eq!(ProcessState::of(&state(ServerPhase::Exited, false)), ProcessState::Failed);
}

fn git(at: &std::path::Path, args: &[&str]) {
    let ran = std::process::Command::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com", "-C"])
        .arg(at)
        .args(args)
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
}

/// **One instance per checkout and run**: a Job's worktree holds its own beside the main checkout's,
/// each on a span of its own, and stopping one leaves the other.
#[tokio::test]
async fn a_run_on_a_jobs_worktree_is_its_own_instance_beside_the_main_checkouts() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let root = std::path::PathBuf::from(fleet.first().root());
    git(&root, &["init", "-q"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "start"]);
    let slots = TempDir::new();
    let slot = slots.path().join("slot-1");
    git(&root, &["worktree", "add", "-q", "-b", "job-branch", &slot.to_string_lossy()]);
    for folder in ["backend", "frontend"] {
        std::fs::create_dir_all(slot.join(folder)).expect("a folder in the worktree");
    }

    let jobs = Checkout::of_a_job(fleet.first(), &slot.to_string_lossy()).expect("a worktree of this repository");
    assert_eq!(jobs.branch(), Some("job-branch"));
    let elsewhere = Checkout::of_a_job(fleet.first(), &slots.path().to_string_lossy());
    assert!(elsewhere.is_err(), "a folder that is not a worktree of it is not one");

    Arc::clone(&fleet).run_started(&RunTarget::Main(None), "app").await.expect("main starts");
    let (on_job, fresh) = Arc::clone(&fleet).run_started_on(jobs.clone(), "app").await.expect("the worktree starts");
    assert!(fresh, "the main checkout's instance is not the worktree's");
    let main = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    let ports = |view: &RunView| view.processes.iter().filter_map(|one| one.port()).collect::<Vec<_>>();
    assert_ne!(ports(&main), ports(&on_job), "each checkout has a span of its own");
    assert_eq!(on_job.checkout, jobs.path());

    fleet.run_stopped_on(&jobs, "app").await.expect("the worktree's stops");
    let main = fleet.run_read(&RunTarget::Main(None), "app").await.expect("reads");
    assert_eq!(main.phase(), RunPhase::Running, "main's run is untouched");
    fleet.run_stopped(&RunTarget::Main(None), "app").await.expect("main stops");
}
