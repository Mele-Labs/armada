//! What a Job is allowed to spend, and the one predicate that says it has.
//!
//! `docs/concepts/machine.md`, Budget, is where this is argued: why the cap is
//! per Job, why it is two numbers rather than one, and why it refuses the next
//! dispatch rather than stopping a Drone. `docs/spikes/005-what-does-a-job-cost.md`
//! is the measurement under all three.
//!
//! Two things the concept page cannot say, because they are about this code:
//!
//! **Wall clock is the spike's third signal and is not here.**
//! `settings.drone-job-timeout` already bounds a Job's wall clock at
//! Kit-to-Manifest scope, so a second ceiling would be two answers to one
//! question. `store::DroneSpend::ran_ms` is recorded anyway, so that row has a
//! figure when somebody enforces it.
//!
//! **Quota is not a fourth.** Spike 5 settled it: the rate-limit event carries
//! a window and a status and no quantity.
//!
//! **Both halves tier the same way**, and [`Allowance::at`] is the only place
//! that order is written. The turns did not until Sept 2026, and
//! [`Overspent::Turns`] carries what that cost.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::Manifest;
use core_model::{Job, JobId};
use store::Spend;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A quantity of money, held as millionths of a dollar and named in the unit it
/// was decided in.
///
/// **An integer, like [`crate::headroom::Bytes`].** A cap compared as a float
/// answers differently on two machines for the same spend, and the figure
/// arrives as an integer anyway: `adapters::transcript` turns `total_cost_usd`
/// into micros at the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Micros(u64);

impl Micros {
    pub const fn dollars(dollars: u64) -> Micros {
        Micros(dollars * 1_000_000)
    }

    /// A figure that already arrived in this unit — off the wire, or off the
    /// Job's own column. **Beside [`Micros::dollars`] rather than replacing
    /// it**: a caller writing a ceiling down says what it means in the unit it
    /// decided in, and a caller carrying one across a boundary does not
    /// convert.
    pub const fn of(count: u64) -> Micros {
        Micros(count)
    }

    pub const fn count(&self) -> u64 {
        self.0
    }
}

/// Which of a Job's two ceilings it has gone past.
///
/// **Both fold to `QueuedReason::OverBudget` on the Board**, which is the only
/// label `job-statuses.toml` gives a `queued` Job held back by what it has
/// spent. Which of the two it was travels beside that label as
/// `core_model::BudgetHold`, exactly as [`Short`](crate::headroom::Short) does
/// as `AdmissionHold` — and it travels because reading it off the figures alone
/// asks a person to know there are two ceilings before they can see which one
/// caught them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overspent {
    /// Past `settings.budget-cost-cap-per-job`, as [`Allowance::at`] resolved
    /// it for this Job. The remedy is a number: raise the cap on this Job, on
    /// the repository, or accept that this Job costs what it costs.
    Cost,
    /// Past `settings.budget-turn-cap-per-job`, as [`Allowance::at`] resolved
    /// it for this Job. **Two remedies and not one**, which is the correction
    /// this variant carries: a Job that is turning and getting nowhere was not
    /// askable as written and wants a redispatch, and a Job that has finished
    /// its work and is held out of a cheap last step wants the number moved.
    /// The type cannot tell those apart and does not try — what it says is
    /// which ceiling, so that both acts are reachable.
    ///
    /// The reading that only the brief could fix shipped with this variant and
    /// was falsified by one Job: 393 turns against 300, every Check passed,
    /// `summarise` never run, and a redispatch would have thrown away the work
    /// rather than finished it.
    Turns,
}

impl std::fmt::Display for Overspent {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Overspent::Cost => "cost",
            Overspent::Turns => "turns",
        })
    }
}

/// What one Job may spend before Fleet stops starting Drones on it.
///
/// **No `Default`**, for [`Concurrency`](crate::Concurrency)'s reason: the
/// numbers are a decision somebody made and wrote down, and a type that
/// supplies them lets a caller not make it. `crates/armada/src/serve.rs`
/// resolves them and carries the argument for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allowance {
    cost: Micros,
    turns: u64,
}

impl Allowance {
    pub const fn of(cost: Micros, turns: u64) -> Allowance {
        Allowance { cost, turns }
    }

    pub const fn cost(&self) -> Micros {
        self.cost
    }

    pub const fn turns(&self) -> u64 {
        self.turns
    }

    /// What this one Job may spend, given what Fleet is running with.
    ///
    /// # Three tiers each, and this is the only place their order is written
    ///
    /// The composition root's constant — `self` — then `armada.yml`'s
    /// `drone.cost_cap_micros_per_job` or `drone.turn_cap_per_job`, then the
    /// Job's own column, each deferring upward where it states nothing.
    /// **`Some(0)` is a cap and `None` is an absence**: capped at zero a Job
    /// starts nothing, holding one Job or one repository.
    ///
    /// # The turns tier too, and that reverses a decision
    ///
    /// They did not until Sept 2026, because spike 5's three runs of one Job
    /// spread 2.31x on price while their turns held at 7, 7 and 4 — so a Job
    /// over the turn cap was going in circles, where a bigger number buys more
    /// circles. The conclusion did not survive the measurement: One Job
    /// finished, passed every Check, and stopped at 393 turns against 300 with
    /// a cheap `summarise` unrun.
    ///
    /// # Live at every tier, and frozen at none
    ///
    /// A Job past its cap is refused at every admission until a number moves,
    /// so a cap frozen at creation — as `quiet_after_seconds` is — would reach
    /// every Job but the one that needs it.
    pub fn at(self, manifest: &Manifest, job: &Job) -> Allowance {
        let repository = match manifest.cost_cap_micros() {
            Some(micros) => Micros(u64::from(micros)),
            None => self.cost,
        };
        let turns = match manifest.turn_cap() {
            Some(turns) => u64::from(turns),
            None => self.turns,
        };
        Allowance {
            cost: match job.cost_cap_micros() {
                Some(micros) => Micros(micros),
                None => repository,
            },
            turns: match job.turn_cap() {
                Some(capped) => capped,
                None => turns,
            },
        }
    }

    /// Which ceiling this spend is past, or `None` where it is inside both.
    ///
    /// **Cost first**, and the order decides only which one is named: dollars
    /// are what the row a person set is denominated in, so a Job over both
    /// reads as the thing they were watching.
    ///
    /// The comparison is `>=` and not `>`. A cap is what a Job may spend up to,
    /// and a Job that has spent exactly its allowance has nothing left to start
    /// a Drone with.
    pub fn exceeded_by(&self, spent: &Spend) -> Option<Overspent> {
        if spent.cost_micros >= self.cost.count() {
            return Some(Overspent::Cost);
        }
        if spent.turns >= self.turns {
            return Some(Overspent::Turns);
        }
        None
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

    /// What this Job has spent across every Drone that has worked it.
    pub(crate) async fn spend_of(&self, job: &JobId) -> Result<Spend, Adrift> {
        self.store()
            .lock()
            .await
            .spend_for(job)
            .map_err(Adrift::Reading)
    }

}
