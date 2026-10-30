//! A lifecycle, a rule and where each was kept, as the wire spells them.
//!
//! **Chosen field by field.** The model holds its parts behind constructors that refuse what no
//! engine could use, and none of it is serialisable; this is where a part becomes a word on the
//! wire. A closed set the model owns is spelled by its own `as_wire` where it has one, and by a
//! `match` here where it has none, which fails to compile when the model gains a variant.

use std::path::Path;

use config::{Rank, Reach};
use core_model::rule::{
    Act, ActKind, Among, AutonomyDefaults, Condition, ForgeWrite, JobCondition, Rule, Target, TrackerWrite, When,
};
use core_model::{Lifecycle, LifecycleScope, PieceSource, Standing, StateRead};

/// The place a piece was kept, as the wire spells it.
pub fn place(place: PieceSource) -> String {
    match place {
        PieceSource::Armada => "armada",
        PieceSource::Repository => "repository",
        PieceSource::Kit => "kit",
    }
    .to_string()
}

fn reach(reach: Reach) -> String {
    match reach {
        Reach::Everywhere => "everywhere",
        Reach::Workspace => "workspace",
        Reach::Repository => "repository",
    }
    .to_string()
}

fn rank(rank: Rank) -> ipc::PieceRank {
    ipc::PieceRank { reach: reach(rank.reach), source: place(rank.source), home: rank.home }
}

/// Where a lifecycle or a rule was kept, and what became of it beside another of its id.
pub fn kept(
    kept: Rank,
    path: Option<&Path>,
    overridden_by: Option<Rank>,
    hides: Option<PieceSource>,
) -> ipc::PieceKept {
    ipc::PieceKept {
        rank: rank(kept),
        path: path.map(|path| path.to_string_lossy().into_owned()),
        overridden_by: overridden_by.map(rank),
        hides: hides.map(place),
    }
}

/// The files set aside, each with its sentence.
pub fn left_out(left: &[config::PieceLeftOut]) -> Vec<ipc::PieceLeftOut> {
    left.iter()
        .map(|one| ipc::PieceLeftOut {
            source: place(one.source()),
            path: one.path().to_string_lossy().into_owned(),
            said: one.said().to_string(),
        })
        .collect()
}

pub fn scope(scope: &LifecycleScope) -> ipc::PieceScope {
    match scope {
        LifecycleScope::Everywhere => ipc::PieceScope::Everywhere,
        LifecycleScope::Within(id) => ipc::PieceScope::Within { id: id.as_str().to_string() },
    }
}

fn state_read(read: &StateRead) -> ipc::StateReadFrom {
    match read {
        StateRead::Tracker(words) => ipc::StateReadFrom::Tracker { words: words.as_slice().to_vec() },
        StateRead::Forge(fact) => ipc::StateReadFrom::Forge { fact: fact.as_wire().to_string() },
        StateRead::Watch(state) => ipc::StateReadFrom::Watch { state: state.as_wire().to_string() },
        StateRead::Fleet(fact) => ipc::StateReadFrom::Fleet { fact: fact.as_wire() },
    }
}

/// The kinds a layer of defaults set, in the order of the kinds.
pub fn autonomy_set(set: &AutonomyDefaults) -> Vec<ipc::ActKindDefault> {
    set.iter()
        .map(|(kind, autonomy)| ipc::ActKindDefault { kind: kind.as_wire().to_string(), autonomy: autonomy.as_wire().to_string() })
        .collect()
}

/// What Armada sets for every kind of act, in the order of the kinds.
pub fn armada_autonomy() -> Vec<ipc::ActKindDefault> {
    ActKind::ALL
        .iter()
        .map(|kind| ipc::ActKindDefault { kind: kind.as_wire().to_string(), autonomy: kind.default_autonomy().as_wire().to_string() })
        .collect()
}

/// A lifecycle's states, the moves it expects and the states that end it.
pub fn lifecycle(lifecycle: &Lifecycle) -> ipc::LifecycleDefinition {
    ipc::LifecycleDefinition {
        id: lifecycle.id().as_str().to_string(),
        name: lifecycle.name().as_str().to_string(),
        scope: scope(lifecycle.scope()),
        states: lifecycle
            .states()
            .iter()
            .map(|state| ipc::LifecycleStateDefinition {
                id: state.id().as_str().to_string(),
                label: state.label().as_str().to_string(),
                read: state_read(state.read()),
            })
            .collect(),
        transitions: lifecycle
            .transitions()
            .iter()
            .map(|one| ipc::LifecycleTransition { from: one.from.as_str().to_string(), to: one.to.as_str().to_string() })
            .collect(),
        ends: lifecycle.ends().iter().map(|state| state.as_str().to_string()).collect(),
    }
}

/// Where an instance stands.
pub fn standing(standing: &Standing) -> ipc::InstanceStanding {
    match standing {
        Standing::Unread => ipc::InstanceStanding::Unread,
        Standing::In(state) => ipc::InstanceStanding::In { state: state.as_str().to_string() },
        Standing::Unmapped(said) => ipc::InstanceStanding::Unmapped { said: said.clone() },
    }
}

fn among(among: &Among) -> ipc::RuleAmong {
    match among {
        Among::Entered => ipc::RuleAmong::Entered,
        Among::LinkedFrom(lifecycle) => ipc::RuleAmong::LinkedFrom { lifecycle: lifecycle.as_str().to_string() },
        Among::LinkingTo(lifecycle) => ipc::RuleAmong::LinkingTo { lifecycle: lifecycle.as_str().to_string() },
        Among::Held(lifecycle) => ipc::RuleAmong::Held { lifecycle: lifecycle.as_str().to_string() },
    }
}

fn target(target: &Target) -> ipc::RuleTarget {
    ipc::RuleTarget { among: among(&target.among), in_state: target.in_state.as_ref().map(|state| state.as_str().to_string()) }
}

fn when(when: &When) -> ipc::RuleWhen {
    match when {
        When::Enters { lifecycle, state } => {
            ipc::RuleWhen::Enters { lifecycle: lifecycle.as_str().to_string(), state: state.as_str().to_string() }
        }
        When::StepStarts(step) => ipc::RuleWhen::StepStarts { step: step.as_str().to_string() },
        When::StepEnds(step) => ipc::RuleWhen::StepEnds { step: step.as_str().to_string() },
        When::Job(event) => ipc::RuleWhen::Job { event: event.as_wire().to_string() },
    }
}

fn condition(condition: &Condition) -> ipc::RuleCondition {
    match condition {
        Condition::Instances { among: which, quantity, in_state } => ipc::RuleCondition::Instances {
            among: among(which),
            quantity: quantity.as_wire().to_string(),
            in_state: in_state.as_str().to_string(),
        },
        Condition::Job(JobCondition::Workflow(slug)) => ipc::RuleCondition::Workflow { workflow: slug.as_str().to_string() },
        Condition::Job(JobCondition::AtStep(step)) => ipc::RuleCondition::AtStep { step: step.as_str().to_string() },
        Condition::Job(JobCondition::PullRequestOpened) => ipc::RuleCondition::PullRequestOpened,
    }
}

fn act(act: &Act) -> ipc::RuleAct {
    match act {
        Act::Tracker { write: TrackerWrite::Assign { to }, target: of } => {
            ipc::RuleAct::TrackerAssign { to: to.as_str().to_string(), target: target(of) }
        }
        Act::Tracker { write: TrackerWrite::Move { to }, target: of } => {
            ipc::RuleAct::TrackerMove { to: to.as_str().to_string(), target: target(of) }
        }
        Act::Tracker { write: TrackerWrite::Close, target: of } => ipc::RuleAct::TrackerClose { target: target(of) },
        Act::Forge { write: ForgeWrite::RequestReviewers { from }, target: of } => ipc::RuleAct::ForgeRequestReviewers {
            from: from.iter().map(|one| one.as_str().to_string()).collect(),
            target: target(of),
        },
        Act::Forge { write: ForgeWrite::MarkReady, target: of } => ipc::RuleAct::ForgeMarkReady { target: target(of) },
        Act::Forge { write: ForgeWrite::Merge, target: of } => ipc::RuleAct::ForgeMerge { target: target(of) },
        Act::Forge { write: ForgeWrite::AutoMerge, target: of } => ipc::RuleAct::ForgeAutoMerge { target: target(of) },
        Act::Draft { template, target: of } => ipc::RuleAct::Draft { template: template.as_str().to_string(), target: target(of) },
        Act::StartJob { workflow, prompt } => ipc::RuleAct::StartJob {
            workflow: workflow.as_ref().map(|slug| slug.as_str().to_string()),
            prompt: prompt.as_str().to_string(),
        },
        Act::StartWatch { recipe, values, deploy } => ipc::RuleAct::StartWatch {
            recipe: recipe.as_str().to_string(),
            values: values.iter().map(|(name, value)| (name.as_str().to_string(), value.as_str().to_string())).collect(),
            deploy: deploy.as_ref().map(|deploy| deploy.as_str().to_string()),
        },
        Act::Tell { message } => ipc::RuleAct::Tell { message: message.as_str().to_string() },
        Act::Wake => ipc::RuleAct::Wake,
        Act::RunCommand { command } => ipc::RuleAct::RunCommand { command: command.as_str().to_string() },
        Act::RunSkill { skill } => ipc::RuleAct::RunSkill { skill: skill.as_str().to_string() },
        Act::RunDrone { prompt } => ipc::RuleAct::RunDrone { prompt: prompt.as_str().to_string() },
    }
}

/// A rule with each act's autonomy as its own word or its kind's default. **For a write**, which
/// carries the rule's own `autonomy` and never an act's resolved one; a read uses [`rule_under`].
pub fn rule(rule: &Rule) -> ipc::LifecycleRule {
    rule_under(rule, &AutonomyDefaults::new())
}

/// A rule: when, if and do, with each act's autonomy worked out: the rule's own word, else what
/// `defaults` set for the kind (the repository's laid layers), else the kind's own.
pub fn rule_under(rule: &Rule, defaults: &AutonomyDefaults) -> ipc::LifecycleRule {
    ipc::LifecycleRule {
        id: rule.id().as_str().to_string(),
        name: rule.name().as_str().to_string(),
        scope: scope(rule.scope()),
        when: when(rule.when()),
        conditions: rule.conditions().iter().map(condition).collect(),
        does: rule
            .does()
            .iter()
            .map(|one| ipc::RuleDoing {
                what: act(one),
                kind: one.kind().as_wire().to_string(),
                autonomy: defaults.resolved(one.kind(), rule.autonomy()).as_wire().to_string(),
            })
            .collect(),
        autonomy: rule.autonomy().map(|autonomy| autonomy.as_wire().to_string()),
        on_failure: ipc::RuleOnFailure {
            retries: rule.on_failure().retries.times(),
            then: rule.on_failure().then.as_wire().to_string(),
        },
    }
}
