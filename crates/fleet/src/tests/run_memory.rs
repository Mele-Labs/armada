//! How a repository runs, remembered from what its Jobs ran: read off the stream with no press,
//! kept where the repository's place says, a failed guess left out, and a dropped line never back.

use adapters::session_runs::{Reading, Record};
use api::{Commands, Queries};
use ipc::{DropRunFact, RunFactKind, RunFactState, RunMemory, RunMemoryPlace, RunPlace};

use super::session_host::{eventually, rig_placing, Rig};
use crate::run_memory::PLACE_REFUSED;
use crate::session_host::Heard;

fn call(id: &str, command: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"{id}\",\"name\":\"Bash\",\"input\":{{\"command\":\"{command}\"}}}}]}}}}"
    )
}

fn result(id: &str, printed: &str) -> String {
    let failed = printed.starts_with("Exit code ");
    let flag = if failed { ",\"is_error\":true" } else { "" };
    format!(
        "{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"{id}\"{flag},\"content\":\"{}\"}}]}}}}",
        printed.replace('\n', "\\n")
    )
}

/// What the stream's lines for one command come to, as the process seam hands them over.
pub(super) fn ran(id: &str, command: &str, printed: &str) -> Vec<Record> {
    let mut reading = Reading::default();
    reading.feed(call(id, command).as_bytes(), 1_790_848_800_000);
    reading.feed(result(id, printed).as_bytes(), 1_790_848_800_000);
    reading.records
}

pub(super) async fn running(rig: &Rig) -> ipc::SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    id
}

/// The turn ends, which is when the session's runs are learnt.
pub(super) async fn turn_over(rig: &Rig, id: &ipc::SessionId) {
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(id).await == ipc::SessionTurn::Idle }).await;
}

pub(super) async fn read(rig: &Rig) -> RunMemory {
    rig.fleet.get_run_memory(rig.root.clone()).await.expect("read")
}

pub(super) fn commands(memory: &RunMemory) -> Vec<(String, RunFactKind, String)> {
    memory
        .workspaces
        .iter()
        .flat_map(|one| one.runs.iter().map(|run| (run.fact.workspace.clone(), run.fact.kind, run.fact.command.clone())))
        .collect()
}

/// What the last process of the Job `id` is told in its system prompt, besides the job rules.
async fn told(rig: &Rig, id: &ipc::SessionId) -> String {
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    rig.stand_in.starts().into_iter().rev().find(|one| one.session == id.as_str()).and_then(|one| one.note).unwrap_or_default()
}

/// Another Job on the same repository, started and sent its first line.
async fn the_next_job(rig: &Rig) -> ipc::SessionId {
    let next = rig.start().await;
    rig.send(&next, "go").await;
    next
}

fn tests_fact() -> (String, RunFactKind, String) {
    (".".to_string(), RunFactKind::Tests, "cargo test".to_string())
}

#[tokio::test]
async fn a_command_a_job_ran_is_remembered_in_the_kit_without_a_press() {
    let rig = rig_placing(None);
    let id = running(&rig).await;

    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "test result: ok")));
    turn_over(&rig, &id).await;
    eventually(|| async { !commands(&read(&rig).await).is_empty() }).await;

    let memory = read(&rig).await;
    assert_eq!(commands(&memory), vec![tests_fact()]);
    let run = &memory.workspaces[0].runs[0];
    assert_eq!((run.fact.source.as_str(), run.fact.succeeded), ("jobs", true));
    assert_eq!((run.state, run.kept_in), (RunFactState::Kept, Some(RunPlace::ThisMac)));
    let kit = std::path::Path::new(&rig.fleet.host().kit_home).join("runs");
    let files: Vec<_> = std::fs::read_dir(&kit).expect("the Kit holds a runs folder").collect();
    assert_eq!(files.len(), 1, "one file for the repository");
    let text = std::fs::read_to_string(files[0].as_ref().expect("a file").path()).expect("readable");
    assert!(text.contains("cargo test") && text.contains("kind = \"tests\""), "{text}");
}

#[tokio::test]
async fn a_dev_server_is_kept_with_the_port_it_listened_on() {
    let rig = rig_placing(None);
    let id = running(&rig).await;

    rig.stand_in.hears(0, Heard::Ran(ran("d1", "pnpm dev", "ready on http://localhost:5173")));
    turn_over(&rig, &id).await;
    eventually(|| async { !commands(&read(&rig).await).is_empty() }).await;

    let fact = read(&rig).await.workspaces[0].runs[0].fact.clone();
    assert_eq!((fact.kind, fact.port), (RunFactKind::DevServer, Some(5173)));
}

#[tokio::test]
async fn a_command_that_only_ever_failed_is_not_kept() {
    let rig = rig_placing(None);
    let id = running(&rig).await;

    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "Exit code 101\nerror: no such crate")));
    rig.stand_in.hears(0, Heard::Ran(ran("l1", "cargo clippy", "no warnings")));
    turn_over(&rig, &id).await;
    eventually(|| async { !commands(&read(&rig).await).is_empty() }).await;

    assert_eq!(commands(&read(&rig).await), vec![(".".to_string(), RunFactKind::Lint, "cargo clippy".to_string())]);
}

#[tokio::test]
async fn a_dropped_line_is_not_offered_or_written_again_even_when_a_job_runs_it_again() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "ok")));
    turn_over(&rig, &id).await;
    eventually(|| async { !commands(&read(&rig).await).is_empty() }).await;

    let after = rig
        .fleet
        .drop_run_fact(DropRunFact {
            repository: rig.root.clone(),
            workspace: ".".into(),
            kind: RunFactKind::Tests,
            command: "cargo test".into(),
        })
        .await
        .expect("dropped");
    assert!(commands(&after).is_empty());

    rig.stand_in.hears(0, Heard::Ran(ran("t2", "cargo test", "ok")));
    rig.stand_in.finishes(0, "again");
    // The turn after it has been learnt: a fact that was going to come back would be here by now.
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    rig.fleet.runs_learnt(id.as_str(), &rig.fleet.hosts().of(id.as_str())).await;
    assert!(commands(&read(&rig).await).is_empty());
    let kit = std::path::Path::new(&rig.fleet.host().kit_home).join("runs");
    for file in std::fs::read_dir(kit).expect("the folder") {
        let text = std::fs::read_to_string(file.expect("a file").path()).expect("readable");
        assert!(!text.contains("cargo test"), "{text}");
    }
}

#[tokio::test]
async fn the_next_job_in_the_repository_is_told_the_commands_and_the_port_the_last_one_ran() {
    let rig = rig_placing(None);
    let first = running(&rig).await;
    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "test result: ok")));
    rig.stand_in.hears(0, Heard::Ran(ran("d1", "pnpm dev", "ready on http://localhost:5173")));
    turn_over(&rig, &first).await;
    eventually(|| async { commands(&read(&rig).await).len() == 2 }).await;

    let note = told(&rig, &the_next_job(&rig).await).await;

    assert!(note.contains("`cargo test`"), "{note}");
    assert!(note.contains("`pnpm dev`") && note.contains("port 5173"), "{note}");
    assert!(note.contains("never a Check"), "nothing remembered gates a pull request: {note}");
}

#[tokio::test]
async fn a_job_started_before_the_last_turn_ended_is_told_a_command_that_already_came_back() {
    let rig = rig_placing(None);
    let _first = running(&rig).await;
    rig.stand_in.hears(0, Heard::Ran(ran("d1", "pnpm dev", "ready on http://localhost:5173")));
    // The first Job's turn does not end.
    eventually(|| async { !commands(&read(&rig).await).is_empty() }).await;

    let note = told(&rig, &the_next_job(&rig).await).await;

    assert!(note.contains("`pnpm dev`") && note.contains("port 5173"), "{note}");
}

#[tokio::test]
async fn a_dropped_line_is_not_told_to_the_next_job() {
    let rig = rig_placing(None);
    let first = running(&rig).await;
    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "ok")));
    rig.stand_in.hears(0, Heard::Ran(ran("l1", "cargo clippy", "no warnings")));
    turn_over(&rig, &first).await;
    eventually(|| async { commands(&read(&rig).await).len() == 2 }).await;
    rig.fleet
        .drop_run_fact(DropRunFact {
            repository: rig.root.clone(),
            workspace: ".".into(),
            kind: RunFactKind::Tests,
            command: "cargo test".into(),
        })
        .await
        .expect("dropped");

    let note = told(&rig, &the_next_job(&rig).await).await;

    assert!(note.contains("`cargo clippy`"), "{note}");
    assert!(!note.contains("cargo test"), "{note}");
}

#[tokio::test]
async fn the_place_is_this_mac_until_one_is_chosen() {
    let rig = rig_placing(None);
    let first = rig.fleet.get_run_memory_place(rig.root.clone()).await.expect("read");
    assert_eq!((first.place, first.synced_folder), (RunPlace::ThisMac, None));

    let chosen = RunMemoryPlace { repository: rig.root.clone(), place: RunPlace::Repository, synced_folder: None, cloud: None };
    rig.fleet.set_run_memory_place(chosen.clone()).await.expect("chosen");
    assert_eq!(rig.fleet.get_run_memory_place(rig.root.clone()).await.expect("read"), chosen);

    let nowhere = RunMemoryPlace { place: RunPlace::Synced, synced_folder: Some("/no/such/folder".into()), ..chosen };
    let refused = rig.fleet.set_run_memory_place(nowhere).await.expect_err("not a folder");
    assert!(matches!(&refused, api::Refusal::Unacceptable(error) if error.code == PLACE_REFUSED), "{refused:?}");
    let unset = RunMemoryPlace { repository: rig.root.clone(), place: RunPlace::Synced, synced_folder: None, cloud: None };
    assert!(rig.fleet.set_run_memory_place(unset).await.is_err(), "a synced place needs its folder");
}

#[tokio::test]
async fn a_synced_place_gets_the_same_layout_under_the_folder_the_person_named() {
    let rig = rig_placing(None);
    let synced = crate::tests::tmp::TempDir::new();
    let id = running(&rig).await;
    rig.fleet
        .set_run_memory_place(RunMemoryPlace {
            repository: rig.root.clone(),
            place: RunPlace::Synced,
            synced_folder: Some(synced.path().to_string_lossy().to_string()),
            cloud: None,
        })
        .await
        .expect("chosen");

    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "ok")));
    turn_over(&rig, &id).await;
    eventually(|| async { synced.path().join("runs").is_dir() }).await;

    eventually(|| async { read(&rig).await.workspaces.first().is_some_and(|one| one.runs[0].kept_in == Some(RunPlace::Synced)) }).await;
    assert!(!std::path::Path::new(&rig.fleet.host().kit_home).join("runs").exists(), "Kit holds none of it");
}

#[tokio::test]
async fn the_repository_place_writes_armada_yml_on_the_jobs_branch_and_commits_that_file_only() {
    let rig = rig_placing(None);
    let tree = crate::tests::tmp::TempDir::new();
    std::fs::write(tree.path().join("armada.yml"), "version: 1\nid: shop\n").expect("a manifest");
    rig.fleet
        .set_run_memory_place(RunMemoryPlace { repository: rig.root.clone(), place: RunPlace::Repository, synced_folder: None, cloud: None })
        .await
        .expect("chosen");
    let mut live = ran("t1", "cargo test", "ok");
    live.extend(ran("d1", "pnpm dev", "listening on localhost:5173"));
    let id = rig.start().await;
    rig.fleet.hosts().of(id.as_str()).state().runs.heard(live, &tree.path().to_string_lossy());

    // No branch yet: the facts wait, and say what for.
    let runtime = rig.fleet.hosts().of(id.as_str());
    runtime.state().directory = tree.path().to_string_lossy().to_string();
    rig.fleet.runs_learnt(id.as_str(), &runtime).await;
    let waiting = read(&rig).await;
    assert!(waiting.workspaces[0].runs.iter().all(|run| run.state == RunFactState::Waiting && run.why.is_some()));

    let job = crate::run_memory::JobTree { path: tree.path().to_string_lossy().to_string(), branch: "job/one".into() };
    rig.fleet.run_memory_written(&rig.root, Some(job)).await;

    let written = std::fs::read_to_string(tree.path().join("armada.yml")).expect("read");
    assert!(written.contains("commands:") && written.contains("run: cargo test"), "{written}");
    assert!(!written.contains("checks:"), "nothing remembered gates a pull request: {written}");
    assert!(written.contains("commands:") && written.contains("serve: pnpm dev"), "{written}");
    assert!(written.contains("http://localhost:5173"), "{written}");
    let commits = rig.fleet.vcs().committed();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].scope, testkit::CommitScope::Paths(vec!["armada.yml".to_string()]));
    assert!(read(&rig).await.workspaces[0].runs.iter().all(|run| run.kept_in == Some(RunPlace::Repository)));
}

#[tokio::test]
async fn a_line_armada_yml_already_declares_is_never_overwritten() {
    let tree = crate::tests::tmp::TempDir::new();
    std::fs::write(tree.path().join("armada.yml"), "version: 1\nid: shop\ncommands:\n  test:\n    run: make check\n").expect("a manifest");
    let facts = vec![store::KeptRun {
        workspace: ".".into(),
        kind: "tests".into(),
        command: "cargo test".into(),
        port: None,
        succeeded: true,
        last_ran: 1,
        source: "jobs".into(),
        kept_in: None,
        why: None,
    }];

    let written = crate::run_memory::manifest::write(tree.path(), &facts);

    assert!(written.changed.is_empty());
    assert!(written.standing[0].1.as_ref().expect_err("waits").contains("already declares `commands.test`"));
    let text = std::fs::read_to_string(tree.path().join("armada.yml")).expect("read");
    assert!(text.contains("make check") && !text.contains("cargo test"), "{text}");
}

#[tokio::test]
async fn what_a_folders_past_sessions_showed_is_merged_the_first_time_the_repository_is_touched() {
    let rig = rig_placing(None);
    let dir = std::path::Path::new(&rig.fleet.host().home)
        .join(adapters::session_folders::PROJECTS)
        .join(adapters::session_folders::directory_name(&rig.root));
    std::fs::create_dir_all(&dir).expect("the directory");
    let cwd = &rig.root;
    std::fs::write(
        dir.join("one.jsonl"),
        [
            format!("{{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-10-01T10:00:00.000Z\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"1\",\"name\":\"Bash\",\"input\":{{\"command\":\"cargo fmt\"}}}}]}}}}"),
            result("1", "done"),
        ]
        .join("\n"),
    )
    .expect("the record");

    let memory = read(&rig).await;

    let sources: Vec<_> = memory.workspaces.iter().flat_map(|one| one.runs.iter().map(|run| run.fact.source.clone())).collect();
    assert_eq!(sources, vec![adapters::session_runs::SOURCE.to_string()]);
}
