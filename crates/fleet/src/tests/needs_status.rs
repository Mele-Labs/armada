//! The claim of #1059's required status: Fleet puts `needs` on every open pull
//! request. The second pull request's need is behind the first's, so it reads
//! pending naming the first; once the first is out of the way it reads success;
//! a pull request with no needs reads success; and a status is not sent again
//! while it is what the commit already carries. The forge is scripted.

use adapter_traits::{CommitStatus, FromOutside, OpenPull, StatusState};
use api::Needs as _;
use ipc::{NeedAct, NeedCall};

use super::needs::{git, holding_under, PATH};
use crate::tests::tmp::TempDir;

const FIRST: &str = "1111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222";
const THIRD: &str = "3333333333333333333333333333333333333333";

fn pull(number: u64, branch: &str, head: &str) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(branch),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        ci: None,
        head: Some(head.to_string()),
        failing: Vec::new(),
        auto_merge: false,
    }
}

fn need(act: NeedAct, branch: &str) -> NeedCall {
    NeedCall {
        act,
        manifest_id: None,
        branch: branch.to_string(),
        path: PATH.to_string(),
        what: Some("a new migration".to_string()),
        value: Some("V96".to_string()),
    }
}

fn on(statuses: &[CommitStatus], commit: &str) -> Vec<(StatusState, String)> {
    statuses
        .iter()
        .filter(|one| one.commit == commit && one.context == "needs")
        .map(|one| (one.state, one.description.clone()))
        .collect()
}

#[tokio::test]
async fn a_pull_request_reads_pending_behind_the_need_ahead_and_success_once_its_turn_comes() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    git(home.path(), &["branch", "first"]);
    git(home.path(), &["branch", "second"]);
    git(home.path(), &["branch", "third"]);
    let forge = &fleet.vcs().main_ci;
    forge.pulls_are(Some(vec![
        pull(1, "first", FIRST),
        pull(2, "second", SECOND),
        pull(3, "third", THIRD),
    ]));
    let served = fleet.repositories().served()[0].clone();

    fleet
        .act_on_need(need(NeedAct::Declare, "first"))
        .await
        .unwrap();
    fleet
        .act_on_need(need(NeedAct::Took, "first"))
        .await
        .unwrap();
    fleet
        .act_on_need(need(NeedAct::Declare, "second"))
        .await
        .unwrap();
    fleet.notice_pulls(&served).await;

    let sent = forge.statuses();
    assert_eq!(on(&sent, FIRST).last().unwrap().0, StatusState::Success);
    let (state, said) = on(&sent, SECOND).pop().expect("a status on the second");
    assert_eq!(state, StatusState::Pending);
    assert!(
        said.contains("first") && said.contains(PATH) && said.contains("V96"),
        "names who is ahead, on what, and what it took: {said}"
    );
    assert_eq!(
        on(&sent, THIRD).pop().unwrap().0,
        StatusState::Success,
        "no needs at all"
    );

    // Nothing moved, so nothing is sent again.
    let before = forge.statuses().len();
    fleet.notice_pulls(&served).await;
    assert_eq!(
        forge.statuses().len(),
        before,
        "unchanged is not republished"
    );

    // The first gives its need back: the second's turn is its own, with no sweep.
    fleet
        .act_on_need(need(NeedAct::Release, "first"))
        .await
        .unwrap();
    assert_eq!(
        on(&forge.statuses(), SECOND).pop().unwrap().0,
        StatusState::Success
    );
    assert_eq!(
        on(&forge.statuses(), SECOND).len(),
        2,
        "pending, then success"
    );
}

#[tokio::test]
async fn a_forge_that_refuses_is_carried_on_from_and_the_next_sweep_heals_it() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    git(home.path(), &["branch", "first"]);
    let forge = &fleet.vcs().main_ci;
    forge.pulls_are(Some(vec![pull(1, "first", FIRST)]));
    forge.refuses_statuses(Some("Resource not accessible by personal access token"));
    let served = fleet.repositories().served()[0].clone();

    fleet.notice_pulls(&served).await;
    assert!(forge.statuses().is_empty());

    forge.refuses_statuses(None);
    fleet.notice_pulls(&served).await;
    assert_eq!(on(&forge.statuses(), FIRST).len(), 1);
}
