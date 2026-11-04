//! The Operator's call: one question in, one answer out, and no tools.
//!
//! **Kept started and waiting.** The proposer's 5 to 7.5 seconds were mostly the CLI starting, so
//! [`KeptStarted`] starts the next process while a call is out, and an arrival pays only for the
//! model. The waiting process holds its question back until one comes; it is the same process
//! [`crate::judging::said`] would start, with no [`adapter_traits::Reading`], so it can read nothing.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use adapter_traits::{Ask, Environment, Model, ModelClient};
use tokio::process::Child;

use crate::judging::{answered, waiting, CallFailed, JudgeBudget};

/// What a call answers, in time.
pub type Answering<'a> = Pin<Box<dyn Future<Output = Result<String, CallFailed>> + Send + 'a>>;

/// One fresh call to the Operator model. **It holds no tools**, so it can rule and nothing more.
pub trait Calling: Send + Sync {
    fn ask(&self, question: String) -> Answering<'_>;
}

/// What the waiting process is rendered with. Never sent: the real question goes on its stdin.
const NOT_YET: &str = "The Operator's question is written to this call when it has one.";

/// The call through a [`ModelClient`], with one process started and waiting.
pub struct KeptStarted {
    client: Arc<dyn ModelClient + Send + Sync>,
    /// The Operator model as the setting reads now, or `None` where it names nothing.
    model: Box<dyn Fn() -> Option<Model> + Send + Sync>,
    environment: Environment,
    budget: JudgeBudget,
    waiting: tokio::sync::Mutex<Option<(Model, Child)>>,
}

impl KeptStarted {
    pub fn new(
        client: Arc<dyn ModelClient + Send + Sync>,
        model: impl Fn() -> Option<Model> + Send + Sync + 'static,
        environment: Environment,
        budget: JudgeBudget,
    ) -> KeptStarted {
        KeptStarted { client, model: Box::new(model), environment, budget, waiting: tokio::sync::Mutex::new(None) }
    }

    /// Start a process to wait, where none does: before the first arrival, so it pays no start.
    pub async fn ready(&self) -> Result<(), CallFailed> {
        let model = (self.model)().ok_or(CallFailed::NothingToAsk)?;
        let mut held = self.waiting.lock().await;
        if let Some(kept) = held.take().and_then(|kept| still_waiting(kept, &model)) {
            *held = Some(kept);
            return Ok(());
        }
        *held = Some((model.clone(), self.start(&model)?));
        Ok(())
    }

    fn start(&self, model: &Model) -> Result<Child, CallFailed> {
        let ask = Ask::put(model.clone(), NOT_YET, self.environment.clone()).map_err(|_| CallFailed::NothingToAsk)?;
        waiting(&self.client.render(&ask))
    }
}

/// The kept process, where it is on `model` and has not exited.
fn still_waiting((on, mut child): (Model, Child), model: &Model) -> Option<(Model, Child)> {
    (on == *model && matches!(child.try_wait(), Ok(None))).then_some((on, child))
}

impl Calling for KeptStarted {
    fn ask(&self, question: String) -> Answering<'_> {
        Box::pin(async move {
            let model = (self.model)().ok_or(CallFailed::NothingToAsk)?;
            let mut held = self.waiting.lock().await;
            let child = match held.take().and_then(|kept| still_waiting(kept, &model)) {
                Some((_, child)) => child,
                None => self.start(&model)?,
            };
            // The next one starts while this one is out. One that will not start is tried again
            // at the next call, which then pays the start itself.
            *held = self.start(&model).ok().map(|next| (model.clone(), next));
            drop(held);
            answered(child, &question, self.budget).await
        })
    }
}
