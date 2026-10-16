//! A Session's running Checks, held while they run: what each has printed, and a stop a person asked
//! for. `ipc::session_checks`.
//!
//! **Held in memory and never written down.** `armada check` is the only process that has the
//! output, and tells Fleet in `chunk` calls; the store keeps the log whole once the run ends. A
//! Fleet that restarts mid-run loses the live part, and the run's own `end` still lands.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ipc::SessionCheckStop;

/// How much of a run's output is held live. Past it the live view stops and `end` carries the log.
const HELD: usize = 8 * 1024 * 1024;

#[derive(Default)]
struct State {
    log: Vec<u8>,
    stop: Option<SessionCheckStop>,
    ended: bool,
}

/// One run, shared by the calls that feed it and the sockets that read it.
#[derive(Default)]
pub(crate) struct Run {
    state: Mutex<State>,
}

/// Every Session run that is out, by run id.
#[derive(Default)]
pub(crate) struct SessionCheckLive {
    runs: Mutex<HashMap<i64, Arc<Run>>>,
}

impl SessionCheckLive {
    /// The run's entry, made if it is not there: a Fleet restarted mid-run still hears the rest.
    pub(crate) fn run(&self, id: i64) -> Arc<Run> {
        Arc::clone(self.runs.lock().expect("not poisoned").entry(id).or_default())
    }

    pub(crate) fn find(&self, id: i64) -> Option<Arc<Run>> {
        self.runs.lock().expect("not poisoned").get(&id).cloned()
    }

    /// The run ended: readers see it as final, and the entry goes.
    pub(crate) fn ended(&self, id: i64) {
        if let Some(run) = self.runs.lock().expect("not poisoned").remove(&id) {
            run.state.lock().expect("not poisoned").ended = true;
        }
    }
}

impl Run {
    /// Add what printed, and answer the stop standing.
    pub(crate) fn heard(&self, text: &str) -> Option<SessionCheckStop> {
        let mut state = self.state.lock().expect("not poisoned");
        if state.log.len() + text.len() <= HELD {
            state.log.extend_from_slice(text.as_bytes());
        }
        state.stop
    }

    /// A person asked for `stop`. **Force outranks graceful and nothing goes back.** `None` where the
    /// run has already ended.
    pub(crate) fn asked(&self, stop: SessionCheckStop) -> Option<SessionCheckStop> {
        let mut state = self.state.lock().expect("not poisoned");
        if state.ended {
            return None;
        }
        let standing = state.stop.map_or(stop, |held| held.max(stop));
        state.stop = Some(standing);
        Some(standing)
    }
}

/// A live run's log, read as `api::Follow` reads any: from a byte, whole lines until the end.
pub(crate) struct Live(pub(crate) Arc<Run>);

impl api::Follow for Live {
    fn read(&self, from: u64, to_the_end: bool) -> api::Followed {
        let state = self.0.state.lock().expect("not poisoned");
        let at = usize::try_from(from).unwrap_or(usize::MAX).min(state.log.len());
        crate::following::windowed(&state.log[at..], from, to_the_end)
    }

    fn writing(&self) -> bool {
        !self.0.state.lock().expect("not poisoned").ended
    }
}

/// A run's kept log, for one that is over: read whole, never writing.
pub(crate) struct Kept(pub(crate) String);

impl api::Follow for Kept {
    fn read(&self, from: u64, to_the_end: bool) -> api::Followed {
        let at = usize::try_from(from).unwrap_or(usize::MAX).min(self.0.len());
        crate::following::windowed(&self.0.as_bytes()[at..], from, to_the_end)
    }

    fn writing(&self) -> bool {
        false
    }
}
