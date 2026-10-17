//! The fetch every catch-up opens with.
//!
//! A catch-up merges the remote's copy of the base, never the local branch of
//! the same name, which is rarely fast-forwarded (Job 14, 12 Oct 2026).
//! `adapters` reads whatever the last fetch left and does not fetch, so this is
//! where a failure is seen.
//!
//! **A fetch that fails stops the catch-up.** Merging the local branch, or the
//! copy of the remote's an earlier fetch left, would report the branch up to
//! date against a base the forge no longer has: the exact failure this exists
//! for, and silent. The Job's log says which remote, which base and what git
//! said. The cost is that a Job whose remote is unreachable cannot start a
//! step — but its push would be refused for the same reason, and it is
//! stopped here with the reason instead of at a pull request that conflicts.

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
}
