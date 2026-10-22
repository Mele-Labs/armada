//! A person's refresh of one pull request, against a scripted forge: it tells the owner what the
//! pull reading would, once, shares the pull reading's record of the merge queue, and puts what the forge
//! shows now on the merge line. What the pull reading itself tells is `pull_notices`' tests.

use std::sync::Arc;

use adapter_traits::{
    CiState, FromOutside, OpenPull, PullQueue, PullRequestFacts, PullRequestStanding,
    RecentlyMerged, UnderReview, WatchedPull, WhatTheForgeRan,
};
use api::{PullRequests, Refusal};
use ipc::{ManifestId, PullRequestState};

use super::main_ci::{run, ONE};
use super::pull_notices::{a_rig, failing, pull, written_by, Rig};

/// What the forge answers for `pull` read by number, and its standing.
fn the_forge_shows(rig: &Rig, standing: PullRequestStanding, watched: Vec<WatchedPull>) {
    let vcs = rig.fleet.vcs();
    let failed = watched.iter().any(WatchedPull::failed);
    if let Some(one) = watched.first() {
        vcs.now_pull_request(Some(PullRequestFacts {
            standing,
            branch: one.branch.as_written().to_string(),
            auto_merge: one.auto_merge,
            title: "Fix the reader".into(),
            url: one.url.as_written().to_string(),
            body: None,
            closes: Vec::new(),
        }));
    }
    vcs.now_under_review(UnderReview {
        checks: match failed {
            true => WhatTheForgeRan::SomeFailed {
                failed: vec![FromOutside::verbatim("ci")],
                checks: 2,
                weight: adapter_traits::FailureWeight::Required,
            },
            false => WhatTheForgeRan::AllPassed { checks: 2 },
        },
        ..UnderReview::unreadable()
    });
    vcs.main_ci.watched_by_number_are(watched);
}

fn listed(number: u64, title: &str) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(title),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        ci: Some(CiState::Pending),
        head: Some(ONE.to_string()),
        failing: Vec::new(),
        auto_merge: false,
        facts: Default::default(),
    }
}

async fn refreshed(rig: &Rig, number: u64) -> Result<PullRequestState, Refusal> {
    Arc::clone(&rig.fleet)
        .refresh_pull_request(ManifestId::carried(rig.manifest()), number)
        .await
}

/// The merge line's pull requests as last read: number and title.
async fn on_the_merge_line(rig: &Rig) -> Vec<(u64, String)> {
    let root = rig.fleet.repositories().first().unwrap().root().to_string();
    rig.fleet
        .sweeping()
        .lock()
        .await
        .pulls
        .get(&root)
        .map(|pulls| {
            pulls
                .iter()
                .map(|one| (one.pull.number, one.pull.title.as_written().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// The merge line as the hub's own listing reads it.
async fn the_hub_lists(rig: &Rig, pulls: Vec<OpenPull>) {
    rig.fleet.vcs().main_ci.pulls_are(Some(pulls));
    let served = rig.fleet.repositories().served().remove(0);
    assert!(rig.fleet.notice_pulls(&served).await);
}

#[tokio::test]
async fn a_refresh_tells_a_failed_check_to_the_session_that_holds_the_pull_request_once() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "FAIL tests::one\n");
    the_forge_shows(&rig, PullRequestStanding::Open, vec![failing(7)]);

    refreshed(&rig, 7).await.expect("the forge answered");
    refreshed(&rig, 7).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("one turn started: {sent:?}")
    };
    assert!(line.contains("#7 failed its checks"), "{line}");
    assert!(line.contains("FAIL tests::one"), "{line}");
}

#[tokio::test]
async fn a_refresh_tells_a_conflict() {
    let rig = a_rig();
    rig.a_hosted_session_on(8).await;
    let conflicting = WatchedPull { conflicting: true, ..pull(8, Vec::new()) };
    the_forge_shows(&rig, PullRequestStanding::Open, vec![conflicting]);

    refreshed(&rig, 8).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("one turn started: {sent:?}")
    };
    assert!(line.contains("#8 conflicts with"), "{line}");
}

/// The pull reading saw it queued and the refresh sees it out: one record of the queue, so the drop is
/// told once whichever reading sees it.
#[tokio::test]
async fn a_pull_request_the_pull_reading_saw_queued_and_a_refresh_sees_out_is_told_it_left_once() {
    let rig = a_rig();
    rig.a_hosted_session_on(10).await;
    let queued = WatchedPull { queue: PullQueue::Waiting, ..pull(10, Vec::new()) };
    rig.fleet.vcs().main_ci.watched_are(Some(vec![queued]));
    rig.reads().await;

    the_forge_shows(&rig, PullRequestStanding::Open, vec![pull(10, Vec::new())]);
    refreshed(&rig, 10).await.expect("the forge answered");
    refreshed(&rig, 10).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("told once, by the refresh: {sent:?}")
    };
    assert!(line.contains("#10 left the merge queue"), "{line}");

    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(10, Vec::new())]));
    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 1, "the pull reading does not tell it again");
}

#[tokio::test]
async fn a_refresh_tells_auto_merge_on_once() {
    let rig = a_rig();
    rig.a_hosted_session_on(11).await;
    let auto = WatchedPull { auto_merge: true, ..pull(11, Vec::new()) };
    the_forge_shows(&rig, PullRequestStanding::Open, vec![auto]);

    refreshed(&rig, 11).await.expect("the forge answered");
    refreshed(&rig, 11).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("told once: {sent:?}")
    };
    assert!(line.contains("Auto-merge is on for #11"), "{line}");
}

/// **Only the person's own**, as the pull reading: another author's pull request is nobody here's to tell.
#[tokio::test]
async fn a_refresh_tells_the_persons_own_pull_request_and_not_another_authors() {
    let rig = a_rig();
    rig.a_hosted_session_on(8).await;
    let theirs = rig.a_hosted_session_on(9).await;
    rig.fleet.vcs().main_ci.login_is(Some("octocat"));
    let conflicting = |number| WatchedPull { conflicting: true, ..pull(number, Vec::new()) };
    the_forge_shows(
        &rig,
        PullRequestStanding::Open,
        vec![written_by(conflicting(8), "octocat"), written_by(conflicting(9), "teammate")],
    );

    refreshed(&rig, 8).await.expect("the forge answered");
    refreshed(&rig, 9).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("one turn started, for the pull request that is mine: {sent:?}")
    };
    assert!(line.contains("#8 conflicts with"), "{line}");
    assert!(!rig.told(&format!("session:{theirs}"), 9, "conflicts").await);
}

#[tokio::test]
async fn a_refresh_of_a_merged_pull_request_tells_the_merge_and_takes_it_off_the_merge_line() {
    let rig = a_rig();
    rig.a_hosted_session_on(1).await;
    the_hub_lists(&rig, vec![listed(1, "Fix the reader"), listed(9, "Another")]).await;
    the_forge_shows(&rig, PullRequestStanding::Merged, vec![pull(1, Vec::new())]);
    rig.fleet.vcs().main_ci.recently_merged_are(Some(vec![RecentlyMerged {
        number: 1,
        title: FromOutside::verbatim("Fix the reader"),
        branch: FromOutside::verbatim("armada/1"),
        url: FromOutside::verbatim("https://forge.invalid/armada/pull/1"),
        author: None,
        merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
        commit: None,
    }]));

    refreshed(&rig, 1).await.expect("the forge answered");

    let sent = rig.runs_sent_to_the_session();
    assert!(sent.iter().any(|line| line.contains("#1 merged into")), "{sent:?}");
    assert_eq!(on_the_merge_line(&rig).await, [(9, "Another".to_string())]);
}

#[tokio::test]
async fn a_refresh_of_a_closed_pull_request_takes_it_off_the_merge_line() {
    let rig = a_rig();
    the_hub_lists(&rig, vec![listed(1, "Fix the reader"), listed(9, "Another")]).await;
    the_forge_shows(&rig, PullRequestStanding::Closed, vec![pull(1, Vec::new())]);

    refreshed(&rig, 1).await.expect("the forge answered");

    assert_eq!(on_the_merge_line(&rig).await, [(9, "Another".to_string())]);
}

#[tokio::test]
async fn a_refresh_puts_what_the_forge_shows_now_on_the_merge_line() {
    let rig = a_rig();
    the_hub_lists(&rig, vec![listed(7, "Before"), listed(9, "Another")]).await;
    the_forge_shows(&rig, PullRequestStanding::Open, vec![pull(7, Vec::new())]);
    rig.fleet
        .vcs()
        .main_ci
        .open_by_number_are(vec![listed(7, "After"), listed(12, "Not listed yet")]);

    refreshed(&rig, 7).await.expect("the forge answered");
    refreshed(&rig, 12).await.expect("the forge answered");

    let line = on_the_merge_line(&rig).await;
    assert_eq!(
        line,
        [(7, "After".to_string()), (9, "Another".to_string()), (12, "Not listed yet".to_string())]
    );
}
