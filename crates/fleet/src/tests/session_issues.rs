//! The issues a Session works on, kept as `issue` rows on the ledger: the ones the person names,
//! and the ones its pull requests close, as the forge names them on the `pr` row's `closes`. A
//! scripted forge and tracker stand in for `gh`.

use std::sync::Arc;

use adapter_traits::{FromOutside, PullRequestStanding, ReadIssue};
use api::{HostedSessions, PullRequests, Sessions};
use testkit::{FakeTracker, FakeTrackers, FakeWorkProduct};
use ipc::{
    AttachmentReport, AttachmentState, SendSessionMessage, SessionFact, SessionId, SessionReport, SessionTag, TagKind,
};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::pull_requesting::{
    a_fleet, a_session_holding_pull_request_12, facts, forge_ran, manifest, passed, the_row,
};
use crate::tests::session_host::{rig_fitted, Rig};
use crate::tests::tmp::TempDir;

/// The served repository made a checkout whose origin is `o/r` on a forge no test reaches.
fn with_origin(root: &str) {
    for args in [&["init", "-q"][..], &["remote", "add", "origin", "https://forge.invalid/o/r.git"]] {
        let ran = std::process::Command::new("git").args(args).current_dir(root).output().expect("git");
        assert!(ran.status.success(), "{args:?}");
    }
}

fn a_rig() -> Rig {
    let rig = rig_fitted(|_| {});
    with_origin(&rig.root);
    rig
}

/// What the mod tells of an issue the agent filed: the address `gh issue create` printed.
async fn filed(fleet: &impl Sessions, id: &str, address: &str) {
    let number = address.rsplit('/').next().expect("a number");
    fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport {
                    kind: "issue".into(),
                    target: number.into(),
                    detail: [("address".to_string(), address.to_string())].into(),
                },
            },
        })
        .await
        .expect("attached");
}

/// Every `issue` row the Session holds, by target, ascending.
async fn issues<D: Sessions>(fleet: &D, id: &str) -> Vec<ipc::Attachment> {
    let listed = fleet.list_sessions(None, Some(id.into()), None).await.expect("listed");
    let session = listed.sessions.into_iter().find(|one| one.id.as_str() == id).expect("the session");
    let mut rows: Vec<ipc::Attachment> = session.attachments.into_iter().filter(|one| one.kind == "issue").collect();
    rows.sort_by_key(|one| one.target.parse::<u64>().unwrap_or(0));
    rows
}

fn targets(rows: &[ipc::Attachment]) -> Vec<&str> {
    rows.iter().map(|one| one.target.as_str()).collect()
}

async fn said(rig: &Rig, id: &SessionId, text: &str, mentions: Vec<SessionTag>) {
    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage {
            session_id: id.clone(),
            text: text.into(),
            attachments: Vec::new(),
            mentions,
        })
        .await
        .expect("taken");
}

/// A tag is the person naming an issue, and so is a link into the Session's own repository. A
/// link into another repository is not this Session's issue, and a bare `#7` is prose.
#[tokio::test]
async fn an_issue_the_person_tags_or_links_is_held_once_and_a_bare_number_is_not() {
    let rig = a_rig();
    let id = rig.start().await;
    let tag = SessionTag {
        kind: TagKind::Issue,
        id: "8".into(),
        title: "#8 Walk it".into(),
        url: Some("https://forge.invalid/o/r/issues/8".into()),
    };
    let words = "Fix https://forge.invalid/o/r/issues/5 and https://forge.invalid/other/r/issues/6, not #7";

    said(&rig, &id, words, vec![tag.clone()]).await;
    said(&rig, &id, words, vec![tag]).await;

    let rows = issues(&*rig.fleet, id.as_str()).await;
    assert_eq!(targets(&rows), vec!["5", "8"]);
    assert!(rows.iter().all(|one| one.state == AttachmentState::Standing));
    assert_eq!(
        rows[1].detail.get("address").map(String::as_str),
        Some("https://forge.invalid/o/r/issues/8")
    );
}

/// A tracker that is not the forge names its issues by key, and a tag of one is held as one.
#[tokio::test]
async fn an_issue_the_person_tags_by_a_trackers_key_is_held_by_that_key() {
    let rig = a_rig();
    let id = rig.start().await;
    let tag = SessionTag {
        kind: TagKind::Issue,
        id: "ABC-12".into(),
        title: "ABC-12 Walk it".into(),
        url: Some("https://acme.invalid/browse/ABC-12".into()),
    };

    said(&rig, &id, "Work this", vec![tag]).await;

    let rows = issues(&*rig.fleet, id.as_str()).await;
    assert_eq!(targets(&rows), vec!["ABC-12"]);
    assert_eq!(rows[0].state, AttachmentState::Standing);
    assert_eq!(
        rows[0].detail.get("address").map(String::as_str),
        Some("https://acme.invalid/browse/ABC-12")
    );
}

/// The request a Session is dispatched with is the person's first message, whether or not it is
/// ever sent as one: a Session that waits on another holds it as its brief.
#[tokio::test]
async fn the_request_a_session_is_dispatched_with_names_its_issues() {
    let rig = a_rig();

    let record = rig.start_requesting("https://forge.invalid/o/r/issues/2214", |_| {}).await;

    assert_eq!(targets(&issues(&*rig.fleet, record.id.as_str()).await), vec!["2214"]);
}

/// The mod tells every issue `gh issue create` printed; one filed in another repository is not
/// this Session's to work.
#[tokio::test]
async fn an_issue_the_agent_files_is_held_only_where_it_is_in_the_sessions_own_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (_, root) = manifest(&fleet);
    with_origin(&root);
    a_session_holding_pull_request_12(&fleet, "s1").await;

    filed(&fleet, "s1", "https://forge.invalid/o/r/issues/2214").await;
    filed(&fleet, "s1", "https://forge.invalid/other/r/issues/9").await;

    let rows = issues(&fleet, "s1").await;
    assert_eq!(targets(&rows), vec!["2214"]);
    assert_eq!(
        rows[0].detail.get("address").map(String::as_str),
        Some("https://forge.invalid/o/r/issues/2214")
    );
}

#[tokio::test]
async fn a_pull_requests_row_says_which_issues_it_closes_and_says_nothing_where_none() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    forge_ran(&fleet, passed());
    fleet.vcs().now_pull_request(Some(adapter_traits::PullRequestFacts {
        closes: vec![7, 31],
        ..facts(PullRequestStanding::Open)
    }));

    let read = fleet.get_pull_request(id.clone(), 12).await.expect("read");

    assert_eq!(read.closes, vec![7, 31]);
    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.detail.get("closes").map(String::as_str), Some("7,31"));

    fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    fleet.get_pull_request(id, 12).await.expect("read");

    assert_eq!(the_row(&fleet, "s1", "pr").await.detail.get("closes"), None);
}

/// What a Session's pull request closes is what it works on, and a merge that closes it is the
/// same write: the issues stand on the Session that held the pull request.
#[tokio::test]
async fn the_issues_a_sessions_pull_request_closes_are_held_by_the_session() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    forge_ran(&fleet, passed());
    fleet.vcs().now_pull_request(Some(adapter_traits::PullRequestFacts {
        closes: vec![7, 31],
        ..facts(PullRequestStanding::Merged)
    }));

    fleet.get_pull_request(id, 12).await.expect("read");

    let rows = issues(&fleet, "s1").await;
    assert_eq!(targets(&rows), vec!["7", "31"]);
    assert!(rows.iter().all(|one| one.state == AttachmentState::Standing));
}

/// The pull watch reads each issue a Session holds once a pass, as it reads each pull request: a
/// title and a state from the forge, over what the mod or a tag said. A closed issue stands.
#[tokio::test]
async fn the_pull_watch_keeps_each_issue_row_as_the_forge_has_it_and_a_closed_one_stands() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(5, "Walk the queue")]));
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&[]));
    fitted.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::clone(&tracker)));
    let fleet = Fleet::assembled(fitted);
    let (id, root) = manifest(&fleet);
    with_origin(&root);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    filed(&fleet, "s1", "https://forge.invalid/o/r/issues/5").await;

    fleet.pull_ledger_kept_current(&id, &root).await;

    let detail = |rows: &[ipc::Attachment], key: &str| rows[0].detail.get(key).cloned();
    let rows = issues(&fleet, "s1").await;
    assert_eq!(detail(&rows, "title").as_deref(), Some("Walk the queue"));
    assert_eq!(detail(&rows, "state").as_deref(), Some("open"));
    assert_eq!(detail(&rows, "address").as_deref(), Some("https://forge.invalid/o/r/issues/5"));

    tracker.now_lists(Vec::new());
    tracker.knows_closed(5, "Walk the queue");
    fleet.pull_ledger_kept_current(&id, &root).await;
    // Told again, it keeps what the forge said.
    filed(&fleet, "s1", "https://forge.invalid/o/r/issues/5").await;

    let rows = issues(&fleet, "s1").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, AttachmentState::Standing);
    assert_eq!(detail(&rows, "state").as_deref(), Some("closed"));
    assert_eq!(detail(&rows, "title").as_deref(), Some("Walk the queue"));
}

/// An issue held by a tracker's key is read through the Manifest's tracker as a number is.
#[tokio::test]
async fn the_pull_watch_keeps_an_issue_held_by_a_trackers_key_as_the_tracker_has_it() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.knows(ReadIssue::bare(
        "ABC-12".into(),
        FromOutside::verbatim("Walk the queue"),
        FromOutside::verbatim("https://acme.invalid/browse/ABC-12"),
        false,
    ));
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&[]));
    fitted.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::clone(&tracker)));
    let fleet = Fleet::assembled(fitted);
    let (id, root) = manifest(&fleet);
    with_origin(&root);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    let at = fleet.now().as_str().to_string();
    fleet
        .store()
        .lock()
        .await
        .attach(
            &store::KeptAttachment {
                holder: store::Holder::session("s1"),
                kind: "issue".into(),
                manifest_id: id.as_str().to_string(),
                target: "ABC-12".into(),
                state: store::AttachmentState::Standing,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");

    fleet.pull_ledger_kept_current(&id, &root).await;

    let rows = issues(&fleet, "s1").await;
    let detail = |key: &str| rows[0].detail.get(key).cloned();
    assert_eq!(detail("title").as_deref(), Some("Walk the queue"));
    assert_eq!(detail("state").as_deref(), Some("open"));
    assert_eq!(detail("address").as_deref(), Some("https://acme.invalid/browse/ABC-12"));
}

fn plan_naming(id: &SessionId, issues: &[Option<&str>]) -> ipc::RecordPlan {
    ipc::RecordPlan {
        groups: issues
            .iter()
            .enumerate()
            .map(|(at, issue)| ipc::PlanGroupInput {
                id: None,
                title: format!("Group {at}"),
                agent: "rust-engineer".into(),
                paths: Vec::new(),
                issue: issue.map(str::to_string),
                tasks: vec![ipc::PlanTaskInput { id: None, title: "Do it".into() }],
            })
            .collect(),
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    }
}

/// A plan of "work these tickets" shows them before any pull request exists. Recording it again
/// takes nothing a second time, a group that names none adds none, and taking the issue off a
/// group later does not give the row back.
#[tokio::test]
async fn an_issue_a_plan_group_names_is_held_once_and_a_group_naming_none_adds_none() {
    let rig = a_rig();
    let id = rig.start().await;

    rig.fleet.record_plan(None, plan_naming(&id, &[Some("31"), None])).await.expect("recorded");
    rig.fleet.record_plan(None, plan_naming(&id, &[Some("31"), None])).await.expect("recorded again");
    assert_eq!(targets(&issues(&*rig.fleet, id.as_str()).await), vec!["31"]);

    rig.fleet.record_plan(None, plan_naming(&id, &[None, None])).await.expect("recorded without it");
    assert_eq!(targets(&issues(&*rig.fleet, id.as_str()).await), vec!["31"]);

    let other = rig.start().await;
    rig.fleet.record_plan(None, plan_naming(&other, &[None])).await.expect("recorded");
    assert!(issues(&*rig.fleet, other.as_str()).await.is_empty());
}

/// A plan group names a tracker's key as it names a number, and the plan reads the key back as
/// it was recorded.
#[tokio::test]
async fn a_plan_group_naming_a_trackers_key_holds_it_and_the_plan_keeps_the_key() {
    let rig = a_rig();
    let id = rig.start().await;

    rig.fleet.record_plan(None, plan_naming(&id, &[Some("ABC-12"), Some("#7")])).await.expect("recorded");

    let mut held = targets(&issues(&*rig.fleet, id.as_str()).await).into_iter().map(str::to_string).collect::<Vec<_>>();
    held.sort();
    assert_eq!(held, vec!["7", "ABC-12"]);
    let listed = rig.fleet.list_sessions(None, Some(id.as_str().into()), None).await.expect("listed");
    let plan = listed.sessions.into_iter().find(|one| one.id == id).and_then(|one| one.plan).expect("a plan");
    let named: Vec<Option<&str>> = plan.groups.iter().map(|group| group.issue.as_deref()).collect();
    assert_eq!(named, vec![Some("ABC-12"), Some("7")]);
}

/// A group naming what is neither a number nor a key is refused by its place, and nothing is held.
#[tokio::test]
async fn a_plan_group_naming_neither_a_number_nor_a_key_is_refused() {
    let rig = a_rig();
    let id = rig.start().await;

    let refused = rig.fleet.record_plan(None, plan_naming(&id, &[None, Some("abc-1")])).await;

    let said = format!("{:?}", refused.expect_err("refused"));
    assert!(said.contains("groups[1]") && said.contains("abc-1"), "{said}");
    assert!(issues(&*rig.fleet, id.as_str()).await.is_empty());
}

async fn claims(rig: &Rig, id: &SessionId, issue: &str, pulls: &[u64]) -> Result<ipc::IssueClaimed, api::Refusal> {
    rig.fleet
        .claim_issue(
            None,
            ipc::ClaimIssue { issue: issue.into(), session_id: Some(id.clone()), pull_requests: pulls.to_vec() },
        )
        .await
}

/// An agent or the person attaches an issue to a Session by the forge's number or a tracker's key,
/// and nothing that is neither is held.
#[tokio::test]
async fn an_issue_a_session_claims_is_held_by_a_key_or_a_number_and_anything_else_is_refused() {
    let rig = a_rig();
    let id = rig.start().await;

    assert_eq!(claims(&rig, &id, "ABC-12", &[]).await.expect("claimed").issue, "ABC-12");
    assert_eq!(claims(&rig, &id, "7", &[]).await.expect("claimed").issue, "7");
    assert_eq!(claims(&rig, &id, "#7", &[]).await.expect("claimed again").holder_id, id.as_str());
    for written in ["hello", "abc-1", ""] {
        match claims(&rig, &id, written, &[]).await {
            Err(api::Refusal::Unacceptable(why)) => assert_eq!(why.code, "fleet.issue_not_claimable", "{written:?}"),
            other => panic!("{written:?} is not a 422: {other:?}"),
        }
    }

    let mut held: Vec<String> = targets(&issues(&*rig.fleet, id.as_str()).await).into_iter().map(str::to_string).collect();
    held.sort();
    assert_eq!(held, vec!["7", "ABC-12"]);
}

/// The pull requests a claim names are the ones that work the issue: the row says them ascending,
/// and a second claim adds to them.
#[tokio::test]
async fn a_claimed_issue_names_the_pull_requests_that_work_it_and_a_second_claim_adds_to_them() {
    let rig = a_rig();
    let id = rig.start().await;
    let named = |rows: Vec<ipc::Attachment>| rows[0].detail.get("pull_requests").cloned();

    let claimed = claims(&rig, &id, "ABC-12", &[31, 7]).await.expect("claimed");
    assert_eq!(claimed.pull_requests, vec![7, 31]);
    assert_eq!(named(issues(&*rig.fleet, id.as_str()).await).as_deref(), Some("7,31"));

    let again = claims(&rig, &id, "ABC-12", &[12, 7]).await.expect("claimed again");
    assert_eq!(again.pull_requests, vec![7, 12, 31]);
    assert_eq!(named(issues(&*rig.fleet, id.as_str()).await).as_deref(), Some("7,12,31"));
}
