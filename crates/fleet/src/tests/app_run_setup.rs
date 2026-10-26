//! A run's processes and the setup before them, against real processes: a program that is not in
//! the checkout, and `setup.requires` run once per checkout and again when what it was traced from
//! changed.

use std::sync::Arc;

use ipc::{AppRunSetupState, Event};

use crate::app_runs::{ProcessState, RunPhase, RunTarget, SetupState};
use crate::tests::app_runs::settled;
use crate::tests::servers::{a_fleet_holding, next_event};
use crate::tests::tmp::TempDir;

/// Each server cannot start for a reason of its own.
const UNSTARTABLE: &str = r#"version: 1
id: 01FIXTUREMANIFEST
commands:
  backend:
    serve: .venv/bin/uvicorn main:app
    dir: backend
  frontend:
    serve: python3 -m http.server
    dir: frontend
  script:
    serve: ./serve.sh
  ghost:
    serve: armada-no-such-program-anywhere
runs:
  broken:
    commands: [backend, frontend, script, ghost]
"#;

#[tokio::test]
async fn a_process_that_cannot_start_says_why_on_the_first_line_of_its_log() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, UNSTARTABLE, UNSTARTABLE);
    let root = std::path::PathBuf::from(fleet.first().root());
    std::fs::create_dir_all(root.join("backend")).expect("backend, with no .venv in it");
    std::fs::write(root.join("serve.sh"), "#!/bin/sh\n").expect("a script with no execute bit");

    Arc::clone(&fleet).run_started(&RunTarget::Main(None), "broken").await.expect("it is started");
    let view = settled(&fleet, "broken", |view| view.processes.iter().all(|one| one.state == ProcessState::Failed)).await;

    let first_lines: Vec<String> = view
        .processes
        .iter()
        .map(|one| {
            let id = &one.server.as_ref().expect("an instance").id;
            let read = fleet.observed_server(id).expect("its log reads");
            read.history.first().cloned().unwrap_or_default()
        })
        .collect();
    assert_eq!(
        first_lines,
        [
            "`.venv/bin/uvicorn` is not in this checkout",
            "`frontend` is not a folder in this checkout",
            "`./serve.sh` is not executable",
            "`armada-no-such-program-anywhere` is not on this machine's PATH",
        ]
    );
    let ended = view.processes[0].server.as_ref().and_then(|one| one.ended.clone());
    assert_eq!(ended.as_deref(), Some("could not start: `.venv/bin/uvicorn` is not in this checkout"));
}

/// `install` counts its runs in `installs.txt` and makes `deps`; `web` serves once it has.
const SET_UP: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  web: {}
commands:
  install:
    run: "/bin/sh -c 'echo installing; mkdir -p deps; echo ran >> installs.txt'"
  broken:
    run: "/bin/sh -c 'echo no network; exit 4'"
  web:
    serve: python3 -m http.server ${port.web} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.web}/
setup:
  requires: [install]
runs:
  app:
    commands: [web]
"#;

fn installs(root: &std::path::Path) -> usize {
    std::fs::read_to_string(root.join("installs.txt")).map_or(0, |text| text.lines().count())
}

#[tokio::test]
async fn a_run_sets_its_checkout_up_once_and_again_when_what_it_was_traced_from_changes() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, SET_UP, SET_UP);
    let root = std::path::PathBuf::from(fleet.first().root());
    std::fs::write(root.join("requirements.txt"), "fastapi\n").expect("what setup is traced from");
    let target = RunTarget::Main(None);
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it is started");
    let setup = started.setup.clone().expect("the checkout is new to its setup");
    assert_eq!((setup.state, setup.commands.as_slice()), (SetupState::SettingUp, &[String::from("install")][..]));
    assert!(started.processes.iter().all(|one| one.state == ProcessState::NotStarted), "nothing starts before setup");
    let told = next_event(&mut watching, |event| {
        matches!(event, Event::AppRunChanged(run) if run.setup.as_ref().is_some_and(|one| one.state == AppRunSetupState::SettingUp))
    })
    .await;
    let Event::AppRunChanged(told) = told else { unreachable!("filtered") };
    let wired = told.setup.expect("on the wire");
    assert_eq!(wired.id, setup.id);
    assert!(wired.log_path.starts_with('/') && wired.log_path.contains(&setup.id), "{}", wired.log_path);

    let up = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    assert_eq!(up.setup.as_ref().map(|one| one.state), Some(SetupState::Ready));
    assert_eq!(installs(&root), 1);
    let output = fleet.observed_server(&setup.id).expect("its output is read as a server's is");
    assert!(output.history.iter().any(|line| line == "installing"), "{:?}", output.history);
    assert!(std::fs::read_to_string(&wired.log_path).expect("the file is there").contains("installing"));
    let process = fleet.app_run_wired(up.clone(), None).await.expect("wired").processes.remove(0);
    let log = process.log_path.expect("a started process names its log");
    assert!(std::path::Path::new(&log).is_file(), "{log}");

    // Set up and unchanged: the processes start at once, and setup is not run again.
    fleet.run_stopped(&target, "app").await.expect("it stops");
    let (again, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    assert_eq!(again.setup.as_ref().map(|one| one.id.as_str()), Some(setup.id.as_str()), "the same setup");
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    assert_eq!(installs(&root), 1);

    // What it was traced from changed: set up again.
    fleet.run_stopped(&target, "app").await.expect("it stops");
    std::fs::write(root.join("requirements.txt"), "fastapi\nuvicorn\n").expect("changed");
    let (changed, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    assert_ne!(changed.setup.as_ref().map(|one| one.id.as_str()), Some(setup.id.as_str()), "a new setup");
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    assert_eq!(installs(&root), 2);

    // What it made is gone, as a slot's clean takes it: set up again.
    fleet.run_stopped(&target, "app").await.expect("it stops");
    std::fs::remove_dir_all(root.join("deps")).expect("cleaned");
    Arc::clone(&fleet).run_started(&target, "app").await.expect("it starts");
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    assert_eq!(installs(&root), 3);
    fleet.run_stopped(&target, "app").await.expect("it stops");
}

#[tokio::test]
async fn a_setup_command_that_fails_starts_no_process_and_the_run_says_which() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let manifest = SET_UP.replace("requires: [install]", "requires: [install, broken]");
    let fleet = a_fleet_holding(&home, &events, &manifest, &manifest);
    let target = RunTarget::Main(None);

    Arc::clone(&fleet).run_started(&target, "app").await.expect("it is started");
    let view = settled(&fleet, "app", |view| view.setup.as_ref().is_some_and(|one| one.state == SetupState::Failed)).await;

    let setup = view.setup.expect("the setup");
    assert_eq!(setup.failed.as_deref(), Some("broken"));
    let ended = setup.ended.expect("how it ended");
    assert!(ended.contains("`broken`") && ended.contains("exited 4") && ended.contains("no network"), "{ended}");
    assert!(view.processes.iter().all(|one| one.state == ProcessState::NotStarted && one.server.is_none()), "no process started");
    let output = fleet.observed_server(&setup.id).expect("its output");
    assert!(output.history.iter().any(|line| line == "no network"), "{:?}", output.history);

    // A setup that failed left no mark, so the next start runs it again.
    let (again, _) = Arc::clone(&fleet).run_started(&target, "app").await.expect("it is started");
    assert_ne!(again.setup.map(|one| one.id), Some(setup.id));
}
