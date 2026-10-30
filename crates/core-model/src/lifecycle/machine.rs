//! A lifecycle: the states one kind of thing passes through, how each is read, and which are ends.
//!
//! **Declared once, checked once.** [`Lifecycle::declared`] is the only way to make one, and it
//! refuses a lifecycle that names a state it does not have, reads one fact two ways, or can never
//! end, so nothing downstream asks again whether a transition's far end exists.
//!
//! **A transition is an expected move, not a permitted one.** A state is read from the world and
//! never set, so a ticket that goes from To do straight to Closed is a ticket in Closed whether or
//! not the lifecycle listed that move. Transitions are what a rule's When can name and what an
//! editor draws; [`Lifecycle::allows`] says whether a move was one the lifecycle expected.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::workflow::Title;

use super::ids::{LifecycleId, ScopeId, StateId};
use super::read::{Observed, Overlap, StateRead};

/// Where a lifecycle applies.
///
/// **Which of two lifecycles with one id wins is not decided here.** The narrower scope wins and
/// yours beats the repository's, and that is the work of reading the places a piece is kept.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleScope {
    /// Every repository, which is where the lifecycles Armada carries apply.
    Everywhere,
    /// One Workspace (a set of repositories) or one repository, by id.
    Within(ScopeId),
}

/// One state of a lifecycle: what it is called, and how an instance is seen to be in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleState {
    id: StateId,
    label: Title,
    read: StateRead,
}

impl LifecycleState {
    pub fn new(id: StateId, label: Title, read: StateRead) -> LifecycleState {
        LifecycleState { id, label, read }
    }

    pub fn id(&self) -> &StateId {
        &self.id
    }

    /// What a person reads for it.
    pub fn label(&self) -> &Title {
        &self.label
    }

    pub fn read(&self) -> &StateRead {
        &self.read
    }
}

/// A move the lifecycle expects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleTransition {
    pub from: StateId,
    pub to: StateId,
}

/// Where an instance stands: in a state, in none yet, or somewhere the lifecycle has no state for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Taken on and not yet read.
    Unread,
    In(StateId),
    /// A source said something no state claims. **Kept as it was said** and never rounded to the
    /// nearest state: a rule that wrote to a tracker on a guess about its words has acted on
    /// nothing anyone said. It is not an end, so the instance stays in flight until it is mapped.
    Unmapped(String),
}

/// A lifecycle: an id, its states, the moves it expects and the states that end it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lifecycle {
    id: LifecycleId,
    name: Title,
    scope: LifecycleScope,
    states: Vec<LifecycleState>,
    transitions: Vec<LifecycleTransition>,
    ends: Vec<StateId>,
}

impl Lifecycle {
    /// A lifecycle, or the first thing wrong with it.
    pub fn declared(
        id: LifecycleId,
        name: Title,
        scope: LifecycleScope,
        states: Vec<LifecycleState>,
        transitions: Vec<LifecycleTransition>,
        ends: Vec<StateId>,
    ) -> Result<Lifecycle, MalformedLifecycle> {
        if states.is_empty() {
            return Err(MalformedLifecycle::NoStates);
        }
        for (n, state) in states.iter().enumerate() {
            if states[..n].iter().any(|earlier| earlier.id == state.id) {
                return Err(MalformedLifecycle::StateTwice { state: state.id.clone() });
            }
            for earlier in &states[..n] {
                match earlier.read.overlaps(&state.read) {
                    Some(Overlap::Word(word)) => {
                        return Err(MalformedLifecycle::WordClaimedTwice {
                            word,
                            first: earlier.id.clone(),
                            second: state.id.clone(),
                        })
                    }
                    Some(Overlap::Fact) => {
                        return Err(MalformedLifecycle::FactClaimedTwice {
                            first: earlier.id.clone(),
                            second: state.id.clone(),
                        })
                    }
                    None => {}
                }
            }
        }
        let has = |wanted: &StateId| states.iter().any(|state| &state.id == wanted);
        for transition in &transitions {
            for named in [&transition.from, &transition.to] {
                if !has(named) {
                    return Err(MalformedLifecycle::TransitionNamesNoState { state: named.clone() });
                }
            }
        }
        if ends.is_empty() {
            return Err(MalformedLifecycle::NoEnd);
        }
        if let Some(missing) = ends.iter().find(|named| !has(named)) {
            return Err(MalformedLifecycle::EndNamesNoState { state: missing.clone() });
        }
        Ok(Lifecycle { id, name, scope, states, transitions, ends })
    }

    pub fn id(&self) -> &LifecycleId {
        &self.id
    }

    pub fn name(&self) -> &Title {
        &self.name
    }

    pub fn scope(&self) -> &LifecycleScope {
        &self.scope
    }

    /// The states, in the order the lifecycle declared them.
    pub fn states(&self) -> &[LifecycleState] {
        &self.states
    }

    pub fn state(&self, id: &StateId) -> Option<&LifecycleState> {
        self.states.iter().find(|state| &state.id == id)
    }

    pub fn transitions(&self) -> &[LifecycleTransition] {
        &self.transitions
    }

    /// The states that end an instance.
    pub fn ends(&self) -> &[StateId] {
        &self.ends
    }

    /// Where something seen stands in this lifecycle. A value no state claims is
    /// [`Standing::Unmapped`], and no other state is tried.
    pub fn standing_of(&self, seen: &Observed<'_>) -> Standing {
        match self.states.iter().find(|state| state.read.claims(seen)) {
            Some(state) => Standing::In(state.id.clone()),
            None => Standing::Unmapped(seen.spelled()),
        }
    }

    /// Whether `from` to `to` is a move the lifecycle expects.
    pub fn allows(&self, from: &StateId, to: &StateId) -> bool {
        self.transitions.iter().any(|move_| &move_.from == from && &move_.to == to)
    }

    /// Whether the standing is an end. **Only a state ends an instance**: one not read yet or not
    /// mapped is still going, because an instance nobody can place must not be let go.
    pub fn is_over(&self, standing: &Standing) -> bool {
        match standing {
            Standing::In(state) => self.ends.contains(state),
            Standing::Unread | Standing::Unmapped(_) => false,
        }
    }
}

/// What is wrong with a lifecycle that was declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MalformedLifecycle {
    NoStates,
    StateTwice { state: StateId },
    /// A tracker word with nothing in it.
    BlankWord,
    /// A tracker word listed twice in one state.
    WordTwice { word: String },
    /// A tracker word listed in two states.
    WordClaimedTwice { word: String, first: StateId, second: StateId },
    /// A forge, watch or Fleet fact two states are read from.
    FactClaimedTwice { first: StateId, second: StateId },
    TransitionNamesNoState { state: StateId },
    EndNamesNoState { state: StateId },
    /// Nothing ends it, so an instance of it would hold a Job open for good.
    NoEnd,
}

impl fmt::Display for MalformedLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MalformedLifecycle::NoStates => f.write_str("has no state"),
            MalformedLifecycle::StateTwice { state } => write!(f, "declares `{state}` twice"),
            MalformedLifecycle::BlankWord => f.write_str("lists a blank tracker word"),
            MalformedLifecycle::WordTwice { word } => write!(f, "lists the tracker word `{word}` twice in one state"),
            MalformedLifecycle::WordClaimedTwice { word, first, second } => write!(
                f,
                "has the tracker word `{word}` in `{first}` and in `{second}`, so one status would be in two states"
            ),
            MalformedLifecycle::FactClaimedTwice { first, second } => {
                write!(f, "reads `{first}` and `{second}` from the same fact, so one thing would be in two states")
            }
            MalformedLifecycle::TransitionNamesNoState { state } => {
                write!(f, "has a transition naming `{state}`, which it does not declare")
            }
            MalformedLifecycle::EndNamesNoState { state } => {
                write!(f, "ends in `{state}`, which it does not declare")
            }
            MalformedLifecycle::NoEnd => {
                f.write_str("has no end state, so an instance of it would hold its Job open for good")
            }
        }
    }
}

impl core::error::Error for MalformedLifecycle {}
