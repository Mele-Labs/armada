//! One server, on its own task: `run` first, then `serve` held, `ready`
//! polled, and the end.
//!
//! **Output goes to `observe_server`'s channel and never to `/events`**, which
//! carries the three lifecycle facts. `serve` writes straight into the log —
//! the file is the pipe — and a pass over it every [`api::FOLLOW`] offers the
//! whole lines it grew by.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Served, Writing};
use ipc::{Event, ServerPhase, ServerState};
use tokio::sync::watch;
use verification::{Exit, NeverRan};

use super::held::Held;
use crate::daemon::Fleet;
use crate::ports::PortProbe;
use crate::rehearsing::records;

/// How often `ready` is asked — `crate::showing`'s interval for the evidence
/// harness's. **A command polled, never a duration waited out**: nothing here
/// decides a server is up because time passed.
const ASKING_EVERY: Duration = Duration::from_millis(250);

/// The most one ask of `ready` may take. A probe that hangs is ended and asked
/// again, rather than holding the server at `starting` for good.
const ONE_ASK: Duration = Duration::from_secs(30);

/// Everything the task needs, decided before it was spawned.
pub(crate) struct Plan {
    /// How a record left behind by a crashed Fleet says whose server this was.
    pub(crate) whose: String,
    pub(crate) run: Option<String>,
    pub(crate) serve: String,
    pub(crate) ready: Option<String>,
    pub(crate) worktree: PathBuf,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) dir: PathBuf,
    pub(crate) feed: api::RunFeed,
    /// Where the server stands. **Shared with `super::held::Live`**, so a
    /// merge that moved the checkout can write the row too — see
    /// [`super::held::Servers::moved_on`].
    pub(crate) now: std::sync::Arc<watch::Sender<ServerState>>,
}

struct End {
    exit: Exit,
    stopped: bool,
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
    /// The server, start to end. **Nothing here can fail the Job**: every
    /// failure is a fact on the instance.
    ///
    /// `server.exited` is published **before** the final state is sent, so a
    /// caller waiting on it — a Job's teardown — returns after the event is
    /// out, and the Job's own terminal event follows it rather than racing it.
    pub(crate) async fn served(&self, plan: Plan, held: Held, stopped: watch::Receiver<bool>) {
        let log = plan.dir.join(records::LOG);
        let (finished, finishing) = watch::channel(false);
        let lived = async {
            let end = self.lived(&plan, &log, stopped).await;
            let _ = finished.send(true);
            end
        };
        let servers = self.servers().clone();
        let heard = |lines: &[(u64, String)]| servers.lines_heard(&plan.now.borrow(), lines);
        let (end, ()) = tokio::join!(lived, pumped(&plan.feed, &log, finishing, heard));
        // Its group has been ended by now, on every road out of `lived`.
        super::left::forgotten(&plan.dir);
        let ended_at = ipc::Instant::from(&self.now());
        let exit_code = match end.exit {
            Exit::Code(code) => Some(code),
            _ => None,
        };
        let serving = plan.now.borrow().clone();
        let ended = match answering_now(&serving, &end).await {
            Some(port) => format!(
                "{}. Something else is answering on port {port} now, so the address still \
                 opens and what it draws is not this",
                said(&end.exit, end.stopped)
            ),
            None => said(&end.exit, end.stopped),
        };
        // **The row is read again as it is let go, never kept from above**: a
        // merge landing during the probe moved it, and a copy would drop that.
        let state = held.ended(&plan.now, |state| {
            state.phase = ServerPhase::Exited;
            state.ended_at = Some(ended_at);
            state.exit_code = exit_code;
            state.ended = Some(ended);
            state.stopped = end.stopped;
        });
        self.publish(Event::ServerExited(state.clone()));
        // **Before the final state is sent**, so a Studio holding this server
        // has kept what it said by the time anything waiting on `Exited`
        // returns — Fleet stopping every server is one such waiter, and a
        // write started after it would race the process going. `#1345`.
        self.kept_what_studios_hold_of_server(&state, &plan.dir)
            .await;
        plan.now.send_replace(state);
    }

    async fn lived(&self, plan: &Plan, log: &Path, stopped: watch::Receiver<bool>) -> End {
        let path = plan.worktree.as_path();
        let checkout = PathBuf::from(&plan.now.borrow().checkout.path);
        let first = plan.run.as_deref().unwrap_or(&plan.serve);
        if let Some((why, exit)) = unstartable(first, path, &checkout) {
            marked(log, &why);
            return End { exit, stopped: false };
        }
        if let Some(run) = &plan.run {
            marked(log, &format!("--- `run` first: {run} ---"));
            let attempt = checks_runner::run_until(
                run,
                path,
                self.budget().duration(),
                Writing::Appending(log),
                &plan.env,
                until(stopped.clone()),
                checks_runner::Priority::Normal,
            )
            .await;
            if *stopped.borrow() {
                return End {
                    exit: attempt.exit,
                    stopped: true,
                };
            }
            if attempt.exit != Exit::Code(0) {
                return End {
                    exit: Exit::NeverRan(NeverRan::PrerequisiteFailed {
                        command: String::from("run"),
                        run: run.clone(),
                        exit: Box::new(attempt.exit),
                    }),
                    stopped: false,
                };
            }
        }
        if plan.run.is_some() {
            if let Some((why, exit)) = unstartable(&plan.serve, path, &checkout) {
                marked(log, &why);
                return End { exit, stopped: false };
            }
        }
        marked(log, &format!("--- {} ---", plan.serve));
        let mut served = match Served::spawn_logging(&plan.serve, path, &plan.env, log) {
            Ok(served) => served,
            Err(why) => {
                return End {
                    exit: Exit::NeverRan(why),
                    stopped: false,
                }
            }
        };
        self.recorded_server(plan, &served).await;
        if let Some(ready) = &plan.ready {
            marked(log, &format!("--- waiting for `ready`: {ready} ---"));
            loop {
                let asked = async {
                    let answered = checks_runner::run_until(
                        ready,
                        path,
                        ONE_ASK,
                        Writing::Nowhere,
                        &plan.env,
                        std::future::pending(),
                        checks_runner::Priority::Normal,
                    )
                    .await;
                    let up = answered.exit == Exit::Code(0);
                    if !up {
                        tokio::time::sleep(ASKING_EVERY).await;
                    }
                    up
                };
                // The server is watched while `ready` is asked, not only
                // between asks: one that dies on its first line is the
                // ordinary shape of a port conflict, and reads as a failure
                // at once rather than as a server still starting.
                tokio::select! {
                    exit = served.exited() => {
                        served.end().await;
                        return End { exit, stopped: false };
                    }
                    () = until(stopped.clone()) => {
                        served.end().await;
                        return End { exit: Exit::Signalled { signal: libc::SIGKILL }, stopped: true };
                    }
                    up = asked => if up { break; }
                }
            }
        }
        self.now_serving(plan);
        // The group is ended on both roads out, so a server that fell over
        // leaves no child of its own holding the port.
        tokio::select! {
            exit = served.exited() => {
                served.end().await;
                End { exit, stopped: false }
            }
            () = until(stopped) => {
                served.end().await;
                End { exit: Exit::Signalled { signal: libc::SIGKILL }, stopped: true }
            }
        }
    }

    /// What finds this server again if Fleet crashes before it ends —
    /// [`super::left`]. A start time that will not read leaves no record,
    /// because a record nothing can confirm is one startup must not act on.
    async fn recorded_server(&self, plan: &Plan, served: &Served) {
        let Some(group) = served.group() else {
            return;
        };
        let id = plan.now.borrow().id.clone();
        let started = tokio::task::spawn_blocking(move || crate::process::holder_of(group)).await;
        if let Ok(Ok(crate::process::Holder::Held(started))) = started {
            let _ = super::left::recorded(&plan.dir, &id, &plan.whose, group, &started);
        }
    }

    /// **Changed in place, never from a copy**: a merge can move the same row
    /// from another thread, and a copy taken before it would drop its count.
    /// **Published under the same write lock**, as a merge's row is, so a
    /// reader is never handed this row after a newer one.
    fn now_serving(&self, plan: &Plan) {
        let since = ipc::Instant::from(&self.now());
        plan.now.send_modify(|state| {
            state.phase = ServerPhase::Serving;
            state.serving_since = Some(since);
            self.publish(Event::ServerServing(state.clone()));
        });
    }
}

/// The port this server's address is on, where something **else** is
/// answering there now that it has gone.
///
/// **The failure `#1577` names is not the collision — it is that the address
/// keeps answering, so nobody looks.** Another checkout of the same repository
/// declares the same number, and the moment this one lets go the next binder
/// takes it; the link a person is holding goes on opening, with somebody
/// else's build behind it.
///
/// **Only for a server that fell over**, never one a person stopped: a stop is
/// somebody letting the address go on purpose, and the probe would otherwise
/// race this process group's own teardown on every stop.
async fn answering_now(state: &ServerState, end: &End) -> Option<u16> {
    if end.stopped || state.serving_since.is_none() {
        return None;
    }
    let port = state.ports.first()?.port;
    let taken = tokio::task::spawn_blocking(move || !crate::ports::BindConnectProbe.free(port))
        .await
        .ok()?;
    taken.then_some(port)
}

/// Offer what the log has grown by, a pass every [`api::FOLLOW`], until the
/// server is over and its last line has gone out.
pub(crate) async fn pumped(feed: &api::RunFeed, log: &Path, mut finishing: watch::Receiver<bool>, heard: impl Fn(&[(u64, String)])) {
    let mut from = 0u64;
    loop {
        let last = *finishing.borrow();
        if let Some((lines, next)) = records::lines_from(log, from, last) {
            from = next;
            heard(&lines);
            feed.offer(api::RunChunk { lines });
        }
        if last {
            return;
        }
        tokio::select! {
            () = tokio::time::sleep(api::FOLLOW) => {}
            _ = finishing.changed() => {}
        }
    }
}

/// Resolves when something asks the server to stop. A stop that can no longer
/// arrive never resolves, so a server is never ended by its bookkeeping going.
pub(crate) async fn until(mut stopped: watch::Receiver<bool>) {
    if stopped.wait_for(|stop| *stop).await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// Why `command` cannot start in `folder`, as the line its log opens with, and the exit that says
/// so: the folder is not there, or its program is a path that is not there or will not run, or a
/// bare name no folder on `PATH` holds. **Asked before anything is spawned**, so a server that never
/// ran has a log that says why instead of an empty one.
fn unstartable(command: &str, folder: &Path, checkout: &Path) -> Option<(String, Exit)> {
    if !folder.is_dir() {
        let shown = folder.strip_prefix(checkout).unwrap_or(folder).display().to_string();
        return Some((
            format!("`{shown}` is not a folder in this checkout"),
            Exit::NeverRan(NeverRan::WorktreeGone { worktree: folder.display().to_string() }),
        ));
    }
    let (program, _) = checks_runner::split(command)?;
    if !program.contains('/') {
        let on_path = std::env::var_os("PATH")
            .is_some_and(|dirs| std::env::split_paths(&dirs).any(|dir| runnable(&dir.join(&program))));
        return (!on_path).then(|| {
            (format!("`{program}` is not on this machine's PATH"), Exit::NeverRan(NeverRan::NoSuchCommand { program }))
        });
    }
    let at = folder.join(&program);
    if !at.exists() {
        return Some((
            format!("`{program}` is not in this checkout"),
            Exit::NeverRan(NeverRan::NoSuchCommand { program }),
        ));
    }
    (!runnable(&at)).then(|| {
        (
            format!("`{program}` is not executable"),
            Exit::NeverRan(NeverRan::NotSpawned { program, kind: std::io::ErrorKind::PermissionDenied }),
        )
    })
}

/// A file with an execute bit.
fn runnable(at: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(at).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// A line of Fleet's own between commands, so one log reads as the sequence.
pub(crate) fn marked(log: &Path, line: &str) {
    // **One write, line and newline together.** The server appends to this
    // file at the same time, and `writeln!` issues two writes: its first line
    // could land between them, glued to the header.
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
        let _ = file.write_all(format!("{line}\n").as_bytes());
    }
}

/// How it ended, in a sentence. **A server that exits on its own has failed,
/// whatever its code**, so a zero is said as plainly as any other.
fn said(exit: &Exit, stopped: bool) -> String {
    if stopped {
        return String::from("was stopped");
    }
    match exit {
        Exit::Code(code) => {
            format!("stopped on its own, exit {code} — a server that exits has failed")
        }
        Exit::Signalled { signal } => format!("was ended by signal {signal} from outside Fleet"),
        Exit::TimedOut { after } => {
            format!(
                "its `run` ran past its {}s budget and was killed",
                after.as_secs()
            )
        }
        Exit::NeverRan(NeverRan::PrerequisiteFailed { run, exit, .. }) => {
            let how = match exit.as_ref() {
                Exit::Code(code) => format!("exited {code}"),
                other => said(other, false),
            };
            format!("never started: its `run`, `{run}`, {how}")
        }
        Exit::NeverRan(NeverRan::NothingToRun) => {
            String::from("declares a `serve` with no program in it")
        }
        Exit::NeverRan(NeverRan::NoSuchCommand { program }) if program.contains('/') => {
            format!("could not start: `{program}` is not in this checkout")
        }
        Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
            format!("needs `{program}`, which is not on this machine's PATH")
        }
        Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
            format!("could not start: {worktree} is not there")
        }
        Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
            format!("could not start `{program}`: {kind}")
        }
    }
}
