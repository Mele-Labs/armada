//! A Job's instances in memory: rebuilt from what the store kept, handed what was seen, and the
//! move that came of it.
//!
//! **Rebuilt by reading again.** An instance is taken on, linked, and handed the last thing it saw,
//! so it stands where that read put it and the store never has a standing to give back that no read
//! produced. A second read of the same state then moves nothing, across a restart as within a run.

use core_model::{
    HeldInstances, InstanceRef, Lifecycle, LifecycleInstance, Moved, Observed, StateSource,
};
use store::KeptInstance;

/// A change of standing, with what brought it and when. **One row of the Job's append-only list**,
/// which a rule that waits on a state enters reads from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub instance: InstanceRef,
    pub moved: Moved,
    /// The source that said it, and what it said in its own words.
    pub source: StateSource,
    pub seen: String,
    pub at: String,
}

/// The instances `kept` describes, against the lifecycles that apply. **One whose lifecycle is not
/// among them is left out**, as is a link to one: nothing can say where it stands.
pub fn rebuilt(lifecycles: &[Lifecycle], kept: &[KeptInstance]) -> HeldInstances {
    let find = |id: &str| lifecycles.iter().find(|one| one.id().as_str() == id);
    let mut held = HeldInstances::new();
    for one in kept {
        let Some(lifecycle) = find(&one.lifecycle) else { continue };
        let Ok(subject) = InstanceRef::of(lifecycle, &one.key) else { continue };
        let mut instance = LifecycleInstance::taken_on(subject);
        if !one.handed_to.is_empty() {
            instance = instance.handed_on();
        }
        for (to_lifecycle, to_key) in &one.links {
            if let Some(to) = find(to_lifecycle).and_then(|linked| InstanceRef::of(linked, to_key).ok()) {
                instance = instance.linked_to(to);
            }
        }
        let seen = StateSource::from_wire(&one.seen_source).and_then(|source| Observed::read(source, &one.seen));
        if let Some(seen) = seen {
            let _ = instance.observe(lifecycle, &seen);
        }
        let _ = held.take_on(instance);
    }
    held
}

/// Hand `subject` what was seen. **`None` where it stood there already**, or where the Job does
/// not hold it, so a second read of the same state records nothing.
pub fn read_instance(
    held: &mut HeldInstances,
    lifecycles: &[Lifecycle],
    subject: &InstanceRef,
    seen: &Observed<'_>,
    at: &str,
) -> Option<Transition> {
    let lifecycle = lifecycles.iter().find(|one| one.id() == subject.lifecycle())?;
    let moved = held.get_mut(subject)?.observe(lifecycle, seen).ok().flatten()?;
    Some(Transition { instance: subject.clone(), moved, source: seen.source(), seen: seen.spelled(), at: at.to_string() })
}

/// The instance of `lifecycle` named `key`, as the Job holds it.
pub fn subject_of(held: &HeldInstances, lifecycle: &str, key: &str) -> Option<InstanceRef> {
    held.all()
        .iter()
        .map(LifecycleInstance::subject)
        .find(|one| one.lifecycle().as_str() == lifecycle && one.key() == key)
        .cloned()
}
