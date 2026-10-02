//! `armada check <name> --changed`, over a Manifest written for the test.

use std::time::Duration;

use crate::declared::{execute, Asked, Ran, Reached, Registry};
use crate::tests::TempDir;

const BUDGET: Duration = Duration::from_secs(30);

/// A Manifest whose `test` narrows by package under `crates`, as this
/// repository's does, with a whole run that fails so a run that should have
/// been narrowed cannot pass by accident.
fn a_narrowing_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        "version: 1\n\
         id: a-test-project\n\
         checks:\n  \
           test:\n    run: /usr/bin/false\n    \
             narrow:\n      run: /bin/echo narrowed\n      each: \"-p {}\"\n      \
               under: crates\n      except:\n        - acceptance\n    \
             when:\n      - \"crates/**\"\n      - \"Cargo.lock\"\n",
    );
    dir
}

async fn changed(dir: &TempDir, paths: &[&str]) -> Ran {
    let paths: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
    execute(
        dir.path(),
        Registry::Checks,
        "test",
        Asked::Changed(&paths),
        BUDGET,
        None,
    )
    .await
    .expect("`test` is declared")
}

/// **The merge line's narrowing**: every changed path the Check covers names a
/// package, so the narrowed command runs, and a path it does not cover is no
/// reason to run it whole.
#[tokio::test]
async fn a_change_the_narrowing_can_name_runs_narrowed() {
    let dir = a_narrowing_repository();
    let ran = changed(
        &dir,
        &[
            "crates/fleet/src/lib.rs",
            "crates/api",
            "crates/acceptance/tests/a.rs",
            "docs/a.md",
        ],
    )
    .await;
    assert_eq!(ran.command, "/bin/echo narrowed -p api -p fleet");
    assert_eq!(
        ran.narrowed,
        Some(Reached::To("-p api -p fleet".to_string()))
    );
    assert_eq!(ran.status(), 0, "{:?}", ran.attempt);
}

#[tokio::test]
async fn a_covered_path_it_cannot_name_runs_it_whole() {
    let dir = a_narrowing_repository();
    let ran = changed(&dir, &["crates/fleet/src/lib.rs", "Cargo.lock"]).await;
    assert_eq!(ran.command, "/usr/bin/false");
    assert_eq!(ran.narrowed, Some(Reached::Whole));
    assert_eq!(ran.status(), 1);
}

/// What the whole run excludes, alone, is nothing to run — and is said, so a
/// pass is never read as a run.
#[tokio::test]
async fn a_change_reaching_only_the_exclusion_runs_nothing_and_passes() {
    let dir = a_narrowing_repository();
    let ran = changed(&dir, &["crates/acceptance/tests/a.rs"]).await;
    assert_eq!(ran.narrowed, Some(Reached::Nothing));
    assert_eq!(ran.status(), 0);
}
