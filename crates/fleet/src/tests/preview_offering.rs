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
use ipc::{
    BuildNotes, CloseSession, MarkBuildNoteLookedAt, NoteVerdict, OfferPreview, PreviewOffers, PreviewState, PreviewStep, SendBuildNoteFeedback, SessionId,
    SkipPreviewRefresh, WaitingSource, WithdrawPreview,
};

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

pub(super) fn root(rig: &Rig) -> PathBuf {
    PathBuf::from(rig.fleet.repositories().first().expect("served").root())
}

/// The repository has a build to read and a `scripts/preview` to offer to, and nothing compiles for real.
pub(super) fn a_previewable_repository(rig: &Rig) {
    let scripts = root(rig).join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(scripts.join("restart-build"), "#!/bin/sh\n").unwrap();
    fs::write(scripts.join("preview"), "#!/bin/sh\n").unwrap();
    rig.fleet.previewing().compiling_with(Arc::new(Planted));
}

/// A branch off `from` with one file committed on it, and its tip.
pub(super) fn branch_with(rig: &Rig, branch: &str, from: &str, file: &str, contents: &str) -> String {
    let root = root(rig);
    git(&root, &["checkout", "-q", "-B", branch, from]);
    let path = root.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
    git(&root, &["add", "--", file]);
    git(&root, &["commit", "-q", "-m", branch]);
    git(&root, &["rev-parse", "HEAD"])
}

pub(super) fn base(rig: &Rig) -> String {
    git(&root(rig), &["rev-parse", "origin/main"])
}

pub(super) async fn offer(rig: &Rig, session: &str, branch: Option<&str>, about: Option<&str>) -> Result<ipc::PreviewOffered, Refusal> {
    Arc::clone(&rig.fleet)
        .offer_preview(
            None,
            OfferPreview {
                session_id: Some(SessionId::carried(session)),
                branch: branch.map(str::to_string),
                about: about.map(str::to_string),
                // A look, so the tests of the check read only what the check tells the Job.
                look: Some("Look at it".to_string()),
                steps: None,
                approved: None,
                pictures: None,
                words: None,
            },
        )
        .await
}

/// Every check asked for has run.
pub(super) async fn checked(rig: &Rig) {
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
pub(super) async fn a_job_offering(rig: &Rig, branch: &str, file: &str, contents: &str) -> (String, String) {
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
async fn an_offer_the_build_commit_merges_is_in_the_build_though_preview_held_names_no_branch() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("One")).await.expect("offered");
    checked(&rig).await;
    assert!(offers(&rig).await.refresh.is_some());

    let main = base(&rig);
    git(&root(&rig), &["checkout", "-q", "-B", "preview", &main]);
    git(&root(&rig), &["merge", "-q", "--no-ff", "-m", "Merge branch 'armada/one' into preview", "armada/one"]);
    let build = git(&root(&rig), &["rev-parse", "HEAD"]);
    rig.on_file_at(&build, true);
    fs::write(rig.support().join("preview-held"), format!("{build}\n{main}\n")).unwrap();

    let read = offers(&rig).await;
    assert_eq!((read.offers[0].tip.as_str(), read.offers[0].in_build), (tip.as_str(), true), "the commit merges it at that tip");
    assert_eq!(read.refresh, None);
}

/// `tip` has landed on main, and the running preview was cut from that main: its commit stands on `tip` and
/// holds no branch, so neither its merges nor `preview-held` name the branch. Returns the build commit.
pub(super) fn built_on_main_holding(rig: &Rig, tip: &str) -> String {
    git(&root(rig), &["update-ref", "refs/remotes/origin/main", tip]);
    git(&root(rig), &["checkout", "-q", "-B", "preview", tip]);
    git(&root(rig), &["commit", "-q", "--allow-empty", "-m", "the preview, cut from that main"]);
    let build = git(&root(rig), &["rev-parse", "HEAD"]);
    rig.on_file_at(&build, true);
    fs::write(rig.support().join("preview-held"), format!("{build}\n{tip}\n")).unwrap();
    build
}

#[tokio::test]
async fn an_offer_whose_tip_is_on_main_under_the_build_is_in_the_build_though_the_build_names_no_branch() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("One")).await.expect("offered");
    checked(&rig).await;
    assert!(offers(&rig).await.refresh.is_some());

    built_on_main_holding(&rig, &tip);

    let read = offers(&rig).await;
    assert_eq!((read.offers[0].tip.as_str(), read.offers[0].in_build), (tip.as_str(), true), "the build stands on main, which holds the tip");
    assert_eq!(read.refresh, None, "nothing new to refresh for");
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

pub(super) fn step(say: &str, act: Option<&str>) -> PreviewStep {
    PreviewStep { say: say.to_string(), act: act.map(str::to_string) }
}

/// An offer of `session`'s own branch carrying a look and steps, each only where given.
async fn offer_noting(rig: &Rig, session: &str, about: Option<&str>, look: Option<&str>, steps: Option<Vec<PreviewStep>>) -> Result<ipc::PreviewOffered, Refusal> {
    Arc::clone(&rig.fleet)
        .offer_preview(
            None,
            OfferPreview {
                session_id: Some(SessionId::carried(session)),
                branch: None,
                about: about.map(str::to_string),
                look: look.map(str::to_string),
                steps,
                approved: None,
                pictures: None,
                words: None,
            },
        )
        .await
}

async fn offer_kept(rig: &Rig, session: &str) -> store::KeptPreviewOffered {
    rig.fleet.store().lock().await.preview_offered_of(session, "armada/one").unwrap().expect("an offer")
}

pub(super) fn code<T>(refused: Result<T, Refusal>) -> String {
    match refused {
        Ok(_) => panic!("expected a refusal"),
        Err(refusal) => refusal.error().code.clone(),
    }
}

async fn notes(rig: &Rig) -> BuildNotes {
    rig.fleet.build_notes_read().await
}

/// The running build is the preview and holds each (branch, tip) given.
pub(super) fn holding(rig: &Rig, held: &[(&str, &str)]) {
    rig.on_file(true);
    let lines: String = held.iter().map(|(branch, tip)| format!("{branch}\t{tip}\n")).collect();
    fs::write(rig.support().join("preview-held"), format!("{}\n{}\n{lines}", rig.build, base(rig))).unwrap();
}

#[tokio::test]
async fn a_look_and_steps_are_checked_and_a_refused_offer_keeps_nothing() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let about = Some("Keeps one thing");
    let one = |steps: Vec<PreviewStep>| offer_noting(&rig, &session, about, None, Some(steps));

    assert_eq!(code(offer_noting(&rig, &session, about, Some(&"x".repeat(301)), None).await), "fleet.preview_look_invalid");
    assert_eq!(code(offer_noting(&rig, &session, about, Some("one\ntwo"), None).await), "fleet.preview_look_invalid");
    assert_eq!(code(one((0..10).map(|at| step(&format!("Step {at}"), None)).collect()).await), "fleet.preview_steps_invalid", "ten is one over");
    assert_eq!(code(one(vec![step("  ", None)]).await), "fleet.preview_steps_invalid");
    assert_eq!(code(one(vec![step(&"x".repeat(121), None)]).await), "fleet.preview_steps_invalid");
    assert_eq!(code(one(vec![step("one\ntwo", None)]).await), "fleet.preview_steps_invalid");
    assert_eq!(code(one(vec![step("Press Projects", Some("open_the_moon"))]).await), "fleet.preview_step_act_unknown");
    assert!(offers(&rig).await.offers.is_empty(), "nothing was kept");

    let nine: Vec<PreviewStep> = (0..9).map(|at| step(&format!("Step {at}"), Some("open_settings"))).collect();
    assert!(offer_noting(&rig, &session, about, Some(&"x".repeat(300)), Some(nine)).await.is_ok(), "300 characters and nine steps are the most");
    checked(&rig).await;
}

#[tokio::test]
async fn a_re_offer_that_leaves_the_look_and_steps_out_keeps_them_and_an_empty_one_clears() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let steps = vec![step("Open Settings", Some("open_settings")), step("Look at the toasts", None)];
    offer_noting(&rig, &session, Some("Keeps one thing"), Some("The toasts stack"), Some(steps.clone())).await.expect("offered");
    checked(&rig).await;
    branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "2");
    offer_noting(&rig, &session, None, None, None).await.expect("offered again");
    checked(&rig).await;
    let again = offer_kept(&rig, &session).await;
    assert_eq!(again.look.as_deref(), Some("The toasts stack"));
    assert_eq!(again.steps.iter().map(|one| (one.say.as_str(), one.act.as_deref())).collect::<Vec<_>>(), [("Open Settings", Some("open_settings")), ("Look at the toasts", None)]);

    offer_noting(&rig, &session, None, Some("A new look"), Some(Vec::new())).await.expect("offered again");
    checked(&rig).await;
    let cleared = offer_kept(&rig, &session).await;
    assert_eq!((cleared.look.as_deref(), cleared.steps.len()), (Some("A new look"), 0), "an empty list clears the steps");
    offer_noting(&rig, &session, None, Some(""), None).await.expect("offered again");
    checked(&rig).await;
    assert_eq!(offer_kept(&rig, &session).await.look, None, "an empty look clears it");
}

#[tokio::test]
async fn the_build_notes_are_the_held_branches_notes_in_offer_order_and_a_note_outlives_its_offer() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (one, tip_one) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    let (two, tip_two) = a_job_offering(&rig, "armada/two", "crates/x/two.rs", "2").await;
    offer_noting(&rig, &one, Some("Keeps one thing"), Some("Look at one"), Some(vec![step("Open Settings", Some("open_settings"))])).await.expect("offered");
    checked(&rig).await;
    offer_noting(&rig, &two, Some("Keeps two things"), None, None).await.expect("offered");
    checked(&rig).await;

    assert_eq!(notes(&rig).await, BuildNotes { commit: None, notes: Vec::new() }, "the build is main");

    holding(&rig, &[("armada/two", &tip_two), ("armada/one", &tip_one)]);
    let held = notes(&rig).await;
    assert_eq!(held.commit.as_deref(), Some(rig.build.as_str()));
    let [first, second] = held.notes.as_slice() else { panic!("one note a held branch: {held:?}") };
    assert_eq!((first.branch.as_str(), first.tip.as_str(), first.session_id.as_str()), ("armada/one", tip_one.as_str(), one.as_str()), "offer order, not the file's");
    assert_eq!((first.about.as_str(), first.look.as_deref(), first.job_ended, first.looked_at), ("Keeps one thing", Some("Look at one"), false, false));
    assert_eq!((first.steps.len(), first.steps[0].act.as_deref()), (1, Some("open_settings")));
    assert_eq!((second.branch.as_str(), second.look.as_deref(), second.steps.len()), ("armada/two", None, 0));
    assert_eq!(first.title, one, "the Job has no title, so its id");

    Arc::clone(&rig.fleet).withdraw_preview(None, WithdrawPreview { session_id: Some(SessionId::carried(one.as_str())), branch: None }).await.expect("withdrawn");
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: SessionId::carried(two.as_str()) }).await.expect("closed");
    rig.fleet.previews_watched().await;
    checked(&rig).await;

    assert!(offers(&rig).await.offers.is_empty(), "both offers are gone");
    let after = notes(&rig).await;
    assert_eq!(after.notes.iter().map(|note| (note.branch.as_str(), note.job_ended)).collect::<Vec<_>>(), [("armada/one", false), ("armada/two", true)], "the notes stay, and the ended Job says so");

    let newer = branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "3");
    holding(&rig, &[("armada/one", &newer)]);
    assert!(notes(&rig).await.notes.is_empty(), "a build holding a tip nobody offered has no note for it");
}

#[tokio::test]
async fn a_note_is_marked_looked_at_by_branch_and_tip_with_a_verdict_its_job_is_told_and_a_new_tip_is_not() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer_noting(&rig, &session, Some("Keeps one thing"), Some("Look"), None).await.expect("offered");
    checked(&rig).await;
    holding(&rig, &[("armada/one", &tip)]);
    let mut watching = rig.fleet.events().subscribe();

    let mark = |verdict| MarkBuildNoteLookedAt { branch: "armada/one".into(), tip: tip.clone(), verdict };
    let marked = api::Commands::mark_build_note_looked_at(Arc::clone(&rig.fleet), mark(Some(NoteVerdict::Good))).await.expect("marked");

    assert_eq!((marked.notes[0].looked_at, marked.notes[0].verdict), (true, Some(NoteVerdict::Good)));
    assert!(notes(&rig).await.notes[0].looked_at);
    let told = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(api::Next::Send(delivered)) => {
                    if let ipc::Event::BuildNotesChanged(told) = delivered.event {
                        return told;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("build_notes.changed arrives");
    assert!(told.notes[0].looked_at);
    let thread = rig.thread_of(&session).await;
    let named = format!("your preview note \"Keeps one thing\" on armada/one at {}", &tip[..7]);
    assert_eq!(thread.last().map(String::as_str), Some(format!("The owner says {named} is good.").as_str()), "the note's Job is told");

    api::Commands::mark_build_note_looked_at(Arc::clone(&rig.fleet), mark(Some(NoteVerdict::Good))).await.expect("the same again");
    assert_eq!(rig.thread_of(&session).await.len(), thread.len(), "the same verdict again tells it nothing");
    let changed = api::Commands::mark_build_note_looked_at(Arc::clone(&rig.fleet), mark(Some(NoteVerdict::NeedsWork))).await.expect("changed");
    assert_eq!(changed.notes[0].verdict, Some(NoteVerdict::NeedsWork));
    let thread = rig.thread_of(&session).await;
    assert_eq!(
        thread.last().map(String::as_str),
        Some(format!("The owner says {named} needs work. His words on it follow in a message: wait for them before you change it.").as_str()),
        "a changed verdict is told"
    );

    let unknown = api::Commands::mark_build_note_looked_at(Arc::clone(&rig.fleet), MarkBuildNoteLookedAt { branch: "armada/one".into(), tip: "0".repeat(40), verdict: None }).await;
    assert_eq!(unknown.expect_err("no such note").status(), 404);

    let newer = branch_with(&rig, "armada/one", "armada/one", "crates/x/more.rs", "2");
    offer_noting(&rig, &session, None, None, None).await.expect("offered again");
    checked(&rig).await;
    holding(&rig, &[("armada/one", &newer)]);
    let now = notes(&rig).await;
    assert_eq!((now.notes[0].tip.as_str(), now.notes[0].looked_at, now.notes[0].look.as_deref()), (newer.as_str(), false, Some("Look")), "a new tip is a note not looked at, with the look it kept");
    holding(&rig, &[("armada/one", &tip)]);
    assert!(notes(&rig).await.notes[0].looked_at, "the old tip is still looked at");
}

#[tokio::test]
async fn feedback_on_a_note_goes_to_its_live_job_and_starts_a_new_job_where_it_has_ended() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer_noting(&rig, &session, Some("Keeps one thing"), Some("The toasts stack"), Some(vec![step("Open Settings", Some("open_settings"))])).await.expect("offered");
    checked(&rig).await;
    holding(&rig, &[("armada/one", &tip)]);
    let feedback = |words: &str, branch: &str, tip: &str| SendBuildNoteFeedback { branch: branch.into(), tip: tip.into(), words: words.into() };

    assert_eq!(api::Commands::send_build_note_feedback(Arc::clone(&rig.fleet), feedback("  \n", "armada/one", &tip)).await.expect_err("no words").status(), 422);
    assert_eq!(api::Commands::send_build_note_feedback(Arc::clone(&rig.fleet), feedback("Too small", "armada/one", &"0".repeat(40))).await.expect_err("no note").status(), 404);

    let sent = api::Commands::send_build_note_feedback(Arc::clone(&rig.fleet), feedback("The toasts are cut off", "armada/one", &tip)).await.expect("sent");
    assert_eq!((sent.session_id.as_str(), sent.started), (session.as_str(), false));
    assert_eq!(rig.thread_of(&session).await.last().map(String::as_str), Some("On your preview note \"Keeps one thing\": The toasts are cut off"));

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: SessionId::carried(session.as_str()) }).await.expect("closed");
    let started = api::Commands::send_build_note_feedback(Arc::clone(&rig.fleet), feedback("Still cut off", "armada/one", &tip)).await.expect("started");

    assert!(started.started);
    assert_ne!(started.session_id.as_str(), session.as_str(), "a new Job, not the ended one");
    let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("the new Job is there");
    let before = Arc::clone(&rig.fleet).get_session(SessionId::carried(session.as_str())).await.expect("the ended Job is still read");
    assert_eq!(record.session.manifest_id, before.session.manifest_id, "in the same repository");
    let brief = rig.thread_of(started.session_id.as_str()).await.join("\n");
    for held in ["Still cut off", "The toasts stack", "Open Settings (open_settings)", "armada/one", tip.as_str(), "Keeps one thing"] {
        assert!(brief.contains(held), "the new Job's first message names {held}: {brief}");
    }
    assert_eq!(rig.thread_of(&session).await.len(), 2, "nothing more went to the ended Job");
}

#[tokio::test]
async fn an_offer_with_no_look_asks_its_job_for_one_and_one_with_a_look_does_not() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;

    offer_noting(&rig, &session, Some("Keeps one thing"), None, None).await.expect("offered");
    let thread = rig.thread_of(&session).await;
    assert_eq!(thread.len(), 1, "one ask: {thread:?}");
    assert!(thread[0].starts_with("Your offer of armada/one has no look"), "{thread:?}");

    offer_noting(&rig, &session, None, Some("The toasts stack"), None).await.expect("offered again");
    assert_eq!(rig.thread_of(&session).await.len(), 1, "an offer with a look asks nothing");
}

#[tokio::test]
async fn a_step_names_an_act_from_the_registry_the_offered_branch_holds() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    branch_with(&rig, "armada/one", "armada/one", "crates/core-model/domain/actions.toml", "[actions.call_snooze]\nkind = \"Action\"\n");
    let about = Some("Keeps one thing");

    let added = vec![step("Press Snooze", Some("call_snooze"))];
    assert!(offer_noting(&rig, &session, about, Some("Look"), Some(added)).await.is_ok(), "an act the branch adds is known");
    let gone = vec![step("Open Settings", Some("open_settings"))];
    assert_eq!(code(offer_noting(&rig, &session, about, Some("Look"), Some(gone)).await), "fleet.preview_step_act_unknown", "the branch's registry is the one read");
}

fn waiting(id: &str, text: &str, approves: bool, act: Option<(ipc::WaitingActKind, &str)>, options: &[&str]) -> ipc::WaitingInput {
    ipc::WaitingInput {
        approves,
        id: id.into(),
        text: text.into(),
        act: act.map(|(kind, target)| ipc::WaitingAct { kind, target: target.into(), on: None }),
        options: options.iter().map(|label| ipc::WaitingOption { label: (*label).into(), description: None, picture: None }).collect(),
        steps: Vec::new(),
        artifact: None,
    }
}

async fn set_waiting(rig: &Rig, session: &str, items: Vec<ipc::WaitingInput>) -> Result<ipc::SessionRecord, Refusal> {
    rig.fleet.set_waiting(None, ipc::SetWaitingFor { items, session_id: Some(SessionId::carried(session)) }).await
}

async fn waiting_ids(rig: &Rig, session: &str) -> Vec<String> {
    let read = Arc::clone(&rig.fleet).get_session(SessionId::carried(session)).await.unwrap().session.waiting_for;
    read.into_iter().filter(|one| one.source == WaitingSource::Agent).map(|one| one.id).collect()
}

/// The Job's pull request `number`, as the pull reading keeps it.
async fn opened_pull(rig: &Rig, session: &str, number: u64) {
    let at = rig.fleet.now().as_str().to_string();
    let row = store::KeptAttachment {
        holder: store::Holder::session(session),
        kind: "pr".into(),
        manifest_id: rig.fleet.repositories().first().expect("served").manifest().id().as_str().to_string(),
        target: number.to_string(),
        state: store::AttachmentState::Standing,
        detail: [("state".to_string(), "open".to_string())].into(),
        since: at.clone(),
        changed_at: at,
    };
    rig.fleet.store().lock().await.attach(&row, true).unwrap();
}

/// On 17 Nov Jobs raised "Try page notes ..., then approve PR #3832" above the Ready to try card that asked the
/// same: in a repository with a preview, an item asking him to try the Job's own change is refused by name.
#[tokio::test]
async fn an_item_asking_him_to_try_and_approve_the_jobs_own_change_is_refused() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    opened_pull(&rig, &session, 3832).await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;

    for item in [
        waiting("try", "Try page notes on a Job's Browser tab with ⌥⌘A, then approve PR #3832", false, None, &[]),
        waiting("approve", "Approve the page notes", false, Some((ipc::WaitingActKind::ApprovePr, "#3832")), &[]),
        waiting("approves", "Approve site asks behind the lock on armada/one?", true, None, &[]),
    ] {
        let refused = set_waiting(&rig, &session, vec![item]).await.expect_err("refused").error().clone();
        assert_eq!(refused.code, "fleet.session_waiting_look");
        assert!(refused.message.contains("offer_preview") && refused.message.contains("Ready to try"), "{}", refused.message);
    }
    assert!(waiting_ids(&rig, &session).await.is_empty(), "nothing was kept");
}

/// Another Job's pull request and a decision with options are his to settle, so they stand.
#[tokio::test]
async fn an_approval_of_another_jobs_pull_request_and_a_decision_with_options_stand() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    opened_pull(&rig, &session, 3832).await;
    let other = rig.a_hosted_session().await;
    opened_pull(&rig, &other, 3834).await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;

    set_waiting(
        &rig,
        &session,
        vec![
            waiting("theirs", "Approve PR #3834, the lock Job's change?", true, None, &[]),
            waiting("pick", "Try #3832's row: keep the count or drop it?", false, None, &["Drop it", "Keep it"]),
        ],
    )
    .await
    .expect("kept");

    assert_eq!(waiting_ids(&rig, &session).await, ["theirs", "pick"]);
}

/// An ask kept before the rule leaves Needs you once the Job's branch is on the preview list, with no one acting.
#[tokio::test]
async fn a_kept_ask_to_try_the_change_leaves_needs_you_once_the_branch_is_offered() {
    let rig = a_rig();
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    opened_pull(&rig, &session, 3832).await;
    set_waiting(
        &rig,
        &session,
        vec![
            waiting("try", "Try site asks and Site settings behind the lock, then approve PR #3832", false, None, &[]),
            waiting("restart", "Restart Fleet onto the new build", false, Some((ipc::WaitingActKind::Run, "scripts/restart")), &[]),
        ],
    )
    .await
    .expect("no preview yet, so nothing is refused");
    assert_eq!(waiting_ids(&rig, &session).await, ["try", "restart"]);

    a_previewable_repository(&rig);
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;

    assert_eq!(waiting_ids(&rig, &session).await, ["restart"]);
}
