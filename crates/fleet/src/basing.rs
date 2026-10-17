//! The Manifest's base, attached to the worktrees Fleet reads.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};

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
    /// The Manifest's base, attached to a worktree Fleet is about to read.
    ///
    /// **Every reading of a Session's work is measured from the same commit, or
    /// two surfaces answer one question differently.** Without a base,
    /// `adapters` falls back to the main checkout's HEAD — a floating
    /// reference that says where a person is standing rather than what the
    /// work branched from — and then to the branch tip, where the reading is only
    /// what is uncommitted.
    ///
    /// `adapters` cannot read a Manifest — the layers forbid it — so this is
    /// where the base is attached.
    pub(crate) fn based(
        &self,
        served: &crate::repositories::Served,
        worktree: Worktree,
    ) -> Worktree {
        match served.manifest().base() {
            Some(base) => worktree.from_base(base),
            None => worktree,
        }
    }

}
