//! The links Fleet adds to an artifact's For line when a revision is kept, after the agent's own: the plan
//! task the Job has in progress, that task's plan group, and each open pull request the Job holds.
//! `docs/concepts/artifact.md`, *The For line*.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{ArtifactFor, ArtifactLink, ArtifactLinkKind, PlanTaskState, ARTIFACT_FOR};
use store::{AttachmentState, Holder, KeptAttachment, KeptPlan};

use crate::daemon::Fleet;
use crate::plan_record::state_of;
use crate::pull_requesting::PR;

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
    /// The agent's For line with the links Fleet knows of Job `id` after it. None where both are empty.
    pub(crate) async fn for_line(&self, id: &str, said: Option<ArtifactFor>) -> Option<ArtifactFor> {
        let (plan, held) = {
            let store = self.store().lock().await;
            (store.session_plan(id).ok().flatten(), store.attachments_of(&Holder::session(id)).unwrap_or_default())
        };
        ArtifactFor::with_links(said, known_links(plan.as_ref(), &held))
    }
}

/// The task the plan has in progress, the newest started where several are, then its group, then each open
/// pull request held, lowest number first.
fn known_links(plan: Option<&KeptPlan>, held: &[KeptAttachment]) -> Vec<ArtifactLink> {
    let mut links = Vec::new();
    let working = plan.into_iter().flat_map(|plan| plan.groups.iter()).flat_map(|group| group.tasks.iter().map(move |task| (group, task)));
    let working = working.filter(|(_, task)| state_of(task).0 == PlanTaskState::Working).max_by(|a, b| a.1.started_at.cmp(&b.1.started_at));
    if let Some((group, task)) = working {
        links.push(ArtifactLink { kind: ArtifactLinkKind::Task, label: task.text.clone(), target: task.id.clone() });
        links.push(ArtifactLink { kind: ArtifactLinkKind::PlanStep, label: group.title.clone(), target: group.id.clone() });
    }
    let mut pulls: Vec<(u64, String)> = held
        .iter()
        .filter(|one| one.kind == PR && one.state != AttachmentState::GivenBack)
        .filter(|one| !matches!(one.detail.get("state").map(String::as_str), Some("merged" | "closed")))
        .filter_map(|one| Some((one.target.trim_start_matches('#').parse().ok()?, one.detail.get("title").map(|title| title.trim().to_string()).unwrap_or_default())))
        .collect();
    pulls.sort();
    pulls.dedup_by_key(|(number, _)| *number);
    links.extend(pulls.into_iter().map(|(number, title)| ArtifactLink {
        kind: ArtifactLinkKind::PullRequest,
        label: if title.is_empty() { format!("#{number}") } else { format!("#{number} {title}") },
        target: format!("#{number}"),
    }));
    links
}

/// `detail` with the For line written where there is one.
pub(crate) fn with_for(detail: &mut BTreeMap<String, String>, line: Option<&ArtifactFor>) {
    if let Some(encoded) = line.and_then(|line| ipc::encode(line).ok()) {
        detail.insert(ARTIFACT_FOR.to_string(), encoded);
    }
}
