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
use ipc::{
    HeldCommand, MessagesHeld, SendSessionMessage, SessionId, SessionRecord, SessionRow,
    SessionRowChanged, TerminalFacts, TuneSession,
};
use ipc::{TakeHeld, WireError};
use store::{KeptSession, SessionState};

use super::rows::{mention_line, SESSION_THREAD_UNREADABLE as THREAD_UNREADABLE};
use super::serving::{MESSAGE_EMPTY, SESSION_COMMAND_REFUSED};
use super::thread_cache::ThreadCache;
use crate::daemon::Fleet;

/// A send to a session whose mod has not asked lately. A 409.
pub(super) const TERMINAL_UNREACHABLE: &str = "fleet.terminal_session_unreachable";

/// How long after its mod last asked a session still counts as listening. A
/// mod holding an ask open is listening throughout, and this covers the moment
/// between one ask's end and the next.
pub(crate) const LISTENING_FOR: Duration = Duration::from_secs(10);

/// The longest Fleet holds an ask for held messages, whatever the mod asked for.
pub(crate) const HELD_HOLD_MAX: Duration = Duration::from_secs(25);

/// How often a thread being watched is looked at. One `stat` when nothing grew.
const LOOKS_EVERY: Duration = Duration::from_secs(1);

struct Listening {
    asked_at: Instant,
    /// Asks held open on this session now. One is listening however long ago
    /// `asked_at` was.
    holding: usize,
    held: Vec<String>,
    commands: Vec<HeldCommand>,
    /// A person pressed End session: handed over once, after the messages.
    end: bool,
    /// Rung when something is held, for the ask waiting on it.
    rung: Arc<tokio::sync::Notify>,
}

impl Listening {
    fn new() -> Self {
        Listening {
            asked_at: Instant::now(),
            holding: 0,
            held: Vec::new(),
            commands: Vec::new(),
            end: false,
            rung: Arc::default(),
        }
    }

    fn listening_within(&self, within: Duration) -> bool {
        self.holding > 0 || self.asked_at.elapsed() <= within
    }

    fn take(&mut self) -> MessagesHeld {
        self.asked_at = Instant::now();
        MessagesHeld {
            messages: std::mem::take(&mut self.held),
            commands: std::mem::take(&mut self.commands),
            end: std::mem::take(&mut self.end),
        }
    }
}

/// An ask held open on a session, counted for as long as it lives, **including
/// where the request is dropped** from under it.
struct Holding<'a> {
    terminals: &'a Terminals,
    session: String,
}

impl Drop for Holding<'_> {
    fn drop(&mut self) {
        let mut table = self.terminals.listening.lock().expect("held across no panic");
        if let Some(one) = table.get_mut(&self.session) {
            one.holding = one.holding.saturating_sub(1);
            one.asked_at = Instant::now();
        }
    }
}

/// What Fleet holds for terminal sessions, which lives only as long as Fleet.
#[derive(Default)]
pub struct Terminals {
    listening: Mutex<HashMap<String, Listening>>,
    watched: Mutex<HashSet<String>>,
    tuned: Mutex<HashMap<String, TerminalFacts>>,
    /// Sessions a person ended from Bridge whose terminal has not ended yet.
    /// Memory only: a Fleet that restarts meanwhile ends the row as the
    /// terminal reports it and parks nothing.
    ending: Mutex<HashSet<String>>,
    /// When each standing question was last polled for, by call. Memory only: a
    /// Fleet that restarts gives every mod a fresh lapse to poll again in.
    polled: Mutex<HashMap<String, Instant>>,
    /// Rung when a question is answered or closed, for the polls holding on it.
    pub(crate) rung: tokio::sync::Notify,
    /// The terminal Sessions with a turn running, and when anything was last heard from each. Memory
    /// only: a Fleet that restarts does not know a turn is running until the mod says something.
    running: Mutex<HashMap<String, Instant>>,
    /// The permission prompt each terminal Session's harness has open, as its mod told it. Memory only:
    /// a mod tells it again when a restarted Fleet answers.
    prompts: Mutex<HashMap<String, PromptOpen>>,
    /// The threads Bridge has opened, kept drawn.
    threads: Arc<ThreadCache>,
}

/// A permission prompt open in a terminal: the tool, its command or path, and when Fleet heard of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromptOpen {
    pub tool: String,
    pub detail: String,
    pub since: String,
}

impl Terminals {
    /// A turn began in this terminal Session. Any prompt the last one left open is gone with it.
    pub(crate) fn turn_began(&self, session: &str) {
        self.running.lock().expect("held across no panic").insert(session.to_string(), Instant::now());
        self.prompt_closed(session);
    }

    /// The harness opened a permission prompt. True where it was not already the one open.
    pub(crate) fn prompt_opened(&self, session: &str, open: PromptOpen) -> bool {
        let mut table = self.prompts.lock().expect("held across no panic");
        let same = table
            .get(session)
            .is_some_and(|one| one.tool == open.tool && one.detail == open.detail);
        if !same {
            table.insert(session.to_string(), open);
        }
        !same
    }

    /// The prompt closed. True where one was open.
    pub(crate) fn prompt_closed(&self, session: &str) -> bool {
        self.prompts.lock().expect("held across no panic").remove(session).is_some()
    }

    /// The permission prompt open in this Session's terminal, if one is.
    pub(crate) fn prompt_of(&self, session: &str) -> Option<PromptOpen> {
        self.prompts.lock().expect("held across no panic").get(session).cloned()
    }

    /// The mod said something about this Session: a running turn is not quiet.
    pub(crate) fn turn_heard(&self, session: &str) {
        if let Some(at) = self.running.lock().expect("held across no panic").get_mut(session) {
            *at = Instant::now();
        }
    }

    /// The turn is over, or the Session is, and any prompt it held open with it.
    pub(crate) fn turn_over(&self, session: &str) {
        self.running.lock().expect("held across no panic").remove(session);
        self.prompt_closed(session);
    }

    /// Every terminal Session with a turn running and how long it has been quiet.
    pub(crate) fn running_quiet(&self) -> Vec<(String, Duration)> {
        self.running
            .lock()
            .expect("held across no panic")
            .iter()
            .map(|(id, at)| (id.clone(), at.elapsed()))
            .collect()
    }

    /// How long the running turn has been quiet, where one is running.
    pub(crate) fn quiet_for(&self, session: &str) -> Option<Duration> {
        self.running.lock().expect("held across no panic").get(session).map(Instant::elapsed)
    }

    /// The mod asked: it is listening now, and takes what was held.
    fn asked(&self, session: &str) -> MessagesHeld {
        let mut table = self.listening.lock().expect("held across no panic");
        table
            .entry(session.to_string())
            .or_insert_with(Listening::new)
            .take()
    }

    /// The mod asked and will wait: it is listening for as long as this holds,
    /// and the answer comes the moment something is held, or empty after `wait`.
    pub(super) async fn asked_for(&self, session: &str, wait: Duration) -> MessagesHeld {
        if wait.is_zero() {
            return self.asked(session);
        }
        let rung = {
            let mut table = self.listening.lock().expect("held across no panic");
            let one = table.entry(session.to_string()).or_insert_with(Listening::new);
            one.holding += 1;
            Arc::clone(&one.rung)
        };
        let _holding = Holding {
            terminals: self,
            session: session.to_string(),
        };
        let until = tokio::time::Instant::now() + wait;
        loop {
            // Rung before the look, so a message held between the two is not missed.
            let notified = rung.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let held = self.asked(session);
            if !held.messages.is_empty() || !held.commands.is_empty() || held.end {
                return held;
            }
            if tokio::time::timeout_at(until, notified).await.is_err() {
                return held;
            }
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
        self.polled
            .lock()
            .expect("held across no panic")
            .remove(call);
    }

    /// Put something by for the mod and ring the ask waiting on it, or say nobody is listening.
    fn held_for_mod(&self, session: &str, listening_for: Duration, put: impl FnOnce(&mut Listening)) -> bool {
        let mut table = self.listening.lock().expect("held across no panic");
        match table.get_mut(session) {
            Some(one) if one.listening_within(listening_for) => {
                put(one);
                one.rung.notify_waiters();
                true
            }
            _ => false,
        }
    }

    /// Keep a command for the mod, or say nobody is listening.
    fn hold_command(&self, session: &str, command: HeldCommand, listening_for: Duration) -> bool {
        self.held_for_mod(session, listening_for, |one| one.commands.push(command))
    }

    /// Hold the end for the mod, or say nobody is listening. True where it was held.
    pub(super) fn hold_end(&self, session: &str, listening_for: Duration) -> bool {
        let held = self.held_for_mod(session, listening_for, |one| one.end = true);
        if held {
            self.ending
                .lock()
                .expect("held across no panic")
                .insert(session.to_string());
        }
        held
    }

    /// Whether a person ended this session from Bridge, forgotten as it is read.
    pub(crate) fn was_ended_from_bridge(&self, session: &str) -> bool {
        self.ending
            .lock()
            .expect("held across no panic")
            .remove(session)
    }

    /// Whether the session's mod is holding an ask, or has asked within [`LISTENING_FOR`].
    pub(crate) fn listening(&self, session: &str) -> bool {
        self.listening
            .lock()
            .expect("held across no panic")
            .get(session)
            .is_some_and(|one| one.listening_within(LISTENING_FOR))
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
        self.held_for_mod(session, listening_for, |one| one.held.push(text))
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
        self.threads.forget(session);
    }

    pub(super) fn threads(&self) -> Arc<ThreadCache> {
        Arc::clone(&self.threads)
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
        let threads = self.hosts().terminals().threads();
        let (file, rows, next) = tokio::task::spawn_blocking(move || threads.thread(&home, &id))
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
        // A terminal's messages are its transcript, never rows, so the walk the person has now
        // written after (an approval included) is settled here or it would wait for good.
        let reads = {
            let store = self.store().lock().await;
            self.ledger_reads(&store, session)?
        };
        let shown = self.ledger_built(session, reads);
        let walks: Vec<String> = shown
            .waiting_for
            .iter()
            .filter(|one| one.source == ipc::WaitingSource::Walk)
            .map(|one| one.id.clone())
            .collect();
        let record = if walks.is_empty() {
            shown
        } else {
            let reads = {
                let mut store = self.store().lock().await;
                for walk in &walks {
                    store.dismiss_waiting(&session.id, walk).map_err(|why| self.ledger_fault(why))?;
                }
                self.ledger_reads(&store, session)?
            };
            self.ledger_built(session, reads)
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// The mod's ask. A session that is not a terminal one is handed nothing
    /// and not marked listening. **Where the ask carries `wait_ms` it is held,
    /// up to [`HELD_HOLD_MAX`], until something is held**, and the store is not
    /// held with it.
    pub(crate) async fn held_for(&self, ask: TakeHeld) -> Result<MessagesHeld, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .session(&ask.session_id)
            .map_err(|why| self.ledger_fault(why))?;
        let wait = ask
            .wait_ms
            .map_or(Duration::ZERO, |ms| Duration::from_millis(ms).min(HELD_HOLD_MAX));
        Ok(match kept {
            Some(one) if one.origin == "terminal" => self.hosts().terminals().asked_for(&one.id, wait).await,
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
        let now = self
            .hosts()
            .terminals()
            .facts_of(&session.id)
            .unwrap_or_default();
        let wanted = [
            ("model", tuned.model, now.model),
            ("effort", tuned.effort, now.effort),
        ];
        for (command, asked, has) in wanted {
            let Some(args) =
                asked.filter(|one| !one.trim().is_empty() && Some(one) != has.as_ref())
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

    /// A slash command a person pressed in the thread, held for the mod to run as typed. **One slash
    /// command on one line**: anything else is refused, so the mod is never handed free text to run.
    pub(crate) async fn run_terminal_command(
        &self,
        session: &KeptSession,
        text: &str,
    ) -> Result<SessionRecord, Refusal> {
        if session.state == SessionState::Ended {
            return Err(self.closed(&session.id));
        }
        let Some(held) = slash_command(text) else {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                "that is not one slash command. It starts with / and a command's name, and is one line",
            ));
        };
        if !self
            .hosts()
            .terminals()
            .hold_command(&session.id, held, LISTENING_FOR)
        {
            return Err(self.terminal_unreachable());
        }
        let store = self.store().lock().await;
        self.ledger_row(&store, session)
    }

    pub(crate) fn terminal_unreachable(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            TERMINAL_UNREACHABLE,
            "that job is not listening. It has ended, or its terminal is closed, or \
             the armada mod is not loaded in it",
            self.run_id(),
        ))
    }
}

/// `/name` or `/name args` on one line, as the mod runs it: the command's name without the slash,
/// and the rest as its arguments. A name is lower-case letters, digits, `:` and `-`.
fn slash_command(text: &str) -> Option<HeldCommand> {
    let text = text.trim();
    let rest = text.strip_prefix('/')?;
    if rest.contains(['\n', '\r']) {
        return None;
    }
    let (name, args) = rest.split_once(' ').unwrap_or((rest, ""));
    let mut chars = name.chars();
    let named = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '-');
    named.then(|| HeldCommand {
        command: name.to_string(),
        args: args.trim().to_string(),
    })
}
