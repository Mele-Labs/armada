//! A run's processes and the setup before them, against real processes: a program that is not in
//! the checkout, and `setup.requires` run once per checkout and again when what it was traced from
//! changed.

use std::sync::Arc;

use crate::app_runs::{ProcessState, RunTarget};
use crate::tests::app_runs::settled;
use crate::tests::servers::a_fleet_holding;
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
