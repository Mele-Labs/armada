//! The notices a Fleet mod's rule says to batch: held for a Job, and sent as one message when the
//! window the first of them opened ends. `docs/concepts/mods.md`, *fleet.toml*.
//!
//! **In memory and never written down**: a Fleet that restarts inside a window loses what was held,
//! which is a notice the Job can do without, and a table of them would outlive the rule that held them.

use std::collections::BTreeMap;
use std::sync::Mutex;

use config::settings::{self as keys};

use super::*;

/// What is held for each Job and notice word, until the window ends.
#[derive(Default)]
pub(crate) struct NoticeBatches(Mutex<BTreeMap<(String, &'static str), Held>>);

struct Held {
    texts: Vec<String>,
    news: bool,
}

impl NoticeBatches {
    /// Hold `text`. **Whether it opened the batch**, which is the caller's to end after the window.
    fn hold(&self, session: &str, notice: ipc::Notice, text: &str, news: bool) -> bool {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let key = (session.to_string(), notice.word());
        let opened = !held.contains_key(&key);
        let one = held.entry(key).or_insert(Held { texts: Vec::new(), news });
        let text = text.trim();
        if !one.texts.iter().any(|kept| kept == text) {
            one.texts.push(text.to_string());
        }
        opened
    }

    fn take(&self, session: &str, notice: ipc::Notice) -> Option<Held> {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.remove(&(session.to_string(), notice.word()))
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Hold `text` for `session`. The first held for a Job and word opens the window; the one message
    /// goes when it ends.
    pub(super) fn batch_held(self: &Arc<Self>, session: &str, text: &str, news: bool, notice: ipc::Notice) {
        if !self.notice_batches().hold(session, notice, text, news) {
            return;
        }
        let window = self.machine_settings().now().get(keys::NOTICE_BATCH_SECONDS);
        let (fleet, session) = (Arc::clone(self), session.to_string());
        tokio::spawn(async move {
            tokio::time::sleep(window).await;
            fleet.batch_sent(&session, notice).await;
        });
    }

    /// Send what is held for `session` under `notice` as one message. **Not asked of the rules again**:
    /// the rule that held it already spoke. A Job that cannot be told now (ended, or a terminal one
    /// that is not listening) loses what was held, since the caller was told it was handled.
    pub(super) async fn batch_sent(self: &Arc<Self>, session: &str, notice: ipc::Notice) {
        let Some(held) = self.notice_batches().take(session, notice) else { return };
        let _ = self.fleet_word(session, &held.texts.join("\n\n"), held.news, Some(Written::Noticed(notice))).await;
    }
}
