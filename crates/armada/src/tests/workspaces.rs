//! `armada covers`, `armada check <dir>:<name>` and the merge line's setup,
//! over a scratch repository holding a root and workspaces `lib`, `a`, `b`
//! (`a` and `b` depend on `lib`).

use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

use checks_runner::Priority;

use crate::declared::{covering, execute, Asked, Reached, Registry};
use crate::land::Env;
use crate::tests::TempDir;

const BUDGET: Duration = Duration::from_secs(30);

fn workspace(dir: &TempDir, name: &str, depends: &str, extra: &str) {
    dir.write(&format!("{name}/package.json"), "{}\n");
    dir.write(
        &format!("{name}/armada.yml"),
        &format!(
            "version: 1\nid: {name}\n{depends}\
             checks:\n  test:\n    run: /usr/bin/touch ran-test\n    when: [\"src/**\"]\n  \
               lint:\n    run: /usr/bin/touch ran-lint\n{extra}"
        ),
    );
}

fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: root\n\
         checks:\n  rust:\n    run: /usr/bin/touch ran-rust\n    when: [\"crates/**\"]\n\
         commands:\n  bootstrap:\n    run: /usr/bin/touch bootstrapped\n  \
           rooted:\n    run: /usr/bin/touch rooted\n",
    );
    workspace(&dir, "lib", "", "");
    let on_lib = "depends_on: [\"lib/**\"]\n";
    let setup = "commands:\n  gen:\n    run: /usr/bin/touch generated\n\
                 setup:\n  requires: [bootstrap, gen, rooted]\n";
    workspace(&dir, "a", on_lib, setup);
    workspace(&dir, "b", on_lib, "");
    dir
}

fn hits(dir: &TempDir, paths: &[&str]) -> Vec<String> {
    let changed: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    covering(dir.path(), &changed).expect("the manifests read")
}

#[test]
fn a_path_in_a_workspace_names_that_workspaces_checks_and_no_others() {
    let dir = a_repository();
    assert_eq!(hits(&dir, &["a/src/x.ts"]), ["a:test", "a:lint"]);
    // `lint` declares no `when`, so it reads every path a owns.
    assert_eq!(hits(&dir, &["a/README.md"]), ["a:lint"]);
}

#[test]
fn a_path_in_lib_names_lib_and_what_depends_on_it() {
    let dir = a_repository();
    // `test` has a `when` over a's own `src/**`, which lib's path is not.
    // Workspaces come in directory order.
    assert_eq!(
        hits(&dir, &["lib/src/x.ts"]),
        ["a:lint", "b:lint", "lib:test", "lib:lint"]
    );
}

#[test]
fn a_root_only_path_names_the_roots_checks_only() {
    let dir = a_repository();
    assert_eq!(hits(&dir, &["crates/x/src/lib.rs"]), ["rust"]);
    assert!(hits(&dir, &["docs/INDEX.md"]).is_empty());
}

#[test]
fn a_change_across_the_root_and_a_workspace_names_the_roots_first() {
    let dir = a_repository();
    assert_eq!(
        hits(&dir, &["a/src/x.ts", "crates/x/src/lib.rs"]),
        ["rust", "a:test", "a:lint"]
    );
}

#[test]
fn no_paths_name_nothing_where_there_are_workspaces() {
    assert!(hits(&a_repository(), &[]).is_empty());
}

/// No workspace `armada.yml` is the answer it always was, an empty diff
/// included.
#[test]
fn a_repository_with_no_workspaces_answers_as_it_did() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: plain\nchecks:\n  test:\n    run: /usr/bin/true\n  \
           ui:\n    run: /usr/bin/true\n    when: [\"packages/**\"]\n",
    );
    dir.write("sub/package.json", "{}\n");
    assert_eq!(hits(&dir, &[]), ["test"]);
    assert_eq!(hits(&dir, &["sub/x", "packages/a"]), ["test", "ui"]);
}

async fn check(dir: &TempDir, key: &str, asked: Asked<'_>) -> crate::declared::Ran {
    execute(
        dir.path(),
        Registry::Checks,
        key,
        asked,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("declared")
}

#[tokio::test]
async fn a_workspace_check_runs_in_its_directory_and_the_bare_name_in_the_roots() {
    let dir = a_repository();
    let ran = check(&dir, "a:lint", Asked::Whole).await;
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
    assert_eq!(ran.name, "a:lint");
    assert!(dir.path().join("a/ran-lint").exists());
    assert!(!dir.path().join("ran-lint").exists());

    assert_eq!(check(&dir, "rust", Asked::Whole).await.status(), 0);
    assert!(dir.path().join("ran-rust").exists());
}

#[tokio::test]
async fn a_workspace_checks_prerequisites_and_one_test_run_in_its_directory() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: root\n");
    dir.write("w/package.json", "{}\n");
    dir.write(
        "w/armada.yml",
        "version: 1\nid: w\ncommands:\n  gen:\n    run: /usr/bin/touch generated\n\
         checks:\n  test:\n    run: /usr/bin/false\n    requires: [gen]\n    \
           one_test:\n      run: /usr/bin/touch only-{}\n",
    );
    let ran = check(&dir, "w:test", Asked::OneTest("alpha")).await;
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
    assert!(dir.path().join("w/generated").exists());
    assert!(dir.path().join("w/only-alpha").exists());
    assert!(!dir.path().join("generated").exists());
}

/// **Narrowing reads what the workspace owns, relative to it**, and a path
/// the root owns is not the workspace's to narrow over.
#[tokio::test]
async fn a_workspace_checks_narrowing_is_relative_to_its_directory() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: root\n");
    dir.write("w/package.json", "{}\n");
    dir.write(
        "w/armada.yml",
        "version: 1\nid: w\nchecks:\n  test:\n    run: /usr/bin/false\n    \
           narrow:\n      run: /bin/echo narrowed\n      each: \"-p {}\"\n      under: src\n    \
           when: [\"src/**\"]\n",
    );
    let paths = ["w/src/api/x.ts".to_string(), "docs/a.md".to_string()];
    let ran = check(&dir, "w:test", Asked::Changed(&paths)).await;
    assert_eq!(ran.command, "/bin/echo narrowed -p api");
    assert_eq!(ran.narrowed, Some(Reached::To("-p api".to_string())));
}

#[tokio::test]
async fn a_key_for_a_workspace_that_is_not_there_is_not_declared() {
    let dir = a_repository();
    let refused = execute(
        dir.path(),
        Registry::Checks,
        "nope:test",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("no such workspace")
    .to_string();
    assert!(refused.contains("nope:test"), "{refused}");
}

/// The merge line's setup: the root's names once, then each gating workspace's
/// own. A root Command a workspace names runs in the root, and not twice.
#[test]
fn the_merge_line_sets_up_the_root_once_then_each_gating_workspace() {
    let dir = a_repository();
    let log = dir.path().join("calls.log");
    dir.write(
        "stub",
        &format!(
            "#!/bin/sh\necho \"$PWD $*\" >> {}\n",
            log.to_str().expect("a UTF-8 path")
        ),
    );
    let stub = dir.path().join("stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let logs = TempDir::new();

    let mut env = Env::read();
    env.armada = stub.to_str().expect("a UTF-8 path").to_string();
    env.setup = vec!["bootstrap".to_string()];
    let keys: Vec<String> = ["a:test", "a:lint", "b:lint"].map(String::from).to_vec();
    crate::land::prepare::setup_for(dir.path(), &env, logs.path(), &keys).expect("sets up");

    let root = dir.path().display();
    let calls = std::fs::read_to_string(&log).expect("the stub ran");
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        [
            format!("{root} run bootstrap"),
            format!("{root} run a:gen"),
            format!("{root} run rooted"),
        ],
        "b declares no setup, and the root's bootstrap is not run twice"
    );
}
