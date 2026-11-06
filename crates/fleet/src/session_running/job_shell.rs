//! A Job's shells: each an interactive `$SHELL -l`, in the directory its runs take, open until it
//! exits, is stopped, or the Job ends. A Job has several, `t1`, `t2` and so on
//! (`super::job_terminals`).
//!
//! **A shell is a run like any other**, so typing, resizing, watching and stopping it are the run
//! operations on its id. **Its end tells the Job nothing**: no command was run to be told about.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, SessionRunEvent};
use ipc::{JobTerminal, ReadTerminal, TerminalScreen, TerminalTyped, TypeInTerminal};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;

use super::job_terminals::{terminal_id, Opening};
use super::{other, plain, refusal, size, Ended, Launch, SessionRuns, COLS, ROWS};
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

/// How much of the text's start has to appear for it to count as echoed.
const ECHOED_PREFIX: usize = 16;

/// Whether `printed` shows the start of `text`'s first printable line, spaces left out of both
/// sides. **Only the start**: zsh redraws a long line in place, writing only what changed and
/// moving over the rest, so no reading of the output holds the whole line in one piece. A password
/// prompt or a program that is not reading shows none of it.
fn echoed(text: &str, printed: &str) -> bool {
    let Some(first) = text.lines().map(plain::printed_through).find(|line| !line.is_empty()) else {
        return true;
    };
    let start: String = first.chars().take(ECHOED_PREFIX).collect();
    plain::printed_through(&plain::last_lines(printed, usize::MAX)).contains(&start)
        || plain::printed_through(printed).contains(&start)
}

/// What `read_terminal` waits for: text, or a regular expression.
enum Until {
    Text(String),
    Pattern(regex::Regex),
}

impl Until {
    fn of(until: String, regex: bool) -> Result<Until, String> {
        if !regex {
            return Ok(Until::Text(until));
        }
        regex::Regex::new(&until).map(Until::Pattern).map_err(|why| format!("until is not a regular expression: {why}"))
    }

    fn found(&self, printed: &str) -> bool {
        match self {
            Until::Text(text) => printed.contains(text.as_str()),
            Until::Pattern(pattern) => pattern.is_match(printed),
        }
    }
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
        let mut live = {
            let mut state = run.state.lock().expect("not poisoned");
            state.typed_at = Some(state.printed);
            run.feed.subscribe()
        };
        let typing = Arc::clone(&run);
        // A command that is not reading can hold a write back.
        if !tokio::task::spawn_blocking(move || typing.type_in(&keys)).await.unwrap_or(false) {
            return None;
        }
        Some(heard(&mut live, Instant::now() + wait).await)
    }

    /// The Job's terminal `number`, started with `launch` where none is in flight under it, and
    /// whether it started now. `on_end` is handed the new shell's id and answers what runs once it
    /// has ended.
    fn open_shell<E, F, Fut>(&self, launch: Launch<'_>, number: u32, on_end: E) -> std::io::Result<(String, bool)>
    where
        E: FnOnce(String) -> F,
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let session = launch.session.ok_or_else(|| other("a Job's shell names its Job"))?.to_string();
        // Held across the start, so two opens at once start one shell.
        let mut shells = self.shells.lock().expect("not poisoned");
        if let Some(open) = shells.get(&session).and_then(|own| own.get(&number)).filter(|id| self.in_flight(id).is_some()) {
            return Ok((open.clone(), false));
        }
        let id = self.mint();
        self.start_as(id.clone(), launch, on_end(id.clone()))?;
        shells.entry(session).or_default().insert(number, id.clone());
        Ok((id, true))
    }

    /// The run id of `session`'s terminal `number`, where its shell is in flight.
    pub fn shell_open(&self, session: &str, number: u32) -> Option<String> {
        let id = self.shells.lock().expect("not poisoned").get(session)?.get(&number).cloned()?;
        self.in_flight(&id).map(|_| id)
    }

    /// `session`'s terminals whose shells are in flight, lowest number first, with their run ids.
    pub fn shells_open(&self, session: &str) -> Vec<(u32, String)> {
        let shells = self.shells.lock().expect("not poisoned");
        let Some(own) = shells.get(session) else {
            return Vec::new();
        };
        own.iter().filter(|(_, id)| self.in_flight(id).is_some()).map(|(number, id)| (*number, id.clone())).collect()
    }

    /// Whether a program other than the shell has the foreground of the shell `run_id`.
    pub fn shell_running(&self, run_id: &str) -> bool {
        self.in_flight(run_id).is_some_and(|run| run.foreground_taken())
    }

    /// `session`'s terminal `number` in flight and what it holds, once `until` was printed since
    /// the agent last typed or `deadline` passed, and whether it was. Read as the screen resolved
    /// it, so an escape between two letters does not hide a match.
    async fn shell_awaited(
        &self,
        session: &str,
        number: u32,
        until: Option<&Until>,
        deadline: Instant,
    ) -> Option<(String, String, Option<bool>)> {
        let id = self.shell_open(session, number)?;
        let run = self.in_flight(&id)?;
        let Some(until) = until else {
            let held = run.state.lock().expect("not poisoned").held.clone();
            return Some((id, held, None));
        };
        // Taken under the lock output is appended under, so nothing printed between is missed.
        let (mut live, mut since) = {
            let state = run.state.lock().expect("not poisoned");
            (run.feed.subscribe(), state.since_typed().to_string())
        };
        let matched = loop {
            if until.found(&plain::last_lines(&since, usize::MAX)) {
                break true;
            }
            match tokio::time::timeout_at(deadline, live.recv()).await {
                Ok(Ok(SessionRunEvent::Output(text))) => since.push_str(&text),
                Ok(Err(RecvError::Lagged(_))) => {
                    since = run.state.lock().expect("not poisoned").since_typed().to_string();
                }
                Ok(Ok(SessionRunEvent::Exited(_)) | Err(RecvError::Closed)) | Err(_) => break false,
            }
        };
        let held = run.state.lock().expect("not poisoned").held.clone();
        Some((id, held, Some(matched)))
    }

    /// What `session`'s terminal `number` prints from now on, where its shell is in flight: a
    /// terminal watch's.
    pub fn shell_heard(&self, session: &str, number: u32) -> Option<tokio::sync::broadcast::Receiver<SessionRunEvent>> {
        let id = self.shell_open(session, number)?;
        Some(self.in_flight(&id)?.feed.subscribe())
    }

    /// Stop every shell `session` has in flight, as a run is stopped.
    pub fn stop_shells(&self, session: &str) {
        let open: Vec<String> = self.shells.lock().expect("not poisoned").get(session).into_iter().flat_map(|own| own.values().cloned()).collect();
        for run_id in open {
            self.stop(&run_id);
        }
    }

    /// Forget `run_id` as `session`'s terminal `number`, unless a newer shell has taken its place.
    pub fn shell_ended(&self, session: &str, number: u32, run_id: &str) {
        let mut shells = self.shells.lock().expect("not poisoned");
        if let Some(own) = shells.get_mut(session) {
            if own.get(&number).is_some_and(|id| id == run_id) {
                own.remove(&number);
            }
        }
    }
}

impl Fleet
{
    /// The person's plain open, as the tests make it: [`Opening::Any`], answered with the run id.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn open_job_shell(
        self: Arc<Self>,
        session_id: &str,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<String, Refusal> {
        Ok(self.job_shell(session_id, Opening::Any, cols, rows).await?.1)
    }

    /// A terminal of the Job as `opening` says, its shell started where none is in flight under it:
    /// its number, its run id, and whether the shell started now.
    pub async fn job_shell(
        self: Arc<Self>,
        session_id: &str,
        opening: Opening,
        cols: Option<u16>,
        rows: Option<u16>,
    ) -> Result<(u32, String, bool), Refusal> {
        let (cols, rows) = (cols.unwrap_or(COLS), rows.unwrap_or(ROWS));
        if cols == 0 || rows == 0 {
            return Err(self.no_cells());
        }
        let dir = self.run_dir(session_id).await?;
        let fleet = Arc::clone(&self);
        let session = session_id.to_string();
        // Held from choosing the number to starting its shell, so two opens of a Job with none
        // start one.
        let _opening = self.session_runs().opening.lock().await;
        let number = self.terminal_opened(session_id, opening).await?;
        let launch = Launch {
            session: Some(session_id),
            command: None,
            dir: &dir,
            size: size(cols, rows),
            keeping: Some(self.terminal_keeping(session_id, number)),
        };
        let (run_id, opened) = self
            .session_runs()
            .open_shell(launch, number, move |run_id| {
                move |_: Ended| async move { fleet.job_shell_ended(&session, number, &run_id) }
            })
            .map_err(|why| refusal(format!("the shell could not start in {dir}: {why}"), self.run_id()))?;
        if opened {
            self.publish(ipc::Event::TerminalOpened(JobTerminal {
                session_id: ipc::SessionId::carried(session_id),
                terminal: terminal_id(number),
                run_id: run_id.clone(),
            }));
        }
        Ok((number, run_id, opened))
    }

    /// A new terminal for the agent: [`Self::job_shell`] under [`Opening::New`], with `ended` passed on.
    async fn job_shell_new(self: Arc<Self>, session: &str, ended: Option<u32>) -> Result<(u32, String, bool, Option<u32>), Refusal> {
        let (number, run_id, opened) = self.job_shell(session, Opening::New, None, None).await?;
        Ok((number, run_id, opened, ended))
    }

    /// The Job a call is about: the hosted one holding the connection, else the one named.
    pub fn shell_job(&self, caller: Option<api::Caller>, named: Option<&ipc::SessionId>) -> Result<String, Refusal> {
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

    /// The agent's typing, into the terminal it named, its last used, or a new one opened for it.
    /// **Never the person's terminal by default**: an agent that never used one, or whose last has
    /// ended, is given a new one.
    pub async fn typed_in_job_shell(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        typed: TypeInTerminal,
    ) -> Result<TerminalTyped, Refusal> {
        let session = self.shell_job(caller, typed.session_id.as_ref())?;
        if typed.text.is_empty() && !typed.enter {
            return Err(refusal("nothing to type: text is empty and enter is false", self.run_id()));
        }
        // Which terminal, its run, whether its shell started now, and the last used one that ended.
        let (number, run_id, opened, ended) = match (typed.terminal.as_deref(), typed.new) {
            (Some(_), true) => return Err(self.terminal_and_new()),
            (Some(named), false) => {
                let (number, run_id) = self.terminal_named_open(&session, named)?;
                (number, run_id, false, None)
            }
            (None, true) => Arc::clone(&self).job_shell_new(&session, None).await?,
            (None, false) => match self.terminal_last_used(&session).await {
                Some(last) => match self.session_runs().shell_open(&session, last) {
                    Some(run_id) => (last, run_id, false, None),
                    None => Arc::clone(&self).job_shell_new(&session, Some(last)).await?,
                },
                None => Arc::clone(&self).job_shell_new(&session, None).await?,
            },
        };
        // Settled before a refusal too, so the read it asks for shows the new shell's prompt.
        if opened {
            self.session_runs().settled(&run_id).await;
        }
        if let Some(ended) = ended {
            return Err(refusal(
                format!(
                    "nothing was typed: this job's terminal {} ended, and {} is a new one, \
                     not where your last command ran. read_terminal {} then type again",
                    terminal_id(ended),
                    terminal_id(number),
                    terminal_id(number)
                ),
                self.run_id(),
            ));
        }
        if let Some(replaced) = self.terminal_seen(&session, number).await.filter(|seen| *seen != run_id) {
            return Err(refusal(
                format!(
                    "nothing was typed: this job's terminal {id} was a shell {replaced} that ended, and {run_id} is a new one \
                     under that id, not where your last command ran. read_terminal {id}, then type again",
                    id = terminal_id(number)
                ),
                self.run_id(),
            ));
        }
        let keys = keys(&typed);
        let wait = Duration::from_secs(typed.wait_seconds.unwrap_or(WAIT).clamp(1, MOST_WAIT));
        let (printed, still_printing) = self
            .session_runs()
            .typed_and_heard(&run_id, keys, wait)
            .await
            .ok_or_else(|| self.not_in_flight())?;
        self.terminal_saw(&session, number, &run_id).await;
        Ok(TerminalTyped {
            terminal: terminal_id(number),
            run_id,
            echoed: echoed(&typed.text, &printed),
            printed: kept(plain::last_lines(&printed, TYPED_LINES)),
            still_printing,
        })
    }

    /// The last `lines` of the terminal the agent named, its last used, or the Job's first open, as
    /// the person's pane shows them, once `until` was printed or its wait ran out.
    pub async fn job_shell_read(
        &self,
        caller: Option<api::Caller>,
        asked: ReadTerminal,
    ) -> Result<TerminalScreen, Refusal> {
        let session = self.shell_job(caller, asked.session_id.as_ref())?;
        let lines = asked.lines.unwrap_or(SCREEN_LINES).clamp(1, MOST_SCREEN_LINES);
        let until = match asked.until.filter(|until| !until.is_empty()) {
            None => None,
            Some(until) => Some(Until::of(until, asked.regex).map_err(|why| refusal(why, self.run_id()))?),
        };
        let wait = Duration::from_secs(asked.wait_seconds.unwrap_or(WAIT).clamp(1, MOST_WAIT));
        let number = match asked.terminal.as_deref() {
            Some(named) => self.terminal_named_open(&session, named)?.0,
            None => self
                .terminal_read_by_default(&session)
                .await
                .ok_or_else(|| refusal("this job has no terminal open. type_in_terminal opens one", self.run_id()))?,
        };
        let (run_id, held, matched) = self
            .session_runs()
            .shell_awaited(&session, number, until.as_ref(), Instant::now() + wait)
            .await
            .ok_or_else(|| self.terminal_not_open(&terminal_id(number)))?;
        self.terminal_saw(&session, number, &run_id).await;
        Ok(TerminalScreen { terminal: terminal_id(number), run_id, screen: kept(plain::last_lines(&held, lines)), matched })
    }
}
