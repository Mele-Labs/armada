//! The line a hosted session joins when its first write finds every worktree
//! slot held, and the grant that serves it when one frees.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **Memory only, one line per repository, first come first served.** A session
//! is in it once, and leaves when it is leased a slot, however that came about,
//! or when it ends. A Fleet that restarts forgets the line: the session's next
//! write joins it again.
//!
//! **The grant does not wait for a turn to end.** The session was told to stop
//! when it was queued, so by the time a slot frees it is usually idle, and
//! `Runtime::moving` is only read where a turn ends. Grant to an idle session
//! therefore starts the move itself; a session still finishing that turn is left
//! to `hearing::turn_ended`, which reads `moving` as it always has.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{ManifestId, SessionTurn};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::gate::Taken;
use super::Move;
use crate::daemon::Fleet;

/// How often the line is looked at while anyone is in it, whether or not Fleet
/// saw a slot given back: `armada worktree release` from a terminal frees one
/// without telling Fleet.
pub const EVERY: Duration = Duration::from_secs(15);

/// Who waits for a slot, by repository.
#[derive(Default)]
pub(crate) struct SlotQueue {
    waiting: Mutex<HashMap<String, VecDeque<String>>>,
    wake: Notify,
}

impl SlotQueue {
    /// Put the session at the back of the repository's line unless it is in it,
    /// and say how many are ahead of it.
    pub(crate) fn join(&self, manifest: &str, session: &str) -> usize {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let line = waiting.entry(manifest.to_string()).or_default();
        match line.iter().position(|one| one == session) {
            Some(at) => at,
            None => {
                line.push_back(session.to_string());
                line.len() - 1
            }
        }
    }

    /// How many are ahead of the session: its place if it waits, and everyone
    /// if it does not, because a newcomer goes to the back.
    pub(crate) fn ahead_of(&self, manifest: &str, session: &str) -> usize {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let Some(line) = waiting.get(manifest) else {
            return 0;
        };
        line.iter().position(|one| one == session).unwrap_or(line.len())
    }

    /// The session is not waiting any more, in whichever line it was.
    pub(crate) fn leave(&self, session: &str) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.retain(|_, line| {
            line.retain(|one| one != session);
            !line.is_empty()
        });
    }

    fn head(&self, manifest: &str) -> Option<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.get(manifest).and_then(|line| line.front().cloned())
    }

    fn manifests(&self) -> Vec<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.keys().cloned().collect()
    }

    /// Whoever keeps the grant is to look now. Stored if it is not waiting, so
    /// a slot given back in the instant between two looks is not missed.
    pub(crate) fn freed(&self) {
        self.wake.notify_one();
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
    /// Lease a slot for the head of each repository's line while one is free,
    /// and move that session into it. **Strictly in order**: a head that finds
    /// the pool full ends the repository's turn, so nobody behind it is served
    /// ahead of it.
    pub(crate) async fn grant_queued_slots(self: &Arc<Self>) {
        let queue = Arc::clone(self.hosts().slot_queue());
        for manifest in queue.manifests() {
            let Ok(served) = self.served_named(Some(&ManifestId::carried(&manifest))) else {
                continue;
            };
            while let Some(id) = queue.head(&manifest) {
                match self.take_lease(&id, &served).await {
                    Taken::Full => break,
                    // Ended, or leased a slot some other way: nothing to give it.
                    Taken::Gone | Taken::Held { .. } => queue.leave(&id),
                    Taken::Refused(why) => {
                        queue.leave(&id);
                        self.fleet_note(&id, format!("Could not lease the slot it was waiting for: {why}"))
                            .await;
                    }
                    Taken::Moving(moving) => {
                        queue.leave(&id);
                        self.moved_if_idle(&id, moving).await;
                    }
                }
            }
        }
    }

    /// Start the move now if the session's turn is over. Decided under the
    /// state's lock, so a turn that ends between the lease and here takes
    /// `moving` itself and is not moved twice.
    async fn moved_if_idle(self: &Arc<Self>, id: &str, moving: Move) {
        let runtime = self.hosts().of(id);
        let idle = {
            let mut state = runtime.state();
            let idle = matches!(state.turn, SessionTurn::Idle);
            if idle {
                state.moving = None;
            }
            idle
        };
        if idle {
            self.moved_into(id, moving).await;
        }
    }
}

/// Look at the lines when a slot is given back and every [`EVERY`] besides.
pub fn keep_granting_slots<H, V, W>(fleet: Arc<Fleet<H, V, W>>, every: Duration) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let queue = Arc::clone(fleet.hosts().slot_queue());
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(every) => {}
                _ = queue.wake.notified() => {}
            }
            fleet.grant_queued_slots().await;
        }
    })
}
