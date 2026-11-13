//! A Job offers its branch for the owner's preview, and Fleet keeps the offer, checks the set against
//! main and itself, tells the Job what it found once, watches the branch, writes the file
//! `scripts/preview` reads, and serves one Refresh row. **Nothing here compiles**: a planted compiler
//! fails where the tree holds `crates/x/broken.rs`. The merges are real git, in the rig's repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapters::preview_check::{Broke, Compile, Compiler, CONFLICTS};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{CloseSession, OfferPreview, PreviewOffers, PreviewState, SessionId, SkipPreviewRefresh, WaitingSource, WithdrawPreview};

use super::build_notices::{a_rig, commit_on, git, Rig};

/// Fails wherever the tree holds `crates/x/broken.rs`, naming it.
struct Planted;

impl Compiler for Planted {
    fn compile(&self, tree: &Path, _what: Compile) -> Result<(), Broke> {
        if tree.join("crates/x/broken.rs").exists() {
            return Err(Broke { why: "error[E0308]: mismatched types".into(), files: vec!["crates/x/broken.rs".into()] });
        }
        Ok(())
    }
}

fn root(rig: &Rig) -> PathBuf {
    PathBuf::from(rig.fleet.repositories().first().expect("served").root())
}

/// The repository has a build to read and a `scripts/preview` to offer to, and nothing compiles for real.
fn a_previewable_repository(rig: &Rig) {
    let scripts = root(rig).join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(scripts.join("restart-build"), "#!/bin/sh\n").unwrap();
    fs::write(scripts.join("preview"), "#!/bin/sh\n").unwrap();
    rig.fleet.previewing().compiling_with(Arc::new(Planted));
}

/// A branch off `from` with one file committed on it, and its tip.
fn branch_with(rig: &Rig, branch: &str, from: &str, file: &str, contents: &str) -> String {
    let root = root(rig);
    git(&root, &["checkout", "-q", "-B", branch, from]);
    let path = root.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
    git(&root, &["add", "--", file]);
    git(&root, &["commit", "-q", "-m", branch]);
    git(&root, &["rev-parse", "HEAD"])
}

fn base(rig: &Rig) -> String {
    git(&root(rig), &["rev-parse", "origin/main"])
}

async fn offer(rig: &Rig, session: &str, branch: Option<&str>, about: Option<&str>) -> Result<ipc::PreviewOffered, Refusal> {
    Arc::clone(&rig.fleet)
        .offer_preview(
            None,
            OfferPreview {
                session_id: Some(SessionId::carried(session)),
                branch: branch.map(str::to_string),
                about: about.map(str::to_string),
            },
        )
        .await
}

/// Every check asked for has run.
async fn checked(rig: &Rig) {
    for _ in 0..3000 {
        if rig.fleet.previewing().idle() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the check never finished");
}

async fn offers(rig: &Rig) -> PreviewOffers {
    rig.fleet.preview_offers_read().await
}

fn offered_file(rig: &Rig) -> String {
    fs::read_to_string(rig.support().join("preview-offered")).unwrap_or_default()
}

fn states(read: &PreviewOffers) -> Vec<(String, PreviewState)> {
    read.offers.iter().map(|one| (one.branch.clone(), one.state)).collect()
}

/// A Job holding one branch with one file on it, offered with a line.
async fn a_job_offering(rig: &Rig, branch: &str, file: &str, contents: &str) -> (String, String) {
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, branch).await;
    let tip = branch_with(rig, branch, &base(rig), file, contents);
    (session, tip)
}

#[tokio::test]
async fn an_offer_naming_no_branch_is_the_branch_of_the_slot_the_job_holds() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;

    let offered = offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");

    assert_eq!((offered.branch.as_str(), offered.tip.as_str(), offered.state), ("armada/one", tip.as_str(), PreviewState::Checking));
    checked(&rig).await;
}

#[tokio::test]
async fn an_offer_is_refused_for_a_job_with_no_branch_an_unknown_one_one_not_ahead_of_main_and_a_repository_with_no_preview_script() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    let code = |refused: Result<ipc::PreviewOffered, Refusal>| refused.expect_err("refused").error().code.clone();

    assert_eq!(code(offer(&rig, &session, Some("armada/work"), Some("x")).await), "fleet.preview_unavailable", "no scripts/preview yet");
    a_previewable_repository(&rig);
    assert_eq!(code(offer(&rig, &session, None, Some("x")).await), "fleet.preview_no_branch", "it holds none and names none");
    assert_eq!(code(offer(&rig, &session, Some("armada/missing"), Some("x")).await), "fleet.preview_branch_unknown");
    git(&root(&rig), &["branch", "-f", "armada/level", &base(&rig)]);
    assert_eq!(code(offer(&rig, &session, Some("armada/level"), Some("x")).await), "fleet.preview_not_ahead");
    assert!(offers(&rig).await.offers.is_empty(), "nothing was kept");
}

#[tokio::test]
async fn a_first_offer_without_a_line_on_it_is_refused_and_so_is_a_line_that_is_long_or_has_two() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let code = |refused: Result<ipc::PreviewOffered, Refusal>| refused.expect_err("refused").error().code.clone();

    assert_eq!(code(offer(&rig, &session, None, None).await), "fleet.preview_about_needed");
    assert_eq!(code(offer(&rig, &session, None, Some("   ")).await), "fleet.preview_about_needed");
    assert_eq!(code(offer(&rig, &session, None, Some(&"x".repeat(121))).await), "fleet.preview_about_invalid");
    assert_eq!(code(offer(&rig, &session, None, Some("one\ntwo")).await), "fleet.preview_about_invalid");
    assert!(offer(&rig, &session, None, Some(&"x".repeat(120))).await.is_ok(), "120 is the most");
    checked(&rig).await;
}

#[tokio::test]
async fn a_re_offer_keeps_the_line_it_had_and_replaces_the_offer_at_the_new_tip() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, first) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    let newer = branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "2");

    let again = offer(&rig, &session, None, None).await.expect("offered again");
    checked(&rig).await;

    assert_ne!(first, newer);
    assert_eq!(again.tip, newer);
    let read = offers(&rig).await;
    let [row] = read.offers.as_slice() else { panic!("one offer a branch: {read:?}") };
    assert_eq!((row.tip.as_str(), row.about.as_str(), row.state), (newer.as_str(), "Keeps one thing", PreviewState::Ready));
}

#[tokio::test]
async fn a_set_that_compiles_makes_every_offer_ready_writes_the_file_and_tells_each_job_once() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (one, tip_one) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let (two, tip_two) = a_job_offering(&rig, "armada/two", "crates/x/two.rs", "2").await;
    offer(&rig, &one, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    offer(&rig, &two, None, Some("Keeps\tanother thing")).await.expect("offered");
    checked(&rig).await;

    let read = offers(&rig).await;
    assert_eq!(states(&read), [("armada/one".to_string(), PreviewState::Ready), ("armada/two".to_string(), PreviewState::Ready)]);
    assert_eq!(
        offered_file(&rig),
        format!("armada/one\t{tip_one}\t{one}\tKeeps one thing\narmada/two\t{tip_two}\t{two}\tKeeps another thing\n"),
        "oldest first, one line each"
    );
    let refresh = read.refresh.expect("a Refresh row: nothing runs the preview");
    assert_eq!(refresh.branches, ["armada/one", "armada/two"]);
    assert_eq!(rig.thread_of(&one).await, [format!("armada/one is on the preview list at {}.", &tip_one[..7])]);
    assert_eq!(rig.thread_of(&two).await, [format!("armada/two is on the preview list at {}.", &tip_two[..7])]);

    rig.fleet.previews_watched().await;
    checked(&rig).await;
    assert_eq!(rig.thread_of(&one).await.len(), 1, "a check that finds the same is not told again");
}

#[tokio::test]
async fn a_failing_set_is_checked_one_by_one_and_only_the_breaking_offer_fails_with_why_and_files() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (good, _) = a_job_offering(&rig, "armada/good", "crates/x/good.rs", "1").await;
    let (bad, tip_bad) = a_job_offering(&rig, "armada/bad", "crates/x/broken.rs", "2").await;
    for (session, about) in [(&good, "Good"), (&bad, "Bad")] {
        offer(&rig, session, None, Some(about)).await.expect("offered");
    }
    checked(&rig).await;

    let read = offers(&rig).await;
    assert_eq!(states(&read), [("armada/good".to_string(), PreviewState::Ready), ("armada/bad".to_string(), PreviewState::Failed)]);
    let failed = &read.offers[1];
    assert_eq!(failed.why.as_deref(), Some("error[E0308]: mismatched types"));
    assert_eq!(failed.files, ["crates/x/broken.rs"]);
    assert!(offered_file(&rig).starts_with("armada/good\t") && !offered_file(&rig).contains("armada/bad"), "{}", offered_file(&rig));
    assert_eq!(
        rig.thread_of(&bad).await,
        ["armada/bad is off the preview list: error[E0308]: mismatched types. Files: crates/x/broken.rs. Offer it again once it is fixed.".to_string()]
    );
    assert_eq!(rig.thread_of(&good).await.len(), 1, "told it is on the list, once");
    assert_ne!(tip_bad, base(&rig));
}

#[tokio::test]
async fn an_offer_that_conflicts_with_another_fails_naming_the_conflicting_files() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (first, _) = a_job_offering(&rig, "armada/first", "crates/x/shared.rs", "one").await;
    let (second, _) = a_job_offering(&rig, "armada/second", "crates/x/shared.rs", "two").await;
    offer(&rig, &first, None, Some("First")).await.expect("offered");
    checked(&rig).await;
    offer(&rig, &second, None, Some("Second")).await.expect("offered");
    checked(&rig).await;

    let read = offers(&rig).await;
    assert_eq!(states(&read), [("armada/first".to_string(), PreviewState::Ready), ("armada/second".to_string(), PreviewState::Failed)]);
    assert_eq!(read.offers[1].files, ["crates/x/shared.rs"]);
    assert_eq!(
        rig.thread_of(&second).await,
        [format!("armada/second is off the preview list: {CONFLICTS} crates/x/shared.rs. Offer it again once it is fixed.")]
    );
}

#[tokio::test]
async fn a_branch_that_moved_after_it_was_offered_is_dropped_and_its_job_is_told_once() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    let moved = branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "2");

    rig.fleet.previews_watched().await;
    checked(&rig).await;
    rig.fleet.previews_watched().await;
    checked(&rig).await;

    assert!(offers(&rig).await.offers.is_empty());
    assert_eq!(offered_file(&rig), "", "no ready offer, an empty file");
    let thread = rig.thread_of(&session).await;
    assert_eq!(
        thread.last().map(String::as_str),
        Some(format!("armada/one moved to {} since you offered it, so it is off the preview list. Offer it again with offer_preview.", &moved[..7]).as_str())
    );
    assert_eq!(thread.iter().filter(|said| said.contains("moved to")).count(), 1);
}

#[tokio::test]
async fn an_offer_whose_tip_is_on_main_is_dropped_without_a_word() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    let told = rig.thread_of(&session).await;
    git(&root(&rig), &["update-ref", "refs/remotes/origin/main", &tip]);

    rig.fleet.previews_watched().await;
    checked(&rig).await;

    assert!(offers(&rig).await.offers.is_empty());
    assert_eq!(rig.thread_of(&session).await, told, "nothing new was said");
}

#[tokio::test]
async fn an_offer_of_a_job_that_ended_is_dropped_without_a_word() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    let told = rig.thread_of(&session).await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: SessionId::carried(session.as_str()) }).await.expect("closed");

    rig.fleet.previews_watched().await;
    checked(&rig).await;

    assert!(offers(&rig).await.offers.is_empty());
    assert_eq!(offered_file(&rig), "");
    assert_eq!(rig.thread_of(&session).await, told);
}

#[tokio::test]
async fn a_withdrawal_takes_the_offer_off_the_list_and_one_that_was_never_made_is_not_found() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let withdraw = |branch: Option<&str>| WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: branch.map(str::to_string) };
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    assert!(offered_file(&rig).starts_with("armada/one\t"));

    let withdrawn = Arc::clone(&rig.fleet).withdraw_preview(None, withdraw(None)).await.expect("withdrawn");
    checked(&rig).await;

    assert_eq!(withdrawn.branch, "armada/one");
    assert!(offers(&rig).await.offers.is_empty());
    assert_eq!(offered_file(&rig), "");
    let again = Arc::clone(&rig.fleet).withdraw_preview(None, withdraw(Some("armada/one"))).await.expect_err("nothing offered");
    assert_eq!(again.status(), 404);
}

#[tokio::test]
async fn skipping_the_refresh_hides_it_until_the_set_of_ready_offers_changes() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (one, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let (two, _) = a_job_offering(&rig, "armada/two", "crates/x/two.rs", "2").await;
    offer(&rig, &one, None, Some("One")).await.expect("offered");
    checked(&rig).await;
    let row = offers(&rig).await.refresh.expect("a Refresh row");

    api::Commands::skip_preview_refresh(&*rig.fleet, SkipPreviewRefresh { id: row.id.clone() }).await.expect("skipped");

    assert_eq!(offers(&rig).await.refresh, None, "skipped");
    offer(&rig, &two, None, Some("Two")).await.expect("offered");
    checked(&rig).await;
    let back = offers(&rig).await.refresh.expect("the ready set changed, so the row is back");
    assert_ne!(back.id, row.id);
    assert_eq!(back.branches, ["armada/one", "armada/two"]);
}

#[tokio::test]
async fn an_offer_the_running_preview_holds_at_its_tip_is_in_the_build_and_needs_no_refresh() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("One")).await.expect("offered");
    checked(&rig).await;
    assert!(offers(&rig).await.refresh.is_some());

    rig.on_file(true);
    fs::write(rig.support().join("preview-held"), format!("{}\n{}\narmada/one\t{tip}\n", rig.build, base(&rig))).unwrap();

    let read = offers(&rig).await;
    assert!(read.offers[0].in_build);
    assert_eq!(read.refresh, None);

    let newer = branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "2");
    offer(&rig, &session, None, None).await.expect("offered again");
    checked(&rig).await;
    let read = offers(&rig).await;
    assert_eq!((read.offers[0].tip.as_str(), read.offers[0].in_build), (newer.as_str(), false), "the build holds the old tip");
    assert!(read.refresh.is_some());
}

#[tokio::test]
async fn the_moved_since_the_preview_left_it_out_row_is_no_longer_raised() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/other").await;
    rig.on_file(true);
    rig.left_out(&rig.build, &[("armada/other", "conflict: apps/a.ts")]);
    commit_on(&root(&rig), "armada/other", "armada/other", "moved on");

    rig.fleet.previews_watched().await;
    rig.fleet.builds_told().await;

    let record = Arc::clone(&rig.fleet).get_session(SessionId::carried(session.as_str())).await.unwrap();
    let raised: Vec<_> = record.session.waiting_for.iter().filter(|one| one.source == WaitingSource::PreviewOffer).collect();
    assert!(raised.is_empty(), "{raised:?}");
}
