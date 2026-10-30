//! `ends_on`, read: the words for what the person does, and a lifecycle with its state.
//!
//! **A lifecycle may be written three ways and means one thing.** The string `"pr: approved"`, the
//! map `{ pr: approved }` that an unquoted `- pr: approved` is, and for the first of several
//! instances `any ticket: closed`, which an unquoted line reads as the map `{ "any ticket": closed }`.
//! Whether the lifecycle exists is not asked here: a file is read before anyone knows which apply to
//! the repository, and [`names_no_lifecycle`] asks it once Fleet does.

use std::fmt;

use core_model::{Lifecycle, LifecycleEnd, NotALifecycleEnd, StepEnd};
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::yaml;

/// The words, in the order the refusal lists them.
const WORDS: &[&str] = &["replied", "walk_approved", "sketch_approved", "draft_approved", "idea_picked", "document_approved"];

/// What is wrong with an `ends_on` item that is a lifecycle end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndFault {
    /// Written like a lifecycle and not one.
    Written(NotALifecycleEnd),
    /// A map that is not one lifecycle with one state.
    NotOneState,
    /// On a step whose place is already decided by what it is, which a lifecycle would be ignored on.
    Placed { by: &'static str },
}

impl fmt::Display for EndFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EndFault::Written(why) => write!(f, "is not a lifecycle end: {why}"),
            EndFault::NotOneState => f.write_str("wanted one lifecycle and its state, such as `pr: approved`"),
            EndFault::Placed { by } => write!(
                f,
                "puts a lifecycle end on a step that {by}, and that decides where the step is. A step waits on a \
                 lifecycle state when nothing else places it: put the wait on the step before or after"
            ),
        }
    }
}

/// The list at `key`, each item a word or a lifecycle end.
pub(super) fn read(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Vec<StepEnd>> {
    let items = yaml::list(key, value, out)?;
    let read: Vec<Option<StepEnd>> = items.iter().map(|(at, item)| one(at, item, out)).collect();
    read.into_iter().collect()
}

fn one(key: &str, item: &Value, out: &mut Vec<Refusal>) -> Option<StepEnd> {
    let written = match item {
        Value::Mapping(map) => match map.iter().collect::<Vec<_>>().as_slice() {
            [(Value::String(left), Value::String(right))] => format!("{left}: {right}"),
            _ => {
                out.push(Refusal::new(key, Fault::Ends(EndFault::NotOneState)));
                return None;
            }
        },
        other => {
            let found = yaml::text(key, other, out)?;
            if !found.contains(':') {
                return word(key, found, out);
            }
            found
        }
    };
    match LifecycleEnd::from_wire(&written) {
        Ok(end) => Some(StepEnd::Lifecycle(end)),
        Err(why) => {
            out.push(Refusal::new(key, Fault::Ends(EndFault::Written(why))));
            None
        }
    }
}

fn word(key: &str, found: String, out: &mut Vec<Refusal>) -> Option<StepEnd> {
    if let Some(end) = StepEnd::ALL.iter().find(|end| end.as_wire() == found) {
        return Some(end.clone());
    }
    out.push(Refusal::new(key, Fault::NotInTheSchema { value: found, legal: WORDS }));
    None
}

/// A lifecycle end on a step that something else places. The refusal, where there is one.
pub(super) fn placed(key: &str, ends: &[StepEnd], step: Placed) -> Option<Refusal> {
    let by = step.by()?;
    ends.iter().any(|end| end.lifecycle().is_some()).then(|| Refusal::new(key, Fault::Ends(EndFault::Placed { by })))
}

/// What already decides where a step is.
#[derive(Clone, Copy)]
pub(super) struct Placed {
    pub walks: bool,
    pub follows_plan: bool,
    pub records_plan: bool,
}

impl Placed {
    fn by(self) -> Option<&'static str> {
        [
            (self.walks, "walks"),
            (self.follows_plan, "follows the plan"),
            (self.records_plan, "records the plan"),
        ]
        .into_iter()
        .find_map(|(is, by)| is.then_some(by))
    }
}

/// A lifecycle end that names a lifecycle or a state none of `lifecycles` has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesNoLifecycle {
    /// The step's id.
    pub step: String,
    pub end: LifecycleEnd,
    /// The lifecycle's states, where it exists and lacks the state; absent where it does not exist.
    pub states: Option<Vec<String>>,
    /// Every lifecycle there is.
    pub lifecycles: Vec<String>,
}

impl fmt::Display for NamesNoLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.states {
            Some(states) => write!(
                f,
                "step `{}` ends on `{}`, and the {} lifecycle has no state `{}`. It has {}",
                self.step,
                self.end,
                self.end.lifecycle(),
                self.end.state(),
                states.join(", ")
            ),
            None => write!(
                f,
                "step `{}` ends on `{}`, and no lifecycle here is called `{}`. There is {}",
                self.step,
                self.end,
                self.end.lifecycle(),
                self.lifecycles.join(", ")
            ),
        }
    }
}

/// Every lifecycle end among `steps` (each a step's id and what ends it) that `lifecycles` cannot
/// reach, in the order written.
pub fn names_no_lifecycle<'a>(
    steps: impl IntoIterator<Item = (&'a str, &'a [StepEnd])>,
    lifecycles: &[Lifecycle],
) -> Vec<NamesNoLifecycle> {
    let known = || lifecycles.iter().map(|one| one.id().to_string()).collect::<Vec<_>>();
    let mut refused = Vec::new();
    for (step, ends) in steps {
        for end in ends.iter().filter_map(StepEnd::lifecycle) {
            let found = lifecycles.iter().find(|one| one.id() == end.lifecycle());
            let states = match found {
                Some(one) if one.state(end.state()).is_some() => continue,
                Some(one) => Some(one.states().iter().map(|state| state.id().to_string()).collect()),
                None => None,
            };
            refused.push(NamesNoLifecycle { step: step.to_string(), end: end.clone(), states, lifecycles: known() });
        }
    }
    refused
}
