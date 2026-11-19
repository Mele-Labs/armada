//! The settings that change the burn, what each would save, and what each change did, for the Usage page.
//! `docs/practices/protocol.md`, *Settings that change the burn*.
//!
//! **`now`, `suggested` and every choice's `value` are what `save_settings` takes for `key`**: a word for a
//! choice, a number for a count of tokens or seconds. A `label` is what a person reads. A share is 0 to 1 of
//! the window's whole burn, in the units `get_usage_summary` weighs. **Names and figures only.**

use serde::{Deserialize, Serialize};

use crate::SettingValue;

/// One value a setting may be changed to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustChoice {
    pub value: SettingValue,
    pub label: String,
}

/// A setting the page changes in place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustSetting {
    /// The settings.json path, which `save_settings` takes as the key of a change.
    pub key: String,
    pub label: String,
    /// What the file holds now, or what ships where it holds none.
    pub now: SettingValue,
    /// The values the page offers, always with `now` among them.
    pub choices: Vec<AdjustChoice>,
    /// The value the window's requests say would save the most. Absent where none would, or where Fleet
    /// cannot say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested: Option<SettingValue>,
    /// The id of the waste kind it cuts, as `get_usage_waste` names it.
    pub cuts: String,
    /// The share of the window's burn a change to `suggested` would save. Absent where Fleet cannot
    /// estimate it from the window's requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saves_share: Option<f64>,
}

/// A change to one of those settings, and the waste it cuts before and since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustChange {
    pub key: String,
    pub label: String,
    /// What it was and what it became, as a person reads them: `5 min`, `1 hour`.
    pub from: String,
    pub to: String,
    /// When the change came into force.
    pub at: String,
    pub cuts: String,
    /// The `cuts` kind's share of the burn over the seven days before `at`.
    pub before_share: f64,
    /// The same share from `at` to now. Absent until a day has passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_share: Option<f64>,
    /// The days from `at` to now, which is how much `since_share` has to go on.
    pub since_days: f64,
}

/// `get_usage_adjustments` (`GET /usage/adjustments?window=`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageAdjustments {
    pub settings: Vec<AdjustSetting>,
    /// Newest first, of the last thirty days.
    pub changes: Vec<AdjustChange>,
}
