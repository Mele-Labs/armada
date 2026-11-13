//! The Workflows page's routes on the walk's mock Fleet: a definition step by step, its text, a save, a copy reset
//! or deleted, and a proposal to Armada. They answer from the definitions in `mock-workflow-definitions.json`, the
//! ones Classic's Workflows page is walked on (`apps/desktop/src/renderer/src/mock/workflows-managed.ts`), and keep
//! the list `GET /workflows` answers in step with them, as Fleet reads the three places again before answering.
//! A change asked of an agent is `drafting` for one read and drafted on the next, as Classic's mock drafts it
//! (`apps/desktop/src/renderer/src/mock/workflows-fleet.ts`, `settled`).

use ipc::{BasedOn, DraftBy, DraftChange, RevertWorkflow, VersionOrigin, WorkflowVersion, WorkflowVersions, PieceDraft, PieceDraftDiscarded, PiecePublished, PublishPieceDraft, PublishWorkflowDraft, PutPieceDraft, PutWorkflowDraft, WorkflowDraft, WorkflowDraftDiscarded, WorkflowPublished, DeleteLifecyclePiece, LifecyclePiece, LifecyclePieceDeleted, LifecyclePieceKind, LifecyclePieceSaved, ArmadaProposed, AskWorkflowChange, ChangeOrigin, ChangeState, ChangeStep, Instant, OverriddenWorkflow, ProposeToArmada, SaveWorkflow, SendBackWorkflowChange, SessionId, StepId, StepReading, WorkflowChange, WorkflowChangeApproved, WorkflowChanges, WorkflowCopy, WorkflowCopyDeleted, WorkflowDefinition, WorkflowId, WorkflowSaved, WorkflowScope, WorkflowSteps, WorkflowSupportBuilding};
use serde::Deserialize;

use super::definition::{text_of, tree_of, with_words, Json};
use super::mock::MockFleet;
use super::wire::Refused;

/// One definition held in a place; a repository's under the Manifest of the repository it is in, and a project's
/// under the Manifest of the repository of the project it applies in.
#[derive(Clone, Debug, Deserialize)]
pub struct Held {
    pub source: String,
    #[serde(default)]
    pub manifest: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    pub definition: Json,
}

impl Held {
    fn id(&self) -> &str {
        self.definition.get("workflow_id").and_then(Json::text).unwrap_or_default()
    }

    fn holds(&self, id: &str, source: &str, manifest: &str) -> bool {
        self.id() == id && self.source == source && (!matches!(source, "repository" | "project") || self.manifest.as_deref() == Some(manifest))
    }

    fn file(&self) -> String {
        file_of(&self.source, self.project.as_deref(), self.id())
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

fn file_of(source: &str, project: Option<&str>, id: &str) -> String {
    match source {
        "armada" => format!("[armada]/{id}.yml"),
        "repository" => format!(".armada/workflows/{id}.yml"),
        "project" => format!("~/.armada/workflows/projects/{}/{id}.yml", project.unwrap_or_default()),
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

/// The drafts the mock keeps, as Fleet keeps them: one for each workflow in a place (a repository's by its Manifest too)
/// and one for each piece at a scope. Fleet numbers every change with the one counter.
#[derive(Clone, Debug, Default)]
pub struct Drafts {
    workflows: Vec<(String, WorkflowDraft)>,
    pieces: Vec<PieceDraft>,
    last: i64,
}

/// Where a workflow is kept: its source, the place's own key (a repository's Manifest, a project's id, none for Kit) and its id.
type At = (String, String, String);

fn at_of(source: &str, project: Option<&str>, manifest: &str, id: &str) -> At {
    let key = match source {
        "repository" => manifest,
        "project" => project.unwrap_or_default(),
        _ => "",
    };
    (source.to_string(), key.to_string(), id.to_string())
}

/// What a copy began from: the `version` Armada's text declared and the text.
#[derive(Clone, Debug)]
struct Base {
    version: u32,
    text: String,
}

/// The versions the mock keeps for each place, oldest first, and what each copy began from, as Fleet keeps them.
#[derive(Clone, Debug, Default)]
pub struct Versions {
    kept: Vec<(At, Vec<WorkflowVersion>)>,
    bases: Vec<(At, Base)>,
    /// How many were written, so each is stamped a minute after the one before.
    written: u32,
}

/// A stand-in for the sha256 Fleet computes: the same text always gives the same hex.
fn mock_hash(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3));
    format!("{hash:016x}")
}

/// The `version` a definition declares.
fn declared(text: &str) -> u32 {
    tree_of(text).and_then(|tree| match tree.get("version") {
        Some(Json::Number(n)) => n.as_u64().map(|n| n as u32),
        _ => None,
    }).unwrap_or(1)
}

impl Versions {
    /// The walk's scenario: armada's own copy of `feature` was made from an older Armada's, which had no Walk and said less
    /// of Refine, so Armada has changed it since.
    pub fn seeded(definitions: &[Held]) -> Versions {
        let mut versions = Versions::default();
        let Some(armada) = definitions.iter().find(|one| one.holds("feature", "armada", "armada")) else { return versions };
        let current = text_of(&armada.definition);
        let Some(older) = with_words(&current, "refine", "Wire it to the real thing.") else { return versions };
        let Some(Json::Map(mut fields)) = tree_of(&older) else { return versions };
        for (name, value) in fields.iter_mut() {
            if let (true, Json::List(steps)) = (name == "steps", value) {
                steps.retain(|one| one.get("id").and_then(Json::text) != Some("walk"));
            }
        }
        let text = text_of(&Json::Map(fields));
        versions.bases.push((at_of("repository", None, "armada", "feature"), Base { version: declared(&text), text }));
        versions
    }

    fn base_at(&self, at: &At) -> Option<&Base> {
        self.bases.iter().find(|(held, _)| held == at).map(|(_, base)| base)
    }

    fn set_base(&mut self, at: At, text: String) {
        let base = Base { version: declared(&text), text };
        match self.bases.iter_mut().find(|(held, _)| *held == at) {
            Some((_, held)) => *held = base,
            None => self.bases.push((at, base)),
        }
    }

    fn drop_place(&mut self, at: &At, base: bool) {
        self.kept.retain(|(held, _)| held != at);
        if base {
            self.bases.retain(|(held, _)| held != at);
        }
    }

    /// A publish into `at`: the text that was there is kept first where it is not the latest version, then the publish's own.
    /// The number the publish took.
    fn publish(&mut self, at: At, found: Option<String>, text: String) -> u32 {
        let at_index = match self.kept.iter().position(|(held, _)| *held == at) {
            Some(index) => index,
            None => {
                self.kept.push((at, Vec::new()));
                self.kept.len() - 1
            }
        };
        let mut rows = Vec::new();
        if let Some(found) = found.filter(|found| self.kept[at_index].1.last().is_none_or(|last| last.text != *found)) {
            rows.push((found, VersionOrigin::Found));
        }
        rows.push((text, VersionOrigin::Published));
        for (text, origin) in rows {
            self.written += 1;
            let at = Instant::carried(format!("2026-11-13T09:{:02}:00Z", 10 + self.written % 50));
            let held = &mut self.kept[at_index].1;
            held.push(WorkflowVersion { version: held.len() as u32 + 1, hash: mock_hash(&text), text, at, origin });
        }
        self.kept[at_index].1.len() as u32
    }
}

fn draft_refused(code: &str, said: &str) -> Refused {
    refused(code, said.to_string())
}

/// The sentence a text that would not load is given, as the loader says it.
fn unfit_workflow(id: &str, text: &str) -> Option<String> {
    let Some(tree) = tree_of(text) else { return Some("The definition is not a workflow".into()) };
    (tree.get("workflow_id").and_then(Json::text) != Some(id)).then(|| format!("The definition declares another workflow than `{id}`"))
}

/// What Armada lacks for "Propose should end when a walk, a sketch or a document is approved" (`CANNOT_YET`).
const CANNOT_YET: [&str; 3] = ["A step ends on one thing, not on any of several", "Nothing ends a step when a sketch is approved", "Nothing ends a step when a document is approved"];

/// The sentence an agent adds to Walk's words for "Walk should say why when it skips the walk" (`SAY_WHY_SKIPPED`).
const SAY_WHY_SKIPPED: &str = "Where nothing would be seen, say so and why before going on.";

/// The steps an agent comes back with for "split Refine into Wire and Tests" (`WIRE_AND_TESTS`).
const WIRE_AND_TESTS: [(&str, &str, &str); 2] = [("wire", "Wire", "Wire it to the real thing. Show a new walk only if the look changed."), ("tests", "Tests", "Write the tests now, against what the walk showed.")];

/// The Job a press on Start a job to build it starts.
const SUPPORT_JOB: &str = "01SESSIONSUPPORTAAAAAAAAAA";

/// The place a copy's source names, a project's with its id.
fn scope_of(source: &str, project: Option<String>) -> WorkflowScope {
    match (source, project) {
        ("repository", _) => WorkflowScope::Repository,
        ("project", Some(id)) => WorkflowScope::Project { id },
        _ => WorkflowScope::Kit,
    }
}

fn refused(code: &str, said: String) -> Refused {
    Refused { key: None, said, code: Some(code.to_string()) }
}

fn place_said(source: &str) -> &'static str {
    match source {
        "armada" => "Armada",
        "repository" => "This repository",
        "project" => "This project",
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
        Ok(WorkflowDefinition { workflow_id: WorkflowId::carried(id), source: source.into(), project: held.project.clone(), file: held.file(), definition: text_of(&held.definition), overridden_by: None, based_on: None })
    }

    pub fn save_workflow(&mut self, manifest: &str, save: SaveWorkflow) -> Result<WorkflowSaved, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let tree = tree_of(&save.definition).ok_or_else(|| refused("fleet.workflow_unfit", "The definition is not a workflow".into()))?;
        let id = tree.get("workflow_id").and_then(Json::text).unwrap_or_default().to_string();
        let source = save.scope.as_wire();
        if let Some(project) = save.scope.project() {
            let Some(one) = self.projects.projects.iter().find(|one| one.id == project) else {
                return Err(refused("fleet.workflow_project_unknown", format!("There is no project `{project}`")));
            };
            if !one.members.iter().any(|member| member.manifest_id.as_str() == manifest) {
                return Err(refused("fleet.workflow_project_not_member", format!("{manifest} is not in {}", one.name)));
            }
        }
        let at = self.definitions.iter().position(|one| one.holds(&id, source, manifest));
        match at {
            Some(_) if !save.overwrite => return Err(refused("fleet.workflow_exists", format!("{} already holds a workflow `{id}`", place_said(source)))),
            Some(at) => self.definitions[at].definition = tree,
            None => self.definitions.push(Held { source: source.into(), manifest: matches!(source, "repository" | "project").then(|| manifest.to_string()), project: save.scope.project().map(String::from), definition: tree }),
        }
        // A copy of Armada's text, byte for byte, into a place that has no base is what a copy is made from, so it is the base.
        let place = at_of(source, save.scope.project(), manifest, &id);
        if self.versions.base_at(&place).is_none() {
            if let Some(armada) = self.definitions.iter().find(|one| one.holds(&id, "armada", manifest)).filter(|armada| Some(&armada.definition) == tree_of(&save.definition).as_ref()) {
                let text = text_of(&armada.definition);
                self.versions.set_base(place, text);
            }
        }
        self.relist(&id);
        Ok(WorkflowSaved { workflow_id: WorkflowId::carried(&id), file: file_of(source, save.scope.project(), &id), scope: save.scope, replaced: at.is_some(), runs_from: self.runs_from(&id, manifest) })
    }

    pub fn reset_workflow(&mut self, manifest: &str, copy: WorkflowCopy) -> Result<WorkflowSaved, Refused> {
        let id = copy.workflow_id.as_str().to_string();
        let armada = self.held(&id, "armada", manifest)?.definition.clone();
        let at = self.definitions.iter().position(|one| one.holds(&id, &copy.source, manifest)).ok_or_else(|| refused("fleet.workflow_not_found", format!("{} holds no workflow `{id}`", place_said(&copy.source))))?;
        let place = at_of(&copy.source, copy.project.as_deref(), manifest, &id);
        self.versions.drop_place(&place, true);
        self.versions.set_base(place, text_of(&armada));
        self.definitions[at].definition = armada;
        self.relist(&id);
        let file = self.definitions[at].file();
        let scope = scope_of(&copy.source, copy.project.clone());
        Ok(WorkflowSaved { workflow_id: copy.workflow_id, scope, file, replaced: true, runs_from: self.runs_from(&id, manifest) })
    }

    pub fn delete_workflow(&mut self, manifest: &str, copy: WorkflowCopy) -> Result<WorkflowCopyDeleted, Refused> {
        let id = copy.workflow_id.as_str().to_string();
        if copy.source == "armada" {
            return Err(refused("fleet.workflow_armada", "Armada's own workflow is not a copy, and is not deleted".into()));
        }
        let file = self.held(&id, &copy.source, manifest)?.file();
        self.definitions.retain(|one| !one.holds(&id, &copy.source, manifest));
        self.versions.drop_place(&at_of(&copy.source, copy.project.as_deref(), manifest, &id), true);
        self.relist(&id);
        Ok(WorkflowCopyDeleted { workflow_id: copy.workflow_id, file, source: copy.source, project: copy.project, runs_from: self.runs_from(&id, manifest) })
    }

    pub fn propose_workflow(&mut self, proposing: ProposeToArmada) -> Result<ArmadaProposed, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        Ok(ArmadaProposed { workflow_id: proposing.workflow_id, session_id: SessionId::carried("01SESSIONPROPOSEAAAAAAAAAA"), send_failed: None })
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
        let published = text_of(&self.held(&id, &asking.source, manifest)?.definition);
        let before = self.draft_of(&id, &asking.source, asking.project.as_deref(), manifest).map_or(published, |draft| draft.text.clone());
        if asking.text.trim().is_empty() {
            return Err(refused("fleet.workflow_change_empty", "Say what to change".into()));
        }
        let change = WorkflowChange {
            id: self.changes.iter().map(|one| one.change.id).max().unwrap_or(0) + 1,
            workflow_id: asking.workflow_id,
            source: asking.source,
            project: None,
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
            drafted: false,
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
        let at: Vec<usize> = (0..self.changes.len()).filter(|at| self.changes[*at].change.workflow_id.as_str() == id && self.changes[*at].change.source == source && (source != "repository" || self.changes[*at].manifest == manifest)).collect();
        for at in at {
            let kept = &mut self.changes[at];
            if kept.change.state == ChangeState::Drafting {
                if kept.reads_left == 0 {
                    kept.change = drafted(kept.change.clone());
                    let landing = kept.change.clone();
                    let manifest = kept.manifest.clone();
                    if self.lands(&landing, &manifest) {
                        self.changes[at].change.drafted = true;
                    }
                } else {
                    kept.reads_left -= 1;
                }
            }
            changes.push(self.changes[at].change.clone());
        }
        Ok(WorkflowChanges { changes })
    }

    /// An asked change that is ready goes into the draft as the agent's, where the draft (or, with none, what is
    /// published) still reads as the change was made from: then it begins the draft.
    fn lands(&mut self, change: &WorkflowChange, manifest: &str) -> bool {
        let (ChangeOrigin::Asked { text: asked }, Some(after), ChangeState::Ready) = (&change.origin, change.after.clone(), change.state) else { return false };
        let (id, source, project) = (change.workflow_id.as_str().to_string(), change.source.clone(), change.project.clone());
        let now = self.draft_of(&id, &source, project.as_deref(), manifest).map(|draft| draft.text.clone()).or_else(|| self.held(&id, &source, manifest).ok().map(|held| text_of(&held.definition)));
        if now.as_deref() != Some(change.before.as_str()) {
            return false;
        }
        self.add_draft_change(&id, &source, project.as_deref(), manifest, after, DraftBy::Agent, Some(asked.clone()), Some(change.id));
        true
    }

    fn draft_key(source: &str, manifest: &str) -> String {
        if source == "repository" { manifest.to_string() } else { String::new() }
    }

    fn draft_of(&self, id: &str, source: &str, project: Option<&str>, manifest: &str) -> Option<&WorkflowDraft> {
        let key = Self::draft_key(source, manifest);
        self.drafts.workflows.iter().find(|(by, one)| *by == key && one.workflow_id.as_str() == id && one.source == source && one.project.as_deref() == project).map(|(_, one)| one)
    }

    /// Begins the draft from what is published in the place where there is none, and adds one change to it. A text the
    /// draft already ends on adds nothing.
    #[allow(clippy::too_many_arguments)]
    fn add_draft_change(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str, text: String, by: DraftBy, summary: Option<String>, change: Option<i64>) -> WorkflowDraft {
        let key = Self::draft_key(source, manifest);
        let at = self.drafts.workflows.iter().position(|(by, one)| *by == key && one.workflow_id.as_str() == id && one.source == source && one.project.as_deref() == project);
        let now = Instant::carried("2026-11-13T09:00:00Z");
        let at = at.unwrap_or_else(|| {
            let base = self.held(id, source, manifest).map(|held| text_of(&held.definition)).unwrap_or_default();
            let begun = WorkflowDraft { workflow_id: WorkflowId::carried(id), source: source.into(), project: project.map(String::from), text: base.clone(), base, unfit: None, changes: Vec::new(), started_at: now.clone(), updated_at: now.clone() };
            self.drafts.workflows.push((key, begun));
            self.drafts.workflows.len() - 1
        });
        let one = &mut self.drafts.workflows[at].1;
        if one.changes.last().map(|last| last.text.as_str()) != Some(text.as_str()) {
            self.drafts.last += 1;
            one.changes.push(DraftChange { id: self.drafts.last, by, at: now.clone(), text: text.clone(), summary, change });
        }
        one.unfit = unfit_workflow(id, &text);
        one.text = text;
        one.updated_at = now;
        one.clone()
    }

    /// `GET /workflows/draft`.
    pub fn workflow_draft(&self, id: &str, source: &str, project: Option<&str>, manifest: &str) -> Result<WorkflowDraft, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        self.draft_of(id, source, project, manifest).cloned().ok_or_else(|| draft_refused("fleet.workflow_draft_none", "No draft is kept for that workflow"))
    }

    /// `PUT /workflows/draft`.
    pub fn put_workflow_draft(&mut self, manifest: &str, put: PutWorkflowDraft) -> Result<WorkflowDraft, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let id = put.workflow_id.as_str().to_string();
        if put.source == "project" && put.project.is_none() {
            return Err(draft_refused("fleet.workflow_project_unnamed", "Say which project"));
        }
        if self.draft_of(&id, &put.source, put.project.as_deref(), manifest).is_none() && put.source == "armada" {
            self.held(&id, "armada", manifest).map_err(|_| draft_refused("fleet.no_such_workflow_definition", "Armada carries no such workflow"))?;
        }
        Ok(self.add_draft_change(&id, &put.source, put.project.as_deref(), manifest, put.text, DraftBy::You, put.summary, None))
    }

    /// `DELETE /workflows/draft`.
    pub fn discard_workflow_draft(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str) -> Result<WorkflowDraftDiscarded, Refused> {
        self.workflow_draft(id, source, project, manifest)?;
        let key = Self::draft_key(source, manifest);
        self.drafts.workflows.retain(|(by, one)| !(*by == key && one.workflow_id.as_str() == id && one.source == source && one.project.as_deref() == project));
        Ok(WorkflowDraftDiscarded { workflow_id: WorkflowId::carried(id), source: source.into(), project: project.map(String::from) })
    }

    /// `POST /workflows/draft/publish`: written where it applies, if what the place holds is still what it began from.
    pub fn publish_workflow_draft(&mut self, manifest: &str, publish: PublishWorkflowDraft) -> Result<WorkflowPublished, Refused> {
        let id = publish.workflow_id.as_str().to_string();
        let draft = self.workflow_draft(&id, &publish.source, publish.project.as_deref(), manifest)?;
        let now = self.held(&id, &publish.source, manifest).map(|held| text_of(&held.definition)).unwrap_or_default();
        if now != draft.base {
            return Err(draft_refused("fleet.workflow_draft_stale", "This workflow changed since the draft was started"));
        }
        let armada = publish.source == "armada";
        let scope = match (&publish.scope, armada) {
            (Some(scope), true) => scope.clone(),
            (None, true) => return Err(draft_refused("fleet.workflow_draft_scope_needed", "Say where this applies")),
            (_, false) => scope_of(&publish.source, publish.project.clone()),
        };
        let exists = armada && self.definitions.iter().any(|one| one.holds(&id, scope.as_wire(), manifest));
        if exists && !publish.overwrite {
            return Err(refused("fleet.workflow_exists", format!("{} already holds a workflow `{id}`", place_said(scope.as_wire()))));
        }
        let place = at_of(scope.as_wire(), scope.project(), manifest, &id);
        let found = self.definitions.iter().find(|one| one.holds(&id, scope.as_wire(), manifest)).map(|one| text_of(&one.definition));
        let saved = self.save_workflow(manifest, SaveWorkflow { scope, definition: draft.text.clone(), overwrite: !armada || publish.overwrite })?;
        self.drafts.workflows.retain(|(_, one)| !(one.workflow_id.as_str() == id && one.source == publish.source && one.project == publish.project));
        // A publish of Armada's own begins a base: the carried text the draft began from.
        if armada {
            self.versions.set_base(place.clone(), draft.base.clone());
        }
        let version = self.versions.publish(place, found, draft.text);
        self.relist(&id);
        Ok(WorkflowPublished { saved, version })
    }

    /// The scenario's base, and the list read again so the copy that began from it says Armada changed it.
    pub(super) fn seed_versions(&mut self) {
        self.versions = Versions::seeded(&self.definitions);
        self.relist("feature");
    }

    /// `GET /workflows/versions`: newest first.
    pub fn workflow_versions(&self, id: &str, source: &str, project: Option<&str>, manifest: &str) -> Result<WorkflowVersions, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        if source == "project" && project.is_none() {
            return Err(draft_refused("fleet.workflow_project_unnamed", "Say which project"));
        }
        let at = at_of(source, project, manifest, id);
        let mut versions = self.versions.kept.iter().find(|(held, _)| *held == at).map(|(_, rows)| rows.clone()).unwrap_or_default();
        versions.reverse();
        Ok(WorkflowVersions { versions })
    }

    /// `POST /workflows/revert`: the version's text goes into the draft as a change of his, and nothing is published.
    pub fn revert_workflow(&mut self, manifest: &str, revert: RevertWorkflow) -> Result<WorkflowDraft, Refused> {
        let id = revert.workflow_id.as_str().to_string();
        let held = self.workflow_versions(&id, &revert.source, revert.project.as_deref(), manifest)?;
        let Some(version) = held.versions.into_iter().find(|one| one.version == revert.version) else {
            return Err(draft_refused("fleet.workflow_version_none", &format!("There is no version {} of `{id}` here", revert.version)));
        };
        Ok(self.add_draft_change(&id, &revert.source, revert.project.as_deref(), manifest, version.text, DraftBy::You, Some(format!("Back to version {}", revert.version)), None))
    }

    /// `GET /workflows/definition` with `source=base`: what the copy at `of` began from.
    pub fn workflow_base(&self, id: &str, of: &str, project: Option<&str>, manifest: &str) -> Result<WorkflowDefinition, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let base = self.versions.base_at(&at_of(of, project, manifest, id)).ok_or_else(|| draft_refused("fleet.workflow_base_none", "This copy has no base"))?;
        Ok(WorkflowDefinition { workflow_id: WorkflowId::carried(id), source: "base".into(), project: project.map(String::from), file: file_of("armada", None, id), definition: base.text.clone(), overridden_by: None, based_on: Some(BasedOn { version: base.version, hash: mock_hash(&base.text) }) })
    }

    fn piece_at(&self, kind: LifecyclePieceKind, id: &str, scope: &ipc::PieceScope) -> Option<usize> {
        self.drafts.pieces.iter().position(|one| one.kind == kind && one.id == id && one.scope == *scope)
    }

    /// `GET /lifecycles/pieces/draft`.
    pub fn piece_draft(&self, kind: LifecyclePieceKind, id: &str, scope: &ipc::PieceScope) -> Result<PieceDraft, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        self.piece_at(kind, id, scope).map(|at| self.drafts.pieces[at].clone()).ok_or_else(|| draft_refused("fleet.piece_draft_none", "No draft is kept for that piece"))
    }

    /// `PUT /lifecycles/pieces/draft`.
    pub fn put_piece_draft(&mut self, put: PutPieceDraft) -> Result<PieceDraft, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let now = Instant::carried("2026-11-13T09:00:00Z");
        let at = self.piece_at(put.kind, &put.id, &put.scope).unwrap_or_else(|| {
            self.drafts.pieces.push(PieceDraft { kind: put.kind, id: put.id.clone(), scope: put.scope.clone(), text: String::new(), base: String::new(), unfit: None, changes: Vec::new(), started_at: now.clone(), updated_at: now.clone() });
            self.drafts.pieces.len() - 1
        });
        let one = &mut self.drafts.pieces[at];
        if one.changes.last().map(|last| last.text.as_str()) != Some(put.text.as_str()) {
            self.drafts.last += 1;
            one.changes.push(DraftChange { id: self.drafts.last, by: DraftBy::You, at: now.clone(), text: put.text.clone(), summary: put.summary, change: None });
        }
        one.unfit = ipc::decode::<LifecyclePiece>("piece", put.text.as_bytes()).err().map(|why| why.to_string());
        one.text = put.text;
        one.updated_at = now;
        Ok(one.clone())
    }

    /// `DELETE /lifecycles/pieces/draft`.
    pub fn discard_piece_draft(&mut self, kind: LifecyclePieceKind, id: &str, scope: &ipc::PieceScope) -> Result<PieceDraftDiscarded, Refused> {
        let at = self.piece_at(kind, id, scope).ok_or_else(|| draft_refused("fleet.piece_draft_none", "No draft is kept for that piece"))?;
        self.drafts.pieces.remove(at);
        Ok(PieceDraftDiscarded { kind, id: id.to_string(), scope: scope.clone() })
    }

    /// `POST /lifecycles/pieces/draft/publish`.
    pub fn publish_piece_draft(&mut self, publish: PublishPieceDraft) -> Result<PiecePublished, Refused> {
        let draft = self.piece_draft(publish.kind, &publish.id, &publish.scope)?;
        if let Some(why) = draft.unfit {
            return Err(refused("fleet.piece_unfit", why));
        }
        let at = self.piece_at(publish.kind, &publish.id, &publish.scope).ok_or_else(|| draft_refused("fleet.piece_draft_none", "No draft is kept for that piece"))?;
        self.drafts.pieces.remove(at);
        let saved = LifecyclePieceSaved { kind: publish.kind, file: format!("~/.armada/kit/{}s/{}.yml", publish.kind.as_wire(), publish.id), id: publish.id, scope: publish.scope, replaced: true };
        Ok(PiecePublished { saved })
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
        let held = self.held(&id, &change.source, &manifest)?;
        let (now, file, project) = (text_of(&held.definition), held.file(), held.project.clone());
        if now != change.before {
            return Err(refused("fleet.workflow_changed_since", format!("{file} changed since the draft was made. Dismiss the draft and ask again")));
        }
        let scope = scope_of(&change.source, project);
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

    /// The definitions of an id that apply in a repository, the one that runs first: its project's, Kit's, the
    /// repository's, then Armada's.
    fn applying(&self, id: &str, manifest: &str) -> Vec<&Held> {
        ["project", "kit", "repository", "armada"].iter().filter_map(|source| self.definitions.iter().find(|one| one.holds(id, source, manifest))).collect()
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
            row.project = runs.project.clone();
            row.file = runs.file();
            row.for_requests = runs.definition.get("for_requests").and_then(Json::text).map(String::from);
            let base = (runs.source != "armada").then(|| self.versions.base_at(&at_of(&runs.source, runs.project.as_deref(), &manifest, id))).flatten();
            let armada = self.definitions.iter().find(|one| one.holds(id, "armada", &manifest)).map(|one| text_of(&one.definition));
            row.based_on = base.map(|base| BasedOn { version: base.version, hash: mock_hash(&base.text) });
            row.armada_changed = base.zip(armada).is_some_and(|(base, armada)| base.text != armada);
            row.overrides = applying[1..].iter().map(|one| OverriddenWorkflow { source: one.source.clone(), file: one.file(), project: one.project.clone() }).collect();
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
        WorkflowCopy { workflow_id: WorkflowId::carried(id), source: source.into(), file: None, ..Default::default() }
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
    fn a_projects_copy_runs_ahead_of_kits_and_a_project_the_repository_is_not_in_is_refused() {
        let mut fleet = MockFleet::new();
        let text = fleet.workflow_definition("prototype", "armada", "armada").expect("Armada's prototype").definition;
        fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Kit, definition: text.clone(), overwrite: false }).expect("Kit's");
        let saved = fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Project { id: "armada".into() }, definition: text.clone(), overwrite: false }).expect("Armada's project");
        assert_eq!(saved.file, "~/.armada/workflows/projects/armada/prototype.yml");
        let row = fleet.workflows.iter().find(|one| one.id.as_str() == "prototype").expect("listed");
        assert_eq!((row.source.as_str(), row.project.as_deref(), row.overrides.iter().map(|one| one.source.as_str()).collect::<Vec<_>>()), ("project", Some("armada"), vec!["kit", "armada"]));
        let elsewhere = fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Project { id: "storefront".into() }, definition: text, overwrite: false });
        assert!(elsewhere.is_err_and(|why| why.is("fleet.workflow_project_not_member")));
        let gone = fleet.delete_workflow("armada", WorkflowCopy { project: Some("armada".into()), ..copy("prototype", "project") }).expect("deleted");
        assert_eq!((gone.project.as_deref(), gone.runs_from.as_deref()), (Some("armada"), Some("kit")));
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
        let ask = |text: &str| AskWorkflowChange { workflow_id: WorkflowId::carried("feature"), source: "repository".into(), text: text.into(), ..Default::default() };
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
        let asking = AskWorkflowChange { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), text: "Add a Review step after Build".into(), project: None };
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

    #[test]
    fn a_draft_is_kept_for_each_place_changes_by_put_and_is_published_where_armada_says() {
        let mut fleet = MockFleet::new();
        let text = fleet.workflow_definition("prototype", "armada", "armada").expect("Armada's prototype").definition;
        let put = |text: &str| PutWorkflowDraft { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), text: text.into(), ..Default::default() };
        assert!(fleet.workflow_draft("prototype", "armada", None, "armada").is_err_and(|why| why.is("fleet.workflow_draft_none")));
        let other = text.replace("Propose", "Think");
        let first = fleet.put_workflow_draft("armada", put(&other)).expect("put");
        assert_eq!((first.base.as_str(), first.changes.len()), (text.as_str(), 1));
        assert_eq!(fleet.put_workflow_draft("armada", put(&other)).expect("put").changes.len(), 1, "the text it ends on adds nothing");
        assert_eq!(fleet.put_workflow_draft("armada", put(&text)).expect("put").changes.len(), 2, "an earlier text is a change");
        // Armada's own is told where; a copy is new.
        let publish = |scope: Option<WorkflowScope>| PublishWorkflowDraft { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), scope, ..Default::default() };
        assert!(fleet.publish_workflow_draft("armada", publish(None)).is_err_and(|why| why.is("fleet.workflow_draft_scope_needed")));
        let saved = fleet.publish_workflow_draft("armada", publish(Some(WorkflowScope::Kit))).expect("published").saved;
        assert_eq!(saved.scope, WorkflowScope::Kit);
        assert!(fleet.workflow_draft("prototype", "armada", None, "armada").is_err(), "the draft goes when it is written");
        // A draft over a copy is stale once the copy moved under it.
        fleet.put_workflow_draft("armada", PutWorkflowDraft { source: "kit".into(), ..put(&other) }).expect("Kit's draft");
        fleet.save_workflow("armada", SaveWorkflow { scope: WorkflowScope::Kit, definition: other.replace("Think", "Ponder"), overwrite: true }).expect("saved");
        let stale = fleet.publish_workflow_draft("armada", PublishWorkflowDraft { source: "kit".into(), ..publish(None) });
        assert!(stale.is_err_and(|why| why.is("fleet.workflow_draft_stale")));
        assert!(fleet.discard_workflow_draft("prototype", "kit", None, "armada").is_ok());
    }

    #[test]
    fn an_agents_ready_change_lands_in_the_draft_as_the_agents_only_where_the_draft_still_reads_as_it_was_made_from() {
        let mut fleet = MockFleet::new();
        let asking = |text: &str| AskWorkflowChange { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), text: text.into(), project: None };
        fleet.ask_change("armada", asking("Add a Review step after Build")).expect("asked");
        let read = |fleet: &mut MockFleet| fleet.workflow_changes("prototype", "armada", "armada").expect("read").changes.remove(0);
        read(&mut fleet);
        let ready = read(&mut fleet);
        assert_eq!((ready.state, ready.drafted), (ChangeState::Ready, true));
        let draft = fleet.workflow_draft("prototype", "armada", None, "armada").expect("begun by the change");
        assert_eq!((draft.changes[0].by, draft.changes[0].summary.as_deref(), draft.changes[0].change), (DraftBy::Agent, Some("Add a Review step after Build"), Some(ready.id)));
        // Asked of a draft that has since moved, the change is ready and not drafted.
        fleet.ask_change("armada", asking("Add a Notes step after Build")).expect("asked");
        let moved = draft.text.replace("Propose", "Think");
        fleet.put_workflow_draft("armada", PutWorkflowDraft { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), text: moved, ..Default::default() }).expect("put");
        let find = |fleet: &mut MockFleet| fleet.workflow_changes("prototype", "armada", "armada").expect("read").changes.into_iter().find(|one| one.id != ready.id).expect("the second");
        assert_eq!(find(&mut fleet).state, ChangeState::Drafting);
        let second = find(&mut fleet);
        assert_eq!((second.state, second.drafted), (ChangeState::Ready, false), "the draft moved since it was asked, so the change stays ready");
    }
}
