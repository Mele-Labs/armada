//! The issues a Session works on, kept as `issue` rows on the ledger: the ones the person names,
//! and the ones its pull requests close, as the forge names them on the `pr` row's `closes`. A
//! scripted forge and tracker stand in for `gh`.

use std::sync::Arc;

use adapter_traits::PullRequestStanding;
use api::{HostedSessions, PullRequests, Sessions};
use ipc::{AttachmentState, SendSessionMessage, SessionId, SessionTag, TagKind};

use crate::tests::pull_requesting::{
    a_fleet, a_session_holding_pull_request_12, facts, forge_ran, manifest, passed, the_row,
};
use crate::tests::session_host::{rig_fitted, Rig};
use crate::tests::tmp::TempDir;

/// The served repository is a checkout whose origin is `o/r` on a forge no test reaches.
fn a_rig() -> Rig {
    let rig = rig_fitted(|_| {});
    for args in [&["init", "-q"][..], &["remote", "add", "origin", "https://forge.invalid/o/r.git"]] {
        let ran = std::process::Command::new("git").args(args).current_dir(&rig.root).output().expect("git");
        assert!(ran.status.success(), "{args:?}");
    }
    rig
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

/// The request a Session is dispatched with is the person's first message, whether or not it is
/// ever sent as one: a Session that waits on another holds it as its brief.
#[tokio::test]
async fn the_request_a_session_is_dispatched_with_names_its_issues() {
    let rig = a_rig();

    let record = rig.start_requesting("https://forge.invalid/o/r/issues/2214", |_| {}).await;

    assert_eq!(targets(&issues(&*rig.fleet, record.id.as_str()).await), vec!["2214"]);
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
