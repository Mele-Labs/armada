//! A pull request's page and the writes its comment box makes, through a real
//! Fleet and a scripted forge. Since 23.83.
//!
//! **No forge is called.** `FakeVcs` stands in for `gh`, and a test that says
//! nothing was written says so by counting what the fake was asked to write.

use adapter_traits::{
    CheckRow, CheckStanding, CommitLine, FileChange, FilePatch, Mergeability, Omitted, PageFile,
    PullRequestPageFacts, PullRequestStanding, PullRequestWrite, ReviewSaid, ReviewStanding, Said,
};
use api::PullRequestPanel;
use ipc::{
    PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestMergeable,
    PullRequestReviewState, PullRequestVerdict, PullRequestWriteKind, SessionId, WritePullRequest,
};

use super::pull_requesting::{
    a_fleet, a_session_holding_pull_request_12, code, facts, forge_ran, manifest, passed, the_row,
};

pub(super) fn a_page() -> PullRequestPageFacts {
    PullRequestPageFacts {
        base: "main".into(),
        author: Some("NickMele".into()),
        created_at: Some("2026-10-13T15:13:44Z".into()),
        additions: 12,
        deletions: 3,
        mergeable: Mergeability::Conflicting,
        review: ReviewStanding::ChangesRequested,
        approvals: 1,
        conversation: vec![
            Said::Comment {
                author: "ada".into(),
                at: "2026-10-13T15:30:00Z".into(),
                body: "why?".into(),
            },
            Said::Review {
                author: "grace".into(),
                at: "2026-10-13T16:00:00Z".into(),
                verdict: ReviewSaid::ChangesRequested,
                body: String::new(),
            },
        ],
        checks: vec![CheckRow {
            name: "plan".into(),
            standing: CheckStanding::Running,
            seconds: None,
            address: Some("https://forge.invalid/run/1".into()),
        }],
        files: vec![PageFile {
            path: "a.rs".into(),
            additions: 12,
            deletions: 3,
            change: FileChange::Renamed,
        }],
        commits: vec![CommitLine {
            sha: "b42a108".into(),
            headline: "Add a thing".into(),
            author: None,
            at: None,
        }],
        omitted: Omitted::default(),
    }
}

fn writing(kind: PullRequestWriteKind, body: Option<&str>) -> WritePullRequest {
    WritePullRequest {
        pull_request: 12,
        kind,
        body: body.map(str::to_string),
        session_id: None,
    }
}

#[tokio::test]
async fn the_page_is_the_forges_in_the_wires_words() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let page = fleet.get_pull_request_detail(id, 12).await.expect("read");
    assert_eq!(page.mergeable, PullRequestMergeable::Conflicting);
    assert_eq!(page.review, PullRequestReviewState::ChangesRequested);
    assert_eq!(page.checks[0].state, PullRequestCheckState::Running);
    assert_eq!(page.files[0].status, PullRequestFileStatus::Renamed);
    assert_eq!(page.commits[0].sha, "b42a108");
    assert!(matches!(
        &page.conversation[1],
        PullRequestEntry::Review { verdict: PullRequestVerdict::ChangesRequested, body, .. }
            if body.is_empty()
    ));
}

#[tokio::test]
async fn an_opened_file_is_the_forges_patch_for_it() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request_file_patch(Some(FilePatch::Patch("@@ -1 +1 @@\n-a\n+b".into())));
    let file = fleet
        .get_pull_request_file(id, 12, "a.rs".into())
        .await
        .expect("read");
    assert_eq!(file.patch.as_deref(), Some("@@ -1 +1 @@\n-a\n+b"));
    assert_eq!(file.said, None);
    assert_eq!(file.path, "a.rs");
}

#[tokio::test]
async fn a_file_with_no_patch_says_so() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request_file_patch(Some(FilePatch::NoPatch));
    let file = fleet
        .get_pull_request_file(id, 12, "logo.png".into())
        .await
        .expect("an answer");
    assert_eq!(file.patch, None);
    assert!(file.said.expect("a sentence").starts_with("The forge sent no patch"));
}

#[tokio::test]
async fn a_path_the_pull_request_does_not_list_is_refused() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .now_pull_request_file_patch(Some(FilePatch::NoSuchFile));
    let refused = fleet
        .get_pull_request_file(id, 12, "z.rs".into())
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 422);
    assert_eq!(code(&refused), "fleet.pull_request_file_unknown");
}

#[tokio::test]
async fn a_forge_that_will_not_say_what_a_file_changed_is_a_fault() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    let refused = fleet
        .get_pull_request_file(id, 12, "a.rs".into())
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.pull_request_unreadable");
}

#[tokio::test]
async fn a_forge_that_will_not_say_is_a_fault() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    let refused = fleet.get_pull_request_detail(id, 12).await.unwrap_err();
    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.pull_request_unreadable");
}

#[tokio::test]
async fn a_blank_comment_or_request_is_refused_before_the_forge() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    for write in [
        writing(PullRequestWriteKind::Comment, None),
        writing(PullRequestWriteKind::Comment, Some("  \n")),
        writing(PullRequestWriteKind::RequestChanges, Some("")),
    ] {
        let refused = fleet.write_pull_request(id.clone(), write).await.unwrap_err();
        assert_eq!(refused.status(), 422);
        assert_eq!(code(&refused), "fleet.pull_request_write_blank");
    }
    assert!(fleet.vcs().pull_request_writes().is_empty());
}

#[tokio::test]
async fn a_session_the_ledger_does_not_know_is_refused_with_nothing_written() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let mut write = writing(PullRequestWriteKind::Approve, None);
    write.session_id = Some(SessionId::carried("nobody"));
    let refused = fleet.write_pull_request(id, write).await.unwrap_err();
    assert_eq!(refused.status(), 422);
    assert_eq!(code(&refused), "fleet.session_unknown");
    assert!(fleet.vcs().pull_request_writes().is_empty());
}

#[tokio::test]
async fn a_write_reaches_the_forge_and_answers_the_page_read_again() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let page = fleet
        .write_pull_request(id, writing(PullRequestWriteKind::Comment, Some("looks right")))
        .await
        .expect("written");
    assert_eq!(page.number, 12);
    assert_eq!(
        fleet.vcs().pull_request_writes(),
        [("12".to_string(), PullRequestWrite::Comment { body: "looks right".into() })]
    );
}

#[tokio::test]
async fn the_forges_refusal_is_a_conflict_in_its_own_words() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet
        .vcs()
        .write_on_pull_request_refuses("cannot approve your own pull request");
    let refused = fleet
        .write_pull_request(id, writing(PullRequestWriteKind::Approve, None))
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 409);
    assert_eq!(code(&refused), "fleet.pull_request_write_refused");
    assert!(refused.error().message.contains("your own pull request"));
}

#[tokio::test]
async fn a_write_lets_the_ledger_follow_the_forge() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet
        .vcs()
        .now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&fleet, passed());
    let mut write = writing(PullRequestWriteKind::Approve, Some("ship it"));
    write.session_id = Some(SessionId::carried("s1"));
    fleet.write_pull_request(id, write).await.expect("written");
    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.detail.get("title").map(String::as_str), Some("Take a draft out"));
    assert_eq!(row.detail.get("checks").map(String::as_str), Some("passed"));
}
