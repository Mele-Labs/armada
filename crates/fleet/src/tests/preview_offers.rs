//! Fleet offers a refresh of the preview, and never starts one, when a branch the preview left out
//! moves past the tip it was left out at: once a tip, on the Job holding the branch, until the
//! person presses it or skips it, or the build changes.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use api::HostedSessions;
use ipc::{DismissWaiting, SessionId, WaitingActKind, WaitingItem, WaitingSource};

use super::build_notices::{a_rig, commit_on, Rig};

const WHY: &str = "conflict: apps/a.ts";

fn root(rig: &Rig) -> PathBuf {
    PathBuf::from(rig.fleet.repositories().first().expect("served").root())
}

/// The repository holds `scripts/restart-build`, so Fleet has a build to read the branches in.
fn with_wrapper(rig: &Rig) {
    let scripts = root(rig).join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let wrapper = scripts.join("restart-build");
    fs::write(&wrapper, "#!/bin/sh\necho \"$@\" > ran\n").unwrap();
    Command::new("chmod").arg("+x").arg(&wrapper).status().unwrap();
}

/// A new commit on `armada/other`, whose tip it answers.
fn moved(rig: &Rig) -> String {
    commit_on(&root(rig), "armada/other", "armada/other", "moved on")
}

/// The preview at the rig's build left `armada/other` out at its tip, and the branch moved on since.
fn left_out_and_moved(rig: &Rig) -> String {
    with_wrapper(rig);
    rig.on_file(true);
    rig.left_out(&rig.build, &[("armada/other", WHY)]);
    moved(rig)
}

async fn sweep(rig: &Rig) {
    rig.fleet.preview_offered_when_left_out_moved().await;
}

async fn offers_on(rig: &Rig, session: &str) -> Vec<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(SessionId::carried(session)).await.unwrap();
    record.session.waiting_for.into_iter().filter(|one| one.source == WaitingSource::PreviewOffer).collect()
}

fn nothing_restarted(rig: &Rig) {
    assert!(!rig.support().join("restart-build.status").exists(), "Fleet started a restart by itself");
}

#[tokio::test]
async fn a_moved_left_out_branch_is_offered_to_the_job_holding_it_once_and_nothing_restarts() {
    let rig = a_rig();
    let holding = rig.a_hosted_session().await;
    let elsewhere = rig.a_hosted_session().await;
    rig.holds_branch(&holding, "armada/other").await;
    rig.holds_branch(&elsewhere, "armada/work").await;
    let tip = left_out_and_moved(&rig);

    sweep(&rig).await;

    nothing_restarted(&rig);
    let offers = offers_on(&rig, &holding).await;
    let [offer] = offers.as_slice() else { panic!("one offer: {offers:?}") };
    assert_eq!(offer.id, format!("preview:armada/other@{tip}"));
    assert_eq!(offer.text, format!("armada/other moved since the preview left it out: {WHY}"));
    assert_eq!(
        offer.act.as_ref().map(|act| (act.kind, act.target.as_str())),
        Some((WaitingActKind::Run, "scripts/preview --restart"))
    );
    assert!(offers_on(&rig, &elsewhere).await.is_empty(), "its branch was not left out");
    assert!(rig.thread_of(&holding).await.is_empty(), "the agent is told nothing");

    sweep(&rig).await;
    rig.restarts().await;
    sweep(&rig).await;
    assert_eq!(offers_on(&rig, &holding).await, offers, "offered once a tip, across a restart");
    nothing_restarted(&rig);
}

#[tokio::test]
async fn a_branch_left_out_for_a_breaking_migration_or_still_where_it_was_is_not_offered() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/other").await;
    rig.holds_branch(&session, "armada/work").await;
    with_wrapper(&rig);
    rig.on_file(true);
    rig.left_out(
        &rig.build,
        &[("armada/other", "breaking migration projects.span_repositories is not on main"), ("armada/work", WHY)],
    );
    moved(&rig);

    sweep(&rig).await;

    assert!(offers_on(&rig, &session).await.is_empty());
    nothing_restarted(&rig);
}

#[tokio::test]
async fn skip_hides_the_offer_tells_the_agent_nothing_and_only_a_newer_tip_offers_again() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/other").await;
    left_out_and_moved(&rig);
    sweep(&rig).await;
    let offers = offers_on(&rig, &session).await;
    let [offer] = offers.as_slice() else { panic!("one offer: {offers:?}") };

    Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting {
            session_id: SessionId::carried(session.as_str()),
            item_id: offer.id.clone(),
            reason: None,
            misread: false,
        })
        .await
        .expect("skipped");

    assert!(offers_on(&rig, &session).await.is_empty(), "skipped");
    assert!(rig.thread_of(&session).await.is_empty(), "Skip tells the agent nothing");
    sweep(&rig).await;
    rig.restarts().await;
    sweep(&rig).await;
    assert!(offers_on(&rig, &session).await.is_empty(), "the tip he skipped is not offered again");

    let newer = moved(&rig);
    sweep(&rig).await;
    let offers: Vec<String> = offers_on(&rig, &session).await.into_iter().map(|one| one.id).collect();
    assert_eq!(offers, [format!("preview:armada/other@{newer}")]);
    nothing_restarted(&rig);
}

#[tokio::test]
async fn a_new_build_drops_the_offer_made_for_the_old_one() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/other").await;
    left_out_and_moved(&rig);
    sweep(&rig).await;
    assert_eq!(offers_on(&rig, &session).await.len(), 1);

    let rebuilt = commit_on(&root(&rig), "preview", &rig.build, "rebuilt");
    rig.on_file_at(&rebuilt, true);
    rig.left_out(&rebuilt, &[("armada/other", WHY)]);
    sweep(&rig).await;

    assert!(offers_on(&rig, &session).await.is_empty(), "the refresh happened");
}

#[tokio::test]
async fn the_build_notice_does_not_take_an_offer_for_a_restart_it_answers() {
    let rig = a_rig();
    let session = rig.a_hosted_session().await;
    rig.holds_branch(&session, "armada/other").await;
    left_out_and_moved(&rig);
    rig.fleet.builds_told().await;
    let told = rig.thread_of(&session).await;
    assert_eq!(told.len(), 1, "told why its branch is out: {told:?}");
    sweep(&rig).await;
    assert_eq!(offers_on(&rig, &session).await.len(), 1);

    let rebuilt = commit_on(&root(&rig), "preview", &rig.build, "rebuilt");
    rig.on_file_at(&rebuilt, true);
    rig.left_out(&rebuilt, &[("armada/other", WHY)]);
    rig.fleet.builds_told().await;

    assert_eq!(rig.thread_of(&session).await, told, "its branch is where it was, and an offer is no wait to answer");
    assert_eq!(
        rig.fleet.sweeping().lock().await.build_told.as_deref(),
        Some(rebuilt.as_str()),
        "the build is settled, with nobody left to ask again"
    );
}
