//! A red main, fixed by one Job Autopilot starts. **No call**: a red is fixed, and what a person
//! would check is that it is one Job, once a red commit, and not where another already holds it.
//! **With the Operator on, the red is its arrival instead**, and it rules a new Job, a live Job
//! near the red, or a raise; a red a Job already holds never arrives.
//!
//! The claim is the one a Job takes with `take_main_red` (`crate::red_claim`), made for the new Job
//! before it is briefed, so a Fleet on another machine that holds the red refuses this one too.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{AutopilotDecision, CloseSession, ManifestId, Notice, SessionId, SessionVoice, StartSession};
use store::{MainCi, TakenBy, TakenHow};

use super::decisions::Helping;
use crate::daemon::Fleet;
use crate::main_fix::brief_of;
use crate::operator::RedMain;
use crate::red_claim::Taker;
use crate::repositories::Served;

const AUTOPILOT: &str = "Autopilot";
const OPERATOR: &str = "Operator";

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
        if self.decision_kept(&id).await {
            return 0;
        }
        if self.operator_on() {
            let fixing = self.taking(&main).await.map(|TakenBy::Session(job)| job);
            let red = RedMain { manifest_id: manifest.as_str().to_string(), commit: main.red_commit().to_string(), seen: brief_of(&main), fixing };
            if let Some(arrival) = red.arrival() {
                self.hand_once(arrival).await;
            }
            return 0;
        }
        if self.taking(&main).await.is_some() {
            return 0;
        }
        let Some(job) = self.red_started(&served, &main, AUTOPILOT, "").await else { return 0 };
        let asked = format!("Main is red at {}", short(main.red_commit()));
        let because = "A red main is fixed by one Job, once for each red commit.".to_string();
        self.decision_kept_as(helping, &id, Some(&job), "Fix main", AutopilotDecision::Red, asked, "Start a Job to fix it", because).await;
        1
    }

    /// A Job titled "Fix main" started on `main`'s red, the red claimed for it and its brief said in
    /// `by`'s name. **Nothing where the red is not free**, or the Job would not take its brief.
    /// `extra` is what the brief ends with, such as how the fix is to be got in.
    pub(crate) async fn red_started(self: &Arc<Self>, served: &Served, main: &MainCi, by: &str, extra: &str) -> Option<String> {
        self.red_is_free(main, false).await.ok()?;
        let started = self
            .start_session(StartSession {
                project: None,
                manifest_id: ManifestId::carried(served.manifest().id().as_str()),
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
                on_your_words: None,
            })
            .await;
        let job = started.ok()?.id.as_str().to_string();
        let close = |job: &str| CloseSession { session_id: SessionId::carried(job) };
        if self.claim_red(main, &Taker::Session(job.clone()), false).await.is_err() {
            let _ = self.closed_and_parked(close(&job)).await;
            return None;
        }
        let facts = self.with_the_log(served, main, brief_of(main)).await;
        let said = format!("Main went red at {}.\n\n{facts}{extra}", short(main.red_commit()));
        let voice = SessionVoice::Session { id: by.to_lowercase(), title: by.to_string() };
        if self.first_message_from(&job, voice, &said).await.is_err() {
            self.release_red(main).await;
            let _ = self.closed_and_parked(close(&job)).await;
            return None;
        }
        self.recorded_session_take(main, &job, TakenHow::Dispatched, None).await;
        Some(job)
    }

    /// The red the Operator ruled on, where it is still `commit`'s and no Job holds it.
    pub(crate) async fn red_still(&self, manifest: &str, commit: &str) -> Option<(Served, MainCi)> {
        let served = self.served_named(Some(&ManifestId::carried(manifest))).ok()?;
        let (served, main) = self.red_of(served.root()).await.ok()?;
        if main.red_commit() != commit || self.taking(&main).await.is_some() {
            return None;
        }
        Some((served, main))
    }

    /// The Operator ruled a new Job for the red: Autopilot's own start, in the Operator's name.
    pub(crate) async fn operator_fixed_red(self: &Arc<Self>, manifest: &str, commit: &str) {
        let Some((served, main)) = self.red_still(manifest, commit).await else { return };
        self.red_started(&served, &main, OPERATOR, "").await;
    }

    /// Whether `commit`'s red in `manifest` stands with no Job on it.
    pub(crate) async fn red_is_standing(&self, manifest: &str, commit: &str) -> bool {
        self.red_still(manifest, commit).await.is_some()
    }

    /// The person pressed Start a fix: a Job on `commit`'s red at once, **with no plan step**. The Job's
    /// id, or why none was started.
    pub(crate) async fn operator_started_fix(self: &Arc<Self>, manifest: &str, commit: &str) -> Result<String, String> {
        let (served, main) = self.red_still(manifest, commit).await.ok_or("main is no longer red there, or a Job is on it already")?;
        let how = "\n\nJust get the fix in and have a pull request ready. There is no plan step: do not write a plan or wait for one to be approved.";
        self.red_started(&served, &main, OPERATOR, how).await.ok_or_else(|| "the Job did not start".to_string())
    }

    /// The Operator ruled the red to live Job `job`, whose work is near it: the red is claimed for
    /// it, and it is told, as `take_main_red` would have it.
    pub(crate) async fn operator_gave_red(self: &Arc<Self>, manifest: &str, commit: &str, job: &str) {
        let Some((served, main)) = self.red_still(manifest, commit).await else { return };
        if self.red_is_free(&main, false).await.is_err() || self.claim_red(&main, &Taker::Session(job.to_string()), false).await.is_err() {
            return;
        }
        let facts = self.with_the_log(&served, &main, brief_of(&main)).await;
        let said = format!("Main went red at {}.\n\n{facts}", short(main.red_commit()));
        if !matches!(self.told_by_fleet(job, &said, Notice::Operator).await, Ok(true)) {
            self.release_red(&main).await;
            return;
        }
        self.recorded_session_take(&main, job, TakenHow::SentBack, None).await;
    }
}
