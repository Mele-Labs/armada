//! A person's own `armada.yml` for a repository, kept in Kit, over the repository's.

use std::path::Path;

use crate::manifest::{kit_copy_fault, KitKind, KitName, Manifest};

const REPOSITORY: &str = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n  \
                          lint:\n    run: make lint\ncommands:\n  dev:\n    run: make dev\n";

fn file() -> &'static Path {
    Path::new("/work/widgets/armada.yml")
}

fn kit() -> &'static Path {
    Path::new("/home/.armada/manifests/acme--widgets.yml")
}

fn named(kind: KitKind, name: &str, hides: bool) -> KitName {
    KitName { kind, name: name.to_string(), hides, added: false, gates: None }
}

fn added(kind: KitKind, name: &str, gates: Option<bool>) -> KitName {
    KitName { kind, name: name.to_string(), hides: false, added: true, gates }
}

/// The Checks `every_manifest_check` would take, by name.
fn gated(manifest: &Manifest) -> Vec<String> {
    manifest.checks_as_written().iter().filter(|name| manifest.check(name).is_some_and(|check| check.gates())).cloned().collect()
}

#[test]
fn kits_copy_is_the_manifest_where_the_repository_has_none() {
    let manifest = Manifest::from_kit_alone(file(), kit(), REPOSITORY).expect("Kit's parses");
    assert_eq!(manifest.path(), file(), "the repository is found from the Manifest, not Kit's folder");
    assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make test"));
    let from = manifest.from_kit().expect("read from Kit");
    assert!(from.whole);
    assert_eq!(from.file, kit());
    assert_eq!(
        from.names,
        [
            KitName { gates: Some(true), ..named(KitKind::Check, "test", false) },
            KitName { gates: Some(true), ..named(KitKind::Check, "lint", false) },
            named(KitKind::Command, "dev", false)
        ]
    );
}

#[test]
fn kits_checks_and_commands_replace_the_repositorys_by_name_and_the_rest_stands() {
    let mine = "checks:\n  test:\n    run: cargo nextest run\n  lint:\n    run: make lint\n  \
                e2e:\n    run: make e2e\ncommands:\n  dev:\n    run: pnpm dev\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    let run = |name: &str| manifest.check(name).map(|check| check.run().to_string());
    assert_eq!(run("test").as_deref(), Some("cargo nextest run"), "Kit's wins");
    assert_eq!(run("lint").as_deref(), Some("make lint"));
    assert_eq!(run("e2e").as_deref(), Some("make e2e"), "a name the repository does not declare is added");
    assert_eq!(manifest.id().as_str(), "01FIXTUREMANIFEST", "the rest of the repository's stands");
    assert_eq!(manifest.path(), file());
    let from = manifest.from_kit().expect("read from Kit");
    assert!(!from.whole && from.left_out.is_none());
    assert_eq!(
        from.names,
        [
            named(KitKind::Check, "test", true),
            named(KitKind::Check, "lint", false),
            added(KitKind::Check, "e2e", Some(false)),
            named(KitKind::Command, "dev", true)
        ],
        "a copy that is the same hides nothing"
    );
}

#[test]
fn a_check_only_kit_declares_is_available_and_gates_nothing() {
    let mine = "checks:\n  e2e:\n    run: make e2e\ncommands:\n  seed:\n    run: make seed\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    assert!(manifest.check("e2e").is_some(), "runnable by name");
    assert!(manifest.check_names().contains(&"e2e".to_string()), "listed");
    assert!(manifest.command("seed").is_some(), "a Command it adds is available");
    assert_eq!(gated(&manifest), ["test", "lint"], "and not part of the gate");
    let from = manifest.from_kit().expect("read from Kit");
    assert_eq!(from.names, [added(KitKind::Check, "e2e", Some(false)), added(KitKind::Command, "seed", None)]);
}

#[test]
fn a_check_only_kit_declares_gates_where_it_is_marked() {
    let mine = "checks:\n  e2e:\n    run: make e2e\n    gate: true\n  smoke:\n    run: make smoke\n    gate: false\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    assert_eq!(gated(&manifest), ["test", "lint", "e2e"]);
    let from = manifest.from_kit().expect("read from Kit");
    assert_eq!(from.names, [added(KitKind::Check, "e2e", Some(true)), added(KitKind::Check, "smoke", Some(false))]);
}

#[test]
fn a_check_added_where_the_repository_declares_no_checks_is_added() {
    let bare = "version: 1\nid: 01FIXTUREMANIFEST\n";
    let mine = "checks:\n  e2e:\n    run: make e2e\n    gate: true\n";
    let manifest = Manifest::with_kit(file(), bare, kit(), mine).expect("they merge");
    assert_eq!(gated(&manifest), ["e2e"]);
}

#[test]
fn a_check_kit_replaces_still_gates_and_may_not_say_gate() {
    let mine = "checks:\n  test:\n    run: cargo nextest run\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    assert_eq!(gated(&manifest), ["test", "lint"]);
    let marked = "checks:\n  test:\n    run: cargo nextest run\n    gate: true\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), marked).expect("the repository's stands");
    let said = manifest.from_kit().and_then(|from| from.left_out.clone()).expect("named");
    assert!(said.contains("checks.test.gate") && said.contains("This Check is the repository's"), "{said}");
    assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make test"));
}

#[test]
fn gate_on_a_check_the_repository_declares_is_refused() {
    let refused = Manifest::parse(file(), "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  build:\n    run: make\n    gate: true\n");
    let said = refused.expect_err("refused").to_string();
    assert!(said.contains("checks.build.gate") && said.contains("This Check is the repository's"), "{said}");
}

#[test]
fn a_gate_that_is_not_true_or_false_leaves_kits_copy_out() {
    let mine = "checks:\n  e2e:\n    run: make e2e\n    gate: sometimes\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("the repository's stands");
    let said = manifest.from_kit().and_then(|from| from.left_out.clone()).expect("named");
    assert!(said.contains("`checks.e2e.gate` is not true or false"), "{said}");
}

#[test]
fn a_whole_copy_gates_each_check_unless_it_says_not() {
    let mine = "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  test:\n    run: make test\n  lint:\n    run: make lint\n  \
                e2e:\n    run: make e2e\n    gate: false\n";
    let manifest = Manifest::from_kit_alone(file(), kit(), mine).expect("Kit's parses");
    assert_eq!(gated(&manifest), ["test", "lint"]);
    assert!(manifest.check("e2e").is_some());
}

#[test]
fn a_kit_copy_that_will_not_merge_is_left_out_and_the_repositorys_stands() {
    for mine in ["checks: [unclosed", "checks:\n  test:\n    run: make test\n    flavour: x\n"] {
        let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("the repository's stands");
        assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make test"));
        let said = manifest.from_kit().and_then(|from| from.left_out.clone()).expect("named");
        assert!(said.starts_with("/home/.armada/manifests/acme--widgets.yml was left out"), "{said}");
    }
}

#[test]
fn a_copy_naming_only_its_tracker_or_ci_gate_is_kept_and_any_other_key_is_refused() {
    for mine in [
        "tracker:\n  provider: jira\n  site: acme\n  project: ABC\n",
        "ci_gate: build\n",
        "checks:\n  test:\n    run: make quick\ntracker:\n  provider: jira\n  site: acme\n  project: ABC\n",
    ] {
        assert_eq!(kit_copy_fault(mine), None, "{mine}");
    }
    let said = kit_copy_fault("tracker:\n  provider: jira\n  site: acme\nflavour: x\n").expect("refused");
    assert!(said.contains("`checks`, `commands`, `ports`, `runs`, `tracker` and `ci_gate`"), "says what a copy may hold: {said}");
    assert!(!said.contains(['\u{2014}']), "no dash in {said}");
}

#[test]
fn kits_tracker_and_ci_gate_replace_the_repositorys_whole() {
    let theirs = "version: 1\nid: 01FIXTUREMANIFEST\nci_gate: ci\ntracker:\n  provider: jira\n  site: old\n  jql: assignee = me\n";
    let mine = "ci_gate: build\ntracker:\n  provider: jira\n  site: acme\n  project: ABC\n";
    for repository in [REPOSITORY, theirs] {
        let manifest = Manifest::with_kit(file(), repository, kit(), mine).expect("they merge");
        let tracker = manifest.tracker().expect("Kit's tracker");
        assert_eq!(tracker.provider(), Some("jira"));
        assert_eq!(
            tracker.keys().iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>(),
            ["project=ABC", "site=acme"],
            "the whole block is Kit's, with nothing of the repository's left in it"
        );
        assert_eq!(manifest.ci_gate(), "build");
        assert_eq!(manifest.id().as_str(), "01FIXTUREMANIFEST", "the rest of the repository's stands");
        assert!(manifest.from_kit().is_some_and(|from| from.left_out.is_none()));
    }
}

#[test]
fn a_repository_that_will_not_load_is_refused_whatever_kit_holds() {
    let refused = Manifest::with_kit(file(), "version: 1\n", kit(), "checks: {}\n").expect_err("no id");
    assert_eq!(refused.path(), file());
}

/// What a Job's gate resolves to is the same answer `gated` gives: a Kit-only Check is left out of
/// `every_manifest_check` and a marked one is in it.
#[test]
fn every_manifest_check_takes_a_kit_check_only_where_it_is_marked() {
    use crate::resolve::ResolvedWorkflow;
    use crate::workflow::WorkflowDef;

    let def = WorkflowDef::parse(
        &crate::tests::named("workflows/feature.yml"),
        "version: 1\nworkflow_id: feature\nname: feature\nsteps:\n  - id: implement\n    label: Implement\n    \
         evidence: {submitted: {type: diff}}\n    delivers: false\n    advance_gate: auto\n    \
         mechanical_checks:\n      - { type: every_manifest_check }\n",
        &crate::tests::roster(),
    )
    .expect("a workflow");
    let labels = |mine: &str| {
        let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
        let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("resolves");
        resolved.steps()[0].checks().iter().map(|check| check.label().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(labels("checks:\n  e2e:\n    run: make e2e\n"), ["test", "lint"]);
    assert_eq!(labels("checks:\n  e2e:\n    run: make e2e\n    gate: true\n"), ["test", "lint", "e2e"]);
}

const SERVING: &str = "version: 1\nid: 01FIXTUREMANIFEST\nports:\n  web: {}\ncommands:\n  web:\n    \
                       serve: npx vite --port ${port.web}\n    dir: frontend\n";

#[test]
fn kits_copy_as_the_whole_manifest_takes_servers_and_runs() {
    let mine = format!("{SERVING}runs:\n  app:\n    label: App\n    commands: [web]\n");
    let manifest = Manifest::from_kit_alone(file(), kit(), &mine).expect("Kit's parses");
    assert_eq!(manifest.server("web").and_then(|server| server.dir()), Some("frontend"));
    assert_eq!(manifest.app_run("app").map(|run| run.servers().to_vec()), Some(vec![String::from("web")]));
    assert_eq!(kit_copy_fault(&mine), None);
}

#[test]
fn kits_copy_naming_only_a_server_and_a_run_is_kept_and_merged_over_the_repositorys() {
    let mine = "commands:\n  web:\n    serve: npx vite --port ${port.web}\nruns:\n  app:\n    commands: [web]\n";
    assert_eq!(kit_copy_fault(mine), None, "a Command with `serve` and no `run` is one");
    let repository = format!("{REPOSITORY}ports:\n  web: {{}}\n");
    let manifest = Manifest::with_kit(file(), &repository, kit(), mine).expect("they merge");
    assert!(manifest.server("web").is_some(), "Kit's server is added");
    assert_eq!(manifest.app_run("app").map(|run| run.servers().to_vec()), Some(vec![String::from("web")]));
    let from = manifest.from_kit().expect("read from Kit");
    assert!(from.names.contains(&added(KitKind::Run, "app", None)), "{:?}", from.names);
}

#[test]
fn kits_run_naming_a_server_nobody_declares_leaves_the_copy_out() {
    let mine = "runs:\n  app:\n    commands: [web]\n";
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("the repository's stands");
    let said = manifest.from_kit().and_then(|from| from.left_out.clone()).expect("named");
    assert!(said.contains("was left out"), "{said}");
    assert!(manifest.app_run("app").is_none());
}

#[test]
fn a_copy_entry_with_nothing_to_run_or_serve_is_refused() {
    let said = kit_copy_fault("commands:\n  web:\n    ready: curl localhost\n").expect("refused");
    assert!(said.contains("`run` or `serve`"), "{said}");
    let said = kit_copy_fault("runs:\n  app:\n    label: App\n").expect("refused");
    assert!(said.contains("`commands` list"), "{said}");
}

#[test]
fn kits_copy_naming_only_its_ports_servers_and_run_brings_all_three_to_a_repository_that_has_none_of_them() {
    let mine = "ports:\n  web: {}\ncommands:\n  web:\n    serve: npx vite --port ${port.web}\nruns:\n  app:\n    commands: [web]\n";
    assert_eq!(kit_copy_fault(mine), None);
    let manifest = Manifest::with_kit(file(), REPOSITORY, kit(), mine).expect("they merge");
    assert!(manifest.from_kit().is_some_and(|from| from.left_out.is_none()), "{:?}", manifest.from_kit());
    assert!(manifest.port("web").is_some() && manifest.server("web").is_some());
    assert_eq!(manifest.app_run("app").map(|run| run.servers().to_vec()), Some(vec![String::from("web")]));
    let from = manifest.from_kit().expect("read from Kit");
    assert!(from.names.contains(&added(KitKind::Port, "web", None)), "{:?}", from.names);
}

#[test]
fn a_port_in_kits_copy_that_is_not_a_mapping_is_refused() {
    let said = kit_copy_fault("ports:\n  web: 80\n").expect("refused");
    assert!(said.contains("`ports` entry `web` is not a mapping"), "{said}");
}

#[test]
fn kits_setup_requires_follows_the_repositorys_each_name_once() {
    let repository = "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  bootstrap:\n    run: make bootstrap\n  \
                      install_web:\n    run: npm ci --prefix web\nsetup:\n  requires: [bootstrap, install_web]\n";
    let mine = "commands:\n  venv:\n    run: python3 -m venv .venv\nsetup:\n  requires: [install_web, venv]\n";
    let manifest = Manifest::with_kit(file(), repository, kit(), mine).expect("they merge");
    assert!(manifest.from_kit().is_some_and(|from| from.left_out.is_none()));
    let names: Vec<&str> = manifest.prepared_by().iter().map(|step| step.name()).collect();
    assert_eq!(names, ["bootstrap", "install_web", "venv"], "the repository's first, then Kit's, each once");

    let bare = "version: 1\nid: 01FIXTUREMANIFEST\n";
    let manifest = Manifest::with_kit(file(), bare, kit(), mine).expect("they merge");
    assert!(manifest.prepared_by().is_empty(), "a name declared nowhere leaves Kit's copy out whole");
    assert!(manifest.from_kit().is_some_and(|from| from.left_out.is_some()));
}

#[test]
fn a_copy_that_is_not_whole_holds_setup_requires_and_nothing_else_under_setup() {
    assert_eq!(kit_copy_fault("commands:\n  venv:\n    run: python3 -m venv .venv\nsetup:\n  requires: [venv]\n"), None);
    let said = kit_copy_fault("commands:\n  venv:\n    run: python3 -m venv .venv\nsetup:\n  worktrees: 4\n").expect("refused");
    assert!(said.contains("holds only `requires`"), "{said}");
}
