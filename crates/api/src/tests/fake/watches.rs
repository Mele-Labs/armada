//! Watches, as the fake answers them: nothing is run or kept, and a start is echoed as recorded.

use ipc::{
    Instant, SessionId, StartWatch, StopWatch, WatchList, WatchStarts, WatchState, WatchSummary,
    WeighWatch,
};

use super::FakeDaemon;
use crate::{Caller, Refusal, Watches};

/// The one watch id the fake answers for; any other is not there.
pub const THE_WATCH: &str = "w-1";

fn summary(session_id: SessionId, name: String, command: Vec<String>, recipe: Option<String>) -> WatchSummary {
    WatchSummary {
        id: THE_WATCH.to_string(),
        session_id,
        name,
        command,
        recipe,
        every_seconds: 60,
        gives_up_after_seconds: 7200,
        follows: None,
        environment: None,
        state: WatchState::Waiting,
        live: true,
        started_at: Instant::carried("2026-10-22T00:00:00.000Z"),
        last: None,
        readings: Vec::new(),
        set_aside: Vec::new(),
        verdicts: Vec::new(),
    }
}

fn the_watch(id: &str) -> Result<WatchSummary, Refusal> {
    match id == THE_WATCH {
        true => Ok(summary(SessionId::carried("session"), "QA deploy".to_string(), Vec::new(), None)),
        false => Err(Refusal::NotFound(ipc::WireError::raised(
            "fleet.no_such_watch",
            format!("No watch is {id}."),
            super::run_id(),
        ))),
    }
}

impl Watches for FakeDaemon {
    async fn start_watch(&self, _caller: Option<Caller>, start: StartWatch) -> Result<WatchSummary, Refusal> {
        let (command, recipe) = match start.starts {
            WatchStarts::Command { command, .. } => (command, None),
            WatchStarts::Recipe { recipe, .. } => (Vec::new(), Some(recipe)),
        };
        let session_id = start.session_id.unwrap_or_else(|| SessionId::carried("session"));
        Ok(summary(session_id, start.name, command, recipe))
    }

    async fn weigh_watch(&self, _caller: Option<Caller>, weigh: WeighWatch) -> Result<WatchSummary, Refusal> {
        the_watch(&weigh.watch)
    }

    async fn stop_watch(&self, _caller: Option<Caller>, stop: StopWatch) -> Result<WatchSummary, Refusal> {
        let mut stopped = the_watch(&stop.watch)?;
        stopped.state = WatchState::Stopped;
        stopped.live = false;
        Ok(stopped)
    }

    async fn list_watches(&self, _caller: Option<Caller>, session_id: Option<SessionId>) -> Result<WatchList, Refusal> {
        Ok(WatchList { session_id: session_id.unwrap_or_else(|| SessionId::carried("session")), watches: Vec::new() })
    }
}
