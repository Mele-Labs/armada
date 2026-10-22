//! The follow-ups an agent keeps for itself, kept across its replies until they are settled.
//! `extracting` reads them off the close of one reply; here each is held a Session at a time, so the
//! next reply does not wipe it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

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
    /// Keep `sentences`, said when a reply ended at `said_at`. A sentence already kept, open or
    /// settled, is not kept again. Whether any was new.
    pub(crate) async fn later_kept(&self, id: &str, sentences: &[String], said_at: &str) -> bool {
        let mut store = self.store().lock().await;
        sentences.iter().fold(false, |any, sentence| store.keep_later(id, sentence, said_at).unwrap_or(false) || any)
    }

    /// The follow-ups of the Session's last reply, once the model has read it. Nothing while the
    /// readers are not in force, the reply is not read yet, or the Session has ended.
    pub(crate) async fn later_read(&self, id: &str) {
        if !super::extracting::IN_FORCE {
            return;
        }
        let Some(model) = self.language_model().model() else {
            return;
        };
        let reply = {
            let store = self.store().lock().await;
            let live = store.session(id).ok().flatten().is_some_and(|session| session.state != store::SessionState::Ended);
            store.turn_end(id).ok().flatten().filter(|_| live)
        };
        let Some((at, said)) = reply else {
            return;
        };
        if let Some(read) = self.extract_reads().read_of(super::extracting::read, model, id, &said) {
            self.later_kept(id, &read.later, &at).await;
        }
    }
}
