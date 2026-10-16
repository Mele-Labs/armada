//! What a hosted session's agent runs beside its turn: a shell command it backgrounded, or a
//! watch. Each is one `background` row on the ledger. `docs/concepts/session.md`, *The ledger*.
//!
//! **Detected off the session's own stream, never reported.** The call that asked is remembered
//! when it is heard (`Calls`), and the harness's `task_started` for it names the task and says it
//! runs beside the turn; its status change and notice end it. A subagent's backgrounded command is
//! heard the same way, so it is the session's row too. A task whose call was not a shell command
//! or a watch (a subagent, say) names no remembered call and so has no row, and an end for an id
//! that never started here settles nothing.
//!
//! **A standing row does not make the session working.** The agent's turn is what is running, and
//! a command it left behind is not the person's to wait on: the agent is woken when it ends.

use std::collections::VecDeque;

use api::Sessions;
use adapter_traits::{AgentHarness, Delivery, DroneEvent, TaskStatus, Vcs, WorkProduct};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionReport};
use store::{AttachmentState, Holder};

use crate::daemon::Fleet;

/// The kind a background task is kept under.
pub(super) const KIND: &str = "background";

/// How many calls are remembered. A task starts within moments of its call, so this is how many a
/// subagent's burst of commands may be ahead of the line that names the first.
const REMEMBERED: usize = 64;

/// What a shell or watch call asked, by the id of the call. In memory.
#[derive(Default)]
pub(crate) struct Calls(VecDeque<(String, &'static str, String)>);

impl Calls {
    fn told(&mut self, call: &str, form: &'static str, command: &str) {
        self.0.push_back((call.to_string(), form, command.to_string()));
        while self.0.len() > REMEMBERED {
            self.0.pop_front();
        }
    }

    fn take(&mut self, call: &str) -> Option<(&'static str, String)> {
        let at = self.0.iter().position(|(held, _, _)| held == call)?;
        self.0.remove(at).map(|(_, form, command)| (form, command))
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
    /// One thing the session's process or a subagent in it said, as it bears on its background tasks.
    pub(super) async fn background_heard(&self, id: &str, event: &DroneEvent) {
        match event {
            DroneEvent::Called { tool, call, detail } => {
                if let Some(form) = adapters::background_form(tool) {
                    let command = detail.whole().unwrap_or(detail.shown());
                    self.hosts().of(id).state().background.told(call, form, command);
                }
            }
            DroneEvent::TaskStarted { task, call, description, backgrounded: true } => {
                let asked = self.hosts().of(id).state().background.take(call);
                let Some((form, command)) = asked else { return };
                let detail = [("form", form), ("task", description.as_str()), ("command", command.as_str())]
                    .into_iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect();
                let _ = self
                    .report_session(SessionReport {
                        harness: String::from(adapters::HOSTED_HARNESS),
                        session_id: SessionId::carried(id),
                        fact: SessionFact::Attached {
                            attachment: AttachmentReport { kind: KIND.into(), target: task.clone(), detail },
                        },
                    })
                    .await;
            }
            DroneEvent::TaskEnded { task, status, output, exit } => {
                self.background_ended(id, task, *status, output.as_deref(), *exit).await;
            }
            _ => {}
        }
    }

    /// The task ended: spent, with how. **Two lines end it and each adds what it has**, in either
    /// order, so the row is settled by the first and filled in by the second.
    async fn background_ended(&self, id: &str, task: &str, status: TaskStatus, output: Option<&str>, exit: Option<i32>) {
        let holder = Holder::session(id);
        let now = self.now().as_str().to_string();
        let changed = {
            let mut store = self.store().lock().await;
            let held = store.attachments_of(&holder).unwrap_or_default();
            let Some(row) = held.into_iter().find(|one| one.kind == KIND && one.target == task) else {
                return;
            };
            let mut detail = row.detail.clone();
            detail.insert("status".into(), status.as_str().into());
            if let Some(output) = output {
                detail.insert("output".into(), output.into());
            }
            if let Some(exit) = exit {
                detail.insert("exit".into(), exit.to_string());
            }
            matches!(store.follow(&row, AttachmentState::Spent, &detail, &now), Ok(true))
        };
        if changed {
            let _ = self.published_hosted(id).await;
        }
    }

    /// The background tasks a replaced process had not finished, one line each. **Called where a new
    /// process starts for a session that has run**: they ran in the process that is gone, so none of
    /// them can still be running. Their rows are given back.
    pub(crate) async fn cut_off_background(&self, id: &str) -> Vec<String> {
        let holder = Holder::session(id);
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let held = store.attachments_of(&holder).unwrap_or_default();
        let mut cut = Vec::new();
        for one in held.iter().filter(|one| one.kind == KIND && one.state == AttachmentState::Standing) {
            let _ = store.settle(&holder, &one.kind, &one.manifest_id, &one.target, AttachmentState::GivenBack, &at);
            let said = |key: &str| one.detail.get(key).map(String::as_str).unwrap_or_default();
            cut.push(format!("{} \"{}\" ({})", said("form"), said("task"), one.target));
        }
        cut
    }
}
