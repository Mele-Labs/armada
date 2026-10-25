//! What an adversarial review of a plan is shown, and what Fleet makes of its answer.
//! `docs/concepts/adversarial-review.md`.
//!
//! **Pure.** The brief is laid from what [`Shown`] carries and nothing else, and the answer is held to
//! that brief and to the files it cites, read through a function the caller hands in. `adversarial_serving`
//! is the Fleet around it: the call, the store and the person's acts.

use ipc::{AdversarialFinding, FindingAnchor, SessionPlan};

/// Everything the call is shown. **There is no field for the agent's words**: its messages, its account
/// of the work and its transcript are withheld, as the Judge's are, and a brief built from this cannot
/// carry them.
pub struct Shown<'a> {
    /// The person's first message in the Job's thread, as the Judge reads it.
    pub request: &'a str,
    /// His messages after it, oldest first.
    pub later: &'a [String],
    pub plan: &'a SessionPlan,
    /// The repository's standing rules, as the Judge's brief lays them.
    pub standing: &'a str,
}

/// The brief the call is put.
pub fn brief(_shown: &Shown<'_>) -> String {
    String::new()
}

/// The findings in `answer`, each pinned to the part of `plan` it is about, or why the answer is
/// refused. `file` reads a path from the checkout's root, and says `None` for one it does not hold.
pub fn findings_read(
    _answer: &str,
    _brief: &str,
    _plan: &SessionPlan,
    _file: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<AdversarialFinding>, String> {
    Err("not read yet".to_string())
}

/// The part of `plan` the words `about` name.
pub fn pinned(_about: &str, _plan: &SessionPlan) -> FindingAnchor {
    FindingAnchor::Plan
}
