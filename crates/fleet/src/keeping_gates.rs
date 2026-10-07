//! Reading which manifests a change reaches before the gate, and keeping what
//! each came to after it. Only for a repository with workspaces: one with none
//! records nothing, as it always did.
//!
//! **The refresh replaces `Job::gate_manifests` whole.** `job_manifests` is
//! read back on every load and not folded from `job_events`, so a rebuild finds
//! the refreshed list as it finds the rest of the row. The refresh is also
//! written to the Job's log, where a person reads it.

use adapter_traits::{AgentHarness, Changed, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Component, Envelope, FieldValue, FrozenWorkflow, GateManifest, JobId, Level, ResolvedCheck,
    ResolvedStep,
};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::gated::Gated;
use crate::repositories::Served;

const THE_GATE_READ_THE_DIFF: &str = "the manifests gating the job were read from its diff";

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
    /// The manifests the worktree's change reaches. `None` for a repository
    /// with no workspaces, and where the diff cannot be read: the gate says
    /// that itself.
    pub(crate) fn gated_by_the_change(
        &self,
        served: &Served,
        worktree: &Worktree,
    ) -> Option<Gated> {
        let workspaces = served.workspaces();
        if workspaces.is_empty() {
            return None;
        }
        let changed = self.work().changed_files(worktree).ok()?;
        Some(Gated::of(
            served.manifest(),
            &workspaces,
            &Changed::paths(&changed),
        ))
    }

    /// The Job's frozen workflow with the Checks of every gating manifest it
    /// froze none for, or `None` where there is nothing to add.
    ///
    /// **Read off the manifests Fleet serves, never off the Job's branch**: a
    /// Drone writing a manifest on its branch adds no Check it is gated on.
    /// The Job's own workflow is not changed, so a manifest it did freeze is
    /// not read again. `docs/concepts/manifest.md`, *Workspace gating*.
    pub(crate) fn gated_with_new_manifests(
        &self,
        job: &JobId,
        workflow: &FrozenWorkflow,
        served: &Served,
        gated: Option<&Gated>,
    ) -> Option<FrozenWorkflow> {
        let gated = gated?;
        let held = served.workspaces();
        let added = gated.unmet_by(workflow, served.manifest(), &held);
        if added.is_empty() {
            return None;
        }
        for manifest in &added {
            let envelope = Envelope::new(
                self.now(),
                Level::Info,
                Component::Fleet,
                self.run().clone(),
                "a manifest gating the job had no Checks frozen, so its Checks were added from main",
            )
            .in_job(job.as_ulid().clone())
            .with_field("manifest", FieldValue::Str(manifest.id().as_str().to_string()))
            .with_field(
                "checks",
                FieldValue::Str(manifest.checks_as_written().join(" ")),
            );
            self.noted_in_the_log(job, &envelope);
        }
        Some(config::with_manifests_added(workflow, &added))
    }

    /// Replace the Job's gating manifests with what the ruling's Checks came
    /// to, and say so in its log. A step with no manifest Check, or a gate that
    /// ran none, leaves the list as it was.
    pub(crate) async fn kept_gates(
        &self,
        job: &JobId,
        step: &ResolvedStep,
        gated: Option<&Gated>,
        ruling: &Ruling,
    ) {
        let Some(gated) = gated else {
            return;
        };
        let declares = step
            .checks()
            .iter()
            .any(|check| matches!(check, ResolvedCheck::ManifestCheck { .. }));
        if !declares || ruling.checks().is_empty() {
            return;
        }
        let came_to = gated.outcomes(step.checks(), ruling.checks());
        let kept = self
            .store()
            .lock()
            .await
            .replace_gate_manifests(job, &came_to);
        if kept.is_ok() {
            self.noted_gates(job, step, &came_to);
        }
    }

    fn noted_gates(&self, job: &JobId, step: &ResolvedStep, came_to: &[GateManifest]) {
        let said: Vec<String> = came_to
            .iter()
            .map(|gate| {
                let (outcome, reason) = gate.outcome.as_wire();
                match reason {
                    Some(reason) => format!("{} {outcome} {reason}", gate.manifest_id.as_str()),
                    None => format!("{} {outcome}", gate.manifest_id.as_str()),
                }
            })
            .collect();
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            THE_GATE_READ_THE_DIFF,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.id().as_str())
        .with_field("manifests", FieldValue::Int(came_to.len() as i64))
        .with_field("outcomes", FieldValue::Str(said.join(" | ")));
        self.noted_in_the_log(job, &envelope);
    }
}
