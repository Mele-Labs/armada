//! What the forge-reading loops keep between their passes.
//!
//! [`Sweep`] is one struct behind one lock, so the pull-request notices, main's
//! hub, the build notices and the catch-up each find their own cursor in the
//! same place. Nothing here is written down: what each loop decides is
//! re-derived from the forge and the store every time.

use std::collections::BTreeMap;
use std::time::Duration;

use core_model::{Envelope, JobId, Timestamp};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

/// How often the forge is asked about one pull request.
///
/// A newtype rather than a bare `Duration`, like [`Polling`](crate::Polling), so the value cannot
/// be handed to something measuring a different quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noticing(Duration);

impl Noticing {
    pub const fn every(interval: Duration) -> Noticing {
        Noticing(interval)
    }

    pub const fn interval(&self) -> Duration {
        self.0
    }
}

/// What the forge-reading loops last saw, and where each one stands.
///
/// **In memory and never written down.** A cursor that outlived the process
/// would name a position in a list that has since changed, and losing it costs
/// one restart of a rotation over a set that is small by construction.
#[derive(Clone, Debug, Default)]
pub(crate) struct Sweep {
    /// When main's CI was last read, and which served repository is next. Their
    /// own, for `crate::issue_noticing`'s reason: `crate::main_ci` reads a
    /// repository and this rotation reads a pull request.
    pub(crate) main_last: Option<Timestamp>,
    /// When pull requests were last read to tell their owners, `crate::pull_notices`.
    pub(crate) pulls_told_last: Option<Timestamp>,
    /// The login the forge's command line is signed in as, once it has answered. `crate::my_pulls`.
    pub(crate) local_login: Option<String>,
    /// When the build on file was last read to tell sessions, and the commit this process has
    /// told every session of. `crate::build_notices`.
    pub(crate) builds_told_last: Option<Timestamp>,
    pub(crate) build_told: Option<String>,
    pub(crate) main_next: usize,
    /// How the forge answered each repository's last visit, by root. `crate::main_hub`.
    pub(crate) forge_visits: BTreeMap<String, ForgeVisit>,
    /// The commits whose merging pull request was asked about, as `repository@commit`.
    pub(crate) culprit_asked: std::collections::BTreeSet<String>,
    /// Each repository's open pull requests as last listed, by root. `crate::main_hub`.
    pub(crate) pulls: BTreeMap<String, Vec<crate::main_hub::OpenPulled>>,
    /// Each repository's merge queue as last read, by root. `crate::main_hub`.
    pub(crate) queues: BTreeMap<String, Vec<adapter_traits::QueueEntry>>,
    /// What `needs` was last put on each open pull request's commit, and the
    /// ones the forge refused. `crate::needing::status`.
    pub(crate) needs_published: crate::needing::status::Published,
    pub(crate) needs_refused: std::collections::BTreeSet<String>,
    /// Each repository's newest merged pull requests as last listed, by root.
    pub(crate) merged: BTreeMap<String, Vec<crate::main_hub::MergedPulled>>,
    /// The CI run on main for each of those merge commits, by root and commit.
    /// A settled one is kept and never asked again.
    pub(crate) main_runs: BTreeMap<String, BTreeMap<String, crate::main_hub::MergedRun>>,
    /// Another machine's claim on each repository's red, as last read from the
    /// store, by root. `crate::red_claim`. Never holds this machine's own.
    pub(crate) elsewhere: BTreeMap<String, adapter_traits::Holder>,
    /// Each served checkout main moved past and that could not be caught up,
    /// by root, and why. `crate::catching_up`.
    pub(crate) held_back: BTreeMap<String, adapter_traits::HeldBack>,
    /// The pull request each held checkout's uncommitted files went up as, by
    /// root, on a person's press. Kept with the hold and dropped with it, so a
    /// later hold starts without one. `crate::catching_up`.
    pub(crate) checkout_sent: BTreeMap<String, ipc::CheckoutPullRequest>,
}

/// When the forge last answered a repository's whole visit, and when it first stopped answering.
#[derive(Clone, Debug, Default)]
pub(crate) struct ForgeVisit {
    pub(crate) read_at: Option<Timestamp>,
    pub(crate) quiet_since: Option<Timestamp>,
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
    /// A line in the Job's own log. **A log line that will not write does not
    /// stop anything.**
    pub(crate) fn logged(&self, job: &JobId, envelope: Envelope) {
        self.noted_in_the_log(job, &envelope);
    }
}
