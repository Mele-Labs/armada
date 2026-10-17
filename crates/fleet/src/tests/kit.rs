//! What a Drone is handed, once Kit holds servers — `#1275`.
//!
//! The subject is the document a spawn writes, not the table: `crate::kit`
//! resolves and `crate::spawning` writes, and the one thing worth proving is
//! that the two agree with what `get_kit_servers` draws.

use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn adding(name: &str) -> ipc::AddKitServer {
    ipc::AddKitServer {
        name: name.to_string(),
        address: ipc::ServerAddress::Http {
            url: format!("https://{name}.test/mcp"),
        },
    }
}

/// The document the next Drone was spawned against, read off the spawn the
/// harness recorded rather than off a path this test composed.

/// The list as a person reads it, for the Manifest the fixture serves.
async fn listed(fleet: &Fixture) -> Vec<ipc::KitServerRow> {
    let served = fleet.served_named(None).expect("one repository");
    fleet.kit_servers_listed(&served).await.servers
}

/// What a person cannot add. Each is a 409 rather than a row nothing can spawn
/// against.
#[tokio::test]
async fn a_name_and_an_address_are_refused_where_a_drone_could_not_use_them() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));

    for refused in ["", "two words", "dotted.name"] {
        assert!(
            fleet.add_kit_server(adding(refused)).await.is_err(),
            "`{refused}` is not a name a JSON key and a tool name spell alike"
        );
    }
    assert!(
        fleet.add_kit_server(adding("armada")).await.is_err(),
        "a second `armada` would leave a Drone's brief naming two servers"
    );
    assert!(
        fleet
            .add_kit_server(ipc::AddKitServer {
                name: "nexus".to_string(),
                address: ipc::ServerAddress::Http {
                    url: "nexus.test".to_string()
                },
            })
            .await
            .is_err(),
        "a bare host is not an address"
    );
    assert!(
        fleet
            .forget_kit_server(ipc::ForgetKitServer {
                name: "nexus".to_string()
            })
            .await
            .is_err(),
        "Kit holds no `nexus`"
    );
    assert!(listed(&fleet).await.is_empty());
}
