//! A Job parked on a step that ends on a lifecycle state, moved on when the state is reached.
//! `docs/concepts/session/the-rail.md`, *A step that ends on a lifecycle state*.
//!
//! **Nothing here polls, and nothing here decides.** The readers already record each move in
//! `lifecycle_moves`; when one lands on a Job whose step waits on a lifecycle, the row is drawn
//! again, so the rail (which reads those moves) places the Job and `rail_acted_on` tells the agent.
//! The line that names what happened rides with the next step's instructions, as one message.
//! **A delivering step is not left when its state comes** (the merge passes it), so `lifecycle_lifted`
//! sends the line where the Job stands.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::SessionRow;
use store::{KeptInstance, Store};

use crate::daemon::Fleet;
use crate::rail::RailStep;
use crate::rail_keeping::rail_read;
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
    /// **And for a delivering step**, which landing leaves and not the state: see [`Self::lifecycle_lifted`].
    pub(crate) async fn lifecycle_woken(&self, session: &str, step: &RailStep, workflow: &str, manifest: &str) -> Option<String> {
        if !step.has_lifecycle_end() || step.delivers {
            return None;
        }
        let (held, titles) = {
            let store = self.store().lock().await;
            let held = held_of(&store, session)?;
            let titles = titles_of(&store, &held);
            (held, titles)
        };
        let lifecycles = self.lifecycles_that_apply(manifest);
        let name = |one: &KeptInstance| rail_lifecycle::named(one, titles.get(&one.key).map(String::as_str));
        rail_lifecycle::woken(step, workflow, &lifecycles, &held, &name)
    }

    /// The one line that lifts the wait of a **delivering** step gated by a lifecycle, once its end has
    /// come: the step is not left, so this is sent where the Job stands. **`None` once the thread holds
    /// it, and when the person wrote on the step**, which lifts the wait himself.
    pub(crate) async fn lifecycle_lifted(
        &self,
        session: &str,
        at: usize,
        step: &RailStep,
        workflow: &str,
        manifest: &str,
    ) -> Option<String> {
        if !step.delivers || !step.has_lifecycle_end() {
            return None;
        }
        let (reads, titles, rows) = {
            let store = self.store().lock().await;
            let kept = store.session(session).ok().flatten()?;
            let reads = rail_read(&store, &kept, true).ok()?;
            let titles = titles_of(&store, &reads.held);
            let rows = store.session_rows(session).ok()?;
            (reads, titles, rows)
        };
        if reads.extra.messaged.has(at) {
            return None;
        }
        let lifecycles = self.lifecycles_that_apply(manifest);
        let name = |one: &KeptInstance| rail_lifecycle::named(one, titles.get(&one.key).map(String::as_str));
        let line = rail_lifecycle::woken(step, workflow, &lifecycles, &reads.held, &name)?;
        let said = rows.iter().filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok()).any(
            |row| matches!(&row, SessionRow::Message { text, .. } if text.contains(&line)),
        );
        (!said).then_some(line)
    }
}

fn held_of(store: &Store, session: &str) -> Option<Held> {
    Some(Held { instances: store.lifecycle_instances_of(session).ok()?, moves: store.lifecycle_moves_of(session).ok()? })
}

/// What a Job's own title is, for each Job it holds: how `named` calls one.
pub(crate) fn titles_of(store: &Store, held: &Held) -> HashMap<String, String> {
    held.instances
        .iter()
        .filter(|one| one.lifecycle == "job")
        .filter_map(|one| Some((one.key.clone(), store.session(&one.key).ok().flatten()?.title?)))
        .collect()
}
