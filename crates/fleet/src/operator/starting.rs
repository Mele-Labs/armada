//! A Job the Operator starts on work nobody owns: a note, an error, a bug report. **It follows the repository's
//! `agent_starts` as any Job an agent starts does**: running at once with the brief its first turn, or held on
//! its start card with the brief for the person, which is the ask. What the person starts himself is his own.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{CloseSession, ManifestId, SessionVoice, StartSession};

use crate::daemon::Fleet;

const OPERATOR: &str = "Operator";

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
    /// A Job titled `title` started in `manifest` on `brief`, in the Operator's name. Its id, or why none was
    /// started: a Job that could not be briefed is closed, so none is left standing empty.
    pub(super) async fn operator_job_started(self: &Arc<Self>, manifest: &str, title: &str, brief: &str) -> Result<String, &'static str> {
        let found = self.found_elsewhere(manifest, title, brief).await;
        let brief = match found.told() {
            Some(told) => format!("{brief}\n\n{told}"),
            None => brief.to_string(),
        };
        let brief = brief.as_str();
        let held = self.agent_start_held(None, manifest).await;
        let start = StartSession {
            project: None,
            manifest_id: ManifestId::carried(manifest),
            title: Some(title.to_string()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        let record = self.start_session(start).await.map_err(|_| "the Job did not start")?;
        let job = record.id.as_str().to_string();
        let briefed = match held {
            true => self.held_with(&job, brief).await.is_ok(),
            false => {
                let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
                self.first_message_from(&job, voice, brief).await.is_ok()
            }
        };
        if !briefed {
            let _ = self.closed_and_parked(CloseSession { session_id: record.id }).await;
            return Err("the Job was not briefed");
        }
        self.elsewhere_logged(&job, manifest, title, &found).await;
        Ok(job)
    }
}
