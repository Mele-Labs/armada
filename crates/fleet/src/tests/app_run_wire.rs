//! A run on the wire: spelled as `list_app_runs` and its acts answer, told again as
//! `app_run.changed` after each server fact, and drafted and saved in the words a person reads.
//! **Every start here is a real process**, for `crate::tests::servers`' reason.

use std::sync::Arc;
use std::time::Duration;

use api::AppRuns;
use ipc::{
    AppRun, AppRunCheckout, AppRunDraftProcess, AppRunOpensIn, AppRunState, Event, NamedAppRun,
    RunPlace, SaveAppRun,
};
use store::KeptRun;

use crate::tests::app_runs::{a_fleet_with_folders, answers, git};
use crate::tests::servers::{a_fleet_holding, next_event};
use crate::tests::tmp::TempDir;

/// `site` serves a page and offers it as a link; `quiet` serves one and offers nothing.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  site: {}
  quiet: {}
commands:
  site:
    serve: python3 -m http.server ${port.site} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.site}/
    links:
      - url: http://localhost:${port.site}
        name: Site
  quiet:
    serve: python3 -m http.server ${port.quiet} --bind 127.0.0.1
runs:
  page:
    label: The page
    commands: [site]
  background:
    commands: [quiet]
"#;

fn named(name: &str, job: Option<&str>) -> NamedAppRun {
    NamedAppRun { name: name.to_string(), job_id: job.map(str::to_string) }
}

/// The next `app_run.changed` for `name` that `wanted` accepts, or a panic that says it never came.
async fn changed(watching: &mut api::Subscription, name: &str, wanted: impl Fn(&AppRun) -> bool) -> AppRun {
    let waiting = next_event(watching, |event| matches!(event, Event::AppRunChanged(run) if run.name == name && wanted(run)));
    match tokio::time::timeout(Duration::from_secs(30), waiting).await {
        Ok(Event::AppRunChanged(run)) => run,
        Ok(_) => unreachable!("filtered"),
        Err(_) => panic!("no `app_run.changed` for {name} that was wanted"),
    }
}

#[tokio::test]
async fn a_run_is_spelled_for_the_wire_and_told_again_after_each_of_its_servers_facts() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_holding(&home, &events, MANIFEST, MANIFEST);
    crate::app_runs::keep_the_runs(Arc::clone(&fleet));
    let mut watching = events.subscribe();
    let root = fleet.first().root().to_string();

    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert_eq!(listed.iter().map(|run| run.name.as_str()).collect::<Vec<_>>(), ["page", "background"]);
    let page = &listed[0];
    assert_eq!((page.label.as_str(), page.opens_in), ("The page", AppRunOpensIn::Browser));
    assert_eq!(listed[1].opens_in, AppRunOpensIn::App, "a server that offers no link opens nowhere");
    assert_eq!(page.manifest_id.as_str(), "01FIXTUREMANIFEST");
    assert_eq!(page.checkout, AppRunCheckout::Main { path: root.clone() });
    assert_eq!((page.processes[0].state, page.processes[0].server_id.as_deref()), (AppRunState::NotStarted, None));
    assert!(page.last_used.is_none(), "nothing has been used yet");

    let started = Arc::clone(&fleet).start_app_run(None, named("page", None)).await.expect("it starts");
    let id = started.processes[0].server_id.clone().expect("an instance to observe");
    assert!(matches!(started.processes[0].state, AppRunState::Starting | AppRunState::Ready));

    // The server's own fact is on the stream first, and the run's follows it.
    next_event(&mut watching, |event| matches!(event, Event::ServerServing(state) if state.id == id)).await;
    let ready = changed(&mut watching, "page", |run| run.processes[0].state == AppRunState::Ready).await;
    let port = ready.processes[0].port.expect("the leased port");
    assert_eq!(ready.processes[0].server_id.as_deref(), Some(id.as_str()));
    assert_eq!(ready.processes[0].links[0].url, format!("http://localhost:{port}"));
    assert!(answers(&format!("http://127.0.0.1:{port}/")));
    assert!(ready.last_used.is_some());

    let stopped = fleet.stop_app_run(None, named("page", None)).await.expect("it stops");
    assert_eq!(stopped.processes[0].state, AppRunState::Exited);
    assert!(stopped.processes[0].links.is_empty(), "an address is offered only while it is ready");
    let exited = changed(&mut watching, "page", |run| run.processes[0].state == AppRunState::Exited).await;
    assert_eq!(exited.processes[0].ended, stopped.processes[0].ended);
    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert!(listed[0].last_used.is_some() && listed[1].last_used.is_none(), "only the run that was used says when");

    let refused = Arc::clone(&fleet).start_app_run(None, named("nothing", None)).await.expect_err("no such run");
    assert_eq!(crate::tests::app_runs::code(refused), "fleet.not_an_app_run");
}

/// A Job's worktree is found from Fleet's record of its lease, for an event that names only a path.
#[tokio::test]
async fn a_run_on_a_jobs_worktree_names_the_job_its_branch_and_its_bay() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_with_folders(&home, &events);
    crate::app_runs::keep_the_runs(Arc::clone(&fleet));
    let root = std::path::PathBuf::from(fleet.first().root());
    git(&root, &["init", "-q"]);
    git(&root, &["commit", "-q", "--allow-empty", "-m", "start"]);
    let slot = adapter_traits::slot_path(&root.to_string_lossy(), 1);
    git(&root, &["worktree", "add", "-q", "-b", "job-branch", &slot]);
    for folder in ["backend", "frontend"] {
        std::fs::create_dir_all(std::path::Path::new(&slot).join(folder)).expect("a folder in the worktree");
    }
    let at = "2026-10-25T10:00:00.000Z";
    {
        let mut store = fleet.store().lock().await;
        store
            .keep_session(&store::KeptSession {
                id: String::from("01JOB"),
                harness: String::from(adapters::HOSTED_HARNESS),
                origin: String::from("bridge"),
                manifest_id: Some(String::from("01FIXTUREMANIFEST")),
                cwd: String::new(),
                title: Some(String::from("A Job")),
                state: store::SessionState::Live,
                started_at: at.to_string(),
                last_seen_at: at.to_string(),
                last_turn_at: None,
                ended_at: None,
                end_reason: None,
                figures: Default::default(),
                mod_version: None,
            })
            .expect("a session");
        let mut hosting = store::KeptHosting {
            session_id: String::from("01JOB"),
            manifest_id: String::from("01FIXTUREMANIFEST"),
            model: None,
            effort: None,
            mode: String::from("auto"),
            ran: true,
            lease_slot: Some(1),
            lease_branch: Some(String::from("job-branch")),
            fork_of: None,
            helm: false,
            setup: false,
            workflow: None,
            proposed: String::new(),
            turn_model: None,
            turn_effort: None,
            paused_at: None,
            paused_resuming: false,
            auto_land: false,
            mod_job: false,
            start_held: false,
        };
        store.keep_hosting(&hosting).expect("a hosting");
        // A Job with no bay has nothing to run.
        hosting.session_id = String::from("01BAYLESS");
        hosting.lease_slot = None;
        let kept = store.session("01JOB").expect("reads").expect("kept");
        store.keep_session(&store::KeptSession { id: String::from("01BAYLESS"), ..kept }).expect("a session");
        store.keep_hosting(&hosting).expect("a hosting");
    }
    let mut watching = events.subscribe();

    let listed = AppRuns::list_app_runs(&*fleet, None, Some(String::from("01JOB"))).await.expect("listed").runs;
    let AppRunCheckout::Job { job_id, branch, bay, .. } = &listed[0].checkout else { panic!("a Job's checkout: {:?}", listed[0].checkout) };
    assert_eq!((job_id.as_str(), branch.as_deref(), *bay), ("01JOB", Some("job-branch"), 1));

    Arc::clone(&fleet).start_app_run(None, named("app", Some("01JOB"))).await.expect("it starts on the worktree");
    // An event names the path alone, and the lease says whose it is.
    let told = changed(&mut watching, "app", |run| run.processes.iter().all(|one| one.state == AppRunState::Ready)).await;
    let AppRunCheckout::Job { job_id, bay, .. } = &told.checkout else { panic!("a Job's checkout: {:?}", told.checkout) };
    assert_eq!((job_id.as_str(), *bay), ("01JOB", 1));

    let bayless = AppRuns::list_app_runs(&*fleet, None, Some(String::from("01BAYLESS"))).await.expect_err("nothing to run");
    assert_eq!(crate::tests::app_runs::code(bayless), "fleet.job_has_no_worktree");
    fleet.stop_app_run(None, named("app", Some("01JOB"))).await.expect("it stops");
}

fn a_fact() -> KeptRun {
    KeptRun {
        workspace: String::from("frontend"),
        kind: String::from("dev_server"),
        command: String::from("npx vite --port 5373 --strictPort /tmp/x.log &)"),
        port: Some(5373),
        succeeded: true,
        last_ran: 1,
        source: String::from("jobs"),
        kept_in: None,
        why: None,
    }
}

#[tokio::test]
async fn a_draft_reads_its_port_as_a_placeholder_and_a_save_gives_each_listener_a_port_of_its_own() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let bare = "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  fmt:\n    run: /usr/bin/true\n";
    let fleet = a_fleet_holding(&home, &events, bare, bare);
    let root = fleet.first().root().to_string();
    {
        let mut store = fleet.store().lock().await;
        store.set_run_memory_place(&root, "this_mac", None, None).expect("a place, so nothing is imported");
        store.merge_run_facts(&root, &[a_fact()]).expect("kept");
    }

    let draft = AppRuns::get_app_run_draft(&*fleet, None).await.expect("drafted");

    assert_eq!((draft.root.as_str(), draft.place), (root.as_str(), RunPlace::ThisMac));
    assert_eq!(
        draft.processes,
        [AppRunDraftProcess {
            name: String::from("frontend"),
            dir: Some(String::from("frontend")),
            command: String::from("npx vite --port ${port} --strictPort"),
            port_env: None,
        }]
    );

    let process = |name: &str, command: &str| AppRunDraftProcess {
        name: name.to_string(),
        dir: None,
        command: command.to_string(),
        port_env: None,
    };
    let saved = Arc::clone(&fleet)
        .save_app_run(
            None,
            SaveAppRun {
                name: String::from("Web app"),
                processes: vec![
                    process("web", "python3 -m http.server ${port} --bind 127.0.0.1"),
                    process("worker", "sleep 300"),
                ],
                setup: Vec::new(),
                job_id: None,
            },
        )
        .await
        .expect("it saves and starts");

    assert_eq!(saved.place, RunPlace::ThisMac);
    assert!(saved.file.ends_with(".yml"), "{}", saved.file);
    assert_eq!((saved.run.name.as_str(), saved.run.label.as_str()), ("web_app", "Web app"));
    assert_eq!(saved.run.opens_in, AppRunOpensIn::Browser, "a listener a link was written for");
    let [web, worker] = &saved.run.processes[..] else { panic!("two processes: {:?}", saved.run.processes) };
    assert!(web.port.is_some(), "the command that held `${{port}}` listens");
    assert_eq!(worker.port, None, "and the one that held none listens on none");
    assert!(
        std::fs::read_to_string(&saved.file).expect("Kit's copy").contains("${port.web}"),
        "the placeholder became the Manifest's own"
    );

    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert_eq!(listed.iter().map(|run| run.name.as_str()).collect::<Vec<_>>(), ["web_app"]);
    fleet.stop_app_run(None, named("web_app", None)).await.expect("it stops");
}
