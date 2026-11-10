//! How full a limit is, as a ring draws it. Carried on a plan window and on a Job's context, so a
//! client reads the band and does not hold the two lines. **A state, never a colour.**

use serde::{Deserialize, Serialize};

/// Warning from this whole percent.
pub const WARNING_FROM: u32 = 80;
/// Danger from this whole percent.
pub const DANGER_FROM: u32 = 95;

/// Normal below 80, warning from 80, danger from 95. Bridge's `UsageLevel`, value for value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RingBand {
    #[default]
    Normal,
    Warning,
    Danger,
}

impl RingBand {
    /// The band of a whole percent. A figure past 100, an exceeded spend limit's, is danger.
    pub fn of(percent: u32) -> RingBand {
        if percent >= DANGER_FROM {
            RingBand::Danger
        } else if percent >= WARNING_FROM {
            RingBand::Warning
        } else {
            RingBand::Normal
        }
    }
}
