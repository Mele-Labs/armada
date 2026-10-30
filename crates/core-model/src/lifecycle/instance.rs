//! One thing a Job holds, such as ticket ARM-41 or PR #812, standing in a state of its lifecycle.
//!
//! **An instance starts when the Job takes the thing on** (tickets attached at dispatch, a pull
//! request the Job opens, a deploy a merge sets off) and is linked to the instances it came from,
//! so a pull request knows its tickets and a rule's conditions can read across the links. A Job a
//! Job starts is an instance of the Job lifecycle, linked to the Job that started it.
//!
//! **A live instance holds its Job open.** [`HeldInstances::in_flight`] is what is left when the
//! ones in an end state are taken away, and it is what End lists before it ends the Job.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::ids::LifecycleId;
use super::machine::{Lifecycle, Standing};
use super::read::Observed;

/// Which thing, by its lifecycle and the key its source gives it: `ticket` and `ARM-41`.
///
/// Made through the lifecycle it is of, so the lifecycle id on it is one that was declared.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceRef {
    lifecycle: LifecycleId,
    key: String,
}

impl InstanceRef {
    /// A thing of `lifecycle`, named by `key` trimmed. A blank key names nothing.
    pub fn of(lifecycle: &Lifecycle, key: &str) -> Result<InstanceRef, BlankKey> {
        match key.trim() {
            "" => Err(BlankKey),
            trimmed => Ok(InstanceRef { lifecycle: lifecycle.id().clone(), key: String::from(trimmed) }),
        }
    }

    pub fn lifecycle(&self) -> &LifecycleId {
        &self.lifecycle
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for InstanceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.lifecycle, self.key)
    }
}

/// An instance key with nothing in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankKey;

impl fmt::Display for BlankKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a thing a Job holds is named by its ticket key or its pull request number, and blank is neither")
    }
}

impl core::error::Error for BlankKey {}

/// A change of standing a read brought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub from: Standing,
    pub to: Standing,
}

/// One thing a Job holds, where it stands, and what it is linked to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleInstance {
    subject: InstanceRef,
    standing: Standing,
    links: Vec<InstanceRef>,
    handed: bool,
}

impl LifecycleInstance {
    /// A thing just taken on, standing nowhere until it is read.
    pub fn taken_on(subject: InstanceRef) -> LifecycleInstance {
        LifecycleInstance { subject, standing: Standing::Unread, links: Vec::new(), handed: false }
    }

    /// Given to another Job, which holds it now. The Job still holds the record, so a link from a
    /// Job it started finds it, and `Among::Held` no longer does.
    pub fn handed_on(mut self) -> LifecycleInstance {
        self.handed = true;
        self
    }

    pub fn is_handed(&self) -> bool {
        self.handed
    }

    /// Linked to an instance it came from. A link to itself, or to one already linked, adds
    /// nothing.
    pub fn linked_to(mut self, other: InstanceRef) -> LifecycleInstance {
        if other != self.subject && !self.links.contains(&other) {
            self.links.push(other);
        }
        self
    }

    pub fn subject(&self) -> &InstanceRef {
        &self.subject
    }

    pub fn standing(&self) -> &Standing {
        &self.standing
    }

    /// What it is linked to, in the order the links were made. **An instance a Job does not hold
    /// can be linked**, as the Job that started this one is.
    pub fn links(&self) -> &[InstanceRef] {
        &self.links
    }

    /// Record something seen. **The only way a standing changes**: there is no setter, and what an
    /// agent says is not an [`Observed`], so a state can be read and cannot be claimed.
    ///
    /// `None` where it changed nothing, which is what lets a rule run once for each instance
    /// entering a state and run nothing on the second read of the same one. Refused where
    /// `lifecycle` is not the one this instance is of.
    pub fn observe(
        &mut self,
        lifecycle: &Lifecycle,
        seen: &Observed<'_>,
    ) -> Result<Option<Moved>, NotOfThisLifecycle> {
        if lifecycle.id() != self.subject.lifecycle() {
            return Err(NotOfThisLifecycle { held: self.subject.clone(), given: lifecycle.id().clone() });
        }
        let now = lifecycle.standing_of(seen);
        if self.standing == now {
            return Ok(None);
        }
        let from = core::mem::replace(&mut self.standing, now.clone());
        Ok(Some(Moved { from, to: now }))
    }
}

/// A read against a lifecycle the instance is not of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotOfThisLifecycle {
    pub held: InstanceRef,
    pub given: LifecycleId,
}

impl fmt::Display for NotOfThisLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} cannot be read against the {} lifecycle", self.held, self.given)
    }
}

impl core::error::Error for NotOfThisLifecycle {}

/// The instances one Job holds.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HeldInstances {
    instances: Vec<LifecycleInstance>,
}

/// An instance the Job already holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlreadyHeld(pub InstanceRef);

impl fmt::Display for AlreadyHeld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is already held by this Job", self.0)
    }
}

impl core::error::Error for AlreadyHeld {}

impl HeldInstances {
    pub fn new() -> HeldInstances {
        HeldInstances::default()
    }

    /// Hold one more. One thing is held once: its standing is single.
    pub fn take_on(&mut self, instance: LifecycleInstance) -> Result<(), AlreadyHeld> {
        match self.get(instance.subject()) {
            Some(_) => Err(AlreadyHeld(instance.subject)),
            None => {
                self.instances.push(instance);
                Ok(())
            }
        }
    }

    pub fn get(&self, subject: &InstanceRef) -> Option<&LifecycleInstance> {
        self.instances.iter().find(|held| &held.subject == subject)
    }

    pub fn get_mut(&mut self, subject: &InstanceRef) -> Option<&mut LifecycleInstance> {
        self.instances.iter_mut().find(|held| &held.subject == subject)
    }

    /// Every instance, in the order they were taken on.
    pub fn all(&self) -> &[LifecycleInstance] {
        &self.instances
    }

    /// What `of` came from and the Job holds: a pull request's tickets.
    pub fn linked(&self, of: &InstanceRef) -> Vec<&LifecycleInstance> {
        match self.get(of) {
            Some(held) => held.links.iter().filter_map(|link| self.get(link)).collect(),
            None => Vec::new(),
        }
    }

    /// What came from `of` and the Job holds: the Jobs a Job started.
    pub fn linking(&self, of: &InstanceRef) -> Vec<&LifecycleInstance> {
        self.instances.iter().filter(|held| held.links.contains(of)).collect()
    }

    /// What is not over, given the lifecycles the Job holds, one for each id. **An instance whose
    /// lifecycle is not among them is in flight**: nothing can say it ended.
    pub fn in_flight(&self, lifecycles: &[Lifecycle]) -> Vec<&LifecycleInstance> {
        self.instances
            .iter()
            .filter(|held| {
                let own = lifecycles.iter().find(|lifecycle| lifecycle.id() == held.subject.lifecycle());
                !own.is_some_and(|lifecycle| lifecycle.is_over(&held.standing))
            })
            .collect()
    }
}
