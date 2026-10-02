//! The apparatus Arc's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for `focus`'s reason: Arc asks who signed each row a
//! ruling wrote, which is a different question of the same machinery. It
//! reaches [`Bench`]'s private step log because it is a child module of the one
//! that declares it, and a second copy of that log is how the two would come to
//! disagree.

use core_model::{Actor, StepId, StepTarget};
use fleet::Clock;

use super::{Bench, Run};

impl Bench {
    /// Move one step, signed by whoever the caller names. [`Bench::settled`]
    /// names [`fleet::Ruling::signed_by`], which is what `fleet::dispatch`
    /// signs the stop with.
    pub fn step_moved_by(&self, run: &mut Run, step: &StepId, to: StepTarget, by: Actor) {
        let moved = run
            .job
            .transition_step(step, to, by, self.clock.now())
            .expect("a legal step move");
        self.step_moves.borrow_mut().push(moved.event);
        run.job = moved.job;
    }
}

/// Who signed each step move, oldest first.
pub fn step_signers(bench: &Bench) -> Vec<Actor> {
    bench
        .step_moves
        .borrow()
        .iter()
        .map(|event| event.actor())
        .collect()
}
