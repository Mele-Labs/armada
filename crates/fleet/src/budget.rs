//! Racing a plain command's work against its own budget, spawned rather than
//! merely timed.
//!
//! **Split out of `commanding` at the 900-line refusal, `#897`.** [`budgeted`]
//! and [`budgeted_for`] are generic over the answer and reach for nothing
//! `commanding` owns, and [`CommandBudget`], the type they carry, lives here
//! with them.

use std::future::Future;
use std::time::Duration;

use ipc::JobId;

use crate::adrift::Adrift;

/// How long Fleet gives a plain command before answering
/// [`Adrift::CommandTimedOut`]. Paired with Bridge's `COMMAND_MS` — see
/// `timeouts.commandSeconds` in settings.json.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandBudget(Duration);

impl CommandBudget {
    pub fn of(budget: Duration) -> CommandBudget {
        CommandBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Race a plain command's work against [`CommandBudget`], spawned rather than
/// merely timed: a losing race stops waiting without cancelling a write
/// already in progress.
pub(crate) async fn budgeted<T>(
    budget: CommandBudget,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    let waited = budget.duration();
    // The door the request came through goes with the work onto its task, so
    // a move the command makes is signed for it. `crate::retro::signed`.
    let work = api::carrying(api::via(), work);
    match tokio::time::timeout(waited, tokio::spawn(work)).await {
        Ok(Ok(answered)) => answered,
        // Resumed rather than folded into a refusal a caller might retry: a
        // panic has already unwound past every lock it held.
        Ok(Err(panicked)) => std::panic::resume_unwind(panicked.into_panic()),
        Err(_elapsed) => Err(Adrift::CommandTimedOut { job: None, waited }),
    }
}

/// [`budgeted`], naming the Job a losing race's refusal is about.
pub(crate) async fn budgeted_for<T>(
    budget: CommandBudget,
    job: JobId,
    work: impl Future<Output = Result<T, Adrift>> + Send + 'static,
) -> Result<T, Adrift>
where
    T: Send + 'static,
{
    match budgeted(budget, work).await {
        Err(Adrift::CommandTimedOut { waited, .. }) => Err(Adrift::CommandTimedOut {
            job: Some(job.to_domain()),
            waited,
        }),
        answered => answered,
    }
}
