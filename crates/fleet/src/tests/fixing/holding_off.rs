//! A Job whose test another Job is fixing, held off that test's files until the
//! fix has landed and is in its own copy. #1673.
//!
//! Job A reported the test, Job B is the fix, and the claim is written straight
//! to the store, `waiting`'s shape. A works a Drone per task, so the release is
//! read off the second task's Drone: Fleet catches a branch up only at a spawn.

use adapter_traits::{BroughtUpToDate, Standing, WorkProduct, Worktree};
use core_model::{
    Approach, Breakage, BreakageClaim, EvidenceType, JobId, ManifestId, NewTask, PlanChange,
    RepoPath, Ulid,
};
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::fixing::{FixAnswer, NotFixed};
use crate::scope::NotDeclared;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fitted_over, manifest, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

use super::{fix_for, TEST};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const HELD: &str = "src/parse.rs";
const BLOCK: &str = "FILES ANOTHER JOB IS FIXING";

/// Plan, then an `implement` that declares a scope and works a Drone per task.
fn held_workflow() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-held.yml"),
        "version: 1\nworkflow_id: fixture-held\nname: fixture\nstructure: linear\n\
         steps:\n  - id: plan\n    label: \"Plan the change\"\n    \
         evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      \
         - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    \
         advance_gate: auto\n  - id: implement\n    label: \"Implement\"\n    \
         follows_plan: true\n    drone_per_task: true\n    \
         evidence: {submitted: {type: diff}}\n    \
         evidence_scope: {context_source: drone_declared}\n    \
         mechanical_checks:\n      - { type: diff_nonempty }\n    delivers: false\n    \
         advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

/// A Fleet whose catch-up writes [`HELD`] into the Job's copy, standing in
/// for the fix arriving with the base.
fn a_held_fleet(home: &TempDir) -> Fixture {
    let work = FakeWorkProduct::changed(&["src/read.rs"]);
    let copy = work.holding();
    let mut fittings = fitted_over(
        home,
        work,
        FakeHarness::that_listens(),
        FakeVcs::new().writing_into(copy, &[HELD]),
    );
    fittings.starting().workflows = one(held_workflow());
    Fleet::assembled(fittings)
}

/// Job A, proposed, and Job B claiming [`TEST`] in A's repository for [`HELD`].
async fn a_held_by_b(fleet: &Fixture, home: &TempDir) -> (JobId, JobId) {
    let a = fleet
        .propose(a_proposal_for("bound the parser", "fixture-held"))
        .await
        .expect("A is proposed");
    worktree_directory(home, &a);
    let b = fleet
        .propose(a_proposal_for("fix the parser on main", "fixture-held"))
        .await
        .expect("B is proposed");
    let owner = fleet.names().owner_of(a.id()).expect("an owner");
    let now = fleet.now();
    fleet
        .store()
        .lock()
        .await
        .claim_breakage(
            &BreakageClaim {
                fix: b.id().clone(),
                repository: ManifestId::carried(Ulid::carried(owner)),
                breakage: Breakage {
                    check: String::from("suite"),
                    test: TEST.to_string(),
                    failure: String::from("exited 1"),
                },
                reported_by: a.id().clone(),
                files: vec![RepoPath::new(HELD)],
            },
            &now,
        )
        .expect("claimed");
    (a.id().clone(), b.id().clone())
}

/// What the last Drone was launched with: what it may not edit, and its brief.
fn last_launch(fleet: &Fixture) -> (Vec<String>, String) {
    let config = fleet
        .harness()
        .configured()
        .last()
        .cloned()
        .expect("a Drone was launched");
    (
        config.toolbelt().held_off().to_vec(),
        config.prompt().as_str().to_string(),
    )
}

async fn declares(fleet: &Fixture, job: &JobId, path: &str) -> Result<(), NotDeclared> {
    fleet
        .declare_scope(
            job,
            &DeclareScope {
                context_paths: vec![path.to_string()],
            },
        )
        .await
        .map(|_| ())
}

async fn hands_in(fleet: &Fixture, kind: EvidenceType, claimed: &'static str) {
    submitted_by_the_one(
        fleet,
        Call {
            evidence_type: kind,
            claimed: Claimed(claimed),
            shown_by: ShownBy("the fixture"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("handed in");
    fleet.turn().await.expect("a turn");
}

/// The proof the owner asked for: told, denied and refused while B fixes the
/// test, still held once B lands, and free once A's copy holds B's change.
#[tokio::test]
async fn a_job_is_held_off_a_test_another_job_fixes_until_the_fix_is_in_its_copy() {
    let home = TempDir::new();
    let fleet = a_held_fleet(&home);
    let (a, b) = a_held_by_b(&fleet, &home).await;

    dispatched(&fleet, &a).await.expect("A dispatches");
    let (held, brief) = last_launch(&fleet);
    assert_eq!(held, [HELD], "the plan's Drone is launched unable to edit it");
    assert!(brief.contains(BLOCK), "{brief}");
    assert!(brief.contains("fix the parser on main"), "{brief}");
    assert!(brief.contains(&format!("`{HELD}`")), "{brief}");

    let task = |title: &str| NewTask::new(title, "", &[], "").expect("a title");
    fleet
        .change_plan(
            &a,
            &PlanChange::Recorded {
                approach: Approach::new("Bound it, then cover it").expect("an approach"),
                tasks: vec![task("Bound the parser"), task("Cover the bound")],
            },
        )
        .await
        .expect("the plan records");
    hands_in(&fleet, EvidenceType::Plan, "Planned as two tasks.").await;
    let (held, brief) = last_launch(&fleet);
    assert_eq!(held, [HELD], "so is the first task's Drone");
    assert!(brief.contains(BLOCK), "{brief}");

    let refused = declares(&fleet, &a, HELD).await;
    assert!(
        matches!(&refused, Err(NotDeclared::HeldOffByFix { paths, .. }) if paths == &[RepoPath::new(HELD)]),
        "{refused:?}"
    );
    assert!(
        matches!(declares(&fleet, &a, "src").await, Err(NotDeclared::HeldOffByFix { .. })),
        "a directory over the file reaches it"
    );
    declares(&fleet, &a, "src/read.rs")
        .await
        .expect("the rest of the part is A's");

    // B lands. A's copy has not taken it, so the hold stands.
    fleet.fix_settled(&b, true).await;
    assert!(
        matches!(declares(&fleet, &a, HELD).await, Err(NotDeclared::HeldOffByFix { .. })),
        "a landed fix not yet in A's copy still holds it"
    );

    // The next task's Drone is put on a branch the base moved under.
    fleet.vcs().now_behind(
        Standing::Behind { commits: 1 },
        Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 1,
        }),
    );
    hands_in(&fleet, EvidenceType::Diff, "The parser is bounded.").await;
    let (held, brief) = last_launch(&fleet);
    assert!(held.is_empty(), "the second task's Drone may edit it: {held:?}");
    assert!(!brief.contains(BLOCK), "{brief}");
    assert!(
        brief.contains("that fix is already in your copy"),
        "the landed line says so: {brief}"
    );
    let copy = fleet
        .work()
        .changed_files(&Worktree::at(home.path().to_string_lossy(), "armada/a"))
        .expect("A's copy reads");
    assert!(
        copy.paths().iter().any(|path| path == HELD),
        "A's copy holds B's change: {:?}",
        copy.paths()
    );
    declares(&fleet, &a, HELD)
        .await
        .expect("and A may change the file again");
}

/// A Drone names the test's files: one not on main, or reaching outside it,
/// drafts nothing, and one that is rides the claim and is held off the reporter.
#[tokio::test]
async fn the_files_a_drone_names_are_checked_against_main_and_kept_on_the_claim() {
    let home = TempDir::new();
    let fleet = super::a_fleet(&home, super::gated_on("/usr/bin/false {}"), 1);
    let reporter = super::started(&fleet, &home).await;

    for named in ["src/nowhere.rs", "../outside.rs", "/etc/hosts"] {
        let mut asked = fix_for(TEST);
        asked.files = vec![named.to_string()];
        let refused = fleet.draft_fix(&reporter, asked).await;
        assert!(
            matches!(&refused, Err(NotFixed::NotOnMain { files }) if files == &[named]),
            "{named}: {refused:?}"
        );
    }

    std::fs::create_dir_all(home.path().join("src")).expect("a source directory");
    std::fs::write(home.path().join(HELD), "fn parse() {}\n").expect("the test's file");
    let mut asked = fix_for(TEST);
    asked.files = vec![HELD.to_string()];
    let FixAnswer::Started(running) = fleet.draft_fix(&reporter, asked).await.expect("asked")
    else {
        panic!("nothing claims the test yet");
    };
    running
        .finished()
        .await
        .expect("the run reported")
        .expect("drafted");

    let job = fleet.load(&reporter).await.expect("the reporter");
    let claims = fleet.breakages_of(&job).await.expect("read");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].held_off, [HELD], "the wire names the held file");
    assert_eq!(
        fleet.held_off(&reporter).await.paths(),
        [RepoPath::new(HELD)],
        "and it is held off the reporter"
    );
}
