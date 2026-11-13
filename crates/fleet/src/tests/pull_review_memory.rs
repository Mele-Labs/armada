//! What Fleet remembers of a pull request for the person, through a real Fleet and a scripted
//! forge: the head he left it at, the files he marked viewed, where each row stands in his inbox,
//! the change since a commit, and his named Reviews layouts. `docs/concepts/pull-review.md`,
//! *Since you looked*.

use adapter_traits::{ComparedPatches, LatestReview, LatestReviewSaid, PatchedFile, ReviewList};
use api::PullRequestPanel;
use ipc::{
    MarkFileViewed, MarkPullSeen, MyReviewSaid, NamedReviewLayout, PullInbox, PullMyReview, PullRequestCompare,
    PullRequestCompareOf, PullSeen, SaveReviewLayout, UnmarkFileViewed,
};

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_fleet, code, manifest};
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::pull_review_memory::inbox_of;
use crate::tests::tmp::TempDir;

/// What `a_page` and `pull` say the head is.
const HEAD: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";
/// A head the pull request was at before.
const OLD: &str = "1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d";

fn seen_at(head: &str) -> PullSeen {
    PullSeen { head: head.into(), at: "2026-11-13T09:00:00Z".into() }
}

fn mine(said: MyReviewSaid) -> PullMyReview {
    PullMyReview { said, commit: Some(OLD.into()), submitted_at: None }
}

#[test]
fn the_inbox_is_the_first_state_that_holds() {
    let (moved, here) = (seen_at(OLD), seen_at(HEAD));
    let approved = mine(MyReviewSaid::Approved);
    let asked = mine(MyReviewSaid::ChangesRequested);
    let commented = mine(MyReviewSaid::Commented);
    assert_eq!(inbox_of(Some(&moved), HEAD, Some(&approved)), PullInbox::ChangedSinceYouLooked, "a moved head beats his approval");
    assert_eq!(inbox_of(Some(&here), HEAD, Some(&approved)), PullInbox::Done);
    assert_eq!(inbox_of(None, HEAD, Some(&approved)), PullInbox::Done);
    assert_eq!(inbox_of(Some(&here), HEAD, Some(&asked)), PullInbox::WaitingOnAuthor);
    assert_eq!(inbox_of(None, HEAD, Some(&commented)), PullInbox::WaitingOnAuthor);
    assert_eq!(inbox_of(Some(&here), HEAD, None), PullInbox::NeedsFirstLook);
    assert_eq!(inbox_of(None, HEAD, None), PullInbox::NeedsFirstLook);
    assert_eq!(inbox_of(Some(&moved), "", None), PullInbox::NeedsFirstLook, "a head the forge did not give moved nowhere");
}

#[tokio::test]
async fn a_pull_request_marked_seen_shows_it_on_its_page_and_its_row_and_a_moved_head_is_changed_since_you_looked() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let mut approved = pull(12, SLUG);
    approved.my_latest = Some(LatestReview { said: LatestReviewSaid::Approved, commit: Some(OLD.into()), submitted_at: None });
    fleet.vcs().now_review_list(ReviewList::Requested, vec![approved, pull(13, SLUG)]);
    fleet.vcs().now_pull_request_page(Some(a_page()));

    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.requested[0].inbox, PullInbox::Done, "never seen, and he approved");
    assert_eq!(lists.requested[0].my_review.as_ref().map(|it| it.said), Some(MyReviewSaid::Approved));
    assert_eq!(lists.requested[1].inbox, PullInbox::NeedsFirstLook);

    let kept = fleet.mark_pull_request_seen(id.clone(), MarkPullSeen { number: 12, head: OLD.into() }).await.expect("kept");
    assert_eq!(kept.head, OLD);
    let page = fleet.get_pull_request_detail(id.clone(), 12).await.expect("read");
    assert_eq!(page.seen.map(|it| it.head), Some(OLD.to_string()));
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.requested[0].seen.as_ref().map(|it| it.head.as_str()), Some(OLD));
    assert_eq!(lists.requested[0].inbox, PullInbox::ChangedSinceYouLooked);
    assert_eq!(lists.requested[1].inbox, PullInbox::NeedsFirstLook, "a mark is one pull request's");

    // He reads it again and leaves at the head: back to what his review says.
    fleet.mark_pull_request_seen(id.clone(), MarkPullSeen { number: 12, head: HEAD.into() }).await.expect("kept");
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.requested[0].inbox, PullInbox::Done);

    let refused = fleet.mark_pull_request_seen(id, MarkPullSeen { number: 12, head: "main".into() }).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (422, "fleet.pull_review_memory_invalid"));
}

fn viewed(path: &str, commit: &str, blob: Option<&str>) -> MarkFileViewed {
    MarkFileViewed { number: 12, path: path.into(), commit: commit.into(), blob: blob.map(Into::into) }
}

#[tokio::test]
async fn a_file_viewed_at_an_older_head_stays_viewed_only_while_its_blob_is_the_same_and_unmarking_forgets_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet.vcs().now_pull_request_patches(Some(vec![
        PatchedFile { path: "same.rs".into(), patch: Some("@@ -1 +1 @@".into()), blob: Some("b1".into()) },
        PatchedFile { path: "edited.rs".into(), patch: Some("@@ -1 +1 @@".into()), blob: Some("b9".into()) },
    ]));
    for mark in [
        viewed("same.rs", OLD, Some("b1")),
        viewed("edited.rs", OLD, Some("b2")),
        viewed("at_head.rs", HEAD, None),
        viewed("no_blob.rs", OLD, None),
    ] {
        let kept = fleet.mark_file_viewed(id.clone(), mark).await.expect("kept");
        assert!(kept.still);
    }

    let page = fleet.get_pull_request_detail(id.clone(), 12).await.expect("read");
    let mut still: Vec<(&str, bool)> = page.viewed.iter().map(|it| (it.path.as_str(), it.still)).collect();
    still.sort();
    assert_eq!(still, [("at_head.rs", true), ("edited.rs", false), ("no_blob.rs", false), ("same.rs", true)]);

    let unmark = UnmarkFileViewed { number: 12, path: "same.rs".into() };
    fleet.unmark_file_viewed(id.clone(), unmark.clone()).await.expect("forgotten");
    fleet.unmark_file_viewed(id.clone(), unmark).await.expect("one not marked is not an error");
    let page = fleet.get_pull_request_detail(id.clone(), 12).await.expect("read");
    assert!(page.viewed.iter().all(|it| it.path != "same.rs"), "{:?}", page.viewed);
    assert_eq!(page.viewed.len(), 3);

    let blank = fleet.mark_file_viewed(id.clone(), viewed(" ", HEAD, None)).await.unwrap_err();
    let headless = fleet.mark_file_viewed(id, viewed("a.rs", "not-a-commit", None)).await.unwrap_err();
    for refused in [blank, headless] {
        assert_eq!((refused.status(), code(&refused)), (422, "fleet.pull_review_memory_invalid"));
    }
}

#[tokio::test]
async fn a_compare_is_the_forges_files_between_two_commits_and_a_lost_base_is_an_answer() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    let of = PullRequestCompareOf { since: OLD.into(), head: HEAD.into() };
    fleet.vcs().now_pull_request_compare(
        OLD,
        HEAD,
        ComparedPatches::Patches(vec![PatchedFile { path: "a.rs".into(), patch: Some("@@ -1 +1 @@\n-a\n+b".into()), blob: Some("b1".into()) }]),
    );
    match fleet.get_pull_request_compare(id.clone(), 12, of.clone()).await.expect("compared") {
        PullRequestCompare::Patches { since, head, files } => {
            assert_eq!((since.as_str(), head.as_str()), (OLD, HEAD));
            assert_eq!(files.len(), 1);
            assert_eq!((files[0].path.as_str(), files[0].blob.as_deref(), files[0].number), ("a.rs", Some("b1"), 12));
        }
        other => panic!("files were expected, not {other:?}"),
    }
    assert_eq!(fleet.vcs().compares_asked(), [(OLD.to_string(), HEAD.to_string())]);

    fleet.vcs().now_pull_request_compare(OLD, HEAD, ComparedPatches::BaseGone);
    let gone = fleet.get_pull_request_compare(id.clone(), 12, of).await.expect("an answer, not a refusal");
    assert_eq!(gone, PullRequestCompare::BaseGone { since: OLD.into(), head: HEAD.into() });

    let silent = PullRequestCompareOf { since: HEAD.into(), head: OLD.into() };
    let refused = fleet.get_pull_request_compare(id.clone(), 12, silent).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (500, "fleet.pull_request_compare_unreadable"));
    let named = PullRequestCompareOf { since: "main".into(), head: HEAD.into() };
    let refused = fleet.get_pull_request_compare(id, 12, named).await.unwrap_err();
    assert_eq!((refused.status(), code(&refused)), (422, "fleet.pull_review_memory_invalid"));
}

const MINE: &str = r#"{"root":{"kind":"pane","id":"p1","tabs":[{"id":"t1","kind":"pr-files"}],"active":"t1"},"focus":"p1","next":2}"#;

fn save(name: &str, arrangement: &str) -> SaveReviewLayout {
    SaveReviewLayout { name: name.into(), arrangement: arrangement.into() }
}

fn names(layouts: &ipc::ReviewLayouts) -> Vec<&str> {
    layouts.layouts.iter().map(|it| it.name.as_str()).collect()
}

#[tokio::test]
async fn default_is_listed_first_and_picked_until_another_is_and_a_deleted_pick_falls_back_to_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let listed = fleet.list_review_layouts().await.expect("read");
    assert_eq!((names(&listed), listed.picked.as_str()), (vec!["Default"], "Default"));
    assert!(listed.layouts[0].shipped);
    assert!(listed.layouts[0].arrangement.contains("pr-findings"), "three columns: {}", listed.layouts[0].arrangement);

    let saved = fleet.save_review_layout(save("Files only", MINE)).await.expect("kept");
    assert_eq!(names(&saved), ["Default", "Files only"]);
    assert!(!saved.layouts[1].shipped);
    assert_eq!(saved.picked, "Default", "saving is not picking");

    let picked = fleet.pick_review_layout(NamedReviewLayout { name: "Files only".into() }).await.expect("picked");
    assert_eq!(picked.picked, "Files only");
    assert_eq!(fleet.list_review_layouts().await.expect("read").picked, "Files only");

    let deleted = fleet.delete_review_layout(NamedReviewLayout { name: "Files only".into() }).await.expect("deleted");
    assert_eq!((names(&deleted), deleted.picked.as_str()), (vec!["Default"], "Default"));
}

#[tokio::test]
async fn default_cannot_be_saved_over_or_deleted_and_a_layout_is_an_object_with_a_root() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let refusals = [
        fleet.save_review_layout(save("default", MINE)).await.unwrap_err(),
        fleet.save_review_layout(save("  ", MINE)).await.unwrap_err(),
        fleet.save_review_layout(save(&"x".repeat(61), MINE)).await.unwrap_err(),
        fleet.save_review_layout(save("Mine", "[1, 2]")).await.unwrap_err(),
        fleet.save_review_layout(save("Mine", r#"{"focus":"p1"}"#)).await.unwrap_err(),
        fleet.save_review_layout(save("Mine", "not json")).await.unwrap_err(),
        fleet.delete_review_layout(NamedReviewLayout { name: "Default".into() }).await.unwrap_err(),
        fleet.pick_review_layout(NamedReviewLayout { name: "Nobody saved this".into() }).await.unwrap_err(),
    ];
    for refused in &refusals {
        assert_eq!((refused.status(), code(refused)), (422, "fleet.pull_review_memory_invalid"));
    }
    assert_eq!(names(&fleet.list_review_layouts().await.expect("read")), ["Default"], "nothing was kept");
}
