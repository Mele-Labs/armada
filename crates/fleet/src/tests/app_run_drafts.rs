//! A run drafted for a repository that declares none, and a confirmed draft written down.
//! **Facts are the shapes real sessions left**, trailing shell and all, under made-up names.

use std::path::Path;

use store::KeptRun;

use crate::app_runs::draft::{drafted, DraftPort, DraftProcess, DraftSource, RunDraft};
use crate::app_runs::save::{amended, into_file, SavedIn};
use crate::app_runs::RunTarget;
use crate::tests::servers::a_fleet_holding;
use crate::tests::tmp::TempDir;

fn fact(workspace: &str, command: &str, port: Option<u16>, last_ran: u64, succeeded: bool) -> KeptRun {
    KeptRun {
        workspace: workspace.to_string(),
        kind: String::from("dev_server"),
        command: command.to_string(),
        port,
        succeeded,
        last_ran,
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

fn serves(draft: &RunDraft) -> Vec<(&str, &str, Option<&str>)> {
    draft.processes.iter().map(|one| (one.name.as_str(), one.serve.as_str(), one.dir.as_deref())).collect()
}

#[test]
fn a_remembered_dev_server_loses_the_shell_around_it_and_its_port_becomes_a_name() {
    let dir = TempDir::new();
    for (said, port, expected) in [
        (
            "npx vite --config vite.mock.config.ts --port 5176 --strictPort /tmp/slot2-mock.log &)",
            5176,
            "npx vite --config vite.mock.config.ts --port ${port.vite} --strictPort",
        ),
        (
            "nohup pnpm exec vite --port 5199 --strictPort /tmp/vite-file-panel.log &",
            5199,
            "pnpm exec vite --port ${port.vite} --strictPort",
        ),
        (
            "pnpm exec vite --port 5199 --strictPort /tmp/vite-fp2.log & echo $! /tmp/vite-fp2.pid)",
            5199,
            "pnpm exec vite --port ${port.vite} --strictPort",
        ),
        ("npx vite --port=5373", 5373, "npx vite --port=${port.vite}"),
    ] {
        let draft = drafted(dir.path(), &[fact(".", said, Some(port), 1, true)]).expect("a draft");
        assert_eq!(draft.processes[0].serve, expected, "{said}");
        assert_eq!(draft.processes[0].port, Some(DraftPort { name: String::from("vite"), env: None }));
    }
}

#[test]
fn a_port_number_inside_a_longer_number_is_left_alone() {
    let dir = TempDir::new();
    let draft = drafted(dir.path(), &[fact(".", "serve --id 15173 --port 5173", Some(5173), 1, true)]).expect("a draft");
    assert_eq!(draft.processes[0].serve, "serve --id 15173 --port ${port.serve}");
}

#[test]
fn one_process_per_folder_the_newest_that_succeeded_and_a_slots_copy_is_the_same_folder() {
    let dir = TempDir::new();
    let facts = [
        fact("apps/web", "npx vite --port 5391 --strictPort", Some(5391), 10, true),
        fact(".armada/slots/slot-4/apps/web", "npx vite --port 5392 --strictPort", Some(5392), 30, true),
        fact("apps/web", "npx vite --port 5393 --strictPort", Some(5393), 40, false),
        fact("/elsewhere/app", "npx vite --port 5394", Some(5394), 50, true),
    ];
    let draft = drafted(dir.path(), &facts).expect("a draft");
    assert_eq!(serves(&draft), [("web", "npx vite --port ${port.web} --strictPort", Some("apps/web"))]);
}

#[test]
fn a_fact_that_cannot_start_by_exec_is_not_proposed() {
    let dir = TempDir::new();
    let facts = [
        fact("a", "cd frontend npm run dev", None, 1, true),
        fact("b", "PORT=3000 npm run dev", Some(3000), 1, true),
    ];
    assert_eq!(drafted(dir.path(), &facts), None);
}

#[test]
fn a_remembered_frontend_and_a_python_backend_from_the_files_make_one_run() {
    let dir = TempDir::new();
    write(dir.path(), "backend/requirements.txt", "fastapi\nuvicorn[standard]\n");
    write(dir.path(), "backend/main.py", "from fastapi import FastAPI\n\napp: FastAPI = FastAPI()\n");
    write(dir.path(), ".venv/bin/python", "");
    write(dir.path(), "frontend/package.json", "{\"scripts\": {\"dev\": \"vite\"}}");
    let facts = [fact("frontend", "npx vite --port 5373 --strictPort", Some(5373), 1, true)];

    let draft = drafted(dir.path(), &facts).expect("a draft");

    assert_eq!(
        serves(&draft),
        [
            ("frontend", "npx vite --port ${port.frontend} --strictPort", Some("frontend")),
            (
                "backend",
                "../.venv/bin/python -m uvicorn main:app --reload --port ${port.backend}",
                Some("backend")
            ),
        ],
        "memory first, and frontend's package.json is not read twice"
    );
    assert_eq!(draft.processes[1].from, DraftSource::Python);
    assert_eq!(draft.name, "dev");
    assert_eq!(draft.label.as_deref(), dir.path().file_name().map(|name| name.to_string_lossy().into_owned()).as_deref());
}

#[test]
fn a_package_json_dev_script_takes_its_port_the_way_the_script_reads_it() {
    for (lock, script, expected, env) in [
        ("pnpm-lock.yaml", "vite", "pnpm exec vite --port ${port.web} --strictPort", None),
        ("package-lock.json", "vite --host", "npx vite --host --port ${port.web} --strictPort", None),
        ("yarn.lock", "next dev", "yarn dev", Some("PORT")),
        ("package-lock.json", "node server.js", "npm run dev", Some("PORT")),
        ("pnpm-lock.yaml", "vite --port 4000", "pnpm run dev", None),
    ] {
        let dir = TempDir::new();
        write(dir.path(), lock, "");
        write(dir.path(), "package.json", &format!("{{\n  \"name\": \"shop\",\n  \"scripts\": {{\n    \"build\": \"tsc\",\n    \"dev\": \"{script}\"\n  }}\n}}\n"));
        let draft = drafted(dir.path(), &[]).expect("a draft");
        let process = &draft.processes[0];
        assert_eq!((process.serve.as_str(), process.dir.as_deref()), (expected, None), "{script}");
        assert_eq!(process.port.as_ref().and_then(|port| port.env.as_deref()), env, "{script}");
        assert_eq!(process.port.is_some(), script != "vite --port 4000", "{script}");
    }
}

#[test]
fn a_root_dev_script_is_the_app_and_libraries_are_not() {
    let dir = TempDir::new();
    write(dir.path(), "package.json", "{\"scripts\":{\"dev\":\"turbo dev\"}}");
    write(dir.path(), "apps/site/package.json", "{\"scripts\":{\"dev\":\"vite\"}}");
    write(dir.path(), "packages/ui/package.json", "{\"scripts\":{\"dev\":\"tsup --watch\"}}");
    let draft = drafted(dir.path(), &[]).expect("a draft");
    assert_eq!(serves(&draft), [("web", "npm run dev", None)]);

    let dir = TempDir::new();
    write(dir.path(), "package.json", "{\"private\":true}");
    write(dir.path(), "apps/site/package.json", "{\"scripts\":{\"dev\":\"vite\"}}");
    write(dir.path(), "packages/ui/package.json", "{\"scripts\":{\"dev\":\"tsup --watch\"}}");
    let draft = drafted(dir.path(), &[]).expect("a draft");
    assert_eq!(serves(&draft), [("site", "npx vite --port ${port.site} --strictPort", Some("apps/site"))]);
}

#[test]
fn a_repository_whose_files_name_no_server_has_no_draft() {
    let dir = TempDir::new();
    write(dir.path(), "package.json", "{\"scripts\":{\"build\":\"tsc\"}}");
    write(dir.path(), "README.md", "# shop\n");
    assert_eq!(drafted(dir.path(), &[]), None);
}

#[test]
fn two_folders_with_one_name_are_told_apart_and_the_port_follows_the_name() {
    let dir = TempDir::new();
    let facts = [
        fact("a/web", "npx vite --port 5001", Some(5001), 1, true),
        fact("b/web", "npx vite --port 5002", Some(5002), 1, true),
    ];
    let draft = drafted(dir.path(), &facts).expect("a draft");
    assert_eq!(
        serves(&draft),
        [("web", "npx vite --port ${port.web}", Some("a/web")), ("web_2", "npx vite --port ${port.web_2}", Some("b/web"))]
    );
    assert_eq!(draft.processes[1].port.as_ref().map(|port| port.name.as_str()), Some("web_2"));
}

fn a_draft() -> RunDraft {
    let process = |name: &str, serve: &str, dir: &str| DraftProcess {
        name: name.to_string(),
        serve: serve.to_string(),
        dir: Some(dir.to_string()),
        port: Some(DraftPort { name: name.to_string(), env: (name == "backend").then(|| String::from("PORT")) }),
        from: DraftSource::RunMemory,
    };
    RunDraft {
        name: String::from("dev"),
        label: Some(String::from("Budget app")),
        processes: vec![
            process("backend", "python3 -m uvicorn main:app --port ${port.backend}", "backend"),
            process("frontend", "npx vite --port ${port.frontend} --strictPort", "frontend"),
        ],
    }
}

const COMMENTED: &str = "# How budget-app runs.\nversion: 1\nid: budget-app\n\n# One test command.\nchecks:\n  test:\n    run: pytest -q\n";

#[test]
fn a_saved_run_is_in_the_file_with_its_ports_servers_and_folders_and_every_comment_stays() {
    let dir = TempDir::new();
    let file = dir.path().join("armada.yml");
    std::fs::write(&file, COMMENTED).expect("written");

    into_file(&file, &a_draft()).expect("it saves");

    let text = std::fs::read_to_string(&file).expect("read");
    assert!(text.starts_with("# How budget-app runs.\nversion: 1"), "{text}");
    assert!(text.contains("# One test command.\nchecks:"), "{text}");
    let manifest = config::Manifest::parse(&file, &text).expect("it loads");
    let run = manifest.app_run("dev").expect("the run");
    assert_eq!((run.label(), run.servers()), (Some("Budget app"), &[String::from("backend"), String::from("frontend")][..]));
    assert_eq!(manifest.server("frontend").and_then(|server| server.dir()), Some("frontend"));
    assert_eq!(manifest.server("frontend").map(|server| server.links().len()), Some(1), "a link to its port");
    assert_eq!(manifest.port("backend").and_then(|port| port.env()), Some("PORT"));
    assert!(manifest.port("frontend").is_some());
    assert!(!file.with_extension("yml.writing").exists(), "the staging file went");
}

#[test]
fn a_draft_naming_something_the_file_declares_or_that_will_not_load_is_refused_and_writes_nothing() {
    let dir = TempDir::new();
    let file = dir.path().join("armada.yml");
    let text = format!("{COMMENTED}commands:\n  frontend:\n    run: npm run build\n");
    std::fs::write(&file, &text).expect("written");

    let refused = into_file(&file, &a_draft()).expect_err("already declared");
    assert!(refused.to_string().contains("`commands.frontend` is already declared"), "{refused}");
    assert_eq!(std::fs::read_to_string(&file).expect("read"), text, "the file is as it was");

    let changes: [(fn(&mut RunDraft), &str); 5] = [
        (|draft: &mut RunDraft| draft.name = String::from("two words"), "not a name for a run"),
        (|draft: &mut RunDraft| draft.processes.clear(), "at least one server"),
        (|draft: &mut RunDraft| draft.processes[0].name = String::from("Bad Name"), "not a name for a server"),
        (|draft: &mut RunDraft| draft.processes[0].serve = String::from("  "), "nothing to serve"),
        (|draft: &mut RunDraft| draft.processes[1].dir = Some(String::from("../elsewhere")), "folder a server starts in"),
    ];
    for (change, wanted) in changes {
        let mut draft = a_draft();
        change(&mut draft);
        let refused = amended(&file, COMMENTED, &draft, &[]).expect_err("refused");
        assert!(refused.to_string().contains(wanted), "{wanted}: {refused}");
    }
}

#[tokio::test]
async fn a_repository_with_a_file_gets_the_run_in_it_and_is_told_it_waits_for_a_restart() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, COMMENTED, COMMENTED);

    let saved = fleet.run_saved(None, a_draft()).await.expect("it saves");

    assert_eq!(saved.name, "dev");
    assert_eq!(saved.saved_in, SavedIn::Repository(home.path().join("armada.yml")));
    assert!(saved.takes_effect_at_restart, "Fleet holds the Manifest it started with");
    let text = std::fs::read_to_string(home.path().join("armada.yml")).expect("read");
    assert!(text.contains("runs:") && text.contains("# One test command."), "{text}");
    let target = RunTarget::Main(None);
    let held = fleet.runs_listed(&target).await.expect("listed");
    assert!(held.is_empty(), "the running Fleet still declares none: {held:?}");
}

#[tokio::test]
async fn a_repository_with_no_file_gets_a_whole_copy_in_kit_that_stands_as_its_manifest() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, COMMENTED, COMMENTED);
    std::fs::remove_file(home.path().join("armada.yml")).expect("a repository with none");

    let saved = fleet.run_saved(None, a_draft()).await.expect("it saves");

    let SavedIn::Kit(label) = &saved.saved_in else { panic!("expected Kit's copy: {saved:?}") };
    let text = std::fs::read_to_string(label).expect("Kit's file");
    assert!(text.contains("version: 1") && text.contains("id: "), "a whole Manifest: {text}");
    let manifest = config::Manifest::from_kit_alone(&home.path().join("armada.yml"), Path::new(label), &text).expect("it loads");
    assert_eq!(manifest.app_run("dev").map(|run| run.servers().len()), Some(2));

    // A second run goes into the copy that is there, and keeps its id.
    let mut second = a_draft();
    second.name = String::from("other");
    for process in &mut second.processes {
        process.name = format!("{}_2", process.name);
        process.port = process.port.take().map(|port| DraftPort { name: format!("{}_2", port.name), ..port });
        process.serve = process.serve.replace("}", "_2}");
    }
    fleet.run_saved(None, second).await.expect("it saves again");
    let again = std::fs::read_to_string(label).expect("Kit's file");
    let manifest = config::Manifest::from_kit_alone(&home.path().join("armada.yml"), Path::new(label), &again).expect("it loads");
    assert_eq!(manifest.app_run_names(), ["dev", "other"]);
    assert_eq!(text.lines().find(|line| line.starts_with("id: ")), again.lines().find(|line| line.starts_with("id: ")));
}

#[tokio::test]
async fn a_draft_comes_from_what_the_stores_facts_say_and_only_where_no_run_is_declared() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, COMMENTED, COMMENTED);
    let root = fleet.first().root().to_string();
    {
        let mut store = fleet.store().lock().await;
        store.set_run_memory_place(&root, "this_mac", None, None).expect("a place, so nothing is imported");
        store
            .merge_run_facts(&root, &[fact("frontend", "npx vite --port 5373 --strictPort /tmp/x.log &)", Some(5373), 1, true)])
            .expect("kept");
    }

    let draft = fleet.run_drafted(None).await.expect("drafted").expect("a draft");
    assert_eq!(serves(&draft), [("frontend", "npx vite --port ${port.frontend} --strictPort", Some("frontend"))]);
    assert!(std::fs::read_to_string(home.path().join("armada.yml")).expect("read") == COMMENTED, "nothing was saved");

    let declared = format!("{COMMENTED}ports:\n  web: {{}}\ncommands:\n  web:\n    serve: node s.js\nruns:\n  app:\n    commands: [web]\n");
    let fleet = a_fleet_holding(&TempDir::new(), &api::Broadcaster::new(), &declared, &declared);
    let refused = fleet.run_drafted(None).await.expect_err("one is declared");
    let api::Refusal::IllegalMove(why) = refused else { panic!("expected a conflict") };
    assert_eq!(why.code, "fleet.app_runs_already_saved");
}
