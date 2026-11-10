//! A saved request's runs, kept as the send finishes and read back as a page and as one run whole.
//!
//! **A run is the request's, not the Job's**: it is keyed by the repository and the collection's and
//! request's own ids, written whether or not a Job's thread is open, and read after the Job ends.
//! The store holds each request's newest [`store::RUNS_KEPT`] (`store::request_runs`).
//!
//! **Through the agent's door a run is cut as an agent's own answer is**: no `set-cookie`, and a
//! body at [`AGENT_ANSWER_CAP`], whoever sent the run.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, RunsAsked};
use ipc::{
    RequestBy, RequestHeader, RequestRun, RequestRunAnswer, RequestRunJob, RequestRunSent, RequestRunStatus,
    RequestRunSummary, RequestRuns, SessionId, WireError,
};
use store::{KeptRequestRun, StatusWanted};

use super::{cut, AGENT_ANSWER_CAP};
use crate::daemon::Fleet;

/// A run's id that is none of Fleet's, or a read that names none. A 422.
const RUN_REFUSED: &str = "fleet.request_run_refused";
/// The repository keeps no such run. A 404.
const NO_SUCH_RUN: &str = "fleet.no_such_request_run";
/// The store would not keep or read a run. A 500.
const RUNS_UNREAD: &str = "fleet.request_runs_unread";

/// A page's length where the read names none, and the most it may name.
const PAGE: usize = 50;
const MOST_A_PAGE: usize = 200;
/// How much of a request's body a list's line starts with.
const SUMMARY: usize = 80;

fn by_word(by: RequestBy) -> &'static str {
    match by {
        RequestBy::Owner => "owner",
        RequestBy::Agent => "agent",
    }
}

fn by_of(word: &str) -> RequestBy {
    match word {
        "agent" => RequestBy::Agent,
        _ => RequestBy::Owner,
    }
}

fn headers(pairs: &[(String, String)]) -> Vec<RequestHeader> {
    pairs.iter().map(|(name, value)| RequestHeader { name: name.clone(), value: value.clone() }).collect()
}

/// The start of a request's body on one line, and nothing where it sent none.
fn summary_of(body: Option<&str>) -> String {
    let line = body.unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&line, SUMMARY).0
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
    /// Keep the run `done` came to, where it was a saved request's and a repository is placed.
    /// **The answer is the caller's whether or not the store took the run**: a store that would
    /// not answers the sentence for the send's `note`, so the caller is told the run is missing.
    pub(super) async fn run_kept(&self, root: Option<&str>, job: Option<&str>, done: &super::Sent) -> Option<String> {
        let root = root?;
        let mut store = self.store().lock().await;
        // Only a Job the ledger knows is named, so a name Fleet does not know is never kept.
        let job = job.and_then(|job| {
            let session = store.session(job).ok().flatten()?;
            Some((job.to_string(), session.title))
        });
        let run = done.run(root, job, self.instant().as_str())?;
        store.keep_request_run(&run).err().map(|why| format!("The run was not kept: {why}."))
    }

    fn run_unread(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(RUNS_UNREAD, format!("The runs of a saved request did not read: {why}"), self.run_id()))
    }

    fn run_refused(&self, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(RUN_REFUSED, why, self.run_id()))
    }

    /// A run's id as the store holds it.
    fn run_number(&self, id: &str) -> Result<i64, Refusal> {
        id.trim().parse().map_err(|_| self.run_refused(format!("`{id}` is not a run's id: take one from `list_request_runs`")))
    }

    /// `job`'s title as the ledger holds it now, else the one kept with the run.
    fn run_job(store: &store::Store, kept: &KeptRequestRun) -> Option<RequestRunJob> {
        let id = kept.run.session_id.clone()?;
        let title = store.session(&id).ok().flatten().and_then(|session| session.title).or_else(|| kept.run.session_title.clone());
        Some(RequestRunJob { id: SessionId::carried(id), title })
    }

    pub(super) async fn request_runs_page(&self, root: &str, asked: &RunsAsked) -> Result<RequestRuns, Refusal> {
        let before = asked.before.as_deref().map(|id| self.run_number(id)).transpose()?;
        let most = asked.limit.unwrap_or(PAGE).clamp(1, MOST_A_PAGE);
        let by = asked.by.map(by_word);
        let status = asked.status.map(|status| match status {
            RequestRunStatus::Success => StatusWanted::Class(2),
            RequestRunStatus::Redirect => StatusWanted::Class(3),
            RequestRunStatus::ClientError => StatusWanted::Class(4),
            RequestRunStatus::ServerError => StatusWanted::Class(5),
            RequestRunStatus::Failed => StatusWanted::Failed,
        });
        let store = self.store().lock().await;
        // One more than the page, so a last page is known to be the last.
        let mut found = store
            .request_runs(&store::RunsAsked {
                repository: root,
                collection_id: &asked.collection,
                request_id: &asked.request,
                sent_by: by,
                status,
                before,
                most: most + 1,
            })
            .map_err(|why| self.run_unread(why))?;
        let more = found.len() > most;
        found.truncate(most);
        let next = more.then(|| found.last().map(|kept| kept.id.to_string())).flatten();
        let runs = found
            .iter()
            .map(|kept| RequestRunSummary {
                id: kept.id.to_string(),
                at: ipc::Instant::carried(kept.run.sent_at.clone()),
                by: by_of(&kept.run.sent_by),
                job: Self::run_job(&store, kept),
                method: kept.run.method.clone(),
                url: kept.run.url.clone(),
                status: kept.run.status,
                ms: kept.run.ms,
                error: kept.run.error.clone(),
                summary: summary_of(kept.run.request_body.as_deref()),
            })
            .collect();
        Ok(RequestRuns { runs, next })
    }

    pub(super) async fn request_run_whole(&self, root: &str, id: &str, reader: RequestBy) -> Result<RequestRun, Refusal> {
        let number = self.run_number(id)?;
        let store = self.store().lock().await;
        let Some(kept) = store.request_run(root, number).map_err(|why| self.run_unread(why))? else {
            return Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_RUN,
                format!("This repository keeps no run {number}: it was dropped past the newest runs of its request, or it is another repository's."),
                self.run_id(),
            )));
        };
        let job = Self::run_job(&store, &kept);
        let run = &kept.run;
        let mut answered = run.response_headers.clone();
        let (mut body, mut truncated) = (run.response_body.clone(), run.response_cut);
        if reader == RequestBy::Agent {
            answered.retain(|(name, _)| !name.eq_ignore_ascii_case("set-cookie"));
            let (cut_body, cut_here) = cut(&body, AGENT_ANSWER_CAP);
            (body, truncated) = (cut_body, truncated || cut_here);
        }
        let content_type = answered
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.clone());
        Ok(RequestRun {
            id: kept.id.to_string(),
            at: ipc::Instant::carried(run.sent_at.clone()),
            by: by_of(&run.sent_by),
            job,
            collection: run.collection_id.clone(),
            request: run.request_id.clone(),
            environment: run.environment.clone(),
            sent: RequestRunSent {
                method: run.method.clone(),
                url: run.url.clone(),
                headers: headers(&run.request_headers),
                body: run.request_body.clone(),
            },
            answer: RequestRunAnswer {
                status: run.status,
                ms: run.ms,
                headers: headers(&answered),
                body,
                truncated,
                bytes: run.response_bytes,
                content_type,
                error: run.error.clone(),
            },
        })
    }
}
