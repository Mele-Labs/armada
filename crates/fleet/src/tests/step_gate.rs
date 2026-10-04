//! The step gate runs a step's Checks one at a time, and narrows `test` and
//! `build` the merge line's way. The owner's decision of 4 Oct 2026, after Job
//! 3's implement gate ran its own Checks over each other and all 4211 Rust
//! tests for a change to `apps/` alone. `docs/concepts/manifest.md`.
//!
//! Every Check's whole command and narrowed one print which they are, so a
//! row's output says which of the two the gate ran. The narrowed workspace is
//! real, because which members depend on which is `cargo tree`'s to say.

use std::sync::Arc;

use adapter_traits::{Footprint, Worktree};
use config::ResolvedWorkflow;
use core_model::{Attempt, StepId};
use testkit::{FakeWorkProduct, Gate, Narrows, Sketch};
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckOutput, Ruling};
use crate::places::{ChecksAtOnce, Room};
use crate::policy::Policies;
use crate::tests::gate::{budget, diff_evidence, judging, Stopped};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::underway::most_places_at_once;
use crate::underway::Announcing;

const RUST: &[&str] = &["crates/**", "xtask/**", "apps/**", "Cargo.lock"];

fn step(gates: &'static [Gate<'static>]) -> Sketch<'static> {
    Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates,
        judged_on: &[],
        scope: None,
        gaming: None,
    }
}

/// `test` narrowing by package under `crates`, with `-p xtask` in every
/// narrowed run, as this repository's Manifest writes it; `format` narrowing
/// verbatim, as `format`'s does.
fn narrowing() -> ResolvedWorkflow {
    testkit::narrowing(
        &[step(&[
            Gate::Check {
                name: "test",
                run: "/bin/echo whole",
                expect_exit_code: 0,
                when: RUST,
            },
            Gate::Check {
                name: "format",
                run: "/bin/echo whole",
                expect_exit_code: 0,
                when: RUST,
            },
        ])],
        &[
            Narrows {
                check: "test",
                run: "/bin/echo narrowed -p xtask",
                each: "-p {}",
                from: &[],
                under: Some("crates"),
                except: &["acceptance"],
            },
            Narrows {
                check: "format",
                run: "/bin/echo narrowed",
                each: "{}",
                from: &[],
                under: None,
                except: &[],
            },
        ],
    )
}

/// `leaf` depends on `mid` depends on `base`, and `apart` on nothing.
fn a_workspace() -> TempDir {
    let dir = TempDir::new();
    let write = |relative: &str, contents: &str| {
        let at = dir.path().join(relative);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("the parent");
        std::fs::write(at, contents).expect("the file");
    };
    write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    );
    for (name, depends) in [
        ("base", ""),
        ("mid", "base = { path = \"../base\" }\n"),
        ("leaf", "mid = { path = \"../mid\" }\n"),
        ("apart", ""),
    ] {
        write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\n{depends}"
            ),
        );
        write(&format!("crates/{name}/src/lib.rs"), "");
    }
    let locked = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(dir.path())
        .output()
        .expect("cargo runs");
    assert!(locked.status.success(), "{locked:?}");
    dir
}

async fn ruled(
    workflow: &ResolvedWorkflow,
    repo: &TempDir,
    changed: &[&str],
    announcing: &Announcing,
) -> Ruling {
    let worktree = Worktree::at(
        repo.path().display().to_string(),
        "armada/01J0000000000000000000JOB0",
    );
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        &[],
        crate::gate::Began::At(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(changed),
        budget(),
        &Room::ignoring_the_machine(ChecksAtOnce::of(4)),
        &judging(),
        &keeping_nowhere(),
        Policies::unstated(),
        announcing,
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await
}

fn printed<'a>(ruling: &'a Ruling, check: &str) -> &'a CheckOutput {
    ruling
        .output()
        .iter()
        .find(|kept| kept.check == check)
        .unwrap_or_else(|| panic!("{check} printed nothing: {ruling:?}"))
}

/// The control each whole case is read against: the same gate over a change
/// it can name runs `test` narrowed, so a whole run below is the reading's
/// answer and not a gate that never narrows.
async fn narrows_a_change_it_can_name(workflow: &ResolvedWorkflow, repo: &TempDir) {
    let ruling = ruled(
        workflow,
        repo,
        &["crates/base/src/lib.rs"],
        &Announcing::nowhere(),
    )
    .await;
    assert!(
        printed(&ruling, "test").narrowed_to.is_some(),
        "the control ran whole: {ruling:?}"
    );
}

/// **Decision one.** Three Checks, room for four: the gate is published as
/// never holding more than one of them, so the second starts after the first
/// has been said to end.
#[tokio::test]
async fn a_steps_gate_checks_never_overlap() {
    let repo = TempDir::new();
    let workflow = testkit::resolved(&[step(&[
        Gate::Check {
            name: "first",
            run: "/bin/sleep 0.2",
            expect_exit_code: 0,
            when: &[],
        },
        Gate::Check {
            name: "second",
            run: "/bin/sleep 0.2",
            expect_exit_code: 0,
            when: &[],
        },
        Gate::Check {
            name: "third",
            run: "/bin/sleep 0.2",
            expect_exit_code: 0,
            when: &[],
        },
    ])]);
    let events = api::Broadcaster::new();
    let mut heard = events.subscribe();
    let announcing = Announcing::on(
        ipc::JobId::carried("01JOB"),
        StepId::new("implement"),
        Attempt::FIRST,
        crate::underway::Underway::default(),
        events.clone(),
        Arc::new(Stopped),
        &repo.path().display().to_string(),
        "01JOB",
    );
    let ruling = ruled(&workflow, &repo, &["src/lib.rs"], &announcing).await;
    drop(announcing);
    drop(events);
    let mut said = Vec::new();
    while let Some(api::Next::Send(delivered)) = heard.next().await {
        if let ipc::Event::JobChecking(one) = delivered.event {
            said.push(one);
        }
    }

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    assert_eq!(
        most_places_at_once(&said),
        1,
        "the gate ran its own Checks over each other"
    );
}

/// **Decision two, and the merge line's own reading of `apps/`.** A covered
/// path outside `under` is one the narrowing cannot name, so `test` runs whole
/// and nothing is recorded as narrowed.
#[tokio::test]
async fn a_change_only_to_apps_runs_test_whole_as_the_merge_line_does() {
    let repo = a_workspace();
    narrows_a_change_it_can_name(&narrowing(), &repo).await;
    let ruling = ruled(
        &narrowing(),
        &repo,
        &["apps/desktop/src/a.ts"],
        &Announcing::nowhere(),
    )
    .await;

    let test = printed(&ruling, "test");
    assert_eq!(test.output.stdout.trim(), "whole");
    assert_eq!(test.narrowed_to, None);
}

/// A change under `crates/base` reaches `mid` and `leaf`, which depend on it,
/// and not `apart`; `-p xtask` rides in from the Manifest's `narrow.run`.
#[tokio::test]
async fn a_change_under_a_crate_runs_test_over_it_and_its_dependents() {
    let repo = a_workspace();
    let ruling = ruled(
        &narrowing(),
        &repo,
        &["crates/base/src/lib.rs"],
        &Announcing::nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    let test = printed(&ruling, "test");
    assert_eq!(
        test.output.stdout.trim(),
        "narrowed -p xtask -p base -p leaf -p mid"
    );
    assert_eq!(
        test.narrowed_to.as_deref(),
        Some("/bin/echo narrowed -p xtask -p base -p leaf -p mid")
    );
}

/// One covered path the narrowing cannot name runs the Check whole, whatever
/// the rest could have narrowed to.
#[tokio::test]
async fn a_covered_path_the_narrowing_cannot_name_runs_test_whole() {
    let repo = a_workspace();
    narrows_a_change_it_can_name(&narrowing(), &repo).await;
    for beside in ["xtask/src/main.rs", "Cargo.lock", "crates/base/data.toml"] {
        let ruling = ruled(
            &narrowing(),
            &repo,
            &["crates/base/src/lib.rs", beside],
            &Announcing::nowhere(),
        )
        .await;

        let test = printed(&ruling, "test");
        assert_eq!(test.output.stdout.trim(), "whole", "{beside}");
        assert_eq!(test.narrowed_to, None, "{beside}");
    }
}

/// A verbatim `narrow` hands the command what changed and nothing it reads
/// beside it, so the gate never narrows to one.
#[tokio::test]
async fn format_stays_whole() {
    let repo = a_workspace();
    let ruling = ruled(
        &narrowing(),
        &repo,
        &["crates/base/src/lib.rs"],
        &Announcing::nowhere(),
    )
    .await;

    assert!(
        printed(&ruling, "test").narrowed_to.is_some(),
        "`test` narrowed over the same change"
    );
    let format = printed(&ruling, "format");
    assert_eq!(format.output.stdout.trim(), "whole");
    assert_eq!(format.narrowed_to, None);
}

/// **A red the gate narrowed is confirmed by the same narrowed command.** Red
/// the first run naming one test, green after; the test passes alone, so the
/// whole Check runs again alone — and that run is the narrowed one the gate
/// ruled on, not the whole suite. `crate::confirming`.
#[tokio::test]
async fn a_narrowed_red_run_again_alone_reruns_the_narrowed_command() {
    let repo = a_workspace();
    std::fs::write(
        repo.path().join("check.sh"),
        "echo \"$*\" >> runs\nif [ -f .ran ]; then exit 0; fi\ntouch .ran\n\
         echo 'Summary [   0.010s] 1 tests run: 0 passed, 1 failed, 0 skipped'\n\
         echo 'FAIL [   0.010s] (1/1) nt base::a_test'\nexit 1\n",
    )
    .expect("the script");
    std::fs::write(repo.path().join("one.sh"), "exit 0\n").expect("the script");
    let workflow = testkit::narrowing_and_testing_one(
        &[step(&[Gate::Check {
            name: "test",
            run: "sh check.sh whole",
            expect_exit_code: 0,
            when: RUST,
        }])],
        &[Narrows {
            check: "test",
            run: "sh check.sh narrowed -p xtask",
            each: "-p {}",
            from: &[],
            under: Some("crates"),
            except: &[],
        }],
        &[testkit::OneTest {
            check: "test",
            run: "sh one.sh {}",
        }],
    );

    let ruling = ruled(
        &workflow,
        &repo,
        &["crates/base/src/lib.rs"],
        &Announcing::nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "the ruling was {ruling:?}");
    let runs = std::fs::read_to_string(repo.path().join("runs")).expect("the runs");
    assert_eq!(
        runs.lines().collect::<Vec<_>>(),
        vec![
            "narrowed -p xtask -p base -p leaf -p mid",
            "narrowed -p xtask -p base -p leaf -p mid"
        ]
    );
    let test = printed(&ruling, "test");
    assert!(test.alone.is_some(), "run again alone: {ruling:?}");
    assert_eq!(
        test.narrowed_to.as_deref(),
        Some("sh check.sh narrowed -p xtask -p base -p leaf -p mid")
    );
}
