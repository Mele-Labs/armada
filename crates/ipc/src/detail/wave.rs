//! One pass of an Epic's plan, as its wave strip reads it. Spike 022, slice 6
//! (#1692): `WaveRoundView`'s per-pass line.

use serde::{Deserialize, Serialize};

/// What one pass of a Job's plan split the work into.
///
/// **The plan's own `approach`, as that pass last recorded it.** A Judge's
/// refusal reruns the plan inside the same pass and its recording replaces the
/// one before, so the pass reads what was proposed and not a first draft; a
/// return from the roll-up is a new pass, and the old pass keeps its line.
///
/// **The whole paragraph, never a cut of it.** Where a strip has room for one
/// line it clips, and the paragraph is still there for whatever shows it
/// whole — a sentence Fleet cut would be a summary nobody wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveRound {
    /// Which pass, counted from one: the number a child's
    /// `JobSummary::dispatched_pass` carries.
    pub pass: u32,
    /// The approach that pass's plan opened with. Never blank.
    pub approach: String,
}
