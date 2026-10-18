//! Subagents a replaced process took down with it. `docs/concepts/session.md`,
//! *What a restart does to a session*.
//!
//! **A subagent lives in its agent's process.** A keeper carries the process across a Fleet
//! restart, but a process Fleet ends (a tune, a quiet spell, the move into a slot) or loses
//! (the keeper gone) takes every subagent still working with it, and the one that replaces it
//! has never heard of them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, Holder};

use super::State;
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
    /// The subagents the session's last process had not finished, one line each. **Called
    /// where a new process starts for a session that has run**, so nothing it names can still be
    /// working. Their ledger rows are given back: they were let go before they were used.
    pub(crate) async fn cut_off_subagents(&self, id: &str) -> Vec<String> {
        let holder = Holder::session(id);
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let held = store.attachments_of(&holder).unwrap_or_default();
        let mut cut = Vec::new();
        for one in held
            .iter()
            .filter(|one| one.kind == "subagent" && one.state == AttachmentState::Standing)
            .filter(|one| !adapters::terminal_thread::subagent_ended(&self.host().home, id, &one.target))
        {
            let _ = store.settle(
                &holder,
                &one.kind,
                &one.manifest_id,
                &one.target,
                AttachmentState::GivenBack,
                &at,
            );
            let said = |key: &str| one.detail.get(key).map(String::as_str).unwrap_or_default();
            cut.push(format!("{} \"{}\" ({})", said("type"), said("description"), one.target));
        }
        cut
    }
}

/// What the retro keeps of a restart that cut subagents off.
pub(super) fn cut_off_said(cut: &[String]) -> Option<String> {
    (!cut.is_empty()).then(|| format!("cut off: {}", cut.join("; ")))
}

/// `turn`, with what the last restart cut off ahead of it, once.
pub(super) fn told(state: &mut State, turn: String) -> String {
    if state.cut_off.is_empty() {
        return turn;
    }
    let cut = std::mem::take(&mut state.cut_off);
    format!(
        "This job's process was replaced while these subagents or background tasks were running. They stopped \
         and their work is lost: {}. Start again any that is still needed.\n\n{turn}",
        cut.join("; ")
    )
}
