//! The workflows switched off, for this machine and for single repositories: the value of the
//! `workflows.off` setting, saved by Settings. `features.workflows` is the switch for all of them;
//! this is for one. `crates/config/settings.toml`, *Workflows switched off*.
//!
//! **Here for `reach_folders`' reason**: JSON is read only in `ipc` and `store`. Which workflow a
//! Job may follow is Fleet's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Workflow ids per place, in the order they were switched off.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowsOff {
    /// Off in every repository this machine serves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub machine: Vec<String>,
    /// Off in one repository, by its manifest id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub repositories: BTreeMap<String, Vec<String>>,
}

impl WorkflowsOff {
    /// What `text` holds. **Lenient**: text that is not the shape reads as nothing off, so a hand
    /// edit gone wrong leaves every workflow on rather than none.
    pub fn read(text: &str) -> WorkflowsOff {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// Whether `workflow` is off for the repository `manifest`: off for the machine, or for it.
    pub fn has(&self, manifest: &str, workflow: &str) -> bool {
        self.machine.iter().any(|off| off == workflow)
            || self.repositories.get(manifest).is_some_and(|off| off.iter().any(|one| one == workflow))
    }
}
