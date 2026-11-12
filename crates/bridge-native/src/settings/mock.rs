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
        MockFleet { list: settings(), repositories: crate::mock::repositories(), projects, connected: true, opened: 0, saves: Vec::new(), preferences: ipc::Preferences::default() }
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
                return Err(Refused { key: Some(key.clone()), said: format!("settings.json names `{key}`, which is not a setting Armada has") });
            };
            if let Some(value) = value {
                if let Some(why) = fault(&one.kind, value) {
                    return Err(Refused { key: Some(key.clone()), said: format!("`{key}` {why}") });
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
            other => return Err(Refused::said(format!("there is no preference {other}"))),
        }
        Ok(self.preferences.clone())
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
