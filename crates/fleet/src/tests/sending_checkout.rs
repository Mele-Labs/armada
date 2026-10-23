//! A held checkout's files sent up as a pull request answer when the send is accepted, and the
//! commit, the push and the pull request finish after the reply: the hub shows the pull request
//! when they have, and a refusal is told on the stream as `act.failed`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::HeldBack;
use api::{Commands, Next};
use ipc::{BackgroundAct, CheckoutPullRequest, CheckoutToPullRequest, Event};
use testkit::Hold;

use super::pull_requesting::{a_fleet, manifest, Fixture};
use crate::tests::tmp::TempDir;

fn held() -> HeldBack {
    HeldBack {
        base: "main".into(),
        on: None,
        uncommitted: vec!["src/log.rs".into()],
        clashing: Vec::new(),
        already_on_base: Vec::new(),
        behind: 1,
        ahead: 0,
    }
}

/// A Fleet whose checkout is held back, as the noticing sweep would have read it.
async fn holding_a_checkout(home: &TempDir) -> (Arc<Fixture>, ipc::ManifestId, String) {
    let fleet = Arc::new(a_fleet(home));
    let (id, root) = manifest(&fleet);
    fleet.vcs().held_back_by(Some(held()));
    fleet.sweeping().lock().await.held_back.insert(root.clone(), held());
    (fleet, id, root)
}

async fn pull_request_on_the_hub(fleet: &Fixture, root: &str) -> Option<CheckoutPullRequest> {
    let hubs = fleet.merge_hubs().await;
    hubs.into_iter().find(|(at, _)| at == root).and_then(|(_, hub)| hub.checkout?.pull_request)
}

#[tokio::test]
async fn the_reply_comes_back_while_the_push_is_held_and_the_hub_then_shows_the_pull_request() {
    let home = TempDir::new();
    let (fleet, id, root) = holding_a_checkout(&home).await;
    let hold: Arc<Hold> = fleet.vcs().hold_checkout_sends();

    let asked = CheckoutToPullRequest { manifest_id: id };
    tokio::time::timeout(Duration::from_secs(3), Arc::clone(&fleet).checkout_to_pull_request(asked))
        .await
        .expect("the reply waited on the push")
        .expect("accepted");

    assert_eq!(pull_request_on_the_hub(&fleet, &root).await, None, "the push has not gone");
    hold.open();
    super::session_host::eventually(|| async { pull_request_on_the_hub(&fleet, &root).await.is_some() }).await;
    let shown = pull_request_on_the_hub(&fleet, &root).await.expect("shown");
    assert_eq!(shown.number, 2402);
    assert_eq!(fleet.vcs().checkouts_sent().len(), 1);
}

#[tokio::test]
async fn a_refused_push_is_told_as_act_failed_and_the_hub_shows_no_pull_request() {
    let home = TempDir::new();
    let (fleet, id, root) = holding_a_checkout(&home).await;
    fleet.vcs().now_sending_the_checkout_is_refused("remote: permission denied");
    let mut watching = fleet.events().subscribe();

    let asked = CheckoutToPullRequest { manifest_id: id.clone() };
    Arc::clone(&fleet).checkout_to_pull_request(asked).await.expect("accepted");

    let told = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::ActFailed(failed) = delivered.event {
                        return failed;
                    }
                }
                other => panic!("the stream ended: {other:?}"),
            }
        }
    })
    .await
    .expect("an act.failed arrives");
    assert_eq!(told.act, BackgroundAct::CheckoutToPullRequest);
    assert_eq!(told.manifest_id, Some(id));
    assert_eq!(told.session_id, None);
    assert_eq!(told.error.code, "fleet.checkout_not_sent");
    assert!(told.error.message.contains("permission denied"), "{}", told.error.message);
    assert_eq!(pull_request_on_the_hub(&fleet, &root).await, None);
}

#[tokio::test]
async fn nothing_to_send_is_refused_in_the_reply() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let (id, _) = manifest(&fleet);

    let refused = Arc::clone(&fleet)
        .checkout_to_pull_request(CheckoutToPullRequest { manifest_id: id })
        .await
        .unwrap_err();

    assert_eq!(refused.error().code, "fleet.checkout_not_held");
    assert!(fleet.vcs().checkouts_sent().is_empty());
}
