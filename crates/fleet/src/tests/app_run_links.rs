//! A process expecting another on a port it does not get: found in a folder's files while a run is
//! listed or drafted, found in a refused connection while it runs, kept in run memory, and ended
//! when the file stops saying it. Names are made up. **Starts are real processes**, for
//! `crate::tests::servers`' reason.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::AppRuns;
use ipc::{AppRun, AppRunLink, Event, NamedAppRun, PinAppRunPort, UnpinAppRunPort};
use store::KeptRun;

use crate::app_runs::links::{self, Changes, Link};
use crate::tests::app_run_pins::{a_free_port, said};
use crate::tests::servers::{a_fleet_holding, next_event};
use crate::tests::tmp::TempDir;

/// `api` listens where its `.env` says, and `web` is the frontend that proxies to it.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  api:
    env: API_PORT
  web: {}
commands:
  api:
    serve: python3 -m http.server ${port.api} --bind 127.0.0.1
    dir: backend
  web:
    serve: python3 -m http.server ${port.web} --bind 127.0.0.1
    dir: frontend
runs:
  app:
    commands: [api, web]
"#;

const PROXYING: &str = "export default { server: { proxy: { '/api': 'http://127.0.0.1:8731' } } }\n";

fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("made");
    std::fs::write(path, text).expect("written");
}

fn the_proxy_link() -> AppRunLink {
    AppRunLink {
        from: String::from("web"),
        to: String::from("api"),
        port: 8731,
        found_in: String::from("frontend/vite.config.js"),
        brief: String::from("Make `web` read `api`'s port from `ARMADA_PORT_API`, falling back to `8731`, in `frontend/vite.config.js`."),
    }
}

/// A fleet on `manifest` with a place chosen, so nothing past is imported from the machine.
async fn a_fleet(home: &TempDir, events: &api::Broadcaster, manifest: &str) -> Arc<crate::tests::servers::Fixture> {
    let fleet = a_fleet_holding(home, events, manifest, manifest);
    let root = fleet.first().root().to_string();
    fleet.store().lock().await.set_run_memory_place(&root, "this_mac", None, None).expect("a place");
    fleet
}

async fn link_facts(fleet: &crate::tests::servers::Fixture) -> Vec<KeptRun> {
    let root = fleet.first().root().to_string();
    fleet.store().lock().await.run_facts(&root).expect("read").into_iter().filter(|fact| fact.kind == links::KIND).collect()
}

fn candidate(name: &str, dir: &str, command: &str) -> links::Candidate {
    links::Candidate { name: name.to_string(), dir: Some(dir.to_string()), command: command.to_string(), pinned: None }
}

#[test]
fn a_port_a_folder_expects_that_a_sibling_is_known_on_is_a_link_once_per_pair() {
    let dir = TempDir::new();
    write(dir.path(), "backend/.env", "PORT=8731\n");
    write(dir.path(), "frontend/vite.config.js", PROXYING);
    // A second file naming the same port is the same pair, and one naming its own port is nothing.
    write(dir.path(), "frontend/.env", "VITE_API=http://localhost:8731\nVITE_SELF=http://localhost:5173\n");
    let both = [candidate("web", "frontend", "vite --port ${port.web}"), candidate("api", "backend", "uvicorn app:app --port ${port.api}")];
    let found = links::found(dir.path(), &both, &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].from.as_str(), found[0].to.as_str(), found[0].port), ("web", "api", 8731));
    assert_eq!(found[0].found_in, "frontend/vite.config.js", "the first file in the fixed order that holds it");

    // The port a sibling's command writes is as known as one its config does.
    let in_the_command = [candidate("web", "frontend", "vite"), candidate("api", "backend", "uvicorn app:app --port 8731")];
    std::fs::remove_file(dir.path().join("backend/.env")).expect("removed");
    assert_eq!(links::found(dir.path(), &in_the_command, &[]).len(), 1);
    // And a port nobody is known on is no link.
    assert!(links::found(dir.path(), &[candidate("web", "frontend", "vite"), candidate("api", "backend", "uvicorn")], &[]).is_empty());
}

#[test]
fn a_link_ends_only_where_the_file_no_longer_holds_it_and_a_log_link_gives_way_to_a_file_that_finds_it() {
    let link = |found_in: &str| Link {
        from: String::from("web"),
        to: String::from("api"),
        port: 8731,
        found_in: found_in.to_string(),
        workspace: String::from("frontend"),
    };
    let scanned = [String::from("web")];
    // Fixed in the file, on the main checkout: it ends.
    let gone = links::reconciled(&[link("frontend/vite.config.js")], &[], &scanned, true);
    assert_eq!(gone, Changes { add: Vec::new(), end: vec![link("frontend/vite.config.js")] });
    // Fixed in a bay's copy only: the memory both share is left alone.
    assert_eq!(links::reconciled(&[link("frontend/vite.config.js")], &[], &scanned, false), Changes::default());
    // One read from output is not a file's to end.
    assert_eq!(links::reconciled(&[link(links::LOG)], &[], &scanned, true), Changes::default());
    // Until a file finds the same pair, which says where.
    let better = links::reconciled(&[link(links::LOG)], &[link("frontend/.env")], &scanned, true);
    assert_eq!((better.add, better.end), (vec![link("frontend/.env")], vec![link(links::LOG)]));
}

#[tokio::test]
async fn a_proxy_target_in_a_config_file_is_listed_kept_and_ended_when_the_file_stops_holding_it() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet(&home, &events, MANIFEST).await;
    let root = fleet.first().root().to_string();
    write(home.path(), "backend/.env", "PORT=8731\n");
    write(home.path(), "frontend/vite.config.js", PROXYING);

    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert_eq!(listed[0].links, [the_proxy_link()]);
    let kept = link_facts(&fleet).await;
    assert_eq!(kept.len(), 1);
    assert!(kept[0].succeeded && kept[0].port == Some(8731) && kept[0].workspace == "frontend", "{kept:?}");

    // The person mends the file: the link is gone from the list, and stays gone though the place
    // that keeps the memory is read and merged back over it.
    write(home.path(), "frontend/vite.config.js", "export default { server: { proxy: { '/api': `http://127.0.0.1:${process.env.ARMADA_PORT_API}` } } }\n");
    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert!(listed[0].links.is_empty(), "{:?}", listed[0].links);
    fleet.run_memory_written(&root, None).await;
    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert!(listed[0].links.is_empty(), "an ended link did not come back from the file");
    assert!(link_facts(&fleet).await.iter().all(|fact| !fact.succeeded), "it is kept as ended, not deleted");
}

#[tokio::test]
async fn a_draft_lists_where_a_proposed_process_expects_another_and_the_next_draft_remembers_it() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let plain = "version: 1\nid: 01FIXTUREMANIFEST\n";
    let fleet = a_fleet(&home, &events, plain).await;
    let root = fleet.first().root().to_string();
    write(home.path(), "frontend/package.json", "{\"scripts\": {\"dev\": \"vite\"}}");
    write(home.path(), "frontend/vite.config.js", PROXYING);
    let said_dev = KeptRun {
        workspace: String::from("backend"),
        kind: String::from("dev_server"),
        command: String::from("uvicorn app:app --port 8731"),
        port: Some(8731),
        succeeded: true,
        last_ran: 1,
        source: String::from("jobs"),
        kept_in: None,
        why: None,
    };
    fleet.store().lock().await.merge_run_facts(&root, &[said_dev]).expect("kept");

    let draft = AppRuns::get_app_run_draft(&*fleet, None).await.expect("drafted");
    assert_eq!(draft.processes.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["backend", "frontend"]);
    assert_eq!(draft.links.len(), 1, "{:?}", draft.links);
    let link = &draft.links[0];
    assert_eq!((link.from.as_str(), link.to.as_str(), link.port), ("frontend", "backend", 8731));
    assert_eq!(link.found_in, "frontend/vite.config.js");
    assert!(link.brief.contains("`ARMADA_PORT_BACKEND`") && link.brief.ends_with("in `frontend/vite.config.js`."), "{}", link.brief);

    // Without the file, the next draft still says it: it was kept.
    std::fs::remove_file(home.path().join("frontend/vite.config.js")).expect("removed");
    let again = AppRuns::get_app_run_draft(&*fleet, None).await.expect("drafted");
    assert_eq!(again.links, draft.links);
}

#[tokio::test]
async fn a_process_refused_a_port_a_sibling_is_known_on_is_a_link_told_on_the_run_once() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    // `web` prints the refusal twice and stays up; its folder holds no file that names the port.
    let manifest = MANIFEST.replace(
        "serve: python3 -m http.server ${port.web} --bind 127.0.0.1",
        "serve: \"/bin/sh -c 'echo Error: connect ECONNREFUSED 127.0.0.1:8731; echo Error: connect ECONNREFUSED 127.0.0.1:8731; sleep 60'\"",
    );
    let fleet = a_fleet(&home, &events, &manifest).await;
    crate::app_runs::keep_the_runs(Arc::clone(&fleet));
    let mut watching = events.subscribe();
    write(home.path(), "backend/.env", "PORT=8731\n");
    std::fs::create_dir_all(home.path().join("frontend")).expect("the folder `frontend` starts in");

    Arc::clone(&fleet).start_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it starts");
    let waiting = next_event(&mut watching, |event| matches!(event, Event::AppRunChanged(run) if !run.links.is_empty()));
    let told = tokio::time::timeout(Duration::from_secs(20), waiting).await.expect("the link is told on the run");
    let Event::AppRunChanged(run) = told else { unreachable!("filtered") };
    assert_eq!(run.links.len(), 1);
    let link = &run.links[0];
    assert_eq!((link.from.as_str(), link.to.as_str(), link.port, link.found_in.as_str()), ("web", "api", 8731, "log"));
    assert!(link.brief.contains("`ARMADA_PORT_API`") && link.brief.contains("refused connection to port 8731"), "{}", link.brief);

    let kept = link_facts(&fleet).await;
    assert_eq!(kept.len(), 1, "one link for the pair: {kept:?}");
    assert!(kept[0].command.ends_with(" in log") && kept[0].succeeded, "{kept:?}");
    // The list says what the event did, with the run still up.
    let listed = AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs;
    assert_eq!(listed[0].links.len(), 1);

    fleet.stop_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it stops");
}

#[tokio::test]
async fn a_refusal_of_a_port_nobody_is_known_on_finds_nothing() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    // 8732 names no process.
    let manifest = MANIFEST.replace(
        "serve: python3 -m http.server ${port.web} --bind 127.0.0.1",
        "serve: \"/bin/sh -c 'echo Error: connect ECONNREFUSED 127.0.0.1:8732; sleep 60'\"",
    );
    let fleet = a_fleet(&home, &events, &manifest).await;
    crate::app_runs::keep_the_runs(Arc::clone(&fleet));
    let mut watching = events.subscribe();
    write(home.path(), "backend/.env", "PORT=8731\n");

    Arc::clone(&fleet).start_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it starts");
    next_event(&mut watching, |event| matches!(event, Event::AppRunChanged(run) if run.processes[1].server_id.is_some())).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(link_facts(&fleet).await.is_empty());
    fleet.stop_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it stops");
}

fn pin(process: &str, port: u16) -> PinAppRunPort {
    PinAppRunPort { process: process.to_string(), port, name: None, job_id: None }
}

#[tokio::test]
async fn a_pin_is_written_into_kits_copy_with_the_ports_own_keys_and_an_unpin_takes_it_out_again() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet(&home, &events, MANIFEST).await;
    let root = fleet.first().root().to_string();
    let number = a_free_port();

    let pinned = fleet.pin_app_run_port(None, pin("api", number)).await.expect("pinned");
    assert_eq!(pinned.processes[0].pinned_port, Some(number));
    assert_eq!(pinned.processes[1].pinned_port, None);
    assert_eq!(std::fs::read_to_string(home.path().join("armada.yml")).expect("read"), MANIFEST, "the repository's file is as it was");
    let (_, kit) = fleet.kit_manifest_read(&root).await.expect("read");
    let config::KitRead::Text(kit) = kit else { panic!("Kit's copy was written: {kit:?}") };
    assert!(kit.contains(&format!("fixed: {number}")) && kit.contains("env: API_PORT"), "the port keeps what the repository gave it: {kit}");

    // It starts on exactly that number, and a listing says it is pinned.
    let started = Arc::clone(&fleet).start_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it starts");
    assert_eq!(started.processes[0].port, Some(number));
    fleet.stop_app_run(None, NamedAppRun { name: String::from("app"), job_id: None }).await.expect("it stops");

    let unpinned = fleet.unpin_app_run_port(None, UnpinAppRunPort { process: String::from("api"), name: None, job_id: None }).await.expect("unpinned");
    assert_eq!(unpinned.processes[0].pinned_port, None);
    let (_, kit) = fleet.kit_manifest_read(&root).await.expect("read");
    let config::KitRead::Text(kit) = kit else { panic!("Kit's copy is still there") };
    assert!(!kit.contains("fixed"), "{kit}");
}

#[tokio::test]
async fn an_unpin_ends_what_was_remembered_of_a_process_and_a_pin_that_cannot_be_is_refused_with_its_code() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet(&home, &events, MANIFEST).await;
    let root = fleet.first().root().to_string();
    let from_output = Link {
        from: String::from("web"),
        to: String::from("api"),
        port: 8731,
        found_in: String::from(links::LOG),
        workspace: String::from("frontend"),
    };
    fleet.links_kept(&root, Changes { add: vec![from_output], end: Vec::new() }).await;
    assert_eq!(AppRuns::list_app_runs(&*fleet, None, None).await.expect("listed").runs[0].links.len(), 1);

    fleet.pin_app_run_port(None, pin("api", a_free_port())).await.expect("pinned");
    let unpinned = fleet.unpin_app_run_port(None, UnpinAppRunPort { process: String::from("api"), name: None, job_id: None }).await.expect("unpinned");
    assert!(unpinned.links.is_empty(), "{:?}", unpinned.links);
    assert!(links::remembered(&link_facts(&fleet).await).is_empty());

    for (process, port) in [("nothing", 9000), ("api", 80)] {
        let refused = fleet.pin_app_run_port(None, pin(process, port)).await.expect_err("refused");
        let sentence = said(refused.clone());
        assert_eq!(crate::tests::app_runs::code(refused), "fleet.app_run_pin_refused", "{sentence}");
    }
}
