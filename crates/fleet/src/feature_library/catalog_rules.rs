//! His reasons for how a project's catalog is shaped, kept as the catalog's rules, and his Not this feature and Doesn't go
//! together. `docs/concepts/feature-catalog.md`, *His rules*.
//!
//! **A rule reaches whoever shapes the catalog by being in what it is given**: the librarian's question and the Operator's
//! catalog tells carry [`rules_text`] whole, so neither has to choose to read them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CatalogReject, CatalogRule, CatalogRuleAct, CatalogRuleKept, CatalogRules, DropCatalogRule, Instant, RejectInCatalog};
use store::{catalog_rule_act as act, KeptCatalogRule, Store, WriteError};

use super::reading::Loaded;
use super::writing::AREA_UNKNOWN;
use crate::daemon::Fleet;

/// A reject that names too little, or paths with no one feature.
const REJECT: &str = "fleet.catalog_reject";
/// A rule the project does not hold.
const RULE_UNKNOWN: &str = "fleet.catalog_rule_unknown";
/// The most rules a prompt carries, newest first.
const MOST_RULES: usize = 40;

/// What a rule is about, as the act named it.
#[derive(Default)]
pub(super) struct About {
    pub(super) features: Vec<String>,
    pub(super) areas: Vec<String>,
    pub(super) paths: Vec<String>,
}

fn act_name(one: CatalogRuleAct) -> &'static str {
    match one {
        CatalogRuleAct::MakeArea => act::MAKE_AREA,
        CatalogRuleAct::RenameArea => act::RENAME_AREA,
        CatalogRuleAct::SplitArea => act::SPLIT_AREA,
        CatalogRuleAct::MergeAreas => act::MERGE_AREAS,
        CatalogRuleAct::MergeFeatures => act::MERGE_FEATURES,
        CatalogRuleAct::RenameFeature => act::RENAME_FEATURE,
        CatalogRuleAct::MoveFeature => act::MOVE_FEATURE,
        CatalogRuleAct::NotThisFeature => act::NOT_THIS_FEATURE,
        CatalogRuleAct::NotTogether => act::NOT_TOGETHER,
    }
}

fn act_of(name: &str) -> Option<CatalogRuleAct> {
    [
        CatalogRuleAct::MakeArea,
        CatalogRuleAct::RenameArea,
        CatalogRuleAct::SplitArea,
        CatalogRuleAct::MergeAreas,
        CatalogRuleAct::MergeFeatures,
        CatalogRuleAct::RenameFeature,
        CatalogRuleAct::MoveFeature,
        CatalogRuleAct::NotThisFeature,
        CatalogRuleAct::NotTogether,
    ]
    .into_iter()
    .find(|one| act_name(*one) == name)
}

/// A kept rule as the wire carries it. A row whose act this Fleet does not know is left out.
fn wired(kept: KeptCatalogRule) -> Option<CatalogRule> {
    Some(CatalogRule {
        id: kept.id,
        text: kept.text,
        act: act_of(&kept.act)?,
        features: kept.features,
        areas: kept.areas,
        paths: kept.paths,
        at: Instant::carried(&kept.at),
    })
}

/// His words on one line, `None` where there are none.
fn said(why: Option<&str>) -> Option<String> {
    why.map(|why| why.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|why| !why.is_empty())
}

/// `a`, `a and b`, `a, b and c`.
fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn ticked(names: &[String]) -> String {
    listed(&names.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>())
}

/// What he did, for the line a prompt carries.
fn what(rule: &KeptCatalogRule) -> String {
    let (features, areas, paths) = (&rule.features, &rule.areas, &rule.paths);
    let two = |names: &[String]| match names {
        [from, to, ..] => (format!("`{from}`"), format!("`{to}`")),
        [one] => (format!("`{one}`"), format!("`{one}`")),
        [] => (String::new(), String::new()),
    };
    let mut parts = match rule.act.as_str() {
        act::MAKE_AREA => format!("he made the area {}", ticked(areas)),
        act::RENAME_AREA => {
            let (from, to) = two(areas);
            if from == to { format!("he retitled the area {from}") } else { format!("he renamed the area {from} to {to}") }
        }
        act::SPLIT_AREA => {
            let (from, to) = two(areas);
            format!("he split {to} out of the area {from} with {}", ticked(features))
        }
        act::MERGE_AREAS => {
            let (from, to) = two(areas);
            format!("he merged the area {from} into {to}")
        }
        act::MERGE_FEATURES => {
            let (from, to) = two(features);
            format!("he merged the feature {from} into {to}")
        }
        act::RENAME_FEATURE => {
            let (from, to) = two(features);
            let named = if from == to { format!("he retitled the feature {from}") } else { format!("he renamed the feature {from} to {to}") };
            if areas.is_empty() { named } else { format!("{named} and moved it to the area {}", ticked(areas)) }
        }
        act::MOVE_FEATURE => format!("he moved the feature {} to the area {}", ticked(features), ticked(areas)),
        act::NOT_TOGETHER => "he said these do not go together".to_string(),
        _ => "he said this is not the feature".to_string(),
    };
    if matches!(rule.act.as_str(), act::NOT_THIS_FEATURE | act::NOT_TOGETHER) {
        for (kind, names) in [("features", features), ("areas", areas), ("paths", paths)] {
            if !names.is_empty() {
                parts.push_str(&format!("; {kind} {}", ticked(names)));
            }
        }
    }
    parts
}

/// The project's rules, newest first, as the block a prompt carries. Empty where it keeps none.
pub(crate) fn rules_text(rules: &[KeptCatalogRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }
    let mut text = String::from("\nThe owner's rules for this catalog, newest first. Follow every one when you draft, file, merge, split or name features and areas:\n");
    for rule in rules.iter().take(MOST_RULES) {
        let day = rule.at.get(..10).unwrap_or(&rule.at);
        text.push_str(&format!("- {} ({}, {day})\n", rule.text.split_whitespace().collect::<Vec<_>>().join(" "), what(rule)));
    }
    text
}

/// `project`'s rules as [`rules_text`] gives them, empty where they cannot be read.
pub(crate) fn rules_of(store: &Store, project: &str) -> String {
    rules_text(&store.catalog_rules(project).unwrap_or_default())
}

/// What he rejected, said as a fact, for a reject he gave no words for.
fn rejected_text(reject: CatalogReject, loaded: &Loaded, about: &About) -> String {
    let feature = |slug: &String| loaded.feature(slug).map_or_else(|| slug.clone(), |one| one.title.clone());
    let area = |slug: &String| loaded.areas.iter().find(|one| one.slug == *slug).map_or_else(|| slug.clone(), |one| one.title.clone());
    match reject {
        CatalogReject::NotTogether => {
            let names: Vec<String> = about.features.iter().map(feature).chain(about.areas.iter().map(area)).chain(about.paths.iter().cloned()).collect();
            format!("{} do not go together.", listed(&names))
        }
        CatalogReject::NotThisFeature if !about.paths.is_empty() => {
            let verb = if about.paths.len() == 1 { "is" } else { "are" };
            format!("{} {verb} not part of {}.", listed(&about.paths), about.features.first().map(feature).unwrap_or_default())
        }
        CatalogReject::NotThisFeature => {
            let mut lines: Vec<String> = about.features.iter().map(|slug| format!("{} is not a feature.", feature(slug))).collect();
            lines.extend(about.areas.iter().map(|slug| format!("{} is not an area.", area(slug))));
            lines.join(" ")
        }
    }
}

fn trimmed(names: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for one in names.iter().map(|one| one.trim()).filter(|one| !one.is_empty()) {
        if !kept.iter().any(|held| held == one) {
            kept.push(one.to_string());
        }
    }
    kept
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
    /// Keep his words as a rule of `project`'s catalog, about what the act named: the rule's id, `None` where he gave no words.
    pub(super) fn rule_kept(&self, store: &mut Store, project: &str, why: Option<&str>, made_by: CatalogRuleAct, about: About) -> Result<Option<String>, WriteError> {
        let Some(text) = said(why) else { return Ok(None) };
        self.rule_kept_as(store, project, text, made_by, about).map(|kept| Some(kept.id))
    }

    fn rule_kept_as(&self, store: &mut Store, project: &str, text: String, made_by: CatalogRuleAct, about: About) -> Result<KeptCatalogRule, WriteError> {
        let kept = KeptCatalogRule {
            id: self.minted(),
            project: project.to_string(),
            text,
            act: act_name(made_by).to_string(),
            features: about.features,
            areas: about.areas,
            paths: about.paths,
            at: self.now().as_str().to_string(),
        };
        store.keep_catalog_rule(&kept)?;
        Ok(kept)
    }

    fn rules_read(&self, store: &Store, project: &str) -> Result<CatalogRules, Refusal> {
        let rules = store.catalog_rules(project).map_err(|why| self.ledger_fault(why))?;
        Ok(CatalogRules { project: project.to_string(), rules: rules.into_iter().filter_map(wired).collect() })
    }

    /// `list_catalog_rules`.
    pub(super) async fn catalog_rules_listed(&self, project: String) -> Result<CatalogRules, Refusal> {
        let project = self.asked_project(&project)?;
        let store = self.store().lock().await;
        self.rules_read(&store, &project)
    }

    /// `drop_catalog_rule`.
    pub(super) async fn catalog_rule_dropped(&self, caller: Option<api::Caller>, drop: DropCatalogRule) -> Result<CatalogRules, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&drop.project)?;
        let mut store = self.store().lock().await;
        if !store.drop_catalog_rule(&project, drop.rule.trim()).map_err(|why| self.ledger_fault(why))? {
            let message = format!("project `{project}` keeps no catalog rule `{}`", drop.rule.trim());
            return Err(Refusal::NotFound(ipc::WireError::raised(RULE_UNKNOWN, message, self.run_id())));
        }
        self.rules_read(&store, &project)
    }

    /// `reject_in_catalog`. **Keeps a rule and changes nothing else.**
    pub(super) async fn catalog_rejected(&self, caller: Option<api::Caller>, reject: RejectInCatalog) -> Result<CatalogRuleKept, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = self.asked_project(&reject.project)?;
        let (features, areas, paths) = (trimmed(&reject.features), trimmed(&reject.areas), trimmed(&reject.paths));
        match reject.reject {
            CatalogReject::NotThisFeature if features.is_empty() && areas.is_empty() => {
                return Err(self.hosted_refusal(REJECT, "Not this feature names a feature or an area"));
            }
            CatalogReject::NotThisFeature if !paths.is_empty() && (features.len() != 1 || !areas.is_empty()) => {
                return Err(self.hosted_refusal(REJECT, "paths that are not part of a feature name that one feature and no area"));
            }
            CatalogReject::NotTogether if features.len() + areas.len() + paths.len() < 2 => {
                return Err(self.hosted_refusal(REJECT, "Doesn't go together names two things or more: features, areas or paths"));
            }
            _ => {}
        }
        let mut store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        let mut slugs = Vec::new();
        for one in &features {
            let slug = loaded.feature_named(one).ok_or_else(|| self.feature_unknown(one))?.slug.clone();
            if !slugs.contains(&slug) {
                slugs.push(slug);
            }
        }
        if let Some(area) = areas.iter().find(|area| !loaded.areas.iter().any(|kept| kept.slug == **area)) {
            return Err(self.hosted_refusal(AREA_UNKNOWN, &format!("the project keeps no area `{area}`")));
        }
        let about = About { features: slugs, areas, paths };
        let made_by = match reject.reject {
            CatalogReject::NotThisFeature => CatalogRuleAct::NotThisFeature,
            CatalogReject::NotTogether => CatalogRuleAct::NotTogether,
        };
        let text = said(reject.why.as_deref()).unwrap_or_else(|| rejected_text(reject.reject, &loaded, &about));
        let kept = self.rule_kept_as(&mut store, &project, text, made_by, about).map_err(|why| self.ledger_fault(why))?;
        let rule = CatalogRule {
            id: kept.id,
            text: kept.text,
            act: made_by,
            features: kept.features,
            areas: kept.areas,
            paths: kept.paths,
            at: Instant::carried(&kept.at),
        };
        Ok(CatalogRuleKept { project, rule })
    }
}
