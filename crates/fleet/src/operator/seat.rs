//! The Operator as a running Fleet holds it: the one seated at start, the ruling each start's
//! first write may wait on, the place each start takes as it joins Waiting to start, and the Jobs
//! kept in touch that moved since the last tick. In memory: a start ruled before a restart is in
//! the Log, its Job's first write no longer waits, and it joins the line last.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::watch;

use super::{held, Arrival, KeptStarted, Operator, Place, Ruled, Way};

/// What a Job kept in touch with another did, which that other is told of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    Plan,
    Pushed,
}

/// A start's ruling, `None` while the call is out.
pub type StartRuling = watch::Receiver<Option<Ruled>>;

#[derive(Default)]
pub struct Seat {
    operator: OnceLock<Arc<Operator>>,
    /// The call the seated Operator makes, where it is the kept one: started and stopped with the switch.
    kept: OnceLock<Arc<KeptStarted>>,
    starts: Arc<Mutex<HashMap<String, StartRuling>>>,
    moved: Mutex<BTreeMap<String, Moved>>,
    /// The Jobs a read found gone quiet, and those a call is out on.
    pub stalls: super::Stalls,
    places: Mutex<HashMap<String, Place>>,
    /// The questions and cards a read found waiting, and those a call is out on.
    pub questions: super::answering::Questions,
    /// Reports, issues, review requests and Later items waiting to be handed, and those a call is out on.
    pub sources: super::Sources,
    /// The standing orders, as the store keeps them: read where no lock on the store may be taken.
    orders: Mutex<super::Orders>,
}

impl Seat {
    /// Seat `operator`, once: `false` where one already sits.
    pub fn seat(&self, operator: Arc<Operator>) -> bool {
        self.operator.set(operator).is_ok()
    }

    pub fn operator(&self) -> Option<Arc<Operator>> {
        self.operator.get().cloned()
    }

    pub fn orders(&self) -> super::Orders {
        held(&self.orders).clone()
    }

    pub fn orders_are(&self, orders: super::Orders) {
        *held(&self.orders) = orders;
    }

    pub fn keep(&self, call: Arc<KeptStarted>) {
        let _ = self.kept.set(call);
    }

    pub fn kept(&self) -> Option<Arc<KeptStarted>> {
        self.kept.get().cloned()
    }

    /// Rule on `arrival` in the background. Nothing waits on it: what it rules is logged and acted on.
    pub fn hand(&self, arrival: Arrival, within: Duration) {
        let Some(operator) = self.operator() else { return };
        tokio::spawn(async move {
            operator.ruled(arrival, within).await;
        });
    }

    /// Rule on a Job's start in the background, and keep its ruling for the Job's first write. A
    /// ruling that leaves the Job working is let go once it lands: only one that ends the Job is
    /// still read by a write.
    pub fn hand_start(&self, arrival: Arrival, within: Duration) {
        let Some(operator) = self.operator() else { return };
        let id = arrival.session_id.clone();
        let (tell, told) = watch::channel(None);
        held(&self.starts).insert(id.clone(), told);
        let starts = Arc::clone(&self.starts);
        tokio::spawn(async move {
            let ruled = operator.ruled(arrival, within).await;
            let ends = matches!(&ruled, Ruled::Ruled(ruling) if matches!(ruling.way, Way::ExistingJob | Way::Merge));
            let _ = tell.send(Some(ruled));
            if !ends {
                held(&starts).remove(&id);
            }
        });
    }

    /// The ruling `session`'s start is waiting on, or has, where one still matters.
    pub fn start_ruling(&self, session: &str) -> Option<StartRuling> {
        held(&self.starts).get(session).cloned()
    }

    /// `session` moved, and the Jobs kept in touch with it are owed word of it. A push outweighs a
    /// plan within one tick.
    pub fn moved(&self, session: &str, how: Moved) {
        let mut moved = held(&self.moved);
        let was = moved.get(session).copied();
        moved.insert(session.to_string(), if was == Some(Moved::Pushed) { Moved::Pushed } else { how });
    }

    /// Keep the place `session` takes when it joins Waiting to start.
    pub fn place(&self, session: &str, place: Place) {
        held(&self.places).insert(session.to_string(), place);
    }

    /// The place `session` was ruled, taken.
    pub fn take_place(&self, session: &str) -> Option<Place> {
        held(&self.places).remove(session)
    }

    /// Every move owed word, taken.
    pub fn take_moved(&self) -> Vec<(String, Moved)> {
        std::mem::take(&mut *held(&self.moved)).into_iter().collect()
    }
}
