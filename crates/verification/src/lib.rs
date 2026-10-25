//! The Judge's brief for a Session's work, and how a Check's process ended.
//!
//! The Judge is a veto and not a grant, and it is blind to how the work went:
//! [`Work`] has no field for a transcript, so a brief cannot carry the agent's
//! account of its own work. A Judge's answer is read back by field name and an
//! answer that quotes words which are not in the brief is discarded.
//!
//! **Armada does not parse:** which lines of a test run were the failure is a
//! Judge's question, answered by reading the diff, never a runner's output.
//! [`Exit`] is the fact a Check's process ended with, before anything decides
//! what it means.

mod judge;
mod mechanical;
mod quoted;
mod session_work;
mod shown;
mod standing;
mod wording;

#[cfg(test)]
mod tests;

pub use judge::{field, Brief, Refusals, Unreadable};
pub use mechanical::{Exit, NeverRan};
pub use quoted::invented;
pub use session_work::Work;
pub use shown::{digest, digest_of};
pub use standing::{Standing, STANDING_RULES};
pub use wording::{fill, Piece, Wording};

/// Every authored piece of a Judge brief, by name, for a caller that hands in
/// its own words for one. What each says is beside the code that lays it.
pub mod pieces {
    pub use crate::judge::{JUDGE_ANSWER, JUDGE_OPENING, JUDGE_QUESTION, JUDGE_RISK};
    pub use crate::standing::{
        JUDGE_READABLE, JUDGE_STANDING, JUDGE_STANDING_CUT, JUDGE_STANDING_UNREADABLE,
    };
}
