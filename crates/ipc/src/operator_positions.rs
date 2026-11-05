//! How far each of the Operator's roles goes: Off, Suggest or On, for the machine and for single
//! repositories. `crates/config/settings.toml`, *Operator positions per repository*.
//!
//! **Here for `reach_folders`' reason**: JSON is read only in `ipc` and `store`. What a position
//! does is Fleet's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One of the roles a person turns. Evaluate is the Operator switch itself; Raise has no control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRole {
    /// Starts, holds and merges Jobs.
    Dispatch,
    /// Sends words to a live Job.
    Delegate,
    /// Settles questions and permission cards as the person's press would.
    Answer,
    /// Files new issues.
    File,
}

/// How far a role goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPosition {
    /// Each ruling is only in the Log.
    #[default]
    Off,
    /// Each ruling is in the Log and on a raised row, for one press.
    Suggest,
    /// Fleet does what is ruled.
    On,
}

impl OperatorPosition {
    /// The position a setting's word names. **A word that is not one is Off**, so a hand edit gone
    /// wrong acts on nothing.
    pub fn read(word: &str) -> OperatorPosition {
        match word {
            "on" => OperatorPosition::On,
            "suggest" => OperatorPosition::Suggest,
            _ => OperatorPosition::Off,
        }
    }
}

/// The roles that differ from the machine's position, per repository by its manifest id: the value
/// of `features.operatorRepositories`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperatorRepositories(pub BTreeMap<String, BTreeMap<OperatorRole, OperatorPosition>>);

impl OperatorRepositories {
    /// What `text` holds. **Lenient**: text that is not the shape reads as no repository differing.
    pub fn read(text: &str) -> OperatorRepositories {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// The position `manifest` sets for `role`, where it differs from the machine's.
    pub fn of(&self, manifest: &str, role: OperatorRole) -> Option<OperatorPosition> {
        self.0.get(manifest)?.get(&role).copied()
    }
}
