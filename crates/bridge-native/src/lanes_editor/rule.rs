//! A rule as a sentence: its moment, its conditions and its act said in plain words, the choices each field offers, and the
//! rule as the text of its piece draft. The vocabulary is the wire's: the variants of `ipc::RuleWhen`, `RuleCondition` and
//! `RuleAct` are what a field offers, and what fills them is what the editor already reads: the workflow's steps, the
//! lifecycles and their states, `ipc::PlanGate`, the workflows Fleet runs and the watch recipes.
//!
//! **The draft text is the piece's JSON** (`ipc::LifecyclePiece::Rule`), as `PieceDraft.text` says, and Fleet writes the file
//! from it. A word list the wire holds only in a doc comment is a const here, naming where it is said.

use std::rc::Rc;

use ipc::{LifecycleDefinition, LifecyclePiece, LifecycleRule, PieceScope, PlanGate, RuleAct, RuleAmong, RuleCondition, RuleDoing, RuleOnFailure, RuleTarget, RuleWhen, StateReadFrom};

/// `RuleWhen::Job`'s `event`, as the variant's doc comment lists them, with how each reads after "When".
pub const JOB_EVENTS: [(&str, &str); 4] = [("started", "the Job starts"), ("pull_request_opened", "a pull request is opened"), ("landed", "the Job lands"), ("ended", "the Job ends")];

/// `RuleCondition::Instances`' `quantity`, as the variant's doc comment lists them.
pub const QUANTITIES: [&str; 3] = ["every", "any", "no"];

/// `RuleDoing.kind`: the kind of act an autonomy default is set for, as the field's doc comment lists them.
const KINDS: [&str; 8] = ["tracker_write", "forge_write", "draft", "start_job", "start_watch", "tell_agent", "run_command", "run_agent"];

/// What the choices are filled from.
#[derive(Clone, Debug, Default)]
pub struct Vocab {
    /// The workflow's steps, `(id, name)`.
    pub steps: Vec<(String, String)>,
    /// The lifecycles that run, in the order Fleet lists them.
    pub lifecycles: Vec<LifecycleDefinition>,
    /// The workflows Fleet runs, `(id, name)`.
    pub workflows: Vec<(String, String)>,
    /// The watch recipes a start may name.
    pub recipes: Vec<String>,
    /// Each kind of act's autonomy where no rule sets one: Armada's own, with each layer that sets some laid over it.
    pub defaults: Vec<(String, String)>,
}

/// What a field's choice sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Set {
    When(RuleWhen),
    Condition(RuleCondition),
    Act(RuleAct),
}

/// What picking a choice does.
#[derive(Clone)]
pub enum Next {
    /// The field is set.
    Set(Set),
    /// A second choice in the same field.
    More(Vec<Opt>),
    /// Words typed in the same field, which make the value. `name` is what the box is called.
    Words { name: &'static str, make: Rc<dyn Fn(String) -> Set> },
}

/// One choice of a field.
#[derive(Clone)]
pub struct Opt {
    pub label: String,
    pub next: Next,
}

fn opt(label: impl Into<String>, next: Next) -> Opt {
    Opt { label: label.into(), next }
}

fn set(label: impl Into<String>, what: Set) -> Opt {
    opt(label, Next::Set(what))
}

fn lower(word: &str) -> String {
    word.to_lowercase()
}

fn capital(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn spoken(word: &str) -> String {
    word.replace('_', " ")
}

fn words_of(text: &str) -> Vec<String> {
    text.split(',').map(|one| one.trim().to_string()).filter(|one| !one.is_empty()).collect()
}

/// A line cut to what a field has room for.
pub fn short(said: &str, most: usize) -> String {
    if said.chars().count() <= most {
        said.to_string()
    } else {
        format!("{}…", said.chars().take(most).collect::<String>().trim_end())
    }
}

impl Vocab {
    fn lifecycle(&self, id: &str) -> Option<&LifecycleDefinition> {
        self.lifecycles.iter().find(|one| one.id == id)
    }

    fn lifecycle_name(&self, id: &str) -> String {
        self.lifecycle(id).map_or_else(|| spoken(id), |one| one.name.clone())
    }

    fn step_name(&self, id: &str) -> String {
        self.steps.iter().find(|(one, _)| one == id).map_or_else(|| capital(&spoken(id)), |(_, name)| name.clone())
    }

    /// A state's name, in `lifecycle` where it is said, else in the first lifecycle that has it.
    fn state_name(&self, lifecycle: Option<&str>, state: &str) -> String {
        let held = lifecycle.and_then(|id| self.lifecycle(id)).into_iter().chain(self.lifecycles.iter());
        for one in held {
            if let Some(found) = one.states.iter().find(|held| held.id == state) {
                return found.label.clone();
            }
        }
        capital(&spoken(state))
    }

    /// The first lifecycle whose states are read from the tracker, and the first read from the forge.
    fn tracker(&self) -> Option<&LifecycleDefinition> {
        self.lifecycles.iter().find(|one| one.states.iter().any(|state| matches!(state.read, StateReadFrom::Tracker { .. })))
    }

    fn forge(&self) -> Option<&LifecycleDefinition> {
        self.lifecycles.iter().find(|one| one.states.iter().any(|state| matches!(state.read, StateReadFrom::Forge { .. })))
    }

    /// What a kind of act is set to where no rule says.
    pub fn default_for(&self, kind: &str) -> Option<&str> {
        self.defaults.iter().find(|(one, _)| one == kind).map(|(_, autonomy)| autonomy.as_str())
    }
}

/// The lifecycle a target looks at, where the rule's moment names it for the instance that set it off.
fn lifecycle_of<'a>(among: &'a RuleAmong, when: &'a RuleWhen) -> Option<&'a str> {
    match among {
        RuleAmong::Entered => match when {
            RuleWhen::Enters { lifecycle, .. } => Some(lifecycle),
            _ => None,
        },
        RuleAmong::LinkedFrom { lifecycle } | RuleAmong::LinkingTo { lifecycle } | RuleAmong::Held { lifecycle } => Some(lifecycle),
    }
}

// ---- said in words ---------------------------------------------------------------------------

pub fn when_phrase(vocab: &Vocab, when: &RuleWhen) -> String {
    match when {
        RuleWhen::StepStarts { step } => format!("{} starts", vocab.step_name(step)),
        RuleWhen::StepEnds { step } => format!("{} ends", vocab.step_name(step)),
        RuleWhen::Enters { lifecycle, state } => format!("{} enters {}", vocab.lifecycle_name(lifecycle), vocab.state_name(Some(lifecycle), state)),
        RuleWhen::GateCloses { gate } => gate_phrase(gate),
        RuleWhen::Job { event } => JOB_EVENTS.iter().find(|(word, _)| word == event).map_or_else(|| spoken(event), |(_, said)| (*said).to_string()),
    }
}

/// A gate closing, said as what happened. The match is whole, so a gate the wire adds is said here before it builds.
fn gate_phrase(gate: &str) -> String {
    match PlanGate::from_word(gate) {
        PlanGate::PlanApproved => "the plan is approved".into(),
        PlanGate::WalkApproved => "the walk is approved".into(),
        PlanGate::SketchApproved => "the sketch is approved".into(),
        PlanGate::DocumentApproved => "the document is approved".into(),
        PlanGate::YouApproved => "you approve".into(),
        PlanGate::ChecksGreen => "the checks pass".into(),
        PlanGate::Merged => "it merges".into(),
        PlanGate::JobsEnded => "every Job it started ends".into(),
        PlanGate::None => format!("the {} gate closes", spoken(gate)),
    }
}

fn among_phrase(vocab: &Vocab, among: &RuleAmong) -> String {
    match among {
        RuleAmong::Entered => "the one that entered".into(),
        RuleAmong::Held { lifecycle } => format!("{} held", lower(&vocab.lifecycle_name(lifecycle))),
        RuleAmong::LinkedFrom { lifecycle } => format!("{} it came from", lower(&vocab.lifecycle_name(lifecycle))),
        RuleAmong::LinkingTo { lifecycle } => format!("{} that came from it", lower(&vocab.lifecycle_name(lifecycle))),
    }
}

pub fn condition_phrase(vocab: &Vocab, condition: &RuleCondition) -> String {
    match condition {
        RuleCondition::Instances { among, quantity, in_state } => {
            let lifecycle = match among {
                RuleAmong::Held { lifecycle } | RuleAmong::LinkedFrom { lifecycle } | RuleAmong::LinkingTo { lifecycle } => Some(lifecycle.as_str()),
                RuleAmong::Entered => None,
            };
            format!("{quantity} {} is in {}", among_phrase(vocab, among), vocab.state_name(lifecycle, in_state))
        }
        RuleCondition::Workflow { workflow } => format!("the Job follows {}", vocab.workflows.iter().find(|(id, _)| id == workflow).map_or_else(|| spoken(workflow), |(_, name)| name.clone())),
        RuleCondition::AtStep { step } => format!("the Job is on {}", vocab.step_name(step)),
        RuleCondition::PullRequestOpened => "the Job has opened a pull request".into(),
    }
}

/// What an act is done to, as the words after its verb.
fn target_phrase(vocab: &Vocab, target: &RuleTarget) -> String {
    let who = match &target.among {
        RuleAmong::Entered => "the one that entered".to_string(),
        RuleAmong::Held { lifecycle } => format!("each {}", lower(&vocab.lifecycle_name(lifecycle))),
        RuleAmong::LinkedFrom { lifecycle } => format!("the {} it came from", lower(&vocab.lifecycle_name(lifecycle))),
        RuleAmong::LinkingTo { lifecycle } => format!("each {} that came from it", lower(&vocab.lifecycle_name(lifecycle))),
    };
    let lifecycle = match &target.among {
        RuleAmong::Held { lifecycle } | RuleAmong::LinkedFrom { lifecycle } | RuleAmong::LinkingTo { lifecycle } => Some(lifecycle.as_str()),
        RuleAmong::Entered => None,
    };
    match &target.in_state {
        Some(state) => format!("{who} in {}", vocab.state_name(lifecycle, state)),
        None => who,
    }
}

pub fn act_phrase(vocab: &Vocab, act: &RuleAct) -> String {
    let said = |words: &str, lead: &str| if words.trim().is_empty() { lead.to_string() } else { format!("{lead}: {}", words.trim()) };
    match act {
        RuleAct::TrackerAssign { to, target } => format!("assign {} to {to}", target_phrase(vocab, target)),
        RuleAct::TrackerMove { to, target } => {
            let lifecycle = match &target.among {
                RuleAmong::Held { lifecycle } | RuleAmong::LinkedFrom { lifecycle } | RuleAmong::LinkingTo { lifecycle } => Some(lifecycle.as_str()),
                RuleAmong::Entered => None,
            };
            format!("move {} to {}", target_phrase(vocab, target), vocab.state_name(lifecycle, to))
        }
        RuleAct::TrackerClose { target } => format!("close {}", target_phrase(vocab, target)),
        RuleAct::ForgeRequestReviewers { from, target } => format!("ask {} to review {}", from.join(", "), target_phrase(vocab, target)),
        RuleAct::ForgeMarkReady { target } => format!("mark {} ready for review", target_phrase(vocab, target)),
        RuleAct::ForgeMerge { target } => format!("merge {}", target_phrase(vocab, target)),
        RuleAct::ForgeAutoMerge { target } => format!("merge {} once checks pass", target_phrase(vocab, target)),
        RuleAct::Draft { template, target } => format!("draft {} for {}", spoken(template), target_phrase(vocab, target)),
        RuleAct::StartJob { workflow, prompt } => match workflow {
            Some(workflow) => said(prompt, &format!("start a Job on {}", spoken(workflow))),
            None => said(prompt, "start a Job"),
        },
        RuleAct::StartWatch { recipe, deploy, .. } => match deploy {
            Some(deploy) => format!("start the {} watch on {deploy}", spoken(recipe)),
            None => format!("start the {} watch", spoken(recipe)),
        },
        RuleAct::Tell { message } => said(message, "tell the agent"),
        RuleAct::Wake => "wake the agent".into(),
        RuleAct::RunCommand { command } => said(command, "run a command"),
        RuleAct::RunSkill { skill } => format!("run the {skill} skill"),
        RuleAct::RunDrone { prompt } => said(prompt, "run a Drone"),
    }
}

/// The kind of act an autonomy default is set for, by the act's variant.
pub fn kind_of(act: &RuleAct) -> &'static str {
    let at = match act {
        RuleAct::TrackerAssign { .. } | RuleAct::TrackerMove { .. } | RuleAct::TrackerClose { .. } => 0,
        RuleAct::ForgeRequestReviewers { .. } | RuleAct::ForgeMarkReady { .. } | RuleAct::ForgeMerge { .. } | RuleAct::ForgeAutoMerge { .. } => 1,
        RuleAct::Draft { .. } => 2,
        RuleAct::StartJob { .. } => 3,
        RuleAct::StartWatch { .. } => 4,
        RuleAct::Tell { .. } | RuleAct::Wake => 5,
        RuleAct::RunCommand { .. } => 6,
        RuleAct::RunSkill { .. } | RuleAct::RunDrone { .. } => 7,
    };
    KINDS[at]
}

// ---- the rule ---------------------------------------------------------------------------------

/// What the rule does with its acts: its own word, else what the first act's kind is set to.
pub fn autonomy_of<'a>(vocab: &'a Vocab, rule: &'a LifecycleRule) -> &'a str {
    rule.autonomy.as_deref().or_else(|| rule.does.first().and_then(|one| vocab.default_for(&one.kind))).unwrap_or("acts")
}

/// Whether the rule asks first.
pub fn asks(vocab: &Vocab, rule: &LifecycleRule) -> bool {
    autonomy_of(vocab, rule) == "asks"
}

/// The act as the rule's doing, with the autonomy it comes to.
pub fn doing(vocab: &Vocab, rule_autonomy: Option<&str>, act: RuleAct) -> RuleDoing {
    let kind = kind_of(&act).to_string();
    let autonomy = rule_autonomy.or_else(|| vocab.default_for(&kind)).unwrap_or("acts").to_string();
    RuleDoing { what: act, kind, autonomy }
}

/// The rule asking first or not. Off is the absence of the word where nothing asks by default; where the act's kind asks
/// by default, off is the word `acts`, which is what says it does not.
pub fn with_asks(vocab: &Vocab, rule: &mut LifecycleRule, on: bool) {
    let default_asks = rule.does.first().and_then(|one| vocab.default_for(&one.kind)) == Some("asks");
    rule.autonomy = match (on, default_asks) {
        (true, _) => Some("asks".into()),
        (false, true) => Some("acts".into()),
        (false, false) => None,
    };
    for one in &mut rule.does {
        one.autonomy = rule.autonomy.clone().or_else(|| vocab.default_for(&one.kind).map(String::from)).unwrap_or_else(|| "acts".into());
    }
}

/// A rule with nothing said yet but who it is, to be filled field by field.
pub fn blank(id: &str, name: &str, scope: PieceScope, when: RuleWhen, does: Vec<RuleDoing>) -> LifecycleRule {
    LifecycleRule { id: id.into(), name: name.into(), scope, when, conditions: Vec::new(), does, autonomy: None, on_failure: RuleOnFailure { retries: 1, then: "tell_agent".into() } }
}

/// The rule as the text of its piece draft.
pub fn text_of(rule: &LifecycleRule) -> String {
    serde_json::to_string(&LifecyclePiece::Rule { rule: rule.clone() }).unwrap_or_default()
}

/// The rule a piece draft's text holds.
pub fn rule_of(text: &str) -> Option<LifecycleRule> {
    match ipc::decode::<LifecyclePiece>("piece", text.as_bytes()) {
        Ok(LifecyclePiece::Rule { rule }) => Some(rule),
        _ => None,
    }
}

/// The text with the rule kept at `scope`.
pub fn scoped(text: &str, scope: &PieceScope) -> Option<String> {
    let mut rule = rule_of(text)?;
    rule.scope = scope.clone();
    Some(text_of(&rule))
}

/// `current` with what `change` set taken back to what `before` had, a field at a time. None where a later change set that
/// field again, so taking this one back would take the later one with it.
pub fn undone(current: &str, change: &str, before: &str) -> Option<String> {
    let (mut now, was, then) = (rule_of(current)?, rule_of(change)?, rule_of(before)?);
    macro_rules! field {
        ($field:ident) => {
            if was.$field != then.$field {
                if now.$field != was.$field {
                    return None;
                }
                now.$field = then.$field.clone();
            }
        };
    }
    field!(name);
    field!(when);
    field!(conditions);
    field!(does);
    field!(autonomy);
    field!(on_failure);
    Some(text_of(&now))
}

/// The nodes a rule's edge runs between, `(from, to)`, where the rule's moment and its first act name them: a step or a
/// state it enters, and the state its act moves something to or looks at. None where the rule is set off by a gate or the Job
/// as a whole, or its act names no state, which the canvas does not draw.
pub fn ends_of(rule: &LifecycleRule) -> Option<(String, String)> {
    let from = match &rule.when {
        RuleWhen::StepStarts { step } | RuleWhen::StepEnds { step } => format!("workflow/{step}"),
        RuleWhen::Enters { lifecycle, state } => format!("{lifecycle}/{state}"),
        _ => return None,
    };
    let (target, state) = match &rule.does.first()?.what {
        RuleAct::TrackerMove { to, target } => (target, to.clone()),
        RuleAct::TrackerAssign { target, .. } | RuleAct::TrackerClose { target } | RuleAct::ForgeRequestReviewers { target, .. } | RuleAct::ForgeMarkReady { target } | RuleAct::ForgeMerge { target } | RuleAct::ForgeAutoMerge { target } | RuleAct::Draft { target, .. } => {
            (target, target.in_state.clone()?)
        }
        _ => return None,
    };
    Some((from, format!("{}/{state}", lifecycle_of(&target.among, &rule.when)?)))
}

// ---- the choices ------------------------------------------------------------------------------

fn each_state(vocab: &Vocab) -> impl Iterator<Item = (&LifecycleDefinition, &ipc::LifecycleStateDefinition)> {
    vocab.lifecycles.iter().flat_map(|one| one.states.iter().map(move |state| (one, state)))
}

/// What the When field offers: a step starting or ending, a state entered, a gate closing, or something the Job does.
pub fn when_options(vocab: &Vocab) -> Vec<Opt> {
    let mut out = Vec::new();
    if !vocab.steps.is_empty() {
        let starts = vocab.steps.iter().map(|(id, _)| RuleWhen::StepStarts { step: id.clone() }).map(|when| set(when_phrase(vocab, &when), Set::When(when))).collect();
        let ends = vocab.steps.iter().map(|(id, _)| RuleWhen::StepEnds { step: id.clone() }).map(|when| set(when_phrase(vocab, &when), Set::When(when))).collect();
        out.push(opt("A step starts", Next::More(starts)));
        out.push(opt("A step ends", Next::More(ends)));
    }
    let entered: Vec<Opt> = each_state(vocab).map(|(one, state)| RuleWhen::Enters { lifecycle: one.id.clone(), state: state.id.clone() }).map(|when| set(when_phrase(vocab, &when), Set::When(when))).collect();
    if !entered.is_empty() {
        out.push(opt("A state is entered", Next::More(entered)));
    }
    let gates: Vec<Opt> = PlanGate::ALL.into_iter().filter(|gate| !gate.is_none()).map(|gate| RuleWhen::GateCloses { gate: gate.word().into() }).map(|when| set(when_phrase(vocab, &when), Set::When(when))).collect();
    out.push(opt("A gate closes", Next::More(gates)));
    for (event, _) in JOB_EVENTS {
        let when = RuleWhen::Job { event: event.into() };
        out.push(set(capital(&when_phrase(vocab, &when)), Set::When(when)));
    }
    out
}

/// What an If field offers: how many instances stand in a state, the workflow the Job follows, the step it is on, or that it
/// has opened a pull request.
pub fn condition_options(vocab: &Vocab) -> Vec<Opt> {
    let mut out = Vec::new();
    for quantity in QUANTITIES {
        let states: Vec<Opt> = each_state(vocab)
            .map(|(one, state)| RuleCondition::Instances { among: RuleAmong::Held { lifecycle: one.id.clone() }, quantity: quantity.into(), in_state: state.id.clone() })
            .map(|condition| set(condition_phrase(vocab, &condition), Set::Condition(condition)))
            .collect();
        if !states.is_empty() {
            out.push(opt(format!("{} held instance is in a state", capital(quantity)), Next::More(states)));
        }
    }
    if !vocab.workflows.is_empty() {
        let followed = vocab.workflows.iter().map(|(id, _)| RuleCondition::Workflow { workflow: id.clone() }).map(|condition| set(condition_phrase(vocab, &condition), Set::Condition(condition))).collect();
        out.push(opt("The Job follows a workflow", Next::More(followed)));
    }
    if !vocab.steps.is_empty() {
        let on = vocab.steps.iter().map(|(id, _)| RuleCondition::AtStep { step: id.clone() }).map(|condition| set(condition_phrase(vocab, &condition), Set::Condition(condition))).collect();
        out.push(opt("The Job is on a step", Next::More(on)));
    }
    out.push(set(capital(&condition_phrase(vocab, &RuleCondition::PullRequestOpened)), Set::Condition(RuleCondition::PullRequestOpened)));
    out
}

/// What a Do field offers. An act on a ticket or a pull request is done to the one that set the rule off where the moment is
/// that lifecycle, else to each one the Job holds.
pub fn act_options(vocab: &Vocab, when: Option<&RuleWhen>) -> Vec<Opt> {
    let target = |lifecycle: &LifecycleDefinition| RuleTarget {
        among: match when {
            Some(RuleWhen::Enters { lifecycle: entered, .. }) if *entered == lifecycle.id => RuleAmong::Entered,
            _ => RuleAmong::Held { lifecycle: lifecycle.id.clone() },
        },
        in_state: None,
    };
    let mut out = Vec::new();
    if let Some(tracker) = vocab.tracker() {
        let noun = lower(&tracker.name);
        let moves = tracker.states.iter().map(|state| RuleAct::TrackerMove { to: state.id.clone(), target: target(tracker) }).map(|act| set(act_phrase(vocab, &act), Set::Act(act))).collect();
        out.push(opt(format!("Move each {noun}"), Next::More(moves)));
        let on = target(tracker);
        out.push(opt(format!("Assign each {noun}"), Next::Words { name: "Assignee", make: Rc::new({
            let on = on.clone();
            move |to| Set::Act(RuleAct::TrackerAssign { to, target: on.clone() })
        }) }));
        out.push(set(format!("Close each {noun}"), Set::Act(RuleAct::TrackerClose { target: on })));
    }
    if let Some(forge) = vocab.forge() {
        let noun = lower(&forge.name);
        let on = target(forge);
        out.push(opt(format!("Ask reviewers on each {noun}"), Next::Words { name: "Reviewers", make: Rc::new({
            let on = on.clone();
            move |said| Set::Act(RuleAct::ForgeRequestReviewers { from: words_of(&said), target: on.clone() })
        }) }));
        out.push(set(format!("Mark each {noun} ready"), Set::Act(RuleAct::ForgeMarkReady { target: on.clone() })));
        out.push(set(format!("Merge each {noun}"), Set::Act(RuleAct::ForgeMerge { target: on.clone() })));
        out.push(set(format!("Merge each {noun} once checks pass"), Set::Act(RuleAct::ForgeAutoMerge { target: on.clone() })));
        out.push(opt("Draft a message", Next::Words { name: "Template", make: Rc::new(move |template| Set::Act(RuleAct::Draft { template, target: on.clone() })) }));
    }
    out.push(opt("Start a Job", Next::Words { name: "Prompt", make: Rc::new(|prompt| Set::Act(RuleAct::StartJob { workflow: None, prompt })) }));
    if !vocab.recipes.is_empty() {
        let recipes = vocab.recipes.iter().map(|recipe| RuleAct::StartWatch { recipe: recipe.clone(), values: Default::default(), deploy: None }).map(|act| set(act_phrase(vocab, &act), Set::Act(act))).collect();
        out.push(opt("Start a watch", Next::More(recipes)));
    }
    out.push(opt("Tell the agent", Next::Words { name: "Message", make: Rc::new(|message| Set::Act(RuleAct::Tell { message })) }));
    out.push(set("Wake the agent", Set::Act(RuleAct::Wake)));
    out.push(opt("Run a command", Next::Words { name: "Command", make: Rc::new(|command| Set::Act(RuleAct::RunCommand { command })) }));
    out.push(opt("Run a skill", Next::Words { name: "Skill", make: Rc::new(|skill| Set::Act(RuleAct::RunSkill { skill })) }));
    out.push(opt("Run a Drone", Next::Words { name: "Prompt", make: Rc::new(|prompt| Set::Act(RuleAct::RunDrone { prompt })) }));
    out
}
