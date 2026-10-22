//! A server knows the Session whose agent started it, and the Session's live view is the newest
//! page it showed or served. `docs/concepts/session/working-on.md`.
//!
//! **The server is a real process, `sleep`**, because what is asserted is the row Fleet publishes for
//! it and not what it prints.

use std::path::Path;
use std::sync::Arc;

use api::{Commands, HostedSessions, Sessions};
use config::Manifest;
use ipc::{SessionId, StartServer};

use super::session_host::{eventually, rig_fitted, Rig};

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

fn shown(url: &str, title: &str) -> ipc::ShowWindow {
    let body = format!(r#"{{"url":"{url}","title":"{title}"}}"#);
    ipc::decode("a window", body.as_bytes()).expect("a window")
}

async fn live(rig: &Rig, id: &SessionId) -> Option<ipc::SessionLive> {
    let row = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    row.activity.and_then(|activity| activity.live)
}

/// **The newest page wins, whichever way it came**: shown with `show_window`, or served by a server
/// the agent started. His next message clears it, as it clears the account.
#[tokio::test]
async fn the_live_view_is_the_newest_page_the_session_showed_or_served() {
    let rig = rig_serving();
    let id = running(&rig).await;
    assert_eq!(live(&rig, &id).await, None, "nothing shown, nothing live");

    rig.fleet.show_window(super::agent_starts::caller(), shown("http://localhost:5173/mock", "The mock")).await.expect("shown");
    let first = live(&rig, &id).await.expect("a page was shown");
    assert_eq!((first.url.as_str(), first.title.as_str()), ("http://localhost:5173/mock", "The mock"));

    let server = Arc::clone(&rig.fleet).start_server(super::agent_starts::caller(), web(), None).await.expect("it starts");
    let served = live(&rig, &id).await.expect("a page was served");
    assert_eq!(served.url, format!("http://localhost:{}", server.ports[0].port), "the server's page is newer");
    assert_eq!(served.title, "The page", "named as its link is");

    rig.fleet.show_window(super::agent_starts::caller(), shown("http://localhost:5173/other", "Another")).await.expect("shown");
    assert_eq!(live(&rig, &id).await.map(|one| one.title), Some("Another".into()), "and a window shown after it is newer still");

    rig.fleet.stopped_server(&server.id).await.expect("it stops");
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    assert!(live(&rig, &id).await.is_some(), "a turn ending clears nothing");
    rig.send(&id, "now something else").await;
    rig.stand_in.init(0);
    eventually(|| async { live(&rig, &id).await.is_none() }).await;
}
