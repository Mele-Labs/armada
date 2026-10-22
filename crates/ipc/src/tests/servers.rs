//! A server, as the stream, the list and the Drone's tool spell it.

use core_model::Timestamp;

use crate::{
    decode, encode, Cursor, Delivered, Event, Instant, ManifestId, ServerCheckout,
    ServerLink, ServerPhase, ServerPort, ServerState, StartServer, StartedBy, StreamMessage,
};

fn at(text: &str) -> Instant {
    Instant::from(&Timestamp::from_rfc3339(text))
}

fn serving() -> ServerState {
    ServerState {
        id: "01SERVER".to_string(),
        name: "storybook".to_string(),
        manifest_id: Some(ManifestId::carried("01MF")),
        checkout: ServerCheckout {
            path: "/repos/armada".to_string(),
            branch: None,
            commit: None,
            behind: None,
        },
        phase: ServerPhase::Serving,
        serve: "storybook dev -p 41207".to_string(),
        ports: vec![ServerPort {
            name: "storybook".to_string(),
            port: 41207,
        }],
        links: vec![ServerLink {
            url: "http://localhost:41207".to_string(),
            name: Some("Storybook".to_string()),
        }],
        started_by: StartedBy::Person,
        session_id: None,
        started_at: at("2026-09-11T10:00:00.000Z"),
        serving_since: Some(at("2026-09-11T10:00:04.000Z")),
        ended_at: None,
        exit_code: None,
        ended: None,
        stopped: false,
        log: ".armada/servers/main/01SERVER/output.log".to_string(),
    }
}

/// Three kinds, dotted as `operations/` keys them, each carrying the row
/// whole.
#[test]
fn the_three_lifecycle_kinds_carry_the_instance_whole() {
    let spelled = encode(&StreamMessage::Event(Delivered {
        cursor: Cursor::at(4),
        event: Event::ServerServing(serving()),
    }))
    .expect("plain data");
    assert!(spelled.contains(r#""kind":"server.serving""#), "{spelled}");
    assert!(spelled.contains(r#""phase":"serving""#), "{spelled}");
    assert!(spelled.contains(r#""started_by":"person""#), "{spelled}");
    assert!(spelled.contains(r#""manifest_id":"01MF""#), "{spelled}");
    let back: StreamMessage = decode("a stream message", spelled.as_bytes()).expect("reads");
    assert!(matches!(
        back,
        StreamMessage::Event(Delivered { event: Event::ServerServing(state), .. }) if state == serving()
    ));

    let spelled = encode(&Event::ServerStarting(serving())).expect("plain data");
    assert!(spelled.contains(r#""kind":"server.starting""#), "{spelled}");

    // A Fleet that predates the repository says none, and still reads.
    let unnamed = ServerState {
        manifest_id: None,
        ..serving()
    };
    let spelled = encode(&unnamed).expect("plain data");
    assert!(!spelled.contains("manifest_id"), "{spelled}");
    let back: ServerState = decode("a server", spelled.as_bytes()).expect("reads");
    assert_eq!(back, unnamed);
}

/// The body names a server and, optionally, a checkout. **No port field exists
/// to send**, so a caller cannot choose one.
#[test]
fn a_start_names_the_server_and_perhaps_a_checkout() {
    let asked: StartServer =
        decode("a server to start", br#"{"name":"storybook"}"#).expect("reads");
    assert_eq!(asked.checkout, None);
    let asked: StartServer = decode(
        "a server to start",
        br#"{"name":"storybook","checkout":"/repos/armada-2"}"#,
    )
    .expect("reads");
    assert_eq!(asked.checkout.as_deref(), Some("/repos/armada-2"));
}
