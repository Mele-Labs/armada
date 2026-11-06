//! What a Job still holds when no end state has taken it, and what that does to the Job.
//! `docs/concepts/session/the-rail.md`, *A live instance holds its Job open*.
//!
//! **A live instance holds its Job open, as an open pull request does.** The agent's `all_done` and
//! `finish_up` are refused while the Job holds one standing in a state its lifecycle knows and does
//! not end on, each named as `Deploy is Rolling out`, and Fleet tells the agent when the last one
//! reaches an end state. **One unread, or in a status nobody could place, holds nothing**: the
//! built-in Ticket lifecycle lists no tracker words but `closed` where its tracker declares none, so a Job
//! holding a ticket under it would be refused with nothing the agent could do to clear it. **A status the
//! tracker declares a word for is placed**, and holds. **A Job it started that has not begun holds
//! nothing either**, on its start card or waiting on another, and holds it from the moment it runs.
//! **Ending the Job ends its instances**, the unplaced too: each one no end state has taken is kept
//! as ended, standing where it was, and no reader moves it again; the Job it started is not ended
//! with it. Bridge lists the same instances before the person ends the Job. A Job that holds none,
//! which is every Job started before lifecycles, is held by nothing and ends as it always did.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{HeldInstances, InstanceRef, Lifecycle, Standing};
use ipc::WireError;
use store::Store;

use super::holding::rebuilt;
use super::keeping::JOB;
use super::serving::label_of;
use crate::daemon::Fleet;

pub const IN_FLIGHT: &str = "fleet.lifecycles_in_flight";

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
    /// Job open**; one unread or unmapped is listed at End and refuses nothing, and so is a Job started
    /// that has not begun ([`Fleet::lifecycles_flying`]).
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

/// What the agent is told while `holding` keeps its Job open.
fn in_flight_text(holding: &[Flying]) -> String {
    let listed: Vec<String> = holding.iter().map(|one| format!("- {}", one.held())).collect();
    format!("Still in flight:\n\n{}\n\nEnd your turn. Fleet tells you when the last one reaches an end state.", listed.join("\n"))
}

/// A Job `key` names, started by the Job asking, that has not run: on its start card or waiting on
/// another. `hosting.ran` is kept when its process first spawns, so a first turn still going has run.
fn not_begun(store: &Store, lifecycle: &str, key: &str) -> bool {
    lifecycle == JOB && store.hosting(key).ok().flatten().is_some_and(|hosting| !hosting.ran)
}

/// What the agent is told when nothing holds it now: `last`, the instance that held it, reached an
/// end state, and `waiting` names the Jobs it started that have not begun. `None` where no end was
/// reached: it was refused for a Job that had not begun, before those held nothing.
pub fn clear_text(last: Option<&Flying>, waiting: &[String]) -> String {
    let over = last.map(|one| format!("{}. ", one.held())).unwrap_or_default();
    match waiting {
        [] => format!("{over}Nothing holds this Job open."),
        [one] => format!("{over}Nothing holds this Job open: {one} has not begun, so you may finish."),
        many => format!("{over}Nothing holds this Job open: {} have not begun, so you may finish.", many.join(", ")),
    }
}

impl Fleet
{
    /// What `session` holds that no end state has taken, read against the lifecycles it was read
    /// with. **Nothing for a Job that holds nothing**, and for an instance ending the Job ended. **A
    /// Job it started that has not begun is listed and holds nothing**, whatever its instance reads,
    /// and holds it again once it runs, since this asks each time.
    pub fn lifecycles_flying(&self, store: &Store, session: &str) -> Vec<Flying> {
        let mut kept = store.lifecycle_instances_of(session).unwrap_or_default();
        kept.retain(|one| one.ended_at.is_empty());
        let Some(manifest) = store.session(session).ok().flatten().and_then(|one| one.manifest_id).filter(|_| !kept.is_empty()) else {
            return Vec::new();
        };
        let lifecycles = self.lifecycles_of_job(store, session, &manifest);
        let mut listed = flying(&rebuilt(&lifecycles, &kept), &lifecycles, &|subject| label_of(store, subject));
        for one in &mut listed {
            one.holds &= !not_begun(store, &one.lifecycle, &one.key);
        }
        listed
    }

    /// What of `lifecycles_flying` keeps `session` open: the instances standing in a state their
    /// lifecycle knows and does not end on, other than a Job it started that has not begun.
    pub fn lifecycles_holding(&self, store: &Store, session: &str) -> Vec<Flying> {
        let mut flying = self.lifecycles_flying(store, session);
        flying.retain(|one| one.holds);
        flying
    }

    /// The agent's `call` (`all_done` or `finish_up`) is refused while the Job holds an instance
    /// standing in a state that is not an end, and **kept as told to wait**, which is what lets the
    /// last instance's end tell it so. Only the agent's call asks: the person's End job is `close_session`.
    pub fn lifecycles_hold(&self, store: &mut Store, session: &str, call: &str) -> Result<(), Refusal> {
        match self.lifecycles_hold_text(store, session) {
            None => Ok(()),
            Some(text) => Err(Refusal::IllegalMove(WireError::raised(IN_FLIGHT, format!("{call} refused. {text}"), self.run_id()))),
        }
    }

    /// What `lifecycles_hold` refuses with, for a call that names every reason at once. Kept as told
    /// to wait when there is one.
    pub fn lifecycles_hold_text(&self, store: &mut Store, session: &str) -> Option<String> {
        let holding = self.lifecycles_holding(store, session);
        if holding.is_empty() {
            return None;
        }
        let _ = store.hold_lifecycle_job(session, self.now().as_str());
        Some(in_flight_text(&holding))
    }

    /// Nothing holds `session` open now, and its agent had been refused for what did: it is told so
    /// once, and **kept as told only when it was heard**, since a Job not listening is told again at
    /// the next move. A Job whose agent was never refused is not told, and neither is one still held.
    /// **The move of a Job that has not begun is no end**, so the line is about the last that was, and
    /// names what has not begun.
    pub async fn lifecycles_clear_told(self: &Arc<Self>, session: &str) {
        let text = {
            let store = self.store().lock().await;
            if !store.lifecycle_job_held(session).unwrap_or(false) {
                return;
            }
            let Some(manifest) = store.session(session).ok().flatten().and_then(|one| one.manifest_id) else { return };
            if !self.lifecycles_holding(&store, session).is_empty() {
                return;
            }
            let mut waiting = self.lifecycles_flying(&store, session);
            waiting.retain(|one| not_begun(&store, &one.lifecycle, &one.key));
            let lifecycles = self.lifecycles_of_job(&store, session, &manifest);
            let moves = store.lifecycle_moves_of(session).unwrap_or_default();
            let last = moves.iter().rev().find(|one| !waiting.iter().any(|not_yet| not_yet.lifecycle == one.lifecycle && not_yet.key == one.key)).and_then(|last| {
                let standing = Standing::from_wire(&last.to)?;
                let lifecycle = lifecycles.iter().find(|one| one.id().as_str() == last.lifecycle)?;
                let subject = InstanceRef::of(lifecycle, &last.key).ok()?;
                let (state, label) = (state_said(&lifecycles, &subject, &standing), label_of(&store, &subject));
                Some(Flying { lifecycle: last.lifecycle.clone(), key: last.key.clone(), label, state, holds: false })
            });
            let waiting: Vec<String> = waiting.into_iter().map(|one| one.label).collect();
            if last.is_none() && waiting.is_empty() {
                return;
            }
            clear_text(last.as_ref(), &waiting)
        };
        if matches!(self.told_by_fleet(session, &text, ipc::Notice::LifecyclesClear).await, Ok(true)) {
            let _ = self.store().lock().await.release_lifecycle_job(session);
        }
    }

    /// Ending the Job ends its instances: each one still in flight is kept as ended. A Job that holds
    /// none changes nothing and publishes nothing.
    pub async fn lifecycles_ended_with(&self, session: &str) {
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
