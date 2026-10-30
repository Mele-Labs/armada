//! Lifecycles and rules' claim: **A Job in a Workspace (a set of repositories) moves its ticket
//! when a step starts, waits on its pull request's approval with no agent running, asks me before
//! it writes to the tracker or forge unless I set the rule to act, and lists what is still in
//! flight before it ends.** The design is "Workspaces, lifecycles and rules", walked and approved
//! on 29 Oct.
//!
//! Written before the code it names, and asserted in the order the owner meets it. **Workspace
//! here is a set of repositories**, and not the Manifest's monorepo unit; this file uses the word
//! for nothing but the id a lifecycle is scoped to.
//!
//! Each step of the claim not proved yet is a row below, naming the build task that carries it.
//! The task that builds a step adds its assertion beside the code and takes its row out.
//!
//! | Proved | Not proved, and the build task that carries it |
//! |---|---|
//! | A ticket reads in the words of the Workspace's tracker, a status no state lists is unmapped, and a step's move (To do to In progress) is one the lifecycle expects | |
//! | Fleet reads a ticket's status from the tracker, a pull request's state from the forge, a deploy's from a watch and a Job's from its own events, and keeps a move only when the standing changed (`lifecycles/readers.rs`) | |
//! | A step starting plans the ticket move: a rule read from its file, whose When is the step starting, whose If reads the tickets the Job holds and whose Do is the move, plans it once for each ticket still in To do, and under the default it is planned as Asks | |
//! | A step ends on `pr: approved` or `job: landed`, the agent is stopped while it waits, and Fleet wakes it on the transition; Land waits on `pr: approved` before it merges (`lifecycles/ends.rs`) | |
//! | Approval is a state of the pull request read from the forge alone, reaching it is reported once, it is not an end, and a pull request knows its tickets | |
//! | A Job a Job starts is an instance of the Job lifecycle, linked to the Job that started it, and landing ends it. Bridge and the agent's tools can see lifecycles, instances and rules: they cross the wire as the wire's own types, are served at four read routes with one agent tool for a Job's own, and are spelled the same in TypeScript (`lifecycles/wire.rs`) | |
//! | What is in flight is what no end state has taken, and a status nobody can place keeps its instance there; End lists each by its name and state before it ends the Job (`lifecycles/ending.rs`, and `fleet`'s `lifecycle_holding` for the refusal and the ending) | |
//! | A planned act is asked about in the owner's words while its autonomy is Asks and done at once when it is Acts, the repository's autonomy laid over the Workspace's; a watch a rule starts for a deploy takes the deploy on and its readings move it (`lifecycles/acts.rs`, and `fleet`'s `rule_asking` and `rule_acting` for the kept item, its answer and a restart) | |
//! | A planned write is made in the tracker, in the Workspace's own word for the state, or in the forge, a merge being the forge's own so a merge queue takes it; one the repository cannot make is kept as not carried, with why, and raises nothing (`lifecycles/writes.rs`, and `fleet`'s `rule_writing` for the Do it and Skip item and a write that fails) | |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

// The readers', the wait's, End's, the acts', the wire's and the writes' halves of the claim, each in a file of its own to keep this one short.
#[path = "lifecycles/readers.rs"] mod readers;
#[path = "lifecycles/ends.rs"] mod ends;
#[path = "lifecycles/ending.rs"] mod ending;
#[path = "lifecycles/acts.rs"] mod acts;
#[path = "lifecycles/wire.rs"] mod wire;
#[path = "lifecycles/writes.rs"] mod writes;

use std::path::Path;

use core_model::rule::{Act, ActKind, Autonomy, AutonomyDefaults, Course, RanOnce, Target, TrackerWrite};
use core_model::{FleetFact, ForgeFact, HeldInstances, Lifecycle, LifecycleId, LifecycleScope, MalformedLifecycle, Observed, ReadingState, ScopeId, Standing, StateRead};

use bench::lifecycles::{a_job_holding_three_tickets, at, in_state, keys, lifecycle, plan_step_start, rules_from, see, state, taken_on, the_jobs_copy, BACK_OFFICE_TICKETS, MOVES_TICKETS, MY_STORE_TICKETS};

// ---------------------------------------------------------------------------
// Moves its ticket
// ---------------------------------------------------------------------------

#[test]
fn a_ticket_reads_in_the_words_of_the_workspace_it_is_in_and_a_word_nobody_listed_is_unmapped() {
    let my_store = lifecycle(MY_STORE_TICKETS);
    let back_office = lifecycle(BACK_OFFICE_TICKETS);
    assert_eq!(my_store.scope(), &LifecycleScope::Within(ScopeId::new("my-store").expect("a scope")));

    for review in ["Code Review", "In Review"] {
        assert_eq!(my_store.standing_of(&Observed::Tracker(review)), in_state("in_review"), "{review}");
    }
    assert_eq!(my_store.standing_of(&Observed::Tracker("In Progress")), in_state("in_progress"));

    // The same column in the other Workspace goes by another name, and the first one's word means
    // nothing there.
    assert_eq!(back_office.standing_of(&Observed::Tracker("Peer Review")), in_state("in_review"));
    assert_eq!(
        back_office.standing_of(&Observed::Tracker("Code Review")),
        Standing::Unmapped("Code Review".into())
    );

    // A status no state lists is a fact of its own. It is neither rounded to the nearest state nor
    // dropped, so nothing is written to a tracker on a guess about its words.
    assert_eq!(
        my_store.standing_of(&Observed::Tracker("Awaiting QA")),
        Standing::Unmapped("Awaiting QA".into())
    );
    assert_eq!(
        my_store.standing_of(&Observed::Tracker("in review")),
        Standing::Unmapped("in review".into()),
        "a different case is a different word"
    );

    // What Armada carries lists no words at all, so before a Workspace says which are which every
    // status is unmapped.
    assert_eq!(
        Lifecycle::ticket().standing_of(&Observed::Tracker("In Progress")),
        Standing::Unmapped("In Progress".into())
    );

    // The move a step makes when it starts is one the lifecycle expects, and the word a write
    // would put the ticket into is the tracker's own.
    assert!(my_store.allows(&state("todo"), &state("in_progress")));
    assert!(!my_store.allows(&state("closed"), &state("todo")));
    let StateRead::Tracker(words) = my_store.state(&state("in_progress")).expect("a state").read() else {
        panic!("a ticket is read from the tracker");
    };
    assert_eq!(words.as_slice(), ["In Progress"]);
}

#[test]
fn a_step_starting_plans_the_ticket_move_and_under_the_default_it_is_planned_as_asks() {
    let rules = rules_from(&[MOVES_TICKETS]);
    assert!(rules.left_out().is_empty(), "{:?}", rules.left_out());
    let (rules, held) = (rules.frozen(), a_job_holding_three_tickets());

    let planned = plan_step_start("plan", "10:00", &rules, &AutonomyDefaults::new(), &held, &mut RanOnce::new());
    // The two tickets in To do move, under either of the tracker's names; the one in progress stays.
    assert_eq!(planned.iter().filter_map(|one| one.on()).map(|on| on.key()).collect::<Vec<_>>(), ["ARM-41", "ARM-42"]);
    let ticket = LifecycleId::new("ticket").expect("an id");
    let moves = Act::Tracker {
        write: TrackerWrite::Move { to: state("in_progress") },
        target: Target::held(ticket).standing_in(state("todo")),
    };
    assert!(planned.iter().all(|one| one.act() == &moves && one.act().kind() == ActKind::TrackerWrite));
    // A write to the tracker asks until someone says otherwise, and the move is the same when they do.
    assert!(planned.iter().all(|one| one.course() == Course::Asks));
    let acting = AutonomyDefaults::new().setting(ActKind::TrackerWrite, Autonomy::Acts);
    let planned = plan_step_start("plan", "10:00", &rules, &acting, &held, &mut RanOnce::new());
    assert!(planned.iter().all(|one| one.course() == Course::Acts && one.act() == &moves));
}

#[test]
fn a_rule_plans_nothing_until_it_is_saved_nothing_for_a_job_that_holds_nothing_and_nothing_twice() {
    let (rules, held) = (rules_from(&[MOVES_TICKETS]).frozen(), a_job_holding_three_tickets());
    let (defaults, mut ran) = (AutonomyDefaults::new(), RanOnce::new());
    let step = |step: &str, when: &str, rules: &[core_model::rule::Rule], held: &HeldInstances, ran: &mut RanOnce| {
        plan_step_start(step, when, rules, &defaults, held, ran).len()
    };
    assert_eq!(step("plan", "10:00", &rules_from(&[]).frozen(), &held, &mut RanOnce::new()), 0, "no rule file");
    assert_eq!(step("plan", "10:00", &rules, &HeldInstances::new(), &mut RanOnce::new()), 0, "a Job from before instances");
    assert_eq!(step("build", "10:00", &rules, &held, &mut RanOnce::new()), 0, "another step");
    assert_eq!(step("plan", "10:00", &rules, &held, &mut ran), 2);
    assert_eq!(step("plan", "10:00", &rules, &held, &mut ran), 0, "the same transition read again");
    let mut after_a_restart: RanOnce = ran.iter().map(|(rule, key)| (rule.clone(), key.clone())).collect();
    assert_eq!(step("plan", "10:00", &rules, &held, &mut after_a_restart), 0, "Fleet kept what ran, and the replay finds it");
    assert_eq!(step("plan", "10:20", &rules, &held, &mut after_a_restart), 2, "the step entered again is another transition");
}

// ---------------------------------------------------------------------------
// Waits on its pull request's approval
// ---------------------------------------------------------------------------

#[test]
fn a_pull_request_knows_its_tickets_and_reaches_approval_once_from_what_the_forge_shows() {
    let copy = the_jobs_copy();
    let (tickets, pull_request) = (&copy[0], &copy[1]);

    // Approval is a state the forge's review decides. The lifecycle has no way to be told it by
    // anyone else: what a state is read from is a forge fact, a tracker word, a watch or an event.
    assert_eq!(
        pull_request.state(&state("approved")).expect("a state").read(),
        &StateRead::Forge(ForgeFact::Approved)
    );

    let mut held = HeldInstances::new();
    held.take_on(taken_on(tickets, "ARM-41")).expect("held");
    held.take_on(taken_on(tickets, "ARM-42")).expect("held");
    held.take_on(
        taken_on(pull_request, "my-store#812").linked_to(at(tickets, "ARM-41")).linked_to(at(tickets, "ARM-42")),
    )
    .expect("held");
    see(&mut held, tickets, "ARM-41", Observed::Tracker("In Review"));
    see(&mut held, tickets, "ARM-42", Observed::Tracker("Code Review"));

    let pr = at(pull_request, "my-store#812");
    assert_eq!(keys(&held.linked(&pr)), ["ARM-41", "ARM-42"], "a pull request knows its tickets");
    assert!(
        held.linked(&pr).iter().all(|ticket| ticket.standing() == &in_state("in_review")),
        "a rule's condition reads across the links"
    );

    // Each fact the forge shows is a move once, in the order a pull request meets them. Showing the
    // same one again moves nothing, which is what lets the wait wake the Job once.
    let mut entered = Vec::new();
    for fact in [
        ForgeFact::Draft,
        ForgeFact::Ready,
        ForgeFact::ReviewersOn,
        ForgeFact::InReview,
        ForgeFact::Approved,
        ForgeFact::Approved,
    ] {
        let moved = held.get_mut(&pr).expect("held").observe(pull_request, &Observed::Forge(fact)).expect("its own");
        entered.push(moved.map(|moved| moved.to));
    }
    assert_eq!(
        entered,
        [
            Some(in_state("draft")),
            Some(in_state("ready")),
            Some(in_state("reviewers_on")),
            Some(in_state("in_review")),
            Some(in_state("approved")),
            None,
        ]
    );

    // Approved is where the Job waits and is not where the pull request ends: it holds the Job
    // open until it is merged.
    assert!(pull_request.allows(&state("in_review"), &state("approved")));
    assert!(!pull_request.is_over(held.get(&pr).expect("held").standing()));
    see(&mut held, pull_request, "my-store#812", Observed::Forge(ForgeFact::Merged));
    assert!(pull_request.is_over(held.get(&pr).expect("held").standing()));
}

#[test]
fn a_job_a_job_starts_is_an_instance_linked_to_the_job_that_started_it() {
    let job = Lifecycle::job();
    let mut held = HeldInstances::new();
    let starter = at(&job, "33b2b07b");
    held.take_on(taken_on(&job, "98632ca8").linked_to(starter.clone())).expect("held");
    held.take_on(taken_on(&job, "b58bf253").linked_to(starter.clone())).expect("held");
    see(&mut held, &job, "98632ca8", Observed::Fleet(FleetFact::Started));
    see(&mut held, &job, "b58bf253", Observed::Fleet(FleetFact::Started));

    // Every Job it started, which is what an Oversee step would wait on.
    assert_eq!(keys(&held.linking(&starter)), ["98632ca8", "b58bf253"]);
    assert!(held.linking(&starter).iter().all(|one| !job.is_over(one.standing())));

    see(&mut held, &job, "98632ca8", Observed::Fleet(FleetFact::Landed));
    let landed: Vec<_> = held.linking(&starter).into_iter().filter(|one| job.is_over(one.standing())).collect();
    assert_eq!(keys(&landed), ["98632ca8"]);
    assert_eq!(landed[0].standing(), &in_state("landed"));
}

// ---------------------------------------------------------------------------
// Lists what is still in flight
// ---------------------------------------------------------------------------

#[test]
fn what_is_in_flight_is_what_no_end_state_has_taken_and_a_status_nobody_can_place_stays_there() {
    let copy = the_jobs_copy();
    let (tickets, pull_request, deploy) = (&copy[0], &copy[1], &copy[2]);
    let mut held = HeldInstances::new();
    for (lifecycle, key) in [
        (tickets, "ARM-41"),
        (tickets, "ARM-42"),
        (tickets, "ARM-43"),
        (pull_request, "my-store#812"),
        (deploy, "prod"),
    ] {
        held.take_on(taken_on(lifecycle, key)).expect("held");
    }
    see(&mut held, tickets, "ARM-41", Observed::Tracker("Done"));
    see(&mut held, tickets, "ARM-42", Observed::Tracker("Awaiting QA"));
    see(&mut held, pull_request, "my-store#812", Observed::Forge(ForgeFact::Merged));
    see(&mut held, deploy, "prod", Observed::Watch(ReadingState::Running));

    let flying = |held: &HeldInstances| -> Vec<String> {
        held.in_flight(&copy).iter().map(|one| one.subject().to_string()).collect()
    };
    // ARM-41 is closed and the pull request merged. ARM-42 is somewhere nobody has a name for,
    // ARM-43 has not been read, and the deploy is rolling out: none of the three is over.
    assert_eq!(flying(&held), ["ticket ARM-42", "ticket ARM-43", "deploy prod"]);

    see(&mut held, deploy, "prod", Observed::Watch(ReadingState::Passed));
    assert_eq!(flying(&held), ["ticket ARM-42", "ticket ARM-43"]);

    see(&mut held, tickets, "ARM-42", Observed::Tracker("Done"));
    see(&mut held, tickets, "ARM-43", Observed::Tracker("Done"));
    assert!(flying(&held).is_empty(), "nothing is left to hold the Job open");
}

// ---------------------------------------------------------------------------
// What a file may not say
// ---------------------------------------------------------------------------

#[test]
fn a_lifecycle_file_that_contradicts_itself_is_refused_before_a_job_can_hold_anything_of_it() {
    let text = |states: &str, rest: &str| {
        format!("version: 1\nlifecycle_id: release\nname: Release\nstates:\n{states}{rest}")
    };
    let read = |text: String| config::parse_lifecycle(Path::new("release.yml"), &text);
    let shipped = "  - { id: shipped, label: Shipped, read: { tracker: [Done] } }\n";

    // One status in two states would be a ticket in two places.
    let twice = format!("{shipped}  - {{ id: live, label: Live, read: {{ tracker: [Done] }} }}\n");
    match read(text(&twice, "ends: [live]\n")) {
        Err(config::LifecycleError::Malformed { why: MalformedLifecycle::WordClaimedTwice { word, .. }, .. }) => {
            assert_eq!(word, "Done")
        }
        other => panic!("wanted the lifecycle refused for a word in two states, got {other:?}"),
    }

    // A move to a state it lacks, and an end it lacks, are refused whole.
    assert!(matches!(
        read(text(shipped, "transitions:\n  - { from: shipped, to: live }\nends: [shipped]\n")),
        Err(config::LifecycleError::Malformed { why: MalformedLifecycle::TransitionNamesNoState { .. }, .. })
    ));
    assert!(matches!(
        read(text(shipped, "ends: [live]\n")),
        Err(config::LifecycleError::Malformed { why: MalformedLifecycle::EndNamesNoState { .. }, .. })
    ));

    // A source that is not one of the four, and an id that would not survive `pr: approved`, name
    // their keys.
    let refused = read(text("  - { id: 'pr: approved', label: A, read: { agent: said so } }\n", "ends: [a]\n"))
        .expect_err("not a lifecycle")
        .to_string();
    assert!(refused.contains("`states[0].id`"), "{refused}");
    assert!(refused.contains("`states[0].read.agent`"), "{refused}");
}
