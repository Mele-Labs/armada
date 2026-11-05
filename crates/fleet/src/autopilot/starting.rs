//! The Session a pick starts: the issue as its brief, sent at once.
//!
//! **The road a Dispatch and `start_another_session` take**, `start_session`, and then the first
//! message by the road any message takes, so the process starts and the turn runs under the rules
//! any message is under. The picker has read the issue already, so the proposer is not asked
//! again: the pick names the workflow, and the Session starts on the defaults for the rest.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{CloseSession, SessionId, SessionTag, SessionVoice, SetSessionWorkflow, StartSession, TagKind};
use store::KeptBugReport;

use crate::daemon::Fleet;
use crate::enriching::{enrich, Enriched};
use crate::repositories::Served;

/// Who the brief is from in the Session's thread.
const AUTOPILOT: &str = "Autopilot";

/// The address a reported bug is offered and picked under, in place of a tracker's.
pub(super) const REPORT_URL: &str = "report:";

/// The issue a Session is started on.
pub(super) struct Issue<'a> {
    pub url: &'a str,
    pub title: &'a str,
    /// What the tracker listed of its body, for when the issue cannot be read in full.
    pub excerpt: &'a str,
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
    /// A Session on `issue`, running on its brief. Answers its id. **A Session that cannot be
    /// sent its brief is closed**: one left idle would hold a place for nothing.
    ///
    /// `because` is the picker's reason, said at the foot of the brief where Autopilot chose.
    pub(super) async fn session_on_issue(
        self: &Arc<Self>,
        served: &Served,
        issue: &Issue<'_>,
        workflow: Option<&str>,
        because: Option<&str>,
    ) -> Result<String, Refusal> {
        // A reported bug has no page to read: it is briefed from the report, and held by the Job.
        let report = match issue.url.strip_prefix(REPORT_URL) {
            Some(id) => self.store().lock().await.bug_report(id).ok().flatten(),
            None => None,
        };
        let brief = match (&report, issue.url.starts_with(REPORT_URL)) {
            (Some(report), _) => foot(report_brief(report), because),
            (None, true) => foot(issue.title.to_string(), because),
            (None, false) => self.issue_brief(served, issue, because).await,
        };
        let record = self
            .start_session(StartSession {
                manifest_id: ipc::ManifestId::carried(served.manifest().id().as_str()),
                title: Some(issue.title.to_string()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await?;
        let id = record.id.as_str().to_string();
        // A workflow the repository no longer holds, or workflows switched off, starts it plain.
        if let Some(named) = workflow {
            let set = SetSessionWorkflow {
                session_id: SessionId::carried(&id),
                workflow: Some(named.to_string()),
            };
            if let Err(why) = Arc::clone(self).set_session_workflow(set).await {
                eprintln!("autopilot: job {id} starts without workflow `{named}`: {why:?}");
            }
        }
        let voice = SessionVoice::Session { id: AUTOPILOT.to_lowercase(), title: AUTOPILOT.to_string() };
        if let Err(why) = self.first_message_from(&id, voice, &brief).await {
            let _ = self.closed_and_parked(CloseSession { session_id: SessionId::carried(&id) }).await;
            return Err(why);
        }
        if let Some(report) = report {
            let tag = SessionTag { kind: TagKind::Report, id: report.id, title: report.title, url: None };
            self.reports_named(&id, served.manifest().id().as_str(), &[tag]).await;
        }
        Ok(id)
    }

    /// The issue as a Session is briefed on it: its address, then what the tracker holds of it,
    /// read the way a dispatch from its address reads it, else its title and excerpt.
    async fn issue_brief(&self, served: &Served, issue: &Issue<'_>, because: Option<&str>) -> String {
        let links = self.links_of(served);
        let body = match enrich(issue.url, links).await {
            Enriched::Resolved(text) => format!("{}\n\n{text}", issue.url),
            Enriched::AsGiven | Enriched::Failed => {
                format!("{}\n\n{}\n\n{}", issue.url, issue.title, issue.excerpt)
            }
        };
        foot(body, because)
    }
}

/// `body`, with the picker's reason said at its foot where Autopilot chose.
fn foot(body: String, because: Option<&str>) -> String {
    match because {
        Some(why) => format!("{}\n\nAutopilot picked this issue: {why}", body.trim_end()),
        None => body.trim_end().to_string(),
    }
}

/// A report as a brief, the parts Bridge's own brief of one has: the title, what happened, then the
/// steps numbered, the errors and the logs, each left out where empty.
pub(crate) fn report_brief(report: &KeptBugReport) -> String {
    let mut brief = format!("{}\n\n{}", report.title, report.what);
    let mut part = |name: &str, lines: Vec<String>| {
        if !lines.is_empty() {
            brief.push_str(&format!("\n\n{name}\n{}", lines.join("\n")));
        }
    };
    part("Steps", report.steps.iter().enumerate().map(|(at, one)| format!("{}. {one}", at + 1)).collect());
    part("Errors", report.errors.clone());
    part("Logs", report.logs.clone());
    brief
}
