//! Fleet's local language model, loaded once and shared by every reader that needs one.
//!
//! **Fetched and loaded after start, and only while a reader in force reads with it.** With none,
//! Fleet fetches and holds nothing. Until the model is on disk and loaded, each reader falls back to
//! the rules it had before the model, so a machine that is offline, or a fetch that failed, reads as
//! Fleet always did. A failure is said once and tried again later.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Delivery, Embedder, Vcs, WorkProduct};
use tokio::task::JoinHandle;

use crate::daemon::Fleet;

/// How long after a failed fetch or load Fleet tries again.
pub const RETRY_EVERY: Duration = Duration::from_secs(30 * 60);

/// The model once it is loaded. Set once, and never unloaded while Fleet runs.
#[derive(Default)]
pub struct LanguageModel {
    loaded: OnceLock<Arc<dyn Embedder>>,
}

impl LanguageModel {
    /// The model, once loaded.
    pub(crate) fn embedder(&self) -> Option<Arc<dyn Embedder>> {
        self.loaded.get().cloned()
    }

    /// Put the loaded model in force. A second call changes nothing: the first model stays.
    pub fn loaded(&self, model: Arc<dyn Embedder>) {
        let _ = self.loaded.set(model);
    }
}

/// Whether a reader in force reads with the model. The one place a reader that needs it is named.
pub(crate) fn wanted() -> bool {
    crate::session_host::asks_model::SHIPPED.reads_model()
}

/// Fetch and load the model under `models` on a blocking thread, until it loads. `say` is handed the
/// load, and the first failure only. Nothing is started, and `None` comes back, while no reader in
/// force reads with the model.
pub fn keep_loading<H, V, W>(
    fleet: Arc<Fleet<H, V, W>>,
    models: PathBuf,
    load: fn(&Path) -> Result<Arc<dyn Embedder>, String>,
    say: impl Fn(String) + Send + 'static,
) -> Option<JoinHandle<()>>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    if !wanted() {
        return None;
    }
    Some(tokio::spawn(async move {
        let mut told = false;
        loop {
            let from = models.clone();
            let started = Instant::now();
            match tokio::task::spawn_blocking(move || load(&from)).await {
                Ok(Ok(model)) => {
                    say(format!("language model {} loaded in {} ms", model.model(), started.elapsed().as_millis()));
                    fleet.language_model().loaded(model);
                    return;
                }
                Ok(Err(why)) if !told => {
                    say(format!("the language model is not loaded, so replies are read by rules alone until it is: {why}"));
                    told = true;
                }
                Err(why) if !told => {
                    say(format!("the language model's load stopped: {why}"));
                    told = true;
                }
                _ => {}
            }
            tokio::time::sleep(RETRY_EVERY).await;
        }
    }))
}
