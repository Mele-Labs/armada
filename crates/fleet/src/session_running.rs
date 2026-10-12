//! A shell block run from a Session's thread: the person's own command, in the Session's slot
//! worktree (or its directory where it holds none), with its output held in memory.
//!
//! **Nothing is written down.** A run keeps its last [`TAIL`] lines for as long as it lives and
//! [`KEPT_AFTER_EXIT`] after it ends, so a viewer that opens late reads what happened. A restart
//! forgets every run, which is also what happens to its processes' parent.
//!
//! **The history and the subscription are taken under one lock**, the one the reader appends
//! under, so a viewer loses no line and reads none twice.

use std::collections::{BTreeMap, VecDeque};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ObservedSessionRun, Refusal, SessionRunEvent, SESSION_RUN_BACKLOG};
use ipc::{SessionRunUnderway, StartSessionRun, StopSessionRun, WireError};
use store::{AttachmentState, Holder};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;

use crate::daemon::Fleet;

/// Lines of output a run keeps.
const TAIL: usize = 2000;
/// How long a run is held after it exits.
const KEPT_AFTER_EXIT: Duration = Duration::from_secs(90);
/// How long a stopped group has to leave before it is killed.
const GRACE: Duration = Duration::from_secs(3);
/// How long output still in the pipe is read for after the shell has exited.
const DRAIN: Duration = Duration::from_millis(300);
/// Lines sent as one message, at most.
const BATCH: usize = 200;

/// A run that was refused: a 422.
const REFUSED: &str = "fleet.session_run_refused";

struct State {
    tail: VecDeque<String>,
    /// `Some` once the run ended, with its code.
    exited: Option<Option<i32>>,
}

struct Run {
    pgid: i32,
    stopped: AtomicBool,
    state: Mutex<State>,
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

    fn lines(&self, lines: Vec<String>) {
        let mut state = self.state.lock().expect("not poisoned");
        state.tail.extend(lines.iter().cloned());
        while state.tail.len() > TAIL {
            state.tail.pop_front();
        }
        let _ = self.feed.send(SessionRunEvent::Lines(lines));
    }

    fn exit(&self, code: Option<i32>) {
        let mut state = self.state.lock().expect("not poisoned");
        state.exited = Some(code);
        let _ = self.feed.send(SessionRunEvent::Exited(code));
    }
}

/// Every run Fleet holds, in flight or lately ended. Never written down.
#[derive(Default)]
pub(crate) struct SessionRuns {
    held: Arc<Mutex<BTreeMap<String, Arc<Run>>>>,
    minted: AtomicU64,
}

fn refusal(why: impl Into<String>, run_id: ipc::RunId) -> Refusal {
    Refusal::Unacceptable(WireError::raised(REFUSED, why, run_id))
}

fn shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.trim().is_empty())
        .unwrap_or_else(|| "/bin/zsh".into())
}

impl SessionRuns {
    fn get(&self, run_id: &str) -> Option<Arc<Run>> {
        self.held.lock().expect("not poisoned").get(run_id).cloned()
    }

    /// Start `command` in `dir` and answer its id. The reading and the wait are tasks of their own.
    fn start(&self, command: &str, dir: &str) -> std::io::Result<String> {
        let (reader, writer) = std::io::pipe()?;
        let mut child = {
            let mut process = tokio::process::Command::new(shell());
            process
                .arg("-lc")
                .arg(command)
                .current_dir(dir)
                .stdin(Stdio::null())
                .stdout(writer.try_clone()?)
                .stderr(writer)
                .process_group(0);
            process.spawn()?
        };
        let reader = tokio::net::unix::pipe::Receiver::from_owned_fd(reader.into())?;
        let run = Arc::new(Run {
            pgid: child.id().map_or(0, |pid| pid as i32),
            stopped: AtomicBool::new(false),
            state: Mutex::new(State {
                tail: VecDeque::new(),
                exited: None,
            }),
            feed: broadcast::channel(SESSION_RUN_BACKLOG).0,
        });
        let id = format!("session-run-{}", self.minted.fetch_add(1, Ordering::Relaxed) + 1);
        self.held
            .lock()
            .expect("not poisoned")
            .insert(id.clone(), run.clone());

        let printing = tokio::spawn(read_lines(reader, run.clone()));
        let held = self.held.clone();
        let forget = id.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            // A grandchild holding the pipe open must not keep the run from ending.
            let mut printing = printing;
            if tokio::time::timeout(DRAIN, &mut printing).await.is_err() {
                printing.abort();
            }
            let code = if run.stopped.load(Ordering::Relaxed) {
                None
            } else {
                status.ok().and_then(|status| status.code())
            };
            run.exit(code);
            tokio::time::sleep(KEPT_AFTER_EXIT).await;
            held.lock().expect("not poisoned").remove(&forget);
        });
        Ok(id)
    }

    /// SIGTERM, then SIGKILL if the group is still there after [`GRACE`].
    fn stop(&self, run_id: &str) -> bool {
        let Some(run) = self.get(run_id).filter(|run| !run.ended()) else {
            return false;
        };
        run.stopped.store(true, Ordering::Relaxed);
        run.signal("TERM");
        tokio::spawn(async move {
            tokio::time::sleep(GRACE).await;
            if !run.ended() {
                run.signal("KILL");
            }
        });
        true
    }

    fn watch(&self, run_id: &str) -> Option<ObservedSessionRun> {
        let run = self.get(run_id)?;
        // Subscribe and copy under the lock lines are appended under.
        let state = run.state.lock().expect("not poisoned");
        Some(ObservedSessionRun {
            run_id: run_id.to_string(),
            history: state.tail.iter().cloned().collect(),
            live: match state.exited {
                Some(code) => Err(code),
                None => Ok(run.feed.subscribe()),
            },
        })
    }
}

/// Read the merged output as lines, a burst of what is already buffered at a time.
async fn read_lines(reader: tokio::net::unix::pipe::Receiver, run: Arc<Run>) {
    let mut reader = BufReader::new(reader);
    let mut raw = Vec::new();
    loop {
        let mut lines = Vec::new();
        loop {
            raw.clear();
            match reader.read_until(b'\n', &mut raw).await {
                Ok(0) | Err(_) => {
                    if !lines.is_empty() {
                        run.lines(lines);
                    }
                    return;
                }
                Ok(_) => {
                    let text = String::from_utf8_lossy(&raw);
                    lines.push(text.trim_end_matches(['\n', '\r']).to_string());
                }
            }
            if lines.len() >= BATCH || reader.buffer().is_empty() {
                break;
            }
        }
        run.lines(lines);
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
        &self,
        start: StartSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if start.command.trim().is_empty() {
            return Err(refusal("a run needs a command", self.run_id()));
        }
        let dir = {
            let store = self.store().lock().await;
            let session = store
                .session(start.session_id.as_str())
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| refusal("no session Fleet knows by that id", self.run_id()))?;
            if session.state == store::SessionState::Ended {
                return Err(refusal("the session has ended", self.run_id()));
            }
            let held = store
                .attachments_of(&Holder::session(&session.id))
                .map_err(|why| self.ledger_fault(why))?;
            let roots = self.ledger_roots();
            held.iter()
                .find(|one| one.kind == "slot" && one.state == AttachmentState::Standing)
                .and_then(|one| {
                    let slot = one.target.parse::<u32>().ok()?;
                    let root = &roots.iter().find(|(id, _)| *id == one.manifest_id)?.1;
                    Some(adapter_traits::slot_path(root.trim_end_matches('/'), slot))
                })
                .unwrap_or(session.cwd)
        };
        match self.session_runs().start(&start.command, &dir) {
            Ok(run_id) => Ok(SessionRunUnderway { run_id }),
            Err(why) => Err(refusal(
                format!("the command could not start in {dir}: {why}"),
                self.run_id(),
            )),
        }
    }

    pub(crate) fn end_session_run(
        &self,
        stop: StopSessionRun,
    ) -> Result<SessionRunUnderway, Refusal> {
        if self.session_runs().stop(&stop.run_id) {
            Ok(SessionRunUnderway { run_id: stop.run_id })
        } else {
            Err(refusal("that run is not in flight", self.run_id()))
        }
    }

    pub(crate) fn watch_session_run(&self, run_id: &str) -> Result<ObservedSessionRun, Refusal> {
        self.session_runs()
            .watch(run_id)
            .ok_or_else(|| refusal("Fleet holds no such run", self.run_id()))
    }
}
