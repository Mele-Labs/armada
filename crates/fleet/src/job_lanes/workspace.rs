//! The Workspace a workflow's editor draws, read off the pieces Fleet serves: `workspaceOf` in Bridge's
//! `lifecycle-editing/read.ts`, with the lifecycles and rules a keep draft would add (`keep-drafts.ts`).
//!
//! The editor's model is layered (a level changes only what it changes) and the pieces are whole (a level that
//! has a piece runs that one), so each level's piece is compared with the one it would otherwise read, and only
//! the difference is kept against the level.

use ipc::{
    LaneAutonomy, LaneKept, LaneLifecycleKind, LaneMoment, LaneRef, LifecycleDefinition, LifecycleRule, PieceKept, PieceRank, PieceScope,
    RuleAct, RuleCondition, RuleWhen, StateReadFrom,
};

use super::levels::{
    clash_of, effective, exists_at, inherited, level_of, levels_of_read, lifecycle_layers, rule_layers, Clash, Layers, PiecesRead,
    DEFAULT,
};
use super::read::{autonomy_of, lane_of_target, source_of};

/// The lifecycle a watch's `deploy` lands on.
const DEPLOY: &str = "deploy";

/// What a keep draft waiting on the owner's Approve would add, read from its text by the caller.
#[derive(Clone, Debug, Default)]
pub(crate) struct Drafts {
    /// The lifecycles of the drafts that read.
    pub lifecycles: Vec<LifecycleDefinition>,
    /// The rules of the drafts that read.
    pub rules: Vec<LifecycleRule>,
    /// Every piece a draft keeps, as `lifecycle:<id>` and `rule:<id>`, whether its text read or not.
    pub keeps: Vec<String>,
}

/// A state as the Workspace defines it, with what each level below the default changes of it.
pub(super) struct StateDef {
    pub id: String,
    pub label: String,
    /// The level it was added at, so it exists there and below.
    pub scope: String,
    /// Another name for it at a level below its own.
    pub names: Vec<(String, String)>,
    /// The levels whose tracker words for it are their own: its own level's, and each below that reads it otherwise.
    pub reads: Vec<String>,
    pub end: bool,
    /// The levels whose own piece does not have it.
    pub absent: Vec<String>,
    pub clash: Clash,
}

/// A lifecycle as the Workspace defines it: its states and moves at every level.
pub(super) struct LifecycleDef {
    pub id: String,
    pub label: String,
    pub from: &'static str,
    pub kind: LaneLifecycleKind,
    pub states: Vec<StateDef>,
    pub next: Vec<(String, String)>,
    /// The widest level that keeps a piece of it.
    pub scope: String,
}

/// What the lanes draw of one rule: its ends, its words, and the one workflow it belongs to.
#[derive(Clone)]
pub(super) struct Drawn {
    pub label: String,
    pub from: LaneRef,
    pub at: LaneMoment,
    pub to: LaneRef,
    pub autonomy: LaneAutonomy,
    pub workflow: Option<String>,
}

/// A rule as the Workspace defines it: drawn from its widest piece, with each narrower level that keeps another.
pub(super) struct RuleDef {
    pub id: String,
    pub drawn: Drawn,
    pub kept: LaneKept,
    pub scope: String,
    /// Each level below that keeps a piece other than the one above it, and what that piece draws.
    pub overrides: Vec<(String, Drawn)>,
    pub clash: Clash,
}

pub(super) struct Workspace {
    pub lifecycles: Vec<LifecycleDef>,
    pub rules: Vec<RuleDef>,
    /// The rules of the drafts that no repository keeps, drawn as proposed.
    pub drafted: Vec<RuleDef>,
}

/// What a rule is worded by: a lifecycle's states in order, and the ones that end it.
struct Named<'a> {
    id: &'a str,
    states: Vec<&'a str>,
    ends: Vec<&'a str>,
}

fn kept_as(kept: &PieceKept) -> LaneKept {
    match kept.rank.source.as_str() {
        "armada" => LaneKept::Armada,
        "kit" => LaneKept::Yours,
        _ => LaneKept::Repository,
    }
}

/// The words a tracker reads a state as. None for a state another source reads.
fn words_of(state: &ipc::LifecycleStateDefinition) -> Option<&Vec<String>> {
    match &state.read {
        StateReadFrom::Tracker { words } => Some(words),
        _ => None,
    }
}

// ---- Lifecycles ---------------------------------------------------------------------------------------------------------------

fn lifecycle_of(id: &str, layers: &Layers<LifecycleDefinition>, levels: &[String], read: &PiecesRead) -> LifecycleDef {
    let clash = clash_of(layers, read);
    let own: Vec<&String> = levels.iter().filter(|level| layers.has(level)).collect();
    let piece_at = |level: &str| layers.get(level).map(|layer| &layer.piece);
    let first = piece_at(own[0]).expect("a level that keeps a piece");
    let (kind, from) = source_of(first.states.first().map(|state| &state.read));
    let mut order: Vec<(&str, &str)> = Vec::new();
    for level in &own {
        for state in &piece_at(level).expect("own").states {
            if !order.iter().any(|(id, _)| *id == state.id) {
                order.push((&state.id, level));
            }
        }
    }
    let states = order
        .iter()
        .map(|(state_id, at)| {
            let home = piece_at(at).expect("own");
            let home_state = home.states.iter().find(|one| one.id == *state_id).expect("its home has it");
            let narrower = own.iter().skip_while(|level| level.as_str() != *at).skip(1);
            let mut names = Vec::new();
            let mut reads = Vec::new();
            if words_of(home_state).is_some() {
                reads.push(at.to_string());
            }
            for level in narrower {
                let Some(mine) = piece_at(level).expect("own").states.iter().find(|one| one.id == *state_id) else { continue };
                let found = inherited(layers, level, read).found;
                let above = found.and_then(|(_, layer)| layer.piece.states.iter().find(|one| one.id == *state_id)).unwrap_or(home_state);
                if mine.label != above.label {
                    names.push((level.to_string(), mine.label.clone()));
                }
                if let Some(words) = words_of(mine).filter(|words| !words.is_empty()) {
                    if Some(words) != words_of(above) {
                        reads.push(level.to_string());
                    }
                }
            }
            // Fleet runs a tab's piece whole: a tab whose piece lacks the state does not have it, whatever a level above says.
            let absent = levels
                .iter()
                .filter(|level| {
                    exists_at(at, level, read, &clash)
                        && effective(layers, level, read).is_some_and(|(_, layer)| !layer.piece.states.iter().any(|one| one.id == *state_id))
                })
                .cloned()
                .collect();
            StateDef {
                id: state_id.to_string(),
                label: home_state.label.clone(),
                scope: at.to_string(),
                names,
                reads,
                end: home.ends.iter().any(|end| end == state_id),
                absent,
                clash: clash.clone(),
            }
        })
        .collect();
    let mut next: Vec<(String, String)> = Vec::new();
    for level in &own {
        for step in &piece_at(level).expect("own").transitions {
            let known = |id: &str| order.iter().any(|(one, _)| *one == id);
            if known(&step.from) && known(&step.to) && !next.iter().any(|(from, to)| *from == step.from && *to == step.to) {
                next.push((step.from.clone(), step.to.clone()));
            }
        }
    }
    LifecycleDef { id: id.to_string(), label: first.name.clone(), from, kind, states, next, scope: own[0].clone() }
}

// ---- Rules --------------------------------------------------------------------------------------------------------------------

/// The node a rule lands on: the state a ticket is moved to or closed in, the state a pull request is brought
/// to, the deploy a watch takes on, else the step an `at_step` condition names. None lands it nowhere.
fn landing(rule: &LifecycleRule, defs: &[Named<'_>]) -> Option<LaneRef> {
    let def = |id: &str| defs.iter().find(|one| one.id == id);
    let at = |lane: String, item: &str| Some(LaneRef { lane, item: item.to_string() });
    for one in &rule.does {
        let found = match &one.what {
            RuleAct::TrackerMove { to, target } => lane_of_target(target, rule).and_then(|lane| at(lane, to)),
            RuleAct::TrackerClose { target } => lane_of_target(target, rule).and_then(|lane| {
                let end = *def(&lane)?.ends.first()?;
                at(lane, end)
            }),
            RuleAct::ForgeMarkReady { target } => lane_of_target(target, rule).and_then(|lane| at(lane, "ready")),
            RuleAct::ForgeMerge { target } => lane_of_target(target, rule).and_then(|lane| at(lane, "merged")),
            RuleAct::StartWatch { deploy: Some(deploy), .. } if !deploy.is_empty() => {
                def(DEPLOY).and_then(|one| one.states.first().copied()).and_then(|first| at(DEPLOY.into(), first))
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    rule.conditions.iter().find_map(|one| match one {
        RuleCondition::AtStep { step } => at("workflow".into(), step),
        _ => None,
    })
}

/// What the lanes draw of one rule. None where a Job event or a gate sets it off, or it lands on no node.
fn drawn_rule(rule: &LifecycleRule, defs: &[Named<'_>]) -> Option<Drawn> {
    let (from, at) = match &rule.when {
        RuleWhen::Job { .. } | RuleWhen::GateCloses { .. } => return None,
        RuleWhen::Enters { lifecycle, state } => (LaneRef { lane: lifecycle.clone(), item: state.clone() }, LaneMoment::Enters),
        RuleWhen::StepStarts { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Starts),
        RuleWhen::StepEnds { step } => (LaneRef { lane: "workflow".into(), item: step.clone() }, LaneMoment::Ends),
    };
    let to = landing(rule, defs)?;
    let workflow = rule.conditions.iter().find_map(|one| match one {
        RuleCondition::Workflow { workflow } => Some(workflow.clone()),
        _ => None,
    });
    Some(Drawn { label: rule.name.clone(), from, at, to, autonomy: autonomy_of(rule), workflow })
}

/// A piece as it says itself apart from the scope it is kept at.
fn sans(piece: &LifecycleRule) -> LifecycleRule {
    LifecycleRule { scope: PieceScope::Everywhere, ..piece.clone() }
}

fn rule_of(id: &str, layers: &Layers<LifecycleRule>, defs: &[Named<'_>], levels: &[String], read: &PiecesRead) -> Option<RuleDef> {
    let own: Vec<&String> = levels.iter().filter(|level| layers.has(level)).collect();
    let base = layers.get(own[0])?;
    let drawn = drawn_rule(&base.piece, defs)?;
    // **A level that keeps a piece of its own has that piece whole**: Fleet runs it in place of the one above.
    let overrides = own[1..]
        .iter()
        .filter_map(|level| {
            let piece = &layers.get(level)?.piece;
            let mine = drawn_rule(piece, defs)?;
            let above = inherited(layers, level, read).found.map(|(_, layer)| &layer.piece);
            match above {
                Some(above) if sans(piece) == sans(above) => None,
                _ => Some((level.to_string(), mine)),
            }
        })
        .collect();
    Some(RuleDef { id: id.to_string(), drawn, kept: kept_as(&base.kept), scope: own[0].clone(), overrides, clash: clash_of(layers, read) })
}

// ---- The Workspace ------------------------------------------------------------------------------------------------------------

/// The read with the lifecycles of its drafts that no repository keeps, kept as the first repository's own so
/// the lanes draw them.
fn with_draft_lifecycles(read: &PiecesRead, drafts: &Drafts) -> PiecesRead {
    let held = lifecycle_layers(read);
    let added: Vec<ipc::AppliedLifecycle> = drafts
        .lifecycles
        .iter()
        .filter(|one| !held.iter().any(|(id, _)| *id == one.id))
        .map(|one| ipc::AppliedLifecycle {
            lifecycle: one.clone(),
            kept: PieceKept {
                rank: PieceRank { reach: "repository".into(), source: "repository".into(), home: false },
                path: Some(format!("{}.yml", one.id)),
                overridden_by: None,
                hides: None,
            },
        })
        .collect();
    let mut out = read.clone();
    if let Some(first) = out.repositories.first_mut() {
        first.lifecycles.extend(added);
    }
    out
}

/// The Workspace the editor draws. **A piece scoped to a repository this read did not cover has no level to be
/// drawn at**, and is left out.
pub(super) fn workspace_of(read: &PiecesRead, drafts: &Drafts) -> Workspace {
    let read = &with_draft_lifecycles(read, drafts);
    let levels = levels_of_read(read);
    let readable = |own: &dyn Fn(&str) -> bool| levels.iter().any(|level| own(level));
    let lifecycles: Vec<LifecycleDef> = lifecycle_layers(read)
        .iter()
        .filter(|(_, layers)| readable(&|level| layers.has(level)))
        .map(|(id, layers)| lifecycle_of(id, layers, &levels, read))
        .collect();
    let named: Vec<Named<'_>> = lifecycles
        .iter()
        .map(|one| Named {
            id: &one.id,
            states: one.states.iter().map(|state| state.id.as_str()).collect(),
            ends: one.states.iter().filter(|state| state.end).map(|state| state.id.as_str()).collect(),
        })
        .collect();
    let held = rule_layers(read);
    let rules = held
        .iter()
        .filter(|(_, layers)| readable(&|level| layers.has(level)))
        .filter_map(|(id, layers)| rule_of(id, layers, &named, &levels, read))
        .collect();
    let drafted = drafts
        .rules
        .iter()
        .filter(|one| !held.iter().any(|(id, _)| *id == one.id))
        .filter_map(|one| {
            Some(RuleDef {
                id: one.id.clone(),
                drawn: drawn_rule(one, &named)?,
                kept: LaneKept::Repository,
                scope: level_of(&one.scope, read),
                overrides: Vec::new(),
                clash: Vec::new(),
            })
        })
        .collect();
    Workspace { lifecycles, rules, drafted }
}

impl StateDef {
    /// The levels below the default that name it another way, read it as other words, leave it out, or add it.
    pub(super) fn differs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let all = self.reads.iter().chain(self.names.iter().map(|(level, _)| level)).chain(self.absent.iter()).chain(std::iter::once(&self.scope));
        for level in all {
            if level != DEFAULT && !out.contains(level) {
                out.push(level.clone());
            }
        }
        out
    }

    /// The levels that set it: where it was added, and each that names it or reads it another way.
    pub(super) fn owners(&self) -> Vec<String> {
        std::iter::once(self.scope.clone()).chain(self.names.iter().map(|(level, _)| level.clone())).chain(self.reads.iter().cloned()).collect()
    }
}

impl RuleDef {
    /// The levels it differs at: where it applies alone, and where a level below changes it.
    pub(super) fn differs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let all = std::iter::once(&self.scope).filter(|scope| *scope != DEFAULT).chain(self.overrides.iter().map(|(level, _)| level));
        for level in all {
            if !out.contains(level) {
                out.push(level.clone());
            }
        }
        out
    }

    /// The levels that set it: where it applies, and each that changes it.
    pub(super) fn owners(&self) -> Vec<String> {
        std::iter::once(self.scope.clone()).chain(self.overrides.iter().map(|(level, _)| level.clone())).collect()
    }
}
