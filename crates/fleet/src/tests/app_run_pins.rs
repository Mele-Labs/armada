//! A process pinned to a number takes exactly that number, and a second checkout that wants the
//! same number is refused, naming what holds it. **Every start is a real process**, for
//! `crate::tests::servers`' reason: whether a number is free is what a fake cannot stand in for.

use std::sync::Arc;

use api::Refusal;

use crate::app_runs::{RunPhase, RunTarget};
use crate::checkouts::Checkout;
use crate::tests::app_runs::{a_fleet_with_folders, answers, code, git, settled};
use crate::tests::tmp::TempDir;

/// A port nothing holds right now, from the kernel's own pick.
pub(super) fn a_free_port() -> u16 {
    let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    socket.local_addr().expect("an address").port()
}

pub(super) fn said(refusal: Refusal) -> String {
    match refusal {
        Refusal::Unacceptable(why) | Refusal::IllegalMove(why) | Refusal::NotFound(why) | Refusal::Fault(why) => why.message,
    }
}

/// The fixture's own file with `api` pinned to `port`.
pub(super) fn pinned_api(port: u16) -> String {
    format!(
        "version: 1\nid: 01FIXTUREMANIFEST\nports:\n  api:\n    fixed: {port}\n  web: {{}}\ncommands:\n  api:\n    serve: python3 -m http.server ${{port.api}} --bind 127.0.0.1\n    dir: backend\n  web:\n    serve: python3 -m http.server ${{port.web}} --bind 127.0.0.1\n    dir: frontend\nruns:\n  app:\n    commands: [api, web]\n"
    )
}

async fn answering(port: u16) {
    let up = async {
        while !answers(&format!("http://127.0.0.1:{port}/")) {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), up).await.expect("the server answers");
}

#[tokio::test]
async fn a_pinned_process_takes_exactly_its_number_and_the_other_stays_leased() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    let pin = a_free_port();
    std::fs::write(fleet.first().manifest().path(), pinned_api(pin)).expect("the file pins api");

    Arc::clone(&fleet).run_started(&RunTarget::Main(None), "app").await.expect("it starts");
    let running = settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;
    assert_eq!(running.processes[0].port(), Some(pin), "api is on the number it was pinned to");
    assert_ne!(running.processes[1].port(), Some(pin), "web is leased as before");
    answering(pin).await;

    fleet.run_stopped(&RunTarget::Main(None), "app").await.expect("it stops");
}

#[tokio::test]
async fn a_jobs_bay_wanting_a_pinned_number_main_holds_is_refused_naming_the_number_and_the_holder() {
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
    let jobs = Checkout::of_a_job(fleet.first(), &slot.to_string_lossy()).expect("a worktree");
    let pin = a_free_port();
    std::fs::write(fleet.first().manifest().path(), pinned_api(pin)).expect("the file pins api");
    std::fs::write(slot.join("armada.yml"), pinned_api(pin)).expect("and so does the Job's own");

    Arc::clone(&fleet).run_started(&RunTarget::Main(None), "app").await.expect("main takes the number");
    settled(&fleet, "app", |view| view.phase() == RunPhase::Running).await;

    let refused = Arc::clone(&fleet).run_started_on(jobs.clone(), "app").await.expect_err("the number is taken");
    let sentence = said(refused);
    assert!(sentence.contains(&pin.to_string()), "{sentence}");
    assert!(sentence.contains("`api` on") && sentence.contains(&root.to_string_lossy().to_string()), "{sentence}");
    let again = Arc::clone(&fleet).run_started_on(jobs.clone(), "app").await.expect_err("still taken");
    assert_eq!(code(again), "fleet.pinned_port_taken");
    // Nothing of the bay's run was left half up.
    assert!(fleet.run_stopped_on(&jobs, "app").await.expect("reads").processes.iter().all(|one| one.server.is_none()));

    fleet.run_stopped(&RunTarget::Main(None), "app").await.expect("main stops");
}
