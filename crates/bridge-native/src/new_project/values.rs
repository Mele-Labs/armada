//! The settings a new project can start with, in the groups the approved sheet draws them, and how each is
//! read from what every project uses (`GET /projects/values`) and written into `NewProject.values`.
//!
//! A value is held as its wire word (`checks-pass`); the sheet draws its label. A word the same as what every
//! project uses is not sent, so the project sets only what was changed.

use std::collections::HashMap;

use ipc::{DeliveryChoice, LookSet, PrMode, ResolvedValues, ReviewsSet, ValuesSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Delivery,
    PrMode,
    AutoMerge,
    AgentStarts,
    Plans,
    PullRequests,
    Walks,
    ReviewModel,
    Tracker,
    Theme,
    Layout,
    Density,
}

pub struct Group {
    pub title: &'static str,
    pub icon: &'static str,
    pub keys: &'static [Key],
}

/// The groups in the sheet's order. Sharing, a switch, follows them.
pub const GROUPS: [Group; 5] = [
    Group { title: "Landing", icon: "git-merge", keys: &[Key::Delivery, Key::PrMode, Key::AutoMerge] },
    Group { title: "Starting Jobs", icon: "play", keys: &[Key::AgentStarts] },
    Group { title: "Operator reviews", icon: "eye", keys: &[Key::Plans, Key::PullRequests, Key::Walks, Key::ReviewModel] },
    Group { title: "Issues", icon: "ticket", keys: &[Key::Tracker] },
    Group { title: "Look", icon: "layers", keys: &[Key::Theme, Key::Layout, Key::Density] },
];

/// `delivery`'s word for a project that asks on its first Job and keeps the answer.
pub const ASK: &str = "ask";

impl Key {
    pub fn label(self) -> &'static str {
        match self {
            Key::Delivery => "How work lands",
            Key::PrMode => "Pull requests open as",
            Key::AutoMerge => "Merge",
            Key::AgentStarts => "Jobs a Job or the Operator starts",
            Key::Plans => "Plans",
            Key::PullRequests => "Pull requests",
            Key::Walks => "Walks",
            Key::ReviewModel => "Review model",
            Key::Tracker => "Tracker",
            Key::Theme => "Theme",
            Key::Layout => "Layout",
            Key::Density => "Density",
        }
    }

    pub fn note(self) -> Option<&'static str> {
        match self {
            Key::Delivery => Some("Learned from your first answer if left on Ask"),
            Key::Tracker => Some("This project only"),
            _ => None,
        }
    }

    /// A project's own: no other layer sets it, so it is never drawn as what every project uses.
    pub fn own(self) -> bool {
        self == Key::Tracker
    }

    /// Its words and how each reads, in the order the menu offers them. `models` is what a review may run on.
    pub fn options(self, models: &[String], layouts: &[(String, String)]) -> Vec<(String, String)> {
        let fixed = |pairs: &[(&str, &str)]| pairs.iter().map(|(word, label)| (word.to_string(), label.to_string())).collect();
        match self {
            Key::Delivery => std::iter::once((ASK.to_string(), "Ask".to_string()))
                .chain(DeliveryChoice::OFFERED.iter().map(|one| (delivery_word(*one).to_string(), one.label().to_string())))
                .collect(),
            Key::PrMode => fixed(&[("ready", "Ready"), ("draft", "Draft")]),
            Key::AutoMerge => fixed(&[("never", "Never"), ("checks-pass", "When checks pass"), ("always", "Always")]),
            Key::AgentStarts => fixed(&[("at_once", "Start at once"), ("ask_me", "Ask me")]),
            Key::Plans | Key::PullRequests => fixed(&[("off", "Off"), ("review", "Review"), ("review_and_approve", "Review and approve")]),
            Key::Walks => fixed(&[("off", "Off"), ("review", "Review")]),
            Key::ReviewModel => models.iter().map(|model| (model.clone(), model.clone())).collect(),
            Key::Tracker => fixed(&[("", "None"), ("github", "GitHub Issues"), ("linear", "Linear"), ("beads", "Beads"), (CUSTOM, "Custom")]),
            Key::Theme => std::iter::once((String::new(), "Armada".to_string()))
                .chain(crate::look::catalogue().iter().map(|one| (one.id.clone(), one.title.clone())))
                .collect(),
            Key::Layout => std::iter::once((String::new(), "Default".to_string())).chain(layouts.iter().cloned()).collect(),
            Key::Density => fixed(&[("compact", "Compact"), ("default", "Default"), ("roomy", "Roomy")]),
        }
    }

    /// How `word` reads: the option's label, else the word itself.
    pub fn said(self, word: &str, models: &[String], layouts: &[(String, String)]) -> String {
        let unlisted = if self == Key::Layout { CUSTOM_LAYOUT.to_string() } else { word.to_string() };
        self.options(models, layouts).into_iter().find(|(one, _)| one == word).map_or(unlisted, |(_, label)| label)
    }
}

/// The tracker word that means "a tracker Armada is asked to build": never sent as a provider.
pub const CUSTOM: &str = "custom";

/// How a project's stored layout reads when no layout mod's text matches it.
pub const CUSTOM_LAYOUT: &str = "Custom layout";

/// The word Fleet's delivery question keeps for a choice (`session_delivery.rs`, snake case).
pub fn delivery_word(choice: DeliveryChoice) -> &'static str {
    match choice {
        DeliveryChoice::AutoLand => "auto_land",
        DeliveryChoice::OpenPr => "open_pr",
        DeliveryChoice::KeepLocal => "keep_local",
    }
}

/// How a delivery word reads, for the line over a Job's box.
pub fn delivery_said(word: &str) -> Option<&'static str> {
    DeliveryChoice::OFFERED.iter().find(|one| delivery_word(**one) == word).map(|one| one.label())
}

/// What every project uses, word by word. A key Fleet gave no word for reads as its first option's.
pub fn inherited(values: &ResolvedValues) -> HashMap<Key, String> {
    let word = |value: &ipc::ResolvedValue| value.value.clone().unwrap_or_default();
    let mut all = HashMap::new();
    // A layer that sets no `delivery` leaves the value empty: the project asks.
    all.insert(Key::Delivery, values.delivery.as_ref().and_then(|one| one.value.clone()).filter(|word| !word.is_empty()).unwrap_or_else(|| ASK.to_string()));
    all.insert(Key::PrMode, word(&values.pr_mode));
    all.insert(Key::AutoMerge, word(&values.auto_merge));
    all.insert(Key::ReviewModel, word(&values.review_model));
    all.insert(Key::Theme, word(&values.theme));
    all.insert(Key::Layout, word(&values.layout));
    all.insert(Key::Density, word(&values.density));
    all.insert(Key::AgentStarts, values.agent_starts.as_ref().and_then(|one| one.value.clone()).unwrap_or_else(|| "at_once".into()));
    let reviews = values.reviews.as_ref();
    all.insert(Key::Plans, reviews.map_or_else(|| "off".into(), |one| one.plans.clone()));
    all.insert(Key::PullRequests, reviews.map_or_else(|| "off".into(), |one| one.pull_requests.clone()));
    all.insert(Key::Walks, reviews.map_or_else(|| "off".into(), |one| one.walks.clone()));
    all.insert(Key::Tracker, String::new());
    all
}

/// The words a project's own block sets, as the sheet's pickers hold them: the reverse of [`values_of`].
pub fn picked_of(set: &ValuesSet) -> HashMap<Key, String> {
    let mut all = HashMap::new();
    let mut put = |key: Key, word: Option<String>| {
        if let Some(word) = word {
            all.insert(key, word);
        }
    };
    put(Key::PrMode, set.pr_mode.map(|mode| mode.as_wire().to_string()));
    put(Key::AutoMerge, set.auto_merge.clone());
    put(Key::ReviewModel, set.review_model.clone());
    put(Key::AgentStarts, set.agent_starts.clone());
    put(Key::Delivery, set.delivery.clone());
    put(Key::Tracker, set.tracker.as_ref().and_then(|block| block.get("provider").cloned()));
    if let Some(look) = &set.look {
        put(Key::Theme, look.theme.clone());
        put(Key::Layout, look.layout.clone());
        put(Key::Density, look.density.clone());
    }
    if let Some(reviews) = &set.reviews {
        put(Key::Plans, reviews.plans.clone());
        put(Key::PullRequests, reviews.pull_requests.clone());
        put(Key::Walks, reviews.walks.clone());
    }
    all
}

/// The block `NewProject.values` carries: only the keys `picked` changes from `inherited`, `None` where it
/// changes none. Reviews go as all three where one changed, as a layer that sets them must.
pub fn values_of(picked: &HashMap<Key, String>, inherited: &HashMap<Key, String>) -> Option<ValuesSet> {
    let changed = |key: Key| picked.get(&key).filter(|word| inherited.get(&key) != Some(*word)).cloned();
    let now = |key: Key| picked.get(&key).or_else(|| inherited.get(&key)).cloned();
    let mut set = ValuesSet {
        pr_mode: changed(Key::PrMode).and_then(|word| PrMode::from_wire(&word)),
        auto_merge: changed(Key::AutoMerge),
        review_model: changed(Key::ReviewModel),
        agent_starts: changed(Key::AgentStarts),
        delivery: changed(Key::Delivery),
        tracker: changed(Key::Tracker).filter(|word| !word.is_empty() && word != CUSTOM).map(|word| [("provider".to_string(), word)].into_iter().collect()),
        ..ValuesSet::default()
    };
    let look = LookSet {
        theme: changed(Key::Theme).filter(|word| !word.is_empty()),
        layout: changed(Key::Layout).filter(|word| !word.is_empty()),
        density: changed(Key::Density),
    };
    if look != LookSet::default() {
        set.look = Some(look);
    }
    if [Key::Plans, Key::PullRequests, Key::Walks].into_iter().any(|key| changed(key).is_some()) {
        set.reviews = Some(ReviewsSet { plans: now(Key::Plans), pull_requests: now(Key::PullRequests), walks: now(Key::Walks) });
    }
    (set != ValuesSet::default()).then_some(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_project() -> HashMap<Key, String> {
        let values: ResolvedValues = ipc::decode("ResolvedValues", br#"{"pr_mode":{"value":"ready","from":"default"},"auto_merge":{"value":"checks-pass","from":"kit"},"review_model":{"value":"opus","from":"kit"},"layout":{"from":"default"},"theme":{"from":"default"},"density":{"value":"default","from":"default"},"reviews":{"plans":"review","pull_requests":"off","walks":"off","from":"kit"}}"#)
            .expect("decodes");
        inherited(&values)
    }

    #[test]
    fn a_new_project_sets_only_what_was_changed() {
        let inherited = every_project();
        let mut picked = HashMap::new();
        picked.insert(Key::AutoMerge, "checks-pass".to_string());
        assert_eq!(values_of(&picked, &inherited), None, "a pick that is what every project uses sets nothing");
        picked.insert(Key::Density, "roomy".to_string());
        picked.insert(Key::Tracker, "linear".to_string());
        let set = values_of(&picked, &inherited).expect("two changed");
        assert_eq!(set.auto_merge, None);
        assert_eq!(set.look, Some(LookSet { density: Some("roomy".into()), ..LookSet::default() }));
        assert_eq!(set.tracker.and_then(|block| block.get("provider").cloned()).as_deref(), Some("linear"));
    }

    #[test]
    fn one_review_changed_sends_all_three() {
        let inherited = every_project();
        let picked: HashMap<Key, String> = [(Key::Walks, "review".to_string())].into_iter().collect();
        let reviews = values_of(&picked, &inherited).and_then(|set| set.reviews).expect("reviews");
        assert_eq!((reviews.plans.as_deref(), reviews.pull_requests.as_deref(), reviews.walks.as_deref()), (Some("review"), Some("off"), Some("review")));
    }

    #[test]
    fn delivery_reads_as_the_question_names_it() {
        let options = Key::Delivery.options(&[], &[]);
        assert_eq!(options.iter().map(|(_, label)| label.as_str()).collect::<Vec<_>>(), ["Ask", "Auto land", "Open PR only", "Keep local"]);
        assert_eq!(delivery_said("auto_land"), Some("Auto land"));
        assert_eq!(every_project().get(&Key::Delivery).map(String::as_str), Some(ASK), "no layer sets it: the project asks");
    }

    #[test]
    fn delivery_is_read_from_the_layer_that_sets_it_and_sent_where_it_changes() {
        let values: ResolvedValues = ipc::decode(
            "ResolvedValues",
            br#"{"pr_mode":{"value":"ready","from":"default"},"auto_merge":{"value":"checks-pass","from":"kit"},"review_model":{"value":"opus","from":"kit"},"layout":{"from":"default"},"theme":{"from":"default"},"density":{"value":"default","from":"default"},"delivery":{"value":"open_pr","from":"kit"}}"#,
        )
        .expect("decodes");
        let inherited = inherited(&values);
        assert_eq!(inherited.get(&Key::Delivery).map(String::as_str), Some("open_pr"));
        let pick = |word: &str| -> HashMap<Key, String> { [(Key::Delivery, word.to_string())].into_iter().collect() };
        assert_eq!(values_of(&pick("open_pr"), &inherited), None, "what every project uses is not sent");
        assert_eq!(values_of(&pick("keep_local"), &inherited).and_then(|set| set.delivery).as_deref(), Some("keep_local"));
        assert_eq!(values_of(&pick(ASK), &inherited).and_then(|set| set.delivery).as_deref(), Some(ASK), "Ask is a word Fleet keeps: it turns learning off");
    }

    #[test]
    fn tracker_offers_none_and_custom_and_custom_is_never_a_provider() {
        let labels: Vec<String> = Key::Tracker.options(&[], &[]).into_iter().map(|(_, label)| label).collect();
        assert_eq!(labels, ["None", "GitHub Issues", "Linear", "Beads", "Custom"]);
        let inherited = every_project();
        let picked: HashMap<Key, String> = [(Key::Tracker, CUSTOM.to_string())].into_iter().collect();
        assert_eq!(values_of(&picked, &inherited), None, "Custom names no provider until its mod exists");
        let none: HashMap<Key, String> = [(Key::Tracker, String::new())].into_iter().collect();
        assert_eq!(values_of(&none, &inherited), None, "None states no tracker block");
        assert_eq!(Key::Tracker.said("", &[], &[]), "None");
    }

    #[test]
    fn layout_lists_named_layouts_and_reads_an_unlisted_one_as_custom() {
        let named = vec![(r#"{"version":1}"#.to_string(), "quiet".to_string())];
        let labels: Vec<String> = Key::Layout.options(&[], &named).into_iter().map(|(_, label)| label).collect();
        assert_eq!(labels, ["Default", "quiet"]);
        assert_eq!(Key::Layout.said(r#"{"version":1}"#, &[], &named), "quiet");
        let raw = r#"{"rail":{"hidden":["studios"]},"version":1}"#;
        assert_eq!(Key::Layout.said(raw, &[], &named), "Custom layout", "never raw JSON");
        let picked: HashMap<Key, String> = [(Key::Layout, raw.to_string())].into_iter().collect();
        let kept = values_of(&picked, &every_project()).and_then(|set| set.look).and_then(|look| look.layout);
        assert_eq!(kept.as_deref(), Some(raw), "kept as it was");
    }
}
