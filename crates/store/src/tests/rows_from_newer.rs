//! A row a newer build wrote with a value this build does not have is left out of the list it is in, and
//! the list reads every other row: after a revert the build before reads what it knows. A Manifest's word
//! over a Kit server it cannot read is the narrower one, never Kit's wider default.

use core_model::{Actor, AllowedCommand, EdgeGrain, EdgeSource, KitServer, ManifestId, ManifestReach, ServerAddress, ServerName, TestMapEdge, Timestamp, Ulid};

use crate::tests::{at, open, TempDir};

fn manifest() -> ManifestId {
    ManifestId::carried(Ulid::carried("01REPO"))
}

fn server(name: &str) -> KitServer {
    KitServer {
        name: ServerName::named(name).expect("a name"),
        address: ServerAddress::program("npx", &["-y".to_string(), name.to_string()]).expect("an address"),
        drones: core_model::ReachesDrones::No,
        added_at: at("2026-11-06T00:00:00Z"),
        by: Actor::Human,
    }
}

#[test]
fn a_kit_server_with_a_newer_reach_or_transport_is_left_out_and_the_rest_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for name in ["alpha", "beta", "gamma"] {
        store.put_kit_server(&server(name)).expect("put");
    }
    store.conn.execute("UPDATE kit_mcp_servers SET drones = 'some_drones' WHERE name = 'alpha'", []).expect("a newer reach");
    store.conn.execute("UPDATE kit_mcp_servers SET transport = 'sse' WHERE name = 'gamma'", []).expect("a newer transport");

    let names: Vec<String> = store.kit_servers().expect("reads").into_iter().map(|one| one.name.as_str().to_string()).collect();
    assert_eq!(names, ["beta"]);
}

#[test]
fn a_manifests_newer_word_over_a_server_reads_as_restricted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.put_kit_server(&server("alpha")).expect("put");
    store.set_manifest_server_reach(&manifest(), &ServerName::named("alpha").expect("a name"), Some(ManifestReach::Extended), &at("2026-11-06T00:00:00Z"), Actor::Human).expect("said");
    store.conn.execute("UPDATE manifest_mcp_servers SET reach = 'extended_for_judges'", []).expect("a newer word");

    let reaches = store.manifest_server_reaches(&manifest()).expect("reads");
    assert_eq!(reaches, [(ServerName::named("alpha").expect("a name"), ManifestReach::Restricted)]);
}

#[test]
fn an_allowed_command_a_newer_actor_allowed_is_left_out_and_the_rest_read() {
    let dir = TempDir::new();
    let store = open(&dir);
    for (run, by) in [("cargo build", "human"), ("pnpm test", "a_future_actor")] {
        store
            .conn
            .execute(
                "INSERT INTO manifest_allowed_commands (manifest_id, run, allowed_at, \"by\") VALUES (?1, ?2, ?3, ?4)",
                (manifest().as_str(), run, "2026-11-06T00:00:00Z", by),
            )
            .expect("written");
    }

    let allowed = store.repository_allowed_commands(&manifest()).expect("reads");
    assert_eq!(allowed, [AllowedCommand { run: "cargo build".into(), allowed_at: at("2026-11-06T00:00:00Z"), by: Actor::Human }]);
}

#[test]
fn a_test_map_edge_of_a_newer_grain_is_left_out_and_the_rest_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let edge = |test: &str| TestMapEdge {
        repository: manifest(),
        check: "test".into(),
        member: "store".into(),
        test: test.into(),
        file: "src/row.rs".into(),
        source: EdgeSource::Measured,
        grain: EdgeGrain::Test,
        commit: "abc123".into(),
        at: Timestamp::from_rfc3339("2026-11-06T00:00:00Z".to_string()),
        stale: false,
    };
    store.record_test_map_edges(&[edge("reads_a_row"), edge("reads_a_column")]).expect("recorded");
    store.conn.execute("UPDATE test_map_edges SET grain = 'branch' WHERE test = 'reads_a_column'", []).expect("a newer grain");

    let reaching = store.tests_reaching(&manifest(), &["src/row.rs".to_string()]).expect("reads");
    assert_eq!(reaching.edges.iter().map(|one| one.test.as_str()).collect::<Vec<_>>(), ["reads_a_row"]);
    let reached = store.files_a_test_reaches(&manifest(), "test", "store", "reads_a_column").expect("reads");
    assert!(reached.is_empty());
}
