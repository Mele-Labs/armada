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
    /// Whether the mock answers: a walk turns it off to show Settings with Fleet gone.
    pub connected: bool,
    /// How many times settings.json was asked to open.
    pub opened: usize,
    /// Every save it took, oldest first, for a test to read.
    pub saves: Vec<BTreeMap<String, Option<SettingValue>>>,
    pub preferences: ipc::Preferences,
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
    /// What `GET /models` answers: Classic's mock models (`apps/desktop/src/renderer/src/mock/kit-models.ts`).
    pub models: ipc::ModelChoices,
    /// The Phone Gateway Settings › Macs reads (`gateway.rs`).
    pub gateway: super::gateway::MockGateway,
    /// What a press would have opened in the browser or put on the clipboard: a walk touches neither of his.
    pub handed: Vec<String>,
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
        // The first project sets its own reviews and its pull request mode; the rest come from Kit and the defaults.
        if let Some(first) = projects.projects.first_mut() {
            first.values = Some(ValuesSet {
                pr_mode: ipc::PrMode::from_wire("draft"),
                reviews: Some(ipc::ReviewsSet { plans: Some("review".into()), pull_requests: Some("review_and_approve".into()), walks: Some("off".into()) }),
                ..ValuesSet::default()
            });
        }
        MockFleet { list: with_models(settings()), models: kit_models(), workflows: settings_workflows(), definitions: super::mock_workflows::definitions(), changes: Vec::new(), repositories: crate::mock::repositories(), projects, connected: true, opened: 0, saves: Vec::new(),
            preferences: ipc::Preferences::default(),
            gateway: super::gateway::MockGateway::new(),
            handed: Vec::new(),
            mods: crate::mock::mods(),
            sheets: crate::mock::mods().iter().filter(|row| row.enabled && row.valid).filter_map(|row| crate::mock::mod_sheet(&row.name).map(|css| (row.name.clone(), css.to_string()))).collect(),
        }
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

    /// A preference that is text, kept as Fleet keeps it: key bindings it would not read are refused whole.
    pub fn save_preference(&mut self, name: &str, text: String) -> Result<ipc::Preferences, Refused> {
        if !self.connected {
            return Err(Refused::said("Fleet is not connected"));
        }
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

    /// What a project's values come to: its own over Kit's, Kit's over the defaults.
    pub fn values(&self, project: &str) -> Result<ResolvedValues, Refused> {
        let own = self.projects.projects.iter().find(|one| one.id == project).ok_or_else(|| Refused::said(format!("there is no project {project}")))?.values.clone().unwrap_or_default();
        let word = |own: Option<String>, kit: Option<&str>, default: Option<&str>| match (own, kit, default) {
            (Some(value), _, _) => ResolvedValue { value: Some(value), from: ValueLayer::Project },
            (None, Some(value), _) => ResolvedValue { value: Some(value.into()), from: ValueLayer::Kit },
            (None, None, value) => ResolvedValue { value: value.map(String::from), from: ValueLayer::Default },
        };
        let look = own.look.clone().unwrap_or_default();
        let reviews = match &own.reviews {
            Some(set) => ResolvedReviews {
                plans: set.plans.clone().unwrap_or_else(|| "off".into()),
                pull_requests: set.pull_requests.clone().unwrap_or_else(|| "off".into()),
                walks: set.walks.clone().unwrap_or_else(|| "off".into()),
                from: ValueLayer::Project,
            },
            None => ResolvedReviews { plans: "off".into(), pull_requests: "off".into(), walks: "off".into(), from: ValueLayer::Default },
        };
        Ok(ResolvedValues {
            pr_mode: word(own.pr_mode.map(|mode| mode.as_wire().to_string()), None, Some("ready")),
            auto_merge: word(own.auto_merge, Some("checks-pass"), Some("never")),
            review_model: word(own.review_model, Some("opus"), None),
            layout: word(look.layout, None, None),
            theme: word(look.theme, None, Some("dark")),
            density: word(look.density, None, Some("default")),
            tracker: Some(ipc::ResolvedTracker { value: own.tracker.clone(), from: if own.tracker.is_some() { ValueLayer::Project } else { ValueLayer::Default } }),
            reviews: Some(reviews),
            autonomy: Vec::new(),
            ..ResolvedValues::default()
        })
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
}
