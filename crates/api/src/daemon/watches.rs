//! Watches: a command Fleet runs on a cadence for one Job, and the agent's weighing of a failed
//! reading. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own.

use std::future::Future;

use ipc::{
    SaveWatchRecipe, SessionId, StartWatch, StopWatch, TryWatch, WatchList, WatchRecipeList, WatchRecipeSaved,
    WatchSummary, WatchTried, WeighWatch,
};

use crate::daemon::Refusal;

pub trait Watches: Send + Sync + 'static {
    /// `start_watch` — check it against the recipe and the limits, and keep it for the Job
    /// `caller` places, else the one `start.session_id` names.
    fn start_watch(
        &self,
        caller: Option<crate::Caller>,
        start: StartWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `weigh_watch` — the agent's verdict on a failed reading. A placed Job weighs its own.
    fn weigh_watch(
        &self,
        caller: Option<crate::Caller>,
        weigh: WeighWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `stop_watch` — end it `stopped`. A placed Job stops its own; Bridge stops any.
    fn stop_watch(
        &self,
        caller: Option<crate::Caller>,
        stop: StopWatch,
    ) -> impl Future<Output = Result<WatchSummary, Refusal>> + Send;

    /// `list_watches` — the placed Job's watches, live and ended.
    fn list_watches(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<WatchList, Refusal>> + Send;

    /// `try_watch` — run a command or a recipe once, as a watch would, and keep nothing.
    fn try_watch(
        &self,
        caller: Option<crate::Caller>,
        tried: TryWatch,
    ) -> impl Future<Output = Result<WatchTried, Refusal>> + Send;

    /// `list_watch_recipes` — the recipes a start may name: shipped, the repository's and this Mac's.
    fn list_watch_recipes(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<WatchRecipeList, Refusal>> + Send;

    /// `save_watch_recipe` — keep a recipe, and its script, among this Mac's own.
    fn save_watch_recipe(
        &self,
        caller: Option<crate::Caller>,
        save: SaveWatchRecipe,
    ) -> impl Future<Output = Result<WatchRecipeSaved, Refusal>> + Send;
}
