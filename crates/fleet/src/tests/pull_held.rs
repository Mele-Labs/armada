//! A pull request someone holds is watched and listed even where the forge's list of 100 leaves
//! it out. `docs/concepts/fleet.md`, *Telling the owner of a pull request*.

use adapter_traits::{CiState, FromOutside, RecentlyMerged};

use super::main_ci::{pull as open_pull, run, ONE};
use super::main_fix::a_finished_job;
use super::pull_notices::{a_rig, failing, owns_pull};

fn merged(number: u64, now: &str) -> RecentlyMerged {
    RecentlyMerged {
        number,
        title: FromOutside::verbatim("Held"),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        merged_at: FromOutside::verbatim(now),
        commit: None,
    }
}

#[tokio::test]
async fn a_held_pull_request_outside_the_list_is_told_its_failure_and_its_merge() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.watched_are(Some(vec![failing(8)]));
    forge.watched_by_number_are(vec![failing(7)]);

    rig.reads().await;

    assert_eq!(forge.numbers_asked(), [7], "only the held one is read by number");
    let sent = rig.runs_sent_to_the_session();
    assert!(
        sent.iter().any(|line| line.contains("#7 failed its checks")),
        "{sent:?}"
    );

    forge.watched_by_number_are(Vec::new());
    forge.recently_merged_are(Some(vec![merged(7, rig.fleet.now().as_str())]));
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_merged(&served).await;
    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    assert!(
        sent.iter().any(|line| line.contains("#7 merged into")),
        "{sent:?}"
    );
    assert!(rig.told(&format!("session:{session}"), 7, "merged").await);
}

#[tokio::test]
async fn a_pull_request_in_the_list_and_held_is_one_pull_request_told_once() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.watched_are(Some(vec![failing(7)]));
    forge.watched_by_number_are(vec![failing(7)]);

    rig.reads().await;
    rig.reads().await;

    assert!(forge.numbers_asked().is_empty(), "the list already has it");
    assert_eq!(rig.runs_sent_to_the_session().len(), 1);
}

#[tokio::test]
async fn a_pull_request_nobody_holds_outside_the_list_is_not_read_by_number() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.watched_are(Some(vec![failing(8)]));
    forge.watched_by_number_are(vec![failing(7), failing(99)]);

    rig.reads().await;

    assert_eq!(forge.numbers_asked(), [7], "99 is nobody's");
}

#[tokio::test]
async fn a_job_holding_a_pull_request_outside_the_list_has_it_read_and_listed() {
    let rig = a_rig();
    let job = a_finished_job(&rig.fleet, &rig._home, "fix the reader").await;
    owns_pull(&rig, job.id(), 9).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.watched_are(Some(vec![failing(8)]));
    forge.watched_by_number_are(vec![failing(9)]);
    forge.pulls_are(Some(vec![open_pull(8, Some(CiState::Passed), &[])]));
    forge.open_by_number_are(vec![open_pull(9, Some(CiState::Failed), &["ci"])]);
    let served = rig.fleet.repositories().served().remove(0);

    rig.fleet.notice_pulls(&served).await;
    let listed: Vec<u64> = rig
        .fleet
        .sweeping()
        .lock()
        .await
        .pulls
        .get(served.root())
        .expect("read")
        .iter()
        .map(|one| one.pull.number)
        .collect();
    assert_eq!(listed, [8, 9], "Bridge's list shows the held one");

    rig.reads().await;
    assert_eq!(forge.numbers_asked(), [9, 9], "once for the hub, once for the watch");
}
