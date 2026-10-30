//! What a Job still holds when no end state has taken it, and what that does to the Job.
//! `docs/concepts/session/the-rail.md`, *A live instance holds its Job open*.
//!
//! **A live instance holds its Job open, as an open pull request does.** The agent's `all_done` and
//! `finish_up` are refused while the Job holds one standing in a state its lifecycle knows and does
//! not end on, each named as `Deploy is Rolling out`, and Fleet tells the agent when the last one
//! reaches an end state. **One unread, or in a status nobody could place, holds nothing**: the
//! built-in Ticket lifecycle lists no tracker words, so a Job holding a ticket under it would be
//! refused with nothing the agent could do to clear it. **Ending the Job ends its instances**, the
//! unplaced too: each one no end state has taken is kept as ended, standing where it was, and no
//! reader moves it again. Bridge lists the same instances before the person ends the Job. A Job
//! that holds none, which is every Job started before lifecycles, is held by nothing and ends as it
//! always did.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{HeldInstances, InstanceRef, Lifecycle, Standing};
use ipc::WireError;
use store::Store;

use super::holding::rebuilt;
use super::serving::label_of;
use crate::daemon::Fleet;

pub(crate) const IN_FLIGHT: &str = "fleet.lifecycles_in_flight";

/// One instance no end state has taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flying {
    pub lifecycle: String,
    pub key: String,
    /// What a person calls it: `ARM-42`, `PR #812`, `Deploy`.
    pub label: String,
    /// Where it stands, in the lifecycle's own label for the state or, for a status nobody could place,
    /// the word its source used. **Empty for one not read yet.**
    pub state: String,
    /// It stands in a state its lifecycle knows and does not end on. **Only such an instance holds the
    /// Job open**; one unread or unmapped is listed at End and refuses nothing.
    pub holds: bool,
}

impl Flying {
    /// `Deploy is Rolling out`.
    pub fn held(&self) -> String {
        format!("{} is {}", self.label, self.state)
    }
}

/// Where something of `subject`'s lifecycle stands, as `Flying::state` says it.
pub fn state_said(lifecycles: &[Lifecycle], subject: &InstanceRef, standing: &Standing) -> String {
    match standing {
        Standing::In(state) => lifecycles
            .iter()
            .find(|lifecycle| lifecycle.id() == subject.lifecycle())
            .and_then(|lifecycle| lifecycle.state(state))
            .map(|state| state.label().as_str().to_string())
            .unwrap_or_default(),
        Standing::Unmapped(said) => said.clone(),
        Standing::Unread => String::new(),
    }
}

/// What `held` still holds, in the order it was taken on. **An unread or unmapped instance is among
/// them**, since only a state ends one and an instance nobody can place must not be let go, and
/// `holds` says which of them keep the Job open.
pub fn flying(held: &HeldInstances, lifecycles: &[Lifecycle], label: &dyn Fn(&InstanceRef) -> String) -> Vec<Flying> {
    held.in_flight(lifecycles)
        .into_iter()
        .map(|one| {
            let subject = one.subject();
            let state = state_said(lifecycles, subject, one.standing());
            // A state its lifecycle does not list has no label to say, and holds nothing either.
            let holds = matches!(one.standing(), Standing::In(_)) && !state.is_empty();
            Flying { lifecycle: subject.lifecycle().as_str().to_string(), key: subject.key().to_string(), label: label(subject), state, holds }
        })
        .collect()
}

/// What the agent is told when its `call` is refused.
fn refused_text(call: &str, holding: &[Flying]) -> String {
    let listed: Vec<String> = holding.iter().map(|one| format!("- {}", one.held())).collect();
    format!(
        "{call} refused. Still in flight:\n\n{}\n\nEnd your turn. Fleet tells you when the last one reaches an end state.",
        listed.join("\n")
    )
}

/// What the agent is told when `last`, the instance that held it, reached an end state.
pub fn clear_text(last: &Flying) -> String {
    format!("{}. Nothing holds this Job open.", last.held())
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
    /// What `session` holds that no end state has taken, read against the lifecycles it was read
    /// with. **Nothing for a Job that holds nothing**, and for an instance ending the Job ended.
    pub(crate) fn lifecycles_flying(&self, store: &Store, session: &str) -> Vec<Flying> {
        let mut kept = store.lifecycle_instances_of(session).unwrap_or_default();
        kept.retain(|one| one.ended_at.is_empty());
        let Some(manifest) = store.session(session).ok().flatten().and_then(|one| one.manifest_id).filter(|_| !kept.is_empty()) else {
            return Vec::new();
        };
        let lifecycles = self.lifecycles_of_job(store, session, &manifest);
        flying(&rebuilt(&lifecycles, &kept), &lifecycles, &|subject| label_of(store, subject))
    }

    /// What of `lifecycles_flying` keeps `session` open: the instances standing in a state their
    /// lifecycle knows and does not end on.
    pub(crate) fn lifecycles_holding(&self, store: &Store, session: &str) -> Vec<Flying> {
        let mut flying = self.lifecycles_flying(store, session);
        flying.retain(|one| one.holds);
        flying
    }

    /// The agent's `call` (`all_done` or `finish_up`) is refused while the Job holds an instance
    /// standing in a state that is not an end, and **kept as told to wait**, which is what lets the
    /// last instance's end tell it so. Only the agent's call asks: the person's End job is `close_session`.
    pub(crate) fn lifecycles_hold(&self, store: &mut Store, session: &str, call: &str) -> Result<(), Refusal> {
        let holding = self.lifecycles_holding(store, session);
        if holding.is_empty() {
            return Ok(());
        }
        let _ = store.hold_lifecycle_job(session, self.now().as_str());
        Err(Refusal::IllegalMove(WireError::raised(IN_FLIGHT, refused_text(call, &holding), self.run_id())))
    }

    /// Nothing holds `session` open now, and its agent had been refused for what did: it is told so
    /// once, and **kept as told only when it was heard**, since a Job not listening is told again at
    /// the next move. A Job whose agent was never refused is not told, and neither is one still held.
    pub(crate) async fn lifecycles_clear_told(self: &Arc<Self>, session: &str) {
        let text = {
            let store = self.store().lock().await;
            let refused = store.lifecycle_job_held(session).unwrap_or(false);
            let Some(last) = store.lifecycle_moves_of(session).ok().and_then(|mut moves| moves.pop()).filter(|_| refused) else { return };
            let Some(manifest) = store.session(session).ok().flatten().and_then(|one| one.manifest_id) else { return };
            if !self.lifecycles_holding(&store, session).is_empty() {
                return;
            }
            let lifecycles = self.lifecycles_of_job(&store, session, &manifest);
            let Some(subject) = Standing::from_wire(&last.to).zip(lifecycles.iter().find(|one| one.id().as_str() == last.lifecycle)).and_then(|(standing, lifecycle)| {
                let subject = InstanceRef::of(lifecycle, &last.key).ok()?;
                Some((state_said(&lifecycles, &subject, &standing), label_of(&store, &subject)))
            }) else {
                return;
            };
            clear_text(&Flying { lifecycle: last.lifecycle.clone(), key: last.key.clone(), label: subject.1, state: subject.0, holds: false })
        };
        if matches!(self.told_by_fleet(session, &text, ipc::Notice::LifecyclesClear).await, Ok(true)) {
            let _ = self.store().lock().await.release_lifecycle_job(session);
        }
    }

    /// Ending the Job ends its instances: each one still in flight is kept as ended. A Job that holds
    /// none changes nothing and publishes nothing.
    pub(crate) async fn lifecycles_ended_with(&self, session: &str) {
        let at = self.now().as_str().to_string();
        let ended = {
            let mut store = self.store().lock().await;
            let holding = self.lifecycles_flying(&store, session);
            let keys: Vec<(&str, &str)> = holding.iter().map(|one| (one.lifecycle.as_str(), one.key.as_str())).collect();
            !keys.is_empty() && store.end_lifecycle_instances(session, &keys, &at).unwrap_or(0) > 0
        };
        if ended {
            self.job_lifecycles_published(session).await;
        }
    }
}
