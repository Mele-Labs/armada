//! What a person changes on a proposal, through the store, the fake VCS and
//! the routes: spike 022, slice 4. The hermetic claim is
//! `crates/acceptance/tests/drone_per_task.rs`; this is what it cannot reach —
//! the write, the lease, the pull request and the issue rotation.

use std::sync::Arc;
use std::time::Duration;

use api::{Commands, Queries, Refusal};
use core_model::{AutoMerge, CriterionOrigin, IssueSource, JobStatus, Timestamp};
use testkit::{Delivered, FakeHarness, FakeJudge, FakeLinkLookup, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::started;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, one, two_steps_gated_on_a_manifest_rule,
    worktree_directory,
};
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

/// A repository holding three branches, on a workflow whose delivering step
/// defers to the repository's `auto_merge`.
fn a_fleet(home: &TempDir) -> Arc<Fixture> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_manifest_rule(
        "summarise",
        "auto_merge",
        Some("summarise"),
    ));
    fittings.vcs = FakeVcs::new()
        .with_ref_at("main", "a".repeat(40))
        .with_ref_at("reader/bound", "b".repeat(40))
        .with_ref_at("release/2.0", "c".repeat(40));
    Arc::new(Fleet::assembled(fittings))
}

fn body(json: &str) -> ipc::ApproveDispatch {
    ipc::decode("an approval", json.as_bytes()).expect("decodes")
}

const LEFT: &str = r#"{
  "title": "Bound the reader at the last row",
  "facts": "Stop at the last row.",
  "gates": [
    {"step_id": "implement", "checks": true, "judge": false, "you": true},
    {"step_id": "summarise", "checks": true, "judge": false, "you": false, "overridden": true}
  ],
  "criteria": [
    {"criterion_id": "c1", "text": "the symptom is gone at the last row", "source": "check"},
    {"text": "the bound is named", "source": "judge"}
  ],
  "tiers": {"difficult": "another-model"},
  "drone_cap": 3,
  "landing": {"target": "release/2.0", "from_ref": "reader/bound", "pr_mode": "draft"}
}"#;

/// Every field of Approve's body is what `get_job` reads back and what the
/// store rebuilds the Job from, and the Job is released.
#[tokio::test]
async fn an_approval_keeps_every_field_it_carries_and_releases_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");

    let summary = Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(LEFT)))
        .await
        .expect("approved as left");
    assert_eq!(summary.status.as_wire(), "queued");

    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    assert_eq!(detail.job.title, "Bound the reader at the last row");
    assert_eq!(detail.facts.as_deref(), Some("Stop at the last row."));
    let criteria: Vec<(&str, &str, Option<ipc::CriterionOrigin>)> = detail
        .acceptance_criteria
        .iter()
        .map(|c| (c.criterion_id.as_str(), c.text.as_str(), c.origin.clone()))
        .collect();
    assert_eq!(
        criteria,
        [
            (
                "c1",
                "the symptom is gone at the last row",
                Some(ipc::CriterionOrigin::Person)
            ),
            (
                "c2",
                "the bound is named",
                Some(ipc::CriterionOrigin::Person)
            ),
        ],
        "the form's line was a person's, and so is the one typed at the gate"
    );
    assert_eq!(
        detail.steps[0].advance_gate.map(|g| g.as_wire()),
        Some("human_always")
    );
    assert_eq!(
        detail.steps[1].advance_gate.map(|g| g.as_wire()),
        Some("manifest_rule:auto_merge"),
        "an overridden step keeps deferring on the record"
    );
    assert_eq!(
        detail.policy_overrides,
        Some(ipc::PolicyOverrides {
            auto_merge: Some("checks-pass".to_string()),
            review_gate: None,
        })
    );
    assert_eq!(detail.tiers.difficult.as_deref(), Some("another-model"));
    assert_eq!(detail.drone_cap, Some(3));
    assert_eq!(
        detail.landing,
        Some(ipc::LandingRule {
            target: Some("release/2.0".to_string()),
            from_ref: Some("reader/bound".to_string()),
            pr_mode: ipc::PrMode::from_wire("draft").expect("a mode"),
            complete_when: Some(ipc::CompleteWhen::Delivered),
        })
    );
    assert!(detail.approved_at.is_some(), "the press is dated");

    let rebuilt = fleet.load(job.id()).await.expect("the store rebuilds it");
    assert_eq!(rebuilt.title().as_str(), "Bound the reader at the last row");
    assert_eq!(rebuilt.acceptance_criteria().len(), 2);
    let served = fleet.served_by(&rebuilt).expect("served");
    assert_eq!(
        fleet.policies_for(&served, job.id()).await.auto_merge(),
        AutoMerge::ChecksPass,
        "the repository says never, and the override wins"
    );
}

/// A body naming a branch the repository does not hold, or a setting Fleet
/// does not run, keeps nothing: the proposal is as it was, still at its gate.
#[tokio::test]
async fn a_refused_approval_keeps_nothing_of_the_proposal() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");

    for (json, wanted) in [
        (
            LEFT.replace("release/2.0", "nowhere"),
            "fleet.no_such_branch",
        ),
        (
            LEFT.replace(r#""pr_mode": "draft""#, r#""complete_when": "pr_merged""#),
            "fleet.unacceptable_proposal",
        ),
        (
            LEFT.replace(
                r#""tiers": {"difficult": "another-model"},"#,
                r#""tiers": {"easy": "nobody-offers-this"},"#,
            ),
            // `set_tiers`' own refusal of a model nobody offers.
            "fleet.not_resumable",
        ),
    ] {
        let refused =
            Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(&json)))
                .await
                .expect_err("refused");
        assert_eq!(code(&refused), wanted, "{refused:?}");
    }
    let still = fleet.load(job.id()).await.expect("reads");
    assert_eq!(still.status(), JobStatus::AwaitingApproval);
    assert_eq!(still.title().as_str(), "the reader", "nothing was kept");
}

/// `edit_job` saves without releasing, and refuses a Job already released.
#[tokio::test]
async fn an_edit_saves_the_words_and_leaves_the_job_at_its_gate() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    let edit: ipc::EditJob =
        ipc::decode("an edit", br#"{"title":"Bound the reader"}"#).expect("decodes");

    let summary = Commands::edit_job(Arc::clone(&fleet), job.id().into(), edit.clone())
        .await
        .expect("saved");
    assert_eq!(summary.status.as_wire(), "awaiting_approval");
    assert_eq!(summary.title, "Bound the reader");

    Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), None)
        .await
        .expect("approved as it stands");
    let refused = Commands::edit_job(Arc::clone(&fleet), job.id().into(), edit)
        .await
        .expect_err("frozen");
    assert_eq!(code(&refused), "fleet.proposal_frozen");
}

/// The worktree is cut from `from_ref`, and the pull request opens against
/// `target`, as a draft.
#[tokio::test]
async fn the_work_starts_where_it_was_cut_from_and_lands_as_a_draft_where_it_was_aimed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);
    fleet
        .approve_as_left(job.id(), &body(LEFT))
        .await
        .expect("approved");
    started(&fleet, job.id()).await.expect("dispatched");
    assert_eq!(fleet.vcs().cut_from(), ["reader/bound"]);

    // `implement` stops for a person now, so the press carries it on.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("handed in");
    fleet.turn().await.expect("a turn");
    Commands::approve_review(Arc::clone(&fleet), job.id().into())
        .await
        .expect("the person takes it");
    started(&fleet, job.id())
        .await
        .expect("the delivering step");

    let opened = fleet
        .vcs()
        .delivered()
        .into_iter()
        .find_map(|did| match did {
            Delivered::OpenedForReview { base, review } => Some((base, review)),
            _ => None,
        })
        .expect("a pull request was opened");
    assert_eq!(opened.0, "release/2.0");
    assert!(opened.1.draft(), "a draft, as chosen at approval");
}

/// Approval with no body is the proposal as it stands, and lands as ever.
#[tokio::test]
async fn an_approval_with_no_body_runs_the_proposal_as_it_stands() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), None)
        .await
        .expect("approved");
    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    assert_eq!(detail.landing, None);
    assert_eq!(detail.policy_overrides, None);
    assert_eq!(detail.drone_cap, None);
}

/// A request linking an issue marks every criterion the proposer read out of
/// it as the issue's, and the issue rotation notices an edit made after the
/// read — once.
#[tokio::test]
async fn a_criterion_from_an_issue_says_so_and_the_rotation_notices_when_it_moves() {
    let home = TempDir::new();
    let links = Arc::new(
        FakeLinkLookup::resolving("example.test/issues/9", "the reader drops the last row")
            .linking("y#9", "https://example.test/x/y/issues/9"),
    );
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(FakeJudge::saying(
        "workflow: bug\ntitle: The reader drops the last row\ndone_when: the last row is read",
    ));
    fittings.links = links.clone();
    fittings.noticing = Noticing::every(Duration::ZERO);
    let fleet = Fleet::assembled(fittings);

    let made = fleet
        .propose_from("https://example.test/issues/9", None)
        .await
        .expect("a proposal");
    let job = made[0].id().clone();
    assert_eq!(
        made[0].acceptance_criteria()[0].origin,
        CriterionOrigin::Issue
    );

    assert_eq!(
        fleet.notice_an_issue().await.expect("asks"),
        None,
        "a forge that will not say is no news"
    );
    links.edit_issue_at("2099-01-01T00:00:00Z");
    assert_eq!(
        fleet.notice_an_issue().await.expect("asks"),
        Some(job.clone())
    );
    assert_eq!(
        fleet.notice_an_issue().await.expect("asks"),
        None,
        "an edit already noticed is not news twice"
    );
    let source: IssueSource = fleet
        .store()
        .lock()
        .await
        .issue_source(&job)
        .expect("reads")
        .expect("kept");
    assert_eq!(
        source.moved_at,
        Some(Timestamp::from_rfc3339("2099-01-01T00:00:00Z"))
    );
    let detail = Queries::get_job(&fleet, (&job).into())
        .await
        .expect("the detail");
    assert_eq!(
        detail.acceptance_criteria[0].origin,
        Some(ipc::CriterionOrigin::Issue {
            reference: "y#9".to_string(),
            url: "https://example.test/x/y/issues/9".to_string(),
        })
    );
    assert!(detail.acceptance_criteria[0].origin_moved_at.is_some());
}

/// What a person set at dispatch is laid on the Job the request became, at
/// its gate: a cap, and a person at the delivering step.
#[tokio::test]
async fn the_dispatch_settings_reach_the_proposal_and_leave_it_at_its_gate() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    let settings: ipc::DispatchSettings =
        ipc::decode("settings", br#"{"drone_cap":2,"lands":"you_at_review"}"#).expect("decodes");
    fleet
        .dispatched_as_set(std::slice::from_ref(&job), &settings)
        .await
        .expect("laid on");
    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    assert_eq!(detail.job.status.as_wire(), "awaiting_approval");
    assert_eq!(detail.drone_cap, Some(2));
    assert_eq!(
        detail
            .policy_overrides
            .and_then(|o| o.auto_merge)
            .as_deref(),
        Some("never"),
        "the delivering step defers to auto_merge, so a person at review is never"
    );
}

/// **A `from_ref` the repository lacks is made at its start point** (23.21),
/// and the Job lands in it where `target` names it too. A start point the
/// repository lacks is refused by name, and makes nothing.
#[tokio::test]
async fn a_branch_that_does_not_exist_yet_is_cut_from_its_start_point() {
    use adapter_traits::Vcs;
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    let held = |fleet: &Fixture| -> Vec<String> {
        fleet
            .vcs()
            .branches("/a-repository", None)
            .expect("listed")
            .into_iter()
            .map(|branch| branch.name)
            .collect()
    };

    let nowhere = r#"{"landing": {"from_ref": "reader/next", "start_point": "nowhere"}}"#;
    let refused =
        Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(nowhere)))
            .await
            .expect_err("a start point the repository lacks");
    assert_eq!(code(&refused), "fleet.no_such_branch");
    let unstarted = r#"{"landing": {"from_ref": "reader/next"}}"#;
    let refused =
        Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(unstarted)))
            .await
            .expect_err("a new branch with nowhere to start it");
    assert_eq!(code(&refused), "fleet.no_such_branch");
    assert!(
        !held(&fleet).contains(&"reader/next".to_string()),
        "nothing was made"
    );

    let cut = r#"{"landing": {"from_ref": "reader/next", "start_point": "release/2.0", "target": "reader/next"}}"#;
    Commands::approve_dispatch(Arc::clone(&fleet), job.id().into(), Some(body(cut)))
        .await
        .expect("approved onto a new branch");
    assert!(held(&fleet).contains(&"reader/next".to_string()));
    assert_eq!(
        fleet
            .vcs()
            .base_commit("/a-repository", Some("reader/next"))
            .expect("read"),
        Some("c".repeat(40)),
        "made where its start point is"
    );
    let detail = Queries::get_job(&*fleet, job.id().into())
        .await
        .expect("the detail");
    let landing = detail.landing.expect("a landing");
    assert_eq!(landing.from_ref.as_deref(), Some("reader/next"));
    assert_eq!(landing.target.as_deref(), Some("reader/next"));
}
