//! The Operator: one per machine, deciding who does each piece of work that arrives. `armada-ehhu`,
//! and the design page *Helm becomes the Operator*, revision 41.
//!
//! **Fleet detects, a Policy decides, the record remembers.** Fleet hands over an [`Arrival`] with
//! the ways its rules leave open, and finds the live Jobs that share something with it
//! (`candidates`); one fresh [`Calling`] with no tools names one of them; the ruling, or why there
//! was none, goes in the Log through the [`Desk`], signed by the Operator, and [`Acting`] does it.
//!
//! **One call out at a time on this machine.** What arrives while a call is out is ruled together
//! in the next call, so three Jobs dispatched at once are seen side by side.
//!
//! **It sets the order of Waiting to start.** A start where Jobs already wait for a slot in its
//! repository is ruled a place among them, which Fleet applies as it joins the line; a person's drag
//! moves it again and is kept as a correction. Of two Jobs kept in touch, the one that must build
//! first is named, and the other is held with a `StartAfter` until it pushes (`holding`).
//!
//! **Fail open.** A call that fails, answers what cannot be read, or rules what Fleet did not offer
//! passes the item on untouched, and so does one with no ruling by the time it could wait
//! ([`FIRST_WRITE_WAIT`], [`NEEDS_YOU_WAIT`]); so does every item once the day's calls reach the
//! ceiling. A ruling that comes back after its item passed on is dropped.

pub mod acting;
pub mod answering;
pub mod arriving;
pub mod asking;
pub mod boarding;
pub mod calling;
pub mod candidates;
pub mod desk;
pub mod handing;
pub mod holding;
pub mod orders;
pub mod placing;
pub mod questions;
pub mod ruling;
pub mod seat;
pub mod shortlist;
pub mod sources;
pub mod sourcing;
pub mod stalling;
pub mod stalls;
pub mod suggesting;
pub mod talking;

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use core_model::{Actor, Timestamp};
use ipc::operator_positions::{OperatorPosition, OperatorRole};
use tokio::sync::oneshot;

pub use arriving::{Arrival, Arrived, InLine, LiveJob, Source};
pub use boarding::{OnBoard, Unknown};
use candidates::Near;
pub use calling::{Answering, Calling, KeptStarted};
pub use acting::FleetActing;
pub use answering::answers;
pub use desk::FleetDesk;
pub use orders::{Order, Orders};
pub use seat::{Moved, Seat};
pub use questions::{Choice, Earlier, Group, Kind, Present, Question, ALLOW_ONCE};
pub use ruling::{Outcome, PassedOn, Place, Ruled, Ruling, Way};
pub use sources::{Filed, Handed, NewIssue, Note, RedMain, Report, ReviewRequest};
pub use sourcing::Sources;
pub use stalling::{seen_of, Stalls};
pub use stalls::{Quiet, Stall};
pub use talking::{Board, EndedJob, HeldJob, LogRead, NeedsYou, Repository, StoppedJob, TalkFailed, TalkLine, Talked};
pub use suggesting::{reads_shown, suggestion_items, SKIP};

/// How long a Job's first write waits for the ruling on its start.
pub const FIRST_WRITE_WAIT: Duration = Duration::from_secs(20);
/// How long a Needs you item waits for its ruling before it pulses.
pub const NEEDS_YOU_WAIT: Duration = Duration::from_secs(60);

/// What a [`Desk`] answers, in time.
pub type Done<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Fleet, as far as the Operator reaches it.
pub trait Desk: Send + Sync {
    fn now(&self) -> Timestamp;
    /// The instant this machine's day began, at its local midnight: the ceiling counts from it.
    fn day_began(&self) -> Done<'_, Timestamp>;
    /// The most calls a day, as the setting reads now.
    fn daily_calls(&self) -> u32;
    /// The Jobs live now, which a call may name.
    fn live(&self) -> Done<'_, Vec<LiveJob>>;
    /// How many calls the Log names from `since`. **A count it cannot read is the ceiling
    /// reached**, so a broken count makes no calls rather than unbounded ones.
    fn calls_since(&self, since: Timestamp) -> Done<'_, u32>;
    /// Keep one row of the Log.
    fn keep(&self, row: Logged) -> Done<'_, ()>;
    /// The Jobs waiting to start, each repository's in its order. None where Fleet keeps no line.
    fn waiting(&self) -> Done<'_, Vec<InLine>> {
        Box::pin(async { Vec::new() })
    }
    /// Why an answer to `arrival` can no longer be given, as the call comes back: the person
    /// answered it first, or the Operator was turned off. `None` where it still can.
    fn settled(&self, _arrival: &Arrival) -> Done<'_, Option<String>> {
        Box::pin(async { None })
    }
    /// The standing orders, which [`Operator::pass`] goes by before any call.
    fn orders(&self) -> Done<'_, Orders> {
        Box::pin(async { Orders::default() })
    }
    /// What the box may answer from.
    fn board(&self) -> Done<'_, Board> {
        Box::pin(async { Board::default() })
    }
    /// The board changed: an arrival came, went into a call, was ruled, or was reordered.
    fn board_changed(&self) {}
}

/// What Fleet does to Jobs on a ruling, called once it is logged and never for one that came late.
/// **Each act is Fleet's to refuse**: the role's position, read as the ruling comes back, and a Job
/// that has pushed are not the call's to weigh.
pub trait Acting: Send + Sync {
    /// How far `role` goes in the repository `manifest`, as the settings read now. `None` is a way
    /// no role governs, which goes while the Operator is on. **Read as each ruling comes back**, so
    /// a change holds from the next arrival and a call out when it changed goes by the new position.
    fn position(&self, _role: Option<OperatorRole>, _manifest: &str) -> OperatorPosition {
        OperatorPosition::On
    }
    /// The role that `ruling` falls to stands at Suggest: what it would do waits on one press.
    fn suggest(&self, _arrival: &Arrival, _ruling: &Ruling) {}
    /// Nothing live touches `session`: a Job held on its start card starts, as his press would.
    fn go_ahead(&self, session: &str);
    /// A live Job's plan covers `session`'s work, or `group` of its plan: the words go to `into`,
    /// and a whole Job ends.
    fn hand_over(&self, session: &str, group: Option<&str>, into: &str, shared: &[String]);
    /// `ending` stops, and its brief, plan and branch go to `into` as one message.
    fn merge(&self, ending: &str, into: &str, shared: &[String]);
    /// Both carry on, each told the other's plan group and the paths they share.
    fn keep_in_touch(&self, one: &str, other: &str, shared: &[String]);
    /// What is left of a Job gone quiet starts as a Job of its own, briefed with `seen`.
    fn start_rest(&self, _session: &str, _seen: &str) {}
    /// A Job gone quiet goes to the person, with the call's `read`.
    fn raise(&self, _session: &str, _read: &str) {}
    /// `session` goes at `place` in Waiting to start, now if it waits there and else as it joins.
    fn place(&self, _session: &str, _place: &Place) {}
    /// Of two kept in touch, `held` waits until `first` pushes, where it has not begun.
    fn hold(&self, _held: &str, _first: &str, _shared: &[String]) {}
    /// Item `item` on `session`'s Needs you list is settled as the person's press on `choice`
    /// would, and the Job's thread says so in the answer's words.
    fn answer(&self, _session: &str, _item: &str, _choice: &Choice, _because: &str) {}
    /// Work from outside a Job's own run is done as `ruling` says: a report linked or handed on, a
    /// red main fixed, a Later item linked or filed, or any of them raised.
    fn sourced(&self, _arrival: &Arrival, _ruling: &Ruling) {}
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
    /// Where a start goes in Waiting to start.
    pub place: Option<Place>,
    /// Of two kept in touch, the one that builds first.
    pub first: Option<String>,
    /// Each part of the ruling Fleet refused while the rest stood.
    pub refused: Vec<String>,
    /// The answer given, in its option's words: by the Operator, or by the person's own press.
    pub answer: Option<String>,
    /// The open issue a link named.
    pub issue: Option<String>,
}

impl Logged {
    fn of(arrival: &Arrival, ruled: &Ruled, call: Option<String>, handed: Timestamp, at: Timestamp) -> Logged {
        let (outcome, other, shared, because) = match ruled {
            Ruled::Ruled(ruling) => (Outcome::Ruled(ruling.way), ruling.other.clone(), ruling.shared.clone(), ruling.because.clone()),
            Ruled::PassedOn(why) => (Outcome::PassedOn, None, Vec::new(), why.to_string()),
        };
        let (place, first, refused, answer, issue) = match ruled {
            Ruled::Ruled(ruling) => (
                ruling.place.clone(),
                ruling.first.clone(),
                ruling.refused.clone(),
                ruling.answer.as_ref().map(|one| one.label.clone()),
                ruling.issue.clone(),
            ),
            Ruled::PassedOn(_) => (None, None, Vec::new(), None, None),
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
            place,
            first,
            refused,
            answer,
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
            first_session: self.first.clone(),
            refused: self.refused.clone(),
            answer: self.answer.clone(),
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
    desk: Arc<dyn Desk>,
    acting: Arc<dyn Acting>,
    queue: Mutex<Queue>,
}

impl Operator {
    pub fn new(call: Arc<dyn Calling>, desk: Arc<dyn Desk>, acting: Arc<dyn Acting>) -> Arc<Operator> {
        Arc::new(Operator { call, desk, acting, queue: Mutex::new(Queue::default()) })
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
            queue.pending.push(Held { token, handed, arrival, tell });
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
            let waiting = self.desk.waiting().await;
            let orders = self.desk.orders().await;
            let mut asked = Vec::new();
            for (token, mut arrival) in taken {
                if orders.always_raises(arrival.arrived) {
                    let Some(one) = take(&mut held(&self.queue).out, token) else { continue };
                    self.settle(one, Ruled::Ruled(Ruling::alone(Way::Raise, "a standing order always raises this")), None).await;
                    continue;
                }
                // A question is answered for its own Job: no other is near it.
                let near = match arrival.arrived.is_asked() {
                    true => Vec::new(),
                    false => candidates::near(&arrival, &live),
                };
                let alone = near.is_empty() && arrival.ways.iter().any(|way| way.names_another());
                if alone {
                    arrival.ways.retain(|way| !way.names_another());
                }
                let line = line_of(&arrival, &waiting);
                // A start where others wait is put to a call for its place, and an issue or a review
                // request for its read, whatever their ways.
                if arrival.ways.len() > 1 || !line.is_empty() || arrival.arrived.is_read() {
                    asked.push((token, arrival, Asked { near, line }));
                    continue;
                }
                let Some(one) = take(&mut held(&self.queue).out, token) else { continue };
                let because = match (alone, arrival.rule.as_deref()) {
                    (true, _) => "no live Job shares a path, an issue or its words with it",
                    (false, Some(rule)) => rule,
                    (false, None) => "it is the only way Fleet's rules leave it",
                };
                let ruled = match arrival.ways.first() {
                    Some(way) => Ruled::Ruled(Ruling::alone(*way, because)),
                    None => Ruled::PassedOn(PassedOn::Refused("Fleet's rules leave it no way".into())),
                };
                self.settle(one, ruled, None).await;
            }
            if asked.is_empty() {
                continue;
            }
            let now = self.desk.now();
            let ceiling = self.desk.daily_calls();
            if self.desk.calls_since(self.desk.day_began().await).await >= ceiling {
                let still = std::mem::take(&mut held(&self.queue).out);
                for one in still {
                    self.settle(one, Ruled::PassedOn(PassedOn::Ceiling(ceiling)), None).await;
                }
                continue;
            }
            self.call_out(asked, now).await;
        }
    }

    /// One call over the items in `out`, and every ruling it brings settled.
    async fn call_out(&self, asked: Vec<(u64, Arrival, Asked)>, now: Timestamp) {
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
        let (mut near, mut line): (Vec<Vec<Near>>, Vec<Vec<InLine>>) = (Vec::new(), Vec::new());
        for (_, arrival, one) in asked {
            items.push(arrival);
            near.push(one.near);
            line.push(one.line);
        }
        let rulings = match self.call.ask(asking::question(&items, &near, &line)).await {
            Ok(answer) => ruling::read_rulings(&answer, &items, &near, &line),
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
            self.settle(one, ruled, Some(call.clone())).await;
        }
    }

    async fn settle(&self, one: Held, mut ruled: Ruled, call: Option<String>) {
        if matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::Answer) {
            if let Some(why) = self.desk.settled(&one.arrival).await {
                ruled = Ruled::PassedOn(PassedOn::Settled(why));
            }
        }
        self.desk.keep(Logged::of(&one.arrival, &ruled, call, one.handed.clone(), self.desk.now())).await;
        self.desk.board_changed();
        if let Ruled::Ruled(ruling) = &ruled {
            self.act(&one.arrival, ruling);
        }
        let _ = one.tell.send(ruled);
    }

    /// What Fleet does on `ruling`, by the position of the role it falls to. **A note's ruling, but
    /// a raise, is kept as a suggestion whatever the role stands at**, short of Off.
    fn act(&self, arrival: &Arrival, ruling: &Ruling) {
        match self.acting.position(role_of(arrival.arrived, ruling.way), &arrival.manifest_id) {
            OperatorPosition::Off => {}
            // A note is the person's own idea, so Fleet only proposes what to do with it, and a
            // raise is already a row on his list.
            OperatorPosition::On if arrival.arrived == Arrived::Note && ruling.way != Way::Raise => self.acting.suggest(arrival, ruling),
            OperatorPosition::On => self.acted(arrival, ruling),
            OperatorPosition::Suggest => match ruling.way {
                // A raise is already a row on his list.
                Way::Raise => self.acted(arrival, ruling),
                // A start's own card is the press, and a recorded plan is already going ahead.
                Way::NewJob if matches!(arrival.arrived, Arrived::Start | Arrived::Plan) => {}
                _ => self.acting.suggest(arrival, ruling),
            },
        }
    }

    /// `ruling` done: as it came back with its role On, or on the person's press on a suggestion.
    /// The arrival's Job is the one that has done less: a start has written nothing, since its
    /// first write waits on this ruling, and a plan is recorded before its building.
    pub fn acted(&self, arrival: &Arrival, ruling: &Ruling) {
        let session = arrival.session_id.as_str();
        if arrival.arrived.is_sourced() {
            self.acting.sourced(arrival, ruling);
            return;
        }
        if arrival.arrived.is_stall() {
            match ruling.way {
                Way::NewJob => self.acting.start_rest(session, &arrival.words),
                Way::Raise => self.acting.raise(session, &ruling.because),
                _ => {}
            }
            return;
        }
        if arrival.arrived.is_asked() {
            // A raise leaves the item where it is: on his Needs you list, as it was.
            if let (Way::Answer, Some(choice), Some(item)) = (ruling.way, &ruling.answer, questions::asked_item(arrival)) {
                self.acting.answer(session, item, choice, &ruling.because);
            }
            return;
        }
        match (ruling.way, ruling.other.as_deref()) {
            (Way::NewJob, _) => self.acting.go_ahead(session),
            (Way::ExistingJob, Some(other)) => self.acting.hand_over(session, arrival.group.as_deref(), other, &ruling.shared),
            (Way::Merge, Some(other)) => self.acting.merge(session, other, &ruling.shared),
            (Way::KeepInTouch, Some(other)) => self.acting.keep_in_touch(session, other, &ruling.shared),
            // A way between two that names no other was refused as it was read.
            (Way::ExistingJob | Way::Merge | Way::KeepInTouch, None) | (Way::Raise | Way::Answer | Way::Link | Way::File, _) => {}
        }
        if let (Some(first), Some(other)) = (ruling.first.as_deref(), ruling.other.as_deref()) {
            let held = if first == session { other } else { session };
            self.acting.hold(held, first, &ruling.shared);
        }
        if let Some(place) = &ruling.place {
            self.acting.place(session, place);
        }
    }
}

/// The role a way falls to. **A link writes nothing and a red main has Autopilot's own switch**, so
/// neither has one, and a raise of anything but a quiet Job is only its Log row.
fn role_of(arrived: Arrived, way: Way) -> Option<OperatorRole> {
    match way {
        Way::Answer => Some(OperatorRole::Answer),
        Way::File => Some(OperatorRole::File),
        Way::Link => None,
        _ if arrived == Arrived::RedMain => None,
        Way::ExistingJob | Way::KeepInTouch => Some(OperatorRole::Delegate),
        Way::NewJob | Way::Merge => Some(OperatorRole::Dispatch),
        Way::Raise => arrived.is_stall().then_some(OperatorRole::Dispatch),
    }
}

/// What a call is shown beside one item: the live Jobs near it, and the Jobs waiting to start it may
/// be placed among.
struct Asked {
    near: Vec<Near>,
    line: Vec<InLine>,
}

/// The Jobs waiting to start in a start's repository, other than its own. Only a start is placed.
fn line_of(arrival: &Arrival, waiting: &[InLine]) -> Vec<InLine> {
    if arrival.arrived != Arrived::Start {
        return Vec::new();
    }
    waiting.iter().filter(|one| one.manifest_id == arrival.manifest_id && one.session_id != arrival.session_id).cloned().collect()
}

fn take(from: &mut Vec<Held>, token: u64) -> Option<Held> {
    let at = from.iter().position(|one| one.token == token)?;
    Some(from.remove(at))
}

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}
