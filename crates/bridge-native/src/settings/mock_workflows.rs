//! The Workflows page's routes on the walk's mock Fleet: a definition step by step, its text, a save, a copy reset
//! or deleted, and a proposal to Armada. They answer from the definitions in `mock-workflow-definitions.json`, the
//! ones Classic's Workflows page is walked on (`apps/desktop/src/renderer/src/mock/workflows-managed.ts`), and keep
//! the list `GET /workflows` answers in step with them, as Fleet reads the three places again before answering.

use ipc::{ArmadaProposed, OverriddenWorkflow, ProposeToArmada, SaveWorkflow, SessionId, StepId, StepReading, WorkflowCopy, WorkflowCopyDeleted, WorkflowDefinition, WorkflowId, WorkflowSaved, WorkflowScope, WorkflowSteps};
use serde::Deserialize;

use super::definition::{text_of, tree_of, Json};
use super::mock::MockFleet;
use super::wire::Refused;

/// One definition held in a place; a repository's under the Manifest of the repository it is in.
#[derive(Clone, Debug, Deserialize)]
pub struct Held {
    pub source: String,
    #[serde(default)]
    pub manifest: Option<String>,
    pub definition: Json,
}

impl Held {
    fn id(&self) -> &str {
        self.definition.get("workflow_id").and_then(Json::text).unwrap_or_default()
    }

    fn holds(&self, id: &str, source: &str, manifest: &str) -> bool {
        self.id() == id && self.source == source && (source != "repository" || self.manifest.as_deref() == Some(manifest))
    }
}

pub fn definitions() -> Vec<Held> {
    ipc::decode("definitions", include_bytes!("mock-workflow-definitions.json")).unwrap_or_else(|why| panic!("the mock definitions do not decode: {why}"))
}

/// What ends each step and when it is passed over, as Fleet's rail reads them off the rules (`MANAGED_ENDS`). By
/// step id, and `workflow/step` where one workflow's differs.
const ENDS: &[(&str, &str, &[&str], Option<&str>)] = &[
    ("propose", "you reply", &[], None),
    (
        "feature/propose",
        "you approve what it shows",
        &["you approve the plan", "you approve a walk", "you approve a sketch", "you approve a draft", "you approve a document", "you pick one of the ideas shown"],
        None,
    ),
    ("walk", "you approve every walk", &[], Some("nothing would be seen")),
    ("refine", "it is pushed", &[], None),
    ("wire", "it is wired to the real thing", &[], None),
    ("deliver", "it is merged", &[], None),
    ("find_the_cause", "the cause is named", &[], None),
    ("fix", "you approve every walk", &[], Some("nothing would be seen")),
    ("tests", "the tests pass", &[], None),
    ("sketch", "you pick a direction", &[], None),
    ("record_plan", "a plan is recorded", &[], None),
    ("build", "the mock runs", &[], None),
    ("show", "you approve every walk", &[], None),
    ("draft", "the page reads right", &[], None),
];

fn file_of(source: &str, id: &str) -> String {
    match source {
        "armada" => format!("[armada]/{id}.yml"),
        "repository" => format!(".armada/workflows/{id}.yml"),
        _ => format!("~/.armada/kit/workflows/{id}.yml"),
    }
}

fn refused(code: &str, said: String) -> Refused {
    Refused { key: None, said, code: Some(code.to_string()) }
}

fn place_said(source: &str) -> &'static str {
    match source {
        "armada" => "Armada",
        "repository" => "This repository",
        _ => "Kit",
    }
}

impl MockFleet {
    fn held(&self, id: &str, source: &str, manifest: &str) -> Result<&Held, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        self.definitions.iter().find(|one| one.holds(id, source, manifest)).ok_or_else(|| refused("fleet.workflow_not_found", format!("{} holds no workflow `{id}`", place_said(source))))
    }

    pub fn workflow_steps(&self, id: &str, source: &str, manifest: &str) -> Result<WorkflowSteps, Refused> {
        let held = self.held(id, source, manifest)?;
        let Some(Json::List(steps)) = held.definition.get("steps") else { return Ok(WorkflowSteps { workflow_id: WorkflowId::carried(id), source: source.into(), steps: Vec::new() }) };
        let steps = steps
            .iter()
            .map(|one| {
                let step = one.get("id").and_then(Json::text).unwrap_or_default();
                let ends = ENDS.iter().find(|(key, ..)| *key == format!("{id}/{step}")).or_else(|| ENDS.iter().find(|(key, ..)| *key == step));
                StepReading {
                    id: StepId::carried(step),
                    label: one.get("label").and_then(Json::text).unwrap_or(step).to_string(),
                    instructions: one.get("instructions").and_then(Json::text).map(String::from),
                    ends_when: ends.map(|(_, said, ..)| said.to_string()),
                    ends_any: ends.map(|(_, _, any, _)| any.iter().map(|one| one.to_string()).collect()).unwrap_or_default(),
                    passed_over_when: ends.and_then(|(.., passed)| passed.map(String::from)),
                }
            })
            .collect();
        Ok(WorkflowSteps { workflow_id: WorkflowId::carried(id), source: source.into(), steps })
    }

    pub fn workflow_definition(&self, id: &str, source: &str, manifest: &str) -> Result<WorkflowDefinition, Refused> {
        let held = self.held(id, source, manifest)?;
        Ok(WorkflowDefinition { workflow_id: WorkflowId::carried(id), source: source.into(), file: file_of(source, id), definition: text_of(&held.definition), overridden_by: None })
    }

    pub fn save_workflow(&mut self, manifest: &str, save: SaveWorkflow) -> Result<WorkflowSaved, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let tree = tree_of(&save.definition).ok_or_else(|| refused("fleet.workflow_unfit", "The definition is not a workflow".into()))?;
        let id = tree.get("workflow_id").and_then(Json::text).unwrap_or_default().to_string();
        let source = save.scope.as_wire();
        let at = self.definitions.iter().position(|one| one.holds(&id, source, manifest));
        match at {
            Some(_) if !save.overwrite => return Err(refused("fleet.workflow_exists", format!("{} already holds a workflow `{id}`", place_said(source)))),
            Some(at) => self.definitions[at].definition = tree,
            None => self.definitions.push(Held { source: source.into(), manifest: (source == "repository").then(|| manifest.to_string()), definition: tree }),
        }
        self.relist(&id);
        Ok(WorkflowSaved { workflow_id: WorkflowId::carried(&id), scope: save.scope, file: file_of(source, &id), replaced: at.is_some(), runs_from: self.runs_from(&id, manifest) })
    }

    pub fn reset_workflow(&mut self, manifest: &str, copy: WorkflowCopy) -> Result<WorkflowSaved, Refused> {
        let id = copy.workflow_id.as_str().to_string();
        let armada = self.held(&id, "armada", manifest)?.definition.clone();
        let at = self.definitions.iter().position(|one| one.holds(&id, &copy.source, manifest)).ok_or_else(|| refused("fleet.workflow_not_found", format!("{} holds no workflow `{id}`", place_said(&copy.source))))?;
        self.definitions[at].definition = armada;
        self.relist(&id);
        let scope = if copy.source == "repository" { WorkflowScope::Repository } else { WorkflowScope::Kit };
        Ok(WorkflowSaved { workflow_id: copy.workflow_id, scope, file: file_of(&copy.source, &id), replaced: true, runs_from: self.runs_from(&id, manifest) })
    }

    pub fn delete_workflow(&mut self, manifest: &str, copy: WorkflowCopy) -> Result<WorkflowCopyDeleted, Refused> {
        let id = copy.workflow_id.as_str().to_string();
        if copy.source == "armada" {
            return Err(refused("fleet.workflow_armada", "Armada's own workflow is not a copy, and is not deleted".into()));
        }
        self.held(&id, &copy.source, manifest)?;
        self.definitions.retain(|one| !one.holds(&id, &copy.source, manifest));
        self.relist(&id);
        Ok(WorkflowCopyDeleted { workflow_id: copy.workflow_id, file: file_of(&copy.source, &id), source: copy.source, runs_from: self.runs_from(&id, manifest) })
    }

    pub fn propose_workflow(&mut self, proposing: ProposeToArmada) -> Result<ArmadaProposed, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        Ok(ArmadaProposed { workflow_id: proposing.workflow_id, session_id: SessionId::carried("01SESSIONPROPOSEAAAAAAAAAA"), send_failed: None })
    }

    /// The definitions of an id that apply in a repository, the one that runs first: Kit's, the repository's,
    /// then Armada's.
    fn applying(&self, id: &str, manifest: &str) -> Vec<&Held> {
        ["kit", "repository", "armada"].iter().filter_map(|source| self.definitions.iter().find(|one| one.holds(id, source, manifest))).collect()
    }

    fn runs_from(&self, id: &str, manifest: &str) -> Option<String> {
        self.applying(id, manifest).first().map(|one| one.source.clone())
    }

    /// The list's rows of an id, read again: in each repository that lists it, the definition that runs there and
    /// the ones it replaces.
    fn relist(&mut self, id: &str) {
        let mut manifests: Vec<String> = self.workflows.iter().filter(|one| one.id.as_str() == id).map(|one| one.manifest_id.as_str().to_string()).collect();
        for one in self.definitions.iter().filter(|one| one.id() == id) {
            if let Some(manifest) = &one.manifest {
                if !manifests.contains(manifest) {
                    manifests.push(manifest.clone());
                }
            }
        }
        if manifests.is_empty() {
            manifests.push("armada".into());
        }
        for manifest in manifests {
            let applying: Vec<Held> = self.applying(id, &manifest).into_iter().cloned().collect();
            let at = self.workflows.iter().position(|one| one.id.as_str() == id && one.manifest_id.as_str() == manifest);
            let Some(runs) = applying.first() else {
                if let Some(at) = at {
                    self.workflows.remove(at);
                }
                continue;
            };
            let Some(mut row) = at.map(|at| self.workflows[at].clone()).or_else(|| self.workflows.iter().find(|one| one.id.as_str() == id).cloned()) else { continue };
            row.manifest_id = ipc::ManifestId::carried(&manifest);
            row.source = runs.source.clone();
            row.file = file_of(&runs.source, id);
            row.for_requests = runs.definition.get("for_requests").and_then(Json::text).map(String::from);
            row.overrides = applying[1..].iter().map(|one| OverriddenWorkflow { source: one.source.clone(), file: file_of(&one.source, id) }).collect();
            match at {
                Some(at) => self.workflows[at] = row,
                None => self.workflows.push(row),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(id: &str, source: &str) -> WorkflowCopy {
        WorkflowCopy { workflow_id: WorkflowId::carried(id), source: source.into(), file: None }
    }

    #[test]
    fn yours_made_from_armadas_runs_in_its_place_and_deleting_it_gives_armadas_back() {
        let mut fleet = MockFleet::new();
        let text = fleet.workflow_definition("prototype", "armada", "armada").expect("Armada's prototype").definition;
        fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Kit, definition: text.clone(), overwrite: false }).expect("saved");
        let row = fleet.workflows.iter().find(|one| one.id.as_str() == "prototype").expect("listed");
        assert_eq!((row.source.as_str(), row.overrides.iter().map(|one| one.source.as_str()).collect::<Vec<_>>()), ("kit", vec!["armada"]));
        let again = fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Kit, definition: text, overwrite: false });
        assert!(again.is_err_and(|why| why.is("fleet.workflow_exists")));
        fleet.delete_workflow("armada", copy("prototype", "kit")).expect("deleted");
        let row = fleet.workflows.iter().find(|one| one.id.as_str() == "prototype").expect("still listed");
        assert_eq!((row.source.as_str(), row.overrides.len()), ("armada", 0));
    }

    #[test]
    fn a_reset_copy_reads_as_armadas_and_a_step_says_what_ends_it() {
        let mut fleet = MockFleet::new();
        let words = |fleet: &MockFleet| fleet.workflow_steps("feature", "repository", "armada").expect("read").steps.into_iter().find(|one| one.id.as_str() == "refine").and_then(|one| one.instructions);
        assert!(words(&fleet).is_some_and(|said| said.contains("repository's checks")));
        fleet.reset_workflow("armada", copy("feature", "repository")).expect("reset");
        assert_eq!(words(&fleet).as_deref(), Some("Wire it to the real thing and write the tests now. Show a new walk only if the look changed."));
        let propose = fleet.workflow_steps("feature", "armada", "armada").expect("read").steps.remove(0);
        assert_eq!((propose.ends_when.as_deref(), propose.ends_any.len()), (Some("you approve what it shows"), 6));
        assert!(fleet.delete_workflow("armada", copy("feature", "armada")).is_err());
    }
}
