//! A pull request's page and the writes its comment box makes, through a real
//! Fleet and a scripted forge. Since 23.83.
//!
//! **No forge is called.** `FakeVcs` stands in for `gh`, and a test that says
//! nothing was written says so by counting what the fake was asked to write.

use adapter_traits::{
    CheckRow, CheckStanding, CommitLine, DiffSide as FactSide, FileChange, FilePatch, Mergeability, Omitted,
    PageFile, PatchedFile, PullRequestPageFacts, PullRequestStanding, PullRequestWrite, ReviewComment, ReviewSaid,
    ReviewStanding, ReviewThreadFacts, Said, ThreadSaid,
};
use api::PullRequestPanel;
use ipc::{
    DiffSide, ManifestId, PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestMergeable,
    PullRequestReviewState, PullRequestVerdict, PullRequestWriteKind, PullReviewComment, ResolvePullThread,
    SessionId, WritePullRequest,
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
        head: Some("3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into()),
        body: Some("Takes a draft out.".into()),
        reviewers: Vec::new(),
        threads: vec![
            ReviewThreadFacts {
                id: "PRRT_1".into(),
                path: "a.rs".into(),
                line: Some(4),
                side: FactSide::Left,
                resolved: false,
                comments: vec![ThreadSaid {
                    id: "PRRC_1".into(),
                    author: "ada".into(),
                    at: "2026-10-15T09:00:00Z".into(),
                    body: "why?".into(),
                }],
            },
            ReviewThreadFacts {
                id: "PRRT_2".into(),
                path: "b.rs".into(),
                line: None,
                side: FactSide::Right,
                resolved: true,
                comments: vec![ThreadSaid {
                    id: "PRRC_2".into(),
                    author: "ghost".into(),
                    at: "2026-10-15T09:30:00Z".into(),
                    body: "done".into(),
                }],
            },
        ],
    }
}

fn writing(kind: PullRequestWriteKind, body: Option<&str>) -> WritePullRequest {
    WritePullRequest {
        pull_request: 12,
        kind,
        body: body.map(str::to_string),
        comments: Vec::new(),
        commit: None,
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
async fn every_file_is_its_patch_or_the_sentence_in_the_forges_order() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_patches(Some(vec![
        PatchedFile { path: "z.rs".into(), patch: Some("@@ -1 +1 @@\n-a\n+b".into()) },
        PatchedFile { path: "logo.png".into(), patch: None },
    ]));
    let read = fleet
        .get_pull_request_patches(id.clone(), 12)
        .await
        .expect("read");
    let paths: Vec<&str> = read.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["z.rs", "logo.png"]);
    assert_eq!(read.files[0].patch.as_deref(), Some("@@ -1 +1 @@\n-a\n+b"));
    assert_eq!(read.files[0].said, None);
    assert_eq!((&read.files[0].manifest_id, read.files[0].number), (&id, 12));
    assert_eq!(read.files[1].patch, None);
    fleet
        .vcs()
        .now_pull_request_file_patch(Some(FilePatch::NoPatch));
    let one = fleet
        .get_pull_request_file(id, 12, "logo.png".into())
        .await
        .expect("an answer");
    assert_eq!(read.files[1].said, one.said, "the per-file route's sentence");
}

#[tokio::test]
async fn a_forge_that_will_not_say_what_changed_on_a_pull_request_is_a_fault() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    let refused = fleet.get_pull_request_patches(id, 12).await.unwrap_err();
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

fn a_line_comment() -> PullReviewComment {
    PullReviewComment::Line {
        path: "a.rs".into(),
        line: 4,
        side: DiffSide::Left,
        body: "why?".into(),
    }
}

#[tokio::test]
async fn the_page_carries_the_head_the_description_and_the_threads() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let page = fleet.get_pull_request_detail(id, 12).await.expect("read");
    assert_eq!(page.head.as_deref(), Some("3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e"));
    assert_eq!(page.body.as_deref(), Some("Takes a draft out."));
    assert_eq!(page.threads.len(), 2);
    let (open, resolved) = (&page.threads[0], &page.threads[1]);
    assert_eq!((open.line, open.side, open.resolved), (Some(4), DiffSide::Left, false));
    assert_eq!(open.comments[0].author, "ada");
    assert_eq!((resolved.line, resolved.side, resolved.resolved), (None, DiffSide::Right, true));
}

#[tokio::test]
async fn a_review_with_comments_reaches_the_forge_as_one_review_at_the_commit_read() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let reply = PullReviewComment::Reply {
        thread: "PRRT_1".into(),
        body: "because".into(),
    };
    let mut write = writing(PullRequestWriteKind::RequestChanges, Some("see the lines"));
    write.comments = vec![a_line_comment(), reply];
    write.commit = Some("c0ffee".into());
    fleet.write_pull_request(id, write).await.expect("written");
    assert_eq!(
        fleet.vcs().pull_request_writes(),
        [(
            "12".to_string(),
            PullRequestWrite::Review {
                verdict: ReviewSaid::ChangesRequested,
                body: Some("see the lines".into()),
                comments: vec![
                    ReviewComment::Line {
                        path: "a.rs".into(),
                        line: 4,
                        side: FactSide::Left,
                        body: "why?".into()
                    },
                    ReviewComment::Reply {
                        thread: "PRRT_1".into(),
                        body: "because".into()
                    },
                ],
                commit: Some("c0ffee".into()),
            }
        )],
        "one write, not a comment for each"
    );
}

#[tokio::test]
async fn a_blank_body_is_taken_only_where_comments_come_with_it() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    for kind in [PullRequestWriteKind::Comment, PullRequestWriteKind::RequestChanges] {
        let refused = fleet
            .write_pull_request(id.clone(), writing(kind, Some(" ")))
            .await
            .unwrap_err();
        assert_eq!(code(&refused), "fleet.pull_request_write_blank");
    }
    assert!(fleet.vcs().pull_request_writes().is_empty());
    for kind in [
        PullRequestWriteKind::Comment,
        PullRequestWriteKind::Approve,
        PullRequestWriteKind::RequestChanges,
    ] {
        let mut write = writing(kind, None);
        write.comments = vec![a_line_comment()];
        fleet.write_pull_request(id.clone(), write).await.expect("written");
    }
    let writes = fleet.vcs().pull_request_writes();
    assert_eq!(writes.len(), 3);
    assert!(writes.iter().all(|(_, write)| matches!(write, PullRequestWrite::Review { body: None, .. })));
}

#[tokio::test]
async fn a_review_the_forge_refuses_is_a_conflict_in_its_own_words() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet
        .vcs()
        .write_on_pull_request_refuses("Line must be part of the diff");
    let mut write = writing(PullRequestWriteKind::Comment, None);
    write.comments = vec![a_line_comment()];
    let refused = fleet.write_pull_request(id, write).await.unwrap_err();
    assert_eq!(refused.status(), 409);
    assert_eq!(code(&refused), "fleet.pull_request_write_refused");
    assert!(refused.error().message.contains("part of the diff"));
}

#[tokio::test]
async fn a_thread_is_resolved_and_opened_again_as_the_forge_holds_it() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    for resolved in [true, false] {
        let answered = fleet
            .resolve_review_thread(id.clone(), ResolvePullThread { thread: "PRRT_1".into(), resolved })
            .await
            .expect("taken");
        assert_eq!((answered.thread.as_str(), answered.resolved), ("PRRT_1", resolved));
    }
    assert_eq!(
        fleet.vcs().review_thread_resolutions(),
        [("PRRT_1".to_string(), true), ("PRRT_1".to_string(), false)]
    );
}

#[tokio::test]
async fn a_thread_the_forge_will_not_resolve_is_a_conflict_in_its_own_words() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet
        .vcs()
        .resolving_a_thread_refuses("Resource not accessible by personal access token");
    let refused = fleet
        .resolve_review_thread(id, ResolvePullThread { thread: "PRRT_1".into(), resolved: true })
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 409);
    assert_eq!(code(&refused), "fleet.review_thread_refused");
    assert!(refused.error().message.contains("not accessible"));
}

#[tokio::test]
async fn a_thread_of_a_repository_fleet_does_not_serve_is_refused_before_the_forge() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let refused = fleet
        .resolve_review_thread(
            ManifestId::carried("01NOSUCHMANIFEST"),
            ResolvePullThread { thread: "PRRT_1".into(), resolved: true },
        )
        .await
        .unwrap_err();
    assert_eq!(refused.status(), 422);
    assert!(fleet.vcs().review_thread_resolutions().is_empty());
}

#[tokio::test]
async fn a_blank_line_file_or_reply_comment_is_refused_before_the_forge() {
    let home = crate::tests::tmp::TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let blanks = [
        PullReviewComment::Line { path: "a.rs".into(), line: 4, side: DiffSide::Left, body: " ".into() },
        PullReviewComment::File { path: "a.rs".into(), body: String::new() },
        PullReviewComment::Reply { thread: "PRRT_1".into(), body: "\n".into() },
    ];
    for blank in blanks {
        let mut write = writing(PullRequestWriteKind::Approve, None);
        // One good comment beside it: a review with a blank one in it is not posted at all.
        write.comments = vec![a_line_comment(), blank];
        let refused = fleet.write_pull_request(id.clone(), write).await.unwrap_err();
        assert_eq!(code(&refused), "fleet.pull_request_write_blank");
    }
    assert!(fleet.vcs().pull_request_writes().is_empty());
}
