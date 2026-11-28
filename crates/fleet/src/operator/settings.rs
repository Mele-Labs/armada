//! What each project's Operator is set to. Kept as learned values of the project (`project_learned`), each absent
//! where he has not set it, and resolved here: his own value, else the role's, else the default. The values are
//! the project's and never machine settings, so `crates/config/settings.toml` has no row for them.
//! `crates/ipc/operations/set_operator_settings.toml`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SetupKind, Vcs, WhatWasRead, WorkProduct};
use api::Refusal;
use config::settings as keys;
use ipc::{
    ManifestId, OperatorActs, OperatorDecisions, OperatorRole, OperatorSettings, OperatorSettingsList, OperatorSettingsSet, OperatorToolsSet,
    ProjectOperatorSettings, SessionId, SetOperatorSettings, StartSession,
};

use crate::daemon::Fleet;
use crate::project_learned::OPERATOR_ACTS_KEY;

const ON: &str = "operator.on";
const MODEL: &str = "operator.model";
const EFFORT: &str = "operator.effort";
const ROLE: &str = "operator.role";
const WORDS: &str = "operator.words";
const PROMPT: &str = "operator.prompt";
const STYLE: &str = "operator.style";
const START_JOBS: &str = "operator.tool.start_jobs";
const MERGE: &str = "operator.tool.merge";
const END_JOBS: &str = "operator.tool.end_jobs";
const FILE_ISSUES: &str = "operator.tool.file_issues";
const DECISIONS: &str = "operator.decisions";
const NAME: &str = "operator.name";
const ACCENT: &str = "operator.accent";

/// What an Operator is called where he has not named it.
pub(crate) const DEFAULT_NAME: &str = "Operator";

/// The refusal for a value that is none of the project's to keep.
const INVALID: &str = "fleet.operator_setting_invalid";

/// The `unset` names, each with the keys it clears.
const UNSET: [(&str, &[&str]); 16] = [
    ("on", &[ON]),
    ("model", &[MODEL]),
    ("effort", &[EFFORT]),
    ("role", &[ROLE]),
    ("words", &[WORDS]),
    ("prompt", &[PROMPT]),
    ("output_style", &[STYLE]),
    ("decisions", &[DECISIONS]),
    ("name", &[NAME]),
    ("accent", &[ACCENT]),
    ("acts", &[OPERATOR_ACTS_KEY]),
    ("tools", &[START_JOBS, MERGE, END_JOBS, FILE_ISSUES]),
    ("tools.start_jobs", &[START_JOBS]),
    ("tools.merge", &[MERGE]),
    ("tools.end_jobs", &[END_JOBS]),
    ("tools.file_issues", &[FILE_ISSUES]),
];

/// What `project` has kept, each value absent where none is.
fn set_read(store: &store::Store, project: &str) -> OperatorSettingsSet {
    let word = |key: &str| store.project_learned(project, key).ok().flatten();
    let flag = |key: &str| word(key).and_then(|word| match word.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    });
    let tools = OperatorToolsSet { start_jobs: flag(START_JOBS), merge: flag(MERGE), end_jobs: flag(END_JOBS), file_issues: flag(FILE_ISSUES) };
    OperatorSettingsSet {
        on: flag(ON),
        model: word(MODEL),
        effort: word(EFFORT),
        role: word(ROLE).and_then(|word| OperatorRole::from_wire(&word)),
        words: word(WORDS),
        prompt: word(PROMPT),
        output_style: word(STYLE),
        tools: (tools != OperatorToolsSet::default()).then_some(tools),
        decisions: word(DECISIONS).and_then(|word| OperatorDecisions::from_wire(&word)),
        name: word(NAME),
        accent: word(ACCENT),
        acts: word(OPERATOR_ACTS_KEY).and_then(|word| OperatorActs::from_wire(&word)),
    }
}

/// `set` resolved: his value where he set one, else the role's, else the default. `model` and `effort` are the machine's
/// own Operator settings, which a project that sets none follows.
fn resolved(set: &OperatorSettingsSet, model: String, effort: String) -> OperatorSettings {
    let role = set.role.unwrap_or_default();
    let said = |value: &Option<String>| value.clone().filter(|word| !word.trim().is_empty());
    OperatorSettings {
        on: set.on.unwrap_or(true),
        model: said(&set.model).unwrap_or(model),
        effort: said(&set.effort).unwrap_or(effort),
        role,
        words: set.words.clone().unwrap_or_default(),
        prompt: set.prompt.clone().unwrap_or_default(),
        output_style: said(&set.output_style).unwrap_or_else(|| adapters::OPERATOR_STYLE.to_string()),
        tools: set.tools.unwrap_or_default().over(role),
        decisions: set.decisions.unwrap_or_default(),
        name: said(&set.name).unwrap_or_else(|| DEFAULT_NAME.to_string()),
        accent: set.accent.clone(),
        acts: set.acts,
    }
}

/// Whether `accent` is `#rrggbb`.
fn accent_is_hex(accent: &str) -> bool {
    accent.len() == 7 && accent.starts_with('#') && accent[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// What he set for `project`, and nothing else.
    async fn operator_set_of(&self, project: &str) -> OperatorSettingsSet {
        set_read(&*self.store().lock().await, project)
    }

    /// `project`'s Operator as it is now, every value resolved. **What the Operator's door, gate and start read**, so a
    /// value he changed reaches the next call.
    pub(crate) async fn operator_settings_of(&self, project: &str) -> OperatorSettings {
        let set = self.operator_set_of(project).await;
        let (model, effort): (String, String) = {
            let now = self.machine_settings().now();
            (now.get(keys::MODELS_OPERATOR), now.get(keys::EFFORT_OPERATOR))
        };
        resolved(&set, model, effort)
    }

    /// What `project`'s Operator is started with beyond the model: his words, its role, his addition and its style.
    pub(crate) async fn operator_prompt_of(&self, project: &str) -> adapters::OperatorPrompt {
        let settings = self.operator_settings_of(project).await;
        adapters::OperatorPrompt { words: settings.words, role: settings.role, extra: settings.prompt, style: settings.output_style }
    }

    /// The start of `project`'s Operator, on the model and effort its settings give. **The one start both a first
    /// session and its successor take.**
    pub(crate) async fn operator_start_of(&self, project: &str, title: Option<String>) -> StartSession {
        let settings = self.operator_settings_of(project).await;
        StartSession {
            project: Some(project.to_string()),
            manifest_id: ManifestId::carried(""),
            title,
            model: Some(settings.model).filter(|model| !model.trim().is_empty()),
            // "harness default" is no effort named: the harness picks.
            effort: Some(settings.effort).filter(|effort| crate::tuning::effort_of(effort).is_some()),
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            workflow: None,
        }
    }

    /// `project` as one of his own, or the refusal that says there is none.
    pub(crate) async fn operator_project_known(&self, project: &str) -> Result<String, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        match kept.projects.iter().any(|one| one.id == project) {
            true => Ok(project.to_string()),
            false => Err(Refusal::NotFound(ipc::WireError::raised(crate::projects::NO_SUCH_PROJECT, format!("there is no project {project}"), self.run_id()))),
        }
    }

    /// The output styles he may pick: the mod's `armada:Operator` first, then each one the harness finds.
    async fn operator_styles(&self) -> Vec<String> {
        let setup = self.setup();
        let read = tokio::task::spawn_blocking(move || setup.read()).await;
        let mut styles = vec![adapters::OPERATOR_STYLE.to_string()];
        let Ok(inventory) = read else { return styles };
        if let Some(WhatWasRead::Read { items, .. }) = inventory.kind(SetupKind::OutputStyles) {
            for item in items {
                let name = match &item.plugin {
                    Some(plugin) => format!("{plugin}:{}", item.name),
                    None => item.name.clone(),
                };
                if !styles.contains(&name) {
                    styles.push(name);
                }
            }
        }
        styles
    }

    /// `list_operator_settings`: each project of his own, and the styles.
    pub(crate) async fn operator_settings_listed(&self) -> Result<OperatorSettingsList, Refusal> {
        Ok(OperatorSettingsList { projects: self.operator_settings_projects().await, styles: self.operator_styles().await })
    }

    /// Each project of his own with its Operator's settings and session, as the page carries them.
    pub(crate) async fn operator_settings_projects(&self) -> Vec<ProjectOperatorSettings> {
        let kept = self.store().lock().await.projects().map(|kept| kept.projects).unwrap_or_default();
        let mut projects = Vec::new();
        for one in kept {
            let set = self.operator_set_of(&one.id).await;
            let settings = self.operator_settings_of(&one.id).await;
            let session_id = self.operator_session_id(&one.id).await.map(|id| SessionId::carried(&id));
            projects.push(ProjectOperatorSettings { project: one.id, project_name: one.name, settings, set, session_id });
        }
        projects
    }

    /// `set_operator_settings`. Checked whole before anything is kept. A change to what the session starts with hands a live
    /// one on at once; a change to `decisions` tells it in a line.
    pub(crate) async fn operator_settings_set(self: &Arc<Self>, change: SetOperatorSettings) -> Result<OperatorSettingsList, Refusal> {
        let project = self.operator_project_known(&change.project).await?;
        let unset = change.unset.iter().map(|name| self.operator_unset_keys(name)).collect::<Result<Vec<_>, _>>()?;
        let kept = self.operator_kept_words(&change.set)?;
        let before = self.operator_settings_of(&project).await;
        {
            let at = self.now();
            let mut store = self.store().lock().await;
            for key in unset.into_iter().flatten().copied().chain(kept.iter().filter(|(_, word)| word.is_empty()).map(|(key, _)| *key)) {
                store.clear_project_learned(&project, key).map_err(|why| self.ledger_fault(why))?;
            }
            for (key, word) in kept.iter().filter(|(_, word)| !word.is_empty()) {
                store.keep_project_learned(&project, key, word, at.as_str()).map_err(|why| self.ledger_fault(why))?;
            }
        }
        let after = self.operator_settings_of(&project).await;
        if let Some(id) = self.operator_session_id(&project).await {
            if (before.model, before.effort, before.role, before.words, before.prompt, before.output_style)
                != (after.model.clone(), after.effort.clone(), after.role, after.words.clone(), after.prompt.clone(), after.output_style.clone())
            {
                self.handed_on(&id, None).await;
            }
            if before.decisions != after.decisions {
                let _ = self.operator_told(&project, &format!("He changed how you ask decisions in this project: now {}.", after.decisions.as_wire())).await;
            }
        }
        self.operator_page_changed();
        self.operator_settings_listed().await
    }

    /// The keys a name in `unset` clears, or the refusal that names it.
    fn operator_unset_keys(&self, name: &str) -> Result<&'static [&'static str], Refusal> {
        UNSET.iter().find(|(one, _)| *one == name).map(|(_, keys)| *keys).ok_or_else(|| {
            self.hosted_refusal(INVALID, &format!("`{name}` is no setting of an Operator that can be unset"))
        })
    }

    /// Each value `set` carries, as the key it is kept under and the word kept. An empty word clears its key.
    fn operator_kept_words(&self, set: &OperatorSettingsSet) -> Result<Vec<(&'static str, String)>, Refusal> {
        if let Some(accent) = set.accent.as_deref().filter(|accent| !accent.is_empty()) {
            if !accent_is_hex(accent) {
                return Err(self.hosted_refusal(INVALID, &format!("the accent {accent} is not a colour written #rrggbb")));
            }
        }
        let flag = |on: bool| on.to_string();
        let mut words: Vec<(&'static str, String)> = Vec::new();
        let mut put = |key: &'static str, word: Option<String>| words.extend(word.map(|word| (key, word)));
        put(ON, set.on.map(flag));
        put(MODEL, set.model.clone());
        put(EFFORT, set.effort.clone());
        put(ROLE, set.role.map(|role| role.as_wire().to_string()));
        put(WORDS, set.words.clone());
        put(PROMPT, set.prompt.clone());
        put(STYLE, set.output_style.clone());
        put(DECISIONS, set.decisions.map(|one| one.as_wire().to_string()));
        put(NAME, set.name.clone());
        put(ACCENT, set.accent.clone());
        put(OPERATOR_ACTS_KEY, set.acts.map(|acts| acts.as_wire().to_string()));
        if let Some(tools) = set.tools {
            put(START_JOBS, tools.start_jobs.map(flag));
            put(MERGE, tools.merge.map(flag));
            put(END_JOBS, tools.end_jobs.map(flag));
            put(FILE_ISSUES, tools.file_issues.map(flag));
        }
        Ok(words)
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// While the Operator is on, each served repository that no project holds becomes a project of its own, named for the
    /// repository, so it has an Operator. **Only with the machine's switch on**: off, Fleet makes no project he did not.
    pub(crate) async fn operator_loose_projects_made(&self) {
        if !self.operator_on() {
            return;
        }
        let held = self.projects_held_read().await;
        let loose: Vec<String> = self
            .repositories()
            .served()
            .iter()
            .filter(|served| held.holding(served.manifest().id()).is_empty())
            .map(|served| served.manifest().id().as_str().to_string())
            .collect();
        for manifest in loose {
            let name = self.board_repository(&manifest).unwrap_or_else(|| manifest.clone());
            let new = ipc::NewProject { name, members: Some(vec![ipc::ProjectMember { manifest_id: ManifestId::carried(manifest), folder: None }]), place: None, values: None };
            if let Err(why) = api::Projects::add_project(self, new).await {
                self.operator_warned("a repository in no project did not become one", format!("{why:?}"));
            }
        }
    }
}
