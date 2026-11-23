//! The preview across two Macs: what Air shares of its ready offers is held here, checked together with this
//! Mac's own, written for `scripts/preview` and listed with its Mac. A Mac that is asleep or on another build still
//! counts; an offer that cannot build here is left out and named. A ready offer is pushed to origin once another Mac is paired. **One repository stands in
//! for both Macs' checkouts**, so a tip Air names is already here, and origin is a bare repository beside it.

use std::fs;
use std::path::PathBuf;

use api::{HostedSessions, OtherMacs};
use ipc::{Instant, OtherMacPreviewOffers, OtherMacSessions, PreviewLeftOut, PreviewOfferRow, PreviewOffers, PreviewState, SessionId};
use store::{MacState, PairedMac};

use super::build_notices::{a_rig, git, Rig};
use super::preview_offering::{a_job_offering, a_previewable_repository, base, branch_with, checked, offer, root};
use crate::tests::tmp::TempDir;

const AIR: &str = "mac-air";
const EARLIER: &str = "1999-01-01T00:00:00.000Z";
const LATER: &str = "2999-01-01T00:00:00.000Z";

fn air(asleep_since: Option<i64>, wire: Option<String>) -> PairedMac {
    PairedMac {
        id: AIR.into(),
        name: "Air".into(),
        colour: "teal".into(),
        address: "https://air.example".into(),
        public_key: Some(vec![1]),
        build: None,
        paired_at: 1,
        state: MacState::Linked,
        back_code: None,
        last_heard_at: Some(2),
        asleep_since,
        why: None,
        wire,
        built_at: None,
        source: None,
        pin: None,
    }
}

async fn paired(rig: &Rig, mac: PairedMac) {
    rig.fleet.store().lock().await.keep_mac(&mac).expect("paired");
}

async fn paired_awake(rig: &Rig) {
    paired(rig, air(None, Some(ipc::ProtocolId::current().as_str().to_string()))).await;
}

/// A bare repository as origin, holding main, and the rig's repository told to push to it.
fn an_origin(rig: &Rig) -> PathBuf {
    let origin = TempDir::new();
    let path = origin.path().to_path_buf();
    // Kept for the test's length: the directory goes with the handle.
    std::mem::forget(origin);
    git(&path, &["init", "-q", "--bare", "-b", "main"]);
    let url = path.to_string_lossy().into_owned();
    git(&root(rig), &["remote", "add", "origin", &url]);
    git(&root(rig), &["push", "-q", "origin", &format!("{}:refs/heads/main", base(rig))]);
    path
}

fn on_origin(origin: &PathBuf, branch: &str) -> Option<String> {
    let done = std::process::Command::new("git").arg("-C").arg(origin).args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).output().expect("git runs");
    done.status.success().then(|| String::from_utf8_lossy(&done.stdout).trim().to_string())
}

/// What Air shares for a ready offer of `branch` at `tip`, offered at `at`.
fn theirs(branch: &str, tip: &str, session: &str, at: &str) -> PreviewOfferRow {
    PreviewOfferRow {
        branch: branch.into(),
        tip: tip.into(),
        session_id: SessionId::carried(session),
        title: session.into(),
        about: format!("About {branch}"),
        offered_at: Instant::carried(at),
        state: PreviewState::Ready,
        why: None,
        files: Vec::new(),
        in_build: false,
        mac: None,
    }
}

async fn held(rig: &Rig, offers: Vec<PreviewOfferRow>) {
    let held = OtherMacPreviewOffers { mac: AIR.into(), offers };
    std::sync::Arc::clone(&rig.fleet).hold_other_mac_preview_offers(held).await.expect("held");
}

async fn read(rig: &Rig) -> PreviewOffers {
    rig.fleet.preview_offers_read().await
}

fn file(rig: &Rig) -> String {
    fs::read_to_string(rig.support().join("preview-offered")).unwrap_or_default()
}

fn branches(read: &PreviewOffers) -> Vec<(String, Option<String>, PreviewState)> {
    read.offers.iter().map(|one| (one.branch.clone(), one.mac.clone(), one.state)).collect()
}

#[tokio::test]
async fn airs_ready_offer_is_held_here_written_to_the_file_and_listed_with_its_mac() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    paired_awake(&rig).await;
    let tip = branch_with(&rig, "armada/air-work", &base(&rig), "crates/x/air.rs", "1");
    // Air's Job row names the Job, which its offer row names only by id.
    let own = SessionId::carried(rig.a_hosted_session().await);
    let mut row = std::sync::Arc::clone(&rig.fleet).get_session(own).await.expect("a Job").session;
    row.id = SessionId::carried("air-job");
    row.title = Some("Make Air quicker".into());
    rig.fleet.hold_other_mac_sessions(OtherMacSessions { mac: AIR.into(), sessions: vec![row] }).await.expect("held");

    held(&rig, vec![theirs("armada/air-work", &tip, "air-job", EARLIER)]).await;
    checked(&rig).await;

    let read = read(&rig).await;
    assert_eq!(branches(&read), [("armada/air-work".to_string(), Some(AIR.to_string()), PreviewState::Ready)]);
    assert_eq!(read.offers[0].title, "Make Air quicker");
    assert_eq!(file(&rig), format!("armada/air-work\t{tip}\tair-job\tAbout armada/air-work\n"));
    assert_eq!(read.refresh.expect("a refresh row").branches, ["armada/air-work"]);
    assert!(read.left_out.is_empty());
}

#[tokio::test]
async fn a_partner_that_is_asleep_or_on_another_build_still_counts_and_an_offer_not_on_origin_is_named() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let one = branch_with(&rig, "armada/air-one", &base(&rig), "crates/x/one.rs", "1");
    let two = branch_with(&rig, "armada/air-two", &base(&rig), "crates/x/two.rs", "2");
    let sent = vec![theirs("armada/air-one", &one, "air-job-1", EARLIER), theirs("armada/air-two", &two, "air-job-2", EARLIER)];
    let lines = format!("armada/air-one\t{one}\tair-job-1\tAbout armada/air-one\narmada/air-two\t{two}\tair-job-2\tAbout armada/air-two\n");

    // Asleep, and on another wire: neither stops its branches building here.
    paired(&rig, air(Some(1), Some("another-wire".into()))).await;
    held(&rig, sent.clone()).await;
    checked(&rig).await;
    let other = read(&rig).await;
    assert_eq!(other.offers.iter().map(|one| one.state).collect::<Vec<_>>(), [PreviewState::Ready, PreviewState::Ready]);
    assert_eq!(file(&rig), lines);
    assert!(other.left_out.is_empty(), "{:?}", other.left_out);

    // A tip neither here nor on origin cannot build here, and is the one thing left out.
    let gone = "0123456789abcdef0123456789abcdef01234567";
    let mut more = sent.clone();
    more.push(theirs("armada/air-gone", gone, "air-job-3", EARLIER));
    held(&rig, more).await;
    checked(&rig).await;
    let read = read(&rig).await;
    assert_eq!(read.offers.iter().find(|one| one.branch == "armada/air-gone").map(|one| one.state), Some(PreviewState::Failed));
    assert_eq!(file(&rig), lines);
    let why = "Air's armada/air-gone is left out of this refresh: its tip is not on origin.";
    assert_eq!(read.left_out, [PreviewLeftOut { mac: AIR.into(), name: "Air".into(), why: why.into() }]);
}

#[tokio::test]
async fn a_ready_offer_is_pushed_to_origin_when_another_mac_is_paired_and_shared_only_then() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let origin = an_origin(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    assert_eq!(on_origin(&origin, "armada/one"), None, "no other Mac is paired, so nothing is pushed");

    paired_awake(&rig).await;
    rig.fleet.preview_check_wanted();
    checked(&rig).await;

    assert_eq!(on_origin(&origin, "armada/one"), Some(tip.clone()));
    let shared = rig.fleet.share_preview_offers().await.expect("shared");
    assert_eq!(shared.offers.iter().map(|one| (one.branch.as_str(), one.tip.as_str(), one.mac.clone())).collect::<Vec<_>>(), [("armada/one", tip.as_str(), None)]);
    assert!(shared.left_out.is_empty() && shared.refresh.is_none(), "only the offers cross");
}

#[tokio::test]
async fn an_offer_that_cannot_be_pushed_stays_ready_here_is_not_shared_and_tells_its_job_once() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    git(&root(&rig), &["remote", "add", "origin", "/nowhere/at/all.git"]);
    paired_awake(&rig).await;
    let (session, _) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("Keeps one thing")).await.expect("offered");
    checked(&rig).await;
    rig.fleet.preview_check_wanted();
    checked(&rig).await;

    assert_eq!(branches(&read(&rig).await), [("armada/one".to_string(), None, PreviewState::Ready)]);
    assert!(rig.fleet.share_preview_offers().await.expect("shared").offers.is_empty());
    let told: Vec<String> = rig.thread_of(&session).await.into_iter().filter(|text| text.contains("could not be pushed")).collect();
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(told[0].starts_with("armada/one is on the preview list here, but it could not be pushed to origin"), "{}", told[0]);
}

#[tokio::test]
async fn a_partner_offer_that_clashes_with_an_earlier_own_offer_fails_with_the_files_and_is_not_kept_in_the_file() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    an_origin(&rig);
    paired_awake(&rig).await;
    let (session, own) = a_job_offering(&rig, "armada/own", "crates/x/shared.rs", "one").await;
    offer(&rig, &session, None, Some("Mine")).await.expect("offered");
    checked(&rig).await;
    let theirs_tip = branch_with(&rig, "armada/air", &base(&rig), "crates/x/shared.rs", "two");

    held(&rig, vec![theirs("armada/air", &theirs_tip, "air-job", LATER)]).await;
    checked(&rig).await;

    let read = read(&rig).await;
    assert_eq!(
        branches(&read),
        [("armada/own".to_string(), None, PreviewState::Ready), ("armada/air".to_string(), Some(AIR.to_string()), PreviewState::Failed)]
    );
    assert_eq!(read.offers[1].files, ["crates/x/shared.rs"]);
    assert_eq!(file(&rig), format!("armada/own\t{own}\t{session}\tMine\n"));
    assert_eq!(read.left_out.len(), 1);
    assert!(read.left_out[0].why.starts_with("Air's armada/air is left out of this refresh: it does not build here, "), "{}", read.left_out[0].why);
    let shared = rig.fleet.share_preview_offers().await.expect("shared");
    assert_eq!(shared.offers.iter().map(|one| one.branch.as_str()).collect::<Vec<_>>(), ["armada/own"]);
    assert_eq!(rig.thread_of(&session).await.len(), 1, "its own Job is told of its own offer alone");
}

#[tokio::test]
async fn an_own_offer_that_clashes_with_an_earlier_partner_offer_fails_names_the_mac_and_its_job_is_told_once() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    an_origin(&rig);
    paired_awake(&rig).await;
    let (session, _) = a_job_offering(&rig, "armada/own", "crates/x/shared.rs", "one").await;
    offer(&rig, &session, None, Some("Mine")).await.expect("offered");
    checked(&rig).await;
    let theirs_tip = branch_with(&rig, "armada/air", &base(&rig), "crates/x/shared.rs", "two");

    held(&rig, vec![theirs("armada/air", &theirs_tip, "air-job", EARLIER)]).await;
    checked(&rig).await;
    rig.fleet.previews_watched().await;
    checked(&rig).await;

    let read = read(&rig).await;
    assert_eq!(
        branches(&read),
        [("armada/air".to_string(), Some(AIR.to_string()), PreviewState::Ready), ("armada/own".to_string(), None, PreviewState::Failed)]
    );
    assert_eq!(read.offers[1].files, ["crates/x/shared.rs"]);
    assert!(file(&rig).starts_with("armada/air\t") && !file(&rig).contains("armada/own"), "{}", file(&rig));
    assert!(rig.fleet.share_preview_offers().await.expect("shared").offers.is_empty(), "a failed offer is not shared");
    let failed: Vec<String> = rig.thread_of(&session).await.into_iter().filter(|text| text.contains("is off the preview list")).collect();
    assert_eq!(
        failed,
        [format!(
            "armada/own is off the preview list: {} crates/x/shared.rs. Air's armada/air changes it too. Offer it again once it is fixed.",
            adapters::preview_check::CONFLICTS
        )]
    );
}
