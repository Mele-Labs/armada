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
use ipc::{HeldCommand, MessagesHeld, SendSessionMessage, TerminalFacts, TuneSession, SessionId, SessionRecord, SessionRow, SessionRowChanged};
use ipc::{TakeHeld, WireError};
use store::{KeptSession, SessionState};

use super::rows::{mention_line, SESSION_THREAD_UNREADABLE as THREAD_UNREADABLE};
use super::serving::MESSAGE_EMPTY;
use crate::daemon::Fleet;

/// A send to a session whose mod has not asked lately. A 409.
const TERMINAL_UNREACHABLE: &str = "fleet.terminal_session_unreachable";

/// How long after its mod last asked a session still counts as listening. The
/// mod asks every couple of seconds.
pub(crate) const LISTENING_FOR: Duration = Duration::from_secs(10);

/// How often a thread being watched is looked at. One `stat` when nothing grew.
const LOOKS_EVERY: Duration = Duration::from_secs(1);

struct Listening {
    asked_at: Instant,
    held: Vec<String>,
    commands: Vec<HeldCommand>,
}

/// What Fleet holds for terminal sessions, which lives only as long as Fleet.
#[derive(Default)]
pub struct Terminals {
    listening: Mutex<HashMap<String, Listening>>,
    watched: Mutex<HashSet<String>>,
    tuned: Mutex<HashMap<String, TerminalFacts>>,
    /// When each standing question was last polled for, by call. Memory only: a
    /// Fleet that restarts gives every mod a fresh lapse to poll again in.
    polled: Mutex<HashMap<String, Instant>>,
    /// Rung when a question is answered or closed, for the polls holding on it.
    pub(crate) rung: tokio::sync::Notify,
}

impl Terminals {
    /// The mod asked: it is listening now, and takes what was held.
    fn asked(&self, session: &str) -> MessagesHeld {
        let mut table = self.listening.lock().expect("held across no panic");
        let one = table.entry(session.to_string()).or_insert(Listening {
            asked_at: Instant::now(),
            held: Vec::new(),
            commands: Vec::new(),
        });
        one.asked_at = Instant::now();
        MessagesHeld {
            messages: std::mem::take(&mut one.held),
            commands: std::mem::take(&mut one.commands),
        }
    }

    /// The mod polled for this question just now.
    pub(crate) fn polled(&self, call: &str) {
        self.polled
            .lock()
            .expect("held across no panic")
            .insert(call.to_string(), Instant::now());
    }

    /// How long since the mod polled for this question. **A question nobody has
    /// polled for since Fleet began counts from now.**
    pub(crate) fn since_polled(&self, call: &str) -> Duration {
        self.polled
            .lock()
            .expect("held across no panic")
            .entry(call.to_string())
            .or_insert_with(Instant::now)
            .elapsed()
    }

    pub(crate) fn forget_polls(&self, call: &str) {
        self.polled.lock().expect("held across no panic").remove(call);
    }

    /// Keep a command for the mod, or say nobody is listening.
    fn hold_command(&self, session: &str, command: HeldCommand, listening_for: Duration) -> bool {
        let mut table = self.listening.lock().expect("held across no panic");
        match table.get_mut(session) {
            Some(one) if one.asked_at.elapsed() <= listening_for => {
                one.commands.push(command);
                true
            }
            _ => false,
        }
    }

    /// Whether the session's mod has asked within [`LISTENING_FOR`].
    pub(crate) fn listening(&self, session: &str) -> bool {
        self.listening
            .lock()
            .expect("held across no panic")
            .get(session)
            .is_some_and(|one| one.asked_at.elapsed() <= LISTENING_FOR)
    }

    /// What the session's mod said it runs on.
    pub(crate) fn facts_of(&self, session: &str) -> Option<TerminalFacts> {
        self.tuned
            .lock()
            .expect("held across no panic")
            .get(session)
            .cloned()
    }

    /// Fold what the mod said into what is held. **A field left out is
    /// unchanged.** True where anything changed.
    pub(crate) fn tuned(&self, session: &str, said: TerminalFacts) -> bool {
        let mut table = self.tuned.lock().expect("held across no panic");
        let held = table.entry(session.to_string()).or_default();
        let before = held.clone();
        held.model = said.model.or(held.model.take());
        held.effort = said.effort.or(held.effort.take());
        held.mode = said.mode.or(held.mode.take());
        if !said.commands.is_empty() {
            held.commands = said.commands;
        }
        *held != before
    }

    /// Keep `text` for the mod, or say nobody is listening.
    pub(super) fn hold(&self, session: &str, text: String, listening_for: Duration) -> bool {
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
        let mut was = self.hosts().terminals().listening(&id);
        loop {
            tokio::time::sleep(LOOKS_EVERY).await;
            // A mod that stops asking is not an event, so it is noticed here:
            // the record says `listening`, and a window drawing it must be told
            // when that changes.
            let now = self.hosts().terminals().listening(&id);
            if now != was {
                was = now;
                let _ = self.published_hosted(&id).await;
            }
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
        paths: Vec<String>,
    ) -> Result<SessionRecord, Refusal> {
        if session.state == SessionState::Ended {
            return Err(self.closed(&session.id));
        }
        let text = sent.text.trim();
        if text.is_empty() && paths.is_empty() {
            return Err(self.hosted_refusal(
                MESSAGE_EMPTY,
                "a message needs words or something sent with it",
            ));
        }
        let mut turn = match mention_line(&addressed) {
            Some(line) => format!("{line}\n\n{text}"),
            None => text.to_string(),
        };
        for path in &paths {
            turn.push_str(&format!("\n\nAttached file: {path}"));
        }
        if !self
            .hosts()
            .terminals()
            .hold(&session.id, turn, LISTENING_FOR)
        {
            return Err(self.terminal_unreachable());
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
            Some(one) if one.origin == "terminal" => self.hosts().terminals().asked(&one.id),
            _ => MessagesHeld::default(),
        })
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
    /// A model or effort chosen in Bridge, run in the terminal as `/model` or
    /// `/effort`. **The mode is not here**: the mods API cannot switch a live
    /// session's, so it is shown and not set. A value the terminal already has
    /// is not sent again.
    pub(crate) async fn tune_terminal(
        &self,
        session: &KeptSession,
        tuned: TuneSession,
    ) -> Result<SessionRecord, Refusal> {
        if session.state == SessionState::Ended {
            return Err(self.closed(&session.id));
        }
        let now = self.hosts().terminals().facts_of(&session.id).unwrap_or_default();
        let wanted = [
            ("model", tuned.model, now.model),
            ("effort", tuned.effort, now.effort),
        ];
        for (command, asked, has) in wanted {
            let Some(args) = asked.filter(|one| !one.trim().is_empty() && Some(one) != has.as_ref())
            else {
                continue;
            };
            let held = HeldCommand {
                command: command.to_string(),
                args,
            };
            if !self
                .hosts()
                .terminals()
                .hold_command(&session.id, held, LISTENING_FOR)
            {
                return Err(self.terminal_unreachable());
            }
        }
        let store = self.store().lock().await;
        self.ledger_row(&store, session)
    }

    pub(crate) fn terminal_unreachable(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            TERMINAL_UNREACHABLE,
            "that session is not listening. It has ended, or its terminal is closed, or \
             the armada mod is not loaded in it",
            self.run_id(),
        ))
    }
}
