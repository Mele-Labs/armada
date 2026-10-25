//! A hosted Job whose harness's sign-in ran out. Every turn after fails at once and only the person
//! can sign in again, so the Job waits on him for it with the command to run.
//!
//! **In memory, never written.** A Fleet that restarts has no mark, and the Job's next turn fails
//! the same way and marks it again. A turn that starts is the harness signed in, and clears it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, WaitingAct, WaitingActKind, WaitingItem, WaitingSource};

use super::Runtime;
use crate::daemon::Fleet;

/// What a sign-in item's id begins with; the instant the Job was marked follows.
const SIGN_IN_ITEM: &str = "signin:";

/// The item a signed-out Job is on the Needs you list: run the sign-in. A Fleet call, so Bridge
/// offers to dismiss it.
pub(crate) fn sign_in_item(at: &str) -> WaitingItem {
    WaitingItem {
        id: format!("{SIGN_IN_ITEM}{at}"),
        text: String::from("Sign in to Claude"),
        since: Instant::carried(at),
        source: WaitingSource::FleetCall,
        act: Some(WaitingAct { kind: WaitingActKind::Run, target: adapters::SIGN_IN_COMMAND.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
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
    /// The harness said its sign-in ran out. A Job already marked keeps when it was, so its item
    /// keeps its id.
    pub(crate) async fn signed_out_heard(&self, id: &str, runtime: &Arc<Runtime>) {
        let at = self.now().as_str().to_string();
        {
            let mut state = runtime.state();
            if state.signed_out.is_some() {
                return;
            }
            state.signed_out = Some(at);
        }
        let _ = self.published_hosted(id).await;
    }

    /// A turn started, so the harness is signed in. Whether the Job was marked; the caller publishes.
    pub(crate) fn sign_in_cleared(&self, runtime: &Arc<Runtime>) -> bool {
        runtime.state().signed_out.take().is_some()
    }
}
