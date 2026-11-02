//! A red main, fixed by one Job Autopilot starts. **No call**: a red is fixed, and what a person
//! would check is that it is one Job, once a red commit, and not where another already holds it.
//!
//! The claim is the one a Job takes with `take_main_red` (`crate::red_claim`), made for the new Job
//! before it is briefed, so a Fleet on another machine that holds the red refuses this one too.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{AutopilotDecision, CloseSession, ManifestId, SessionId, SessionVoice, StartSession};
use store::TakenHow;

use super::decisions::Helping;
use crate::daemon::Fleet;
use crate::main_fix::brief_of;
use crate::red_claim::Taker;

const AUTOPILOT: &str = "Autopilot";

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
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
    /// Start a Job on this repository's red main, where it is red, no run of newer commits holds it
    /// back, nothing holds it and it has not been started for already. Answers how many it started.
    pub(super) async fn red_fixed(self: &Arc<Self>, helping: &Helping) -> usize {
        if !helping.kinds.contains(&AutopilotDecision::Red) {
            return 0;
        }
        let manifest = ManifestId::carried(&helping.kept.manifest_id);
        let Ok(served) = self.served_named(Some(&manifest)) else { return 0 };
        let Ok((served, main)) = self.red_of(served.root()).await else { return 0 };
        let id = format!("red:{}:{}", manifest.as_str(), main.red_commit());
        if self.decision_kept(&id).await || self.taking(&main).await.is_some() {
            return 0;
        }
        if self.red_is_free(&main, false).await.is_err() {
            return 0;
        }
        let started = self
            .start_session(StartSession {
                manifest_id: manifest.clone(),
                title: Some("Fix main".to_string()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
            })
            .await;
        let Ok(record) = started else { return 0 };
        let job = record.id.as_str().to_string();
        let close = |job: &str| CloseSession { session_id: SessionId::carried(job) };
        if self.claim_red(&main, &Taker::Session(job.clone()), false).await.is_err() {
            let _ = self.closed_and_parked(close(&job)).await;
            return 0;
        }
        let facts = self.with_the_log(&served, &main, brief_of(&main)).await;
        let said = format!("Main went red at {}. Autopilot has claimed the red for you. Fix it.\n\n{facts}", short(main.red_commit()));
        let voice = SessionVoice::Session { id: AUTOPILOT.to_lowercase(), title: AUTOPILOT.to_string() };
        if self.first_message_from(&job, voice, &said).await.is_err() {
            self.release_red(&main).await;
            let _ = self.closed_and_parked(close(&job)).await;
            return 0;
        }
        self.recorded_session_take(&main, &job, TakenHow::Dispatched, None).await;
        let asked = format!("Main is red at {}", short(main.red_commit()));
        let because = "A red main is fixed by one Job, once for each red commit.".to_string();
        self.decision_kept_as(helping, &id, Some(&job), "Fix main", AutopilotDecision::Red, asked, "Start a Job to fix it", because).await;
        1
    }
}
