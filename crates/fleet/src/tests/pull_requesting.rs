//! A pull request by repository and number, through a real Fleet and a scripted
//! forge: what is read, what each act refuses before it writes, what it asks
//! the forge for, and what the session ledger holds afterwards. Since 23.48.
//!
//! **No forge is called.** `FakeVcs` stands in for `gh`, and a test that says a
//! merge was refused says so by counting what the fake was asked to write.

use adapter_traits::{
    NotMerged, PullRequestFacts, PullRequestStanding, UnderReview, WhatTheForgeRan,
};
use api::{PullRequests, Refusal, Sessions};
use ipc::{
    AttachmentReport, AttachmentState, ForgeChecks, ManifestId, PullRequestStanding as Wire,
    ReviewPullRequest, SessionFact, SessionId, SessionRecord, SessionReport,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Merging};

use crate::daemon::Fleet;
use crate::tests::daemon::{fittings, one, workflow_named};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ADDRESS: &str = "https://forge.invalid/armada/pull/12";

fn a_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().workflows = one(workflow_named("code_review"));
    Fleet::assembled(fittings)
}

fn facts(standing: PullRequestStanding) -> PullRequestFacts {
    PullRequestFacts {
        standing,
        branch: "fleet/pr-acts".into(),
        auto_merge: false,
        title: "Take a draft out".into(),
        url: ADDRESS.into(),
    }
}

fn forge_ran(fleet: &Fixture, ran: WhatTheForgeRan) {
    fleet.vcs().now_under_review(UnderReview {
        checks: ran,
        ..UnderReview::unreadable()
    });
}

fn passed() -> WhatTheForgeRan {
    WhatTheForgeRan::AllPassed { checks: 2 }
}

fn running() -> WhatTheForgeRan {
    WhatTheForgeRan::StillWaiting {
        finished: 1,
        checks: 2,
    }
}

fn manifest(fleet: &Fixture) -> (ManifestId, String) {
    let one = fleet.repositories().first().expect("a served repository");
    (
        ManifestId::carried(one.manifest().id().as_str()),
        one.root().to_string(),
    )
}

fn code(refusal: &Refusal) -> &str {
    &refusal.error().code
}

async fn a_session_holding_pull_request_12(fleet: &Fixture, id: &str) -> SessionRecord {
    let (_, root) = manifest(fleet);
    let report = |fact| SessionReport {
        harness: "a_harness".into(),
        session_id: SessionId::carried(id),
        fact,
    };
    fleet
        .report_session(report(SessionFact::Started {
            cwd: root,
            title: Some("fix the ledger".into()),
            origin: ipc::SessionOrigin::Terminal,
            mod_version: None,
        }))
        .await
        .expect("started");
    fleet
        .report_session(report(SessionFact::Attached {
            attachment: AttachmentReport {
                kind: "pr".into(),
                target: "12".into(),
                detail: Default::default(),
            },
        }))
        .await
        .expect("attached")
}

async fn the_row(fleet: &Fixture, id: &str, kind: &str) -> ipc::Attachment {
    let listed = fleet
        .list_sessions(None, Some(id.into()), None)
        .await
        .expect("listed");
    let session = listed
        .sessions
        .into_iter()
        .find(|one| one.id.as_str() == id)
        .expect("the session");
    session
        .attachments
        .into_iter()
        .find(|one| one.kind == kind)
        .expect("the row")
}

// ----------------------------------------------------------------- the read

#[tokio::test]
async fn a_pull_request_is_read_as_the_forge_shows_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Draft)));
    forge_ran(&fleet, running());

    let read = fleet.get_pull_request(id.clone(), 12).await.expect("read");

    assert_eq!(read.state, Wire::Draft);
    assert_eq!(read.checks, ForgeChecks::Pending);
    assert!(!read.auto_merge);
    assert_eq!(
        (read.number, read.branch.as_str(), read.address.as_str()),
        (12, "fleet/pr-acts", ADDRESS)
    );
    assert_eq!(read.title, "Take a draft out");
    assert_eq!(read.manifest_id, id);
}

#[tokio::test]
async fn failed_checks_are_named_and_a_repository_that_runs_nothing_is_pending() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));

    forge_ran(
        &fleet,
        WhatTheForgeRan::SomeFailed {
            failed: vec![adapter_traits::FromOutside::verbatim("ci")],
            checks: 3,
        },
    );
    assert_eq!(
        fleet.get_pull_request(id.clone(), 12).await.unwrap().checks,
        ForgeChecks::Failed {
            failing: vec!["ci".into()]
        }
    );

    forge_ran(&fleet, WhatTheForgeRan::NothingRan);
    assert_eq!(
        fleet.get_pull_request(id, 12).await.unwrap().checks,
        ForgeChecks::Pending,
        "nothing passed, so nothing is said to have"
    );
}

#[tokio::test]
async fn a_forge_that_will_not_answer_is_a_fault_naming_why_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);

    // No facts: silent.
    let refused = fleet.get_pull_request(id.clone(), 12).await.unwrap_err();
    assert_eq!(code(&refused), "fleet.pull_request_unreadable");
    assert_eq!(refused.status(), 500);

    // Facts, and the checks silent: a state with no checks is not a reading.
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    let refused = fleet.get_pull_request(id, 12).await.unwrap_err();
    assert_eq!(code(&refused), "fleet.pull_request_unreadable");
}

#[tokio::test]
async fn a_repository_nothing_serves_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let refused = fleet
        .get_pull_request(ManifestId::carried("nobody"), 12)
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 422, "{refused:?}");
}

// ------------------------------------------------------------ the ledger

#[tokio::test]
async fn a_read_refreshes_the_detail_of_the_row_a_session_holds() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(
        &fleet,
        WhatTheForgeRan::SomeFailed {
            failed: vec![adapter_traits::FromOutside::verbatim("ci")],
            checks: 2,
        },
    );

    fleet.get_pull_request(id, 12).await.expect("read");

    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.state, AttachmentState::Standing);
    let said = |key: &str| row.detail.get(key).map(String::as_str);
    assert_eq!(said("state"), Some("open"));
    assert_eq!(said("checks"), Some("failed"));
    assert_eq!(said("failing"), Some("ci"));
    assert_eq!(said("auto_merge"), Some("false"));
    assert_eq!(said("branch"), Some("fleet/pr-acts"));
    assert_eq!(said("address"), Some(ADDRESS));
    assert_eq!(said("title"), Some("Take a draft out"));
}

#[tokio::test]
async fn a_merged_pull_request_settles_the_row_spent_and_a_closed_one_gives_it_back() {
    for (standing, settled) in [
        (PullRequestStanding::Merged, AttachmentState::Spent),
        (PullRequestStanding::Closed, AttachmentState::GivenBack),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        a_session_holding_pull_request_12(&fleet, "s1").await;
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, passed());

        fleet.get_pull_request(id, 12).await.expect("read");

        let row = the_row(&fleet, "s1", "pr").await;
        assert_eq!(row.state, settled);
        assert!(row.detail.contains_key("state"));
    }
}

#[tokio::test]
async fn a_row_in_another_repository_or_another_number_is_left_alone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());

    fleet.get_pull_request(id, 13).await.expect("read");

    assert!(the_row(&fleet, "s1", "pr").await.detail.is_empty());
}

// --------------------------------------------------------------- ready

#[tokio::test]
async fn a_draft_is_taken_out_of_draft_and_the_answer_is_the_new_state() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Draft)));
    forge_ran(&fleet, running());

    let now = fleet.ready_pull_request(id, 12).await.expect("readied");

    assert_eq!(now.state, Wire::Open);
    assert_eq!(fleet.vcs().times_asked_to_ready(), 1);
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("state")
            .map(String::as_str),
        Some("open")
    );
}

#[tokio::test]
async fn readying_what_is_not_a_draft_is_refused_without_a_write() {
    for (standing, expected) in [
        (PullRequestStanding::Open, "fleet.pull_request_not_a_draft"),
        (PullRequestStanding::Merged, "fleet.merge_not_open"),
        (PullRequestStanding::Closed, "fleet.merge_not_open"),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, passed());

        let refused = fleet.ready_pull_request(id, 12).await.unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(refused.status(), 409);
        assert_eq!(fleet.vcs().times_asked_to_ready(), 0);
    }
}

#[tokio::test]
async fn a_forge_that_will_not_ready_it_says_so_in_its_own_words() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Draft)));
    forge_ran(&fleet, running());
    fleet.vcs().ready_refuses("GraphQL: not permitted");

    let refused = fleet.ready_pull_request(id, 12).await.unwrap_err();

    assert_eq!(code(&refused), "fleet.ready_refused");
    assert!(refused.error().message.contains("not permitted"));
}

// --------------------------------------------------------------- merge

#[tokio::test]
async fn a_pull_request_whose_checks_passed_is_merged_and_the_row_is_spent() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());

    let now = fleet
        .merge_pull_request_by_number(id, 12)
        .await
        .expect("merged");

    assert_eq!(now.state, Wire::Merged);
    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.state, AttachmentState::Spent);
    assert_eq!(row.detail.get("state").map(String::as_str), Some("merged"));
}

/// **The refusal that matters**: nothing reaches the forge while the checks run
/// or have failed.
#[tokio::test]
async fn a_merge_while_the_checks_have_not_passed_is_refused_before_it_writes() {
    for ran in [
        running(),
        WhatTheForgeRan::NothingRan,
        WhatTheForgeRan::SomeFailed {
            failed: vec![adapter_traits::FromOutside::verbatim("clippy")],
            checks: 2,
        },
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        fleet
            .vcs()
            .now_pull_request(Some(facts(PullRequestStanding::Open)));
        forge_ran(&fleet, ran.clone());

        let refused = fleet
            .merge_pull_request_by_number(id, 12)
            .await
            .unwrap_err();

        assert_eq!(code(&refused), "fleet.merge_checks_not_passed");
        assert_eq!(refused.status(), 409);
        assert_eq!(fleet.vcs().times_asked_to_merge(), 0, "{ran:?}");
        if matches!(ran, WhatTheForgeRan::SomeFailed { .. }) {
            assert!(refused.error().message.contains("clippy"), "names it");
        }
    }
}

#[tokio::test]
async fn a_draft_and_a_closed_pull_request_are_not_merged() {
    for (standing, expected) in [
        (PullRequestStanding::Draft, "fleet.pull_request_is_a_draft"),
        (PullRequestStanding::Closed, "fleet.merge_not_open"),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, passed());

        let refused = fleet
            .merge_pull_request_by_number(id, 12)
            .await
            .unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(fleet.vcs().times_asked_to_merge(), 0);
    }
}

#[tokio::test]
async fn one_already_merged_is_answered_as_it_stands_and_nothing_is_written() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Merged)));
    forge_ran(&fleet, passed());

    let now = fleet.merge_pull_request_by_number(id, 12).await.unwrap();

    assert_eq!(now.state, Wire::Merged);
    assert_eq!(fleet.vcs().times_asked_to_merge(), 0);
}

#[tokio::test]
async fn the_forges_own_refusals_keep_the_codes_a_job_merge_has() {
    for (why, expected, status) in [
        (
            NotMerged::Protected {
                said: "needs a review".into(),
            },
            "fleet.merge_branch_protected",
            409,
        ),
        (
            NotMerged::Conflicted {
                said: "conflicts".into(),
            },
            "fleet.merge_conflicted",
            409,
        ),
        (
            NotMerged::NoTool {
                said: "no gh".into(),
            },
            "fleet.merge_no_tool",
            500,
        ),
        (
            NotMerged::Refused {
                said: "something new".into(),
            },
            "fleet.merge_refused",
            500,
        ),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        fleet
            .vcs()
            .now_pull_request(Some(facts(PullRequestStanding::Open)));
        forge_ran(&fleet, passed());
        fleet.vcs().merging(Merging::Refuses(why));

        let refused = fleet
            .merge_pull_request_by_number(id, 12)
            .await
            .unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(refused.status(), status);
        assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    }
}

// ---------------------------------------------------------- auto-merge

#[tokio::test]
async fn auto_merge_is_asked_for_while_the_checks_run() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, running());

    let now = fleet.enable_auto_merge(id.clone(), 12).await.expect("on");

    assert!(now.auto_merge);
    assert_eq!(now.state, Wire::Open, "it merges nothing itself");
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("auto_merge")
            .map(String::as_str),
        Some("true")
    );

    // Asked twice, the second answers as it stands.
    fleet.enable_auto_merge(id, 12).await.expect("as it stands");
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn auto_merge_is_refused_once_the_checks_passed_or_failed_and_on_a_draft() {
    for (standing, ran, expected) in [
        (
            PullRequestStanding::Open,
            passed(),
            "fleet.pull_request_checks_passed",
        ),
        (
            PullRequestStanding::Open,
            WhatTheForgeRan::SomeFailed {
                failed: vec![adapter_traits::FromOutside::verbatim("ci")],
                checks: 1,
            },
            "fleet.merge_checks_not_passed",
        ),
        (
            PullRequestStanding::Draft,
            running(),
            "fleet.pull_request_is_a_draft",
        ),
        (
            PullRequestStanding::Merged,
            passed(),
            "fleet.merge_not_open",
        ),
    ] {
        let home = TempDir::new();
        let fleet = a_fleet(&home);
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, ran);

        let refused = fleet.enable_auto_merge(id, 12).await.unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(refused.status(), 409);
        assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 0);
    }
}

#[tokio::test]
async fn a_repository_that_does_not_allow_auto_merge_says_so_in_the_forges_words() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, running());
    fleet
        .vcs()
        .now_auto_merge(Err("Auto merge is not allowed".into()));

    let refused = fleet.enable_auto_merge(id, 12).await.unwrap_err();

    assert_eq!(code(&refused), "fleet.auto_merge_refused");
    assert!(refused.error().message.contains("not allowed"));
}

// -------------------------------------------------------------- review

fn review(pull_request: &str, session: Option<&str>) -> ReviewPullRequest {
    ReviewPullRequest {
        pull_request: pull_request.into(),
        session_id: session.map(SessionId::carried),
    }
}

#[tokio::test]
async fn a_review_is_a_code_review_job_naming_the_pull_request_at_the_gate() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());

    let made = fleet
        .review_pull_request(id, review("12", None), api::Redirector::Person)
        .await
        .expect("dispatched");

    assert_eq!(made.address, ADDRESS, "a number is read for its address");
    assert_eq!(made.session_id, None);
    let job = fleet.load(&made.job_id.to_domain()).await.expect("a Job");
    assert_eq!(job.workflow_id().as_str(), "code_review");
    assert_eq!(job.origin().as_wire(), "session_dispatched");
    assert_eq!(job.status(), core_model::JobStatus::AwaitingApproval);
    let subject = job.subject().expect("a subject");
    assert_eq!(
        (subject.kind.as_str(), subject.reference.as_str()),
        ("pull_request", ADDRESS)
    );
}

#[tokio::test]
async fn an_address_is_taken_as_it_is_without_asking_the_forge() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    let address = "https://forge.invalid/someone/else/pull/7";

    let made = fleet
        .review_pull_request(id, review(address, None), api::Redirector::Person)
        .await
        .expect("dispatched");

    assert_eq!(made.address, address);
    assert_eq!(
        fleet.vcs().times_asked_what_is_under_review(),
        0,
        "nothing was read"
    );
}

#[tokio::test]
async fn a_review_from_a_session_is_recorded_on_the_ledger_as_dispatched_from_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());

    let made = fleet
        .review_pull_request(
            id.clone(),
            review("12", Some("s1")),
            api::Redirector::Person,
        )
        .await
        .expect("dispatched");

    assert_eq!(made.session_id.as_ref().map(SessionId::as_str), Some("s1"));
    let row = the_row(&fleet, "s1", "job").await;
    assert_eq!(row.target, made.job_id.as_str());
    assert_eq!(
        row.detail.get("origin").map(String::as_str),
        Some("dispatched from Session s1")
    );
    let owners = fleet
        .who_owns("job".into(), made.job_id.as_str().into(), Some(id))
        .await
        .unwrap();
    assert_eq!(owners.holders.len(), 1);
    assert_eq!(owners.holders[0].holder.id, "s1");
}

#[tokio::test]
async fn a_review_asked_by_a_session_nobody_heard_of_makes_no_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);

    let refused = fleet
        .review_pull_request(
            id,
            review("https://x.invalid/a/b/pull/7", Some("ghost")),
            api::Redirector::Person,
        )
        .await
        .unwrap_err();

    assert_eq!(code(&refused), "fleet.session_unknown");
    assert_eq!(refused.status(), 422);
    assert!(fleet.every_job().await.unwrap().0.jobs.is_empty());
}

#[tokio::test]
async fn something_that_names_no_pull_request_is_refused_and_so_is_a_number_the_forge_cannot_read()
{
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);

    for said in ["", "#12", "the one about sessions"] {
        let refused = fleet
            .review_pull_request(id.clone(), review(said, None), api::Redirector::Person)
            .await
            .unwrap_err();
        assert_eq!(code(&refused), "fleet.pull_request_unnamed", "{said:?}");
    }
    let refused = fleet
        .review_pull_request(id, review("12", None), api::Redirector::Person)
        .await
        .unwrap_err();
    assert_eq!(code(&refused), "fleet.pull_request_unreadable");
    assert!(fleet.every_job().await.unwrap().0.jobs.is_empty());
}
