//! The agents' account signed out. Every turn after fails at once, and **every Job shares the account**,
//! so the Fleet is signed out as one: Bridge is sent [`ipc::AgentSignIn`] whole, and each Job the expiry
//! stopped keeps a mark so it is woken once the account works again.
//!
//! **In memory, never written.** A Fleet that restarts has no mark, and a Job's next turn fails the same
//! way and marks it again. The model's own words or call in any Job is the account working, and clears it.

use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::Runtime;
use crate::daemon::Fleet;

/// What a Job the expiry stopped is told once the account works again, as the plan's limit wakes one.
const WAKE: &str = "Continue";

/// The Fleet-wide state. A `std` lock, never held across an `.await`.
#[derive(Default)]
pub(crate) struct Held {
    state: Mutex<ipc::AgentSignIn>,
}

impl Held {
    pub(crate) fn now(&self) -> ipc::AgentSignIn {
        self.held().clone()
    }

    fn held(&self) -> std::sync::MutexGuard<'_, ipc::AgentSignIn> {
        self.state.lock().expect("the sign-in is not held across a panic")
    }

    /// Signed out at `at`, where it was signed in. `Some` of the state where it moved: **a repeat keeps
    /// the first instant**.
    fn signed_out(&self, at: &str) -> Option<ipc::AgentSignIn> {
        let mut held = self.held();
        if held.signed_out_since.is_some() {
            return None;
        }
        held.signed_out_since = Some(ipc::Instant::carried(at));
        Some(held.clone())
    }

    fn is_signed_out(&self) -> bool {
        self.held().signed_out_since.is_some()
    }

    /// Signed in. `Some` of the state where it moved.
    fn signed_in(&self) -> Option<ipc::AgentSignIn> {
        let mut held = self.held();
        let was = held.clone();
        *held = ipc::AgentSignIn::default();
        (was != *held).then(|| held.clone())
    }
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
    /// The harness said its sign-in ran out: the Job is marked, and the Fleet is signed out where it
    /// was not.
    pub(crate) fn signed_out_heard(&self, runtime: &Arc<Runtime>) {
        runtime.state().signed_out = true;
        if let Some(now) = self.agent_sign_in().signed_out(self.now().as_str()) {
            self.publish(ipc::Event::AgentSignInChanged(now));
        }
    }

    /// The model answered in `id`, so the account works: where the Fleet was signed out, every other
    /// Job it stopped is woken. **Not a turn's start**: the harness says `init` before every turn, the
    /// failed ones too.
    pub(crate) async fn signed_in_heard(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>) {
        if self.agent_sign_in().is_signed_out() || runtime.state().signed_out {
            self.signed_back_in(Some(id)).await;
        }
    }

    /// The sign-in run exited clean. Each Job but `ran_by` is woken: the Job that ran it is told of
    /// the run, which starts its turn, so a wake would be a second one.
    pub(crate) async fn signed_in_everywhere(self: &Arc<Self>, ran_by: &str) {
        self.signed_back_in(Some(ran_by)).await;
    }

    /// Every mark cleared and the Fleet signed in, and each marked Job but `answered` woken with
    /// `Continue`. A Job that ended is not told, which `told_by_fleet` answers.
    async fn signed_back_in(self: &Arc<Self>, answered: Option<&str>) {
        let marked: Vec<String> = self
            .hosts()
            .all()
            .into_iter()
            .filter(|(_, runtime)| std::mem::take(&mut runtime.state().signed_out))
            .map(|(id, _)| id)
            .collect();
        if let Some(now) = self.agent_sign_in().signed_in() {
            self.publish(ipc::Event::AgentSignInChanged(now));
        }
        for id in marked.iter().filter(|id| Some(id.as_str()) != answered) {
            let _ = self.told_by_fleet(id, WAKE, ipc::Notice::SignInWake).await;
        }
    }
}
