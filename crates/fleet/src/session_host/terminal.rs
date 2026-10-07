//! A session run in a terminal, opened in Bridge: its thread drawn from the
//! transcript its agent CLI keeps, and what a person sends it held for its mod.
//! `docs/concepts/session.md`, *A terminal session's thread*; spike 27.
//!
//! **Nothing here is inferred from the thread.** Rows are drawn and published as
//! the file grows, and the ledger's row says what the session is and holds as it
//! did before. **Fleet cannot push into a terminal**: it holds a message until
//! the session's own mod asks, and the mod submits it as the person's prompt.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{MessagesHeld, SendSessionMessage, SessionId, SessionRecord, SessionRow, SessionRowChanged};
use ipc::{TakeHeld, WireError};
use store::{KeptSession, SessionState};

use super::rows::{mention_line, SESSION_THREAD_UNREADABLE as THREAD_UNREADABLE};
use super::serving::MESSAGE_EMPTY;
use crate::daemon::Fleet;

/// A send to a session whose mod has not asked lately. A 409.
const TERMINAL_UNREACHABLE: &str = "fleet.terminal_session_unreachable";
/// A terminal session is sent words and nothing else. A 422.
const TERMINAL_TEXT_ONLY: &str = "fleet.terminal_session_text_only";

/// How long after its mod last asked a session still counts as listening. The
/// mod asks every couple of seconds.
pub(crate) const LISTENING_FOR: Duration = Duration::from_secs(10);

/// How often a thread being watched is looked at. One `stat` when nothing grew.
const LOOKS_EVERY: Duration = Duration::from_secs(1);

struct Listening {
    asked_at: Instant,
    held: Vec<String>,
}

/// What Fleet holds for terminal sessions, which lives only as long as Fleet.
#[derive(Default)]
pub struct Terminals {
    listening: Mutex<HashMap<String, Listening>>,
    watched: Mutex<HashSet<String>>,
}

impl Terminals {
    /// The mod asked: it is listening now, and takes what was held.
    fn asked(&self, session: &str) -> Vec<String> {
        let mut table = self.listening.lock().expect("held across no panic");
        let one = table.entry(session.to_string()).or_insert(Listening {
            asked_at: Instant::now(),
            held: Vec::new(),
        });
        one.asked_at = Instant::now();
        std::mem::take(&mut one.held)
    }

    /// Keep `text` for the mod, or say nobody is listening.
    fn hold(&self, session: &str, text: String, listening_for: Duration) -> bool {
        let mut table = self.listening.lock().expect("held across no panic");
        match table.get_mut(session) {
            Some(one) if one.asked_at.elapsed() <= listening_for => {
                one.held.push(text);
                true
            }
            _ => false,
        }
    }

    fn begin_watching(&self, session: &str) -> bool {
        self.watched
            .lock()
            .expect("held across no panic")
            .insert(session.to_string())
    }

    fn end_watching(&self, session: &str) {
        self.watched
            .lock()
            .expect("held across no panic")
            .remove(session);
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A terminal session's thread as the transcript stands, and from here on
    /// each line it gains as a `session.row`, until the session ends.
    pub(crate) async fn terminal_thread(
        self: &Arc<Self>,
        session: &KeptSession,
    ) -> Result<Vec<SessionRow>, Refusal> {
        let home = self.host().home.clone();
        let id = session.id.clone();
        let (file, rows, next) = tokio::task::spawn_blocking(move || {
            let file = adapters::terminal_thread::find(&home, &id);
            match file {
                Some(file) => match adapters::terminal_thread::read_from(&file, 0) {
                    Ok(thread) => (Some(file), thread.rows, thread.next),
                    Err(_) => (Some(file), Vec::new(), 0),
                },
                None => (None, Vec::new(), 0),
            }
        })
        .await
        .map_err(|why| self.hosted_fault(THREAD_UNREADABLE, &why.to_string()))?;
        if self.hosts().terminals().begin_watching(&session.id) {
            let fleet = Arc::clone(self);
            let id = session.id.clone();
            tokio::spawn(async move { fleet.watch_terminal(id, file, next).await });
        }
        Ok(rows)
    }

    async fn watch_terminal(self: Arc<Self>, id: String, mut file: Option<PathBuf>, mut at: u64) {
        loop {
            tokio::time::sleep(LOOKS_EVERY).await;
            if file.is_none() {
                file = adapters::terminal_thread::find(&self.host().home, &id);
            }
            if let Some(path) = file.clone() {
                let read = tokio::task::spawn_blocking(move || {
                    adapters::terminal_thread::read_from(&path, at).map(|thread| (thread, at))
                })
                .await;
                if let Ok(Ok((thread, _))) = read {
                    at = thread.next;
                    for row in thread.rows {
                        self.publish(ipc::Event::SessionRow(SessionRowChanged {
                            session_id: SessionId::carried(&id),
                            row,
                        }));
                    }
                }
            }
            let ended = match self.store().lock().await.session(&id) {
                Ok(Some(one)) => one.state == SessionState::Ended,
                Ok(None) => true,
                Err(_) => false,
            };
            if ended {
                break;
            }
        }
        self.hosts().terminals().end_watching(&id);
    }

    /// What a person sent a terminal session. It starts a turn when the mod
    /// next asks, so a session that is not asking is refused at once.
    pub(crate) async fn send_to_terminal(
        &self,
        session: &KeptSession,
        sent: SendSessionMessage,
        addressed: Vec<(ipc::SessionTag, Option<String>)>,
    ) -> Result<SessionRecord, Refusal> {
        if session.state == SessionState::Ended {
            return Err(self.closed(&session.id));
        }
        if !sent.attachments.is_empty() {
            return Err(self.hosted_refusal(
                TERMINAL_TEXT_ONLY,
                "a session in a terminal takes words; a file or picture is not sent to it",
            ));
        }
        let text = sent.text.trim();
        if text.is_empty() {
            return Err(self.hosted_refusal(MESSAGE_EMPTY, "a message needs words"));
        }
        let turn = match mention_line(&addressed) {
            Some(line) => format!("{line}\n\n{text}"),
            None => text.to_string(),
        };
        if !self
            .hosts()
            .terminals()
            .hold(&session.id, turn, LISTENING_FOR)
        {
            return Err(Refusal::IllegalMove(WireError::raised(
                TERMINAL_UNREACHABLE,
                "that session is not listening. It has ended, or its terminal is closed, or \
                 the armada mod is not loaded in it",
                self.run_id(),
            )));
        }
        let store = self.store().lock().await;
        self.ledger_row(&store, session)
    }

    /// The mod's ask. A session that is not a terminal one is handed nothing
    /// and not marked listening.
    pub(crate) async fn held_for(&self, ask: TakeHeld) -> Result<MessagesHeld, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .session(&ask.session_id)
            .map_err(|why| self.ledger_fault(why))?;
        Ok(match kept {
            Some(one) if one.origin == "terminal" => MessagesHeld {
                messages: self.hosts().terminals().asked(&one.id),
            },
            _ => MessagesHeld::default(),
        })
    }
}
