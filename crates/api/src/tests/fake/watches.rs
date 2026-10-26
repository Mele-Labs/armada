//! Watches, as the fake answers them: nothing is run or kept, and a start is echoed as recorded.

use ipc::{
    Instant, LoginShell, SaveWatchRecipe, SessionId, StartWatch, StopWatch, TryWatch, WatchList, WatchReading,
    WatchRecipeList, WatchRecipeListed, WatchRecipeSaved, WatchRecipeSource, WatchStarts, WatchState, WatchSummary,
    WatchTried, WatchTries, WeighWatch,
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
        in_terminal: None,
        regex: false,
        login_shell: None,
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
            WatchStarts::Terminal { .. } => (Vec::new(), None),
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

    async fn list_watches(
        &self,
        _caller: Option<Caller>,
        session_id: Option<SessionId>,
        _ended: bool,
    ) -> Result<WatchList, Refusal> {
        Ok(WatchList { session_id: session_id.unwrap_or_else(|| SessionId::carried("session")), watches: Vec::new() })
    }

    async fn try_watch(&self, _caller: Option<Caller>, tried: TryWatch) -> Result<WatchTried, Refusal> {
        let command = match tried.tries {
            WatchTries::Command { command } => command,
            WatchTries::Recipe { recipe, .. } => vec![recipe],
        };
        Ok(WatchTried {
            reading: WatchReading::broken(
                Instant::carried("2026-10-22T00:00:00.000Z"),
                String::from("Nothing runs in the fake."),
                None,
            ),
            command,
            cwd: String::from("/slot"),
            path: String::from("/usr/bin"),
            login_shell: LoginShell::FellBack { why: String::from("The fake reads no shell.") },
            scratch: String::from("/scratch"),
        })
    }

    async fn list_watch_recipes(
        &self,
        _caller: Option<Caller>,
        _session_id: Option<SessionId>,
    ) -> Result<WatchRecipeList, Refusal> {
        Ok(WatchRecipeList { recipes: Vec::new(), left_out: Vec::new() })
    }

    async fn save_watch_recipe(&self, _caller: Option<Caller>, save: SaveWatchRecipe) -> Result<WatchRecipeSaved, Refusal> {
        Ok(WatchRecipeSaved {
            recipe: WatchRecipeListed {
                name: save.name.clone(),
                values: Vec::new(),
                command: save.command,
                source: WatchRecipeSource::Machine,
                every_seconds: 60,
                gives_up_after_seconds: None,
                environment: save.environment,
            },
            file: format!("/machine/watches/{}.yml", save.name),
            script: None,
            replaced: false,
        })
    }
}
