//! The Checks of a manifest a Job's frozen workflow never met.
//!
//! **Read off the manifests Fleet serves, never off the Job's branch**, so a
//! Drone cannot add or edit a Check it will be gated on. The frozen Checks are
//! untouched; a manifest the Job froze nothing for gets what
//! `every_manifest_check` would have expanded to. `docs/concepts/manifest.md`,
//! *Workspace gating*.

use core_model::{FrozenWorkflow, ResolvedCheck, RunsAt};

use crate::manifest::Manifest;
use crate::resolve::lifted;

/// What `every_manifest_check` lifts from one manifest. A handoff-only Check is
/// left to the step that runs it before handoff.
pub(crate) fn every_check_of(manifest: &Manifest, runs_handoff: bool) -> Vec<ResolvedCheck> {
    manifest
        .checks_as_written()
        .iter()
        .filter_map(|name| {
            let declared = manifest
                .check(name)
                .expect("`checks_as_written` holds the keys of `checks`");
            if declared.runs_at() == RunsAt::Handoff && !runs_handoff {
                return None;
            }
            Some(lifted(
                name.clone(),
                declared,
                declared.expect_exit_code(),
                manifest.dir(),
            ))
        })
        .collect()
}

/// `workflow` with `added`'s Checks on every step that gates on every Check.
/// A copy for one gate pass: the Job's own frozen workflow is not changed.
pub fn with_manifests_added(workflow: &FrozenWorkflow, added: &[&Manifest]) -> FrozenWorkflow {
    let steps = workflow.steps();
    let every = |at: &usize| steps[*at].gates_on_every_check();
    let delivers = steps
        .iter()
        .position(|step| step.delivers())
        .unwrap_or(steps.len());
    let handoff_at = (0..delivers)
        .rev()
        .find(every)
        .or_else(|| (0..steps.len()).rev().find(every));
    let mut at = 0;
    workflow.clone().regated(|step| {
        let here = at;
        at += 1;
        if !step.gates_on_every_check() {
            return step;
        }
        let more = added
            .iter()
            .flat_map(|manifest| every_check_of(manifest, handoff_at == Some(here)))
            .collect();
        step.also_checking(more)
    })
}
