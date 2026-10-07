//! `armada check` and `armada run`, over a Manifest written for the test.
//!
//! **No Fleet, no store and no worktree.** Both verbs are a Manifest read and a
//! process, which is the whole claim: a second reader of `armada.yml` that
//! needs none of the daemon.

use std::time::Duration;

use checks_runner::{CheckSlots, Priority};

use crate::declared::{execute, Asked, Registry};
use crate::tests::{repository, TempDir};

const BUDGET: Duration = Duration::from_secs(30);

/// A Manifest with one Check and one Command, both of which really run.
fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           build:\n    run: /bin/sh -c true\n  \
           test:\n    run: /usr/bin/false\n\
         commands:\n  \
           fmt:\n    run: /bin/sh -c true\n  \
           wipe:\n    run: /bin/sh -c true\n    destructive: true\n",
    );
    dir
}

#[tokio::test]
async fn a_check_that_passes_comes_back_with_its_command_and_a_zero_status() {
    let dir = a_repository();
    let ran = execute(
        dir.path(),
        Registry::Checks,
        "build",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("`build` is declared");

    assert_eq!(ran.command, "/bin/sh -c true");
    assert_eq!(ran.status(), 0);
}

/// The Check's own exit code comes out, because a person running a Check by
/// hand wants the answer the Check gave.
#[tokio::test]
async fn a_check_that_fails_carries_its_own_exit_code_out() {
    let dir = a_repository();
    let ran = execute(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("`test` is declared");

    assert_eq!(ran.status(), 1);
}

/// **A refusal names what is declared.** Otherwise the reader has to open the
/// file, which is the one thing the command could have saved them.
#[tokio::test]
async fn a_name_that_is_not_declared_is_refused_by_naming_what_is() {
    let dir = a_repository();
    let refused = execute(
        dir.path(),
        Registry::Checks,
        "buidl",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("`buidl` is not a Check")
    .to_string();

    assert!(refused.contains("`buidl` is not a Check"), "{refused}");
    assert!(
        refused.contains("`build`") && refused.contains("`test`"),
        "{refused}"
    );
}

/// **The two registries stay two.** A Check named at `run` is refused with the
/// verb that would have worked, rather than obliged.
#[tokio::test]
async fn a_check_named_at_run_is_refused_with_the_verb_that_would_have_worked() {
    let dir = a_repository();
    let refused = execute(
        dir.path(),
        Registry::Commands,
        "build",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("`build` is a Check, not a Command")
    .to_string();

    assert!(refused.contains("as a Check, not a Command"), "{refused}");
    assert!(refused.contains("armada check build"), "{refused}");
}

#[tokio::test]
async fn a_command_named_at_check_is_refused_the_same_way_round() {
    let dir = a_repository();
    let refused = execute(
        dir.path(),
        Registry::Checks,
        "fmt",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("`fmt` is a Command, not a Check")
    .to_string();

    assert!(refused.contains("armada run fmt"), "{refused}");
}

/// Destructive is said, not enforced. The flag pauses a Drone; the person
/// typing this is already the one triggering it.
#[tokio::test]
async fn a_destructive_command_runs_and_says_it_is_destructive() {
    let dir = a_repository();
    let ran = execute(
        dir.path(),
        Registry::Commands,
        "wipe",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("`wipe` is declared");

    assert!(ran.destructive);
    assert_eq!(ran.status(), 0);
}

/// A directory with no Manifest is not a repository Armada has been set up for,
/// and the refusal says which file it wanted.
#[tokio::test]
async fn a_directory_with_no_manifest_is_refused_by_naming_the_file() {
    let dir = TempDir::new();
    let refused = execute(
        dir.path(),
        Registry::Checks,
        "build",
        Asked::Whole,
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("there is no Manifest here")
    .to_string();

    assert!(refused.contains("armada.yml"), "{refused}");
}

/// **This repository's own Manifest, resolved by the same verb.** Renaming a
/// Check in `armada.yml` and not here is what this catches.
#[tokio::test]
async fn this_repositorys_own_checks_and_commands_resolve() {
    for name in ["build", "test"] {
        let refused = execute(
            &repository(),
            Registry::Commands,
            name,
            Asked::Whole,
            BUDGET,
            None,
            Priority::Normal,
        )
        .await
        .expect_err("they are Checks, not Commands")
        .to_string();
        assert!(refused.contains("as a Check"), "{name}: {refused}");
    }
    for name in ["fmt", "gate"] {
        let refused = execute(
            &repository(),
            Registry::Checks,
            name,
            Asked::Whole,
            BUDGET,
            None,
            Priority::Normal,
        )
        .await
        .expect_err("they are Commands, not Checks")
        .to_string();
        assert!(refused.contains("as a Command"), "{name}: {refused}");
    }
}

/// **A bare test function's name runs it**, as its full path does: `test`'s
/// `one_test` matches the name as whole segments of a path, not the whole path.
#[test]
fn this_repositorys_test_runs_one_test_by_its_bare_name() {
    let manifest = config::Manifest::load(&repository().join("armada.yml")).expect("it reads");
    let template = manifest
        .check("test")
        .and_then(config::Check::one_test)
        .expect("`test` declares a one_test");
    let command = checks_runner::one_test(template, "a_span_holding_one_taken_port_is_not_free")
        .expect("a name runs");
    assert!(
        command.ends_with("-E test(/(^|::)a_span_holding_one_taken_port_is_not_free(::|$)/)"),
        "{command}"
    );
}

/// **A module's name runs every test under it**, in `test` and `acceptance`
/// alike: the name ends at a path segment either way, so `clean_slots` or
/// `tests::servers` reaches the tests beneath it.
#[test]
fn this_repositorys_test_runs_a_module_by_its_name() {
    let manifest = config::Manifest::load(&repository().join("armada.yml")).expect("it reads");
    for check in ["test", "acceptance"] {
        let template = manifest
            .check(check)
            .and_then(config::Check::one_test)
            .expect("it declares a one_test");
        let command = checks_runner::one_test(template, "tests::servers").expect("a name runs");
        assert!(
            command.ends_with("-E test(/(^|::)tests::servers(::|$)/)"),
            "{check}: {command}"
        );
    }
}

/// **`format` only reads.** A `requires: [fmt]` once made it rewrite the tree
/// and then pass on what it had just written; `armada.yml` says why.
#[test]
fn this_repositorys_format_check_formats_nothing_first() {
    let manifest = config::Manifest::load(&repository().join("armada.yml")).expect("it reads");
    let format = manifest.check("format").expect("`format` is a Check");
    assert!(format.requires().is_empty());
}

/// **A Check takes the machine's slots**, waits while every one is held, and
/// tells what it starts that it holds one, so a Check inside it does not wait
/// on its own parent.
#[tokio::test]
async fn a_check_waits_for_a_machine_slot_and_hands_it_down() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\nid: a-test-project\nchecks:\n  env:\n    run: /usr/bin/env\n",
    );
    let slots = CheckSlots::at(dir.path().join("slots"), 1);
    let elsewhere = slots.try_take(1).expect("writable").expect("free");

    let root = dir.path().to_path_buf();
    let waiting = slots.clone();
    let running = tokio::spawn(async move {
        execute(
            &root,
            Registry::Checks,
            "env",
            Asked::Whole,
            BUDGET,
            Some(&waiting),
            Priority::Normal,
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(!running.is_finished(), "ran while the one slot was held");

    drop(elsewhere);
    let ran = tokio::time::timeout(Duration::from_secs(10), running)
        .await
        .expect("ran once the slot was free")
        .expect("the task ran")
        .expect("`env` is declared");
    assert!(
        ran.attempt
            .output
            .stdout
            .contains(&format!("{}=", checks_runner::HELD_ENV)),
        "{}",
        ran.attempt.output.stdout
    );
}

/// **One test runs through the Check's `one_test`**, and a Check with none
/// refuses rather than running the whole suite in its place.
#[tokio::test]
async fn one_test_runs_through_the_checks_one_test() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/false\n    one_test:\n      run: /bin/echo only {}\n  \
           build:\n    run: /usr/bin/true\n",
    );
    let ran = execute(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::OneTest("a b"),
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect("`test` declares a one_test");
    assert_eq!(ran.command, "/bin/echo only \"a b\"");
    assert_eq!(ran.attempt.output.stdout, "only a b\n");
    assert_eq!(ran.status(), 0);

    let refused = execute(
        dir.path(),
        Registry::Checks,
        "build",
        Asked::OneTest("a"),
        BUDGET,
        None,
        Priority::Normal,
    )
    .await
    .expect_err("`build` declares no one_test")
    .to_string();
    assert!(refused.contains("no `one_test`"), "{refused}");
}

/// **Absent `when` means always**, and a declared one is read against the
/// paths — the same answer a Job's gate gives, in the order the file writes.
#[test]
fn covering_names_the_checks_a_change_hits_in_the_order_written() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/true\n  \
           ui:\n    run: /usr/bin/true\n    when: [\"packages/**\"]\n  \
           rust:\n    run: /usr/bin/true\n    when: [\"crates/**\", \"Cargo.lock\"]\n",
    );
    let hits = |paths: &[&str]| {
        let changed: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        crate::declared::covering(dir.path(), &changed).expect("the Manifest reads")
    };

    assert_eq!(hits(&["docs/INDEX.md"]), vec!["test"]);
    assert_eq!(
        hits(&["packages/a/b.ts", "Cargo.lock"]),
        vec!["test", "ui", "rust"]
    );
    assert_eq!(hits(&["crates/x/src/lib.rs"]), vec!["test", "rust"]);
}

#[test]
fn covering_refuses_a_directory_with_no_manifest() {
    let dir = TempDir::new();
    let refused = crate::declared::covering(dir.path(), &["a".to_string()])
        .expect_err("there is no Manifest here")
        .to_string();
    assert!(refused.contains("armada.yml"), "{refused}");
}

/// **This repository's own Manifests.** A Bridge-only change does not pay for
/// `acceptance`, and a Rust one does. The Bridge's Checks are keyed by the
/// workspace that declares them, and a change to one package gates that
/// package and what reads it.
#[test]
fn this_repositorys_checks_are_chosen_by_their_when() {
    let bridge = hits(&["apps/desktop/src/x.ts"]);
    assert!(
        bridge.contains(&"apps/desktop:typecheck".to_string()),
        "{bridge:?}"
    );
    assert!(!bridge.contains(&"acceptance".to_string()), "{bridge:?}");
    assert!(
        !bridge.iter().any(|key| key.starts_with("packages/")),
        "nothing under `packages/` reads `apps/desktop`: {bridge:?}"
    );

    let components = hits(&["packages/components/src/Badge.tsx"]);
    for key in [
        "packages/components:components_test",
        "packages/screens:screens_test",
        "apps/desktop:app_smoke",
        "apps/desktop/unit:desktop_test",
    ] {
        assert!(
            components.contains(&key.to_string()),
            "{key}: {components:?}"
        );
    }
    let screens = hits(&["packages/screens/src/x.ts"]);
    assert!(screens.contains(&"packages/screens:screens_test".to_string()));
    assert!(
        !screens.contains(&"packages/components:components_test".to_string()),
        "components does not read screens: {screens:?}"
    );

    let rust = hits(&["crates/fleet/src/lib.rs"]);
    assert!(rust.contains(&"acceptance".to_string()), "{rust:?}");
    assert!(!rust.contains(&"typecheck".to_string()), "{rust:?}");
    assert!(
        !rust.iter().any(|key| key.contains(':')),
        "a crate change gates no workspace: {rust:?}"
    );

    // The gate leaves "passes" to `acceptance`, so a file the suite embeds
    // has to reach it.
    let workflow = hits(&[".armada/workflows/bug.json"]);
    assert!(workflow.contains(&"acceptance".to_string()), "{workflow:?}");
}

/// **A change to the written record runs nothing**, which is what narrowing the
/// three unscoped Checks bought. `cargo xtask verify-docs` is what reads these,
/// and it is a Command rather than a Check.
#[test]
fn a_change_to_the_documents_alone_hits_no_check() {
    assert_eq!(hits(&["docs/INDEX.md", "README.md"]), Vec::<String>::new());
}

/// **Except the documents code reads.** `fleet` takes `agent-prompt.md` by
/// `include_str!`, `adapters`' tests read `docs/spikes/`, and `xtask`'s studio
/// rule reads the lexicon out of `design-system.md`.
#[test]
fn a_document_the_code_reads_runs_test() {
    for path in [
        "docs/contracts/agent-prompt.md",
        "docs/contracts/design-system.md",
        "docs/spikes/017-a-transcript.ndjson",
    ] {
        assert!(hits(&[path]).contains(&"test".to_string()), "{path}");
    }
}

/// **The lockfile is what `--locked` resolves**, so a bump nothing else in the
/// tree shows still builds and tests the workspace. It is not what `cargo fmt`
/// reads, and `format` says so by leaving it out.
#[test]
fn a_lockfile_bump_alone_still_builds_and_tests() {
    let hit = hits(&["Cargo.lock"]);
    assert!(hit.contains(&"build".to_string()), "{hit:?}");
    assert!(hit.contains(&"test".to_string()), "{hit:?}");
    assert!(!hit.contains(&"format".to_string()), "{hit:?}");
}

/// Each pattern the three narrowed Checks name is there because the command
/// reads it, and this is that claim as a test.
#[test]
fn what_the_narrowed_checks_read_still_selects_them() {
    for path in [
        "crates/fleet/src/lib.rs",
        "xtask/src/rules.rs",
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        "protocol-version.toml",
        ".armada/workflows/bug.json",
        "armada.yml",
    ] {
        let hit = hits(&[path]);
        assert!(hit.contains(&"build".to_string()), "{path}: {hit:?}");
        assert!(hit.contains(&"test".to_string()), "{path}: {hit:?}");
    }
    // `xtask`'s own tests read the Bridge tree, and nothing compiles it in.
    // The root owns only what no workspace claims, so a path under a
    // workspace reaches that run through `apps/desktop:xtask_test`, and a
    // package nobody owns still reaches the root's `test`.
    for path in ["apps/desktop/src/x.ts", "packages/components/src/Badge.tsx"] {
        let hit = hits(&[path]);
        assert!(
            hit.contains(&"apps/desktop:xtask_test".to_string()),
            "{path}: {hit:?}"
        );
        assert!(!hit.contains(&"build".to_string()), "{path}: {hit:?}");
    }
    let unowned = hits(&["packages/tokens/x.css"]);
    assert!(unowned.contains(&"test".to_string()), "{unowned:?}");
    assert!(!unowned.contains(&"build".to_string()), "{unowned:?}");
    for path in ["crates/ipc/build.rs", "xtask/src/main.rs", "Cargo.toml"] {
        assert!(hits(&[path]).contains(&"format".to_string()), "{path}");
    }
}

/// **`armada check` lowers a Check and never a Command.** `main` hands in the
/// priority `ARMADA_CHECK_PRIORITY` names; this is what each registry does
/// with a lowered one, read back from `ps` as the process's own priority.
#[cfg(target_os = "macos")]
#[tokio::test]
async fn a_lowered_check_runs_clamped_and_a_command_does_not() {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           pri:\n    run: /bin/sh -c 'ps -o pri= -p $$'\n\
         commands:\n  \
           priority:\n    run: /bin/sh -c 'ps -o pri= -p $$'\n",
    );
    let mut printed = Vec::new();
    for (registry, name) in [(Registry::Checks, "pri"), (Registry::Commands, "priority")] {
        let ran = execute(
            dir.path(),
            registry,
            name,
            Asked::Whole,
            BUDGET,
            None,
            Priority::Low,
        )
        .await
        .expect("declared");
        let said = ran.attempt.output.stdout.trim().to_string();
        printed.push(said.parse::<u32>().expect("ps printed a number"));
    }
    let own = std::process::Command::new("ps")
        .args(["-o", "pri=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps runs");
    let own: u32 = String::from_utf8_lossy(&own.stdout)
        .trim()
        .parse()
        .expect("a number");
    assert!(printed[0] <= 20, "a lowered Check ran at {}", printed[0]);
    assert_eq!(printed[1], own, "a Command ran at a priority of its own");
}

/// This repository's Manifest, asked what a change hits.
fn hits(paths: &[&str]) -> Vec<String> {
    let changed: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    crate::declared::covering(&repository(), &changed).expect("this repository's Manifest reads")
}
