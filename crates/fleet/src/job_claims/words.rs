//! What Fleet tells a Job about its claims, and the line its record shows while it waits. Bare
//! facts, each with what ends the wait: `docs/concepts/fleet.md`, *Claims a Job holds*.

use adapter_traits::Claimant;

use super::{WaitedOn, Wanted};
use crate::clashing::listed;

/// Who holds a claim, as a person reads it: the Job's title, the person and their Mac.
pub(crate) fn holder_named(by: &Claimant) -> String {
    let job = match by.title.trim() {
        "" => "a Job".to_string(),
        title => format!("Job \"{title}\""),
    };
    match by.name.trim() {
        "" => format!("{job} on {}", by.holder.machine),
        name => format!("{job} on {name}'s Mac {}", by.holder.machine),
    }
}

/// The word that starts a wait.
pub(crate) fn waiting(item: &str, on: &WaitedOn) -> String {
    match on {
        WaitedOn::Job(by) => format!(
            "{item} is claimed by {}. Leave {item} and work on nothing it needs. Fleet tells you when the claim ends.",
            holder_named(by)
        ),
        WaitedOn::Person(person) => format!(
            "{item} is assigned to {person} in beads, so it is their work. Leave {item} and work on nothing it needs. Fleet tells you when it is unassigned."
        ),
        WaitedOn::Main => format!("{item} fails on main too. Main's own fix covers it."),
    }
}

/// The word that ends a wait: nobody else holds the item now.
pub(crate) fn wait_over(item: &str) -> String {
    format!("{item} is free now. Carry on with it.")
}

/// The word that ends a wait on an item that closed meanwhile.
pub(crate) fn item_closed(item: &str) -> String {
    format!("{item} closed or left your Job while you waited on it. Fleet holds no claim on it for you.")
}

/// The word to a Job whose files another Job holds, or reached after it. Both carry on.
pub(crate) fn files_shared(files: &[&Wanted], by: &Claimant) -> String {
    let paths: Vec<String> = files.iter().map(|file| file.named.clone()).collect();
    format!("{} also writes {}. Both Jobs carry on. Keep your change to these files to what your work needs.", holder_named(by), listed(&paths))
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_traits::{Holder, HolderKind};

    #[test]
    fn a_holder_is_named_by_job_person_and_mac() {
        let holder = Holder::new(HolderKind::Session, "j1", "home", 0).expect("a holder");
        assert_eq!(holder_named(&Claimant::new(holder.clone(), "nick", "Fix the flaky queue test")), "Job \"Fix the flaky queue test\" on nick's Mac home");
        assert_eq!(holder_named(&Claimant::new(holder, "", "")), "a Job on home");
    }
}
