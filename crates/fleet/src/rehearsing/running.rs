//! The run itself, on its own task: the snapshot, a Check's prerequisites,
//! the command, its output onto the run's own channel, and the record.
//!
//! **Output goes to the run's channel and never to `/events`**, which
//! carries only `checkout_run.finished`. Each pass over the log offers the whole lines
//! it found with their byte offsets; the log keeps every one of them.

use std::io::Write;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Writing;
use core_model::Timestamp;
use tokio::sync::watch;
use verification::{Exit, NeverRan};

use super::entries::Entry;
use super::owner::{Place, Tree};
use super::record::{Record, Underway};
use super::verifying::{Handed, HANDING_OVER};
use super::{records, Held};
use crate::daemon::Fleet;

/// Everything a run needs, decided before it was spawned.
///
/// Which tree this is, whose ports it draws and where its record goes were all
/// settled in [`Place`]; what follows runs one command in one directory and
/// writes one record.
pub(crate) struct Plan {
    pub(crate) place: Place,
    pub(crate) tree: Tree,
    pub(crate) entry: Entry,
    pub(crate) underway: Underway,
    pub(crate) dir: PathBuf,
    /// Where the run's output goes. Dropped when the run returns, which with
    /// [`Held`] going is what tells every viewer the run finished.
    pub(crate) feed: api::RunFeed,
    /// The Verify this run is a step of, handed the record before it is
    /// published — `super::verifying`. `None` for a run a person started.
    pub(crate) verify: Option<tokio::sync::oneshot::Sender<Handed>>,
}

struct Outcome {
    exit: Exit,
    stopped: bool,
    required: Vec<String>,
}

fn seconds(at: &Timestamp) -> i64 {
    at.epoch_millis().unwrap_or_default().div_euclid(1_000)
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
    /// The run, start to record. Every failure is a fact on the record.
    pub(crate) async fn rehearsed(
        &self,
        mut plan: Plan,
        held: Held,
        stopped: watch::Receiver<bool>,
        done: watch::Sender<Option<Record>>,
    ) {
        let verify = plan.verify.take();
        let log = plan.dir.join(records::LOG);
        let started = self.now();
        let snapshot = {
            let (path, id, at) = (
                plan.tree.path.clone(),
                plan.underway.id.clone(),
                seconds(&started),
            );
            tokio::task::spawn_blocking(move || adapters::snapshot::snapshot(&path, &id, at)).await
        };
        let (finished, finishing) = watch::channel(false);
        let ran = async {
            let outcome = self.ran(&plan, &log, stopped).await;
            let _ = finished.send(true);
            outcome
        };
        let (outcome, ()) = tokio::join!(ran, pumped(&plan.feed, &log, finishing));
        let ended = self.now();
        let (changed, changed_unreadable, reference) = match snapshot {
            Ok(Ok(taken)) => settled(&plan.tree.path, taken, seconds(&ended)).await,
            Ok(Err(why)) => (Vec::new(), Some(why.to_string()), None),
            Err(_) => (
                Vec::new(),
                Some(String::from("the snapshot before the run did not finish")),
                None,
            ),
        };
        let record = Record {
            id: plan.underway.id.clone(),
            name: plan.underway.name.clone(),
            command: plan.underway.command.clone(),
            required: outcome.required,
            started_at: plan.underway.started_at.clone(),
            ended_at: ipc::Instant::from(&ended),
            duration_ms: u64::try_from(crate::clock::elapsed(&started, &ended).as_millis())
                .unwrap_or(u64::MAX),
            exit_code: match outcome.exit {
                Exit::Code(code) => Some(code),
                _ => None,
            },
            expect_exit_code: plan.entry.expect_exit_code,
            ended: said(&outcome.exit, outcome.stopped),
            stopped: outcome.stopped,
            changed,
            changed_unreadable,
            snapshot: reference,
            undone_at: None,
            log: records::relative_log(&plan.place.handle, &plan.underway.id),
            workspace: plan.underway.workspace.clone(),
            requester: plan.underway.requester.clone(),
        };
        let _ = records::write(&plan.dir, &record);
        // Given back before anybody is told, so a caller that starts the next
        // run on `run.finished` is not refused as though this one were out.
        drop(held);
        // A person's workspace run claimed its file's ports for itself; a
        // Verify's steps share the Verify's, given back when it ends.
        if verify.is_none() {
            if let Some(within) = plan.tree.within.as_ref() {
                self.released_workspace_ports(within).await;
            }
        }
        // A Verify's next step takes the slot just given back before anybody
        // is told, so a reader of the sheet on this event finds it out.
        if let Some(verify) = verify {
            let (ack, acked) = tokio::sync::oneshot::channel();
            let record = record.clone();
            if verify.send(Handed { record, ack }).is_ok() {
                let _ = tokio::time::timeout(HANDING_OVER, acked).await;
            }
        }
        let _ = done.send(Some(record.clone()));
        self.publish(ipc::Event::CheckoutRunFinished(record.of_checkout()));
    }

    /// A Check's prerequisites, in order, then the command — into one log.
    ///
    /// **The main checkout's claimed span.** `${port.NAME}` is resolved in each
    /// command before it runs and `ARMADA_PORT_<NAME>` plus any declared `env`
    /// ride in the process's own environment — see `crate::ports`. Both are
    /// empty where the Manifest declares no `ports:`. A workspace Verify's own
    /// file lays its ports over the root's — `super::workspace::Within`.
    async fn ran(&self, plan: &Plan, log: &Path, stopped: watch::Receiver<bool>) -> Outcome {
        let budget = self.budget().duration();
        let dir = plan.tree.commands_in();
        let path = dir.as_path();
        let (mut ports, mut env) = match plan.place.checkout.served() {
            Some(served) => (
                self.main_checkout_ports(served).await,
                self.main_checkout_port_env(served).await,
            ),
            // No root Manifest, so no ports are declared to lease.
            None => Default::default(),
        };
        if let Some(within) = plan.tree.within.as_ref() {
            within.overlaid(&mut ports, &mut env);
        }
        let mut required = Vec::new();
        for needed in &plan.entry.requires {
            if *stopped.borrow() {
                return Outcome {
                    exit: Exit::Signalled {
                        signal: libc::SIGKILL,
                    },
                    stopped: true,
                    required,
                };
            }
            let run = crate::ports::resolve_ports(needed.run(), &ports);
            marked(log, &format!("--- `{}` first: {} ---", needed.name(), run));
            let attempt = checks_runner::run_until(
                &run,
                path,
                budget,
                Writing::Appending(log),
                &env,
                until(stopped.clone()),
                checks_runner::Priority::Normal,
            )
            .await;
            required.push(needed.name().to_string());
            if attempt.exit != Exit::Code(0) {
                let was_stopped = *stopped.borrow();
                return Outcome {
                    exit: Exit::NeverRan(NeverRan::PrerequisiteFailed {
                        command: needed.name().to_string(),
                        run,
                        exit: Box::new(attempt.exit),
                    }),
                    stopped: was_stopped,
                    required,
                };
            }
        }
        let command = crate::ports::resolve_ports(&plan.underway.command, &ports);
        if !required.is_empty() {
            marked(log, &format!("--- {command} ---"));
        }
        let attempt = checks_runner::run_until(
            &command,
            path,
            budget,
            Writing::Appending(log),
            &env,
            until(stopped.clone()),
            checks_runner::Priority::Normal,
        )
        .await;
        let was_stopped = *stopped.borrow();
        Outcome {
            exit: attempt.exit,
            stopped: was_stopped,
            required,
        }
    }
}

/// Offer what the log has grown by, a pass every [`api::FOLLOW`], until the
/// run is over and its last line has gone out.
async fn pumped(feed: &api::RunFeed, log: &Path, mut finishing: watch::Receiver<bool>) {
    let mut from = 0u64;
    loop {
        let last = *finishing.borrow();
        if let Some((lines, next)) = records::lines_from(log, from, last) {
            from = next;
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

/// The tree after the run, against the one before. **A snapshot that would
/// not settle is let go**: its ref still names the tree before, and an Undo
/// from it would read HEAD as the run's starting point.
async fn settled(
    worktree: &Path,
    taken: adapters::snapshot::Snapshot,
    at: i64,
) -> (Vec<ipc::ChangedFile>, Option<String>, Option<String>) {
    let reference = taken.reference().to_string();
    match tokio::task::spawn_blocking(move || taken.settle(at)).await {
        Ok(Ok(settled)) => (
            settled.changed.iter().map(wired).collect(),
            None,
            Some(settled.reference),
        ),
        failed => {
            let _ = adapters::snapshot::forget(worktree, &reference);
            let why = match failed {
                Ok(Err(why)) => why.to_string(),
                _ => String::from("reading the tree after the run did not finish"),
            };
            (Vec::new(), Some(why), None)
        }
    }
}

pub(super) fn wired(file: &adapter_traits::ChangedFile) -> ipc::ChangedFile {
    ipc::ChangedFile {
        path: file.path().to_string(),
        change: crate::wire::kind(file.change()),
    }
}

/// Resolves when a person presses Stop. A stop that can no longer arrive
/// never resolves, so a run is never ended by its bookkeeping going away.
async fn until(mut stopped: watch::Receiver<bool>) {
    if stopped.wait_for(|stop| *stop).await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// A line of Fleet's own between commands, so one log reads as the sequence.
fn marked(log: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

/// How it ended, in a sentence. Unhued wherever it is drawn: a rehearsal.
fn said(exit: &Exit, stopped: bool) -> String {
    if stopped {
        return String::from("was stopped");
    }
    match exit {
        Exit::Code(code) => format!("exited {code}"),
        Exit::Signalled { signal } => format!("was ended by signal {signal}"),
        Exit::TimedOut { after } => {
            format!("ran past its {}s budget and was killed", after.as_secs())
        }
        Exit::NeverRan(NeverRan::NothingToRun) => {
            String::from("declares a `run` with no program in it")
        }
        Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
            format!("needs `{program}`, which is not on this machine's PATH")
        }
        Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
            format!("could not run: {worktree} is not there")
        }
        Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
            format!("could not start `{program}`: {kind}")
        }
        Exit::NeverRan(NeverRan::PrerequisiteFailed { command, run, exit }) => format!(
            "did not run: `{command}` (`{run}`), which it requires first, {}",
            said(exit, false)
        ),
    }
}
