//! A call of an Operator that names a repository, a Job or a pull request outside its project is held: refused, and put
//! to him on a card of Fleet's own (`fleet_cards.rs`). **A repository two projects hold is inside both.** A repository no
//! project holds is nobody's, so it is not held.
//!
//! What a call names is read off the call as the door built it: its path for a repository or a pull request, its body for a
//! Job or a start. **A plain read of the arguments**: a call that names its target some other way, such as a start with no
//! `manifest_id`, which starts in the caller's own, is not held.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Caller;
use ipc::door::Call;
use ipc::{CloseSession, StartAnotherSessions, TellSession};

use crate::daemon::Fleet;

/// What a held call asked, as he reads it: how much of a message the card quotes.
const QUOTED: usize = 400;

/// `text` cut to what a card quotes.
fn quoted(text: &str) -> String {
    let text = text.trim();
    match text.char_indices().nth(QUOTED) {
        Some((at, _)) => format!("{}...", &text[..at]),
        None => text.to_string(),
    }
}

/// The segments of a call's path, its query left off.
fn segments(path: &str) -> Vec<&str> {
    path.split('?').next().unwrap_or_default().split('/').filter(|part| !part.is_empty()).collect()
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
    /// `Admitting::call_held`: why `call` is refused to the Operator at `caller`, where it names another project. `None` for
    /// any other caller and for a call that stays inside.
    pub(crate) async fn operator_call_held(&self, caller: Caller, call: &Call) -> Option<String> {
        let (session, _) = self.session_placed(&caller)?;
        if !self.session_is_the_operator(&session) {
            return None;
        }
        let from = self.operator_project_of(&session).await?;
        let (to, words) = self.call_names_outside(&from, call).await?;
        self.operator_held_across(&session, &from, &to, call.operation, &words).await
    }

    /// The project a call names that `from` is not in, and what it asked.
    async fn call_names_outside(&self, from: &str, call: &Call) -> Option<(String, String)> {
        let body = call.body.as_deref().unwrap_or_default().as_bytes();
        let parts = segments(&call.path);
        match call.operation {
            "start_another_session" => {
                let starts: StartAnotherSessions = ipc::decode_body("a start", body).ok()?;
                let named = starts.manifest_id.iter().map(|id| (id, starts.title.clone(), starts.message.clone())).chain(starts.jobs.iter().filter_map(|job| job.manifest_id.as_ref().map(|id| (id, Some(job.title.clone()), Some(job.message.clone())))));
                named.into_iter().find_map(|(manifest, title, message)| {
                    let to = self.operator_repository_outside(from, manifest.as_str())?;
                    let words = format!("start \"{}\": {}", title.unwrap_or_default(), quoted(&message.unwrap_or_default()));
                    Some((to, words))
                })
            }
            "close_session" => {
                let close: CloseSession = ipc::decode_body("an end", body).ok()?;
                let to = self.operator_job_outside(from, close.session_id.as_str()).await?;
                Some((to, format!("end Job {}", close.session_id.as_str())))
            }
            "tell_session" => {
                let tell: TellSession = ipc::decode_body("a message", body).ok()?;
                let to = self.operator_job_outside(from, tell.session_id.as_str()).await?;
                Some((to, quoted(&tell.text)))
            }
            // `/pull_requests/:repository/:number/merge`
            "merge_pull_request_by_number" | "enable_auto_merge" => {
                let (repository, number) = (parts.get(1)?, parts.get(2)?);
                let to = self.operator_repository_outside(from, repository)?;
                Some((to, format!("{} pull request #{number} in {repository}", if call.operation == "enable_auto_merge" { "merge when checks pass" } else { "merge" })))
            }
            // `/issues/:repository`, `/issues/:repository/:reference`, `/issue_queries/:repository/search`
            "list_issues" | "get_issue" | "search_issues" => {
                let repository = parts.get(1)?;
                let to = self.operator_repository_outside(from, repository)?;
                Some((to, format!("read the issues of {repository}")))
            }
            _ => None,
        }
    }
}
