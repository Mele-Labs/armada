//! The Workflows page's routes on the walk's mock Fleet: a definition step by step, its text, a save, a copy reset
//! or deleted, and a proposal to Armada. They answer from the definitions in `mock-workflow-definitions.json`, the
//! ones Classic's Workflows page is walked on (`apps/desktop/src/renderer/src/mock/workflows-managed.ts`), and keep
//! the list `GET /workflows` answers in step with them, as Fleet reads the three places again before answering.
//! A change asked of an agent is `drafting` for one read and drafted on the next, as Classic's mock drafts it
//! (`apps/desktop/src/renderer/src/mock/workflows-fleet.ts`, `settled`).

use ipc::{DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved, SaveLifecyclePiece, ArmadaProposed, AskWorkflowChange, ChangeOrigin, ChangeState, ChangeStep, Instant, OverriddenWorkflow, ProposeToArmada, SaveWorkflow, SendBackWorkflowChange, SessionId, StepId, StepReading, WorkflowChange, WorkflowChangeApproved, WorkflowChanges, WorkflowCopy, WorkflowCopyDeleted, WorkflowDefinition, WorkflowId, WorkflowSaved, WorkflowScope, WorkflowSteps, WorkflowSupportBuilding};
use serde::Deserialize;

use super::definition::{text_of, tree_of, with_words, Json};
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

/// One change asked for, with the reads left before it is drafted and the Manifest it began under.
#[derive(Clone, Debug)]
pub struct Kept {
    change: WorkflowChange,
    reads_left: u8,
    manifest: String,
}

/// What Armada lacks for "Propose should end when a walk, a sketch or a document is approved" (`CANNOT_YET`).
const CANNOT_YET: [&str; 3] = ["A step ends on one thing, not on any of several", "Nothing ends a step when a sketch is approved", "Nothing ends a step when a document is approved"];

/// The sentence an agent adds to Walk's words for "Walk should say why when it skips the walk" (`SAY_WHY_SKIPPED`).
const SAY_WHY_SKIPPED: &str = "Where nothing would be seen, say so and why before going on.";

/// The steps an agent comes back with for "split Refine into Wire and Tests" (`WIRE_AND_TESTS`).
const WIRE_AND_TESTS: [(&str, &str, &str); 2] = [("wire", "Wire", "Wire it to the real thing. Show a new walk only if the look changed."), ("tests", "Tests", "Write the tests now, against what the walk showed.")];

/// The Job a press on Start a job to build it starts.
const SUPPORT_JOB: &str = "01SESSIONSUPPORTAAAAAAAAAA";

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

/// A definition step by step, with what ends each as Fleet's rail reads it.
fn readings(id: &str, tree: &Json) -> Vec<StepReading> {
    let Some(Json::List(steps)) = tree.get("steps") else { return Vec::new() };
    steps
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
        .collect()
}

/// A definition's text as the steps either side of a change carries (`changeStepsOf`).
fn change_steps(id: &str, text: &str) -> Option<Vec<ChangeStep>> {
    let tree = tree_of(text)?;
    Some(
        readings(id, &tree)
            .into_iter()
            .map(|one| ChangeStep { id: one.id, label: one.label, instructions: one.instructions, ends_when: one.ends_when, ends_any: one.ends_any, passed_over_when: one.passed_over_when, about: None, gate: None, asks_delivery: false, delivers: false })
            .collect(),
    )
}

/// What the agent drafts for his words, as Classic's mock does: a request Armada cannot meet, Refine split into
/// Wire and Tests, a sentence added to Walk's words, or nothing new, which fails.
fn drafted(mut change: WorkflowChange) -> WorkflowChange {
    let ChangeOrigin::Asked { text: asked } = &change.origin else { return change };
    let asked = asked.to_lowercase();
    let id = change.workflow_id.as_str().to_string();
    let has = |words: &[&str]| words.iter().all(|word| asked.contains(word));
    if has(&["propose", "end", "approved"]) {
        change.state = ChangeState::Cannot;
        change.missing = CANNOT_YET.iter().map(|one| one.to_string()).collect();
        return change;
    }
    let mut after = None;
    if let Some(mut tree) = tree_of(&change.before) {
        if asked.contains("split refine") {
            if let Json::Map(fields) = &mut tree {
                if let Some((_, Json::List(steps))) = fields.iter_mut().find(|(name, _)| name == "steps") {
                    if let Some(at) = steps.iter().position(|one| one.get("id").and_then(Json::text) == Some("refine")) {
                        let split = WIRE_AND_TESTS.iter().map(|(id, label, words)| Json::Map(vec![("id".into(), Json::Text(id.to_string())), ("label".into(), Json::Text(label.to_string())), ("instructions".into(), Json::Text(words.to_string()))]));
                        steps.splice(at..=at, split);
                        after = Some(text_of(&tree));
                    }
                }
            }
        }
        // "Add a Review step after Build": a step of that name after the one named.
        if let Some((name, anchor)) = asked.strip_prefix("add a ").and_then(|rest| rest.split_once(" step after ")) {
            let (name, anchor) = (name.trim(), anchor.trim().trim_end_matches('.'));
            if let Json::Map(fields) = &mut tree {
                if let Some((_, Json::List(steps))) = fields.iter_mut().find(|(field, _)| field == "steps") {
                    let at = steps.iter().position(|one| one.get("id").and_then(Json::text) == Some(anchor) || one.get("label").and_then(Json::text).is_some_and(|label| label.to_lowercase() == anchor));
                    if let (Some(at), false) = (at, name.is_empty()) {
                        let label = name[..1].to_uppercase() + &name[1..];
                        steps.insert(at + 1, Json::Map(vec![("id".into(), Json::Text(name.replace(' ', "_"))), ("label".into(), Json::Text(label)), ("instructions".into(), Json::Text("Check the work so far before going on.".into()))]));
                        after = Some(text_of(&tree));
                    }
                }
            }
        }
        let walk = readings(&id, &tree).into_iter().find(|one| one.id.as_str() == "walk").and_then(|one| one.instructions);
        if let Some(words) = walk.filter(|_| has(&["walk", "say why", "skip"])) {
            let closing = "Notes come back as another take.";
            let words = if words.contains(closing) { words.replace(closing, &format!("{SAY_WHY_SKIPPED} {closing}")) } else { format!("{words} {SAY_WHY_SKIPPED}") };
            after = with_words(&change.before, "walk", &words);
        }
    }
    match after {
        Some(after) => {
            change.state = ChangeState::Ready;
            change.before_steps = change_steps(&id, &change.before);
            change.after_steps = change_steps(&id, &after);
            change.after = Some(after);
        }
        None => {
            change.state = ChangeState::Failed;
            change.reason = Some("The drafted workflow is the same as the one it was made from".into());
        }
    }
    change
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
        Ok(WorkflowSteps { workflow_id: WorkflowId::carried(id), source: source.into(), steps: readings(id, &held.definition) })
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

    /// `POST /lifecycles/pieces/save`: answered as written, kept nowhere.
    pub fn save_lifecycle_piece(&mut self, save: SaveLifecyclePiece) -> Result<LifecyclePieceSaved, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let (kind, id, scope) = match save.piece {
            LifecyclePiece::Lifecycle { lifecycle } => (LifecyclePieceKind::Lifecycle, lifecycle.id, lifecycle.scope),
            LifecyclePiece::Rule { rule } => (LifecyclePieceKind::Rule, rule.id, rule.scope),
            LifecyclePiece::Autonomy { autonomy } => (LifecyclePieceKind::Autonomy, autonomy.id, autonomy.scope),
        };
        Ok(LifecyclePieceSaved { kind, file: format!("~/.armada/kit/{}s/{id}.yml", kind.as_wire()), id, scope, replaced: save.overwrite })
    }

    /// `POST /lifecycles/pieces/delete`.
    pub fn delete_lifecycle_piece(&mut self, delete: DeleteLifecyclePiece) -> Result<LifecyclePieceDeleted, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        Ok(LifecyclePieceDeleted { kind: delete.kind, file: format!("~/.armada/kit/{}s/{}.yml", delete.kind.as_wire(), delete.id), id: delete.id, scope: delete.scope })
    }

    /// `POST /workflows/changes/ask`: kept `drafting`, with the read an agent takes.
    pub fn ask_change(&mut self, manifest: &str, asking: AskWorkflowChange) -> Result<WorkflowChange, Refused> {
        // Fleet draws no change on Armada's own workflow today, and the draft over a shipped workflow needs one:
        // the mock answers it, as a Fleet that does will.
        let id = asking.workflow_id.as_str().to_string();
        let before = text_of(&self.held(&id, &asking.source, manifest)?.definition);
        if asking.text.trim().is_empty() {
            return Err(refused("fleet.workflow_change_empty", "Say what to change".into()));
        }
        let change = WorkflowChange {
            id: self.changes.iter().map(|one| one.change.id).max().unwrap_or(0) + 1,
            workflow_id: asking.workflow_id,
            source: asking.source,
            step_id: None,
            origin: ChangeOrigin::Asked { text: asking.text.trim().to_string() },
            before,
            after: None,
            state: ChangeState::Drafting,
            reason: None,
            missing: Vec::new(),
            before_steps: None,
            after_steps: None,
            after_name: None,
            after_for_requests: None,
            notes: None,
            at: Instant::carried("2026-11-12T09:00:00Z"),
        };
        self.changes.push(Kept { change: change.clone(), reads_left: 1, manifest: manifest.to_string() });
        Ok(change)
    }

    /// `GET /workflows/changes`: the open ones, each drafted once its read has passed.
    pub fn workflow_changes(&mut self, id: &str, source: &str, manifest: &str) -> Result<WorkflowChanges, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let mut changes = Vec::new();
        for kept in self.changes.iter_mut().filter(|kept| kept.change.workflow_id.as_str() == id && kept.change.source == source && (source != "repository" || kept.manifest == manifest)) {
            if kept.change.state == ChangeState::Drafting {
                if kept.reads_left == 0 {
                    kept.change = drafted(kept.change.clone());
                } else {
                    kept.reads_left -= 1;
                }
            }
            changes.push(kept.change.clone());
        }
        Ok(WorkflowChanges { changes })
    }

    fn kept(&self, change: i64) -> Result<usize, Refused> {
        self.changes.iter().position(|kept| kept.change.id == change).ok_or_else(|| refused("fleet.no_such_workflow_change", format!("No change {change}")))
    }

    /// `POST /workflows/changes/approve`: the draft written where the change began, if the file still holds `before`.
    pub fn approve_change(&mut self, change: i64) -> Result<WorkflowChangeApproved, Refused> {
        let at = self.kept(change)?;
        let Kept { change, manifest, .. } = self.changes[at].clone();
        let Some(after) = change.after.clone().filter(|_| change.state == ChangeState::Ready) else { return Err(refused("fleet.workflow_change_not_ready", "That change is not ready to approve".into())) };
        let id = change.workflow_id.as_str().to_string();
        let now = text_of(&self.held(&id, &change.source, &manifest)?.definition);
        if now != change.before {
            return Err(refused("fleet.workflow_changed_since", format!("{} changed since the draft was made. Dismiss the draft and ask again", file_of(&change.source, &id))));
        }
        let scope = if change.source == "repository" { WorkflowScope::Repository } else { WorkflowScope::Kit };
        let saved = self.save_workflow(&manifest, SaveWorkflow { scope, definition: after, overwrite: true })?;
        self.changes.remove(at);
        Ok(WorkflowChangeApproved { change: WorkflowChange { state: ChangeState::Approved, ..change }, saved })
    }

    /// `POST /workflows/changes/dismiss`.
    pub fn dismiss_change(&mut self, change: i64) -> Result<WorkflowChange, Refused> {
        let at = self.kept(change)?;
        let kept = self.changes.remove(at);
        Ok(WorkflowChange { state: ChangeState::Dismissed, ..kept.change })
    }

    /// `POST /workflows/changes/send_back`: drafted again with his notes.
    pub fn send_back_change(&mut self, back: SendBackWorkflowChange) -> Result<WorkflowChange, Refused> {
        let at = self.kept(back.change)?;
        if back.notes.trim().is_empty() {
            return Err(refused("fleet.workflow_change_empty", "Say what was wrong with the draft".into()));
        }
        let kept = &mut self.changes[at];
        let change = &mut kept.change;
        change.state = ChangeState::Drafting;
        change.notes = Some(back.notes.trim().to_string());
        change.after = None;
        change.reason = None;
        change.before_steps = None;
        change.after_steps = None;
        kept.reads_left = 1;
        Ok(change.clone())
    }

    /// `POST /workflows/changes/build`: a Job on Armada's repository, and the change it answers closes.
    pub fn build_support(&mut self, change: i64) -> Result<WorkflowSupportBuilding, Refused> {
        let at = self.kept(change)?;
        if self.changes[at].change.state != ChangeState::Cannot {
            return Err(refused("fleet.workflow_change_not_buildable", "Armada can already do what that change asks".into()));
        }
        self.changes.remove(at);
        Ok(WorkflowSupportBuilding { job_id: SessionId::carried(SUPPORT_JOB) })
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

    #[test]
    fn a_change_drafts_on_the_next_read_and_approving_it_writes_the_split_steps() {
        let mut fleet = MockFleet::new();
        let ask = |text: &str| AskWorkflowChange { workflow_id: WorkflowId::carried("feature"), source: "repository".into(), text: text.into() };
        let asked = fleet.ask_change("armada", ask("Split Refine into Wire and Tests")).expect("asked");
        let read = |fleet: &mut MockFleet| fleet.workflow_changes("feature", "repository", "armada").expect("read").changes.remove(0);
        assert_eq!(read(&mut fleet).state, ChangeState::Drafting);
        let ready = read(&mut fleet);
        assert_eq!(ready.state, ChangeState::Ready);
        let ids = |steps: &Option<Vec<ChangeStep>>| steps.iter().flatten().map(|one| one.id.as_str().to_string()).collect::<Vec<_>>();
        assert_eq!(ids(&ready.after_steps), ["propose", "walk", "wire", "tests", "deliver"]);
        fleet.approve_change(asked.id).expect("approved");
        let steps = fleet.workflow_steps("feature", "repository", "armada").expect("read").steps;
        assert_eq!(steps.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["propose", "walk", "wire", "tests", "deliver"]);
        assert!(fleet.workflow_changes("feature", "repository", "armada").expect("read").changes.is_empty());

        let cannot = fleet.ask_change("armada", ask("Propose should end when a walk, a sketch or a document is approved")).expect("asked");
        read(&mut fleet);
        assert_eq!((read(&mut fleet).state, fleet.approve_change(cannot.id).is_err()), (ChangeState::Cannot, true));
        assert_eq!(fleet.build_support(cannot.id).expect("started").job_id.as_str(), SUPPORT_JOB);
        assert!(fleet.ask_change("armada", ask("feature")).is_ok_and(|one| one.state == ChangeState::Drafting));
    }

    #[test]
    fn a_step_asked_for_after_another_is_drafted_on_armadas_own_workflow_and_nothing_is_written() {
        let mut fleet = MockFleet::new();
        let asking = AskWorkflowChange { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), text: "Add a Review step after Build".into() };
        fleet.ask_change("armada", asking).expect("asked");
        let read = |fleet: &mut MockFleet| fleet.workflow_changes("prototype", "armada", "armada").expect("read").changes.remove(0);
        read(&mut fleet);
        let ready = read(&mut fleet);
        assert_eq!(ready.state, ChangeState::Ready);
        let ids = |steps: &Option<Vec<ChangeStep>>| steps.iter().flatten().map(|one| one.id.as_str().to_string()).collect::<Vec<_>>();
        assert_eq!(ids(&ready.after_steps), ["propose", "build", "review", "show"]);
        assert_eq!(ids(&ready.before_steps), ["propose", "build", "show"]);
        let held = fleet.workflow_steps("prototype", "armada", "armada").expect("read").steps;
        assert_eq!(held.len(), 3, "Armada's own is not written");
    }
}
