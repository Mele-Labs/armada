//! A green pull request of a Job, landed the way Auto land lands it: the forge is asked to merge it
//! when its checks pass, by the same call. **Only where every deciding check has passed** and
//! nothing conflicts or already merges it, read from the forge as it is now. A Job with Auto land on
//! is left to Auto land.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, PullQueue, Vcs, WorkProduct};
use ipc::{AutopilotDecision, ManifestId, PullRequestStanding};

use super::deciding::Deciding;
use super::decisions::{Helping, Job};
use crate::daemon::Fleet;

/// How much of a description the helper reads.
const READ: usize = 1_500;

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
    /// Land each green pull request the Jobs in scope hold. Answers how many it landed.
    pub(super) async fn pulls_landed(self: &Arc<Self>, helping: &mut Helping, jobs: &[Job]) -> usize {
        if !helping.kinds.contains(&AutopilotDecision::Land) {
            return 0;
        }
        let manifest = ManifestId::carried(&helping.kept.manifest_id);
        let Ok(served) = self.served_named(Some(&manifest)) else { return 0 };
        let (root, gate) = (served.root().to_string(), served.manifest().ci_gate().to_string());
        let mut landed = 0;
        for job in jobs {
            if !matches!(self.terminal_session(&job.id).await, Ok(None)) || self.auto_land_on(&job.id).await {
                continue;
            }
            for number in self.open_pulls_of(&job.id, manifest.as_str()).await {
                let gate = gate.clone();
                let read = self.forge_asked(&root, move |vcs: &V, root: &str| vcs.followed_pull(root, &gate, number)).await;
                let Some(pull) = read else { continue };
                // Green, mergeable and not already merging: a pull request that is none of these is not
                // waiting on the person to land it.
                if !pull.passed() || pull.conflicting || pull.auto_merge || pull.queue != PullQueue::Outside {
                    continue;
                }
                // A commit pushed since is a pull request to decide again.
                let id = format!("{}:land:{number}:{}", job.id, pull.head.as_deref().unwrap_or_default());
                if self.decision_kept(&id).await {
                    continue;
                }
                let Ok(state) = self.pull_request_read_off(&manifest, &root, number).await else { continue };
                if state.state != PullRequestStanding::Open {
                    continue;
                }
                let body = state.body.as_deref().unwrap_or_default();
                let deciding = Deciding {
                    kind: AutopilotDecision::Land,
                    subject: format!("#{number} {}", state.title),
                    detail: Some(body.chars().take(READ).collect()).filter(|text: &String| !text.is_empty()),
                    ways: vec!["Land it".to_string()],
                };
                let Some((way, because)) = self.helper_says(helping, &id, &deciding).await else { continue };
                // A pull request held behind a clash in the queue is not landed, and is asked again once it clears.
                if !matches!(self.auto_merge_asked_for(&manifest, number, pull.head.as_deref()).await, Ok(None)) {
                    continue;
                }
                let asked = deciding.subject.clone();
                let chose = &deciding.ways[way];
                self.decision_kept_as(helping, &id, Some(&job.id), &job.who, AutopilotDecision::Land, asked, chose, because).await;
                landed += 1;
            }
        }
        landed
    }
}
