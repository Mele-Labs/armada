//! The two counts a step's loop and retry gates are asked against.
//!
//! Beside [`ResolvedStep::may_hand_back`](super::frozen::ResolvedStep::may_hand_back)
//! and `may_return`, which take them. [`Attempt`], the third count, keys the
//! per-run records and stays with the Job record that has them.

use core::fmt;
use core::num::NonZeroU32;

/// Which pass over a step this is. One-based.
///
/// **The loop counterpart of [`Attempt`], and constructed the same two ways for
/// the same reason**: [`returns_made`](Iteration::returns_made) derives it from
/// the step's own log and [`stored`](Iteration::stored) validates one read back
/// off a row. There is no constructor taking an arbitrary number, so nothing
/// can hand a cap a count the history does not support.
///
/// # What it counts, and what it does not claim
///
/// The entries into `running` from `advanced` in this step's log. That edge is
/// the loop return and only the loop return, so the count is an observation
/// rather than a policy — exactly [`Attempt`]'s standing, and for the same
/// reason: the questions the registry leaves open are about *whose* counter
/// increments, and an observation about one step's log does not have to answer
/// them to be true about that step.
///
/// **It is therefore not yet `iteration_count`.** That counter is the emitting
/// step's — `docs/journeys/triage-queue.md` settles it, because a cap and the
/// count it bounds must not be split or `loop_cap` never fires, and because the
/// emitting-step reading is the only one that survives two loops sharing a
/// target. The emitting step has no move of its own on a return, so it has
/// nothing to count yet, and this reads as the routed-to step's passes — which
/// is true about that step, renders as "draft · 3rd", and is not the number the
/// cap is asked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iteration(NonZeroU32);

impl Iteration {
    /// The pass a step is on before anything has routed back to it. **Every
    /// step of a linear workflow is on this one forever**, which is why the
    /// type is not an `Option`: a step that has never looped is on its first
    /// pass, not on no pass.
    pub const FIRST: Iteration = Iteration(NonZeroU32::MIN);

    /// The pass in progress, given how many times a step's log records it
    /// entering `running` from `advanced`.
    ///
    /// **Off by one against the returns, deliberately.** A step redone once is
    /// on its second pass, and the arithmetic lives here rather than at the
    /// caller for the reason `ResolvedStep::may_hand_back` gives about the
    /// retry budget: a caller adding the one itself is a second place the
    /// off-by-one can be wrong.
    pub fn returns_made(returns: u32) -> Iteration {
        match returns.checked_add(1).and_then(NonZeroU32::new) {
            Some(count) => Iteration(count),
            // Unreachable off a real log — it would need four billion returns
            // past a cap of five — and saturating is the only answer that is
            // not a lie: the highest pass this type can name is nearer the
            // truth than wrapping to the first.
            None => Iteration(NonZeroU32::MAX),
        }
    }

    /// A pass read back off a stored row. `None` on zero, which is not a pass
    /// and is a malformed column rather than a first one.
    pub fn stored(number: u32) -> Option<Iteration> {
        NonZeroU32::new(number).map(Iteration)
    }

    /// The ordinal, for a rendering. One-based, so a rail reads "iteration 3 of
    /// 5" off this and the cap without arithmetic of its own.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Iteration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
mod iteration_tests {
    use super::Iteration;

    #[test]
    fn a_step_nothing_has_returned_to_is_on_its_first_pass() {
        assert_eq!(Iteration::returns_made(0), Iteration::FIRST);
        assert_eq!(Iteration::returns_made(0).number(), 1);
    }

    /// The off-by-one the type exists to own: one return, second pass.
    #[test]
    fn one_return_is_the_second_pass() {
        assert_eq!(Iteration::returns_made(1).number(), 2);
        assert_eq!(Iteration::returns_made(4).number(), 5);
    }

    #[test]
    fn a_stored_zero_is_refused_rather_than_read_as_a_first_pass() {
        assert_eq!(Iteration::stored(0), None);
        assert_eq!(Iteration::stored(3), Some(Iteration::returns_made(2)));
    }

    #[test]
    fn a_count_that_could_not_come_off_a_log_saturates_rather_than_wrapping() {
        assert_eq!(Iteration::returns_made(u32::MAX).number(), u32::MAX);
    }

    /// The order is what a cap is asked against.
    #[test]
    fn passes_order_by_their_number() {
        assert!(Iteration::returns_made(0) < Iteration::returns_made(1));
    }
}

/// How many of a step's runs the pass it is on has spent. One-based.
///
/// **The count `retry_limit` is asked against, and the only one.** It is not
/// [`Attempt`], which keys every per-run record and therefore has to climb
/// across a loop return so a second pass's verdicts do not overwrite a first
/// pass's. This resets on that return, because `retry_limit`'s registry row
/// says a re-entry as designed is a fresh budget — and it is a separate type
/// precisely so that the two cannot be handed to the same call.
///
/// **Identical to [`Attempt`] on every step of every linear workflow**, which
/// is why the mistake went unnoticed: nothing had ever returned, so the two
/// readings of the log agreed.
///
/// Constructed one way only, from the step's own log, for [`Attempt`]'s reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spent(NonZeroU32);

impl Spent {
    /// The run a pass is on before anything has been handed back inside it.
    pub const FIRST: Spent = Spent(NonZeroU32::MIN);

    /// The run this pass is on, given how many times the step has entered
    /// `running` since the last verdict routed back to it.
    ///
    /// **Zero runs since the return is still the first of the pass**, for
    /// [`Attempt::runs_begun`]'s reason: the return itself writes the row that
    /// makes it one, and a record written between the two belongs to the run
    /// it is part of.
    pub fn runs_this_pass(entries_into_running: u32) -> Spent {
        match NonZeroU32::new(entries_into_running) {
            Some(count) => Spent(count),
            None => Spent::FIRST,
        }
    }

    /// The ordinal, for the arithmetic `ResolvedStep::may_hand_back` owns.
    /// One-based.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Spent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
mod spent_tests {
    use super::Spent;

    #[test]
    fn a_pass_that_has_not_begun_a_run_is_still_on_its_first() {
        assert_eq!(Spent::runs_this_pass(0), Spent::FIRST);
        assert_eq!(Spent::runs_this_pass(0).number(), 1);
    }

    #[test]
    fn runs_of_a_pass_order_by_their_number() {
        assert!(Spent::runs_this_pass(1) < Spent::runs_this_pass(2));
    }
}
