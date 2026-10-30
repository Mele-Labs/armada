//! The rule part of a teaching: a lifecycle rule, when, if and do, with an autonomy, read from the
//! model's answer and kept as a rule piece in Kit, as the Workflows page keeps one.
//!
//! **The rule is held to what a rule piece is held to when Fleet reads it** (`rule_as_read`): its file
//! parses, and it names no lifecycle or state the repository's lifecycles lack. The model is given
//! those lifecycles, the steps of the repository's workflows and the acts Fleet runs, and a rule
//! outside them is refused. A condition it cannot test comes back in `untestable`, in the person's
//! words, and a rule saved with some is saved without them.
//!
//! **Fleet chooses the scope and the id**: the scope is the reach the person picked, and the id is
//! made from the name, so a rule is never placed or named by a model.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::PieceWritten;
use core_model::{Lifecycle, ManifestId, Ulid};
use ipc::{LifecycleRule, PieceScope, ProposedRule, RuleAct, RuleCondition, RuleDoing, RuleOnFailure, RuleWhen, TeachingReach};
use serde::Deserialize;

use super::answer::{Context, Unfit, MOST_NAME};
use crate::daemon::Fleet;
use crate::piece_places::{files_in, RULES};
use crate::piece_saving::rule_as_read;

/// What a rule is read against, and what the model is told it may name.
pub(crate) struct Ground {
    /// The lifecycles that apply to the repository the teaching is in.
    pub(crate) lifecycles: Vec<Lifecycle>,
    /// The repository's workflows, each as its slug, its name and its steps by id and label.
    pub(crate) workflows: Vec<(String, String, Vec<(String, String)>)>,
    /// Every rule id that applies to the repository or that Kit keeps, since a rule of an id that
    /// applies beside it would replace it or be replaced.
    pub(crate) taken: Vec<String>,
}

impl Ground {
    /// The lifecycles and workflows as the question names them.
    pub(crate) fn listed(&self) -> String {
        let mut lines = vec!["Lifecycles, each with its states as id (label):".to_string()];
        for lifecycle in &self.lifecycles {
            let states: Vec<String> =
                lifecycle.states().iter().map(|state| format!("{} ({})", state.id().as_str(), state.label().as_str())).collect();
            lines.push(format!("- {} ({}): {}", lifecycle.id().as_str(), lifecycle.name().as_str(), states.join(", ")));
        }
        lines.push("Workflows, each with its steps as id (label):".to_string());
        if self.workflows.is_empty() {
            lines.push("- none".to_string());
        }
        for (id, name, steps) in &self.workflows {
            let steps: Vec<String> = steps.iter().map(|(step, label)| format!("{step} ({label})")).collect();
            lines.push(format!("- {id} ({name}): {}", steps.join(", ")));
        }
        lines.join("\n")
    }
}

/// A rule as the model wrote it. **No id, no scope and no per-act kind or autonomy**: Fleet gives
/// those, so none of them is a model's to get wrong.
#[derive(Deserialize)]
pub(crate) struct SaidRule {
    name: String,
    when: RuleWhen,
    #[serde(default, rename = "if")]
    conditions: Vec<SaidCondition>,
    #[serde(rename = "do")]
    does: Vec<RuleAct>,
    #[serde(default)]
    autonomy: Option<String>,
    #[serde(default)]
    on_failure: Option<RuleOnFailure>,
    #[serde(default)]
    untestable: Option<Vec<String>>,
}

/// A condition, or the person's words for one Fleet cannot test, which a model that did not leave
/// it out of `if` put there as a string.
#[derive(Deserialize)]
#[serde(untagged)]
enum SaidCondition {
    Words(String),
    Held(RuleCondition),
}

/// Where a rule taught with this reach is kept: the repository's Manifest id, or every repository.
pub(crate) fn scope_for(reach: TeachingReach, manifest: Option<&str>) -> Result<PieceScope, String> {
    match (reach, manifest) {
        (TeachingReach::Everywhere, _) => Ok(PieceScope::Everywhere),
        (TeachingReach::Repository, Some(id)) => Ok(PieceScope::Within { id: id.to_string() }),
        (TeachingReach::Repository, None) => Err("a rule for one repository needs the repository".to_string()),
    }
}

/// An id made of `name`: lower case letters and digits joined by `_`, starting with a letter, and
/// `_2`, `_3` and on where `taken` holds it. **The editor's rule for a new rule's id.**
pub(crate) fn id_from(name: &str, taken: &[String]) -> String {
    let mut base = String::new();
    for one in name.chars().flat_map(char::to_lowercase) {
        if one.is_ascii_alphanumeric() {
            base.push(one);
        } else if !base.is_empty() && !base.ends_with('_') {
            base.push('_');
        }
    }
    let base = base.trim_end_matches('_');
    let base = match base.chars().next() {
        None => "rule".to_string(),
        Some(first) if first.is_ascii_lowercase() => base.to_string(),
        Some(_) => format!("rule_{base}"),
    };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    id
}

/// Why a name cannot be a rule's.
fn name_fault(name: &str) -> Option<String> {
    let name = name.trim();
    match name.is_empty() || name.chars().count() > MOST_NAME || name.chars().any(char::is_control) {
        true => Some(format!("the name is 1 to {MOST_NAME} characters on one line")),
        false => None,
    }
}

/// The rule piece a rule would be kept as, with the scope and the id Fleet gives it, **held to a
/// read of its file**. The rule as that read answers it.
pub(crate) fn kept(rule: &LifecycleRule, reach: TeachingReach, manifest: Option<&str>, ground: &Ground) -> Result<LifecycleRule, String> {
    if let Some(why) = name_fault(&rule.name) {
        return Err(why);
    }
    let placed = LifecycleRule {
        id: id_from(rule.name.trim(), &ground.taken),
        name: rule.name.trim().to_string(),
        scope: scope_for(reach, manifest)?,
        ..rule.clone()
    };
    rule_as_read(&placed, &ground.lifecycles)
}

/// The rule an answer proposes, or why it is not one.
pub(crate) fn read(said: SaidRule, at: &Context) -> Result<ProposedRule, Unfit> {
    let ground = at.rule.ok_or_else(|| Unfit::Rule("a rule needs the lifecycles it is for".to_string()))?;
    let mut untestable: Vec<String> = Vec::new();
    let mut said_untestable = |words: &str| {
        let words = words.trim();
        if !words.is_empty() && !untestable.iter().any(|held| held == words) {
            untestable.push(words.to_string());
        }
    };
    for words in said.untestable.iter().flatten() {
        said_untestable(words);
    }
    let mut conditions = Vec::new();
    for one in said.conditions {
        match one {
            SaidCondition::Held(condition) => conditions.push(condition),
            SaidCondition::Words(words) => said_untestable(&words),
        }
    }
    let defaults = core_model::rule::OnFailure::default();
    let wanted = LifecycleRule {
        id: String::new(),
        name: said.name,
        scope: PieceScope::Everywhere,
        when: said.when,
        conditions,
        // The kind and the autonomy of each act are worked out by the read of the file.
        does: said.does.into_iter().map(|what| RuleDoing { what, kind: String::new(), autonomy: String::new() }).collect(),
        autonomy: said.autonomy.map(|word| word.trim().to_string()).filter(|word| !word.is_empty()),
        on_failure: said
            .on_failure
            .unwrap_or_else(|| RuleOnFailure { retries: defaults.retries.times(), then: defaults.then.as_wire().to_string() }),
    };
    let rule = kept(&wanted, at.reach, at.manifest, ground).map_err(Unfit::Rule)?;
    Ok(ProposedRule { rule, untestable })
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
    /// What a rule taught in the repository at `root` is read against. **A teaching with no
    /// repository has the four lifecycles Armada carries**, and no workflow.
    pub(crate) fn rule_ground(&self, root: Option<&str>) -> Ground {
        let manifest = root.and_then(|root| self.manifest_id_at(root));
        let lifecycles = match &manifest {
            Some(manifest) => self.lifecycles_that_apply(manifest),
            None => Lifecycle::built_in(),
        };
        let workflows = root
            .and_then(|root| self.repositories().served().into_iter().find(|one| one.root() == root))
            .map(|served| {
                served
                    .workflows()
                    .values()
                    .map(|workflow| {
                        let steps = crate::wire::declared(workflow).into_iter().map(|step| (step.step_id.as_str().to_string(), step.label)).collect();
                        (workflow.id().as_str().to_string(), workflow.name().to_string(), steps)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut taken: BTreeSet<String> = files_in(&self.kit_pieces(RULES), PieceWritten::in_kit)
            .iter()
            .filter_map(|file| config::parse_rule(file.path(), file.text()).ok())
            .map(|rule| rule.id().as_str().to_string())
            .collect();
        if let Some(manifest) = &manifest {
            if let Ok(applying) = self.rules_applying_to(&ManifestId::carried(Ulid::carried(manifest))) {
                taken.extend(applying.files().iter().map(|file| file.rule().id().as_str().to_string()));
            }
        }
        Ground { lifecycles, workflows, taken: taken.into_iter().collect() }
    }
}
