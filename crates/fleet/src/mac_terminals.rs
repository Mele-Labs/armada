//! A Job's terminals across his Macs. **The Mac that holds the Job runs the shell**, in that Job's slot; the other
//! Mac opens it, types into it and follows what it prints through the signed link, with the same calls Bridge makes
//! at home. `docs/concepts/pocket.md`, *Another Mac*.
//!
//! **The holding Mac** answers only for a Job in a shared project, and only for a run that is one of that Job's
//! open terminals, so no other run of it is reached by its id. **The other Mac** keeps, in memory, which Mac and
//! Job each run it opened belongs to, so a keystroke or resize on that run is sent there.
//!
//! **Nothing is queued.** A keystroke the link cannot carry is refused, never typed on wake, and a stream that
//! drops ends as a run a signal ended, so the terminal shows it ended.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ObservedSessionRun, Refusal, SessionRunEnd, SessionRunEvent, Sessions, TerminalShared, SESSION_RUN_BACKLOG};
use ipc::{JobTerminals, SessionId, SessionRunMessage, SessionRunUnderway, TerminalFromMac, TerminalOfJob, WireError};
use tokio::sync::broadcast;

use crate::daemon::Fleet;
use crate::other_macs::{MAC_UNREACHABLE, MAC_UNREADABLE};
use crate::phone::OtherMacFollowed;
use crate::session_host::NO_SUCH_SESSION;

/// A run of a Job held from another Mac, opened here.
#[derive(Clone)]
pub(crate) struct Held {
    mac: String,
    job: SessionId,
    /// Held while one keystroke is sent, so what he types arrives in the order he typed it.
    typing: Arc<tokio::sync::Mutex<()>>,
}

/// The runs of Jobs held from other Macs this Mac has opened, by run id.
#[derive(Default)]
pub(crate) struct Followed(Mutex<HashMap<String, Held>>);

impl Followed {
    fn keep(&self, mac: &str, job: &SessionId, run_id: &str) {
        let mut held = self.0.lock().expect("not poisoned");
        held.entry(run_id.to_string())
            .or_insert_with(|| Held { mac: mac.to_string(), job: job.clone(), typing: Arc::default() });
    }

    fn held(&self, run_id: &str) -> Option<Held> {
        self.0.lock().expect("not poisoned").get(run_id).cloned()
    }

    fn ended(&self, run_id: &str) {
        self.0.lock().expect("not poisoned").remove(run_id);
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
    // ---- the Mac that holds the Job: what it is asked -------------------------------------------------------

    /// `share_job_terminal`: the act, run as at home, where Job and run pass [`Fleet::terminal_of_shared_job`].
    pub(crate) async fn job_terminal_shared(self: Arc<Self>, asked: TerminalFromMac) -> Result<TerminalShared, Refusal> {
        self.act_from_other_mac(asked.session_id()).await?;
        match asked {
            TerminalFromMac::Open(open) => self.open_terminal(open).await.map(TerminalShared::Run),
            TerminalFromMac::List { session_id } => {
                let asked = ipc::ListTerminals { session_id: Some(session_id) };
                self.job_terminals_listed(None, asked).await.map(TerminalShared::Terminals)
            }
            TerminalFromMac::Write { session_id, run_id, data } => {
                self.terminal_of_shared_job(&session_id, &run_id)?;
                self.write_session_run(ipc::WriteSessionRun { run_id, data }).await.map(TerminalShared::Run)
            }
            TerminalFromMac::Resize { session_id, run_id, cols, rows } => {
                self.terminal_of_shared_job(&session_id, &run_id)?;
                self.resize_session_run(ipc::ResizeSessionRun { run_id, cols, rows }).await.map(TerminalShared::Run)
            }
            TerminalFromMac::Stop { session_id, run_id } => {
                self.terminal_of_shared_job(&session_id, &run_id)?;
                self.stop_session_run(ipc::StopSessionRun { run_id }).await.map(TerminalShared::Run)
            }
        }
    }

    /// `share_job_terminal_output`: what the terminal printed and prints next, refused as `share_job_terminal` is.
    pub(crate) async fn job_terminal_output_shared(&self, which: TerminalOfJob) -> Result<ObservedSessionRun, Refusal> {
        self.act_from_other_mac(&which.session_id).await?;
        self.terminal_of_shared_job(&which.session_id, &which.run_id)?;
        self.watch_session_run(&which.run_id)
    }

    /// `run_id` is one of Job `job`'s open terminals. **Any other run is answered as a Job that does not exist**,
    /// so another Mac learns nothing of what else runs here.
    fn terminal_of_shared_job(&self, job: &SessionId, run_id: &str) -> Result<(), Refusal> {
        if self.session_runs().shells_open(job.as_str()).iter().any(|(_, open)| open == run_id) {
            return Ok(());
        }
        Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("No Job has the id {}.", job.as_str()), self.run_id())))
    }

    // ---- this Mac, for a Job another Mac holds -----------------------------------------------------------

    /// `open_terminal` of Job `open.session_id`, which Mac `mac` holds: opened there, and its run kept as that Mac's.
    pub(crate) async fn terminal_opened_on_other_mac(&self, mac: &str, open: ipc::OpenTerminal) -> Result<SessionRunUnderway, Refusal> {
        let job = open.session_id.clone();
        let opened: SessionRunUnderway = self.terminal_sent(mac, &TerminalFromMac::Open(open)).await?;
        self.phone_held().terminals.keep(mac, &job, &opened.run_id);
        Ok(opened)
    }

    /// `list_terminals` of Job `job`, which Mac `mac` holds, and every run in it kept as that Mac's.
    pub(crate) async fn terminals_listed_on_other_mac(&self, mac: &str, job: SessionId) -> Result<JobTerminals, Refusal> {
        let listed: JobTerminals = self.terminal_sent(mac, &TerminalFromMac::List { session_id: job.clone() }).await?;
        for one in &listed.terminals {
            self.phone_held().terminals.keep(mac, &job, &one.run_id);
        }
        Ok(listed)
    }

    /// A keystroke, resize or stop on run `run_id` of a Job held from another Mac, sent there at once and one at a
    /// time. **One the link cannot carry is refused**, and never sent later.
    pub(crate) async fn terminal_act_on_other_mac(&self, run_id: &str, act: impl FnOnce(SessionId) -> TerminalFromMac) -> Option<Result<SessionRunUnderway, Refusal>> {
        let held = self.phone_held().terminals.held(run_id)?;
        let _typing = held.typing.lock().await;
        Some(self.terminal_sent(&held.mac, &act(held.job.clone())).await)
    }

    /// What run `run_id` of a Job held from another Mac prints, followed through the link. **The stream ending
    /// without `exited`, as when that Mac sleeps or the link drops, ends the run here as a signal's**, and a
    /// keystroke after is refused as on a run that has ended.
    pub(crate) async fn terminal_followed_on_other_mac(&self, run_id: &str) -> Option<Result<ObservedSessionRun, Refusal>> {
        let held = self.phone_held().terminals.held(run_id)?;
        Some(self.terminal_followed(held, run_id).await)
    }

    async fn terminal_followed(&self, held: Held, run_id: &str) -> Result<ObservedSessionRun, Refusal> {
        let path = format!(
            "/other_mac/terminal/observe?session_id={}&run_id={}",
            ipc::door::encoded(held.job.as_str()),
            ipc::door::encoded(run_id)
        );
        let followed = self.phone_switch()?.follow_other_mac(&held.mac, &path).await;
        let followed = followed.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))?;
        let mut lines = match followed {
            OtherMacFollowed::Open(lines) => lines,
            OtherMacFollowed::Refused { status, body } => return Err(self.refusal_passed_on(&held.mac, status, &body)),
        };
        let (sender, live) = broadcast::channel(SESSION_RUN_BACKLOG);
        let (terminals, run) = (Arc::clone(&self.phone_held().terminals), run_id.to_string());
        tokio::spawn(async move {
            while let Some(line) = lines.recv().await {
                let said = match ipc::decode::<SessionRunMessage>("a terminal's output from another Mac", line.as_bytes()) {
                    Ok(said) => said,
                    Err(_) => continue,
                };
                let event = match said {
                    SessionRunMessage::Output { text } => SessionRunEvent::Output(text),
                    SessionRunMessage::Exited { code, stopped } => SessionRunEvent::Exited(SessionRunEnd { code, stopped }),
                    SessionRunMessage::Opened { .. } | SessionRunMessage::Missed { .. } => continue,
                };
                let exited = matches!(event, SessionRunEvent::Exited(_));
                // No viewer left: stop following, and the run stays open there.
                if sender.send(event).is_err() {
                    return;
                }
                if exited {
                    terminals.ended(&run);
                    return;
                }
            }
            terminals.ended(&run);
            let _ = sender.send(SessionRunEvent::Exited(SessionRunEnd { code: None, stopped: false }));
        });
        Ok(ObservedSessionRun { run_id: run_id.to_string(), history: String::new(), live: Ok(live) })
    }

    /// `asked`, sent to Mac `mac`'s `/other_mac/terminal`, and its answer read as `T`. What that Mac refuses is
    /// passed on as it said it.
    async fn terminal_sent<T: serde::de::DeserializeOwned>(&self, mac: &str, asked: &TerminalFromMac) -> Result<T, Refusal> {
        let body = ipc::encode(asked).map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREADABLE, why.to_string(), self.run_id())))?;
        let answered = self.phone_switch()?.send_other_mac(mac, "/other_mac/terminal", body.into_bytes()).await;
        let (status, bytes) = answered.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))?;
        if (200..300).contains(&status) {
            return ipc::decode("a terminal's answer from another Mac", &bytes).map_err(|why| {
                Refusal::Fault(WireError::raised(MAC_UNREADABLE, format!("Mac {mac} sent an answer this build cannot read: {why}"), self.run_id()))
            });
        }
        Err(self.refusal_passed_on(mac, status, &bytes))
    }
}
