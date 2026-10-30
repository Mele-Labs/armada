//! What happened, and the memory that lets a rule run once for it.
//!
//! [`Occurrence`] is **the one value the engine takes**: a change of standing, a step starting or
//! ending, or a Job event, with when it was seen. Whatever Fleet keeps a transition as, it becomes
//! an `Occurrence` in one place, [`Occurrence::entered`] for an instance's move and the others for
//! the rest, so the engine never learns where a transition was stored.
//!
//! **A rule runs once for each occurrence.** [`OccurrenceKey`] is the occurrence's identity, and
//! [`RanOnce`] is what remembers which rules have seen which. A second read of the same state
//! moves nothing, so it is no occurrence; a replay after a restart is the same occurrence again,
//! and finds its key already there.

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::lifecycle::{InstanceRef, Moved, Standing};
use crate::workflow::StepId;

use super::ids::RuleId;
use super::when::{JobEvent, When};

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Happening {
    /// An instance changed standing.
    Entered { instance: InstanceRef, moved: Moved },
    StepStarted(StepId),
    StepEnded(StepId),
    Job(JobEvent),
}

/// What happened and when it was seen.
///
/// **`at` is the reader's own stamp and only ever compared for equality**, so the engine has no
/// clock and no format to disagree about. An instance that leaves a state and enters it again is
/// two occurrences because they were seen at two times.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    happening: Happening,
    at: String,
}

impl Occurrence {
    /// An instance moved. This is where a reader's transition becomes one.
    pub fn entered(instance: InstanceRef, moved: Moved, at: &str) -> Occurrence {
        Occurrence { happening: Happening::Entered { instance, moved }, at: String::from(at) }
    }

    pub fn step_started(step: StepId, at: &str) -> Occurrence {
        Occurrence { happening: Happening::StepStarted(step), at: String::from(at) }
    }

    pub fn step_ended(step: StepId, at: &str) -> Occurrence {
        Occurrence { happening: Happening::StepEnded(step), at: String::from(at) }
    }

    pub fn job(event: JobEvent, at: &str) -> Occurrence {
        Occurrence { happening: Happening::Job(event), at: String::from(at) }
    }

    pub fn happening(&self) -> &Happening {
        &self.happening
    }

    pub fn at(&self) -> &str {
        &self.at
    }

    /// The instance that entered a state, where this is that.
    pub fn entered_instance(&self) -> Option<&InstanceRef> {
        match &self.happening {
            Happening::Entered { instance, .. } => Some(instance),
            _ => None,
        }
    }

    /// Whether this is the moment `when` names. **Entering a state is matched on where the
    /// instance landed**, so an instance that landed nowhere a state claims sets off nothing.
    pub fn is(&self, when: &When) -> bool {
        match (&self.happening, when) {
            (Happening::Entered { instance, moved }, When::Enters { lifecycle, state }) => {
                instance.lifecycle() == lifecycle && moved.to == Standing::In(state.clone())
            }
            (Happening::StepStarted(seen), When::StepStarts(named)) => seen == named,
            (Happening::StepEnded(seen), When::StepEnds(named)) => seen == named,
            (Happening::Job(seen), When::Job(named)) => seen == named,
            _ => false,
        }
    }

    /// Its identity. Two occurrences have one key when everything that tells them apart is equal.
    pub fn key(&self) -> OccurrenceKey {
        let mut parts: Vec<String> = Vec::new();
        match &self.happening {
            Happening::Entered { instance, moved } => {
                parts.extend([
                    String::from("entered"),
                    String::from(instance.lifecycle().as_str()),
                    String::from(instance.key()),
                    spelled(&moved.from),
                    spelled(&moved.to),
                ]);
            }
            Happening::StepStarted(step) => parts.extend([String::from("step_started"), String::from(step.as_str())]),
            Happening::StepEnded(step) => parts.extend([String::from("step_ended"), String::from(step.as_str())]),
            Happening::Job(event) => parts.extend([String::from("job"), String::from(event.as_wire())]),
        }
        parts.push(self.at.clone());
        OccurrenceKey(parts)
    }
}

fn spelled(standing: &Standing) -> String {
    match standing {
        Standing::Unread => String::from("unread"),
        Standing::In(state) => format!("in:{state}"),
        Standing::Unmapped(word) => format!("unmapped:{word}"),
    }
}

/// An occurrence's identity: what happened, to what, from where to where, and when.
///
/// Kept as parts and written out with each one's length in front, so no two different occurrences
/// spell alike whatever a tracker's word contains.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccurrenceKey(Vec<String>);

impl fmt::Display for OccurrenceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|part| write!(f, "{}:{part}", part.len()))
    }
}

/// Which rules have seen which occurrences of one Job. **A rule that has is not run again.**
///
/// Held by the Job and rebuilt after a restart from what Fleet kept, with `collect`. The engine is
/// the only thing that adds to it, so a key is there because a rule was run for it and not because
/// someone said so.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RanOnce {
    seen: BTreeSet<(RuleId, OccurrenceKey)>,
}

impl RanOnce {
    pub fn new() -> RanOnce {
        RanOnce::default()
    }

    pub fn has(&self, rule: &RuleId, key: &OccurrenceKey) -> bool {
        self.seen.contains(&(rule.clone(), key.clone()))
    }

    /// Every pair, in a stable order, for Fleet to keep.
    pub fn iter(&self) -> impl Iterator<Item = (&RuleId, &OccurrenceKey)> {
        self.seen.iter().map(|(rule, key)| (rule, key))
    }

    /// Note that `rule` has seen `key`. **False where it had**, which is how the engine knows to
    /// run nothing.
    pub(super) fn note(&mut self, rule: &RuleId, key: &OccurrenceKey) -> bool {
        self.seen.insert((rule.clone(), key.clone()))
    }
}

impl FromIterator<(RuleId, OccurrenceKey)> for RanOnce {
    fn from_iter<I: IntoIterator<Item = (RuleId, OccurrenceKey)>>(pairs: I) -> RanOnce {
        RanOnce { seen: pairs.into_iter().collect() }
    }
}
