//! Fleet's notices to a Session carry a kind word on the thread's row, whichever of the two
//! sends wrote them: one that asks something of the agent and one that is only news. The words are
//! what a person names a notice by; the text of a notice is the senders' own tests'.

use std::sync::Arc;

use adapter_traits::{CiState, FromOutside, RecentlyMerged};
use api::HostedSessions;
use ipc::{SessionId, SessionRow};
use store::{AttachmentState, Holder};

use super::main_ci::{run, ONE};
use super::pull_notices::{a_rig, failing, Rig};

async fn notices_of(rig: &Rig, id: &str) -> Vec<Option<String>> {
    let rows = Arc::clone(&rig.fleet).get_session(SessionId::carried(id)).await.unwrap().rows;
    rows.into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice, .. } => Some(notice),
            _ => None,
        })
        .collect()
}

/// **A pull request whose checks failed is told with its own word.**
#[tokio::test]
async fn a_failed_check_is_told_as_pr_checks_failed() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "FAIL tests::one\n");
    forge.followed_are(Some(vec![failing(7)]));

    rig.reads().await;

    assert_eq!(notices_of(&rig, &session).await, vec![Some(String::from("pr_checks_failed"))]);
}

/// **A merge is news, sent the other way, and has its word too.**
#[tokio::test]
async fn a_merge_is_told_as_pr_merged() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(1).await;
    rig.holds(Holder::session(&session), "pr", "1", AttachmentState::Spent).await;
    rig.holds(Holder::session(&session), "branch", "armada/1", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    rig.fleet.vcs().main_ci.recently_merged_are(Some(vec![RecentlyMerged {
        number: 1,
        title: FromOutside::verbatim("Fix the reader"),
        branch: FromOutside::verbatim("armada/1"),
        url: FromOutside::verbatim("https://forge.invalid/armada/pull/1"),
        author: None,
        merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
        commit: None,
    }]));
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_merged(&served).await;

    rig.reads().await;

    assert_eq!(notices_of(&rig, &session).await, vec![Some(String::from("pr_merged"))]);
}
