//! Sleep mode on the wire: the switch and the night it keeps. `docs/concepts/session.md`.

use serde::{Deserialize, Serialize};

/// A question answered for the owner. `chose` is empty where the agent was told to decide for itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepDecided {
    pub id: String,
    pub who: String,
    pub asked: String,
    pub chose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrected: Option<String>,
}

/// Something only the owner can settle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepBlocked {
    pub id: String,
    pub who: String,
    pub text: String,
}

/// A pull request that merged during the night.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepLanded {
    pub id: String,
    pub who: String,
    pub title: String,
    pub pr: String,
}

/// A walk held for the owner to look at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepWalk {
    pub id: String,
    pub who: String,
    pub title: String,
}

/// `get_sleep`, and what `set_sleep` and `override_sleep` answer: the switch and the night's rows.
/// Also published whole as `sleep.changed`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepState {
    pub on: bool,
    pub decided: Vec<SleepDecided>,
    pub blocked: Vec<SleepBlocked>,
    pub landed: Vec<SleepLanded>,
    pub walks: Vec<SleepWalk>,
}

/// `set_sleep`: turn the mode on (a new night) or off.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSleep {
    pub on: bool,
}

/// `override_sleep`: the owner's correction of one decision, sent to the session it answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverrideSleep {
    pub id: String,
    pub text: String,
}
