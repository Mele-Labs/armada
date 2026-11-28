//! An area made, renamed, split or merged, his rejects of what the catalog put together, and the rules his reasons
//! became. `docs/concepts/feature-catalog.md`, *Areas* and *His rules*.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// `change_area`: one act on one of a project's areas, with his reason where he gave one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeArea {
    /// A project's id.
    pub project: String,
    /// The slug of the area acted on, or the new area's for `make`.
    pub area: String,
    pub change: AreaChange,
    /// His words for why. Kept as a rule of the project's catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// What `change_area` does to the area.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case")]
pub enum AreaChange {
    /// A new area, after every area the project keeps.
    Make { title: String },
    /// Its new slug, its new title, or both.
    Rename {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slug: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A new area kept right after it, and the features named moved into the new one.
    Split { slug: String, title: String, features: Vec<String> },
    /// Every feature it holds moved into the area `into`, and the area gone.
    Merge { into: String },
}

/// What `change_area` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaChanged {
    pub project: String,
    /// The area as it is now: the made, renamed or split off area, or the one merged into.
    pub area: String,
    /// The rule his reason became, where he gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// What he rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogReject {
    /// Not this feature: a feature or area that is not one, or paths that are not part of the feature.
    NotThisFeature,
    /// Doesn't go together: features, areas or paths the catalog should keep apart.
    NotTogether,
}

/// `reject_in_catalog`: his Not this feature or Doesn't go together. Keeps a rule and changes nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectInCatalog {
    /// A project's id.
    pub project: String,
    pub reject: CatalogReject,
    /// Feature slugs, or slugs they were known by.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Area slugs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<String>,
    /// Paths relative to the repository, as he saw them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// His words for why. Absent, the rule says what he rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// The act a rule was kept with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogRuleAct {
    MakeArea,
    RenameArea,
    SplitArea,
    MergeAreas,
    MergeFeatures,
    RenameFeature,
    MoveFeature,
    NotThisFeature,
    NotTogether,
}

/// One rule of a project's catalog: his reason, what it was about, and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRule {
    pub id: String,
    /// His words, or for a reject he gave none for, what he rejected.
    pub text: String,
    pub act: CatalogRuleAct,
    /// The feature slugs it was about, as they were then.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// The area slugs it was about, as they were then.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    pub at: Instant,
}

/// `list_catalog_rules` and `drop_catalog_rule`: a project's rules, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRules {
    pub project: String,
    pub rules: Vec<CatalogRule>,
}

/// `reject_in_catalog`'s answer: the rule kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRuleKept {
    pub project: String,
    pub rule: CatalogRule,
}

/// `drop_catalog_rule`: one rule taken out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropCatalogRule {
    /// A project's id.
    pub project: String,
    /// The rule's id.
    pub rule: String,
}
