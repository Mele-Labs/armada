//! The Operator as a running Fleet holds it: the one seated at start, the ruling each start's
//! first write may wait on, and the Jobs kept in touch that moved since the last tick. In memory:
//! a start ruled before a restart is in the Log, and its Job's first write no longer waits.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::watch;

use super::{held, Arrival, Operator, Ruled, Way};

/// What a Job kept in touch with another did, which that other is told of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Moved {
    Plan,
    Pushed,
}

/// A start's ruling, `None` while the call is out.
pub(crate) type StartRuling = watch::Receiver<Option<Ruled>>;

#[derive(Default)]
pub(crate) struct Seat {
    operator: OnceLock<Arc<Operator>>,
    starts: Arc<Mutex<HashMap<String, StartRuling>>>,
    moved: Mutex<BTreeMap<String, Moved>>,
}

impl Seat {
    /// Seat `operator`, once: `false` where one already sits.
    pub(crate) fn seat(&self, operator: Arc<Operator>) -> bool {
        self.operator.set(operator).is_ok()
    }

    fn operator(&self) -> Option<Arc<Operator>> {
        self.operator.get().cloned()
    }

    /// Rule on `arrival` in the background. Nothing waits on it: what it rules is logged and acted on.
    pub(crate) fn hand(&self, arrival: Arrival, within: Duration) {
        let Some(operator) = self.operator() else { return };
        tokio::spawn(async move {
            operator.ruled(arrival, within).await;
        });
    }

    /// Rule on a Job's start in the background, and keep its ruling for the Job's first write. A
    /// ruling that leaves the Job working is let go once it lands: only one that ends the Job is
    /// still read by a write.
    pub(crate) fn hand_start(&self, arrival: Arrival, within: Duration) {
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
    pub(crate) fn start_ruling(&self, session: &str) -> Option<StartRuling> {
        held(&self.starts).get(session).cloned()
    }

    /// `session` moved, and the Jobs kept in touch with it are owed word of it. A push outweighs a
    /// plan within one tick.
    pub(crate) fn moved(&self, session: &str, how: Moved) {
        let mut moved = held(&self.moved);
        let was = moved.get(session).copied();
        moved.insert(session.to_string(), if was == Some(Moved::Pushed) { Moved::Pushed } else { how });
    }

    /// Every move owed word, taken.
    pub(crate) fn take_moved(&self) -> Vec<(String, Moved)> {
        std::mem::take(&mut *held(&self.moved)).into_iter().collect()
    }
}
