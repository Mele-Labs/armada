//! The line a hosted session joins when its first write finds every worktree
//! slot held, and the grant that serves it when one frees.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **One line per repository, in the Line's order.** A session is in it once, at
//! its place, and leaves when it is leased a slot, however that came about, or
//! when it ends. A newcomer goes behind everyone, which is first come first
//! served until something places one. The line is kept in the store
//! (`line_keeping`), so a Fleet that restarts serves it in the same order.
//!
//! **The person is asked, once per stay in the line** (`WaitingSource::SlotChoice`): wait for the
//! next slot to free, free a slot a Job Fleet hosts holds (`slot_holders`), or have Fleet add one.
//! Wait changes nothing about the order. Add answers at once and makes the slot after, in a task:
//! the pool list is written under `granting`, the checkout is made holding nothing, so a Free is
//! never kept waiting on it, and the slot is leased to the session that asked, ahead of the line,
//! under `granting` again. Between the two, the repository is marked as making a slot, so the grant
//! below and a newcomer's first write leave the new slot to the session that asked for it.
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
use ipc::{
    Instant, ManifestId, SessionRecord, SessionTurn, SlotHolder, WaitingAct, WaitingActKind, WaitingItem, WaitingOption,
    WaitingSource,
};
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
pub const SLOT_CHOICE_ITEM: &str = "slot-choice";

/// A session's open `slot_choice` call, as the item is drawn from it.
pub struct SlotCall {
    pub since: Instant,
    pub holders: Vec<SlotHolder>,
    pub creating_since: Option<Instant>,
    pub create_refused: Option<String>,
}

/// What the person is asked when a session finds every worktree slot held. Choice 0 and 1 are the
/// options in this order: `answer_waiting` takes the index. **Both are always sent**, whether or not
/// there are rows to free: a client that draws no rows keeps its Add.
pub fn slot_choice_item(call: SlotCall) -> WaitingItem {
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    WaitingItem {
        id: SLOT_CHOICE_ITEM.to_string(),
        text: String::from("Every slot is in use"),
        since: call.since,
        source: WaitingSource::SlotChoice,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: SLOT_CHOICE_ITEM.to_string() }),
        options: vec![option("Wait for a slot"), option("Add a slot")],
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: call.holders,
        creating_since: call.creating_since,
        create_refused: call.create_refused,
        operator_read: None,
    }
}

/// What the person has said about a session in the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Not asked yet, or asked and not answered: the call is on his list.
    Pending,
    /// Wait for the next slot to free.
    Waiting,
    /// A slot is being made for this session: the call stays, with `creating_since`.
    Adding,
    /// A slot was added for the session's `armada worktree lease` and is held for it.
    Added,
}

/// One session in a line.
pub struct Entry {
    pub manifest: String,
    /// Where it stands in its line, lowest served first.
    pub place: i64,
    pub choice: Choice,
    pub since: Instant,
    /// The slots a Job Fleet hosts holds, as the call lists them. Read by `slot_holders`.
    pub holders: Vec<SlotHolder>,
    /// When the slot the person asked for was begun, while its task is out. **Kept past a Wait**,
    /// so a second Add while one is out changes nothing.
    pub creating_since: Option<Instant>,
    /// Why the last slot was not made, until the next Add.
    pub create_refused: Option<String>,
}

/// What a press of Add found for a session in a line.
pub enum Begun {
    /// A slot is being made for it already.
    Out,
    /// Begun now, in the repository named.
    Started(String),
}

#[derive(Default)]
pub struct Lines {
    pub by_manifest: HashMap<String, VecDeque<String>>,
    pub entries: HashMap<String, Entry>,
    /// Sessions whose `armada worktree lease` is asking, see [`super::slot_asking`].
    pub wants: HashMap<String, super::slot_asking::Want>,
    /// Sessions a slot is being made for, and the repository it is made in.
    pub making: HashMap<String, String>,
}

/// Who waits for a slot, by repository.
#[derive(Default)]
pub struct SlotQueue {
    pub waiting: Mutex<Lines>,
    wake: Notify,
    /// Held while a slot is leased to someone out of turn, and across the whole of a grant, so
    /// the two never take the same slot.
    pub granting: tokio::sync::Mutex<()>,
}

impl SlotQueue {
    /// Put the session at the back of the repository's line unless it is in it, and say how many
    /// are ahead of it, and the place it took where it was not in the line before.
    pub fn join(&self, manifest: &str, session: &str, now: &Instant) -> (usize, Option<i64>) {
        self.joined(manifest, session, None, now)
    }

    /// [`Self::join`] at `place`, ahead of everyone placed after it: a line read back from the store.
    pub fn join_at(&self, manifest: &str, session: &str, place: i64, since: &Instant) -> usize {
        self.joined(manifest, session, Some(place), since).0
    }

    fn joined(&self, manifest: &str, session: &str, place: Option<i64>, since: &Instant) -> (usize, Option<i64>) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let Lines { by_manifest, entries, .. } = &mut *waiting;
        let line = by_manifest.entry(manifest.to_string()).or_default();
        if let Some(at) = line.iter().position(|one| one == session) {
            return (at, None);
        }
        let placed = |one: &String| entries.get(one).map_or(0, |entry| entry.place);
        let place = place.unwrap_or_else(|| line.back().map_or(0, |last| placed(last) + 1));
        let at = line.iter().position(|one| placed(one) > place).unwrap_or(line.len());
        line.insert(at, session.to_string());
        entries.insert(
            session.to_string(),
            Entry {
                manifest: manifest.to_string(),
                place,
                choice: Choice::Pending,
                since: since.clone(),
                holders: Vec::new(),
                creating_since: None,
                create_refused: None,
            },
        );
        (at, Some(place))
    }

    /// How many are ahead of the session: its place if it waits, and everyone
    /// if it does not, because a newcomer goes to the back.
    pub fn ahead_of(&self, manifest: &str, session: &str) -> usize {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let Some(line) = waiting.by_manifest.get(manifest) else {
            return 0;
        };
        line.iter().position(|one| one == session).unwrap_or(line.len())
    }

    /// The session is not waiting any more, in whichever line it was. Whether it had a call open
    /// on the person's list, which is then gone and wants publishing.
    pub fn leave(&self, session: &str) -> bool {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.by_manifest.retain(|_, line| {
            line.retain(|one| one != session);
            !line.is_empty()
        });
        waiting.entries.remove(session).is_some_and(|one| matches!(one.choice, Choice::Pending | Choice::Adding))
    }

    /// What the person chose for the session, if it is in a line and he has answered.
    pub fn chosen(&self, session: &str) -> Option<Choice> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.entries.get(session).map(|one| one.choice)
    }

    /// The session's call, while the person has not answered it or a slot he asked for is being
    /// made. **One call**: the line's where the session is in it, and otherwise its command's, if
    /// that is still heard from. A command's call has no rows and nothing being made.
    pub fn pending(&self, session: &str) -> Option<SlotCall> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        if let Some(entry) = waiting.entries.get(session) {
            return match entry.choice {
                Choice::Pending | Choice::Adding => Some(SlotCall {
                    since: entry.since.clone(),
                    holders: entry.holders.clone(),
                    creating_since: entry.creating_since.clone().filter(|_| entry.choice == Choice::Adding),
                    create_refused: entry.create_refused.clone(),
                }),
                _ => None,
            };
        }
        let now = std::time::Instant::now();
        waiting.wants.get(session).filter(|want| want.fresh(now) && want.choice == Choice::Pending).map(|want| SlotCall {
            since: want.since.clone(),
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
        })
    }

    /// The person pressed Add for a session in a line: a slot is to be made for it. `None` where it is
    /// in none.
    pub fn creation_begun(&self, session: &str, now: &Instant) -> Option<Begun> {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let entry = waiting.entries.get_mut(session)?;
        if entry.creating_since.is_some() {
            return Some(Begun::Out);
        }
        entry.choice = Choice::Adding;
        entry.creating_since = Some(now.clone());
        entry.create_refused = None;
        let manifest = entry.manifest.clone();
        waiting.making.insert(session.to_string(), manifest.clone());
        Some(Begun::Started(manifest))
    }

    /// The task making the slot is done, whichever way. Whether the session was still in a line with
    /// its call open, which is then to be published.
    pub fn creation_ended(&self, session: &str, refused: Option<String>) -> bool {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.making.remove(session);
        let Some(entry) = waiting.entries.get_mut(session) else {
            return false;
        };
        entry.creating_since = None;
        let Some(why) = refused else {
            return false;
        };
        entry.create_refused = Some(why);
        // A Wait pressed meanwhile stands: the call is not raised again over it.
        if entry.choice == Choice::Adding {
            entry.choice = Choice::Pending;
        }
        entry.choice == Choice::Pending
    }

    /// Record what the person chose for a session that is in a line, and the repository it waits
    /// in. `None` where it is not in one.
    pub fn choose(&self, session: &str, choice: Choice) -> Option<String> {
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
    pub fn freed(&self) {
        self.wake.notify_one();
    }
}

impl Fleet
{
    /// Lease a slot for the head of each repository's line while one is free,
    /// and move that session into it. **Strictly in order**: a head that finds
    /// the pool full ends the repository's turn, so nobody behind it is served
    /// ahead of it.
    pub async fn grant_queued_slots(self: &Arc<Self>) {
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
                match self.take_lease_resuming(&id, &served).await {
                    Taken::Full => break,
                    taken => self.granted(&id, taken).await,
                }
            }
        }
    }

    /// `answer_waiting` on the slot call. **Neither choice is a message**: the agent was told to end
    /// its turn when it was queued, and is moved in when a slot is granted.
    pub async fn slot_chosen(self: &Arc<Self>, id: &str, choice: Option<u32>) -> Result<SessionRecord, api::Refusal> {
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

    /// The person chose to have a slot added for the session. **For a session in the line this
    /// answers at once** and makes the slot in a task ([`Self::slot_created`]), because the checkout
    /// can take a while and the person may press Free meanwhile. A command with no line gets a slot
    /// held for it, as it always did.
    pub async fn slot_added_for(self: &Arc<Self>, id: &str) -> Result<(), api::Refusal> {
        let queue = Arc::clone(self.hosts().slot_queue());
        match queue.creation_begun(id, &self.instant()) {
            Some(Begun::Out) => return Ok(()),
            Some(Begun::Started(manifest)) => {
                let (fleet, id) = (Arc::clone(self), id.to_string());
                tokio::spawn(async move { fleet.slot_created(&id, &manifest).await });
                return Ok(());
            }
            None => {}
        }
        let _alone = queue.granting.lock().await;
        let now = std::time::Instant::now();
        let Some(manifest) = queue.wanted_in(id, now) else {
            return Err(self.settled_already());
        };
        let manifest = ManifestId::carried(&manifest);
        let asked = ipc::ChangeSlotPool { act: ipc::SlotAct::Add, slot: None, holder: None };
        Fleet::change_slot_pool(self, asked, Some(&manifest))?;
        self.worktrees_changed().await;
        // With no line, the slot is held for the command until it leases it.
        queue.reserve(id, now);
        self.published_hosted(id).await.map(|_| ())
    }

    /// Make the slot a session in the line asked for, and lease it to that session ahead of the line.
    /// **Nothing is held across the checkout**: the pool list is written under `granting`, the
    /// checkout is made holding no lock of Fleet's, and `granting` is taken again to lease, so a Free
    /// is never kept waiting on a slow `git worktree add`. A failure puts the call back with its
    /// reason. A session served meanwhile, by a Free or by a slot another path gave it, leaves the
    /// new slot free, and the line is looked at.
    async fn slot_created(self: &Arc<Self>, id: &str, manifest: &str) {
        let queue = Arc::clone(self.hosts().slot_queue());
        if let Err(why) = self.slot_made(&queue, manifest).await {
            if queue.creation_ended(id, Some(why)) {
                let _ = self.published_hosted(id).await;
            }
            queue.freed();
            return;
        }
        let refused = {
            let _alone = queue.granting.lock().await;
            let manifest = ManifestId::carried(manifest);
            match self.served_named(Some(&manifest)) {
                Err(refusal) => Some(refusal.error().message.clone()),
                Ok(served) => {
                    // A command of the same session keeps looking for the next one.
                    queue.choose_want(id, Choice::Waiting);
                    match self.take_lease_resuming(id, &served).await {
                        // The head of the line took it in the gap.
                        Taken::Full => Some(String::from("the new slot went to the next Job in line")),
                        taken => {
                            self.granted(id, taken).await;
                            None
                        }
                    }
                }
            }
        };
        if queue.creation_ended(id, refused) {
            let _ = self.published_hosted(id).await;
        }
        queue.freed();
    }

    /// The pool list gains a slot, and its checkout is made. `Err` is the reason, in Fleet's words.
    async fn slot_made(self: &Arc<Self>, queue: &Arc<SlotQueue>, manifest: &str) -> Result<(), String> {
        let manifest = ManifestId::carried(manifest);
        let added = {
            let _alone = queue.granting.lock().await;
            let asked = ipc::ChangeSlotPool { act: ipc::SlotAct::Add, slot: None, holder: None };
            Fleet::change_slot_pool(self, asked, Some(&manifest)).map_err(|refusal| refusal.error().message.clone())?
        };
        self.worktrees_changed().await;
        let served = self.served_named(Some(&manifest)).map_err(|refusal| refusal.error().message.clone())?;
        let (vcs, pool) = (Arc::clone(self.vcs()), crate::leasing::pool_of(&served));
        let made = tokio::task::spawn_blocking(move || vcs.make_slot(&pool, added.slot))
            .await
            .expect("git panicked making a slot");
        if made.is_err() {
            // The pool is left as it was: a slot that cannot be made is not one the next Add should stack on.
            let _alone = queue.granting.lock().await;
            let removed = ipc::ChangeSlotPool { act: ipc::SlotAct::Remove, slot: Some(added.slot), holder: None };
            let _ = Fleet::change_slot_pool(self, removed, Some(&manifest));
        }
        made
    }

    /// What a lease for a session in the line came to, other than the pool being full: it leaves
    /// the line, and is moved if it was leased a slot.
    pub async fn granted(self: &Arc<Self>, id: &str, taken: Taken) {
        match taken {
            // Ended, or leased a slot some other way: nothing to give it.
            Taken::Gone | Taken::Held { .. } => self.line_left(id).await,
            Taken::Full => {}
            // A paused Job whose Resume waited for the slot: now it carries on.
            Taken::Resumed => {
                self.line_left(id).await;
                self.pause_lifted(id).await;
            }
            Taken::Refused(why) => {
                self.line_left(id).await;
                self.fleet_note(id, format!("Could not lease the slot it was waiting for: {why}")).await;
            }
            Taken::Moving(moving) => {
                self.line_left(id).await;
                self.slot_wait_began(id, "move").await;
                self.moved_if_idle(id, moving).await;
            }
        }
    }

    /// Out of the line, and the call taken off his list if it was still there.
    pub async fn line_left(&self, id: &str) {
        let had_a_call = self.hosts().slot_queue().leave(id);
        self.line_let_go(id).await;
        if had_a_call {
            self.slot_wait_ended(id, Some("line")).await;
            let _ = self.published_hosted(id).await;
        }
    }

    /// A wait for a slot began (`line` or `move`), kept for the split of the Job's time. A fault
    /// here fails nothing: the wait is only counted.
    pub async fn slot_wait_began(&self, id: &str, phase: &str) {
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.begin_slot_wait(id, phase, &at);
    }

    /// A Job's process ended on its own, kept for the split of the Job's time.
    pub async fn stop_kept(&self, id: &str) {
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.keep_session_stop(id, &at);
    }

    /// The wait of `phase` ended, or every standing wait of the Job where it is `None`.
    pub async fn slot_wait_ended(&self, id: &str, phase: Option<&str>) {
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.end_slot_waits(id, phase, &at);
    }

    /// Start the move now if the session's turn is over and no agent of it is working: one that is
    /// leaves `moving` for `held_moves_made`. Decided under the state's lock, so a turn that ends
    /// between the lease and here takes `moving` itself and is not moved twice.
    async fn moved_if_idle(self: &Arc<Self>, id: &str, moving: Move) {
        let runtime = self.hosts().of(id);
        let working = self.work_beyond_the_turn(id).await;
        let idle = {
            let mut state = runtime.state();
            let idle = matches!(state.turn, SessionTurn::Idle) && !working;
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

/// Look at the lines when a slot is given back and every [`EVERY`] besides, and read who holds the
/// slots wherever a call is open.
pub fn keep_granting_slots(fleet: Arc<Fleet>, every: Duration) -> JoinHandle<()>
{
    let queue = Arc::clone(fleet.hosts().slot_queue());
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(every) => {}
                _ = queue.wake.notified() => {}
            }
            fleet.grant_queued_slots().await;
            fleet.refresh_slot_holders().await;
        }
    })
}
