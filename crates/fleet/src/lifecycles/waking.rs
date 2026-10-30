//! A Job parked on a step that ends on a lifecycle state, moved on when the state is reached.
//! `docs/concepts/session/the-rail.md`, *A step that ends on a lifecycle state*.
//!
//! **Nothing here polls, and nothing here decides.** The readers already record each move in
//! `lifecycle_moves`; when one lands on a Job whose step waits on a lifecycle, the row is drawn
//! again, so the rail (which reads those moves) places the Job and `rail_acted_on` tells the agent.
//! The line that names what happened rides with the next step's instructions, as one message.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;
use crate::rail::RailStep;
use crate::rail_lifecycle::{self, Held};

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
    /// A move was just kept for `session`. If its step waits on a lifecycle, draw the row again so the
    /// rail sees it; a Job on any other step is left alone.
    pub(crate) async fn lifecycle_wakes(&self, session: &str) {
        let (manifest, workflow, at) = {
            let store = self.store().lock().await;
            let Some(hosting) = store.hosting(session).ok().flatten() else { return };
            let Some(workflow) = hosting.workflow else { return };
            let Some(at) = store.session_rail(session).ok().and_then(|kept| kept.step) else { return };
            (hosting.manifest_id, workflow, at as usize)
        };
        let waits = self.rail_steps(&manifest, &workflow).is_some_and(|steps| steps.get(at).is_some_and(RailStep::has_lifecycle_end));
        if waits {
            let _ = self.published_hosted(session).await;
        }
    }

    /// The one line that wakes the agent on leaving `step`, where the step waited on a lifecycle and
    /// its end has come. **`None` for a step the person skipped or wrote on first**: nothing got there.
    pub(crate) async fn lifecycle_woken(&self, session: &str, step: &RailStep, workflow: &str, manifest: &str) -> Option<String> {
        if !step.has_lifecycle_end() {
            return None;
        }
        let (held, titles) = {
            let store = self.store().lock().await;
            let held = Held { instances: store.lifecycle_instances_of(session).ok()?, moves: store.lifecycle_moves_of(session).ok()? };
            let titles: HashMap<String, String> = held
                .instances
                .iter()
                .filter(|one| one.lifecycle == "job")
                .filter_map(|one| Some((one.key.clone(), store.session(&one.key).ok().flatten()?.title?)))
                .collect();
            (held, titles)
        };
        let lifecycles = self.lifecycles_that_apply(manifest);
        let name = |one: &store::KeptInstance| named_by_title(one, &titles);
        step.lifecycle_ends().find_map(|end| {
            let lifecycle = lifecycles.iter().find(|one| one.id() == end.lifecycle())?;
            let reading = rail_lifecycle::read(end, lifecycle, &held);
            reading.met(end.waits()).then(|| rail_lifecycle::woke(&step.label, workflow, end, lifecycle, &reading, &name))
        })
    }
}

fn named_by_title(one: &store::KeptInstance, titles: &HashMap<String, String>) -> String {
    rail_lifecycle::named(one, titles.get(&one.key).map(String::as_str))
}
