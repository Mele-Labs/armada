//! End lists what is still in flight before it ends the Job. **Each instance no end state has taken
//! is listed by what a person calls it and the state it stands in** (`Deploy`, Rolling out), a status
//! nobody could place by the word its tracker used, and one not read yet by its name alone. What a
//! state has ended is not listed, and a Job that holds nothing lists nothing. **Only an instance in
//! a state its lifecycle knows holds the Job open**: End lists an unread or unmapped one and the
//! agent's `all_done` is not refused for it. Nothing here opens a store, a tracker or a forge.

use core_model::{ForgeFact, HeldInstances, InstanceRef, Observed, ReadingState};
use fleet::lifecycles::ending::{flying, Flying};

use crate::bench::lifecycles::{see, taken_on, the_jobs_copy};

/// The name a person gives it, as the Job's lifecycles view spells the pull request.
fn called(subject: &InstanceRef) -> String {
    match subject.lifecycle().as_str() {
        "pr" => format!("PR #{}", subject.key()),
        _ => subject.key().to_string(),
    }
}

fn listed(held: &HeldInstances) -> Vec<(String, String)> {
    let copy = the_jobs_copy();
    flying(held, &copy, &called).into_iter().map(|one| (one.label, one.state)).collect()
}

fn row(label: &str, state: &str) -> (String, String) {
    (label.to_string(), state.to_string())
}

#[test]
fn end_lists_each_instance_no_end_state_has_taken_by_its_name_and_its_state_and_a_job_holding_none_lists_nothing() {
    let copy = the_jobs_copy();
    let (tickets, pull_request, deploy) = (&copy[0], &copy[1], &copy[2]);
    let mut held = HeldInstances::new();
    assert!(listed(&held).is_empty(), "a Job that began before lifecycles holds nothing, so End has nothing to list");

    for (lifecycle, key) in [(tickets, "ARM-41"), (tickets, "ARM-42"), (tickets, "ARM-43"), (tickets, "ARM-44"), (pull_request, "812"), (deploy, "Deploy")] {
        held.take_on(taken_on(lifecycle, key)).expect("held");
    }
    see(&mut held, tickets, "ARM-41", Observed::Tracker("Done"));
    see(&mut held, tickets, "ARM-42", Observed::Tracker("In Review"));
    see(&mut held, tickets, "ARM-43", Observed::Tracker("Awaiting QA"));
    see(&mut held, pull_request, "812", Observed::Forge(ForgeFact::Approved));
    see(&mut held, deploy, "Deploy", Observed::Watch(ReadingState::Running));

    // ARM-41 is closed. ARM-43 is in a column nobody mapped and ARM-44 has not been read, and
    // neither of them may be let go, so End lists both.
    assert_eq!(
        listed(&held),
        [
            row("ARM-42", "In review"),
            row("ARM-43", "Awaiting QA"),
            row("ARM-44", ""),
            row("PR #812", "Approved"),
            row("Deploy", "Rolling out"),
        ]
    );

    // What the agent is refused with names the ones in a known state, and neither of the other two.
    let all = flying(&held, &copy, &called);
    let said: Vec<String> = all.iter().filter(|one| one.holds).map(Flying::held).collect();
    assert_eq!(said, ["ARM-42 is In review", "PR #812 is Approved", "Deploy is Rolling out"]);

    see(&mut held, pull_request, "812", Observed::Forge(ForgeFact::Merged));
    see(&mut held, deploy, "Deploy", Observed::Watch(ReadingState::Passed));
    assert_eq!(listed(&held), [row("ARM-42", "In review"), row("ARM-43", "Awaiting QA"), row("ARM-44", "")], "a merged pull request and a deploy that passed are over");

    // A Job holding only a ticket nobody has read, or one in a column nobody mapped, is listed and holds nothing.
    let mut only = HeldInstances::new();
    only.take_on(taken_on(tickets, "ARM-44")).expect("held");
    only.take_on(taken_on(tickets, "ARM-43")).expect("held");
    see(&mut only, tickets, "ARM-43", Observed::Tracker("Awaiting QA"));
    assert_eq!(listed(&only), [row("ARM-44", ""), row("ARM-43", "Awaiting QA")]);
    assert!(flying(&only, &copy, &called).iter().all(|one| !one.holds), "nothing the agent could clear holds the Job");
}
