//! The values a person sets for how work behaves, and what each comes to once the layers are laid over
//! each other. `docs/practices/protocol/projects.md`, *Values*.
//!
//! **A key not set is absent**, so a project's `values` carries only what it sets. `look` is Bridge's:
//! Fleet keeps and serves the words and Bridge decides what each names.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::PrMode;

/// What one layer sets, a project's or Kit's: only the keys it sets, each absent where it says nothing.
/// `auto_merge` is `never`, `checks-pass` or `always`, and `density` is `compact`, `default` or `roomy`;
/// Fleet refuses a word that is not one, 422 `fleet.project_values_refused`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValuesSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_mode: Option<PrMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_merge: Option<String>,
    /// The model an agent run that reviews work runs on, as the agent spells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<LookSet>,
    /// The tracker a project names for its issues and for the Jobs in it, as the Manifest's `tracker:` block
    /// writes it: `provider` and every other key as text, carried as written. **A project's only**: Kit has none,
    /// and Fleet refuses one in Kit's values, 422 `fleet.project_values_refused`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracker: Option<BTreeMap<String, String>>,
    /// How the Operator reviews work before the person sees it, a kind at a time. **A layer sets only the kinds it
    /// names**: a kind it leaves out falls through to the next layer, project then Kit then `off`. Clearing one kind is
    /// sending the block without it, and a block naming none is the same as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviews: Option<ReviewsSet>,
    /// Whether a Job that a Job or the Operator starts begins at once (`at_once`) or waits on the person's start
    /// card (`ask_me`). Fleet refuses another word, 422 `fleet.project_values_refused`. A Job the person starts
    /// himself is never held by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_starts: Option<String>,
    /// How a Job in the project lands its work: `auto_land`, `open_pr`, `keep_local` or `ask`. Fleet refuses another
    /// word, 422 `fleet.project_values_refused`. Unset at every layer, the first answer to the delivery question
    /// is kept here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<String>,
}

/// How each kind of work is reviewed: `off`, `review` or `review_and_approve`, and for `walks` only `off` or
/// `review`. Fleet refuses another word, 422 `fleet.project_values_refused`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewsSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plans: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_requests: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walks: Option<String>,
}

/// How Bridge looks, each key absent where the layer says nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<String>,
}

/// The layer a value came from, from the furthest to the nearest: `default`, `machine`, `kit`,
/// `repository`, `project` or `job`. The nearest that sets a key wins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueLayer {
    #[default]
    Default,
    Machine,
    Kit,
    Repository,
    Project,
    Job,
}

/// One key's answer. **`value` is absent where no layer sets the key and the default has no word
/// for it**, and `from` is then `default`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub from: ValueLayer,
}

/// The tracker a layer named, and the layer. `value` is the block as written, `provider` and every other key,
/// and is absent where no layer names one: the forge's own, `from` then `default`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTracker {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<BTreeMap<String, String>>,
    pub from: ValueLayer,
}

/// How each kind of work is reviewed once the layers are laid over each other, and the layer that said so.
/// **Every kind is a word**: one no layer set is `off`, from `default`. Each kind falls through on its own, so a
/// project can set one and leave the others to Kit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedReviews {
    /// `off`, `review` or `review_and_approve`.
    pub plans: String,
    /// `off`, `review` or `review_and_approve`.
    pub pull_requests: String,
    /// `off` or `review`.
    pub walks: String,
    /// The nearest layer that set any kind, `default` where none did.
    pub from: ValueLayer,
    /// The layer the plans word came from. Absent in an answer from a Fleet that did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plans_from: Option<ValueLayer>,
    /// The layer the pull requests word came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_requests_from: Option<ValueLayer>,
    /// The layer the walks word came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walks_from: Option<ValueLayer>,
}

/// What autonomy comes to for one kind of act: `acts`, `asks` or `off`. `from` is `kit`, `project` or
/// `repository` where a piece sets it, else `default`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedAutonomy {
    /// `tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`.
    pub kind: String,
    pub value: String,
    pub from: ValueLayer,
}

/// Every key resolved, each with the layer it came from. `get_values`' answer, and what a Job froze at
/// its start (`hosted.values`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedValues {
    /// `ready` or `draft`.
    pub pr_mode: ResolvedValue,
    /// `never`, `checks-pass` or `always`.
    pub auto_merge: ResolvedValue,
    pub review_model: ResolvedValue,
    pub layout: ResolvedValue,
    pub theme: ResolvedValue,
    /// `compact`, `default` or `roomy`.
    pub density: ResolvedValue,
    /// The tracker for the repository asked about, a project's replacing its repository's. **Absent from what a
    /// Job froze**: a Job asks its tracker where it acts, and the project's word is read then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracker: Option<ResolvedTracker>,
    /// How the Operator reviews work before the person sees it. What a Job froze, too: **a Job is reviewed as
    /// its start said**, whatever the project says since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviews: Option<ResolvedReviews>,
    /// `at_once` or `ask_me`: how a Job that a Job or the Operator starts begins. Absent from what a Job froze
    /// before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_starts: Option<ResolvedValue>,
    /// `auto_land`, `open_pr`, `keep_local` or `ask`; the value is empty where no layer sets it. Absent from what a
    /// Job froze before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<ResolvedValue>,
    /// One for each kind of act, where a repository was asked about; empty where only a project was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub autonomy: Vec<ResolvedAutonomy>,
}

/// `save_kit_values`: Kit's values, whole. A key absent here is taken away.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveKitValues {
    pub values: ValuesSet,
}

/// `save_kit_values`' answer: where the file was written and what Kit now sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitValuesSaved {
    pub file: String,
    pub values: ValuesSet,
}
