//! A rule, declared once and checked once.
//!
//! **[`Rule::declared`] is the only way to make one**, and it refuses what no engine could make
//! sense of: a rule that does nothing, one that speaks of the instance that entered when its moment
//! has none, and a review request with nobody to ask. What it cannot know alone, whether the
//! lifecycles and states it names exist, is [`Rule::unknown_in`], asked by whoever holds the
//! lifecycles that apply.

use alloc::vec::Vec;
use core::fmt;

use crate::lifecycle::{Lifecycle, LifecycleId, LifecycleScope, StateId};
use crate::workflow::Title;

use super::act::{Act, ForgeWrite, TrackerWrite};
use super::autonomy::Autonomy;
use super::failure::OnFailure;
use super::ids::RuleId;
use super::select::{Condition, Target};
use super::when::When;

/// Everything a rule is made of, before it is checked.
///
/// **`scope` is a lifecycle's**: where a rule applies is the same question as where a lifecycle
/// does, answered by the same reading of the places a piece is kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleParts {
    pub id: RuleId,
    pub name: Title,
    pub scope: LifecycleScope,
    pub when: When,
    /// All of them must hold. None means the moment is enough.
    pub conditions: Vec<Condition>,
    pub does: Vec<Act>,
    /// This rule's own word, over the default for each kind of act. None takes the default.
    pub autonomy: Option<Autonomy>,
    pub on_failure: OnFailure,
}

/// When, If and Do, with an autonomy setting and a way to fail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    id: RuleId,
    name: Title,
    scope: LifecycleScope,
    when: When,
    conditions: Vec<Condition>,
    does: Vec<Act>,
    autonomy: Option<Autonomy>,
    on_failure: OnFailure,
}

impl Rule {
    /// A rule, or the first thing wrong with it.
    pub fn declared(parts: RuleParts) -> Result<Rule, MalformedRule> {
        if parts.does.is_empty() {
            return Err(MalformedRule::NoActs);
        }
        if !parts.when.has_an_instance() {
            let condition = parts.conditions.iter().filter_map(Condition::among);
            let target = parts.does.iter().filter_map(Act::target).map(|target| &target.among);
            if condition.chain(target).any(|among| among.needs_an_instance()) {
                return Err(MalformedRule::NoInstanceToSpeakOf);
            }
        }
        let reviewers = |act: &Act| matches!(act, Act::Forge { write: ForgeWrite::RequestReviewers { from }, .. } if from.is_empty());
        if parts.does.iter().any(reviewers) {
            return Err(MalformedRule::NobodyToAsk);
        }
        let RuleParts { id, name, scope, when, conditions, does, autonomy, on_failure } = parts;
        Ok(Rule { id, name, scope, when, conditions, does, autonomy, on_failure })
    }

    pub fn id(&self) -> &RuleId {
        &self.id
    }

    pub fn name(&self) -> &Title {
        &self.name
    }

    pub fn scope(&self) -> &LifecycleScope {
        &self.scope
    }

    pub fn when(&self) -> &When {
        &self.when
    }

    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    pub fn does(&self) -> &[Act] {
        &self.does
    }

    pub fn autonomy(&self) -> Option<Autonomy> {
        self.autonomy
    }

    pub fn on_failure(&self) -> OnFailure {
        self.on_failure
    }

    /// Every lifecycle the rule names, with the state it names in it where it names one.
    fn mentions(&self) -> Vec<(LifecycleId, Option<StateId>)> {
        let entered = self.when.lifecycle_entered();
        let mut named = Vec::new();
        if let When::Enters { lifecycle, state } = &self.when {
            named.push((lifecycle.clone(), Some(state.clone())));
        }
        for condition in &self.conditions {
            if let Condition::Instances { among, in_state, .. } = condition {
                named.extend(among.lifecycle(entered).map(|lifecycle| (lifecycle.clone(), Some(in_state.clone()))));
            }
        }
        for act in &self.does {
            let Some(Target { among, in_state }) = act.target() else { continue };
            let Some(lifecycle) = among.lifecycle(entered) else { continue };
            named.push((lifecycle.clone(), in_state.clone()));
            if let Act::Tracker { write: TrackerWrite::Move { to }, .. } = act {
                named.push((lifecycle.clone(), Some(to.clone())));
            }
        }
        named
    }

    /// What the rule names that none of `lifecycles` has. **A rule that names `pr: aproved` loads
    /// and never fires**, which is the worse way to be wrong, so whoever holds the lifecycles
    /// that apply asks and sets the rule aside with the sentence.
    pub fn unknown_in(&self, lifecycles: &[&Lifecycle]) -> Vec<UnknownName> {
        let mut unknown: Vec<UnknownName> = Vec::new();
        for (lifecycle, state) in self.mentions() {
            let found = lifecycles.iter().find(|held| held.id() == &lifecycle);
            let one = match (found, state) {
                (None, _) => UnknownName::Lifecycle(lifecycle),
                (Some(held), Some(state)) if held.state(&state).is_none() => UnknownName::State { lifecycle, state },
                _ => continue,
            };
            if !unknown.contains(&one) {
                unknown.push(one);
            }
        }
        unknown
    }
}

/// A lifecycle or a state a rule names and the lifecycles that apply do not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnknownName {
    Lifecycle(LifecycleId),
    State { lifecycle: LifecycleId, state: StateId },
}

impl fmt::Display for UnknownName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnknownName::Lifecycle(lifecycle) => write!(f, "names the `{lifecycle}` lifecycle, which does not apply here"),
            UnknownName::State { lifecycle, state } => {
                write!(f, "names the state `{state}`, which the `{lifecycle}` lifecycle does not have")
            }
        }
    }
}

/// What is wrong with a rule that was declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MalformedRule {
    /// It does nothing.
    NoActs,
    /// A condition or an act speaks of the instance that entered, or those linked to it, and the
    /// rule's moment is not an instance entering a state.
    NoInstanceToSpeakOf,
    /// A request for reviewers names nobody.
    NobodyToAsk,
}

impl fmt::Display for MalformedRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MalformedRule::NoActs => f.write_str("does nothing, and a rule is saved to do something"),
            MalformedRule::NoInstanceToSpeakOf => f.write_str(
                "speaks of the instance that entered a state, or those linked to it, and its `when` is not an instance entering one",
            ),
            MalformedRule::NobodyToAsk => f.write_str("requests reviewers and names none"),
        }
    }
}

impl core::error::Error for MalformedRule {}
