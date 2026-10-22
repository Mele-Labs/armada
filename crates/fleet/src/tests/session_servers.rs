//! A server knows the Session whose agent started it, and the Session's live view is the newest
//! page it showed or served. `docs/concepts/session/working-on.md`.
//!
//! **The server is a real process, `sleep`**, because what is asserted is the row Fleet publishes for
//! it and not what it prints.

use std::path::Path;
use std::sync::Arc;

use api::Commands;
use config::Manifest;
use ipc::{SessionId, StartServer};

use super::session_host::{rig_fitted, Rig};

/// `web` serves, and its link is where the page is.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  web: {}
commands:
  web:
    serve: "/bin/sh -c 'sleep 60'"
    links:
      - url: http://localhost:${port.web}
        name: The page
"#;

/// A rig whose repository declares the `web` server, on a span of its own.
fn rig_serving() -> Rig {
    rig_fitted(|fittings| {
        fittings.starting().manifest = Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
        fittings.port_range = super::servers::a_range_of_its_own();
    })
}

fn web() -> StartServer {
    StartServer { name: String::from("web"), checkout: None }
}

/// A Session with a process running, whose agent a call placed by `agent_starts::caller` is.
async fn running(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "serve the page").await;
    rig.stand_in.init(0);
    id
}

#[tokio::test]
async fn a_server_started_from_a_session_names_it_and_one_started_by_hand_names_none() {
    let rig = rig_serving();
    let id = running(&rig).await;

    let by_agent = Arc::clone(&rig.fleet)
        .start_server(super::agent_starts::caller(), web(), None)
        .await
        .expect("it starts");
    assert_eq!(by_agent.session_id.as_ref(), Some(&id), "the agent's connection places the Session");
    rig.fleet.stopped_server(&by_agent.id).await.expect("it stops");

    let by_hand = Arc::clone(&rig.fleet)
        .start_server(None, web(), None)
        .await
        .expect("it starts");
    assert_eq!(by_hand.session_id, None, "a call from no Session names none");
    rig.fleet.stopped_server(&by_hand.id).await.expect("it stops");
}
