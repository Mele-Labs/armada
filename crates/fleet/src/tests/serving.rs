//! The served operations, over the real router, answered by a real Fleet.
//!
//! `api`'s own suite proves the same routes against a fake daemon. This one
//! proves the claim that fake was standing in for: **the operations answer from
//! Fleet**, over the router that ships, with no socket — a `Router` is a
//! `Service`, and a `Service` does not need a port to be called.
//!
//! Nothing here reaches past the transport. What comes back is read with
//! `ipc::decode`, which is both the only sanctioned way to turn bytes into a
//! typed value outside `store` and the assertion that the redaction at the
//! boundary produced something a Bridge could read.

use std::sync::Arc;

use axum::http::StatusCode;
use ipc::RunId;
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

/// `search_files` served from a real Fleet. `crate::files::search` is proven
/// as a pure walk in its own module, and `api`'s suite proves the route
/// against a fake daemon — neither proves the one line between them, that
/// `Fleet::search_files` walks `Host::repo_root`, the checkout this Fleet
/// actually serves, and not some other path.
#[tokio::test]
async fn search_files_reads_the_repository_fleet_serves() {
    let home = TempDir::new();
    std::fs::write(home.path().join("README.md"), b"").expect("a file");
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let (status, body) = call(&app, "GET", "/manifest/files?q=read", "").await;

    assert_eq!(status, StatusCode::OK);
    let found: ipc::FilesFound = ipc::decode("files found", &body).expect("a list of paths");
    assert_eq!(found.paths, vec!["README.md".to_string()]);
}

/// `read_request_repository` served from a real Fleet: the repository it serves
/// is the one meant, by the route Bridge calls, with the words in the query.
#[tokio::test]
async fn a_request_is_read_against_the_repositories_fleet_serves() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let (status, body) = call(&app, "GET", "/repositories/of_request?q=fix%20the%20reader", "").await;

    assert_eq!(status, StatusCode::OK);
    let read: ipc::RequestRepository = ipc::decode("a request's repository", &body).expect("a reading");
    assert_eq!(
        read.manifest_id.as_ref().map(|id| id.as_str()),
        Some("01FIXTUREMANIFEST"),
        "the only repository served"
    );
    assert_eq!(read.likely.len(), 1);
}
