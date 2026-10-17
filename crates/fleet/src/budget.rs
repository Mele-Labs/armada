//! How long a Check may run.

use std::time::Duration;

/// How long a Check may run before it is a failure.
///
/// A newtype rather than a bare `Duration` so that the argument cannot be confused with any other
/// duration at a call site, and so that the one place the value is decided is visible in a
/// search. **No `Default`, and one constructor taking a duration**: the threshold is a decision
/// somebody wrote down in `settings.toml`, and a default here would put one where nobody can
/// find it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckBudget(Duration);

impl CheckBudget {
    pub fn of(budget: Duration) -> CheckBudget {
        CheckBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}
