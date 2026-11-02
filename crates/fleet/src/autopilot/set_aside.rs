//! An issue the picker was unsure of, started when the helper is sure. The start is the person's own
//! press on the pick (`dispatch_set_aside`), which claims it before the Session starts, so a press
//! and a pass make one Session between them.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Autopilots;
use ipc::{AutopilotDecision, ManifestId};

use super::bounds::PickState;
use super::deciding::Deciding;
use super::decisions::Helping;
use super::destructive::destructive_text;
use crate::daemon::Fleet;

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
            .filter(|pick| pick.state == PickState::SetAside.as_stored())
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
