//! *Edit the run*: the wire answers a saved run whole, so the confirm reopens with it, and a save
//! under the name of a run Kit's copy holds replaces that run and the setup it saved. **A run only
//! the repository's own `armada.yml` declares is not Kit's to replace**, and stays refused.

use api::AppRuns;
use config::{Edit, PortEdit};

use crate::app_runs::draft::{DraftPort, DraftProcess, DraftSetup, DraftSource, RunDraft};
use crate::tests::servers::a_fleet_holding;
use crate::tests::tmp::TempDir;

const REPOSITORY: &str = "version: 1\nid: budget-app\ncommands:\n  bootstrap:\n    run: make bootstrap\nsetup:\n  requires: [bootstrap]\n";

/// `serve` with `${port}` for the process's own port, as the wire spells it.
fn process(name: &str, serve: &str, dir: &str, env: Option<&str>) -> DraftProcess {
    DraftProcess {
        name: name.to_string(),
        serve: serve.replace("${port}", &format!("${{port.{name}}}")),
        dir: Some(dir.to_string()),
        port: Some(DraftPort { name: name.to_string(), env: env.map(str::to_string) }),
        from: DraftSource::Confirmed,
    }
}

fn setup(name: &str, run: &str) -> DraftSetup {
    DraftSetup { name: name.to_string(), run: run.to_string(), creates: None }
}

fn run(name: &str, processes: Vec<DraftProcess>, setup: Vec<DraftSetup>) -> RunDraft {
    RunDraft { name: name.to_string(), label: Some(String::from("Budget app")), processes, setup, replacing: None }
}

#[tokio::test]
async fn a_run_saved_again_under_a_name_kits_copy_holds_replaces_it_and_the_setup_it_saved() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, REPOSITORY, REPOSITORY);
    let first = run(
        "dev",
        vec![
            process("backend", "python3 -m http.server ${port}", "backend", Some("PORT")),
            process("frontend", "npx vite --port ${port} --strictPort", "frontend", None),
        ],
        vec![setup("install_frontend", "npm ci --prefix frontend")],
    );
    let saved = fleet.run_saved(None, first).await.expect("it saves");
    assert!(saved.replaced.is_empty(), "a new run replaces nothing");
    let kit = saved.saved_in.file.clone();
    let text = std::fs::read_to_string(&kit).expect("Kit's file");
    let pin = Edit::Port { name: String::from("backend"), edit: PortEdit::Fixed(Some(4321)) };
    std::fs::write(&kit, config::splice(&text, &[pin]).expect("pinned")).expect("written");

    // The wire answers it whole: what the confirm reopens with.
    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    let dev = listed.iter().find(|one| one.name == "dev").expect("the run");
    assert!(dev.in_kit);
    let backend = &dev.processes[0];
    assert_eq!(
        (backend.command.as_str(), backend.dir.as_deref(), backend.port_env.as_deref(), backend.pinned_port),
        ("python3 -m http.server ${port}", Some("backend"), Some("PORT"), Some(4321))
    );
    let required: Vec<(&str, &str)> = dev.setup_commands.iter().map(|one| (one.name.as_str(), one.command.as_str())).collect();
    assert_eq!(required, [("bootstrap", "make bootstrap"), ("install_frontend", "npm ci --prefix frontend")]);

    // Saved back with `frontend` and its install dropped, and `admin` added.
    let edited = run(
        "dev",
        vec![
            process("backend", "python3 -m http.server ${port} --bind 127.0.0.1", "backend", Some("PORT")),
            process("admin", "npx vite --port ${port} --strictPort", "admin", None),
        ],
        vec![setup("bootstrap", "make bootstrap"), setup("install_admin", "npm ci --prefix admin")],
    );
    let again = fleet.run_saved(None, edited).await.expect("it replaces");

    assert_eq!(again.replaced, ["backend", "frontend"], "what is stopped before it starts again");
    assert_eq!(std::fs::read_to_string(home.path().join("armada.yml")).expect("read"), REPOSITORY);
    let manifest = fleet.run_manifest(&fleet.first(), fleet.first().root()).await.expect("it reads");
    assert_eq!(manifest.app_run_names(), ["dev"]);
    assert_eq!(manifest.app_run("dev").map(|one| one.servers().to_vec()), Some(vec![String::from("backend"), String::from("admin")]));
    assert!(manifest.server("frontend").is_none() && manifest.port("frontend").is_none(), "what only it ran is gone");
    assert!(manifest.server("backend").is_some_and(|one| one.serve().ends_with("--bind 127.0.0.1")));
    assert_eq!(manifest.port("backend").and_then(|one| one.fixed()), Some(4321), "a kept server keeps its pin");
    let required: Vec<(&str, &str)> = manifest.prepared_by().iter().map(|one| (one.name(), one.run())).collect();
    assert_eq!(required, [("bootstrap", "make bootstrap"), ("install_admin", "npm ci --prefix admin")]);
    assert!(manifest.command("install_frontend").is_none(), "a setup Command the confirm dropped is gone");
    let written = std::fs::read_to_string(&kit).expect("Kit's file");
    assert!(!written.contains("bootstrap"), "the repository's own setup stays the repository's: {written}");
}

const DECLARING: &str = r#"version: 1
id: budget-app
ports:
  web: {}
commands:
  web:
    serve: npx vite --port ${port.web} --strictPort
runs:
  dev:
    commands: [web]
"#;

#[tokio::test]
async fn a_run_only_the_repositorys_own_file_declares_is_answered_as_not_kits_and_its_save_stays_refused() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, DECLARING, DECLARING);

    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert!(!listed[0].in_kit);
    assert_eq!(listed[0].processes[0].command, "npx vite --port ${port} --strictPort");

    let draft = run("dev", vec![process("site", "npx vite --port ${port}", "site", None)], Vec::new());
    let refused = fleet.run_saved(None, draft).await.expect_err("the repository's run");

    let api::Refusal::Unacceptable(why) = refused else { panic!("expected a refusal of the draft") };
    assert_eq!(why.code, "fleet.app_run_draft_refused");
    assert!(why.message.contains("already declares `dev`"), "{}", why.message);
    assert_eq!(std::fs::read_to_string(home.path().join("armada.yml")).expect("read"), DECLARING);
}
