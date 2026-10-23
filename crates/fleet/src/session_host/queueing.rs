//! The line a hosted session joins when its first write finds every worktree
//! slot held, and the grant that serves it when one frees.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **Memory only, one line per repository, first come first served.** A session
//! is in it once, and leaves when it is leased a slot, however that came about,
//! or when it ends. A Fleet that restarts forgets the line: the session's next
//! write joins it again.
//!
//! **The person is asked, once per stay in the line** (`WaitingSource::SlotChoice`): wait for the
//! next slot to free, or have Fleet add one. Wait changes nothing about the order. Add leases the
//! new slot to the session that asked, ahead of the line, and `granting` is held across it so the
//! grant below cannot hand the new slot to the head first.
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
use ipc::{Instant, ManifestId, SessionRecord, SessionTurn, WaitingAct, WaitingActKind, WaitingItem, WaitingOption, WaitingSource};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::gate::Taken;
use super::Move;
use crate::daemon::Fleet;

/// How often the line is looked at while anyone is in it, whether or not Fleet
/// saw a slot given back: `armada worktree release` from a terminal frees one
/// without telling Fleet.
pub const EVERY: Duration = Duration::from_secs(15);

/// The id of the call Fleet raises for a session that found every slot held.
pub(crate) const SLOT_CHOICE_ITEM: &str = "slot-choice";

/// What the person is asked when a session finds every worktree slot held. Choice 0 and 1 are the
/// options in this order: `answer_waiting` takes the index.
pub(crate) fn slot_choice_item(since: Instant) -> WaitingItem {
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    WaitingItem {
        id: SLOT_CHOICE_ITEM.to_string(),
        text: String::from("Every slot is in use"),
        since,
        source: WaitingSource::SlotChoice,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: SLOT_CHOICE_ITEM.to_string() }),
        options: vec![option("Wait for a slot"), option("Add a slot")],
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
    }
}

/// What the person has said about a session in the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    /// Not asked yet, or asked and not answered: the call is on his list.
    Pending,
    /// Wait for the next slot to free.
    Waiting,
    /// A slot is being added for this session.
    Adding,
    /// A slot was added for the session's `armada worktree lease` and is held for it.
    Added,
}

/// One session in a line.
pub(super) struct Entry {
    pub(super) manifest: String,
    pub(super) choice: Choice,
    pub(super) since: Instant,
}

#[derive(Default)]
pub(super) struct Lines {
    by_manifest: HashMap<String, VecDeque<String>>,
    pub(super) entries: HashMap<String, Entry>,
    /// Sessions whose `armada worktree lease` is asking, see [`super::slot_asking`].
    pub(super) wants: HashMap<String, super::slot_asking::Want>,
}

/// Who waits for a slot, by repository.
#[derive(Default)]
pub(crate) struct SlotQueue {
    pub(super) waiting: Mutex<Lines>,
    wake: Notify,
    /// Held while a slot is leased to someone out of turn, and across the whole of a grant, so
    /// the two never take the same slot.
    granting: tokio::sync::Mutex<()>,
}

impl SlotQueue {
    /// Put the session at the back of the repository's line unless it is in it,
    /// and say how many are ahead of it, and whether it was not in the line before.
    pub(crate) fn join(&self, manifest: &str, session: &str, now: &Instant) -> (usize, bool) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let line = waiting.by_manifest.entry(manifest.to_string()).or_default();
        match line.iter().position(|one| one == session) {
            Some(at) => (at, false),
            None => {
                line.push_back(session.to_string());
                let ahead = line.len() - 1;
                waiting.entries.insert(
                    session.to_string(),
                    Entry { manifest: manifest.to_string(), choice: Choice::Pending, since: now.clone() },
                );
                (ahead, true)
            }
        }
    }

    /// How many are ahead of the session: its place if it waits, and everyone
    /// if it does not, because a newcomer goes to the back.
    pub(crate) fn ahead_of(&self, manifest: &str, session: &str) -> usize {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let Some(line) = waiting.by_manifest.get(manifest) else {
            return 0;
        };
        line.iter().position(|one| one == session).unwrap_or(line.len())
    }

    /// The session is not waiting any more, in whichever line it was. Whether it had a call open
    /// on the person's list, which is then gone and wants publishing.
    pub(crate) fn leave(&self, session: &str) -> bool {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.by_manifest.retain(|_, line| {
            line.retain(|one| one != session);
            !line.is_empty()
        });
        waiting.entries.remove(session).is_some_and(|one| one.choice == Choice::Pending)
    }

    /// What the person chose for the session, if it is in a line and he has answered.
    pub(crate) fn chosen(&self, session: &str) -> Option<Choice> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.entries.get(session).map(|one| one.choice)
    }

    /// When the session's call was raised, while the person has not answered it. **One call**: the
    /// line's where the session is in it, and otherwise its command's, if that is still heard from.
    pub(crate) fn pending(&self, session: &str) -> Option<Instant> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        if let Some(entry) = waiting.entries.get(session) {
            return (entry.choice == Choice::Pending).then(|| entry.since.clone());
        }
        let now = std::time::Instant::now();
        waiting
            .wants
            .get(session)
            .filter(|want| want.fresh(now) && want.choice == Choice::Pending)
            .map(|want| want.since.clone())
    }

    /// Record what the person chose for a session that is in a line, and the repository it waits
    /// in. `None` where it is not in one.
    pub(crate) fn choose(&self, session: &str, choice: Choice) -> Option<String> {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let entry = waiting.entries.get_mut(session)?;
        entry.choice = choice;
        Some(entry.manifest.clone())
    }

    fn head(&self, manifest: &str) -> Option<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.by_manifest.get(manifest).and_then(|line| line.front().cloned())
    }

    fn manifests(&self) -> Vec<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.by_manifest.keys().cloned().collect()
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
        let _alone = queue.granting.lock().await;
        // A command that died without telling Fleet takes its call and its reservation with it.
        for id in queue.expire(std::time::Instant::now()) {
            let _ = self.published_hosted(&id).await;
        }
        for manifest in queue.manifests() {
            // A slot made for a command that is asking is its own until it leases it.
            if queue.reserved(&manifest, None, std::time::Instant::now()) {
                continue;
            }
            let Ok(served) = self.served_named(Some(&ManifestId::carried(&manifest))) else {
                continue;
            };
            while let Some(id) = queue.head(&manifest) {
                match self.take_lease(&id, &served).await {
                    Taken::Full => break,
                    taken => self.granted(&id, taken).await,
                }
            }
        }
    }

    /// `answer_waiting` on the slot call. **Neither choice is a message**: the agent was told to end
    /// its turn when it was queued, and is moved in when a slot is granted.
    pub(crate) async fn slot_chosen(self: &Arc<Self>, id: &str, choice: Option<u32>) -> Result<SessionRecord, api::Refusal> {
        match choice {
            Some(0) => {
                let queue = self.hosts().slot_queue();
                let (lined, asking) = (queue.choose(id, Choice::Waiting), queue.choose_want(id, Choice::Waiting));
                match lined.is_some() || asking {
                    true => self.published_hosted(id).await,
                    false => Err(self.settled_already()),
                }
            }
            Some(1) => {
                self.slot_added_for(id).await?;
                self.published_hosted(id).await
            }
            _ => Err(self.hosted_refusal(
                "fleet.session_waiting_empty",
                "whether to wait or add a slot is answered by choosing one of the two",
            )),
        }
    }

    /// The person chose to have a slot added for the session: add one and lease it to that session,
    /// ahead of the line. **The new slot is the session's** and not the head's, because the
    /// grant is held back for the whole of this. Falls back to waiting where the slot was taken in
    /// the instant between its being made and leased, and back to asking where it could not be made.
    pub(crate) async fn slot_added_for(self: &Arc<Self>, id: &str) -> Result<(), api::Refusal> {
        let queue = Arc::clone(self.hosts().slot_queue());
        let _alone = queue.granting.lock().await;
        let now = std::time::Instant::now();
        let lined = queue.choose(id, Choice::Adding);
        let Some(manifest) = lined.clone().or_else(|| queue.wanted_in(id, now)) else {
            return Err(self.settled_already());
        };
        let manifest = ManifestId::carried(&manifest);
        let asked = ipc::ChangeSlotPool { act: ipc::SlotAct::Add, slot: None, holder: None };
        if let Err(refusal) = Fleet::change_slot_pool(self, asked, Some(&manifest)) {
            if lined.is_some() {
                queue.choose(id, Choice::Pending);
            }
            return Err(refusal);
        }
        self.worktrees_changed().await;
        // One slot is made, and the session in the line is moved into it. A command of the same
        // session keeps looking for the next one; with no line, the slot is held for the command.
        if lined.is_none() {
            queue.reserve(id, now);
            return self.published_hosted(id).await.map(|_| ());
        }
        queue.choose_want(id, Choice::Waiting);
        let served = self.served_named(Some(&manifest))?;
        match self.take_lease(id, &served).await {
            Taken::Full => {
                queue.choose(id, Choice::Waiting);
            }
            taken => self.granted(id, taken).await,
        }
        let _ = self.published_hosted(id).await;
        Ok(())
    }

    /// What a lease for a session in the line came to, other than the pool being full: it leaves
    /// the line, and is moved if it was leased a slot.
    async fn granted(self: &Arc<Self>, id: &str, taken: Taken) {
        match taken {
            // Ended, or leased a slot some other way: nothing to give it.
            Taken::Gone | Taken::Held { .. } => self.line_left(id).await,
            Taken::Full => {}
            Taken::Refused(why) => {
                self.line_left(id).await;
                self.fleet_note(id, format!("Could not lease the slot it was waiting for: {why}")).await;
            }
            Taken::Moving(moving) => {
                self.line_left(id).await;
                self.moved_if_idle(id, moving).await;
            }
        }
    }

    /// Out of the line, and the call taken off his list if it was still there.
    pub(crate) async fn line_left(&self, id: &str) {
        if self.hosts().slot_queue().leave(id) {
            let _ = self.published_hosted(id).await;
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
