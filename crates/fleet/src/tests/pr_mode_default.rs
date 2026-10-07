//! The default a pull request opens by, served before the approval and kept at
//! it: `crates/acceptance/tests/triggers.rs` holds the claim, and this is the
//! machine's tier and the press with no body, which need the store.

use std::sync::Arc;

use api::{Commands, Queries};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fittings, one, two_steps_gated_on_a_manifest_rule};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Arc<Fixture> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_manifest_rule(
        "summarise",
        "auto_merge",
        Some("summarise"),
    ));
    Arc::new(Fleet::assembled(fittings))
}

fn body(json: &str) -> ipc::ApproveDispatch {
    ipc::decode("an approval", json.as_bytes()).expect("decodes")
}

/// **This machine's default reaches a Job nobody said anything about**: the
/// approval serves it for Bridge to start on, and a press with no body freezes
/// it. A Job's own choice at approval still beats it.
#[tokio::test]
async fn a_machine_that_drafts_serves_it_and_freezes_it_unless_the_job_chose_otherwise() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let before = fleet
        .propose(a_proposal("the reader"))
        .await
        .expect("a Job");
    let served = Queries::get_job(&*fleet, before.id().into())
        .await
        .expect("the detail");
    assert_eq!(served.pr_mode_default, Some(mode("ready")));

    fleet
        .save_preferences(ipc::SavePreference {
            name: "draft_pull_requests".to_string(),
            value: true,
        })
        .await
        .expect("saved");
    let served = Queries::get_job(&*fleet, before.id().into())
        .await
        .expect("the detail");
    assert_eq!(served.pr_mode_default, Some(mode("draft")));

    Commands::approve_dispatch(Arc::clone(&fleet), before.id().into(), None)
        .await
        .expect("approved");
    let approved = Queries::get_job(&*fleet, before.id().into())
        .await
        .expect("the detail");
    assert_eq!(
        approved.landing.map(|landing| landing.pr_mode),
        Some(mode("draft"))
    );
    assert_eq!(approved.pr_mode_default, None, "the landing says now");

    let chosen = fleet
        .propose(a_proposal("the writer"))
        .await
        .expect("a Job");
    fleet
        .approve_as_left(chosen.id(), &body(r#"{"landing": {"pr_mode": "ready"}}"#))
        .await
        .expect("approved");
    let approved = Queries::get_job(&*fleet, chosen.id().into())
        .await
        .expect("the detail");
    assert_eq!(
        approved.landing.map(|landing| landing.pr_mode),
        Some(mode("ready"))
    );
}

fn mode(wire: &str) -> ipc::PrMode {
    ipc::PrMode::from_wire(wire).expect("a mode")
}
