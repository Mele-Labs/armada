//! Bringing in a person's agent CLI folders: listed from the sessions it kept, how one was run, and
//! the mod install that only a request starts. The home is the fixture's own, so nothing here
//! reads a real one, and the CLI is never started.

use std::path::Path;

use adapters::session_folders::{directory_name, PROJECTS};
use adapters::session_runs::SOURCE;
use api::Refusal;
use ipc::{ImportGit, InstallAgentMod, ModInstallStep, ModInstalled, RunFactKind};
use testkit::FakeWorkProduct;

use crate::session_import::{INSTALL_NOT_REQUESTED, NOT_AN_IMPORT_FOLDER};
use crate::tests::tmp::TempDir;

/// A session record under the fixture's home, kept for `folder`.
fn kept(home: &str, folder: &Path, lines: &[String]) {
    let dir = Path::new(home).join(PROJECTS).join(directory_name(&folder.to_string_lossy()));
    std::fs::create_dir_all(&dir).expect("the directory");
    std::fs::write(dir.join("one.jsonl"), lines.join("\n")).expect("the record");
}

fn named_by(refusal: Refusal) -> String {
    match refusal {
        Refusal::Unacceptable(error) => error.code.to_string(),
        other => panic!("a 422, not {other:?}"),
    }
}

#[tokio::test]
async fn an_install_nobody_asked_for_is_refused_before_anything_runs() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));

    let refused = fleet.agent_mod_installed(InstallAgentMod { requested: false }).await;

    assert_eq!(named_by(refused.expect_err("refused")), INSTALL_NOT_REQUESTED);
    assert!(
        !crate::runtime::mod_dir(&fleet.host().home).exists(),
        "no copy of the mod was made for it either"
    );
}

#[tokio::test]
async fn a_requested_install_with_no_copy_of_the_mod_to_make_stops_there() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));

    let installed = fleet.agent_mod_installed(InstallAgentMod { requested: true }).await.expect("an answer");

    let ModInstalled::Failed { step, said } = installed else {
        panic!("nothing to install from");
    };
    assert_eq!(step, ModInstallStep::Populate);
    assert!(said.contains("holds no copy of the mod"), "{said}");
}

#[tokio::test]
async fn folders_are_listed_from_the_sessions_with_their_git_state() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let folder = home.path().join("work").join("shop");
    std::fs::create_dir_all(folder.join(".git")).expect("a repository");
    let line = format!("{{\"type\":\"user\",\"cwd\":\"{}\"}}", folder.display());
    kept(&fleet.host().home, &folder, &[line]);

    let listed = fleet.import_folders().await.expect("listed");

    assert_eq!(listed.folders.len(), 1);
    assert_eq!(listed.folders[0].path, folder.to_string_lossy());
    assert!(listed.folders[0].exists);
    assert_eq!(listed.folders[0].git, ImportGit::Root);
    assert_eq!(listed.folders[0].sessions, 1);
}

#[tokio::test]
async fn how_a_folder_runs_comes_from_its_sessions_in_the_shape_both_readers_share() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let folder = home.path().join("work").join("shop");
    std::fs::create_dir_all(&folder).expect("a folder");
    let cwd = folder.display();
    kept(
        &fleet.host().home,
        &folder,
        &[
            format!("{{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-10-01T10:00:00.000Z\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"1\",\"name\":\"Bash\",\"input\":{{\"command\":\"cargo test\"}}}}]}}}}"),
            "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"1\",\"content\":\"ok\"}]}}".to_string(),
        ],
    );

    let runs = fleet.import_runs(folder.to_string_lossy().to_string()).await.expect("read");

    assert_eq!(runs.root, folder.to_string_lossy());
    assert_eq!(runs.facts.len(), 1);
    let fact = &runs.facts[0];
    assert_eq!((fact.workspace.as_str(), fact.kind, fact.command.as_str()), (".", RunFactKind::Tests, "cargo test"));
    assert_eq!((fact.succeeded, fact.port, fact.source.as_str()), (true, None, SOURCE));
    assert_eq!(fact.last_ran.as_str(), "2026-10-01T10:00:00.000Z");
}

#[tokio::test]
async fn a_folder_no_session_names_is_refused() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));

    let refused = fleet.import_runs("/nowhere/at/all".to_string()).await;

    assert_eq!(named_by(refused.expect_err("refused")), NOT_AN_IMPORT_FOLDER);
}
