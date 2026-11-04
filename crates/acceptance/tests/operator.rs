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
//! | Starts that arrive while a call is out are ruled together in the next call, one call at a time | That Fleet hands its own arrivals over: dispatch, Jobs starting Jobs, plans, stalls. *Who does the work*, *Stalls* |
//! | Each ruling is in the Log, signed by the Operator: what arrived, from where, the outcome, the other Job and the shared paths, why, which call and when | That a merge ends the Job that has done less and a keep in touch tells both Jobs. *Who does the work*: the two `the_end` assertions below are red until it lands |
//! | A failed call and a late one pass the start on untouched, the late one at the 20 seconds a first write waits, each with a Passed on row naming why | That Fleet finds the candidates from plan paths, files written and claims. *Who does the work* |
//! | A ruling that comes back after its item passed on is not acted on and not logged twice | That the call is the Operator model's, kept started and holding no tools. `fleet`'s `tests::operator` |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use std::collections::BTreeSet;
use std::time::Duration;

use core_model::Actor;
use fleet::operator::{Arrived, Logged, Operator, Outcome, PassedOn, Ruled, Source, Way, FIRST_WRITE_WAIT};

use bench::operator::{live, operator, started, FakeCall, FakeFleet, Reply, IN_TAB, PANES, TABS, TERMINAL};

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

    // The end. Red until *Who does the work* acts on a ruling.
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
