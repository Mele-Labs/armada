//! A shell block run from a Session's thread: the person's own command, in the Session's slot
//! worktree (or its directory where it holds none), on a pseudo-terminal of its own. A Job's
//! shell is one more run, [`job_shell`]'s.
//!
//! **The command is the terminal's session leader**, so Ctrl-C written to it reaches it as SIGINT.
//! **Nothing is written down**: a run keeps its last [`RING`] bytes while it lives and
//! [`KEPT_AFTER_EXIT`] after, and a restart forgets every run. Its id is a ULID, so no run after a
//! restart takes the id of one before it.
//!
//! **The history and the subscription are taken under one lock**, the one the reader appends
//! under, so a viewer loses no output and reads none twice. **A character a read ends inside waits
//! for the read that finishes it**, so no message splits one.

mod ended;
mod job_shell;
pub(crate) mod plain;

use std::collections::BTreeMap;
use std::future::Future;
use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ObservedSessionRun, Refusal, SessionRunEvent, SESSION_RUN_BACKLOG};
use ipc::{
    ResizeSessionRun, SessionRunUnderway, StartSessionRun, StopSessionRun, WireError,
    WriteSessionRun,
};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use store::{AttachmentState, Holder};
use tokio::sync::broadcast;

use crate::daemon::Fleet;
use crate::mint::{Mint, UlidMint};

pub(crate) use ended::{fenced, Ended};

/// Bytes of output a run keeps.
const RING: usize = 256 * 1024;
/// How long a run is held after it exits.
const KEPT_AFTER_EXIT: Duration = Duration::from_secs(90);
/// How long a stopped group has to leave before it is killed.
const GRACE: Duration = Duration::from_secs(3);
/// How long output still in the terminal is read for after the shell has exited.
const DRAIN: Duration = Duration::from_millis(300);
/// The terminal's size where Bridge did not say.
const COLS: u16 = 80;
const ROWS: u16 = 24;

/// A run that was refused: a 422.
const REFUSED: &str = "fleet.session_run_refused";

struct State {
    /// The last [`RING`] bytes printed, cut at a character.
    held: String,
    /// `Some` once the run ended, with its code.
    exited: Option<Option<i32>>,
    /// Bytes printed since the run started, [`Self::held`]'s dropped ones included.
    printed: u64,
    /// [`Self::printed`] when the agent last typed, which is where `read_terminal`'s `until` reads from.
    typed_at: Option<u64>,
}

impl State {
    /// What the run printed since the agent last typed into it, or all it holds where it never has.
    fn since_typed(&self) -> &str {
        let after = self.typed_at.map_or(u64::MAX, |at| self.printed - at);
        match usize::try_from(after).ok().and_then(|after| self.held.len().checked_sub(after)) {
            Some(from) if self.held.is_char_boundary(from) => &self.held[from..],
            _ => &self.held,
        }
    }
}

/// What a run is typed into and resized through. Gone once the run has ended.
struct Terminal {
    master: Box<dyn MasterPty + Send>,
    keys: Box<dyn Write + Send>,
}

struct Run {
    pgid: i32,
    session: String,
    command: String,
    stopped: AtomicBool,
    /// What stopping it first sends: a shell ignores TERM but leaves on the HUP a closing terminal sends.
    stop_signal: &'static str,
    state: Mutex<State>,
    terminal: Mutex<Option<Terminal>>,
    feed: broadcast::Sender<SessionRunEvent>,
}

impl Run {
    fn ended(&self) -> bool {
        self.state.lock().expect("not poisoned").exited.is_some()
    }

    /// Signal the group with `kill`, which takes `-<pgid>` for a whole group: the gate lists the
    /// few places that speak `unsafe`, and this is not one of them.
    fn signal(&self, signal: &str) {
        let _ = std::process::Command::new("/bin/kill")
            .args(["-s", signal, "--", &format!("-{}", self.pgid)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    fn output(&self, text: String) {
        let mut state = self.state.lock().expect("not poisoned");
        state.held.push_str(&text);
        state.printed += text.len() as u64;
        if state.held.len() > RING {
            let mut cut = state.held.len() - RING;
            while !state.held.is_char_boundary(cut) {
                cut += 1;
            }
            state.held.drain(..cut);
        }
        let _ = self.feed.send(SessionRunEvent::Output(text));
    }

    fn exit(&self, code: Option<i32>) {
        let mut state = self.state.lock().expect("not poisoned");
        state.exited = Some(code);
        let _ = self.feed.send(SessionRunEvent::Exited(code));
    }

    /// Whether `keys` reached the terminal.
    fn type_in(&self, keys: &str) -> bool {
        let mut terminal = self.terminal.lock().expect("not poisoned");
        terminal.as_mut().is_some_and(|one| {
            one.keys.write_all(keys.as_bytes()).and_then(|()| one.keys.flush()).is_ok()
        })
    }

    fn resize(&self, size: PtySize) -> bool {
        let terminal = self.terminal.lock().expect("not poisoned");
        terminal.as_ref().is_some_and(|one| one.master.resize(size).is_ok())
    }
}

/// Every run Fleet holds, in flight or lately ended. Never written down.
#[derive(Default)]
pub(crate) struct SessionRuns {
    held: Arc<Mutex<BTreeMap<String, Arc<Run>>>>,
    minted: UlidMint,
    /// Each Job's shell, by the Job's id.
    shells: Mutex<BTreeMap<String, String>>,
}

/// What a run needs to start.
struct Launch<'a> {
    session: &'a str,
    /// `None` is an interactive login shell, a Job's shell.
    command: Option<&'a str>,
    dir: &'a str,
    size: PtySize,
}

fn refusal(why: impl Into<String>, run_id: ipc::RunId) -> Refusal {
    Refusal::Unacceptable(WireError::raised(REFUSED, why, run_id))
}

pub(crate) fn shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.trim().is_empty())
        .unwrap_or_else(|| "/bin/zsh".into())
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }
}

fn other(why: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(why.to_string())
}

impl SessionRuns {
    fn get(&self, run_id: &str) -> Option<Arc<Run>> {
        self.held.lock().expect("not poisoned").get(run_id).cloned()
    }

    fn in_flight(&self, run_id: &str) -> Option<Arc<Run>> {
        self.get(run_id).filter(|run| !run.ended())
    }

    fn mint(&self) -> String {
        format!("session-run-{}", self.minted.ulid().as_str())
    }

    /// Start `launch` and answer its id. The reading and the wait are threads of their own, and
    /// `ended` runs once the run is over and its viewers have been told.
    fn start<F, Fut>(&self, launch: Launch<'_>, ended: F) -> std::io::Result<String>
    where
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let id = self.mint();
        self.start_as(id.clone(), launch, ended)?;
        Ok(id)
    }

    /// [`Self::start`], under an id minted already.
    fn start_as<F, Fut>(&self, id: String, launch: Launch<'_>, ended: F) -> std::io::Result<()>
    where
        F: FnOnce(Ended) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let pair = native_pty_system().openpty(launch.size).map_err(other)?;
        let mut shell = CommandBuilder::new(shell());
        match launch.command {
            Some(command) => shell.args(["-lc", command]),
            None => shell.arg("-l"),
        }
        shell.cwd(launch.dir);
        shell.env("TERM", "xterm-256color");
        let mut child = pair.slave.spawn_command(shell).map_err(other)?;
        // With our slave end open the reader would never be told the terminal had closed.
        drop(pair.slave);
        let pid = child.process_id().ok_or_else(|| other("the command has no process id"))?;
        let reader = pair.master.try_clone_reader().map_err(other)?;
        let keys = pair.master.take_writer().map_err(other)?;

        let run = Arc::new(Run {
            // The command is its own session, so its group is its pid.
            pgid: pid as i32,
            session: launch.session.to_string(),
            command: launch.command.unwrap_or_default().to_string(),
            stopped: AtomicBool::new(false),
            stop_signal: if launch.command.is_some() { "TERM" } else { "HUP" },
            state: Mutex::new(State { held: String::new(), exited: None, printed: 0, typed_at: None }),
            terminal: Mutex::new(Some(Terminal { master: pair.master, keys })),
            feed: broadcast::channel(SESSION_RUN_BACKLOG).0,
        });
        self.held
            .lock()
            .expect("not poisoned")
            .insert(id.clone(), run.clone());

        // A blocking read cannot be cancelled: a grandchild holding the terminal open leaves this
        // thread reading until it goes too. The run waits for it no longer than [`DRAIN`].
        let (drained, drain) = tokio::sync::oneshot::channel();
        let printing = run.clone();
        std::thread::Builder::new().name("session-run-output".into()).spawn(move || {
            read_output(reader, &printing);
            let _ = drained.send(());
        })?;
        let held = self.held.clone();
        let forget = id.clone();
        let waiting = tokio::task::spawn_blocking(move || child.wait());
        tokio::spawn(async move {
            let status = waiting.await;
            let _ = tokio::time::timeout(DRAIN, drain).await;
            let stopped = run.stopped.load(Ordering::Relaxed);
            let code = match status {
                Ok(Ok(status)) if !stopped && status.signal().is_none() => {
                    Some(status.exit_code() as i32)
                }
                _ => None,
            };
            run.terminal.lock().expect("not poisoned").take();
            run.exit(code);
            let output = run.state.lock().expect("not poisoned").held.clone();
            ended(Ended {
                session: run.session.clone(),
                command: run.command.clone(),
                code,
                stopped,
                output,
            })
            .await;
            tokio::time::sleep(KEPT_AFTER_EXIT).await;
            held.lock().expect("not poisoned").remove(&forget);
        });
        Ok(())
    }

    /// The run's stop signal, then SIGKILL if the group is still there after [`GRACE`].
    fn stop(&self, run_id: &str) -> bool {
        let Some(run) = self.in_flight(run_id) else {
            return false;
        };
        run.stopped.store(true, Ordering::Relaxed);
        run.signal(run.stop_signal);
        tokio::spawn(async move {
            tokio::time::sleep(GRACE).await;
            if !run.ended() {
                run.signal("KILL");
            }
        });
        true
    }

    async fn write(&self, run_id: &str, keys: String) -> bool {
        let Some(run) = self.in_flight(run_id) else {
            return false;
        };
        // A command that is not reading can hold a write back.
        tokio::task::spawn_blocking(move || run.type_in(&keys)).await.unwrap_or(false)
    }

    fn resize(&self, run_id: &str, size: PtySize) -> bool {
        self.in_flight(run_id).is_some_and(|run| run.resize(size))
    }

    fn watch(&self, run_id: &str) -> Option<ObservedSessionRun> {
        let run = self.get(run_id)?;
        // Subscribe and copy under the lock output is appended under.
        let state = run.state.lock().expect("not poisoned");
        Some(ObservedSessionRun {
            run_id: run_id.to_string(),
            history: state.held.clone(),
            live: match state.exited {
                Some(code) => Err(code),
                None => Ok(run.feed.subscribe()),
            },
        })
    }
}

/// Read the terminal until it closes, passing on whole characters.
fn read_output(mut reader: Box<dyn Read + Send>, run: &Run) {
    let mut buffer = [0u8; 8192];
    let mut pending: Vec<u8> = Vec::new();
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                pending.extend_from_slice(&buffer[..read]);
                let text = whole_characters(&mut pending);
                if !text.is_empty() {
                    run.output(text);
                }
            }
            Err(why) if why.kind() == std::io::ErrorKind::Interrupted => {}
            // Some systems answer a read on a terminal whose far end closed with an error.
            Err(_) => break,
        }
    }
    if !pending.is_empty() {
        run.output(String::from_utf8_lossy(&pending).into_owned());
    }
}

/// Take the text off the front of `pending`, leaving a character that is not yet whole. A byte
/// that can never begin one is replaced.
pub(crate) fn whole_characters(pending: &mut Vec<u8>) -> String {
    let mut text = String::new();
    loop {
        match std::str::from_utf8(pending) {
            Ok(rest) => {
                text.push_str(rest);
                pending.clear();
                return text;
            }
            Err(why) => {
                let good = why.valid_up_to();
                text.push_str(&String::from_utf8_lossy(&pending[..good]));
                match why.error_len() {
                    None => {
                        pending.drain(..good);
                        return text;
                    }
                    Some(bad) => {
                        text.push('\u{FFFD}');
                        pending.drain(..good + bad);
                    }
                }
            }
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
    pub(crate) async fn begin_session_run(
        self: Arc<Self>,
        start: StartSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if start.command.trim().is_empty() {
            return Err(refusal("a run needs a command", self.run_id()));
        }
        let (cols, rows) = (start.cols.unwrap_or(COLS), start.rows.unwrap_or(ROWS));
        if cols == 0 || rows == 0 {
            return Err(self.no_cells());
        }
        let dir = self.run_dir(start.session_id.as_str()).await?;
        let fleet = Arc::clone(&self);
        let launch = Launch {
            session: start.session_id.as_str(),
            command: Some(&start.command),
            dir: &dir,
            size: size(cols, rows),
        };
        match self
            .session_runs()
            .start(launch, move |ended| async move { fleet.session_run_ended(ended).await })
        {
            Ok(run_id) => Ok(SessionRunUnderway { run_id }),
            Err(why) => Err(refusal(
                format!("the command could not start in {dir}: {why}"),
                self.run_id(),
            )),
        }
    }

    /// Where a Job's runs and its shell stand: the worktree of the slot it holds, or its
    /// directory where it holds none. A Job that has ended or that Fleet has no row for is refused.
    async fn run_dir(&self, session_id: &str) -> Result<String, Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(session_id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| refusal("no job Fleet knows by that id", self.run_id()))?;
        if session.state == store::SessionState::Ended {
            return Err(refusal("the job has ended", self.run_id()));
        }
        let held = store
            .attachments_of(&Holder::session(&session.id))
            .map_err(|why| self.ledger_fault(why))?;
        let roots = self.ledger_roots();
        Ok(held
            .iter()
            .find(|one| one.kind == "slot" && one.state == AttachmentState::Standing)
            .and_then(|one| {
                let slot = one.target.parse::<u32>().ok()?;
                let root = &roots.iter().find(|(id, _)| *id == one.manifest_id)?.1;
                Some(adapter_traits::slot_path(root.trim_end_matches('/'), slot))
            })
            .unwrap_or(session.cwd))
    }

    pub(crate) fn end_session_run(
        &self,
        stop: StopSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if self.session_runs().stop(&stop.run_id) {
            Ok(SessionRunUnderway { run_id: stop.run_id })
        } else {
            Err(self.not_in_flight())
        }
    }

    pub(crate) async fn type_into_session_run(
        &self,
        write: WriteSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if self.session_runs().write(&write.run_id, write.data).await {
            Ok(SessionRunUnderway { run_id: write.run_id })
        } else {
            Err(self.not_in_flight())
        }
    }

    pub(crate) fn resize_session_run_terminal(
        &self,
        resize: ResizeSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if resize.cols == 0 || resize.rows == 0 {
            return Err(self.no_cells());
        }
        if self.session_runs().resize(&resize.run_id, size(resize.cols, resize.rows)) {
            Ok(SessionRunUnderway { run_id: resize.run_id })
        } else {
            Err(self.not_in_flight())
        }
    }

    pub(crate) fn watch_session_run(&self, run_id: &str) -> Result<ObservedSessionRun, Refusal> {
        self.session_runs()
            .watch(run_id)
            .ok_or_else(|| refusal("Fleet holds no such run", self.run_id()))
    }

    fn not_in_flight(&self) -> Refusal {
        refusal("that run is not in flight", self.run_id())
    }

    fn no_cells(&self) -> Refusal {
        refusal("a terminal has at least one column and one row", self.run_id())
    }
}
