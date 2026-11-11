//! The values a person sets for how work behaves, laid over each other, the nearest winning:
//! **default, Machine, Kit, repository, Project, Job**. A key's answer names the layer it came
//! from, so a surface can say whose word it is.
//!
//! A Project sets them key by key over Kit, in a `values:` block of its own file. Kit's are a
//! `values.yml` of the same shape. How a repository *runs* (commands, Checks, ports) is not here:
//! it stays the repository's `armada.yml`. **Autonomy keeps the places it already has** (a piece
//! scoped to every repository, to a Project or to one repository) and is only read here, so it has no
//! second home; see [`AutonomyLayers`].
//!
//! **Pure.** Text in, values out, and no file or clock is touched, so a test drives every layer.

use std::path::Path;

use core_model::rule::{ActKind, Autonomy, AutonomyDefaults};
use core_model::{AutoMerge, Layer, Look, PrMode, ReviewMode, Reviews, TrackerNamed, Values, WalkReview};
use serde_yaml_ng::{Mapping, Value};

use crate::error::{Fault, LoadError, Refusal};
use crate::settings::{SHIPPED_THEME, SPACINGS};
use crate::yaml::{self, Table};

const KEYS: &[&str] = &["pr_mode", "auto_merge", "review_model", "look", "tracker", "reviews"];
/// Kit's keys: a tracker is a Project's, so Kit's block refuses it as it does any key it has no word for.
const KIT_KEYS: &[&str] = &["pr_mode", "auto_merge", "review_model", "look", "reviews"];
const LOOK: &[&str] = &["layout", "theme", "density"];
const REVIEWS: &[&str] = &["plans", "pull_requests", "walks"];
const REVIEW_MODES: &[(&str, ReviewMode)] =
    &[("off", ReviewMode::Off), ("review", ReviewMode::Review), ("review_and_approve", ReviewMode::ReviewAndApprove)];
const REVIEW_MODES_LEGAL: &[&str] = &["off", "review", "review_and_approve"];
/// A walk takes no `review_and_approve`: it is the person's to look at.
const WALK_REVIEWS: &[(&str, WalkReview)] = &[("off", WalkReview::Off), ("review", WalkReview::Review)];
const WALK_REVIEWS_LEGAL: &[&str] = &["off", "review"];
const KIT_FILE: &[&str] = &["version", "values"];
const PR_MODES: &[(&str, PrMode)] = &[("draft", PrMode::Draft), ("ready", PrMode::Ready)];
const PR_MODES_LEGAL: &[&str] = &["draft", "ready"];
const AUTO_MERGES: &[(&str, AutoMerge)] =
    &[("never", AutoMerge::Never), ("checks-pass", AutoMerge::ChecksPass), ("always", AutoMerge::Always)];
const AUTO_MERGES_LEGAL: &[&str] = &["never", "checks-pass", "always"];

/// The `values:` block at `key`, refusing every fault with its key. **A key not set is not held**,
/// and a refused one reads as not set, since a file with any refusal does not load. **`tracker` is read
/// only where `with_tracker`**, which is a Project's file and not Kit's.
pub(crate) fn read_block(key: &str, root: &Value, with_tracker: bool, out: &mut Vec<Refusal>) -> Values {
    let Some(mut table) = Table::open(key, root, out) else { return Values::default() };
    let pr_mode = table
        .optional("pr_mode")
        .and_then(|value| yaml::word(&table.at("pr_mode"), value, PR_MODES, PR_MODES_LEGAL, PR_MODES_LEGAL, out));
    let auto_merge = table
        .optional("auto_merge")
        .and_then(|value| yaml::word(&table.at("auto_merge"), value, AUTO_MERGES, AUTO_MERGES_LEGAL, AUTO_MERGES_LEGAL, out));
    let review_model = table.optional("review_model").and_then(|value| yaml::text(&table.at("review_model"), value, out));
    let look_at = table.at("look");
    let look = table.optional("look").map(|value| read_look(&look_at, value, out)).unwrap_or_default();
    let tracker = if with_tracker {
        crate::manifest::tracker::read(&mut table, out).map(|block| TrackerNamed {
            provider: block.provider().map(str::to_string),
            keys: block.keys().clone(),
        })
    } else {
        None
    };
    let reviews_at = table.at("reviews");
    let reviews = table.optional("reviews").map(|value| read_reviews(&reviews_at, value, out));
    table.close(if with_tracker { KEYS } else { KIT_KEYS }, out);
    Values { pr_mode, auto_merge, review_model: review_model.map(|model| model.trim().to_string()), look, tracker, reviews }
}

/// The `reviews:` block. **A kind it does not name is Off.**
fn read_reviews(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Reviews {
    let Some(mut table) = Table::open(key, root, out) else { return Reviews::default() };
    let mode = |table: &mut Table<'_>, name: &str, out: &mut Vec<Refusal>| {
        let at = table.at(name);
        table.optional(name).and_then(|value| yaml::word(&at, value, REVIEW_MODES, REVIEW_MODES_LEGAL, REVIEW_MODES_LEGAL, out)).unwrap_or_default()
    };
    let plans = mode(&mut table, "plans", out);
    let pull_requests = mode(&mut table, "pull_requests", out);
    let walks_at = table.at("walks");
    let walks = table
        .optional("walks")
        .and_then(|value| yaml::word(&walks_at, value, WALK_REVIEWS, WALK_REVIEWS_LEGAL, WALK_REVIEWS_LEGAL, out))
        .unwrap_or_default();
    table.close(REVIEWS, out);
    Reviews { plans, pull_requests, walks }
}

fn read_look(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Look {
    let Some(mut table) = Table::open(key, root, out) else { return Look::default() };
    let layout = table.optional("layout").and_then(|value| yaml::text(&table.at("layout"), value, out));
    let theme = table.optional("theme").and_then(|value| yaml::text(&table.at("theme"), value, out));
    let density_at = table.at("density");
    let density = table.optional("density").and_then(|value| yaml::text(&density_at, value, out)).and_then(|found| {
        if SPACINGS.contains(&found.as_str()) {
            return Some(found);
        }
        out.push(Refusal::new(density_at.clone(), Fault::NotInTheSchema { value: found, legal: SPACINGS }));
        None
    });
    table.close(LOOK, out);
    Look { layout, theme, density }
}

/// The `values:` block as YAML, holding only the keys `values` sets. `None` where it sets none.
pub(crate) fn block_value(values: &Values) -> Option<Value> {
    let text = |written: &str| Value::String(written.to_string());
    let mut block = Mapping::new();
    if let Some(mode) = values.pr_mode {
        block.insert(text("pr_mode"), text(mode.as_wire()));
    }
    if let Some(policy) = values.auto_merge {
        block.insert(text("auto_merge"), text(policy.as_written()));
    }
    if let Some(model) = &values.review_model {
        block.insert(text("review_model"), text(model));
    }
    let mut look = Mapping::new();
    for (name, held) in [("layout", &values.look.layout), ("theme", &values.look.theme), ("density", &values.look.density)] {
        if let Some(held) = held {
            look.insert(text(name), text(held));
        }
    }
    if !look.is_empty() {
        block.insert(text("look"), Value::Mapping(look));
    }
    if let Some(tracker) = &values.tracker {
        let mut named = Mapping::new();
        if let Some(provider) = &tracker.provider {
            named.insert(text("provider"), text(provider));
        }
        for (name, held) in &tracker.keys {
            named.insert(text(name), text(held));
        }
        block.insert(text("tracker"), Value::Mapping(named));
    }
    if let Some(reviews) = &values.reviews {
        let mut named = Mapping::new();
        named.insert(text("plans"), text(reviews.plans.as_wire()));
        named.insert(text("pull_requests"), text(reviews.pull_requests.as_wire()));
        named.insert(text("walks"), text(reviews.walks.as_wire()));
        block.insert(text("reviews"), Value::Mapping(named));
    }
    (!block.is_empty()).then_some(Value::Mapping(block))
}

/// Read Kit's `values.yml` from its text: `version: 1` and a `values:` block. **An empty file is not
/// one**; an absent file is what sets nothing, and is the caller's to say. `path` is only named in a refusal.
pub fn parse_kit_values(path: &Path, text: &str) -> Result<Values, LoadError> {
    let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml { path: path.to_path_buf(), cause })?;
    let mut out = Vec::new();
    let mut values = Values::default();
    if let Some(mut top) = Table::open("", &root, &mut out) {
        if let Some(version) = top.required("version", &mut out).and_then(|value| yaml::positive("version", value, &mut out)) {
            if version != 1 {
                out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried: &["1"] }));
            }
        }
        if let Some(block) = top.optional("values") {
            values = read_block("values", block, false, &mut out);
        }
        top.close(KIT_FILE, &mut out);
    }
    match out.is_empty() {
        true => Ok(values),
        false => Err(LoadError::Refused { path: path.to_path_buf(), refusals: out }),
    }
}

/// `values` as the text of Kit's `values.yml`.
pub fn kit_values_text(values: &Values) -> Result<String, String> {
    let mut top = Mapping::new();
    top.insert(Value::String("version".into()), Value::Number(1.into()));
    top.insert(Value::String("values".into()), block_value(values).unwrap_or_else(|| Value::Mapping(Mapping::new())));
    crate::piece_text(&Value::Mapping(top))
}

/// What Armada ships when nothing else speaks: pull requests ready, no machine merging, the shipped
/// theme and the default density. `review_model` and `layout` have none of their own, and the
/// caller adds the review model the Machine's settings ship.
pub fn shipped_values() -> Values {
    Values {
        pr_mode: Some(PrMode::Ready),
        auto_merge: Some(AutoMerge::Never),
        review_model: None,
        look: Look { layout: None, theme: Some(SHIPPED_THEME.to_string()), density: Some("default".to_string()) },
        tracker: None,
        reviews: None,
    }
}

/// One key's answer: the value, and the layer it came from. **`None` is a key no layer set and the
/// default has no word for**, and then `from` is [`Layer::Default`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedValue<T> {
    pub value: Option<T>,
    pub from: Layer,
}

/// What autonomy says for one kind of act, and the layer it came from: `Kit`, `Project` or
/// `Repository`, or the kind's own default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedAutonomy {
    pub kind: ActKind,
    pub value: Autonomy,
    pub from: Layer,
}

/// The autonomy pieces that apply, by the place each is scoped to: every repository (Kit's own
/// and anywhere else), a Project, and one repository. **Read as `Autonomies` holds them**, which is the
/// only home autonomy has. A repository's laid over its Project's, as the pieces always were.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutonomyLayers {
    pub kit: AutonomyDefaults,
    pub project: AutonomyDefaults,
    pub repository: AutonomyDefaults,
}

/// What each layer sets. `job` is what the Job itself was started with: nothing sets one yet, and it
/// is the nearest, so the day something does it wins without a change here.
#[derive(Clone, Debug, Default)]
pub struct Layers {
    pub default: Values,
    pub machine: Values,
    pub kit: Values,
    pub repository: Values,
    pub project: Values,
    pub job: Values,
    pub autonomy: AutonomyLayers,
}

/// Every key resolved, each with its layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedValues {
    pub pr_mode: ResolvedValue<PrMode>,
    pub auto_merge: ResolvedValue<AutoMerge>,
    pub review_model: ResolvedValue<String>,
    pub layout: ResolvedValue<String>,
    pub theme: ResolvedValue<String>,
    pub density: ResolvedValue<String>,
    /// The tracker a layer named, a Project's over its repository's. **`None` is the forge's own**, or the
    /// repository's where this is a Job's: no Machine, Kit or default layer names one.
    pub tracker: ResolvedValue<TrackerNamed>,
    /// How work is reviewed before the person sees it. **`None` is Off for every kind**: no layer sets it and the
    /// default has no word for it.
    pub reviews: ResolvedValue<Reviews>,
    /// One for each kind of act, in the order of the kinds.
    pub autonomy: Vec<ResolvedAutonomy>,
}

/// The nearest layer that sets the key, else the default's word, else nothing.
fn nearest<T: Clone>(layers: &Layers, of: impl Fn(&Values) -> Option<T>) -> ResolvedValue<T> {
    let ordered = [
        (Layer::Job, &layers.job),
        (Layer::Project, &layers.project),
        (Layer::Repository, &layers.repository),
        (Layer::Kit, &layers.kit),
        (Layer::Machine, &layers.machine),
        (Layer::Default, &layers.default),
    ];
    for (layer, values) in ordered {
        if let Some(value) = of(values) {
            return ResolvedValue { value: Some(value), from: layer };
        }
    }
    ResolvedValue { value: None, from: Layer::Default }
}

/// Lay the layers over each other. **A Project's auto merge replaces the repository's**: nothing
/// is folded between them.
pub fn resolve(layers: &Layers) -> ResolvedValues {
    let autonomy = &layers.autonomy;
    ResolvedValues {
        pr_mode: nearest(layers, |values| values.pr_mode),
        auto_merge: nearest(layers, |values| values.auto_merge),
        review_model: nearest(layers, |values| values.review_model.clone()),
        layout: nearest(layers, |values| values.look.layout.clone()),
        theme: nearest(layers, |values| values.look.theme.clone()),
        density: nearest(layers, |values| values.look.density.clone()),
        tracker: nearest(layers, |values| values.tracker.clone()),
        reviews: nearest(layers, |values| values.reviews),
        autonomy: ActKind::ALL
            .iter()
            .map(|kind| {
                let from = [(Layer::Repository, &autonomy.repository), (Layer::Project, &autonomy.project), (Layer::Kit, &autonomy.kit)]
                    .into_iter()
                    .find_map(|(layer, set)| set.get(*kind).map(|value| (value, layer)));
                match from {
                    Some((value, from)) => ResolvedAutonomy { kind: *kind, value, from },
                    None => ResolvedAutonomy { kind: *kind, value: kind.default_autonomy(), from: Layer::Default },
                }
            })
            .collect(),
    }
}
