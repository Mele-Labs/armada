//! The Operator: one per machine, deciding who does each piece of work that arrives. `armada-ehhu`,
//! and the design page *Helm becomes the Operator*, revision 41.
//!
//! **Fleet detects, a Policy decides, the record remembers.** Fleet hands over an [`Arrival`] with
//! the ways its rules leave open; one fresh [`Calling`] with no tools names one of them; the
//! ruling, or why there was none, goes in the Log through the [`Desk`], signed by the Operator.
//!
//! **One call out at a time on this machine.** What arrives while a call is out is ruled together
//! in the next call, so three Jobs dispatched at once are seen side by side.
//!
//! **Fail open.** A call that fails, answers what cannot be read, or rules what Fleet did not offer
//! passes the item on untouched, and so does one with no ruling by the time it could wait
//! ([`FIRST_WRITE_WAIT`], [`NEEDS_YOU_WAIT`]); so does every item once the day's calls reach the
//! ceiling. A ruling that comes back after its item passed on is dropped.

mod arriving;
mod asking;
mod calling;
mod desk;
mod ruling;

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use core_model::{Actor, Timestamp};
use tokio::sync::oneshot;

pub use arriving::{Arrival, Arrived, LiveJob, Source};
pub use calling::{Answering, Calling, KeptStarted};
pub use desk::FleetDesk;
pub use ruling::{Outcome, PassedOn, Ruled, Ruling, Way};

/// How long a Job's first write waits for the ruling on its start.
pub const FIRST_WRITE_WAIT: Duration = Duration::from_secs(20);
/// How long a Needs you item waits for its ruling before it pulses.
pub const NEEDS_YOU_WAIT: Duration = Duration::from_secs(60);

/// What a [`Desk`] answers, in time.
pub type Done<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Fleet, as far as the Operator reaches it.
pub trait Desk: Send + Sync {
    fn now(&self) -> Timestamp;
    /// The most calls a day, as the setting reads now.
    fn daily_calls(&self) -> u32;
    /// The Jobs live now, which a call may name.
    fn live(&self) -> Done<'_, Vec<LiveJob>>;
    /// How many calls the Log names from `since`. **A count it cannot read is the ceiling
    /// reached**, so a broken count makes no calls rather than unbounded ones.
    fn calls_since(&self, since: Timestamp) -> Done<'_, u32>;
    /// Keep one row of the Log.
    fn keep(&self, row: Logged) -> Done<'_, ()>;
}

/// What Fleet does to Jobs on a ruling. **Not called yet**: the group *Who does the work* acts on
/// rulings, and until it lands every ruling is logged and nothing more.
pub trait Acting: Send + Sync {
    /// `ending` stops, and its brief, plan and branch go to `into` as one message.
    fn merge(&self, ending: &str, into: &str, shared: &[String]);
    /// Both carry on, each told the other's plan group and the paths they share.
    fn keep_in_touch(&self, one: &str, other: &str, shared: &[String]);
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
    pub at: Timestamp,
}

impl Logged {
    fn of(arrival: &Arrival, ruled: &Ruled, call: Option<String>, at: Timestamp) -> Logged {
        let (outcome, other, shared, because) = match ruled {
            Ruled::Ruled(ruling) => (Outcome::Ruled(ruling.way), ruling.other.clone(), ruling.shared.clone(), ruling.because.clone()),
            Ruled::PassedOn(why) => (Outcome::PassedOn, None, Vec::new(), why.to_string()),
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
            at,
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
            at: self.at.as_str().to_string(),
        }
    }
}

/// One item waiting on a ruling, and where its ruling goes.
struct Held {
    token: u64,
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
    queue: Mutex<Queue>,
}

impl Operator {
    pub fn new(call: Arc<dyn Calling>, desk: Arc<dyn Desk>) -> Arc<Operator> {
        Arc::new(Operator { call, desk, queue: Mutex::new(Queue::default()) })
    }

    /// Rule on `arrival`, waiting at most `within`. **Never fails**: an item with no ruling in
    /// time is passed on, and the Log says why.
    pub async fn ruled(self: &Arc<Self>, arrival: Arrival, within: Duration) -> Ruled {
        let (tell, mut told) = oneshot::channel();
        let (token, start) = {
            let mut queue = held(&self.queue);
            let token = queue.next;
            queue.next += 1;
            queue.pending.push(Held { token, arrival, tell });
            let start = !queue.running;
            queue.running = true;
            (token, start)
        };
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
                self.desk.keep(Logged::of(&held.arrival, &ruled, call, self.desk.now())).await;
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
            let (asked, settled) = {
                let mut queue = held(&self.queue);
                if queue.pending.is_empty() {
                    queue.running = false;
                    return;
                }
                let (asked, settled): (Vec<Held>, Vec<Held>) =
                    std::mem::take(&mut queue.pending).into_iter().partition(|one| one.arrival.ways.len() > 1);
                let items: Vec<(u64, Arrival)> = asked.iter().map(|one| (one.token, one.arrival.clone())).collect();
                queue.out = asked;
                (items, settled)
            };
            for one in settled {
                let ruled = match one.arrival.ways.first() {
                    Some(way) => Ruled::Ruled(Ruling { way: *way, other: None, shared: Vec::new(), because: "it is the only way Fleet's rules leave it".into() }),
                    None => Ruled::PassedOn(PassedOn::Refused("Fleet's rules leave it no way".into())),
                };
                self.settle(one, ruled, None).await;
            }
            if asked.is_empty() {
                continue;
            }
            let now = self.desk.now();
            let ceiling = self.desk.daily_calls();
            if self.desk.calls_since(day_of(&now)).await >= ceiling {
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
    async fn call_out(&self, asked: Vec<(u64, Arrival)>, now: Timestamp) {
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
        let at: BTreeMap<u64, usize> = asked.iter().enumerate().map(|(at, (token, _))| (*token, at)).collect();
        let items: Vec<Arrival> = asked.into_iter().map(|(_, arrival)| arrival).collect();
        let live = self.desk.live().await;
        let rulings = match self.call.ask(asking::question(&items, &live)).await {
            Ok(answer) => ruling::read_rulings(&answer, &items, &live),
            Err(failed) => vec![Ruled::PassedOn(PassedOn::Failed(failed.in_a_line("Operator"))); items.len()],
        };
        let still = {
            let mut queue = held(&self.queue);
            queue.call = None;
            std::mem::take(&mut queue.out)
        };
        for one in still {
            let ruled = rulings[at[&one.token]].clone();
            self.settle(one, ruled, Some(call.clone())).await;
        }
    }

    async fn settle(&self, one: Held, ruled: Ruled, call: Option<String>) {
        self.desk.keep(Logged::of(&one.arrival, &ruled, call, self.desk.now())).await;
        let _ = one.tell.send(ruled);
    }
}

/// The start of `now`'s UTC day: the ceiling counts from it.
fn day_of(now: &Timestamp) -> Timestamp {
    let date: String = now.as_str().chars().take(10).collect();
    Timestamp::from_rfc3339(format!("{date}T00:00:00.000Z"))
}

fn take(from: &mut Vec<Held>, token: u64) -> Option<Held> {
    let at = from.iter().position(|one| one.token == token)?;
    Some(from.remove(at))
}

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}
