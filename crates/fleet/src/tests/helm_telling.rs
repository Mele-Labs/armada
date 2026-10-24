//! Helm passing words to a live Session (`tell_session`): the send is the one Bridge's composer
//! makes, and the Helm Session's `sent` row and `helm.thought_routed` follow it and only it.
//! `docs/concepts/helm.md`, *Routing a thought*.
//!
//! **The Session is real and the process behind it is the stand-in**, as `session_host` has it, so
//! a hosted Session takes its turn and a terminal one is held for a mod that is or is not asking.

use std::sync::Arc;
use std::time::Duration;

use api::{Conversations, HostedSessions, Next, Sessions, Subscription};
use ipc::{
    ManifestId, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, StartSession,
    TakeHeld, TellSession, ThoughtKind,
};

use super::session_host::{rig_placing, Rig};

const TERMINAL: &str = "aaaa-2222";

impl Rig {
    fn manifest_id(&self) -> ManifestId {
        let one = self.fleet.repositories().first().expect("a served repository");
        ManifestId::carried(one.manifest().id().as_str())
    }

    async fn titled_hosted(&self, title: &str) -> SessionId {
        self.fleet
            .start_session(StartSession {
                manifest_id: self.manifest_id(),
                title: Some(title.into()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
            })
            .await
            .expect("started")
            .id
    }

    async fn terminal(&self, title: &str) -> SessionId {
        let id = SessionId::carried(TERMINAL);
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: id.clone(),
                fact: SessionFact::Started {
                    cwd: self.root.clone(),
                    title: Some(title.into()),
                    origin: SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .expect("a terminal session starts");
        id
    }

    /// The mod asking once, so the terminal session counts as listening.
    async fn mod_listens(&self) -> Vec<String> {
        self.fleet
            .take_held_messages(TakeHeld {
                session_id: TERMINAL.into(),
                wait_ms: None,
            })
            .await
            .expect("an ask")
            .messages
    }

    async fn tells(&self, id: &SessionId, text: &str) -> Result<ipc::ThoughtTarget, String> {
        Arc::clone(&self.fleet)
            .tell_session(
                TellSession {
                    session_id: id.clone(),
                    text: text.into(),
                },
                None,
            )
            .await
            .map_err(|why| format!("{why:?}"))
    }

    /// The rows of the Helm Session's thread: none where the repository has no Helm Session.
    async fn thread(&self) -> Vec<SessionRow> {
        Arc::clone(&self.fleet)
            .get_helm_session(None)
            .await
            .expect("read")
            .session
            .map(|thread| thread.rows)
            .unwrap_or_default()
    }
}

async fn routed(watching: &mut Subscription) -> Vec<ipc::HelmThoughtRouted> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(50), watching.next()).await
    {
        if let ipc::Event::HelmThoughtRouted(one) = delivered.event {
            seen.push(one);
        }
    }
    seen
}

/// A hosted Session takes the words as a turn, and Helm's thread keeps a row naming it and the
/// repository, with the event beside it.
#[tokio::test]
async fn words_told_to_a_hosted_session_take_a_turn_and_are_a_row_and_an_event() {
    let rig = rig_placing(None);
    let id = rig.titled_hosted("Walk variations").await;
    let mut watching = rig.fleet.events().subscribe();

    let target = rig.tells(&id, "make the take names shorter").await.expect("told");

    assert_eq!(target.kind, ThoughtKind::Session);
    assert_eq!(target.id, id.as_str());
    assert_eq!(target.title, "Walk variations");
    assert_eq!(target.manifest_id, Some(rig.manifest_id()));
    let rows = rig.rows(&id).await;
    assert!(
        rows.iter().any(|row| format!("{row:?}").contains("make the take names shorter")),
        "the Session was sent the words: {rows:?}"
    );
    let thread = rig.thread().await;
    let [SessionRow::Sent { text, target: went, .. }] = &thread[..] else {
        panic!("one sent row and nothing asked of Helm: {thread:?}");
    };
    assert_eq!(text, "make the take names shorter");
    assert_eq!(*went, target);
    let events = routed(&mut watching).await;
    let [event] = events.as_slice() else {
        panic!("one routed event: {events:?}");
    };
    assert_eq!((event.to, event.id.as_str()), (ThoughtKind::Session, id.as_str()));
}

/// A terminal Session whose mod is asking is held the words, as a person's send is.
#[tokio::test]
async fn words_told_to_a_listening_terminal_session_are_held_for_its_mod() {
    let rig = rig_placing(None);
    let id = rig.terminal("Release notes").await;
    assert!(rig.mod_listens().await.is_empty(), "nothing yet, and now it is listening");

    rig.tells(&id, "group them by pull request").await.expect("told");

    assert_eq!(rig.mod_listens().await, vec!["group them by pull request"]);
    assert!(matches!(&rig.thread().await[..], [SessionRow::Sent { .. }]));
}

/// **A refused send records nothing**: not the row, not the event, and the refusal is the send's
/// own, in its own words.
#[tokio::test]
async fn a_terminal_session_not_listening_is_refused_and_nothing_is_recorded() {
    let rig = rig_placing(None);
    let id = rig.terminal("Release notes").await;
    let mut watching = rig.fleet.events().subscribe();

    let refused = rig.tells(&id, "group them by pull request").await.unwrap_err();

    assert!(refused.contains("fleet.terminal_session_unreachable"), "{refused}");
    assert!(rig.thread().await.is_empty(), "no row");
    assert!(routed(&mut watching).await.is_empty(), "no event");
}

/// A Session that does not exist is the send's refusal too, and records nothing.
#[tokio::test]
async fn a_session_that_does_not_exist_is_refused_and_nothing_is_recorded() {
    let rig = rig_placing(None);
    let mut watching = rig.fleet.events().subscribe();

    let refused = rig.tells(&SessionId::carried("01NOSUCHSESSION"), "hello").await;

    assert!(refused.is_err(), "{refused:?}");
    assert!(rig.thread().await.is_empty(), "no row");
    assert!(routed(&mut watching).await.is_empty(), "no event");
}
