//! A shell action run from a flight control: the person's own line, in the worktree slot a Session
//! holds, answered when it has ended. `docs/concepts/kit.md`, *Flight controls*.
//!
//! **The slot is the ledger's**, the one the Session holds standing. A Session that holds none is
//! refused rather than run in its directory, which is what `crate::session_running` falls back
//! to: a tile pressed with nothing to press it on must never touch the main checkout.
//! **Standard output and error share one pipe**, so they come back in the order they arrived.

use std::io::Read;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{RunSessionShell, SessionShellRan, WireError};
use store::{AttachmentState, Holder};

use crate::daemon::Fleet;
use crate::session_running::{shell, Ended};

/// How long a command runs before it is stopped.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(120);
/// How long a stopped group has to leave before it is killed.
const GRACE: Duration = Duration::from_secs(3);
/// How long output still in the pipe is read for after the shell has exited. A background process
/// the command left holding the pipe open must not hold the answer.
const DRAIN: Duration = Duration::from_millis(300);
/// Bytes of output kept: the last of it.
pub(crate) const TAIL: usize = 64 * 1024;

/// A shell action that was refused: a 422.
const REFUSED: &str = "fleet.session_shell_refused";

#[derive(Default)]
struct Kept {
    bytes: Vec<u8>,
    truncated: bool,
}

/// Signal the whole group with `kill`, which takes `-<pgid>`: the gate lists the few places that
/// speak `unsafe`, and this is not one of them.
fn signal(pgid: u32, signal: &str) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-s", signal, "--", &format!("-{pgid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// `$SHELL -lc <command>` in `dir`, until it ends or `timeout` stops it.
pub(crate) async fn run_in(
    dir: &str,
    command: &str,
    timeout: Duration,
) -> std::io::Result<SessionShellRan> {
    let (mut reader, writer) = std::io::pipe()?;
    let mut launch = tokio::process::Command::new(shell());
    launch
        .args(["-lc", command])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .process_group(0)
        .kill_on_drop(true);
    let mut child = launch.spawn()?;
    // The command holds the only other ends. Held here too, the pipe would never read as ended.
    drop(launch);

    let kept = Arc::new(Mutex::new(Kept::default()));
    let reading = {
        let kept = Arc::clone(&kept);
        tokio::task::spawn_blocking(move || {
            let mut chunk = [0u8; 8192];
            while let Ok(read) = reader.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                let mut kept = kept.lock().expect("not poisoned");
                kept.bytes.extend_from_slice(&chunk[..read]);
                if kept.bytes.len() > TAIL {
                    let over = kept.bytes.len() - TAIL;
                    kept.bytes.drain(..over);
                    kept.truncated = true;
                }
            }
        })
    };

    let (status, timed_out) = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => (status?, false),
        Err(_) => {
            let pgid = child.id();
            if let Some(pgid) = pgid {
                signal(pgid, "TERM");
            }
            if tokio::time::timeout(GRACE, child.wait()).await.is_err() {
                if let Some(pgid) = pgid {
                    signal(pgid, "KILL");
                }
            }
            (child.wait().await?, true)
        }
    };
    let _ = tokio::time::timeout(DRAIN, reading).await;

    let kept = std::mem::take(&mut *kept.lock().expect("not poisoned"));
    // The cut may have fallen inside a character.
    let from = kept.bytes.iter().position(|byte| byte & 0xC0 != 0x80).unwrap_or(0);
    Ok(SessionShellRan {
        exit: if timed_out { None } else { status.code() },
        timed_out,
        output: String::from_utf8_lossy(&kept.bytes[from..]).into_owned(),
        truncated: kept.truncated,
    })
}

fn refusal(why: impl Into<String>, run_id: ipc::RunId) -> Refusal {
    Refusal::Unacceptable(WireError::raised(REFUSED, why, run_id))
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
    /// `run_session_shell`. The Session is told how it ended, as it is for a block run from its
    /// thread.
    pub(crate) async fn shell_action_run(
        self: Arc<Self>,
        run: RunSessionShell,
    ) -> Result<SessionShellRan, Refusal> {
        self.shell_action_run_within(run, TIMEOUT).await
    }

    pub(crate) async fn shell_action_run_within(
        self: Arc<Self>,
        run: RunSessionShell,
        timeout: Duration,
    ) -> Result<SessionShellRan, Refusal> {
        if run.command.trim().is_empty() {
            return Err(refusal("a shell action needs a command", self.run_id()));
        }
        let dir = {
            let store = self.store().lock().await;
            let session = store
                .session(run.session_id.as_str())
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| refusal("no job Fleet knows by that id", self.run_id()))?;
            if session.state == store::SessionState::Ended {
                return Err(refusal("the job has ended", self.run_id()));
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
        }
        .ok_or_else(|| {
            refusal(
                "the job holds no worktree slot, and a shell action never runs in the main checkout",
                self.run_id(),
            )
        })?;
        let ran = run_in(&dir, &run.command, timeout).await.map_err(|why| {
            refusal(format!("the command could not start in {dir}: {why}"), self.run_id())
        })?;
        Arc::clone(&self)
            .session_run_ended(Ended {
                session: run.session_id.as_str().to_string(),
                command: run.command,
                code: ran.exit,
                stopped: ran.timed_out,
                output: ran.output.clone(),
            })
            .await;
        Ok(ran)
    }
}
