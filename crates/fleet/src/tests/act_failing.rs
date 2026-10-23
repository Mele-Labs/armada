//! A background act that failed after its reply is told on the stream, with the refusal whole.

use std::time::Duration;

use api::{Next, Refusal};
use ipc::{ActFailed, BackgroundAct, Event, ManifestId, SessionId, WireError};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_failed_background_act_is_published_with_its_refusal() {
    let home = TempDir::new();
    let fleet: Fleet<FakeHarness, FakeVcs, FakeWorkProduct> = Fleet::assembled(fittings(&home, FakeWorkProduct::changed(&[])));
    let mut watching = fleet.events().subscribe();
    let error = WireError::raised("fleet.push_refused", "The push was refused", fleet.run_id());

    fleet.act_failed(
        BackgroundAct::CheckoutToPullRequest,
        Some(ManifestId::carried("01MF")),
        Some(SessionId::carried("s1")),
        Refusal::Fault(error.clone()),
    );

    let told = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => return delivered.event,
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("an act.failed arrives");
    assert_eq!(
        told,
        Event::ActFailed(ActFailed {
            act: BackgroundAct::CheckoutToPullRequest,
            manifest_id: Some(ManifestId::carried("01MF")),
            session_id: Some(SessionId::carried("s1")),
            error,
        })
    );
}
