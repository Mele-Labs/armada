//! A pull request by repository and number, through a real Fleet and a scripted
//! forge: what is read, what each act refuses before it writes, what it asks
//! the forge for, and what the session ledger holds afterwards. Since 23.48.
//!
//! **No forge is called.** `FakeVcs` stands in for `gh`, and a test that says a
//! merge was refused says so by counting what the fake was asked to write.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{
    NotMerged, PullRequestFacts, PullRequestStanding, UnderReview, WhatTheForgeRan,
};
use api::{Next, PullRequests, Refusal, Sessions, Subscription};
use ipc::{
    AttachmentReport, AttachmentState, ForgeChecks, ManifestId, PullRequestStanding as Wire, SessionFact, SessionId, SessionRecord, SessionReport,
};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Merging};

use crate::daemon::Fleet;
use crate::tests::daemon::{fittings, one, workflow_named};
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) const ADDRESS: &str = "https://forge.invalid/armada/pull/12";

pub(super) fn a_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().workflows = one(workflow_named("code_review"));
    // A base, since the merge queue is the base branch's.
    fittings.starting().manifest = config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\n",
    )
    .expect("parses");
    Fleet::assembled(fittings)
}

pub(super) fn facts(standing: PullRequestStanding) -> PullRequestFacts {
    PullRequestFacts {
        standing,
        branch: "fleet/pr-acts".into(),
        base: "main".into(),
        auto_merge: false,
        title: "Take a draft out".into(),
        url: ADDRESS.into(),
        body: None,
        closes: Vec::new(),
    }
}

pub(super) fn forge_ran(fleet: &Fixture, ran: WhatTheForgeRan) {
    fleet.vcs().now_under_review(UnderReview {
        checks: ran,
        ..UnderReview::unreadable()
    });
}

pub(super) fn passed() -> WhatTheForgeRan {
    WhatTheForgeRan::AllPassed { checks: 2 }
}

fn running() -> WhatTheForgeRan {
    WhatTheForgeRan::StillWaiting {
        finished: 1,
        checks: 2,
    }
}

pub(super) fn manifest(fleet: &Fixture) -> (ManifestId, String) {
    let one = fleet.repositories().first().expect("a served repository");
    (
        ManifestId::carried(one.manifest().id().as_str()),
        one.root().to_string(),
    )
}

pub(super) fn code(refusal: &Refusal) -> &str {
    &refusal.error().code
}

pub(super) async fn a_session_holding_pull_request_12(fleet: &Fixture, id: &str) -> SessionRecord {
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

/// Wait for the `session.changed` the ledger publishes when it follows an act's pull request, which
/// is after the act's reply.
async fn ledger_followed(watching: &mut Subscription, id: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let ipc::Event::SessionChanged(record) = delivered.event {
                        if record.id.as_str() == id {
                            return;
                        }
                    }
                }
                other => panic!("the stream ended: {other:?}"),
            }
        }
    })
    .await
    .expect("the ledger followed the act");
}

pub(super) async fn the_row(fleet: &Fixture, id: &str, kind: &str) -> ipc::Attachment {
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
            weight: adapter_traits::FailureWeight::Required,
        },
    );
    assert_eq!(
        fleet.get_pull_request(id.clone(), 12).await.unwrap().checks,
        ForgeChecks::Failed {
            failing: vec!["ci".into()],
            weight: ipc::FailureWeight::Required,
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
            weight: adapter_traits::FailureWeight::Required,
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
    assert_eq!(said("base"), Some("main"));
    assert_eq!(said("address"), Some(ADDRESS));
    assert_eq!(said("title"), Some("Take a draft out"));
}

/// Stacked pull requests are drawn from the base, so a blank one is left off the row.
#[tokio::test]
async fn a_pull_request_with_no_base_leaves_it_out_of_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet.vcs().now_pull_request(Some(PullRequestFacts {
        base: String::new(),
        ..facts(PullRequestStanding::Open)
    }));
    forge_ran(&fleet, passed());

    fleet.get_pull_request(id, 12).await.expect("read");

    let row = the_row(&fleet, "s1", "pr").await;
    assert!(!row.detail.contains_key("base"), "{:?}", row.detail);
}

#[tokio::test]
async fn a_failure_outside_the_gate_is_a_lighter_word_on_the_row_and_still_a_failure_to_a_merge() {
    for (weight, said) in [
        (adapter_traits::FailureWeight::WhileRunning, "pending"),
        (adapter_traits::FailureWeight::AfterRequired, "passed"),
    ] {
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
                failed: vec![adapter_traits::FromOutside::verbatim("app_smoke_shard (2)")],
                checks: 4,
                weight,
            },
        );

        let read = fleet.get_pull_request(id, 12).await.expect("read");

        assert!(matches!(read.checks, ForgeChecks::Failed { .. }), "a merge still reads it as failed");
        let row = the_row(&fleet, "s1", "pr").await;
        assert_eq!(row.detail.get("checks").map(String::as_str), Some(said));
        assert_eq!(row.detail.get("failing").map(String::as_str), Some("app_smoke_shard (2)"));
    }
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
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Draft)));
    forge_ran(&fleet, running());
    let mut watching = fleet.events().subscribe();

    let now = Arc::clone(&fleet).ready_pull_request(id, 12).await.expect("readied");

    assert_eq!(now.state, Wire::Open);
    assert_eq!(fleet.vcs().times_asked_to_ready(), 1);
    ledger_followed(&mut watching, "s1").await;
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("state")
            .map(String::as_str),
        Some("open")
    );
}

/// **The reply does not wait on the read after the write.** The forge is held shut for that read,
/// and the press has answered; the ledger follows when the read is let through.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_reply_comes_back_while_the_forge_is_held_on_the_read_after_the_write() {
    for act in ["ready", "merge", "auto_merge"] {
        let home = TempDir::new();
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        a_session_holding_pull_request_12(&fleet, "s1").await;
        let (standing, ran) = match act {
            "ready" => (PullRequestStanding::Draft, running()),
            "merge" => (PullRequestStanding::Open, passed()),
            _ => (PullRequestStanding::Open, running()),
        };
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, ran);
        let hold = fleet.vcs().hold_reads_after_a_write();
        let mut watching = fleet.events().subscribe();

        let pressed = tokio::spawn({
            let fleet = Arc::clone(&fleet);
            async move {
                match act {
                    "ready" => fleet.ready_pull_request(id, 12).await,
                    "merge" => fleet.merge_pull_request_by_number(id, 12).await,
                    _ => fleet.enable_auto_merge(id, 12).await,
                }
            }
        });
        let now = tokio::time::timeout(Duration::from_secs(3), pressed)
            .await
            .unwrap_or_else(|_| panic!("{act}: the reply waited on the forge's read"))
            .expect("joined")
            .expect("pressed");

        match act {
            "ready" => assert_eq!(now.state, Wire::Open),
            "merge" => assert_eq!(now.state, Wire::Merged),
            _ => assert!(now.auto_merge),
        }
        assert!(the_row(&fleet, "s1", "pr").await.detail.is_empty(), "{act}: the ledger waits on the read");
        hold.open();
        ledger_followed(&mut watching, "s1").await;
        assert!(!the_row(&fleet, "s1", "pr").await.detail.is_empty(), "{act}");
    }
}

#[tokio::test]
async fn readying_what_is_not_a_draft_is_refused_without_a_write() {
    for (standing, expected) in [
        (PullRequestStanding::Open, "fleet.pull_request_not_a_draft"),
        (PullRequestStanding::Merged, "fleet.merge_not_open"),
        (PullRequestStanding::Closed, "fleet.merge_not_open"),
    ] {
        let home = TempDir::new();
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, passed());

        let refused = Arc::clone(&fleet).ready_pull_request(id, 12).await.unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(refused.status(), 409);
        assert_eq!(fleet.vcs().times_asked_to_ready(), 0);
    }
}

#[tokio::test]
async fn a_forge_that_will_not_ready_it_says_so_in_its_own_words() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Draft)));
    forge_ran(&fleet, running());
    fleet.vcs().ready_refuses("GraphQL: not permitted");

    let refused = Arc::clone(&fleet).ready_pull_request(id, 12).await.unwrap_err();

    assert_eq!(code(&refused), "fleet.ready_refused");
    assert!(refused.error().message.contains("not permitted"));
}

// --------------------------------------------------------------- merge

#[tokio::test]
async fn a_pull_request_whose_checks_passed_is_merged_and_the_row_is_spent() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());
    let mut watching = fleet.events().subscribe();

    let now = Arc::clone(&fleet)
        .merge_pull_request_by_number(id, 12)
        .await
        .expect("merged");

    assert_eq!(now.state, Wire::Merged);
    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    ledger_followed(&mut watching, "s1").await;
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
            weight: adapter_traits::FailureWeight::Required,
        },
    ] {
        let home = TempDir::new();
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        fleet
            .vcs()
            .now_pull_request(Some(facts(PullRequestStanding::Open)));
        forge_ran(&fleet, ran.clone());

        let refused = Arc::clone(&fleet)
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
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, passed());

        let refused = Arc::clone(&fleet)
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
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Merged)));
    forge_ran(&fleet, passed());

    let now = Arc::clone(&fleet).merge_pull_request_by_number(id, 12).await.unwrap();

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
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        fleet
            .vcs()
            .now_pull_request(Some(facts(PullRequestStanding::Open)));
        forge_ran(&fleet, passed());
        fleet.vcs().merging(Merging::Refuses(why));

        let refused = Arc::clone(&fleet)
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
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, running());
    let mut watching = fleet.events().subscribe();

    let now = Arc::clone(&fleet).enable_auto_merge(id.clone(), 12).await.expect("on");

    assert!(now.auto_merge);
    assert_eq!(now.state, Wire::Open, "it merges nothing itself");
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
    ledger_followed(&mut watching, "s1").await;
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("auto_merge")
            .map(String::as_str),
        Some("true")
    );

    // Asked twice, the second answers as it stands.
    Arc::clone(&fleet).enable_auto_merge(id, 12).await.expect("as it stands");
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn auto_merge_on_checks_that_passed_already_queues_it_instead_of_refusing() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());
    let mut watching = fleet.events().subscribe();

    let now = Arc::clone(&fleet).enable_auto_merge(id, 12).await.expect("queued");

    assert!(now.auto_merge);
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
    ledger_followed(&mut watching, "s1").await;
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("auto_merge")
            .map(String::as_str),
        Some("true")
    );
}

#[tokio::test]
async fn a_pull_request_already_in_the_merge_queue_is_answered_as_queued_and_not_asked_again() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());
    fleet.vcs().main_ci.queue_is(Some(vec![adapter_traits::QueueEntry {
        number: 12,
        position: 1,
        state: adapter_traits::QueueState::Queued,
        head: None,
    }]));

    let now = Arc::clone(&fleet).enable_auto_merge(id, 12).await.expect("as it stands");

    assert!(now.queued);
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 0);
    assert_eq!(
        the_row(&fleet, "s1", "pr")
            .await
            .detail
            .get("queued")
            .map(String::as_str),
        Some("true")
    );
}

#[tokio::test]
async fn the_pull_reading_settles_a_terminal_sessions_merged_pull_request_nobody_opened() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, root) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Merged)));
    forge_ran(&fleet, passed());

    fleet.pull_ledger_kept_current(&id, &root).await;

    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.state, AttachmentState::Spent);
    assert_eq!(row.detail.get("state").map(String::as_str), Some("merged"));
}

#[tokio::test]
async fn a_row_a_session_took_again_after_the_merge_is_settled_by_the_next_watch() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, root) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Merged)));
    forge_ran(&fleet, passed());
    fleet.get_pull_request(id.clone(), 12).await.expect("read");
    assert_eq!(the_row(&fleet, "s1", "pr").await.state, AttachmentState::Spent);
    // The harness's own report takes it up again, as standing.
    a_session_holding_pull_request_12(&fleet, "s1").await;
    assert_eq!(
        the_row(&fleet, "s1", "pr").await.state,
        AttachmentState::Standing
    );

    fleet.pull_ledger_kept_current(&id, &root).await;

    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.state, AttachmentState::Spent);
    assert_eq!(row.detail.get("state").map(String::as_str), Some("merged"));
}

#[tokio::test]
async fn auto_merge_is_refused_once_a_check_failed_and_on_a_draft() {
    for (standing, ran, expected) in [
        (
            PullRequestStanding::Open,
            WhatTheForgeRan::SomeFailed {
                failed: vec![adapter_traits::FromOutside::verbatim("ci")],
                checks: 1,
                weight: adapter_traits::FailureWeight::Required,
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
        let fleet = Arc::new(a_fleet(&home));
        let (id, _) = manifest(&fleet);
        fleet.vcs().now_pull_request(Some(facts(standing)));
        forge_ran(&fleet, ran);

        let refused = Arc::clone(&fleet).enable_auto_merge(id, 12).await.unwrap_err();

        assert_eq!(code(&refused), expected);
        assert_eq!(refused.status(), 409);
        assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 0);
    }
}

#[tokio::test]
async fn a_repository_that_does_not_allow_auto_merge_says_so_in_the_forges_words() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, running());
    fleet
        .vcs()
        .now_auto_merge(Err("Auto merge is not allowed".into()));

    let refused = Arc::clone(&fleet).enable_auto_merge(id, 12).await.unwrap_err();

    assert_eq!(code(&refused), "fleet.auto_merge_refused");
    assert!(refused.error().message.contains("not allowed"));
}

// -------------------------------------------------------------- review

