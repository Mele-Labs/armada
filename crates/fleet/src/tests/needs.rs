//! The claim of #1059, Fleet's half: two Jobs on one repository each declare a
//! need on the file that lists migrations; the second is told the first is ahead
//! and what it took; the second, ready first, is refused at its merge naming
//! what it waits behind; once the first lands, the second lands; a dropped Job's
//! need blocks nobody. And a Job and a session using `armada need` stand in ONE
//! order, because both read and write the same files.
//!
//! **A real repository under the fixture's home**, because a need is keyed by a
//! branch and a branch is what git says exists. What git does is asserted in
//! `adapters`; what is under test here is what Fleet does with the answer.

use std::process::Command;
use std::time::Duration;

use adapter_traits::Landing;
use adapters::needs::Needs;
use config::Manifest;
use core_model::{JobId, JobStatus};
use ipc::mcp::NeedClaim;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one, two_steps_gated_on_a_person};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture};
use crate::tests::tmp::TempDir;
use testkit::FakeWorkProduct;

const PATH: &str = "crates/store/src/migrations.rs";

fn git(repo: &std::path::Path, args: &[&str]) {
    let run = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
}

/// A Fleet holding delivered Jobs for a person, in a real repository.
fn holding_under(home: &TempDir, merge_by: &str) -> Fixture {
    holding_on(home, merge_by, false)
}

/// The same, with `main` known to the fake as the commit the repository's
/// `main` is at, so a branch's diff from its base can be read.
fn holding_on(home: &TempDir, merge_by: &str, base_known: bool) -> Fixture {
    git(
        home.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(
        home.path(),
        &["commit", "--allow-empty", "-m", "start", "--quiet"],
    );
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    if base_known {
        let sha = Command::new("git")
            .arg("-C")
            .arg(home.path())
            .args(["rev-parse", "main"])
            .output()
            .expect("git on PATH");
        let sha = String::from_utf8_lossy(&sha.stdout).trim().to_string();
        let held = std::mem::replace(&mut fittings.vcs, testkit::FakeVcs::new());
        fittings.vcs = held.with_ref_at("main", sha);
    }
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.starting().manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\nmerge_by: {merge_by}\n"),
    )
    .expect("a manifest");
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// A Job at its gate whose branch exists in the repository, as it does in life.
async fn at_the_gate(fleet: &Fixture, home: &TempDir) -> (JobId, String) {
    let job = at_the_gate_having_delivered(fleet, home).await;
    let branch = fleet
        .load(&job)
        .await
        .expect("the Job")
        .branch()
        .expect("a branch")
        .as_str()
        .to_string();
    git(home.path(), &["branch", &branch]);
    (job, branch)
}

fn migration(took: Option<&str>) -> Vec<NeedClaim> {
    vec![NeedClaim {
        path: PATH.to_string(),
        what: "a new migration".to_string(),
        took: took.map(str::to_string),
    }]
}

/// What the refusal says, and the wire code it carries.
fn refused(fleet: &Fixture, why: Adrift) -> (String, String) {
    let said = why.to_string();
    match fleet.refusal(why) {
        api::Refusal::IllegalMove(wire) => (wire.code.to_string(), said),
        other => panic!("a person's to answer, so a conflict: {other:?}"),
    }
}

async fn the_claim_under(merge_by: &str) {
    let home = TempDir::new();
    let fleet = holding_under(&home, merge_by);
    let (first, first_branch) = at_the_gate(&fleet, &home).await;
    let (second, _) = at_the_gate(&fleet, &home).await;

    // The first declares and says what it took. Nothing is ahead of it.
    fleet.needs_declared(&first, &migration(Some("V95"))).await;
    assert!(
        fleet.peer_news_for_the_brief(&first).await.is_none(),
        "nothing is ahead of the first"
    );

    // The second is told who is ahead and what that one took.
    fleet.needs_declared(&second, &migration(None)).await;
    let told = fleet
        .peer_news_for_the_brief(&second)
        .await
        .expect("the second is told");
    let told = told.text();
    assert!(told.contains(PATH), "{told}");
    assert!(told.contains(&first_branch), "names who is ahead: {told}");
    assert!(told.contains("took V95"), "and what it took: {told}");

    // Ready first, the second is refused, naming what it waits behind.
    let held = fleet
        .merge_pull_request(&second)
        .await
        .expect_err("it waits behind the first");
    let (code, said) = refused(&fleet, held);
    assert_eq!(code, "fleet.merge_waiting_behind");
    assert!(
        said.contains("a new migration") && said.contains(PATH),
        "{said}"
    );
    assert!(
        said.contains(&first_branch) && said.contains("V95"),
        "{said}"
    );
    assert_eq!(
        fleet.load(&second).await.expect("the Job").status(),
        JobStatus::AwaitingReview,
        "refused, so it is where it was"
    );
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        0,
        "nothing reached the forge"
    );

    // The first lands, which spends its need; then the second lands.
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    let landed = fleet
        .merge_pull_request(&first)
        .await
        .expect("the first lands");
    assert_eq!(landed.status(), JobStatus::CompletedSuccess);
    let line = Needs::of(home.path()).expect("needs").standing();
    assert!(
        line.iter().all(|need| need.branch != first_branch),
        "spent: {line:?}"
    );
    let landed = fleet
        .merge_pull_request(&second)
        .await
        .expect("then the second");
    assert_eq!(landed.status(), JobStatus::CompletedSuccess);
}

#[tokio::test]
async fn the_second_job_waits_behind_the_first_under_forge() {
    the_claim_under("forge").await;
}

#[tokio::test]
async fn the_second_job_waits_behind_the_first_under_push() {
    the_claim_under("push").await;
}

/// A dropped Job gives its need back, so the Job behind it is no longer held.
#[tokio::test]
async fn a_dropped_jobs_need_no_longer_blocks() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (first, _) = at_the_gate(&fleet, &home).await;
    let (second, _) = at_the_gate(&fleet, &home).await;
    fleet.needs_declared(&first, &migration(Some("V95"))).await;
    fleet.needs_declared(&second, &migration(None)).await;
    fleet
        .merge_pull_request(&second)
        .await
        .expect_err("held while the first stands");

    let dropped = fleet.reject(&first).await.expect("a person rejects it");
    assert!(dropped.status().is_terminal());

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&second)
        .await
        .expect("nothing is ahead any more");
}

/// One order, not two: a session's `armada need` and a Job's declaration are the
/// same files, so each is ahead of the other by when it declared.
#[tokio::test]
async fn a_job_and_a_session_stand_in_one_order() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, job_branch) = at_the_gate(&fleet, &home).await;
    git(home.path(), &["branch", "session"]);
    let session = Needs::of(home.path()).expect("the session's own view");

    // The session declared first, the way `armada need` does.
    session
        .declare("session", PATH, "a new migration")
        .expect("declared");
    session.took("session", PATH, "V96").expect("took");
    fleet.needs_declared(&job, &migration(None)).await;

    let held = fleet
        .merge_pull_request(&job)
        .await
        .expect_err("the Job waits behind the session");
    let (_, said) = refused(&fleet, held);
    assert!(said.contains("session") && said.contains("V96"), "{said}");

    // And the session sees the Job as behind it, in the same listing.
    let order: Vec<String> = session
        .standing()
        .into_iter()
        .map(|need| need.branch)
        .collect();
    assert_eq!(order, ["session".to_string(), job_branch.clone()]);
    assert!(session.behind("session").is_empty());
    assert_eq!(session.behind(&job_branch).len(), 1);

    // The session landing frees the Job.
    session.spend("session");
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&job)
        .await
        .expect("nothing is ahead");
}

/// The same rule `armada land` holds a session to: a Job whose branch appends a
/// migration with no need declared is refused at its merge, naming what to run,
/// and lands once it has declared.
#[tokio::test]
async fn a_job_that_took_a_migration_undeclared_is_refused() {
    let home = TempDir::new();
    let fleet = holding_on(&home, "forge", true);
    let (job, branch) = at_the_gate(&fleet, &home).await;
    let file = home.path().join(PATH);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("dirs");
    std::fs::write(&file, "pub const MIGRATIONS: &[&str] = &[\n    V1,\n];\n").expect("file");
    git(home.path(), &["add", "."]);
    git(home.path(), &["commit", "-m", "base", "--quiet"]);
    git(home.path(), &["branch", "-f", &branch]);
    git(home.path(), &["checkout", "--quiet", &branch]);
    std::fs::write(
        &file,
        "pub const MIGRATIONS: &[&str] = &[\n    V1,\n    V2,\n];\n",
    )
    .expect("file");
    git(home.path(), &["commit", "-am", "a migration", "--quiet"]);
    // `main` must be the fork: put it back where the branch left it.
    git(home.path(), &["checkout", "--quiet", "main"]);

    let held = fleet
        .merge_pull_request(&job)
        .await
        .expect_err("a number taken with no need");
    let (code, said) = refused(&fleet, held);
    assert_eq!(code, "fleet.merge_waiting_behind");
    assert!(
        said.contains(&format!("armada need {PATH} \"a new migration\"")),
        "{said}"
    );
    assert_eq!(fleet.vcs().times_asked_to_merge(), 0);

    fleet.needs_declared(&job, &migration(Some("V2"))).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&job)
        .await
        .expect("declared, it lands");
}
