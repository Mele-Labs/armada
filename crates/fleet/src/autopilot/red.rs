//! A red main, fixed by one Job. **Fleet starts it whatever the Autopilot switch says** (the
//! owner's ruling of 13 Nov 2026, `.claude/decisions/2026-11-13-claims-store.md`): every reading of
//! main that finds a red no Job has taken starts one, once a red commit, and not where another Mac
//! already holds it. The Operator's own go-ahead, Start a fix, is the same start in its name.
//!
//! The claim is the one a Job takes with `take_main_red` (`crate::red_claim`), made for the new Job
//! before it is briefed, so a Fleet on another machine racing for the same red closes its own Job
//! unbriefed: first claim wins.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{CloseSession, ManifestId, Notice, SessionId, SessionVoice, StartSession};
use store::{MainCi, TakenHow};

use crate::daemon::Fleet;
use crate::main_fix::brief_of;
use crate::red_claim::Taker;
use crate::repositories::Served;

const FLEET: &str = "Fleet";
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
    /// Every served repository's red main that no Job has taken gets one fix Job, once a red commit.
    /// **No Autopilot gate**: a red is fixed, and a take that ended without a fix leaves the red to
    /// the band's buttons. Answers how many started.
    pub(crate) async fn reds_fixed(self: &Arc<Self>) -> usize {
        let mut started = 0;
        for served in self.repositories().served() {
            let Ok((served, main)) = self.red_of(served.root()).await else { continue };
            let Some(red_at) = main.red_at.as_ref() else { continue };
            let taken = self.store().lock().await.main_fixes_of(&main.repository, red_at).is_ok_and(|takes| !takes.is_empty());
            if taken {
                continue;
            }
            if self.red_started(&served, &main, FLEET, "", false).await.is_some() {
                started += 1;
            }
        }
        started
    }

    /// A Job titled "Fix main" started on `main`'s red, the red claimed for it and its brief said in
    /// `by`'s name. **Nothing where the red is not free**, or the Job would not take its brief.
    /// `extra` is what the brief ends with, such as how the fix is to be got in. **`held` leaves the Job on its
    /// start card with the brief for the person**, which is how a Job the Operator starts asks.
    pub(crate) async fn red_started(self: &Arc<Self>, served: &Served, main: &MainCi, by: &str, extra: &str, held: bool) -> Option<String> {
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
                held,
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
        let voice = SessionVoice::Session { id: by.to_lowercase(), title: by.to_string(), mac: None };
        let briefed = match held {
            true => self.held_with(&job, &said).await.is_ok(),
            false => self.first_message_from(&job, voice, &said).await.is_ok(),
        };
        if !briefed {
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
        let held = self.agent_start_held(None, manifest).await;
        self.red_started(&served, &main, OPERATOR, "", held).await;
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
        self.red_started(&served, &main, OPERATOR, how, false).await.ok_or_else(|| "the Job did not start".to_string())
    }

    /// The Operator ruled the red to live Job `job`, whose work is near it: the red is claimed for
    /// it, and it is told, as `take_main_red` would have it. Why not, where it was not.
    pub(crate) async fn operator_gave_red(self: &Arc<Self>, manifest: &str, commit: &str, job: &str) -> Result<(), String> {
        let (served, main) = self.red_still(manifest, commit).await.ok_or("main is no longer red there, or a Job is on it already")?;
        if self.red_is_free(&main, false).await.is_err() || self.claim_red(&main, &Taker::Session(job.to_string()), false).await.is_err() {
            return Err("the red could not be taken for that Job".to_string());
        }
        let facts = self.with_the_log(&served, &main, brief_of(&main)).await;
        let said = format!("Main went red at {}.\n\n{facts}", short(main.red_commit()));
        if !matches!(self.told_by_fleet(job, &said, Notice::Operator).await, Ok(true)) {
            self.release_red(&main).await;
            return Err("the Job did not take the red".to_string());
        }
        self.recorded_session_take(&main, job, TakenHow::SentBack, None).await;
        Ok(())
    }
}
