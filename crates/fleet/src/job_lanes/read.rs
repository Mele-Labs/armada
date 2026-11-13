//! What a Job holds and what its rules did, as the lanes name it: `lanesOfJob` and its helpers in
//! `packages/screens/src/session-lanes.ts`.

use std::collections::{HashMap, HashSet};

use ipc::{
    AppliedRule, HeldInstance, Instant, InstanceMove, InstanceStanding, JobLifecycles, LaneAutonomy, LaneCarriedTicket,
    LaneFired, LaneFiredAct, LaneInstance, LaneKept, LaneLifecycle, LaneLifecycleKind, LaneLifecycleState, LaneMoment,
    LaneOutcome, LaneRef, LaneRule, LaneStep, LifecycleDefinition, LifecycleRule, RuleAct, RuleActReading, RuleAmong,
    RuleCondition, RuleTarget, RuleWhen, StateReadFrom, StepVisit,
};

/// Where the edge of a rule that hands to a step lands when the rule does not name one: the step the Job
/// was on when it fired, else the one it is on.
pub(super) const CURRENT_STEP: &str = "$step";

/// The lifecycle a watch's `deploy` names an instance of.
const DEPLOY: &str = "deploy";

/// The id an instance is drawn under: its lifecycle's, and its key as its source calls it, escaped as
/// JavaScript's `encodeURIComponent` does. A key can hold a slash, which a node id cannot.
pub(super) fn instance_id(lifecycle: &str, key: &str) -> String {
    let mut out = format!("{lifecycle}:");
    for byte in key.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Where a lifecycle's states are read from, in a word under the lane's title.
pub(super) fn source_of(read: Option<&StateReadFrom>) -> (LaneLifecycleKind, &'static str) {
    match read {
        None | Some(StateReadFrom::Tracker { .. }) => (LaneLifecycleKind::Tracker, "Tracker"),
        Some(StateReadFrom::Forge { .. }) => (LaneLifecycleKind::Forge, "Forge"),
        Some(StateReadFrom::Watch { .. }) => (LaneLifecycleKind::Watch, "Watch"),
        Some(StateReadFrom::Fleet { .. }) => (LaneLifecycleKind::Job, "Fleet"),
        Some(StateReadFrom::Own) => (LaneLifecycleKind::Own, "Your own"),
    }
}

/// The column of each state: after the states that lead to it, and never beside another in its lane. States
/// are taken in the order the definition lists them, so a move back to an earlier state leads nowhere new,
/// and one nothing leads to follows the states before it.
pub(super) fn columns_of(lifecycle: &LifecycleDefinition) -> HashMap<&str, i64> {
    let states: Vec<&str> = lifecycle.states.iter().map(|state| state.id.as_str()).collect();
    let moves: Vec<(&str, &str)> = lifecycle.transitions.iter().map(|step| (step.from.as_str(), step.to.as_str())).collect();
    columns(&states, &moves)
}

/// [`columns_of`] over states and moves that no one definition holds: a workflow editor's lifecycle, whose
/// states and moves are those of its pieces at every level.
pub(super) fn columns<'a>(states: &[&'a str], moves: &[(&str, &str)]) -> HashMap<&'a str, i64> {
    let order: HashMap<&str, usize> = states.iter().enumerate().map(|(at, state)| (*state, at)).collect();
    let mut cols: HashMap<&'a str, i64> = HashMap::new();
    let mut taken: HashSet<i64> = HashSet::new();
    let mut last: i64 = -1;
    for (at, state) in states.iter().copied().enumerate() {
        let behind = moves
            .iter()
            .filter(|(from, to)| *to == state && order.get(from).copied().unwrap_or(at) < at)
            .filter_map(|(from, _)| cols.get(from).copied())
            .max();
        let mut col = match behind {
            Some(behind) => behind + 1,
            None if at == 0 => 0,
            None => last + 1,
        };
        while taken.contains(&col) {
            col += 1;
        }
        taken.insert(col);
        cols.insert(state, col);
        last = last.max(col);
    }
    cols
}

pub(super) fn lifecycle(wire: &LifecycleDefinition) -> LaneLifecycle {
    let cols = columns_of(wire);
    let (kind, from) = source_of(wire.states.first().map(|state| &state.read));
    LaneLifecycle {
        id: wire.id.clone(),
        label: wire.name.clone(),
        from: from.to_string(),
        kind,
        states: wire
            .states
            .iter()
            .map(|state| LaneLifecycleState {
                id: state.id.clone(),
                label: state.label.clone(),
                col: cols.get(state.id.as_str()).copied().unwrap_or_default() as u32,
                end: wire.ends.contains(&state.id),
            })
            .collect(),
        next: wire.transitions.iter().map(|step| (step.from.clone(), step.to.clone())).collect(),
    }
}

/// The states an instance stood in, in the order it moved through them, the one it is in last.
fn visited_by(held: &HeldInstance, moves: &[InstanceMove]) -> Vec<String> {
    let mine: Vec<&InstanceMove> = moves.iter().filter(|step| step.lifecycle == held.lifecycle && step.key == held.key).collect();
    let mut seen: Vec<String> = Vec::new();
    let mut note = |state: &str| {
        if !seen.iter().any(|one| one == state) {
            seen.push(state.to_string());
        }
    };
    if let Some(InstanceStanding::In { state }) = mine.first().map(|step| &step.from) {
        note(state);
    }
    for step in &mine {
        if let InstanceStanding::In { state } = &step.to {
            note(state);
        }
    }
    if let InstanceStanding::In { state } = &held.standing {
        note(state);
    }
    seen
}

pub(super) fn instance(held: &HeldInstance, moves: &[InstanceMove], ticket: Option<LaneCarriedTicket>) -> LaneInstance {
    let (state, title, unmapped) = match &held.standing {
        InstanceStanding::In { state } => (state.clone(), None, None),
        InstanceStanding::Unmapped { said } => (String::new(), Some(format!("Unmapped: {said}")), Some(said.clone())),
        InstanceStanding::Unread => (String::new(), None, None),
    };
    LaneInstance {
        id: instance_id(&held.lifecycle, &held.key),
        lifecycle: held.lifecycle.clone(),
        label: held.label.clone(),
        title,
        ticket,
        state,
        unmapped,
        ended: held.ended_at.is_some(),
        visited: visited_by(held, moves),
    }
}

/// What reads a lifecycle's states, by the lifecycle's id among those a Job holds.
fn kind_of(lifecycles: &[LifecycleDefinition], id: &str) -> Option<LaneLifecycleKind> {
    let found = lifecycles.iter().find(|one| one.id == id)?;
    Some(source_of(found.states.first().map(|state| &state.read)).0).filter(|_| !found.states.is_empty())
}

/// The tickets this Job handed to the Jobs it started: the index of the instance of the Job, and of the
/// ticket it holds. **An ended ticket that a `job` instance links to is a handed one**: Fleet ends the
/// ticket and links it from the Job it went to, and no field says so. A ticket still live that a Job links
/// to is the Job's own, and keeps its lane.
pub(super) fn handed_of(held: &JobLifecycles) -> HashMap<usize, usize> {
    let mut handed = HashMap::new();
    for (at, job) in held.instances.iter().enumerate() {
        if kind_of(&held.lifecycles, &job.lifecycle) != Some(LaneLifecycleKind::Job) {
            continue;
        }
        for link in &job.links {
            let ticket = held.instances.iter().position(|one| {
                one.lifecycle == link.lifecycle
                    && one.key == link.key
                    && one.ended_at.is_some()
                    && kind_of(&held.lifecycles, &one.lifecycle) == Some(LaneLifecycleKind::Tracker)
            });
            if let Some(ticket) = ticket {
                handed.insert(at, ticket);
                break;
            }
        }
    }
    handed
}

/// A handed ticket as the card of the Job that holds it draws it: the state **the started Job reads it in**,
/// named as that Job's lifecycle names it. Not read there yet, it has none; a status no state lists keeps
/// the word the source said, and no state id names it.
pub(super) fn carried(
    ticket: &HeldInstance,
    started: Option<&JobLifecycles>,
    held: &JobLifecycles,
    title: Option<&str>,
) -> LaneCarriedTicket {
    let there = started.and_then(|one| one.instances.iter().find(|one| one.lifecycle == ticket.lifecycle && one.key == ticket.key));
    let label = there.map_or(&ticket.label, |one| &one.label).clone();
    let standing = there.map(|one| &one.standing);
    let title = title.filter(|title| !title.is_empty()).map(str::to_string);
    if let Some(InstanceStanding::In { state }) = standing {
        let named = started
            .map(|one| one.lifecycles.as_slice())
            .unwrap_or_default()
            .iter()
            .chain(held.lifecycles.iter())
            .find(|one| one.id == ticket.lifecycle)
            .and_then(|one| one.states.iter().find(|one| one.id == *state))
            .map(|one| one.label.clone());
        return LaneCarriedTicket { label, title, state: named.unwrap_or_else(|| state.clone()), state_id: state.clone() };
    }
    let state = match standing {
        Some(InstanceStanding::Unmapped { said }) => said.clone(),
        _ => String::new(),
    };
    LaneCarriedTicket { label, title, state, state_id: String::new() }
}

// ---- Rules and what they did ------------------------------------------------------------------------------------------------

/// The lifecycle a rule's target is of: the one named, or the one whose entering set the rule off.
pub(super) fn lane_of_target(target: &RuleTarget, rule: &LifecycleRule) -> Option<String> {
    match &target.among {
        RuleAmong::Entered => match &rule.when {
            RuleWhen::Enters { lifecycle, .. } => Some(lifecycle.clone()),
            _ => None,
        },
        RuleAmong::LinkedFrom { lifecycle } | RuleAmong::LinkingTo { lifecycle } | RuleAmong::Held { lifecycle } => {
            Some(lifecycle.clone())
        }
    }
}

/// The strongest word among a rule's acts: any that asks makes it Asks, and all of them off make it Off.
pub(super) fn autonomy_of(rule: &LifecycleRule) -> LaneAutonomy {
    let words: Vec<&str> = match &rule.autonomy {
        Some(word) => vec![word.as_str()],
        None => rule.does.iter().map(|one| one.autonomy.as_str()).collect(),
    };
    if words.contains(&"asks") {
        LaneAutonomy::Asks
    } else if !words.is_empty() && words.iter().all(|one| *one == "off") {
        LaneAutonomy::Off
    } else {
        LaneAutonomy::Acts
    }
}

/// A rule as a Job's lanes draw it, and the deploy instance its edge lands on where the rule takes one on.
pub(super) struct RuleRead {
    pub rule: LaneRule,
    pub lands_on: Option<String>,
}

/// A rule as a Job's lanes draw it: an edge from where it fired to where it lands. **Where it lands is what
/// the rule already says**, in this order: the state a `tracker_move` moves a ticket to; the first state of
/// the deploy a `start_watch` takes on, `lands_on` being that deploy; the step an `at_step` condition names,
/// before and after it fires; and where none of these names a place, the step the Job was on when it fired.
/// A rule a step sets off leaves from that step. A rule a Job event or a gate sets off has no node to leave
/// from, and is not drawn.
pub(super) fn rule(applied: &AppliedRule, lifecycles: &[LifecycleDefinition]) -> Option<RuleRead> {
    let AppliedRule { rule, kept } = applied;
    let (from, at) = match &rule.when {
        RuleWhen::Job { .. } | RuleWhen::GateCloses { .. } => return None,
        RuleWhen::Enters { lifecycle, state } => {
            (LaneRef { lane: lifecycle.clone(), item: state.clone() }, LaneMoment::Enters)
        }
        RuleWhen::StepStarts { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Starts),
        RuleWhen::StepEnds { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Ends),
    };
    let moved = rule.does.iter().find_map(|one| match &one.what {
        RuleAct::TrackerMove { to, target } => Some((lane_of_target(target, rule), to.clone())),
        _ => None,
    });
    let deploy = rule.does.iter().find_map(|one| match &one.what {
        RuleAct::StartWatch { deploy: Some(deploy), .. } if !deploy.is_empty() => Some(deploy.clone()),
        _ => None,
    });
    let at_step = rule.conditions.iter().find_map(|one| match one {
        RuleCondition::AtStep { step } => Some(step.clone()),
        _ => None,
    });
    let first = lifecycles.iter().find(|one| one.id == DEPLOY).and_then(|one| one.states.first()).map(|state| state.id.clone());
    let moved_to = moved.and_then(|(lane, item)| Some(LaneRef { lane: lane?, item }));
    // A deploy's lane has no state to land on until the deploy is held, so the edge waits for it.
    let lands = moved_to.is_none().then(|| deploy.as_ref().map(|deploy| instance_id(DEPLOY, deploy))).flatten();
    let to = match (moved_to, &deploy) {
        (Some(to), _) => to,
        (None, Some(_)) => LaneRef { lane: DEPLOY.into(), item: first.unwrap_or_default() },
        (None, None) => LaneRef { lane: "workflow".into(), item: at_step.unwrap_or_else(|| CURRENT_STEP.into()) },
    };
    Some(RuleRead {
        rule: LaneRule {
            id: rule.id.clone(),
            label: rule.name.clone(),
            from,
            at,
            to,
            autonomy: autonomy_of(rule),
            kept: match kept.rank.source.as_str() {
                "armada" => LaneKept::Armada,
                "kit" => LaneKept::Yours,
                _ => LaneKept::Repository,
            },
        },
        lands_on: lands,
    })
}

/// How an act stands, in the words a person reads. Done reads as the act itself.
fn standing_word(standing: &str) -> Option<&'static str> {
    Some(match standing {
        "planned" => "Planned",
        "asked" => "Asked",
        "stuck" => "Stuck",
        "skipped" => "Skipped",
        "told" => "Told the agent",
        "not_carried" => "Not carried",
        _ => return None,
    })
}

/// How the acts of one rule on one instance end together: any that waits on the person asks, any still to be
/// done waits, and all of them set aside is set aside.
fn outcome_of(acts: &[&RuleActReading]) -> LaneOutcome {
    let any = |words: &[&str]| acts.iter().any(|one| words.contains(&one.standing.as_str()));
    if any(&["asked", "stuck"]) {
        LaneOutcome::Asked
    } else if any(&["planned", "not_carried"]) {
        LaneOutcome::Waiting
    } else if acts.iter().all(|one| one.standing == "skipped") {
        LaneOutcome::Skipped
    } else {
        LaneOutcome::Acted
    }
}

fn say_act(act: &RuleActReading) -> String {
    match standing_word(&act.standing) {
        None => act.text.clone(),
        Some(word) => match act.said.as_deref().filter(|said| !said.is_empty()) {
            Some(said) => format!("{} · {word}: {said}", act.text),
            None => format!("{} · {word}", act.text),
        },
    }
}

/// The index of the step the Job was on at `at`: the last one begun by then, in the order the record keeps them.
fn step_when(at: &Instant, visits: &[StepVisit]) -> Option<u32> {
    let by = at.to_domain().epoch_millis()?;
    visits.iter().filter(|one| one.since.to_domain().epoch_millis().is_some_and(|since| since <= by)).next_back().map(|one| one.step)
}

/// What the Job's rules did, a line for each rule and instance it did it to. Only the rules that are drawn have one.
pub(super) fn fired(
    acts: &[RuleActReading],
    rules: &[LaneRule],
    lands_on: &HashMap<String, String>,
    steps: &[LaneStep],
    visits: &[StepVisit],
) -> Vec<LaneFired> {
    let mut groups: Vec<(&LaneRule, Vec<&RuleActReading>, String)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for act in acts {
        let Some(rule) = rules.iter().find(|one| one.id == act.rule) else { continue };
        // An edge that lands on a state is keyed by the instance it was done to, or the deploy a watch takes on; one that hands to a step, by the instance that set it off, or by none where a step set it off.
        let link = if rule.to.lane == "workflow" {
            if rule.from.lane == "workflow" { None } else { act.entered.as_ref().or(act.on.as_ref()) }
        } else {
            act.on.as_ref().or(act.entered.as_ref())
        };
        let instance = match lands_on.get(&rule.id) {
            Some(deploy) if rule.to.lane != "workflow" && act.on.is_none() => deploy.clone(),
            _ => link.map(|link| instance_id(&link.lifecycle, &link.key)).unwrap_or_default(),
        };
        let key = format!("{}\n{instance}", act.rule);
        match index.get(&key) {
            Some(at) => groups[*at].1.push(act),
            None => {
                index.insert(key, groups.len());
                groups.push((rule, vec![act], instance));
            }
        }
    }
    groups
        .into_iter()
        .map(|(rule, mine, instance)| {
            let last = mine[mine.len() - 1];
            let step = step_when(&mine[0].at, visits).and_then(|at| steps.get(at as usize)).map(|step| step.id.clone());
            LaneFired {
                rule: rule.id.clone(),
                instance,
                outcome: outcome_of(&mine),
                at: last.settled_at.clone().unwrap_or_else(|| last.at.clone()),
                did: mine.iter().map(|one| say_act(one)).collect(),
                step,
                acts: mine
                    .iter()
                    .map(|one| LaneFiredAct {
                        text: one.text.clone(),
                        course: if one.course == "asks" { LaneAutonomy::Asks } else { LaneAutonomy::Acts },
                        standing: one.standing.clone(),
                        said: one.said.clone().filter(|said| !said.is_empty()),
                        at: one.settled_at.clone().unwrap_or_else(|| one.at.clone()),
                    })
                    .collect(),
            }
        })
        .collect()
}
