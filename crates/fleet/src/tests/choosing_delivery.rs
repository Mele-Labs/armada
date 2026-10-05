//! How the work leaves the worktree, as approved: stop at the branch, or a
//! pull request set to merge itself. Since 23.24.

use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use testkit::{Delivered, Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::started;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, one, two_steps_gated_on_a_manifest_rule,
    worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir, delivering: Delivering) -> Arc<Fixture> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_manifest_rule(
        "summarise",
        "auto_merge",
        Some("summarise"),
    ));
    fittings.vcs = FakeVcs::new()
        .with_ref_at("main", "a".repeat(40))
        .delivering(delivering);
    Arc::new(Fleet::assembled(fittings))
}

fn body(json: &str) -> ipc::ApproveDispatch {
    ipc::decode("an approval", json.as_bytes()).expect("decodes")
}

/// Approve with `landing`, then carry the Job to the delivering step.
async fn delivered_as(fleet: &Arc<Fixture>, home: &TempDir, landing: &str) -> core_model::Job {
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    worktree_directory(home, &job);
    let json = format!(r#"{{"landing": {landing}}}"#);
    fleet
        .approve_as_left(job.id(), &body(&json))
        .await
        .expect("approved");
    started(fleet, job.id()).await.expect("dispatched");
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("handed in");
    fleet.turn().await.expect("a turn");
    started(fleet, job.id()).await.expect("the delivering step");
    fleet.load(job.id()).await.expect("the Job")
}

fn log_of(home: &TempDir, job: &core_model::Job) -> String {
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &job.handle(),
    ))
    .unwrap_or_default()
}

/// **Stop at the branch**: committed, and no push, pull request or merge.
#[tokio::test]
async fn a_local_job_is_committed_and_nothing_goes_out() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = delivered_as(&fleet, &home, r#"{"local": true, "pr_mode": "draft"}"#).await;

    let did = fleet.vcs().delivered();
    assert!(
        !did.iter().any(|one| matches!(
            one,
            Delivered::Pushed { .. }
                | Delivered::PushedForcing { .. }
                | Delivered::OpenedForReview { .. }
                | Delivered::Merged { .. }
                | Delivered::AutoMerge { .. }
        )),
        "{did:?}"
    );
    assert!(log_of(&home, &job).contains("stop at its branch"));
    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    let landing = detail.landing.expect("a landing");
    assert!(landing.local && !landing.auto_merge);
    assert_eq!(
        landing.pr_mode.as_wire(),
        "ready",
        "pr_mode is ignored while local holds: one answer"
    );
    assert_eq!(detail.delivery.and_then(|d| d.pull_request), None);
}

/// **The forge's auto-merge is turned on when the pull request opens.**
#[tokio::test]
async fn an_auto_merge_job_has_the_forge_merge_its_pull_request() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = delivered_as(&fleet, &home, r#"{"auto_merge": true}"#).await;
    let did = fleet.vcs().delivered();
    assert!(did.contains(&Delivered::AutoMerge {
        pull_request: "https://forge.invalid/armada/pull/1".to_string()
    }));
    assert!(log_of(&home, &job).contains("auto-merge is on"));
}

/// **A repository that does not allow it is said, not skipped**: the forge's
/// words are in the Job's log, and the pull request is open all the same.
#[tokio::test]
async fn a_forge_that_refuses_auto_merge_is_said_in_the_log() {
    let home = TempDir::new();
    let delivering = Delivering {
        auto_merge: Err("Auto merge is not allowed for this repository".to_string()),
        ..Delivering::default()
    };
    let fleet = a_fleet(&home, delivering);
    let job = delivered_as(&fleet, &home, r#"{"auto_merge": true}"#).await;
    let log = log_of(&home, &job);
    assert!(log.contains("would not turn it on"), "{log}");
    assert!(log.contains("Auto merge is not allowed"), "{log}");
    assert!(fleet
        .vcs()
        .delivered()
        .iter()
        .any(|one| matches!(one, Delivered::OpenedForReview { .. })));
}

/// A job with neither setting is as it was: no auto-merge asked.
#[tokio::test]
async fn a_job_with_neither_setting_asks_the_forge_for_nothing_more() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    delivered_as(&fleet, &home, r#"{"pr_mode": "ready"}"#).await;
    assert!(!fleet
        .vcs()
        .delivered()
        .iter()
        .any(|one| matches!(one, Delivered::AutoMerge { .. })));
}

/// `local` opens no pull request, so there is nothing to auto-merge.
#[tokio::test]
async fn local_with_auto_merge_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    let sent = body(r#"{"landing": {"local": true, "auto_merge": true}}"#);
    let why = Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(sent))
        .await
        .expect_err("refused");
    let Refusal::Unacceptable(error) = &why else {
        panic!("{why:?}");
    };
    assert!(
        error.message.contains("nothing to auto-merge"),
        "{}",
        error.message
    );
}
