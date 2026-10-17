//! The Manifest's base, attached to the worktrees Fleet reads, and the base checkouts an older
//! Fleet left behind.
//!
//! Nothing makes a base checkout now: it was the tree a Job's *before* frames were photographed
//! from. What is on the disk is given back by [`Fleet::bases_gone_by`], on the same sweep that
//! reclaims a worktree.

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
    /// **Every reading of a Job's work is measured from the same commit, or
    /// two surfaces answer one question differently.** Without a base,
    /// `adapters` falls back to the main checkout's HEAD — a floating
    /// reference that says where a person is standing rather than what the Job
    /// branched from — and then to the branch tip, where the reading is only
    /// what is uncommitted. `crate::reviewing`'s diff and `kept_footprint` had
    /// this and the live footprint did not, so a Job's Produced chapter and
    /// its patch could be measured from two different commits on one worktree.
    ///
    /// `adapters` cannot read a Manifest — the layers forbid it — so this is
    /// where the two are held together.
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
