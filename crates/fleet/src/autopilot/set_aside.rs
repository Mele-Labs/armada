//! An issue the picker was unsure of, started when the helper is sure. The start is the person's own
//! press on the pick (`dispatch_set_aside`), which claims it before the Session starts, so a press
//! and a pass make one Session between them.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Autopilots;
use ipc::{AutopilotDecision, ManifestId};

use super::bounds::PickState;
use super::picking::Candidate;
use super::deciding::Deciding;
use super::decisions::Helping;
use super::destructive::destructive_text;
use crate::daemon::Fleet;
use crate::operator::HELD_FOR_OVERLAP;

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
    /// Start each set-aside pick of this run the helper is sure of. Answers how many it started.
    pub(super) async fn set_asides_started(self: &Arc<Self>, helping: &mut Helping) -> usize {
        if !helping.kinds.contains(&AutopilotDecision::Unsure) {
            return 0;
        }
        let run = helping.kept.run_id.clone();
        let aside: Vec<_> = self
            .store()
            .lock()
            .await
            .autopilot_picks(&run)
            .unwrap_or_default()
            .into_iter()
            .filter(|pick| pick.state == PickState::SetAside.as_stored() && !pick.reason.starts_with(HELD_FOR_OVERLAP))
            .collect();
        let mut started = 0;
        for pick in aside {
            let id = format!("pick:{}", pick.pick_id);
            if self.decision_kept(&id).await || destructive_text(&pick.issue_title) {
                continue;
            }
            let deciding = Deciding {
                kind: AutopilotDecision::Unsure,
                subject: format!("{} {}", pick.issue_reference, pick.issue_title),
                detail: Some(format!("The picker set it aside: {}", pick.reason)),
                ways: vec!["Start it".to_string()],
            };
            let Some((way, because)) = self.helper_says(helping, &id, &deciding).await else { continue };
            let manifest = ManifestId::carried(&pick.manifest_id);
            if Arc::clone(self).dispatch_set_aside(manifest, pick.pick_id.clone()).await.is_err() {
                continue;
            }
            let session = self
                .store()
                .lock()
                .await
                .autopilot_picks(&run)
                .unwrap_or_default()
                .into_iter()
                .find(|one| one.pick_id == pick.pick_id)
                .and_then(|one| one.started_session);
            let chose = &deciding.ways[way];
            self.decision_kept_as(helping, &id, session.as_deref(), &pick.issue_title, AutopilotDecision::Unsure, deciding.subject.clone(), chose, because).await;
            started += 1;
        }
        started
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
    /// A sure pick of `issue`, **set aside for the Operator where a live Job is near it**, and `true`.
    /// The Operator makes one call on whether the Job already holds it; a go-ahead stands on Yes or
    /// No, and a ruling that is anything else, or a call that fails, starts it as it was asked.
    pub(super) async fn pick_held_for_overlap(self: &Arc<Self>, kept: &store::KeptAutopilot, issue: &Candidate, workflow: &str) -> bool {
        let pick_id = self.mint().ulid().as_str().to_string();
        let words = format!("{}\n\n{}\n\n{}", issue.url, issue.title, issue.excerpt);
        let arrival = Self::pick_arrival(&kept.manifest_id, &pick_id, &issue.reference, &issue.title, words);
        if !self.start_has_near(&arrival).await {
            return false;
        }
        let row = store::KeptPick {
            pick_id: pick_id.clone(),
            run_id: kept.run_id.clone(),
            manifest_id: kept.manifest_id.clone(),
            issue_reference: issue.reference.clone(),
            issue_title: issue.title.clone(),
            issue_url: issue.url.clone(),
            workflow_id: Some(workflow.to_string()),
            started_session: None,
            state: PickState::SetAside.as_stored().to_string(),
            reason: format!("{HELD_FOR_OVERLAP}a live Job is near it, and the Operator is asking whether to hand it over."),
            at: self.now().as_str().to_string(),
        };
        if self.store().lock().await.add_autopilot_pick(&row).is_err() {
            return false;
        }
        let (fleet, manifest) = (Arc::clone(self), kept.manifest_id.clone());
        tokio::spawn(async move {
            if !fleet.start_ruled(arrival).await {
                fleet.pick_released(&manifest, &pick_id).await;
            }
        });
        true
    }

    /// The pick held for the Operator is started as it was asked, a Job on its issue.
    pub(crate) async fn pick_released(self: &Arc<Self>, manifest: &str, pick: &str) {
        let _ = Arc::clone(self).dispatch_set_aside(ManifestId::carried(manifest), pick.to_string()).await;
    }

    /// The pick held for the Operator is handed to a live Job, so no Job starts on it. Whether it was still held.
    pub(crate) async fn pick_handed_over(&self, pick: &str) -> bool {
        self.store().lock().await.move_autopilot_pick(pick, PickState::SetAside.as_stored(), PickState::Dismissed.as_stored(), None).unwrap_or(false)
    }
}
