//! What follows filing an issue: Then. A go-ahead to file an issue, a note he left or an error
//! Bridge showed him, offers `just_file`, `file_and_start` and, **only where a live Job already works on
//! the same files**, `file_and_hand`.
//!
//! **Fleet finds that Job from what it already keeps**: the files each live Job has written and its plan
//! names, read against the paths the note or error itself names, as the Operator's own call is shown
//! them (`candidates::near`). A Job near it only by sharing words is not one that works the same files.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::operator_page::{OperatorHandTo, OperatorThen, OperatorThenOffer};
use ipc::{Notice, SessionId, SessionVoice, StartSession, WaitingItem};
use store::KeptSuggestion;

use super::arriving::{Arrival, LiveJob};
use super::candidates::near;
use super::suggesting::{arrival_of, suggestion_number};
use super::turning::Done;
use super::Operator;
use crate::daemon::Fleet;

const OPERATOR: &str = "Operator";

impl Operator {
    /// Every hosted Job not ended, as the Operator's own calls are shown them.
    pub(super) async fn live_jobs(&self) -> Vec<LiveJob> {
        self.desk.live().await
    }
}

/// The paths `text` names: words with a `/` in them that end in a file's extension or a folder's slash.
pub(super) fn paths_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in text.split(|one: char| one.is_whitespace() || matches!(one, ',' | ';' | ':' | '(' | ')' | '"' | '\'' | '`' | '<' | '>' | '[' | ']')) {
        let path = word.trim_end_matches('.').trim_start_matches("./");
        let last = path.rsplit('/').next().unwrap_or_default();
        if path.contains('/') && !path.contains("//") && (path.ends_with('/') || last.contains('.')) && !found.iter().any(|one| one == path) {
            found.push(path.to_string());
        }
    }
    found
}

/// The Thens a go-ahead offers, where it is one to file an issue. `live` is the Jobs to hand it to.
pub(super) fn thens_of(kept: &KeptSuggestion, live: &[LiveJob]) -> Vec<OperatorThenOffer> {
    if kept.way != "file" {
        return Vec::new();
    }
    let mut offers = vec![OperatorThenOffer { then: OperatorThen::JustFile, hand_to: None, pick: false }, OperatorThenOffer { then: OperatorThen::FileAndStart, hand_to: None, pick: false }];
    let Some(mut arrival) = arrival_of(kept) else { return offers };
    arrival.paths = paths_in(&kept.words);
    if let Some(shared) = near(&arrival, live).into_iter().find(|one| !one.paths.is_empty()) {
        let hand_to = OperatorHandTo { session_id: SessionId::carried(&shared.job.session_id), job: shared.job.title };
        offers.push(OperatorThenOffer { then: OperatorThen::FileAndHand, hand_to: Some(hand_to), pick: false });
    }
    offers
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
    /// The Thens `item` offers now: none unless it is a go-ahead to file an issue.
    pub(super) async fn thens_offered(&self, item: &WaitingItem) -> Vec<OperatorThenOffer> {
        let Some(n) = suggestion_number(&item.id) else { return Vec::new() };
        let Some(kept) = self.store().lock().await.operator_suggestion(n).ok().flatten().filter(|kept| kept.way == "file") else { return Vec::new() };
        let live = match self.operator_seat().operator() {
            Some(operator) => operator.live_jobs().await,
            None => Vec::new(),
        };
        thens_of(&kept, &live)
    }

    /// The Then after an issue was filed for `arrival`: a Job started on it, or the issue handed to
    /// the live Job `hand_to` works. **The issue stands whatever becomes of this**, so a Then that fails
    /// is said and not refused.
    pub(super) async fn then_done(self: &Arc<Self>, arrival: &Arrival, issue: Option<&str>, then: OperatorThen, hand_to: Option<&str>, done: &mut Done) -> Result<(), String> {
        let named = issue.map(|issue| format!("It was filed as {issue}. ")).unwrap_or_default();
        match then {
            OperatorThen::JustFile => Ok(()),
            OperatorThen::FileAndStart => {
                let started = self
                    .start_session(StartSession {
                        project: None,
                        manifest_id: ipc::ManifestId::carried(&arrival.manifest_id),
                        title: Some(arrival.subject.clone()),
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
                    .await;
                let record = started.map_err(|_| "the Job did not start".to_string())?;
                let job = record.id.as_str().to_string();
                let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string(), mac: None };
                let brief = format!("The Operator filed this for the person and started you on it. {named}Do what it asks.\n\n{}", arrival.words);
                if self.first_message_from(&job, voice, &brief).await.is_err() {
                    let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
                    return Err("the Job was not briefed".to_string());
                }
                done.started = Some(job);
                Ok(())
            }
            OperatorThen::FileAndHand => {
                let into = hand_to.ok_or("no live Job works the same files")?;
                let text = format!("The Operator filed this for the person and hands it to you, as your work is on the same files. {named}\n\n{}\n\nDo it as part of your work, or say why it is not yours.", arrival.words);
                if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
                    return Err("the Job did not take it".to_string());
                }
                done.handed_to = Some(into.to_string());
                Ok(())
            }
        }
    }
}
