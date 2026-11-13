//! The walk's and the tests' Fleet for Settings. Its settings.json is Classic's mock
//! (`packages/surfaces/settings/src/fake-settings.ts`, written out as `mock-settings.json`), so the two apps
//! are compared on the same rows. A save is answered as Fleet answers it (`savedInto`), and refused as
//! `crates/config/src/settings/check.rs` refuses one.

use std::collections::BTreeMap;

use ipc::{Projects, RepositorySummary, ResolvedReviews, ResolvedValue, ResolvedValues, SettingKind, SettingValue, SettingsList, ValueLayer, ValuesSet};

use super::wire::Refused;

pub struct MockFleet {
    pub list: SettingsList,
    pub repositories: Vec<RepositorySummary>,
    pub projects: Projects,
    /// What Kit sets for every project (`values.yml` in Kit's folder).
    pub kit: ValuesSet,
    /// Whether the mock answers: a walk turns it off to show Settings with Fleet gone.
    pub connected: bool,
    /// How many times settings.json was asked to open.
    pub opened: usize,
    /// Every save it took, oldest first, for a test to read.
    pub saves: Vec<BTreeMap<String, Option<SettingValue>>>,
    pub preferences: ipc::Preferences,
    /// Words a save is refused for, as Fleet refuses one outside a set: the walk of a refused save names one.
    pub refuses: Vec<String>,
    /// The mods on this machine, as the walk's scenario lists them (`crate::mock::mods`).
    pub mods: Vec<ipc::ModSummary>,
    /// The checked sheet of each theme mod that is on and valid.
    pub sheets: BTreeMap<String, String>,
    /// What `GET /workflows` answers on All (`settings_workflows`).
    pub workflows: Vec<ipc::WorkflowSummary>,
    /// Each definition's file, by place, as `mock_workflows.rs` answers the Workflows page's acts from them.
    pub definitions: Vec<super::mock_workflows::Held>,
    /// The changes an agent was asked to draft, as `mock_workflows.rs` keeps them until approved or dismissed.
    pub changes: Vec<super::mock_workflows::Kept>,
    /// The drafts of workflows and pieces `PUT` to it, until published or discarded.
    pub drafts: super::mock_workflows::Drafts,
    /// The versions each place's workflows have had, and what each copy began from.
    pub versions: super::mock_workflows::Versions,
    /// What `GET /models` answers: Classic's mock models (`apps/desktop/src/renderer/src/mock/kit-models.ts`).
    pub models: ipc::ModelChoices,
    /// The Phone Gateway Settings › Macs reads (`gateway.rs`).
    pub gateway: super::gateway::MockGateway,
    /// What a press would have opened in the browser or put on the clipboard: a walk touches neither of his.
    pub handed: Vec<String>,
    /// The project each Job was started for, by Job id (`HostedFacts.project`): the walk's Native Bridge
    /// foundation was started for Armada.
    pub job_projects: Vec<(&'static str, &'static str)>,
    /// Whether a repository's copy has its workflow lane drawn from its text, as Fleet draws it: the walk of choosing what
    /// to take sets it, where the other walks keep the lanes file's steps.
    pub lanes_from_text: bool,
}

/// What Fleet runs on All, as Classic's Workflows page is walked on (`mock/workflows-managed.ts`): Armada's four,
/// armada's own copy of `feature`, sample-web's own `bug`, and Yours `docs_only`.
pub fn settings_workflows() -> Vec<ipc::WorkflowSummary> {
    ipc::decode("workflows", include_bytes!("mock-workflows.json")).unwrap_or_else(|why| panic!("the mock workflows do not decode: {why}"))
}

/// Classic's four mock models, as Fleet's `list_models` would answer them.
pub fn kit_models() -> ipc::ModelChoices {
    let levels = r#"["low","medium","high","xhigh","max"]"#;
    let said = format!(
        r#"{{"models":["opus","fable","sonnet"],"default":"sonnet","session":"opus","harnesses":["claude"],"read":{{"what":"read"}},"offered":[
        {{"name":"opus","display":"Opus 5.5","version":"opus-5-5","description":"For complex work and everyday tasks","efforts":{levels},"on":true}},
        {{"name":"fable","display":"Fable 5.1","version":"fable-5-1","description":"For your toughest challenges","efforts":{levels},"on":true}},
        {{"name":"sonnet","display":"Sonnet 5.5","version":"sonnet-5-5","description":"Most efficient for simpler tasks","efforts":{levels},"on":true}},
        {{"name":"haiku","display":"Haiku 5.5","version":"haiku-5-5","description":"Fastest for quick answers","efforts":{levels},"on":false}}]}}"#
    );
    ipc::decode("ModelChoices", said.as_bytes()).unwrap_or_else(|why| panic!("the mock models do not decode: {why}"))
}

/// The mock settings with Classic's mock Models choices in force, and the model rows Classic's fake settings
/// leave out, written as Fleet serves them (`crates/config/src/settings/models.rs`).
fn with_models(mut list: SettingsList) -> SettingsList {
    let text = |word: &str| SettingValue::Text(word.into());
    let like = |list: &SettingsList, key: &str| list.settings.iter().find(|one| one.key == key).cloned();
    let efforts = SettingKind::Choice { options: ["harness default", "low", "medium", "high", "xhigh", "max"].map(String::from).to_vec() };
    let added = [
        ("models.proposer", "models.review", "Review model", "The model an agent run about a pull request in the Reviews tab runs on.", "opus"),
        ("models.proposer", "models.judge", "Judge model", "The model a step naming none of its own is judged on.", "sonnet"),
        ("models.proposer", "models.retro", "Retro model", "The model a Job's retro is written on, once the Job ends.", "sonnet"),
        ("models.proposer", "models.secondOpinion", "Second opinion model", "The model a judged gaming flag is read a second time on.", "opus"),
        ("effort.judge", "effort.proposer", "Job proposer effort", "How hard the Job proposer thinks reading a request.", "harness default"),
    ];
    for (from, key, title, description, value) in added {
        if like(&list, key).is_some() {
            continue;
        }
        let Some(mut one) = like(&list, from) else { continue };
        (one.key, one.title, one.description, one.value, one.saved) = (key.into(), title.into(), description.into(), text(value), None);
        list.settings.push(one);
    }
    for one in &mut list.settings {
        match one.key.as_str() {
            "models.roster" => one.value = SettingValue::List(["opus", "fable", "sonnet"].map(text).to_vec()),
            "models.default" => one.value = text("sonnet"),
            "effort.default" => one.value = text("high"),
            _ => {}
        }
        if one.key.starts_with("effort.") {
            one.kind = efforts.clone();
        }
    }
    list
}

pub fn settings() -> SettingsList {
    ipc::decode("SettingsList", include_bytes!("mock-settings.json")).unwrap_or_else(|why| panic!("the mock settings do not decode: {why}"))
}

impl MockFleet {
    pub fn new() -> MockFleet {
        let mut projects = crate::mock::projects();
        // The first project lands on its own, always merges, and reviews its plans; its pull requests' and walks'
        // reviews fall through, to Kit and to the default. The rest come from Kit and the defaults.
        if let Some(first) = projects.projects.first_mut() {
            first.values = Some(ValuesSet {
                delivery: Some("auto_land".into()),
                auto_merge: Some("always".into()),
                reviews: Some(ipc::ReviewsSet { plans: Some("review_and_approve".into()), ..ipc::ReviewsSet::default() }),
                ..ValuesSet::default()
            });
        }
        // Every project asks each Job how work lands, opens drafts, merges once checks pass, and reviews pull requests.
        let kit = ValuesSet {
            delivery: Some("ask".into()),
            pr_mode: ipc::PrMode::from_wire("draft"),
            auto_merge: Some("checks-pass".into()),
            review_model: Some("opus".into()),
            reviews: Some(ipc::ReviewsSet { pull_requests: Some("review".into()), ..ipc::ReviewsSet::default() }),
            ..ValuesSet::default()
        };
        let mut fleet = MockFleet {
            kit, list: with_models(settings()), models: kit_models(), workflows: settings_workflows(), definitions: super::mock_workflows::definitions(), changes: Vec::new(), drafts: Default::default(), versions: Default::default(), repositories: crate::mock::repositories(), projects, connected: true, opened: 0, saves: Vec::new(),
            preferences: ipc::Preferences::default(),
            refuses: Vec::new(),
            gateway: super::gateway::MockGateway::new(),
            handed: Vec::new(),
            job_projects: vec![("mock-02", "armada")],
            lanes_from_text: false,
            mods: crate::mock::mods(),
            sheets: crate::mock::mods().iter().filter(|row| row.enabled && row.valid).filter_map(|row| crate::mock::mod_sheet(&row.name).map(|css| (row.name.clone(), css.to_string()))).collect(),
        };
        fleet.seed_versions();
        fleet
    }

    pub fn settings(&self) -> Result<SettingsList, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        Ok(self.list.clone())
    }

    /// Fleet's answer to a save: the whole change checked first, nothing written on a refusal, and a key that
    /// applies at a restart marked as waiting for one.
    pub fn save(&mut self, changes: &BTreeMap<String, Option<SettingValue>>) -> Result<SettingsList, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        for (key, value) in changes {
            let Some(one) = self.list.settings.iter().find(|one| &one.key == key) else {
                return Err(Refused { key: Some(key.clone()), said: format!("settings.json names `{key}`, which is not a setting Armada has"), code: None });
            };
            if let Some(value) = value {
                if let Some(why) = fault(&one.kind, value) {
                    return Err(Refused { key: Some(key.clone()), said: format!("`{key}` {why}"), code: None });
                }
            }
        }
        self.saves.push(changes.clone());
        for one in &mut self.list.settings {
            let Some(change) = changes.get(&one.key) else { continue };
            let wanted = change.clone().unwrap_or_else(|| one.default.clone());
            one.saved = change.clone();
            if one.applies == ipc::SettingApplies::AtRestart {
                one.pending_restart = wanted != one.value;
            } else if one.overridden_by_env.is_none() {
                one.value = wanted;
            }
        }
        Ok(self.list.clone())
    }

    /// A preference saved as Fleet takes it: a switch for the names that are switches, a word or text for the rest.
    pub fn save_put(&mut self, name: &str, put: super::wire::Put) -> Result<ipc::Preferences, Refused> {
        match put {
            super::wire::Put::Text(text) => self.save_preference(name, text),
            super::wire::Put::Switch(on) => self.save_switch(name, on),
        }
    }

    /// A switch, kept as Fleet keeps it. A name that is not a switch is refused by name.
    pub fn save_switch(&mut self, name: &str, on: bool) -> Result<ipc::Preferences, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let held = &mut self.preferences;
        match name {
            "first_run_answered" => held.first_run_answered = on,
            "listen_end_summary" => held.listen_end_summary = on,
            "listen_needs_you" => held.listen_needs_you = on,
            "listen_progress" => held.listen_progress = on,
            "listen_operator" => held.listen_operator = on,
            "buttons_words" => held.buttons_words = on,
            other => return Err(Refused::said(format!("there is no preference {other}"))),
        }
        Ok(self.preferences.clone())
    }

    /// A preference that is text, kept as Fleet keeps it: key bindings it would not read are refused whole, and a
    /// word outside its set is refused as `fleet.unacceptable_choice`.
    pub fn save_preference(&mut self, name: &str, text: String) -> Result<ipc::Preferences, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let unacceptable = |text: &str| Refused { key: None, said: format!("`{text}` is not a choice of `{name}`"), code: Some("fleet.unacceptable_choice".into()) };
        if self.refuses.contains(&text) {
            return Err(unacceptable(&text));
        }
        match name {
            "take_in" => self.preferences.take_in = ipc::TakeIn::from_word(&text).ok_or_else(|| unacceptable(&text))?,
            "answer_with" => self.preferences.answer_with = ipc::AnswerWith::from_word(&text).ok_or_else(|| unacceptable(&text))?,
            "read_results" => self.preferences.read_results = ipc::ReadResults::from_word(&text).ok_or_else(|| unacceptable(&text))?,
            "look_results" => self.preferences.look_results = ipc::LookResults::from_word(&text).ok_or_else(|| unacceptable(&text))?,
            "look_questions" => self.preferences.look_questions = ipc::LookQuestions::from_word(&text).ok_or_else(|| unacceptable(&text))?,
            // Blank takes his choice back: the Mac's own voice.
            "listen_voice" => self.preferences.listen_voice = text.trim().to_string(),
            _ => return self.save_text(name, text),
        }
        Ok(self.preferences.clone())
    }

    fn save_text(&mut self, name: &str, text: String) -> Result<ipc::Preferences, Refused> {
        match name {
            "key_bindings" => {
                // Empty takes the person's changes back, as Fleet does.
                if let Some(problem) = (!text.trim().is_empty()).then(|| ipc::key_bindings::problems(&text)).unwrap_or_default().into_iter().next() {
                    return Err(Refused::said(format!("key_bindings: {problem}")));
                }
                self.preferences.key_bindings = text;
            }
            "theme" => self.preferences.theme = text,
            // Empty takes his choice back; text Fleet would not read is refused whole.
            "layout_choices" => {
                if let Some(problem) = (!text.is_empty()).then(|| ipc::layout::problems(&text)).unwrap_or_default().into_iter().next() {
                    return Err(Refused::said(format!("layout_choices: {problem}")));
                }
                self.preferences.layout_choices = text;
            }
            "time_format" => {
                if let Some(problem) = (!text.is_empty()).then(|| ipc::time_format::problems(&text)).unwrap_or_default().into_iter().next() {
                    return Err(Refused::said(format!("time_format: {problem}")));
                }
                self.preferences.time_format = text;
            }
            other => return Err(Refused::said(format!("there is no preference {other}"))),
        }
        Ok(self.preferences.clone())
    }

    /// A mod's switch, kept per machine as Fleet keeps it, answered with its row. A mod switched off has no sheet to draw.
    pub fn set_mod_enabled(&mut self, name: &str, enabled: bool) -> Result<ipc::ModSummary, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let row = self.mods.iter_mut().find(|row| row.name == name).ok_or_else(|| Refused::said(format!("There is no mod named {name}.")))?;
        row.enabled = enabled;
        if !enabled {
            self.sheets.remove(name);
        } else if let Some(css) = crate::mock::mod_sheet(name) {
            self.sheets.insert(name.to_string(), css.to_string());
        }
        Ok(row.clone())
    }

    /// Promote, as Fleet answers one where no pull request could open: the Job and its branch, with the commit on it.
    pub fn promote_mod(&mut self, name: &str) -> Result<ipc::ModPromoted, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let said = format!(r#"{{"name":"{name}","session_id":"01SESSIONPROMOTEAAAAAAAAAA","title":"Add the {name} theme mod","branch":"armada/mod-{name}","commit":"4f2a9c1","pull_request_not_opened":"No remote to push to."}}"#);
        ipc::decode("ModPromoted", said.as_bytes()).map_err(|why| Refused::said(why.to_string()))
    }

    /// What Fleet's check of a mod read: quiet-ci's one rule, batching CI failures except where one blocks a merge.
    pub fn validate_mod(&self, name: &str) -> Result<ipc::ModChecked, Refused> {
        let row = self.mods.iter().find(|row| row.name == name).ok_or_else(|| Refused::said(format!("There is no mod named {name}.")))?;
        let fleet = match row.kind {
            Some(ipc::ModKind::Fleet) => r#","fleet":{"rule":[{"notice":"pr_checks_failed","does":"batch","where":"everywhere","when":[{"if":"blocks_merge","does":"tell"}]}]}"#,
            Some(ipc::ModKind::Layout) => r#","layout":"{\"version\":1,\"rail\":{\"hidden\":[\"guides\"]}}""#,
            Some(ipc::ModKind::Words) => r#","words":{"name":"Jira key","pattern":"[A-Z][A-Z0-9]+-\\d+","example":"STORE-1234","address":[{"repository":"armada","template":"https://acme.atlassian.net/browse/{match}","found":"settings"},{"repository":"*","template":"https://jira.example.com/browse/{match}","found":"pull_request #2140"}]}"#,
            _ => "",
        };
        let said = format!(r#"{{"name":"{name}","valid":true,"problems":[]{fleet}}}"#);
        ipc::decode("ModChecked", said.as_bytes()).map_err(|why| Refused::said(why.to_string()))
    }

    /// A workflow's lanes at a level, as `get_workflow_lanes` answers: the default's and each repository's. The
    /// default's marks what armada changes; armada's own level is where Platform and Mobile tie on Mark ready.
    pub fn workflow_lanes(&self, workflow: &str, scope: &str) -> Result<ipc::JobLanesView, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let served = self.repositories.iter().filter_map(|one| one.manifest.as_ref()).any(|manifest| manifest.id.as_str() == scope);
        if scope != "default" && !served {
            return Err(Refused {
                key: None,
                said: format!("`{scope}` is not a level of this repository's lanes: the default's, a Workspace that holds it, or a repository read with it"),
                code: Some("fleet.no_such_lanes_level".into()),
            });
        }
        let mut lanes: ipc::JobLanesView = ipc::decode("JobLanesView", include_bytes!("mock-lanes.json")).map_err(|why| Refused::said(why.to_string()))?;
        lanes.workflow = workflow.to_string();
        // The workflow's lane is headed with the workflow asked for, as Fleet heads it.
        if let Some(head) = lanes.lanes.first_mut() {
            let words = workflow.replace('_', " ");
            head.title = words[..1].to_uppercase() + &words[1..];
        }
        if self.lanes_from_text {
            if let Some(text) = self.repository_text(workflow, scope) {
                lanes = crate::lanes_editor::with_steps_of(lanes, &text);
            }
        }
        // A rule a publish wrote is kept now: its chip is drawn as the others are.
        if !self.drafts.published_rules().is_empty() {
            let lifecycles = self.lifecycles(scope).map(|read| read.lifecycles.into_iter().map(|one| one.lifecycle).collect()).unwrap_or_default();
            lanes = crate::lanes_editor::with_rules_kept(lanes, self.drafts.published_rules(), lifecycles);
        }
        let ready = lanes.rules.iter_mut().find(|edge| edge.rule == "ready");
        match (scope, ready) {
            ("default", Some(ready)) => ready.differs = vec!["Platform".into(), "armada".into()],
            ("armada", Some(ready)) => {
                ready.differs = vec!["armada".into()];
                ready.clash = Some("Platform and Mobile differ".into());
            }
            _ => {}
        }
        if scope == "default" {
            if let Some(open) = lanes.lanes.iter_mut().flat_map(|lane| lane.nodes.iter_mut()).find(|node| node.id == "pull_request/open") {
                open.differs = vec!["sample-web".into()];
            }
        }
        Ok(lanes)
    }

    /// `GET /lifecycles`: the Ticket and Pull request the mock lanes draw, as Armada carries them.
    pub fn lifecycles(&self, manifest: &str) -> Result<ipc::RepositoryLifecycles, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let state = |id: &str, label: &str, read: ipc::StateReadFrom| ipc::LifecycleStateDefinition { id: id.into(), label: label.into(), read };
        let tracker = |label: &str| ipc::StateReadFrom::Tracker { words: vec![label.into()] };
        let forge = |fact: &str| ipc::StateReadFrom::Forge { fact: fact.into() };
        let moves = |pairs: &[(&str, &str)]| pairs.iter().map(|(from, to)| ipc::LifecycleTransition { from: (*from).into(), to: (*to).into() }).collect();
        let carried = || ipc::PieceKept { rank: ipc::PieceRank { reach: "everywhere".into(), source: "armada".into(), home: false }, path: None, overridden_by: None, hides: None };
        let ticket = ipc::LifecycleDefinition {
            id: "ticket".into(),
            name: "Ticket".into(),
            scope: ipc::PieceScope::Everywhere,
            states: vec![state("todo", "Todo", tracker("Todo")), state("in_progress", "In Progress", tracker("In Progress")), state("in_review", "In Review", tracker("In Review")), state("done", "Done", tracker("Done"))],
            transitions: moves(&[("todo", "in_progress"), ("in_progress", "in_review"), ("in_review", "done")]),
            ends: vec!["done".into()],
        };
        let pull_request = ipc::LifecycleDefinition {
            id: "pull_request".into(),
            name: "Pull request".into(),
            scope: ipc::PieceScope::Everywhere,
            states: vec![state("draft", "Draft", forge("draft")), state("open", "Open", forge("ready")), state("merged", "Merged", forge("merged"))],
            transitions: moves(&[("draft", "open"), ("open", "merged")]),
            ends: vec!["merged".into()],
        };
        Ok(ipc::RepositoryLifecycles {
            manifest_id: ipc::ManifestId::carried(manifest),
            lifecycles: vec![ipc::AppliedLifecycle { lifecycle: ticket, kept: carried() }, ipc::AppliedLifecycle { lifecycle: pull_request, kept: carried() }],
            left_out: Vec::new(),
        })
    }

    /// `GET /lifecycles/rules`: the rules the mock lanes draw a chip for, then any a draft published since. A rule
    /// published again replaces the one of its id.
    pub fn lifecycle_rules(&self, manifest: &str) -> Result<ipc::RepositoryLifecycleRules, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let mut read: ipc::RepositoryLifecycleRules = ipc::decode("RepositoryLifecycleRules", include_bytes!("mock-rules.json")).map_err(|why| Refused::said(why.to_string()))?;
        read.manifest_id = ipc::ManifestId::carried(manifest);
        for rule in self.drafts.published_rules() {
            let kept = ipc::PieceKept { rank: ipc::PieceRank { reach: "everywhere".into(), source: "kit".into(), home: false }, path: Some(format!("~/.armada/kit/rules/{}.yml", rule.id)), overridden_by: None, hides: None };
            read.rules.retain(|one| one.rule.id != rule.id);
            read.rules.push(ipc::AppliedRule { rule: rule.clone(), kept });
        }
        Ok(read)
    }

    /// `GET /autonomy`: Armada's own default for each kind of act, and no layer over it.
    pub fn autonomy_defaults(&self, manifest: &str) -> Result<ipc::RepositoryAutonomy, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let armada = [("tracker_write", "asks"), ("forge_write", "asks"), ("draft", "asks"), ("start_job", "asks"), ("start_watch", "acts"), ("tell_agent", "acts"), ("run_command", "acts"), ("run_agent", "acts")]
            .into_iter()
            .map(|(kind, autonomy)| ipc::ActKindDefault { kind: kind.into(), autonomy: autonomy.into() })
            .collect();
        Ok(ipc::RepositoryAutonomy { manifest_id: ipc::ManifestId::carried(manifest), armada, layers: Vec::new(), left_out: Vec::new() })
    }

    /// `GET /watches/recipes`: the recipes a watch may start from.
    pub fn watch_recipes(&self) -> Result<ipc::WatchRecipeList, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let recipe = |name: &str| format!(r#"{{"name":"{name}","values":[],"command":["true"],"source":"machine","every_seconds":60}}"#);
        let said = format!(r#"{{"recipes":[{},{}]}}"#, recipe("deploy"), recipe("release"));
        ipc::decode("WatchRecipeList", said.as_bytes()).map_err(|why| Refused::said(why.to_string()))
    }

    /// A tracker mod's try, passed against the first repository, as Fleet answers one.
    pub fn try_tracker_mod(&mut self, name: &str) -> Result<ipc::TrackerTry, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        let said = br#"{"at":"2026-11-12T10:00:00Z","manifest_id":"armada","passed":true,"listed":12,"read":"ACME-7"}"#;
        let tried: ipc::TrackerTry = ipc::decode("TrackerTry", said).map_err(|why| Refused::said(why.to_string()))?;
        let row = self.mods.iter_mut().find(|row| row.name == name).ok_or_else(|| Refused::said(format!("There is no mod named {name}.")))?;
        row.tried = Some(tried.clone());
        row.stale = false;
        Ok(tried)
    }

    /// Create mod's empty Mod Job, started.
    pub fn start_mod_session(&self) -> Result<ipc::ModSessionStarted, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        ipc::decode("ModSessionStarted", br#"{"session_id":"01SESSIONMODJOBAAAAAAAAAAA","title":"Mod Job"}"#).map_err(|why| Refused::said(why.to_string()))
    }

    /// What a project's values come to: its own over Kit's, Kit's over the defaults, a kind of review at a time.
    pub fn values(&self, project: &str) -> Result<ResolvedValues, Refused> {
        let own = self.projects.projects.iter().find(|one| one.id == project).ok_or_else(|| Refused::said(format!("there is no project {project}")))?.values.clone().unwrap_or_default();
        Ok(self.resolved(Some(&own)))
    }

    /// What every project uses: Kit's over the defaults.
    pub fn every_values(&self) -> Result<ResolvedValues, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
        Ok(self.resolved(None))
    }

    fn resolved(&self, own: Option<&ValuesSet>) -> ResolvedValues {
        let kit = &self.kit;
        let word = |own: Option<String>, kit: Option<String>, default: Option<&str>| match (own, kit) {
            // What the project overrides: Kit's word, else nothing below sets it.
            (Some(value), kit) => ResolvedValue {
                value: Some(value),
                from: ValueLayer::Project,
                inherited_from: Some(if kit.is_some() { ValueLayer::Kit } else { ValueLayer::Default }),
                inherited: kit,
                ..ResolvedValue::default()
            },
            (None, Some(value)) => ResolvedValue { value: Some(value), from: ValueLayer::Kit, ..ResolvedValue::default() },
            (None, None) => ResolvedValue { value: default.map(String::from), from: ValueLayer::Default, ..ResolvedValue::default() },
        };
        let own_look = own.and_then(|own| own.look.clone()).unwrap_or_default();
        let kit_look = kit.look.clone().unwrap_or_default();
        let own_reviews = own.and_then(|own| own.reviews.clone()).unwrap_or_default();
        let kit_reviews = kit.reviews.clone().unwrap_or_default();
        let kind = |own: Option<String>, kit: Option<String>| word(own, kit, Some("off"));
        let (plans, pull_requests, walks) = (kind(own_reviews.plans, kit_reviews.plans), kind(own_reviews.pull_requests, kit_reviews.pull_requests), kind(own_reviews.walks, kit_reviews.walks));
        let nearest = [plans.from, pull_requests.from, walks.from].into_iter().max_by_key(|from| *from as u8).unwrap_or_default();
        let reviews = ResolvedReviews {
            plans: plans.value.clone().unwrap_or_default(),
            pull_requests: pull_requests.value.clone().unwrap_or_default(),
            walks: walks.value.clone().unwrap_or_default(),
            from: nearest,
            plans_from: Some(plans.from),
            pull_requests_from: Some(pull_requests.from),
            walks_from: Some(walks.from),
            plans_inherited: plans.inherited.clone(),
            plans_inherited_from: plans.inherited_from,
            pull_requests_inherited: pull_requests.inherited.clone(),
            pull_requests_inherited_from: pull_requests.inherited_from,
            walks_inherited: walks.inherited.clone(),
            walks_inherited_from: walks.inherited_from,
        };
        let pick = |field: fn(&ValuesSet) -> Option<String>| (own.and_then(field), field(kit));
        let (own_mode, kit_mode) = pick(|set| set.pr_mode.map(|mode| mode.as_wire().to_string()));
        let (own_merge, kit_merge) = pick(|set| set.auto_merge.clone());
        let (own_model, kit_model) = pick(|set| set.review_model.clone());
        let (own_starts, kit_starts) = pick(|set| set.agent_starts.clone());
        let (own_delivery, kit_delivery) = pick(|set| set.delivery.clone());
        let tracker = own.and_then(|own| own.tracker.clone());
        ResolvedValues {
            pr_mode: word(own_mode, kit_mode, Some("ready")),
            auto_merge: word(own_merge, kit_merge, Some("never")),
            review_model: word(own_model, kit_model, None),
            layout: word(own_look.layout, kit_look.layout, None),
            theme: word(own_look.theme, kit_look.theme, Some("dark")),
            density: word(own_look.density, kit_look.density, Some("default")),
            tracker: Some(ipc::ResolvedTracker { from: if tracker.is_some() { ValueLayer::Project } else { ValueLayer::Default }, value: tracker }),
            reviews: Some(reviews),
            agent_starts: Some(word(own_starts, kit_starts, Some("at_once"))),
            // The first project's word is the one Fleet learned from his first answer, until he changes it.
            delivery: Some(ResolvedValue { learned: own_delivery.as_deref() == Some("auto_land"), ..word(own_delivery, kit_delivery, None) }),
            autonomy: Vec::new(),
        }
    }

    /// Kit's values kept whole, as Fleet does. Kit names no tracker.
    pub fn save_kit_values(&mut self, values: ValuesSet) -> Result<ipc::KitValuesSaved, Refused> {
        if values.tracker.is_some() {
            return Err(Refused::said("Kit's values name no tracker: a tracker is a project's"));
        }
        self.kit = values.clone();
        Ok(ipc::KitValuesSaved { file: "~/.armada/kit/values.yml".into(), values })
    }

    /// The edit: the project's values replaced whole, as Fleet does.
    pub fn set_values(&mut self, project: &str, values: ValuesSet) -> Result<Projects, Refused> {
        let one = self.projects.projects.iter_mut().find(|one| one.id == project).ok_or_else(|| Refused::said(format!("there is no project {project}")))?;
        for word in [&values.reviews.as_ref().and_then(|set| set.plans.clone()), &values.reviews.as_ref().and_then(|set| set.pull_requests.clone())].into_iter().flatten() {
            if !["off", "review", "review_and_approve"].contains(&word.as_str()) {
                return Err(Refused::said(format!("reviews takes off, review or review_and_approve, not {word}")));
            }
        }
        one.values = Some(values);
        Ok(self.projects.clone())
    }

    /// The edit with `shared_with_macs` alone. A project whose file was not read cannot be written.
    pub fn share_with_macs(&mut self, project: &str, on: bool) -> Result<Projects, Refused> {
        let one = self.projects.projects.iter_mut().find(|one| one.id == project).ok_or_else(|| Refused::said(format!("there is no project {project}")))?;
        if let Some(why) = &one.unread {
            return Err(Refused::said(why.clone()));
        }
        one.shared_with_macs = on;
        Ok(self.projects.clone())
    }
}

/// What is wrong with `value` as `kind`, as `check.rs` says it, or `None`.
fn fault(kind: &SettingKind, value: &SettingValue) -> Option<String> {
    match (kind, value) {
        (SettingKind::Integer { min, max, .. }, SettingValue::Integer(n)) if (min..=max).contains(&n) => None,
        (SettingKind::Integer { min, max, unit }, _) => Some(format!("must be a whole number of {} from {min} to {max}", unit.as_deref().unwrap_or(""))),
        (SettingKind::Seconds { min, max }, SettingValue::Integer(n)) if (min..=max).contains(&n) => None,
        (SettingKind::Seconds { min, max }, _) => Some(format!("must be whole seconds from {min} to {max}")),
        (SettingKind::Boolean, SettingValue::Bool(_)) => None,
        (SettingKind::Boolean, _) => Some("must be true or false".into()),
        (SettingKind::Choice { options }, SettingValue::Text(word)) if options.contains(word) => None,
        (SettingKind::Choice { options }, _) => Some(format!("must be one of {}", options.join(", "))),
        (SettingKind::Text, SettingValue::Text(_)) => None,
        (SettingKind::Prompt, SettingValue::Text(text)) if text.trim().is_empty() => Some("must not be blank; remove the key to use the prompt Armada ships".into()),
        (SettingKind::Prompt, SettingValue::Text(_)) => None,
        (SettingKind::Text | SettingKind::Prompt, _) => Some("must be text".into()),
        (SettingKind::TextList, SettingValue::List(items)) if !items.is_empty() && items.iter().all(|item| matches!(item, SettingValue::Text(_))) => None,
        (SettingKind::TextList, _) => Some("must be a list of text with at least one item".into()),
        (SettingKind::Json, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(key: &str, value: Option<SettingValue>) -> BTreeMap<String, Option<SettingValue>> {
        [(key.to_string(), value)].into_iter().collect()
    }

    #[test]
    fn a_save_is_answered_whole_and_a_reset_takes_the_shipped_value_back() {
        let mut fleet = MockFleet::new();
        let saved = fleet.save(&change("limits.dronesAtOnce", Some(SettingValue::Integer(5)))).expect("taken");
        let one = saved.settings.iter().find(|one| one.key == "limits.dronesAtOnce").expect("the row");
        assert_eq!((one.value.clone(), one.saved.clone()), (SettingValue::Integer(5), Some(SettingValue::Integer(5))));
        let reset = fleet.save(&change("limits.dronesAtOnce", None)).expect("taken");
        let one = reset.settings.iter().find(|one| one.key == "limits.dronesAtOnce").expect("the row");
        assert_eq!((one.value.clone(), one.saved.clone()), (one.default.clone(), None));
    }

    #[test]
    fn a_value_out_of_range_is_refused_naming_its_key_and_nothing_is_written() {
        let mut fleet = MockFleet::new();
        let refused = fleet.save(&change("limits.dronesAtOnce", Some(SettingValue::Integer(99)))).expect_err("refused");
        assert_eq!(refused.key.as_deref(), Some("limits.dronesAtOnce"));
        assert!(fleet.saves.is_empty());
    }

    #[test]
    fn a_key_that_applies_at_a_restart_waits_for_one() {
        let mut fleet = MockFleet::new();
        let Some(one) = fleet.list.settings.iter().find(|one| one.applies == ipc::SettingApplies::AtRestart && matches!(one.kind, SettingKind::Seconds { .. })).cloned() else {
            panic!("the mock holds a key that applies at a restart")
        };
        let SettingKind::Seconds { min, .. } = one.kind else { unreachable!() };
        let saved = fleet.save(&change(&one.key, Some(SettingValue::Integer(min)))).expect("taken");
        assert!(saved.settings.iter().find(|row| row.key == one.key).is_some_and(|row| row.pending_restart));
    }

    #[test]
    fn a_way_is_kept_as_a_word_and_a_word_outside_its_set_is_refused_as_fleet_refuses_it() {
        use super::super::wire::Put;
        let mut fleet = MockFleet::new();
        let saved = fleet.save_put("take_in", Put::Text("listen".into())).expect("a word of the set");
        assert_eq!(saved.take_in, ipc::TakeIn::Listen);
        for (name, word) in [("answer_with", "buttons"), ("read_results", "thread_only"), ("look_results", "walk_visible"), ("look_questions", "pictures_whole")] {
            fleet.save_put(name, Put::Text(word.into())).unwrap_or_else(|why| panic!("{name}: {}", why.said));
        }
        let kept = fleet.preferences.clone();
        assert_eq!((kept.answer_with, kept.read_results, kept.look_results, kept.look_questions), (ipc::AnswerWith::Buttons, ipc::ReadResults::ThreadOnly, ipc::LookResults::WalkVisible, ipc::LookQuestions::PicturesWhole));
        let refused = fleet.save_put("take_in", Put::Text("shout".into())).expect_err("not a word of the set");
        assert!(refused.is("fleet.unacceptable_choice"));
        assert_eq!(refused.said, "`shout` is not a choice of `take_in`");
        assert_eq!(fleet.preferences.take_in, ipc::TakeIn::Listen, "a refused word changes nothing");
    }

    #[test]
    fn a_switch_is_kept_and_a_name_that_is_not_a_switch_is_refused() {
        use super::super::wire::Put;
        let mut fleet = MockFleet::new();
        assert!(!fleet.preferences.first_run_answered && fleet.preferences.buttons_words && !fleet.preferences.listen_progress);
        fleet.save_put("first_run_answered", Put::Switch(true)).expect("a switch");
        fleet.save_put("buttons_words", Put::Switch(false)).expect("a switch");
        fleet.save_put("listen_progress", Put::Switch(true)).expect("a switch");
        let kept = fleet.preferences.clone();
        assert!(kept.first_run_answered && !kept.buttons_words && kept.listen_progress);
        assert!(fleet.save_put("take_in", Put::Switch(true)).is_err(), "a word is not a switch");
        // The voice is text; blank is the Mac's own.
        fleet.save_put("listen_voice", Put::Text("com.apple.voice.zoe".into())).expect("text");
        assert_eq!(fleet.preferences.listen_voice, "com.apple.voice.zoe");
        fleet.save_put("listen_voice", Put::Text(String::new())).expect("blank");
        assert_eq!(fleet.preferences.listen_voice, "");
    }

    #[test]
    fn a_word_the_walk_names_is_refused_whatever_set_it_is_asked_in() {
        use super::super::wire::Put;
        let mut fleet = MockFleet::new();
        fleet.refuses.push("pictures_whole".into());
        assert!(fleet.save_put("look_questions", Put::Text("pictures_whole".into())).is_err());
        assert_eq!(fleet.preferences.look_questions, ipc::LookQuestions::PicturesMix);
        assert!(fleet.save_put("look_questions", Put::Text("pictures_mix".into())).is_ok());
    }
}
