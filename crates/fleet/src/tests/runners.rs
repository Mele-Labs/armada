//! A Check narrowed by the runner that drives it, where it declares no
//! narrowing of its own. #1456.
//!
//! **The subject is precedence and the fallbacks.** What a runner's shipped
//! description answers, what it must not override, and the four ways it can
//! have nothing to say — every one of which has to come out as the Check
//! running whole rather than as a command naming nothing.

use core_model::{Narrowing, ResolvedCheck, Runner, RunsAt};

use crate::checking::{by_its_runner, narrowed, Planned};

fn paths(of: &[&str]) -> Vec<String> {
    of.iter().map(|path| path.to_string()).collect()
}

/// One vitest-driven Check, narrowing declared or not.
fn check(runner: Option<Runner>, narrow: Option<Narrowing>) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        manifest_dir: String::new(),
        name: "screens_test".to_string(),
        run: "pnpm --dir packages/screens exec vitest run".to_string(),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow,
        one_test: None,
        runs_at: RunsAt::Everywhere,
        places: std::num::NonZeroU32::MIN,
        width: None,
        runner,
    }
}

fn vitest() -> Runner {
    Runner::declared("vitest".to_string(), Some("packages/screens".to_string()))
}

#[test]
fn a_check_naming_vitest_narrows_to_the_tests_reaching_what_changed() {
    let narrowed_to = by_its_runner(
        &check(Some(vitest()), None),
        &paths(&["packages/screens/src/overview.ts"]),
    )
    .expect("vitest ships a run_changed");
    assert_eq!(
        narrowed_to,
        "pnpm --dir packages/screens exec vitest related src/overview.ts \
         --run --passWithNoTests=false"
    );
}

/// **The declaration in the file wins.** A repository that wrote how it wants
/// this Check narrowed has said so, and a shipped description of the runner
/// answers only where it has not — the precedence the concept page states for
/// a repository's own description against a shipped one, one tier down.
#[test]
fn a_checks_own_narrowing_is_answered_before_its_runners() {
    let own = Narrowing::declared(
        "pnpm --dir packages/screens exec vitest run".to_string(),
        "{}".to_string(),
        None,
        None,
        Vec::new(),
    );
    let planned = narrowed(
        &check(Some(vitest()), Some(own)),
        "screens_test",
        "pnpm --dir packages/screens exec vitest run",
        &paths(&["src/overview.ts"]),
        true,
    );
    let Planned::Command { narrowed_to, .. } = planned else {
        panic!("a Check with a narrowing of its own runs narrowed");
    };
    let narrowed_to = narrowed_to.expect("narrowed by its own declaration");
    assert!(
        !narrowed_to.contains("related"),
        "the runner's shape overrode the file's own: {narrowed_to}"
    );
}

/// Each of these is the Check running whole, which is never less than narrow.
#[test]
fn a_runner_with_nothing_to_say_leaves_the_check_whole() {
    assert_eq!(
        by_its_runner(&check(None, None), &paths(&["src/a.ts"])),
        None,
        "a Check naming no runner"
    );
    assert_eq!(
        by_its_runner(
            &check(Some(Runner::declared("nose".to_string(), None)), None),
            &paths(&["src/a.ts"])
        ),
        None,
        "a runner nobody ships a description of"
    );
    assert_eq!(
        by_its_runner(&check(Some(vitest()), None), &[]),
        None,
        "no changed paths to narrow to"
    );
    assert_eq!(
        by_its_runner(
            &check(Some(Runner::declared("vitest".to_string(), None)), None),
            &paths(&["src/a.ts"])
        ),
        None,
        "a template naming a package the Check does not declare"
    );
}

/// A narrowed run is never carried to a gate, so what this produces can only
/// ever tell a Drone where it stands. `reuse::KeptAskedRun` is what holds that.
#[test]
fn what_a_runner_narrows_to_is_still_a_narrowed_run() {
    let planned = narrowed(
        &check(Some(vitest()), None),
        "screens_test",
        "pnpm --dir packages/screens exec vitest run",
        &paths(&["packages/screens/src/overview.ts"]),
        true,
    );
    let Planned::Command { narrowed_to, .. } = planned else {
        panic!("it narrows");
    };
    assert!(narrowed_to.is_some(), "recorded as narrowed, not as whole");
}

/// The runner's template runs from its `dir`, so `{files}` is dir-relative.
#[test]
fn files_reach_the_runner_relative_to_its_dir() {
    let runner = Runner::declared("vitest".to_string(), Some("apps/desktop".to_string()));
    let made = by_its_runner(
        &check(Some(runner), None),
        &paths(&["apps/desktop/src/a.test.tsx"]),
    )
    .expect("it narrows");
    assert_eq!(
        made,
        "pnpm --dir apps/desktop exec vitest related src/a.test.tsx \
         --run --passWithNoTests=false"
    );
}

/// A path outside the dir cannot be named to the runner, and dropping it would
/// narrow to a subset nobody was told about, so the Check runs whole.
#[test]
fn a_covered_path_outside_the_dir_leaves_the_check_whole() {
    assert_eq!(
        by_its_runner(
            &check(Some(vitest()), None),
            &paths(&["packages/screens/src/a.ts", "crates/x/src/lib.rs"])
        ),
        None
    );
    assert_eq!(
        by_its_runner(
            &check(Some(vitest()), None),
            &paths(&["packages/other/a.ts"])
        ),
        None,
        "nothing under the dir"
    );
}

/// A dir sharing a prefix is not under it.
#[test]
fn a_sibling_sharing_a_prefix_is_not_under_the_dir() {
    assert_eq!(
        by_its_runner(
            &check(Some(vitest()), None),
            &paths(&["packages/screens-extra/a.ts"])
        ),
        None
    );
}

/// A root and one workspace, `packages/screens`, each with a vitest Check that
/// declares no runner `dir`. The root's `run` fails, so a whole run shows.
mod in_a_workspace {
    use std::path::Path;

    use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};

    use super::paths;
    use crate::checking::{narrowed, Planned};
    use crate::gated::{local, Gated};

    const STEP: &str =
        "version: 1\nworkflow_id: fixture\nname: fixture\nsteps:\n  - id: implement\n    \
        label: Implement\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    \
        advance_gate: auto\n    mechanical_checks: [{ type: every_manifest_check }]\n";

    fn manifests() -> (Manifest, Manifest) {
        let body =
            "checks:\n  test:\n    run: pnpm exec vitest run\n    runner:\n      name: vitest\n";
        let root = Manifest::parse(
            Path::new("armada.yml"),
            &format!("version: 1\nid: 01WSROOT\n{body}"),
        )
        .expect("the root parses");
        let screens = Manifest::parse_workspace(
            Path::new("packages/screens/armada.yml"),
            "packages/screens",
            &format!("version: 1\nid: 01WSSCRN\n{body}"),
            &root,
        )
        .expect("the workspace parses");
        (root, screens)
    }

    /// What each Check of the step plans for a Drone's change, by key.
    fn planned(changed: &[&str], narrow: bool) -> Vec<(String, Option<String>)> {
        let (root, screens) = manifests();
        let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
            .expect("the workflow parses");
        let all = [&root, &screens];
        let workflow = ResolvedWorkflow::resolve_gated(&def, &root, &all).expect("resolves");
        let changed = paths(changed);
        let gated = Gated::of(&root, std::slice::from_ref(&screens), &changed);
        workflow.steps()[0]
            .checks()
            .iter()
            .map(|check| {
                let local = local(Some(&gated), check, &changed);
                let Planned::Command { narrowed_to, .. } = narrowed(
                    check,
                    &check.key(),
                    check.run().expect("a command"),
                    &local,
                    narrow,
                ) else {
                    panic!("{} plans a command", check.key());
                };
                (check.key().into_owned(), narrowed_to)
            })
            .collect()
    }

    /// `{dir}` is `.`, since the Check runs in the workspace, and the files are
    /// relative to it.
    #[test]
    fn an_omitted_dir_is_the_manifests_own_directory() {
        let planned = planned(&["packages/screens/src/a.ts"], true);
        assert_eq!(
            planned,
            [
                ("test".to_string(), None),
                (
                    "packages/screens:test".to_string(),
                    Some(
                        "pnpm --dir . exec vitest related src/a.ts --run --passWithNoTests=false"
                            .to_string()
                    )
                ),
            ]
        );
    }

    #[test]
    fn without_a_narrowed_ask_the_workspace_check_runs_whole() {
        let planned = planned(&["packages/screens/src/a.ts"], false);
        assert!(planned.iter().all(|(_, to)| to.is_none()), "{planned:?}");
    }

    /// The root's Check never sees a workspace's path, and with a root path of
    /// its own it still has no `dir` to put in the template.
    #[test]
    fn a_root_check_with_no_dir_runs_whole_as_before() {
        let planned = planned(&["README.md"], true);
        assert_eq!(planned[0], ("test".to_string(), None));
    }
}
