//! What differs in a workspace's `armada.yml`: `depends_on`, a `when` read from
//! its own directory, root Commands in `setup.requires`, and no
//! `setup.worktrees`.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const ROOT: &str = "version: 1\nid: root\ncommands:\n  bootstrap:\n    run: pnpm install\n  \
                    dev:\n    run: pnpm dev\n    serve: pnpm dev\n";
const HEAD: &str = "version: 1\nid: a\n";
const DIR: &str = "packages/a";

fn root() -> Manifest {
    Manifest::parse(&named("armada.yml"), ROOT).expect("the root")
}

fn workspace(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse_workspace(&named("packages/a/armada.yml"), DIR, text, &root())
}

fn paths(written: &[&str]) -> Vec<String> {
    written.iter().map(|path| path.to_string()).collect()
}

// ------------------------------------------------------------- depends_on

#[test]
fn depends_on_is_read_in_the_order_written() {
    let manifest = workspace(&format!(
        "{HEAD}depends_on: [\"packages/lib/**\", pnpm-lock.yaml]\n"
    ))
    .expect("depends_on");
    let written: Vec<&str> = manifest.depends_on().iter().map(|p| p.as_str()).collect();
    assert_eq!(written, ["packages/lib/**", "pnpm-lock.yaml"]);
}

#[test]
fn a_manifest_with_no_depends_on_depends_on_nothing() {
    assert!(workspace(HEAD).expect("bare").depends_on().is_empty());
    assert!(root().depends_on().is_empty());
}

#[test]
fn an_empty_depends_on_is_refused_as_an_empty_when_is() {
    let refused = refusals(workspace(&format!("{HEAD}depends_on: []\n")));
    assert!(matches!(fault_at(&refused, "depends_on"), Fault::Empty));
}

#[test]
fn a_depends_on_pattern_from_another_dialect_is_refused() {
    let refused = refusals(workspace(&format!(
        "{HEAD}depends_on: [\"lib/**/*.{{ts,tsx}}\"]\n"
    )));
    assert!(matches!(
        fault_at(&refused, "depends_on[0]"),
        Fault::NotAPathPattern { .. }
    ));
}

#[test]
fn the_root_may_write_depends_on_too() {
    let text = format!("{HEAD}depends_on: [\"packages/**\"]\n");
    let manifest = Manifest::parse(&named("armada.yml"), &text).expect("root depends_on");
    assert_eq!(manifest.depends_on().len(), 1);
}

// ------------------------------------------------------- when, by directory

const WHEN: &str = "checks:\n  test:\n    run: pnpm test\n    when: [\"src/**\"]\n";

#[test]
fn a_workspaces_when_is_read_from_its_own_directory() {
    let manifest = workspace(&format!("{HEAD}{WHEN}")).expect("a when");
    let test = manifest.check("test").expect("test");
    assert!(manifest.reaches(test, &paths(&["packages/a/src/index.ts"])));
    assert!(!manifest.reaches(test, &paths(&["packages/a/README.md"])));
}

#[test]
fn a_workspaces_when_ignores_paths_outside_its_directory() {
    let manifest = workspace(&format!("{HEAD}{WHEN}")).expect("a when");
    let test = manifest.check("test").expect("test");
    assert!(!manifest.reaches(test, &paths(&["packages/b/src/index.ts"])));
    // `src/**` from the repository root is a different path, not this one.
    assert!(!manifest.reaches(test, &paths(&["src/index.ts"])));
    // A directory that merely starts with the same letters is not below it.
    assert!(!manifest.reaches(test, &paths(&["packages/a-two/src/index.ts"])));
}

#[test]
fn the_roots_when_stays_repository_relative() {
    let manifest = Manifest::parse(&named("armada.yml"), &format!("{HEAD}{WHEN}")).expect("root");
    let test = manifest.check("test").expect("test");
    assert!(manifest.reaches(test, &paths(&["src/index.ts"])));
    assert!(!manifest.reaches(test, &paths(&["packages/a/src/index.ts"])));
}

#[test]
fn a_check_with_no_when_reaches_any_change_in_a_workspace() {
    let text = format!("{HEAD}checks:\n  test:\n    run: pnpm test\n");
    let manifest = workspace(&text).expect("no when");
    let test = manifest.check("test").expect("test");
    assert!(manifest.reaches(test, &paths(&["packages/lib/src/index.ts"])));
}

// --------------------------------------------------------- setup.requires

#[test]
fn setup_requires_in_a_workspace_may_name_a_root_command() {
    let manifest = workspace(&format!("{HEAD}setup:\n  requires: [bootstrap]\n")).expect("root");
    let prepared = manifest.prepared_by();
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name(), "bootstrap");
    assert_eq!(prepared[0].run(), "pnpm install");
}

#[test]
fn a_workspaces_own_command_wins_over_the_roots_of_the_same_name() {
    let text = format!(
        "{HEAD}commands:\n  bootstrap:\n    run: pnpm install --filter a\n\
         setup:\n  requires: [bootstrap]\n"
    );
    let manifest = workspace(&text).expect("own wins");
    assert_eq!(manifest.prepared_by()[0].run(), "pnpm install --filter a");
}

#[test]
fn a_root_command_is_still_refused_in_a_root_file_that_does_not_declare_it() {
    let refused = refusals(Manifest::parse(
        &named("armada.yml"),
        &format!("{HEAD}setup:\n  requires: [bootstrap]\n"),
    ));
    assert!(matches!(
        fault_at(&refused, "setup.requires[0]"),
        Fault::NotADeclaredCommand { .. }
    ));
}

#[test]
fn a_root_server_is_refused_in_setup_requires_as_a_servers_own_is() {
    let refused = refusals(workspace(&format!("{HEAD}setup:\n  requires: [dev]\n")));
    assert!(matches!(
        fault_at(&refused, "setup.requires[0]"),
        Fault::RequiresAServer { .. }
    ));
}

#[test]
fn a_name_neither_file_declares_is_refused_and_lists_the_commands_it_could_have_been() {
    let refused = refusals(workspace(&format!("{HEAD}setup:\n  requires: [nope]\n")));
    let Fault::NotADeclaredCommand { declared, .. } = fault_at(&refused, "setup.requires[0]")
    else {
        panic!("expected NotADeclaredCommand, got {refused:?}");
    };
    assert_eq!(declared, &["bootstrap"]);
}

#[test]
fn a_checks_requires_does_not_reach_the_roots_commands() {
    let text = format!("{HEAD}checks:\n  t:\n    run: x\n    requires: [bootstrap]\n");
    let refused = refusals(workspace(&text));
    assert!(matches!(
        fault_at(&refused, "checks.t.requires[0]"),
        Fault::NotADeclaredCommand { .. }
    ));
}

// ------------------------------------------------------- setup.worktrees

#[test]
fn setup_worktrees_is_refused_in_a_workspace() {
    let refused = refusals(workspace(&format!("{HEAD}setup:\n  worktrees: 3\n")));
    assert_eq!(fault_at(&refused, "setup.worktrees"), &Fault::RootOnly);
}

#[test]
fn setup_worktrees_is_still_read_in_the_root() {
    let text = format!("{HEAD}setup:\n  worktrees: 3\n");
    let manifest = Manifest::parse(&named("armada.yml"), &text).expect("root worktrees");
    assert_eq!(manifest.worktrees().get(), 3);
}

#[test]
fn a_workspace_that_says_nothing_about_worktrees_loads() {
    let manifest = workspace(&format!("{HEAD}setup:\n  requires: [bootstrap]\n")).expect("loads");
    assert_eq!(manifest.dir(), DIR);
}
