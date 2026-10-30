//! A workflow that ends a step on a lifecycle, or a state of one, the repository does not have is
//! refused where it is read, with a sentence naming it. `docs/concepts/workflow.md`, *Walks and
//! replies*.
//!
//! **Here and not in `config`**, which reads a file before anyone knows which lifecycles apply: the
//! pieces a repository has are Kit's, its own and its Workspaces', and only a Fleet has them all.
//! A workflow that ends on none is not asked, so no piece file is read for it.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::{names_no_lifecycle, WorkflowDef};

use crate::daemon::Fleet;
use crate::repositories::{Catalogued, WorkflowNotSaved};

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
    /// `read`, with each workflow that names a lifecycle there is not set aside among the ones left
    /// out. **No other place answers for its id**: the catalogue kept only the one that ran.
    pub(crate) fn lifecycle_ends_checked(&self, manifest: &str, mut read: Catalogued) -> Catalogued {
        let waits = |steps: &[core_model::ResolvedStep]| steps.iter().any(|step| step.ends_on().iter().any(|end| end.lifecycle().is_some()));
        if !read.workflows.values().any(|one| waits(one.steps())) {
            return read;
        }
        let lifecycles = self.lifecycles_that_apply(manifest);
        let refused: Vec<_> = read
            .workflows
            .iter()
            .filter_map(|(id, one)| {
                let steps = one.steps().iter().map(|step| (step.id().as_str(), step.ends_on()));
                let unnamed = names_no_lifecycle(steps, &lifecycles);
                (!unnamed.is_empty()).then(|| (id.clone(), unnamed))
            })
            .collect();
        for (id, unnamed) in refused {
            let Some(one) = read.workflows.remove(&id) else { continue };
            let said: Vec<String> = unnamed.iter().map(ToString::to_string).collect();
            read.left_out.push(ipc::LeftOutWorkflow {
                id: Some(ipc::WorkflowId::from(&id)),
                source: one.source().as_wire().to_string(),
                file: one.workflow_path().to_string_lossy().to_string(),
                said: format!("`{}` was left out, because {}; no `{}` runs here", id.as_str(), said.join("; "), id.as_str()),
                instead: None,
            });
        }
        read
    }

    /// What a definition about to be saved would be refused for, if it ends a step on a lifecycle
    /// the repository does not have. **A definition that will not parse is the loader's to refuse**,
    /// with every fault it has.
    pub(crate) fn lifecycle_ends_unnamed(&self, manifest: &str, definition: &str) -> Result<(), WorkflowNotSaved> {
        let roster = config::Roster::of(self.models_on());
        let Ok(def) = WorkflowDef::parse(Path::new("(definition).json"), definition.trim(), &roster) else { return Ok(()) };
        let steps = || def.steps().iter().map(|step| (step.id().as_str(), step.ends_on()));
        if !def.steps().iter().any(|step| step.ends_on().iter().any(|end| end.lifecycle().is_some())) {
            return Ok(());
        }
        let unnamed = names_no_lifecycle(steps(), &self.lifecycles_that_apply(manifest));
        match unnamed.is_empty() {
            true => Ok(()),
            false => Err(WorkflowNotSaved::Unfit { why: unnamed.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ") }),
        }
    }
}
