//! The Operator: one per machine, deciding who does each piece of work that arrives. `armada-ehhu`,
//! and the design page *Helm becomes the Operator*, revision 41.
//!
//! **Fleet detects, a Policy decides, the record remembers.** Fleet hands over an [`Arrival`] with
//! the ways its rules leave open, and finds the live Jobs that share something with it
//! (`candidates`); one fresh [`Calling`] with no tools names one of them; the ruling, or why there
//! was none, goes in the Log through the [`Desk`], signed by the Operator, and [`Acting`] does it.
//!
//! **An error Bridge showed the person goes first.** In the queue it stands ahead of every waiting
//! arrival that is not one, and errors keep the order they came in.
//!
//! **Only what nobody owns reaches it**: a note, an error, a bug report, a new issue, a red main.
//! A Job's own events (its start, plan, question, card, stop, red pull request, Later item and end)
//! are never handed to it, and nothing a Job does waits on its ruling. Questions and permission
//! cards go to the person; an end and a merge are heard once (`hearing`) and only Discussion sees them.
//!
//! **One call out at a time on this machine.** What arrives while a call is out is ruled together
//! in the next call, so several items are seen side by side.
//!
//! **Fail open.** A call that fails, answers what cannot be read, or rules what Fleet did not offer
//! passes the item on untouched, and so does one with no ruling by the time it could wait
//! ([`NEEDS_YOU_WAIT`]); so does every item once the week's plan is used to the
//! Operator's pause ([`Paused`]). A ruling that comes back after its item passed on is dropped.

mod acting;
mod answering;
mod arriving;
mod asking;
mod boarding;
mod briefing;
mod calling;
mod candidates;
mod closing;
mod desk;
mod erroring;
mod handing;
mod hearing;
mod journal;
mod looking;
mod orders;
mod paging;
mod placing;
mod raising;
mod reading;
mod ruling;
mod seat;
pub(crate) mod shortlist;
mod sources;
mod sourcing;
mod suggesting;
mod talking;
mod thening;
mod turning;

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use core_model::{Actor, Timestamp};
use tokio::sync::oneshot;

pub use arriving::{Arrival, Arrived, InLine, LiveJob, Source};
pub use boarding::{OnBoard, Unknown};
use candidates::Near;
pub use calling::{Answering, Calling, KeptStarted};
pub use looking::{Blind, Folder, Looking, ReadingScout, ToRead};
pub use acting::FleetActing;
#[cfg(test)]
pub(crate) use erroring::fingerprint;
pub use desk::FleetDesk;
pub use hearing::keep_the_ends_told;
pub use raising::keep_the_raises;
#[cfg(test)]
pub(crate) use raising::ToRaise;
pub use orders::{Order, Orders};
pub(crate) use seat::Seat;
pub use ruling::{Grouping, Outcome, PassedOn, Place, Ruled, Ruling, Way};
pub use sources::{BridgeError, Filed, MergeNear, NewIssue, Note, RedMain, Report};
pub(crate) use sourcing::Sources;
pub use talking::{Board, EndedJob, HeldJob, HoldRead, JobRead, LogRead, MergedPull, NeedsYou, Repository, RuleRead, StoppedJob, SuggestionRead, TalkFailed, TalkLine, Talked, TurnRead};
pub(crate) use answering::is_question;
pub(crate) use suggesting::{suggestion_items, SKIP};

/// How long a Needs you item waits for its ruling before it pulses.
pub const NEEDS_YOU_WAIT: Duration = Duration::from_secs(60);

/// The week's plan used to the share where the Operator pauses: both in percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paused {
    pub week: u32,
    pub at: u32,
}

/// What a [`Desk`] answers, in time.
pub type Done<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Fleet, as far as the Operator reaches it.
pub trait Desk: Send + Sync {
    fn now(&self) -> Timestamp;
    /// Whether the week's plan is used to the Operator's pause, as the setting and the plan's reading
    /// stand now. **No reading of the plan is `None`**, so no reading makes no pause.
    fn plan_pause(&self) -> Done<'_, Option<Paused>> {
        Box::pin(async { None })
    }
    /// The Jobs live now, which a call may name.
    fn live(&self) -> Done<'_, Vec<LiveJob>>;
    /// Keep one row of the Log.
    fn keep(&self, row: Logged) -> Done<'_, ()>;
    /// The Jobs waiting to start, each repository's in its order. None where Fleet keeps no line.
    fn waiting(&self) -> Done<'_, Vec<InLine>> {
        Box::pin(async { Vec::new() })
    }
    /// What the box may answer from.
    fn board(&self) -> Done<'_, Board> {
        Box::pin(async { Board::default() })
    }
    /// The open tasks and last words of the newest of `live`, which a message to the box is handed.
    fn jobs_read(&self, _live: &[LiveJob]) -> Done<'_, Vec<talking::JobRead>> {
        Box::pin(async { Vec::new() })
    }
    /// What a message to the box may read, and where its folder is made. `None` where this desk keeps
    /// nothing to read: the message is put to the call that holds no tools.
    fn to_read(&self, _live: &[LiveJob], _needs: &[NeedsYou]) -> Done<'_, Option<ToRead>> {
        Box::pin(async { None })
    }
    /// The board changed: an arrival came, went into a call, was ruled, or was reordered.
    fn board_changed(&self) {}
}

/// What an act came to once it was carried out: done, or why Fleet did not.
pub type Acted<'a> = Done<'a, Result<(), String>>;

/// An act that came to what it was ruled.
pub fn carried() -> Acted<'static> {
    Box::pin(async { Ok(()) })
}

/// What Fleet does to Jobs on a ruling, called as it comes back and never for one that came late.
/// **Each act is Fleet's to refuse**: a Job that has pushed is not the call's to weigh. **Each says
/// how it came out**, since the Log keeps a ruling as done only once it is.
pub trait Acting: Send + Sync {
    /// Whether the Operator is on now. **Read as each ruling comes back**, so a call out when it was
    /// turned off acts on nothing.
    fn on(&self) -> bool {
        true
    }
    /// What `ruling` would do is kept, to wait on the person's go-ahead in Your turn.
    fn suggest(&self, _arrival: &Arrival, _ruling: &Ruling) -> Acted<'_> {
        carried()
    }
    /// `suggest`, but one that repeats a suggestion kept before, standing, skipped or done, is not kept
    /// and says which it repeats: `Some` with the Log's reason.
    fn suggest_once(&self, arrival: &Arrival, ruling: &Ruling) -> Done<'_, Result<Option<String>, String>> {
        let kept = self.suggest(arrival, ruling);
        Box::pin(async move { kept.await.map(|()| None) })
    }
    /// Work from outside a Job's own run is done as `ruling` says: a report linked or handed on, a
    /// red main fixed, a note or an error filed, started or handed on, or any of them raised.
    fn sourced(&self, _arrival: &Arrival, _ruling: &Ruling) -> Acted<'_> {
        carried()
    }
}

/// One row of the Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logged {
    pub arrival: String,
    pub arrived: Arrived,
    pub source: Source,
    pub session_id: String,
    pub manifest_id: String,
    pub subject: String,
    pub outcome: Outcome,
    pub other: Option<String>,
    pub shared: Vec<String>,
    pub because: String,
    /// The call that ruled it, the same for items ruled together; `None` where none was made.
    pub call: Option<String>,
    pub by: Actor,
    /// When the item was handed to the Operator: before `at` where a call was out.
    pub handed: Timestamp,
    pub at: Timestamp,
    /// Where a person's drag put a Job in Waiting to start.
    pub place: Option<Place>,
    /// Each part of the ruling Fleet refused while the rest stood.
    pub refused: Vec<String>,
    /// The open issue a link named.
    pub issue: Option<String>,
}

impl Logged {
    fn of(arrival: &Arrival, ruled: &Ruled, call: Option<String>, handed: Timestamp, at: Timestamp) -> Logged {
        let (outcome, other, shared, because) = match ruled {
            Ruled::Ruled(ruling) => (Outcome::Ruled(ruling.way), ruling.other.clone(), ruling.shared.clone(), ruling.because.clone()),
            Ruled::PassedOn(why) => (Outcome::PassedOn, None, Vec::new(), why.to_string()),
        };
        let (refused, issue) = match ruled {
            Ruled::Ruled(ruling) => (ruling.refused.clone(), ruling.issue.clone()),
            Ruled::PassedOn(_) => (Vec::new(), None),
        };
        Logged {
            arrival: arrival.id.clone(),
            arrived: arrival.arrived,
            source: arrival.source.clone(),
            session_id: arrival.session_id.clone(),
            manifest_id: arrival.manifest_id.clone(),
            subject: arrival.subject.clone(),
            outcome,
            other,
            shared,
            because,
            call,
            by: Actor::Operator,
            handed,
            at,
            place: None,
            refused,
            issue,
        }
    }

    /// The row as the store keeps it.
    pub fn kept(&self) -> store::KeptRuling {
        store::KeptRuling {
            arrival: self.arrival.clone(),
            arrived: self.arrived.word().to_string(),
            source: self.source.word().to_string(),
            source_session: self.source.session().map(str::to_string),
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.subject.clone(),
            outcome: self.outcome.word().to_string(),
            other_session: self.other.clone(),
            shared: self.shared.clone(),
            because: self.because.clone(),
            call: self.call.clone(),
            by: self.by.as_wire().to_string(),
            handed: self.handed.as_str().to_string(),
            at: self.at.as_str().to_string(),
            place: self.place.as_ref().map(Place::word),
            first_session: None,
            refused: self.refused.clone(),
            answer: None,
            issue: self.issue.clone(),
        }
    }
}

/// One item waiting on a ruling, and where its ruling goes.
struct Held {
    token: u64,
    handed: Timestamp,
    arrival: Arrival,
    tell: oneshot::Sender<Ruled>,
}

#[derive(Default)]
struct Queue {
    /// Arrived, and not yet in a call.
    pending: Vec<Held>,
    /// In the call that is out, and still waiting on it.
    out: Vec<Held>,
    /// The call that is out.
    call: Option<String>,
    /// A pass is running: it makes the calls, one after another, until nothing is pending.
    running: bool,
    next: u64,
}

/// The Operator of this machine.
pub struct Operator {
    call: Arc<dyn Calling>,
    /// What a message to the box is put to. A ruling is never: it goes to `call`.
    looking: Arc<dyn Looking>,
    desk: Arc<dyn Desk>,
    acting: Arc<dyn Acting>,
    queue: Mutex<Queue>,
}

impl Operator {
    /// An Operator whose messages are put to `call` too, which reads nothing.
    pub fn new(call: Arc<dyn Calling>, desk: Arc<dyn Desk>, acting: Arc<dyn Acting>) -> Arc<Operator> {
        let looking = Arc::new(Blind(Arc::clone(&call)));
        Operator::new_looking(call, looking, desk, acting)
    }

    /// An Operator that rules through `call` and puts a message to `looking`, which may read.
    pub fn new_looking(call: Arc<dyn Calling>, looking: Arc<dyn Looking>, desk: Arc<dyn Desk>, acting: Arc<dyn Acting>) -> Arc<Operator> {
        Arc::new(Operator { call, looking, desk, acting, queue: Mutex::new(Queue::default()) })
    }

    /// Rule on `arrival`, waiting at most `within`. **Never fails**: an item with no ruling in
    /// time is passed on, and the Log says why.
    pub async fn ruled(self: &Arc<Self>, arrival: Arrival, within: Duration) -> Ruled {
        let (tell, mut told) = oneshot::channel();
        let handed = self.desk.now();
        let (token, start) = {
            let mut queue = held(&self.queue);
            let token = queue.next;
            queue.next += 1;
            // An error goes ahead of everything waiting that is not one, after the errors before it.
            let at = match arrival.arrived {
                Arrived::Error => queue.pending.iter().position(|one| one.arrival.arrived != Arrived::Error),
                _ => None,
            };
            let one = Held { token, handed, arrival, tell };
            match at {
                Some(at) => queue.pending.insert(at, one),
                None => queue.pending.push(one),
            }
            let start = !queue.running;
            queue.running = true;
            (token, start)
        };
        self.desk.board_changed();
        if start {
            tokio::spawn(Arc::clone(self).pass());
        }
        if let Ok(answered) = tokio::time::timeout(within, &mut told).await {
            return answered.unwrap_or_else(|_| Ruled::PassedOn(PassedOn::Failed("the Operator stopped".into())));
        }
        let late = {
            let mut queue = held(&self.queue);
            let call = queue.call.clone();
            match take(&mut queue.pending, token) {
                Some(held) => Some((held, None)),
                None => take(&mut queue.out, token).map(|held| (held, call)),
            }
        };
        match late {
            Some((held, call)) => {
                let ruled = Ruled::PassedOn(PassedOn::Late(within));
                self.desk.keep(Logged::of(&held.arrival, &ruled, call, held.handed.clone(), self.desk.now())).await;
                self.desk.board_changed();
                ruled
            }
            // Ruled as the wait ran out: the ruling is already on its way.
            None => told.await.unwrap_or_else(|_| Ruled::PassedOn(PassedOn::Failed("the Operator stopped".into()))),
        }
    }

    /// Make calls one after another until nothing is pending.
    async fn pass(self: Arc<Self>) {
        loop {
            // Into `out` at once, so an item whose wait runs out from here on is found there.
            let taken: Vec<(u64, Arrival)> = {
                let mut queue = held(&self.queue);
                if queue.pending.is_empty() {
                    queue.running = false;
                    return;
                }
                let all = std::mem::take(&mut queue.pending);
                let items = all.iter().map(|one| (one.token, one.arrival.clone())).collect();
                queue.out = all;
                items
            };
            self.desk.board_changed();
            let live = self.desk.live().await;
            let mut asked = Vec::new();
            for (token, mut arrival) in taken {
                let near = candidates::near(&arrival, &live);
                let alone = near.is_empty() && arrival.ways.iter().any(|way| way.names_another());
                if alone {
                    arrival.ways.retain(|way| !way.names_another());
                }
                // An issue is put to a call for its read, whatever its ways.
                if arrival.ways.len() > 1 || arrival.arrived.is_read() {
                    asked.push((token, arrival, Asked { near }));
                    continue;
                }
                let Some(one) = take(&mut held(&self.queue).out, token) else { continue };
                let because = match alone {
                    true => "no live Job shares a path, an issue or its words with it",
                    false => "it is the only way Fleet's rules leave it",
                };
                let ruled = match arrival.ways.first() {
                    Some(way) => Ruled::Ruled(Ruling::alone(*way, because)),
                    None => Ruled::PassedOn(PassedOn::Refused("Fleet's rules leave it no way".into())),
                };
                self.settled_apart(one, ruled, None);
            }
            if asked.is_empty() {
                continue;
            }
            let now = self.desk.now();
            if let Some(paused) = self.desk.plan_pause().await {
                let still = std::mem::take(&mut held(&self.queue).out);
                for one in still {
                    self.settled_apart(one, Ruled::PassedOn(PassedOn::Paused(paused)), None);
                }
                continue;
            }
            self.call_out(asked, now).await;
        }
    }

    /// One call over the items in `out`, and every ruling it brings settled.
    async fn call_out(self: &Arc<Self>, asked: Vec<(u64, Arrival, Asked)>, now: Timestamp) {
        let call = {
            let mut queue = held(&self.queue);
            if queue.out.is_empty() {
                return;
            }
            let call = format!("{}#{}", now.as_str(), queue.next);
            queue.next += 1;
            queue.call = Some(call.clone());
            call
        };
        self.desk.board_changed();
        let at: BTreeMap<u64, usize> = asked.iter().enumerate().map(|(at, (token, _, _))| (*token, at)).collect();
        let mut items = Vec::new();
        let mut near: Vec<Vec<Near>> = Vec::new();
        for (_, arrival, one) in asked {
            items.push(arrival);
            near.push(one.near);
        }
        let rulings = match self.call.ask(asking::question(&items, &near)).await {
            Ok(answer) => ruling::read_rulings(&answer, &items, &near),
            Err(failed) => vec![Ruled::PassedOn(PassedOn::Failed(failed.in_a_line("Operator"))); items.len()],
        };
        let still = {
            let mut queue = held(&self.queue);
            queue.call = None;
            std::mem::take(&mut queue.out)
        };
        self.desk.board_changed();
        for one in still {
            // An item settled without a call left `out` before it.
            let Some(ruled) = at.get(&one.token).map(|at| rulings[*at].clone()) else { continue };
            self.settled_apart(one, ruled, Some(call.clone()));
        }
    }

    /// `settle` in a task of its own, so an act that takes a while holds nothing else up.
    fn settled_apart(self: &Arc<Self>, one: Held, ruled: Ruled, call: Option<String>) {
        tokio::spawn(Arc::clone(self).settle(one, ruled, call));
    }

    /// An item ruled: carried out, or kept to ask, then kept in the Log and its ruling told.
    /// **A row of the Log says what was done only once it is**: a link's after it came out, a
    /// refused act as what refused it, and an act that asks as one kept to wait on a go-ahead.
    async fn settle(self: Arc<Self>, one: Held, mut ruled: Ruled, call: Option<String>) {
        let Held { handed, arrival, tell, .. } = one;
        let mut row_outcome = None;
        if matches!(ruled, Ruled::Ruled(_)) && !self.acting.on() {
            ruled = Ruled::PassedOn(PassedOn::Off);
        }
        if let Ruled::Ruled(ruling) = &ruled {
            match self.act(&arrival, ruling).await {
                Ok(Carried::Done) => {}
                Ok(Carried::Proposed) => row_outcome = Some(Outcome::Proposed(ruling.way)),
                Ok(Carried::Repeat(of)) => ruled = Ruled::Ruled(Ruling::alone(Way::Noted, of)),
                Err(why) => ruled = Ruled::PassedOn(PassedOn::Undone(why)),
            }
        }
        let mut row = Logged::of(&arrival, &ruled, call, handed, self.desk.now());
        if let Some(outcome) = row_outcome {
            row.outcome = outcome;
        }
        self.desk.keep(row).await;
        self.desk.board_changed();
        let _ = tell.send(ruled);
    }

    /// What Fleet does on `ruling`. **An act asks**: a new Job, words to a live Job and a new issue are
    /// kept as a suggestion, which raising turns into a go-ahead in Your turn, answered Yes, Always
    /// here, Always or No, and which a kept rule covering it does as it is raised. A link writes
    /// nothing, and a raise, a leave and a note are not acts, so they are done as they came.
    async fn act(&self, arrival: &Arrival, ruling: &Ruling) -> Result<Carried, String> {
        match ruling.way {
            Way::NewJob | Way::ExistingJob | Way::File | Way::Close | Way::Group => self.acting.suggest_once(arrival, ruling).await.map(Carried::kept),
            Way::Raise | Way::Link | Way::Leave | Way::Noted => self.acted(arrival, ruling).await.map(|()| Carried::Done),
        }
    }

    /// `ruling` done: a link as it came back, or on the person's go-ahead on a suggestion. **Whether it came out**, so a Log row is not kept for what did not happen.
    pub(crate) async fn acted(&self, arrival: &Arrival, ruling: &Ruling) -> Result<(), String> {
        self.acting.sourced(arrival, ruling).await
    }
}

/// What came of a ruling on its way to the Log.
enum Carried {
    /// It was done.
    Done,
    /// It was kept to wait on a press.
    Proposed,
    /// It repeated a suggestion kept before, so none was kept. Noted in the Log, saying which.
    Repeat(String),
}

impl Carried {
    /// A suggestion as it came out: kept to wait on a press, or a repeat that was not.
    fn kept(repeat: Option<String>) -> Carried {
        repeat.map_or(Carried::Proposed, Carried::Repeat)
    }
}

/// What a call is shown beside one item: the live Jobs near it.
struct Asked {
    near: Vec<Near>,
}

fn take(from: &mut Vec<Held>, token: u64) -> Option<Held> {
    let at = from.iter().position(|one| one.token == token)?;
    Some(from.remove(at))
}

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}
