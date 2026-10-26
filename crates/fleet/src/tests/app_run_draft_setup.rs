//! A draft that proposes what a checkout installs before its app runs, keeps only the remembered
//! commands whose files will be there, and leases a port to every process. **The first case is the
//! repository the owner pressed Run on**: a Python backend and a Vite frontend, nothing installed in
//! the main checkout, and run memory from Jobs' worktrees.

use std::path::Path;

use api::AppRuns;
use store::KeptRun;

use crate::app_runs::draft::{drafted, DraftPort, DraftSetup, RunDraft};
use crate::tests::app_runs::git;
use crate::tests::servers::a_fleet_holding;
use crate::tests::tmp::TempDir;

fn fact(workspace: &str, command: &str, port: Option<u16>) -> KeptRun {
    KeptRun {
        workspace: workspace.to_string(),
        kind: String::from("dev_server"),
        command: command.to_string(),
        port,
        succeeded: true,
        last_ran: 1,
        source: String::from("jobs"),
        kept_in: None,
        why: None,
    }
}

fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

/// A repository with `files` committed, and `scratch` written beside them and never added.
fn a_repository(files: &[(&str, &str)], scratch: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new();
    for (file, text) in files {
        write(dir.path(), file, text);
    }
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "start"]);
    for (file, text) in scratch {
        write(dir.path(), file, text);
    }
    dir
}

fn serves(draft: &RunDraft) -> Vec<(&str, &str, Option<&str>)> {
    draft.processes.iter().map(|one| (one.name.as_str(), one.serve.as_str(), one.dir.as_deref())).collect()
}

fn setup(draft: &RunDraft) -> Vec<(&str, &str)> {
    draft.setup.iter().map(|one| (one.name.as_str(), one.run.as_str())).collect()
}

const VITE_APP: &str = "{\"scripts\": {\"dev\": \"vite\"}}";
const FASTAPI: &str = "from fastapi import FastAPI\n\napp = FastAPI()\n";

#[test]
fn a_remembered_command_naming_what_only_a_jobs_worktree_held_is_not_proposed() {
    let repository = a_repository(
        &[
            ("requirements.txt", "fastapi\nuvicorn\n"),
            ("backend/app/main.py", FASTAPI),
            ("frontend/package.json", VITE_APP),
            ("frontend/package-lock.json", "{}"),
        ],
        &[("frontend/vite.demo.config.mjs", "export default {}\n")],
    );
    let facts = [
        fact(".", ".venv/bin/uvicorn backend.app.main:app --port 8000 --log-level info", Some(8000)),
        fact("frontend", "npx vite --config vite.demo.config.mjs", None),
    ];

    let draft = drafted(repository.path(), &facts).expect("a draft");

    assert_eq!(
        serves(&draft),
        [
            ("backend", ".venv/bin/uvicorn backend.app.main:app --port ${port.backend} --log-level info", None),
            ("frontend", "npx vite --port ${port.frontend} --strictPort", Some("frontend")),
        ],
        "`.venv` is made by the setup proposed; the config file was a Job's scratch, so package.json's dev stands"
    );
    assert_eq!(
        setup(&draft),
        [
            ("venv", "python3 -m venv .venv"),
            ("pip", ".venv/bin/python -m pip install -r requirements.txt"),
            ("install_frontend", "npm ci --prefix frontend"),
        ]
    );

    // With the requirements in `backend/`, nothing proposed makes a `.venv` at the root.
    let elsewhere = a_repository(
        &[("backend/requirements.txt", "fastapi\nuvicorn\n"), ("backend/main.py", FASTAPI)],
        &[],
    );
    let draft = drafted(elsewhere.path(), &facts[..1]).expect("a draft");
    assert_eq!(
        serves(&draft),
        [("backend", ".venv/bin/python -m uvicorn main:app --reload --port ${port.backend}", Some("backend"))],
        "the remembered program is not proposed, and the app found in the files is"
    );
    assert_eq!(setup(&draft)[0], ("venv_backend", "python3 -m venv backend/.venv"));

    // A program the repository tracks is proposed as it was remembered.
    let tracked = a_repository(&[("scripts/serve.sh", "#!/bin/sh\n")], &[]);
    let draft = drafted(tracked.path(), &[fact(".", "scripts/serve.sh --port 4100", Some(4100))]).expect("a draft");
    assert_eq!(serves(&draft), [("scripts", "scripts/serve.sh --port ${port.scripts}", None)]);
}

#[test]
fn each_lockfile_beside_a_package_json_and_each_python_folder_proposes_its_install_from_the_root() {
    let repository = a_repository(
        &[
            ("package.json", "{\"private\": true}"),
            ("pnpm-lock.yaml", ""),
            ("site/package.json", VITE_APP),
            ("site/yarn.lock", ""),
            ("docs/package.json", "{}"),
            ("docs/bun.lock", ""),
            ("worker/pyproject.toml", "[project]\nname = \"worker\"\n"),
            ("loose/package.json", "{}"),
        ],
        &[],
    );

    let draft = drafted(repository.path(), &[]).expect("a draft");

    assert_eq!(
        setup(&draft),
        [
            ("install", "pnpm install --frozen-lockfile"),
            ("install_docs", "bun install --frozen-lockfile --cwd docs"),
            ("install_site", "yarn --cwd site install --frozen-lockfile"),
            ("venv_worker", "python3 -m venv worker/.venv"),
            ("pip_worker", "worker/.venv/bin/python -m pip install -e worker"),
        ],
        "a package.json with no lockfile beside it proposes nothing"
    );
    assert_eq!(draft.setup[2], DraftSetup {
        name: String::from("install_site"),
        run: String::from("yarn --cwd site install --frozen-lockfile"),
        creates: Some(String::from("site/node_modules")),
    });
}

#[test]
fn every_remembered_process_listens_on_a_leased_port_named_for_its_folder() {
    let dir = TempDir::new();
    let facts = [
        fact("web", "npm run dev", None),
        fact("admin", "npx vite --port 5000", None),
        fact("api", "uvicorn main:app --reload", None),
        fact("worker", "pnpm run start", None),
    ];

    let draft = drafted(dir.path(), &facts).expect("a draft");

    assert_eq!(
        serves(&draft),
        [
            ("web", "npm run dev -- --port ${port.web}", Some("web")),
            ("admin", "npx vite --port ${port.admin} --strictPort", Some("admin")),
            ("api", "uvicorn main:app --reload --port ${port.api}", Some("api")),
            ("worker", "pnpm run start --port ${port.worker}", Some("worker")),
        ]
    );
    let ports: Vec<Option<DraftPort>> = draft.processes.iter().map(|one| one.port.clone()).collect();
    assert_eq!(ports[0], Some(DraftPort { name: String::from("web"), env: Some(String::from("PORT")) }));
    assert_eq!(ports[1], Some(DraftPort { name: String::from("admin"), env: None }));
}

const REPOSITORY: &str = "version: 1\nid: budget-app\ncommands:\n  bootstrap:\n    run: make bootstrap\nsetup:\n  requires: [bootstrap]\n";

#[tokio::test]
async fn a_saved_draft_writes_its_setup_into_kits_copy_after_the_repositorys_and_the_next_draft_does_not_propose_it_again() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, REPOSITORY, REPOSITORY);
    write(home.path(), "frontend/package.json", VITE_APP);
    write(home.path(), "frontend/package-lock.json", "{}");

    let draft = AppRuns::get_app_run_draft(&*fleet, None).await.expect("drafted");
    let proposed: Vec<(&str, &str)> = draft.setup.iter().map(|one| (one.name.as_str(), one.command.as_str())).collect();
    assert_eq!(proposed, [("install_frontend", "npm ci --prefix frontend")], "carried on the wire for the confirm to list");

    let mut confirmed = drafted(home.path(), &[]).expect("a draft");
    confirmed.setup[0].run = String::from("npm install --prefix frontend");
    let saved = fleet.run_saved(None, confirmed).await.expect("it saves");

    assert_eq!(std::fs::read_to_string(home.path().join("armada.yml")).expect("read"), REPOSITORY, "never the repository's file");
    let kit = std::fs::read_to_string(&saved.saved_in.file).expect("Kit's file");
    assert!(kit.contains("install_frontend") && kit.contains("setup:"), "{kit}");
    let manifest = fleet.run_manifest(&fleet.first(), fleet.first().root()).await.expect("it reads");
    let required: Vec<(&str, &str)> = manifest.prepared_by().iter().map(|one| (one.name(), one.run())).collect();
    assert_eq!(required, [("bootstrap", "make bootstrap"), ("install_frontend", "npm install --prefix frontend")]);

    // A folder with no run yet: the draft has a process, and the install saved with an edited line
    // is not proposed beside it again.
    write(home.path(), "admin/package.json", VITE_APP);
    let again = fleet.run_drafted(None).await.expect("drafted again").draft.expect("admin is new");
    assert_eq!(serves(&again), [("admin", "npx vite --port ${port.admin} --strictPort", Some("admin"))]);
    assert!(again.setup.is_empty(), "{:?}", again.setup);
}
