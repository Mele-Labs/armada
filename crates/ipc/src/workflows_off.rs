//! The workflows switched off, for this machine and for single repositories: the value of the
//! `workflows.off` setting, saved by Settings. `features.workflows` is the switch for all of them;
//! this is for one. `crates/config/settings.toml`, *Workflows switched off*.
//!
//! **Here for `reach_folders`' reason**: JSON is read only in `ipc` and `store`. Which workflow a
//! Job may follow is Fleet's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ValueLayer;

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

    /// The layer that switched `workflow` off for the repository `manifest`, in a project whose own list is
    /// `project`, or `None` where it is on. **Off wins**: a project cannot switch back on what the machine or the
    /// repository switched off, so the layer named is the furthest that lists it, machine then repository then
    /// project; that is the switch that holds it off. No repository asks the machine's list and the project's.
    pub fn off_from(&self, manifest: Option<&str>, project: &[String], workflow: &str) -> Option<ValueLayer> {
        let names = |list: &[String]| list.iter().any(|one| one == workflow);
        let repository = manifest.and_then(|manifest| self.repositories.get(manifest));
        if names(&self.machine) {
            Some(ValueLayer::Machine)
        } else if repository.is_some_and(|list| names(list)) {
            Some(ValueLayer::Repository)
        } else {
            names(project).then_some(ValueLayer::Project)
        }
    }

    /// The layer holding `workflow` off for a project asked about without one repository, whose repositories are
    /// `manifests`: as [`WorkflowsOff::off_from`] with none, except that **a workflow every one of them switches off is
    /// off from `repository`**, since no Job in the project can follow it. One switched off in only some is left as the
    /// machine's list and the project's say.
    pub fn off_across(&self, manifests: &[&str], project: &[String], workflow: &str) -> Option<ValueLayer> {
        let held = self.off_from(None, project, workflow);
        let everywhere = !manifests.is_empty()
            && manifests.iter().all(|manifest| self.repositories.get(*manifest).is_some_and(|list| list.iter().any(|one| one == workflow)));
        match held {
            Some(ValueLayer::Machine) => held,
            _ if everywhere => Some(ValueLayer::Repository),
            _ => held,
        }
    }
}
