//! The web links a Session's person and agent write, kept as `link` rows on the ledger and kept
//! after the Session ends. `docs/concepts/session/what-is-kept.md`.

use std::sync::Arc;

use adapter_traits::{DroneEvent, Speaker};
use api::{HostedSessions, Sessions};
use ipc::{AttachmentState, CloseSession, SessionId};

use super::session_host::{eventually, Rig};
use super::session_issues::{a_rig, said};

const DOCS: &str = "https://docs.example.com/guide";

/// Every `link` row the Session holds.
async fn links(rig: &Rig, id: &SessionId) -> Vec<ipc::Attachment> {
    let listed = rig.fleet.list_sessions(None, Some(id.as_str().into()), None).await.expect("listed");
    let session = listed.sessions.into_iter().find(|one| one.id == *id).expect("the session");
    session.attachments.into_iter().filter(|one| one.kind == "link").collect()
}

fn by(row: &ipc::Attachment) -> Option<&str> {
    row.detail.get("by").map(String::as_str)
}

/// The person's message starts a turn, and the agent's reply to it is heard the way a Drone's words are.
async fn agent_says(rig: &Rig, id: &SessionId, asked: &str, text: &str) {
    rig.send(id, asked).await;
    rig.stand_in.init(0);
    eventually(|| async { rig.turn(id).await != ipc::SessionTurn::Idle }).await;
    rig.stand_in.says(0, vec![DroneEvent::Said { text: text.into(), by: Speaker::Drone }]);
}

#[tokio::test]
async fn a_link_the_person_writes_is_kept_as_theirs_and_one_to_this_machine_or_this_repository_is_not() {
    let rig = a_rig();
    let id = rig.start().await;

    said(&rig, &id, &format!("Read {DOCS}, then http://localhost:5361/?walk=x and https://forge.invalid/o/r/pull/12"), vec![]).await;

    let rows = links(&rig, &id).await;
    assert_eq!(rows.iter().map(|one| one.target.as_str()).collect::<Vec<_>>(), vec![DOCS]);
    assert_eq!(by(&rows[0]), Some("you"));
}

#[tokio::test]
async fn the_request_a_session_is_dispatched_with_keeps_its_links_as_the_persons() {
    let rig = a_rig();

    let record = rig.start_requesting(&format!("Follow {DOCS}"), |_| {}).await;

    let rows = links(&rig, &record.id).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!((rows[0].target.as_str(), by(&rows[0])), (DOCS, Some("you")));
}

#[tokio::test]
async fn a_link_the_agent_writes_in_its_reply_is_kept_as_the_agents() {
    let rig = a_rig();
    let id = rig.start().await;

    agent_says(&rig, &id, "go", &format!("See [the guide]({DOCS}) and http://127.0.0.1:3000/")).await;

    eventually(|| async { !links(&rig, &id).await.is_empty() }).await;
    let rows = links(&rig, &id).await;
    assert_eq!(rows.iter().map(|one| one.target.as_str()).collect::<Vec<_>>(), vec![DOCS]);
    assert_eq!(by(&rows[0]), Some("agent"));
}

#[tokio::test]
async fn a_link_already_held_keeps_the_voice_that_wrote_it_first() {
    let rig = a_rig();
    let id = rig.start().await;

    agent_says(&rig, &id, DOCS, &format!("Read {DOCS} and https://other.example.com/")).await;

    eventually(|| async { links(&rig, &id).await.len() == 2 }).await;
    let rows = links(&rig, &id).await;
    let voice = |target: &str| rows.iter().find(|one| one.target == target).and_then(by);
    assert_eq!(voice(DOCS), Some("you"));
    assert_eq!(voice("https://other.example.com/"), Some("agent"));
}

#[tokio::test]
async fn a_link_stays_kept_after_the_session_ends() {
    let rig = a_rig();
    let id = rig.start().await;
    said(&rig, &id, DOCS, vec![]).await;

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");

    let rows = links(&rig, &id).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].state, AttachmentState::Spent);
}
