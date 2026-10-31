//! Watches: a command Fleet runs on a cadence for one Job, and the agent's weighing of a failed
//! reading. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own.

use std::future::Future;

use ipc::{
    DeleteWatch, DismissWait, ReadWatchLog, SaveWatchRecipe, SessionId, SessionRecord, StartWatch, StopWatch, TryWatch, WatchList, WatchLog, WatchRecipeList,
    WatchRecipeSaved, WatchSummary, WatchTried, WeighWatch,
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

    /// `dismiss_wait` — stop listing one of a Job's own pull requests as holding it open, leaving the
    /// pull request alone. Bridge only: it answers the Job's row.
    fn dismiss_wait(
        &self,
        dismiss: DismissWait,
    ) -> impl Future<Output = Result<SessionRecord, Refusal>> + Send;

    /// `delete_watch` — forget an ended watch and all kept of it, answering the Job's watches
    /// after, live and ended. A placed Job deletes its own; Bridge deletes any.
    fn delete_watch(
        &self,
        caller: Option<crate::Caller>,
        delete: DeleteWatch,
    ) -> impl Future<Output = Result<WatchList, Refusal>> + Send;

    /// `list_watches` — the placed Job's live watches, and with `ended` its ended ones too.
    fn list_watches(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
        ended: bool,
    ) -> impl Future<Output = Result<WatchList, Refusal>> + Send;

    /// `read_watch_log` — the log a reading of a watch kept, by the path the reading carries. A
    /// placed Job reads its own; Bridge reads any.
    fn read_watch_log(
        &self,
        caller: Option<crate::Caller>,
        read: ReadWatchLog,
    ) -> impl Future<Output = Result<WatchLog, Refusal>> + Send;

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
