//! The Operator's claim: **an arrival is ruled by one fresh call with no tools inside Fleet's
//! rules, the ruling is logged, and the arrival passes through untouched when the call fails or
//! runs late.** `armada-ehhu`, and the design page *Helm becomes the Operator*, revision 41.
//!
//! Its first case is the 3 Nov report: three live Jobs built overlapping answers to one problem.
//! They end as one Job, or as Jobs kept in touch, and not as three.
//!
//! Written before the code it names, and asserted in the order the owner meets it.
//!
//! | Proved | Not proved, and the group that carries it |
//! |---|---|
//! | Starts that arrive while a call is out are ruled together in the next call, one call at a time | That Fleet reads a stall from state: a Job it marked Stopped, and one that ended a turn after it was told its pull request failed, with no push. `fleet`'s `tests::operator_stalls` |
//! | Each ruling is in the Log, signed by the Operator: what arrived, from where, the outcome, the other Job and the shared paths, why, which call and when | That a stall arrives once however often its row is read, and that with the Operator Off no stall is ruled. `fleet`'s `tests::operator_stalls` |
//! | A failed call and a late one pass the start on untouched, the late one at the 20 seconds a first write waits, each with a Passed on row naming why | What a merge and a keep in touch say to each Job, and that a Job that has pushed is raised rather than merged. `fleet`'s `tests::operator_acting` |
//! | A ruling that comes back after its item passed on is not acted on and not logged twice | That Fleet finds the candidates from plan paths, files written and claims, and hands over dispatch, Jobs starting Jobs and plans. `fleet`'s `tests::operator_arrivals` |
//! | A Job stopped twice with tasks left is shown to the call with its plan, what is done and what is left, and is nudged with the next step the call names; one that left its pull request red is raised with the call's read | That the call is the Operator model's, kept started and holding no tools. `fleet`'s `tests::operator` |
//! | A Job nudged once is not offered a nudge again, and one with nothing left is raised with no call | What a nudge, a new Job for the rest and a raise say. `fleet`'s `tests::operator_stalls` |
//! | A question with a recommended option, from a Job on a plan group not seen, is answered as his press would, in a Log row naming the answer | That Fleet reads the seen mark per plan group, finds the person's earlier answer and settles the item as his press does, with a row in the Job's thread. `fleet`'s `tests::operator_answers` |
//! | The same question from a Job on a seen group, or with no plan, is raised with no call | That a question arrives once however often its row is read, and that with the Operator Off no question is handed. `fleet`'s `tests::operator_answers` |
//! | A second Job asking what a first already asked gets his earlier answer, naming the first Job | That Autopilot's switch widens what Answer may do, and that its pass leaves questions and cards to the Operator. `fleet`'s `tests::operator_answers` |
//! | With the Operator Off no question arrives, and one he answered while the call was out is not answered again | |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use std::collections::BTreeSet;
use std::time::Duration;

use core_model::Actor;
use fleet::operator::{Arrived, Earlier, Group, Logged, Operator, Outcome, PassedOn, Present, Quiet, Ruled, Source, Way, FIRST_WRITE_WAIT, NEEDS_YOU_WAIT};

use bench::operator::{asked, live, operator, stall, started, FakeCall, FakeFleet, Reply, ASKED, IN_TAB, PANES, PER_TAB, TABS, TERMINAL};

/// The first call: Tmux-like PR Review Tabs shares nothing with a live Job.
const TABS_RULED: &str = "item: 1\nway: 1\njob: none\nshared: none\nbecause: nothing live touches the review tabs";

/// The second call, holding the two that arrived while the first was out. Terminal Bugs opens
/// terminals in the tab host the first is building; the third is a group still in Terminal Bugs'
/// own plan.
fn both_ruled() -> String {
    format!(
        "item: 1\nway: 4\njob: {TABS}\nshared: {PANES}pane-host.tsx\nbecause: it opens terminals in the tab host the review tabs are building\n\n\
         item: 2\nway: 3\njob: {TERMINAL}\nshared: {PANES}terminal-pane.tsx\nbecause: the same group is still in Terminal Bugs' plan, so it would be built twice"
    )
}

async fn ruled_in_the_background(operator: &std::sync::Arc<Operator>, session: &str, source: Source) -> tokio::task::JoinHandle<Ruled> {
    let operator = operator.clone();
    let arrival = started(session, source);
    tokio::spawn(async move { operator.ruled(arrival, FIRST_WRITE_WAIT).await })
}

fn row<'a>(log: &'a [Logged], session: &str) -> &'a Logged {
    log.iter().find(|one| one.session_id == session).unwrap_or_else(|| panic!("a Log row for {session}"))
}

// ---------------------------------------------------------------------------
// Who does the work
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn the_three_jobs_of_the_3_nov_report_end_as_one_job_or_as_jobs_kept_in_touch_and_not_as_three() {
    let call = FakeCall::replying(vec![
        Reply::Says(TABS_RULED.into(), Duration::from_secs(5)),
        Reply::Says(both_ruled(), Duration::from_secs(3)),
    ]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    // A start never waits for its ruling: all three are live, and the second and third arrive
    // while the first one's call is out.
    let first = ruled_in_the_background(&operator, TABS, Source::Dispatch).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let second = ruled_in_the_background(&operator, TERMINAL, Source::Dispatch).await;
    let third = ruled_in_the_background(&operator, IN_TAB, Source::Job(TERMINAL.into())).await;
    let (first, second, third) = (first.await.expect("ruled"), second.await.expect("ruled"), third.await.expect("ruled"));

    let asked = call.asked();
    assert_eq!(asked.len(), 2, "one call out at a time, and what arrived meanwhile in the next one");
    assert!(asked[0].contains("1. A Job started") && !asked[0].contains("2. A Job started"), "the first alone");
    assert!(
        asked[1].contains("1. A Job started") && asked[1].contains("2. A Job started"),
        "the two that came while the first call was out, numbered side by side:\n{}",
        asked[1]
    );
    assert!(asked[1].contains(TABS), "the call is shown every live Job, the first among them");

    assert!(matches!(&first, Ruled::Ruled(ruling) if ruling.way == Way::NewJob), "{first:?}");
    match &second {
        Ruled::Ruled(ruling) => {
            assert_eq!((ruling.way, ruling.other.as_deref()), (Way::KeepInTouch, Some(TABS)));
            assert_eq!(ruling.shared, vec![format!("{PANES}pane-host.tsx")]);
        }
        other => panic!("kept in touch with the review tabs, not {other:?}"),
    }
    assert!(matches!(&third, Ruled::Ruled(ruling) if ruling.way == Way::Merge && ruling.other.as_deref() == Some(TERMINAL)), "{third:?}");

    // The Log: one row each, signed by the Operator, naming what it read.
    let log = fleet.log();
    assert_eq!(log.len(), 3, "{log:#?}");
    for one in &log {
        assert_eq!((one.by, one.arrived), (Actor::Operator, Arrived::Start));
        assert!(!one.because.trim().is_empty(), "every row says why: {one:?}");
        assert_eq!(one.at.as_str(), bench::operator::NOW);
    }
    let in_tab = row(&log, IN_TAB);
    assert_eq!(in_tab.source, Source::Job(TERMINAL.into()), "where it came from");
    assert_eq!(in_tab.subject, "Agent Opens Terminal in Tab", "what arrived");
    assert_eq!((in_tab.outcome, in_tab.other.as_deref()), (Outcome::Ruled(Way::Merge), Some(TERMINAL)));
    assert_eq!(in_tab.shared, vec![format!("{PANES}terminal-pane.tsx")]);
    assert!(in_tab.because.contains("built twice"));
    let terminal = row(&log, TERMINAL);
    assert_eq!((terminal.outcome, terminal.other.as_deref()), (Outcome::Ruled(Way::KeepInTouch), Some(TABS)));
    assert_eq!(terminal.call, in_tab.call, "ruled together, in one call");
    assert_ne!(terminal.call, row(&log, TABS).call);
    assert!(row(&log, TABS).other.is_none() && row(&log, TABS).shared.is_empty(), "a new Job names no other");

    // The end: the merge ends the Job that has done less, and the keep in touch tells both.
    assert_eq!(
        fleet.carrying_on(),
        BTreeSet::from([TABS.to_string(), TERMINAL.to_string()]),
        "the_end: Agent Opens Terminal in Tab ends into Terminal Bugs, whose plan already held it"
    );
    assert_eq!(
        fleet.in_touch(),
        vec![(TERMINAL.to_string(), TABS.to_string())],
        "the_end: Terminal Bugs and the review tabs carry on, each told of the other"
    );
}

// ---------------------------------------------------------------------------
// Stalls
// ---------------------------------------------------------------------------

const NEXT_STEP: &str = "Keep the scrollback when the pane redraws, in terminal-pane.tsx";

/// One call, holding both stalls: Terminal Bugs has a plain next step left, and the review tabs'
/// pull request fails on something only the person can settle.
fn stalls_ruled() -> String {
    format!(
        "item: 1\nway: 1\nstep: {NEXT_STEP}\nbecause: its plan is approved and the next task waits on nobody\n\n\
         item: 2\nway: 3\nstep: none\nbecause: the failing check needs a signing key only the person holds"
    )
}

#[tokio::test(start_paused = true)]
async fn a_job_stopped_twice_with_tasks_left_is_nudged_with_its_next_step_and_one_left_red_is_raised_with_its_read() {
    let call = FakeCall::replying(vec![Reply::Says(stalls_ruled(), Duration::from_secs(4))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);
    let stopped = stall(TERMINAL, Quiet::Stopped { at: "2026-11-03T15:20:00.000Z".into() }, &[NEXT_STEP, "Open terminals in a tab"], false);
    let red = stall(TABS, Quiet::Red { number: 3211, head: "4e5ff41acc".into(), failing: "notarize".into() }, &[], false);

    let (stopped, red) = tokio::join!(operator.ruled(stopped.arrival(), NEEDS_YOU_WAIT), operator.ruled(red.arrival(), NEEDS_YOU_WAIT));

    // The call reads each Job's plan and tasks against what is left, and is offered only the ways
    // a stall can go.
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "both stalls in one call");
    for part in [
        "1. A Job stopped with work left",
        "2. A Job left its pull request red",
        "Left: Keep the scrollback when the pane redraws, in terminal-pane.tsx; Open terminals in a tab",
        "Done: Find where the scrollback is dropped",
        "#3211 failed its checks at 4e5ff41",
        "The ways it is offered: 1. nudge; 2. new job; 3. raise",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }

    assert!(matches!(&stopped, Ruled::Ruled(ruling) if ruling.way == Way::Nudge && ruling.step.as_deref() == Some(NEXT_STEP)), "{stopped:?}");
    assert!(matches!(&red, Ruled::Ruled(ruling) if ruling.way == Way::Raise), "{red:?}");
    assert_eq!(fleet.nudged(), vec![(TERMINAL.to_string(), NEXT_STEP.to_string())], "told the next step the call named");
    assert_eq!(fleet.raised(), vec![(TABS.to_string(), "the failing check needs a signing key only the person holds".to_string())]);
    assert!(fleet.started_anew().is_empty() && fleet.ended_into().is_empty());

    // Each act is a row of the Log, signed by the Operator, as Fleet's.
    let log = fleet.log();
    assert_eq!(log.len(), 2, "{log:#?}");
    let (nudged, raised) = (row(&log, TERMINAL), row(&log, TABS));
    assert_eq!((nudged.arrived, nudged.outcome, nudged.by), (Arrived::Stopped, Outcome::Ruled(Way::Nudge), Actor::Operator));
    assert_eq!((raised.arrived, raised.outcome, raised.by), (Arrived::Red, Outcome::Ruled(Way::Raise), Actor::Operator));
    assert_eq!((nudged.source.clone(), nudged.subject.as_str()), (Source::Fleet, "Terminal Bugs"));
    assert_eq!(nudged.call, raised.call, "ruled together");
}

#[tokio::test(start_paused = true)]
async fn a_job_nudged_once_is_not_offered_a_nudge_again_and_one_with_nothing_left_is_raised_with_no_call() {
    let anew = "item: 1\nway: 1\nstep: none\nbecause: it stopped again after the Operator's nudge, on the same task";
    let call = FakeCall::replying(vec![Reply::Says(anew.into(), Duration::from_secs(2))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    let again = stall(TERMINAL, Quiet::Stopped { at: "2026-11-03T15:25:00.000Z".into() }, &[NEXT_STEP], true).arrival();
    assert_eq!(again.ways, vec![Way::NewJob, Way::Raise], "nudged once, never again");
    let ruled = operator.ruled(again, NEEDS_YOU_WAIT).await;
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::NewJob), "{ruled:?}");
    assert_eq!(fleet.started_anew(), vec![TERMINAL.to_string()], "the rest starts as a Job of its own");

    let empty = stall(TABS, Quiet::Stopped { at: "2026-11-03T15:26:00.000Z".into() }, &[], false).arrival();
    assert_eq!(empty.ways, vec![Way::Raise], "with nothing left, nothing to nudge or start");
    let ruled = operator.ruled(empty, NEEDS_YOU_WAIT).await;
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::Raise), "{ruled:?}");
    assert_eq!(call.asked().len(), 1, "a stall Fleet's rules leave one way is ruled with no call");
    assert_eq!(fleet.raised().len(), 1);
    assert_eq!(fleet.log().len(), 2);
}

// ---------------------------------------------------------------------------
// Answer
// ---------------------------------------------------------------------------

/// The group Terminal Bugs is working on, which changes nothing the person sees.
const NOT_SEEN: &str = "Keep the scrollback through a redraw";
/// The group of the review tabs he would see.
const SEEN: &str = "Tabs drawn like tmux";

#[tokio::test(start_paused = true)]
async fn a_question_with_a_recommended_option_from_a_job_on_a_group_not_seen_is_answered_as_his_press_would() {
    let answered = "item: 1\nway: 1\nanswer: 1\nbecause: the recommended option keeps what each tab shows, and nothing he sees changes";
    let call = FakeCall::replying(vec![Reply::Says(answered.into(), Duration::from_secs(3))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    let question = asked(TERMINAL, Group::Unseen(NOT_SEEN.into()), Present::Here, None).arrival().expect("the Operator is on");
    assert_eq!(question.arrived, Arrived::Question);
    assert_eq!(question.ways, vec![Way::Answer, Way::Raise], "Fleet's rules let it answer, and the call may still raise");
    let ruled = operator.ruled(question, NEEDS_YOU_WAIT).await;

    let asked = call.asked();
    assert_eq!(asked.len(), 1);
    for part in ["1. An agent asked a question", ASKED, "The answers it may give: 1. \"Keep it per tab (recommended)\"", "The ways it is offered: 1. answer; 2. raise"] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::Answer), "{ruled:?}");
    assert_eq!(fleet.answered(), vec![(TERMINAL.to_string(), "ask:1".to_string(), 0, PER_TAB.to_string())], "settled as his press on the first option");

    let log = fleet.log();
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].arrived, log[0].outcome, log[0].by), (Arrived::Question, Outcome::Ruled(Way::Answer), Actor::Operator));
    assert_eq!((log[0].subject.as_str(), log[0].answer.as_deref()), (ASKED, Some(PER_TAB)), "the row names the question and the answer");
    assert!(log[0].call.is_some());
}

#[tokio::test(start_paused = true)]
async fn the_same_question_from_a_job_on_a_seen_group_or_with_no_plan_is_raised_and_no_call_is_made() {
    let call = FakeCall::replying(Vec::new());
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    let seen = asked(TABS, Group::Seen(SEEN.into()), Present::Here, None).arrival().expect("the Operator is on");
    let unplanned = asked(IN_TAB, Group::NoPlan, Present::Here, None).arrival().expect("the Operator is on");
    assert_eq!((seen.ways.as_slice(), unplanned.ways.as_slice()), ([Way::Raise].as_slice(), [Way::Raise].as_slice()), "the call cannot answer either");
    let (seen, unplanned) = tokio::join!(operator.ruled(seen, NEEDS_YOU_WAIT), operator.ruled(unplanned, NEEDS_YOU_WAIT));

    assert!(matches!(&seen, Ruled::Ruled(ruling) if ruling.way == Way::Raise), "{seen:?}");
    assert!(matches!(&unplanned, Ruled::Ruled(ruling) if ruling.way == Way::Raise), "{unplanned:?}");
    assert!(call.asked().is_empty(), "Fleet makes no call for an item its rules can only raise");
    assert!(fleet.answered().is_empty());
    let log = fleet.log();
    assert_eq!(log.len(), 2, "{log:#?}");
    assert!(row(&log, TABS).because.contains(SEEN), "the row names the seen group: {}", row(&log, TABS).because);
    assert!(row(&log, IN_TAB).because.contains("no plan"), "and the missing plan: {}", row(&log, IN_TAB).because);
    assert!(log.iter().all(|one| one.call.is_none() && one.outcome == Outcome::Ruled(Way::Raise)));
}

#[tokio::test(start_paused = true)]
async fn a_second_job_asking_what_a_first_already_asked_gets_the_earlier_answer_naming_the_first_job() {
    let answered = "item: 1\nway: 1\nanswer: 1\nbecause: the person answered the same question for Terminal Bugs";
    let call = FakeCall::replying(vec![Reply::Says(answered.into(), Duration::from_secs(2))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    // He answered Terminal Bugs himself, against the recommendation; Agent Opens Terminal in Tab
    // has recorded no plan yet, and asks the same.
    let earlier = Earlier { session_id: TERMINAL.into(), title: "Terminal Bugs".into(), answer: "Share one scrollback".into() };
    let question = asked(IN_TAB, Group::NoPlan, Present::Here, Some(earlier)).arrival().expect("the Operator is on");
    assert_eq!(question.ways, vec![Way::Answer, Way::Raise]);
    assert_eq!(question.answers.len(), 1, "his earlier answer, and nothing else");
    assert_eq!((question.answers[0].at, question.answers[0].label.as_str()), (1, "Share one scrollback"));
    let ruled = operator.ruled(question, NEEDS_YOU_WAIT).await;

    assert!(call.asked()[0].contains("the person's answer when Job 01JOBTERMINALBUGS, \"Terminal Bugs\", asked the same"), "{}", call.asked()[0]);
    assert!(matches!(&ruled, Ruled::Ruled(ruling) if ruling.way == Way::Answer && ruling.other.as_deref() == Some(TERMINAL)), "{ruled:?}");
    assert_eq!(fleet.answered(), vec![(IN_TAB.to_string(), "ask:1".to_string(), 1, "Share one scrollback".to_string())]);
    let log = fleet.log();
    assert_eq!((log[0].other.as_deref(), log[0].answer.as_deref()), (Some(TERMINAL), Some("Share one scrollback")), "the row names the first Job");
}

#[tokio::test(start_paused = true)]
async fn with_the_operator_off_nothing_arrives_and_a_question_he_answered_first_is_not_answered_again() {
    assert!(asked(TERMINAL, Group::Unseen(NOT_SEEN.into()), Present::Off, None).arrival().is_none(), "Off is Fleet as it was: nothing is answered");

    let late = "item: 1\nway: 1\nanswer: 1\nbecause: the recommended option";
    let call = FakeCall::replying(vec![Reply::Says(late.into(), Duration::from_secs(5))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);
    let question = asked(TERMINAL, Group::Unseen(NOT_SEEN.into()), Present::Here, None).arrival().expect("the Operator is on");
    let ruled = tokio::spawn({
        let operator = operator.clone();
        async move { operator.ruled(question, NEEDS_YOU_WAIT).await }
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    fleet.answer_first();
    let ruled = ruled.await.expect("ruled");

    assert!(matches!(&ruled, Ruled::PassedOn(PassedOn::Settled(_))), "his answer wins: {ruled:?}");
    assert!(fleet.answered().is_empty(), "the ruling is dropped when it returns");
    assert_eq!(fleet.log()[0].outcome, Outcome::PassedOn);
}

// ---------------------------------------------------------------------------
// When it goes wrong
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn a_call_that_fails_passes_the_start_on_untouched_and_the_log_says_why() {
    let call = FakeCall::replying(vec![Reply::Fails(Duration::from_secs(2))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    let ruled = operator.ruled(started(IN_TAB, Source::Job(TERMINAL.into())), FIRST_WRITE_WAIT).await;

    assert!(matches!(&ruled, Ruled::PassedOn(PassedOn::Failed(_))), "{ruled:?}");
    let log = fleet.log();
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].outcome, log[0].by), (Outcome::PassedOn, Actor::Operator));
    assert!(log[0].because.contains("overloaded"), "the row names why: {}", log[0].because);
    assert!(log[0].call.is_some(), "a call was made, and counts against the day's");
    assert_eq!(fleet.carrying_on().len(), 3, "nothing was done to any Job");
    assert!(fleet.in_touch().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_late_call_passes_the_start_on_at_twenty_seconds_and_its_ruling_is_not_acted_on() {
    let late = format!("item: 1\nway: 3\njob: {TERMINAL}\nshared: {PANES}terminal-pane.tsx\nbecause: the same group");
    let call = FakeCall::replying(vec![Reply::Says(late, Duration::from_secs(30))]);
    let fleet = FakeFleet::with(live(), 100);
    let operator = operator(&call, &fleet);

    let asked_at = tokio::time::Instant::now();
    let ruled = operator.ruled(started(IN_TAB, Source::Job(TERMINAL.into())), FIRST_WRITE_WAIT).await;

    assert_eq!(asked_at.elapsed(), Duration::from_secs(20), "a Job's first write waits at most 20 seconds");
    assert!(matches!(&ruled, Ruled::PassedOn(PassedOn::Late(_))), "{ruled:?}");
    let log = fleet.log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].outcome, Outcome::PassedOn);
    assert!(log[0].because.contains("20 seconds"), "the row names why: {}", log[0].because);

    // The call comes back ten seconds later, naming a merge.
    tokio::time::sleep(Duration::from_secs(15)).await;
    assert_eq!(fleet.log().len(), 1, "the late ruling is not logged as well");
    assert!(fleet.ended_into().is_empty(), "nor acted on: the Job went on as it does today");
}
