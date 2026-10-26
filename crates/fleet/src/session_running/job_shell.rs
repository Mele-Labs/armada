//! A Job's shell: one interactive `$SHELL -l` a Job, in the directory its runs take, open until it
//! exits, is stopped, or the Job ends.
//!
//! **It is a run like any other**, so typing, resizing, watching and stopping it are the run
//! operations on its id. **Its end tells the Job nothing**: no command was run to be told about.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, SessionRunEvent};
use ipc::{JobTerminal, TerminalScreen, TerminalTyped, TypeInTerminal};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;

use super::{plain, refusal, size, Ended, Launch, SessionRuns, COLS, ROWS};
use crate::daemon::Fleet;

/// How long the shell prints nothing before an agent's typing is answered.
const QUIET: Duration = Duration::from_millis(1500);
/// How long an agent waits where it named no wait, and the most it may name.
const WAIT: u64 = 10;
const MOST_WAIT: u64 = 60;
/// How long a shell opened for an agent has to print its prompt before it is typed into.
const SETTLING: Duration = Duration::from_secs(10);
/// Lines and bytes an agent is answered at most: well inside the door's 64 KiB.
const TYPED_LINES: usize = 200;
/// Lines `read_terminal` answers where it named none, and the most it may name.
const SCREEN_LINES: usize = 60;
const MOST_SCREEN_LINES: usize = 500;
const MOST_BYTES: usize = 32 * 1024;

/// The last [`MOST_BYTES`] of `text`, cut at a character.
fn kept(text: String) -> String {
    if text.len() <= MOST_BYTES {
        return text;
    }
    let mut cut = text.len() - MOST_BYTES;
    while !text.is_char_boundary(cut) {
        cut += 1;
    }
    text[cut..].to_string()
}

/// Ctrl-U: zsh, bash and a terminal's own line discipline each clear the line on it.
const CLEAR_LINE: char = '\u{15}';

/// What `typed` writes. **The line is cleared first** where it asks to be and holds something
/// printable: a terminal's answers to a program's queries land on the input line as text, and Enter
/// would run them with the agent's words. A key such as Ctrl-C is sent bare, since a full-screen
/// program may take Ctrl-U as a key of its own.
fn keys(typed: &TypeInTerminal) -> String {
    let mut keys = String::new();
    if typed.clear && typed.text.chars().any(|c| !c.is_control()) {
        keys.push(CLEAR_LINE);
    }
    keys.push_str(&typed.text);
    if typed.enter {
        keys.push('\r');
    }
    keys
}

/// Whether `printed` shows `text`'s first printable line. Spaces are left out of both sides, since
/// a shell wraps a long line where it likes; and it is read both as the screen resolved it and as
/// the characters arrived, since a wrap that returns to the line's start overwrites the first.
fn echoed(text: &str, printed: &str) -> bool {
    let Some(first) = text.lines().map(plain::printed_through).find(|line| !line.is_empty()) else {
        return true;
    };
    plain::printed_through(&plain::last_lines(printed, usize::MAX)).contains(&first)
        || plain::printed_through(printed).contains(&first)
}

/// What a run prints until it has printed nothing for [`QUIET`], it ends, or `until`; and whether
/// `until` came while it was still printing.
async fn heard(live: &mut tokio::sync::broadcast::Receiver<SessionRunEvent>, until: Instant) -> (String, bool) {
    let mut printed = String::new();
    loop {
        let span = QUIET.min(until.saturating_duration_since(Instant::now()));
        if span.is_zero() {
            return (printed, true);
        }
        match tokio::time::timeout(span, live.recv()).await {
            Ok(Ok(SessionRunEvent::Output(text))) => printed.push_str(&text),
            // A reader that fell behind reads on, as a socket does.
            Ok(Err(RecvError::Lagged(_))) => {}
            Ok(Ok(SessionRunEvent::Exited(_)) | Err(RecvError::Closed)) => return (printed, false),
            Err(_) => return (printed, span < QUIET),
        }
    }
}

impl SessionRuns {
    /// Until the shell `run_id` has printed something and then gone quiet, or [`SETTLING`].
    async fn settled(&self, run_id: &str) {
        let Some(run) = self.in_flight(run_id) else {
            return;
        };
        let until = Instant::now() + SETTLING;
        // Subscribed under the lock output is appended under, so nothing printed is missed.
        let (mut live, started) = {
            let state = run.state.lock().expect("not poisoned");
            (run.feed.subscribe(), !state.held.is_empty())
        };
        if !started {
            match tokio::time::timeout_at(until, live.recv()).await {
                Ok(Ok(SessionRunEvent::Output(_)) | Err(RecvError::Lagged(_))) => {}
                _ => return,
            }
        }
        heard(&mut live, until).await;
    }

    /// Type `keys` into `run_id` and answer what it printed after, as [`heard`] gathers it. `None`
    /// where the run is not in flight or the keys did not reach it.
    async fn typed_and_heard(&self, run_id: &str, keys: String, wait: Duration) -> Option<(String, bool)> {
        let run = self.in_flight(run_id)?;
        let mut live = run.feed.subscribe();
        let typing = Arc::clone(&run);
        // A command that is not reading can hold a write back.
        if !tokio::task::spawn_blocking(move || typing.type_in(&keys)).await.unwrap_or(false) {
            return None;
        }
        Some(heard(&mut live, Instant::now() + wait).await)
    }

    /// The Job's shell, started with `launch` where none is in flight, and whether it started now.
    /// `on_end` is handed the new shell's id and answers what runs once it has ended.
    fn open_shell<E, F, Fut>(&self, launch: Launch<'_>, on_end: E) -> std::io::Result<(String, bool)>
    where
        E: FnOnce(String) -> F,
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Held across the start, so two opens at once start one shell.
        let mut shells = self.shells.lock().expect("not poisoned");
        if let Some(open) = shells.get(launch.session).filter(|id| self.in_flight(id).is_some()) {
            return Ok((open.clone(), false));
        }
        let session = launch.session.to_string();
        let id = self.mint();
        self.start_as(id.clone(), launch, on_end(id.clone()))?;
        shells.insert(session, id.clone());
        Ok((id, true))
    }

    /// The Job's shell in flight and what it holds of its output.
    fn shell_held(&self, session: &str) -> Option<(String, String)> {
        let id = self.shells.lock().expect("not poisoned").get(session).cloned()?;
        let run = self.in_flight(&id)?;
        let held = run.state.lock().expect("not poisoned").held.clone();
        Some((id, held))
    }

    /// Stop `session`'s shell, as a run is stopped, where it has one in flight.
    fn stop_shell(&self, session: &str) {
        let open = self.shells.lock().expect("not poisoned").get(session).cloned();
        if let Some(run_id) = open {
            self.stop(&run_id);
        }
    }

    /// Forget `run_id` as `session`'s shell, unless a newer one has taken its place.
    fn shell_ended(&self, session: &str, run_id: &str) {
        let mut shells = self.shells.lock().expect("not poisoned");
        if shells.get(session).is_some_and(|id| id == run_id) {
            shells.remove(session);
        }
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
    /// The Job's shell, opened where it has none. A Job that has ended, or that Fleet has no row
    /// for, is refused as a run is.
    pub(crate) async fn open_job_shell(
        self: Arc<Self>,
        session_id: &str,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<String, Refusal> {
        Ok(self.job_shell(session_id, cols, rows).await?.0)
    }

    /// [`Self::open_job_shell`], and whether the shell started now.
    async fn job_shell(
        self: Arc<Self>,
        session_id: &str,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<(String, bool), Refusal> {
        let (cols, rows) = (cols.unwrap_or(COLS), rows.unwrap_or(ROWS));
        if cols == 0 || rows == 0 {
            return Err(self.no_cells());
        }
        let dir = self.run_dir(session_id).await?;
        let launch = Launch { session: session_id, command: None, dir: &dir, size: size(cols, rows) };
        let fleet = Arc::clone(&self);
        let session = session_id.to_string();
        let (run_id, opened) = self
            .session_runs()
            .open_shell(launch, move |run_id| {
                move |_: Ended| async move { fleet.job_shell_ended(&session, &run_id) }
            })
            .map_err(|why| refusal(format!("the shell could not start in {dir}: {why}"), self.run_id()))?;
        if opened {
            self.publish(ipc::Event::TerminalOpened(JobTerminal {
                session_id: ipc::SessionId::carried(session_id),
                run_id: run_id.clone(),
            }));
        }
        Ok((run_id, opened))
    }

    /// The Job a call is about: the hosted one holding the connection, else the one named.
    fn shell_job(&self, caller: Option<api::Caller>, named: Option<&ipc::SessionId>) -> Result<String, Refusal> {
        caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                refusal(
                    "Fleet hosts no job on this connection, so name yours with session_id",
                    self.run_id(),
                )
            })
    }

    /// The agent's typing, into the Job's shell opened for it where it has none.
    pub(crate) async fn typed_in_job_shell(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        typed: TypeInTerminal,
    ) -> Result<TerminalTyped, Refusal> {
        let session = self.shell_job(caller, typed.session_id.as_ref())?;
        if typed.text.is_empty() && !typed.enter {
            return Err(refusal("nothing to type: text is empty and enter is false", self.run_id()));
        }
        let (run_id, opened) = Arc::clone(&self).job_shell(&session, None, None).await?;
        if opened {
            self.session_runs().settled(&run_id).await;
        }
        let keys = keys(&typed);
        let wait = Duration::from_secs(typed.wait_seconds.unwrap_or(WAIT).clamp(1, MOST_WAIT));
        let (printed, still_printing) = self
            .session_runs()
            .typed_and_heard(&run_id, keys, wait)
            .await
            .ok_or_else(|| self.not_in_flight())?;
        Ok(TerminalTyped {
            run_id,
            echoed: echoed(&typed.text, &printed),
            printed: kept(plain::last_lines(&printed, TYPED_LINES)),
            still_printing,
        })
    }

    /// The last `lines` of the Job's shell, as the person's pane shows them.
    pub(crate) fn job_shell_read(
        &self,
        caller: Option<api::Caller>,
        named: Option<&ipc::SessionId>,
        lines: Option<usize>,
    ) -> Result<TerminalScreen, Refusal> {
        let session = self.shell_job(caller, named)?;
        let lines = lines.unwrap_or(SCREEN_LINES).clamp(1, MOST_SCREEN_LINES);
        let (run_id, held) = self.session_runs().shell_held(&session).ok_or_else(|| {
            refusal("this job has no terminal open. type_in_terminal opens one", self.run_id())
        })?;
        Ok(TerminalScreen { run_id, screen: kept(plain::last_lines(&held, lines)) })
    }

    /// A Job that ended takes its shell with it.
    pub(crate) fn job_shell_wound_up(&self, session: &str) {
        self.session_runs().stop_shell(session);
    }

    fn job_shell_ended(&self, session: &str, run_id: &str) {
        self.session_runs().shell_ended(session, run_id);
        self.publish(ipc::Event::TerminalExited(JobTerminal {
            session_id: ipc::SessionId::carried(session),
            run_id: run_id.to_string(),
        }));
    }
}
