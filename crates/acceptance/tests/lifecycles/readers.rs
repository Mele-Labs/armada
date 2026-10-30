//! Fleet reads where an instance stands from the source that knows, and keeps a move only when the
//! standing changed. **A fake says what each source says**: the tracker, the forge, a watch and
//! Fleet's own events are asked through the traits Fleet asks them by, and nothing real is reached.

use adapter_traits::{
    Forge, FromOutside, IssueField, Mergeability, Omitted, PullRequestFacts, PullRequestPageFacts,
    PullRequestStanding, ReadIssue, ReviewStanding, Reviewer, ReviewerKind, Tracker,
};
use core_model::{HeldInstances, Lifecycle, Observed, Reading, ReadingState};
use fleet::lifecycles::holding::{read_instance, Transition};
use fleet::lifecycles::reading::{fleet_fact, forge_fact, tracker_word, JobSeen};
use testkit::{FakeTracker, FakeVcs};

use crate::bench::lifecycles::{at, in_state, taken_on, the_jobs_copy};

fn ticket(status: &str) -> ReadIssue {
    let mut read = ReadIssue::bare("ARM-41".into(), FromOutside::verbatim("Take the order"), FromOutside::verbatim("u"), false);
    read.fields.push(IssueField { name: "Status".into(), values: vec![FromOutside::verbatim(status)] });
    read
}

fn pull_request(standing: PullRequestStanding) -> PullRequestFacts {
    PullRequestFacts {
        standing,
        branch: "armada/take-the-order".into(),
        base: "main".into(),
        auto_merge: false,
        title: "Take the order".into(),
        url: "https://forge.invalid/my-store/pull/812".into(),
        body: None,
        closes: Vec::new(),
    }
}

fn page(review: ReviewStanding, asked: &[&str]) -> PullRequestPageFacts {
    let reviewer = |login: &str| Reviewer {
        login: login.into(),
        kind: ReviewerKind::Person,
        verdict: None,
        name: None,
        url: None,
        team: None,
    };
    PullRequestPageFacts {
        base: "main".into(),
        author: Some("author".into()),
        created_at: None,
        additions: 0,
        deletions: 0,
        mergeable: Mergeability::Unknown,
        review,
        approvals: 0,
        conversation: Vec::new(),
        checks: Vec::new(),
        files: Vec::new(),
        commits: Vec::new(),
        omitted: Omitted::default(),
        head: None,
        body: None,
        threads: Vec::new(),
        reviewers: asked.iter().map(|login| reviewer(login)).collect(),
    }
}

fn reading(state: ReadingState) -> Reading {
    Reading { state, finished: false, says: String::new(), link: None, parts: Vec::new() }
}

#[test]
fn fleet_reads_a_tickets_status_from_the_tracker_a_pull_requests_state_from_the_forge_a_deploys_from_a_watch_and_a_jobs_from_its_events() {
    let copy = the_jobs_copy();
    let (tickets, pull_requests, deploys, jobs) = (&copy[0], &copy[1], &copy[2], &copy[3]);
    let mut held = HeldInstances::new();
    held.take_on(taken_on(tickets, "ARM-41")).expect("held");
    held.take_on(taken_on(pull_requests, "812").linked_to(at(tickets, "ARM-41"))).expect("held");
    held.take_on(taken_on(deploys, "web")).expect("held");
    held.take_on(taken_on(jobs, "98632ca8").linked_to(at(jobs, "33b2b07b"))).expect("held");

    let tracker = FakeTracker::empty();
    let forge = FakeVcs::default();
    // One pass over the four sources, as the places Fleet reads them hand each reading on.
    let pass = |held: &mut HeldInstances, seen_by_the_watch: ReadingState, job: &JobSeen, when: &str| -> Vec<Transition> {
        let mut recorded = Vec::new();
        let mut hand = |lifecycle: &Lifecycle, key: &str, seen: Observed<'_>| {
            recorded.extend(read_instance(held, &copy, &at(lifecycle, key), &seen, when));
        };
        if let Ok(Some(issue)) = tracker.read_issue("/my-store", "ARM-41") {
            hand(tickets, "ARM-41", Observed::Tracker(&tracker_word(&issue)));
        }
        if let Some(facts) = forge.pull_request_facts("/my-store", "812") {
            let page = forge.pull_request_page("/my-store", "812");
            if let Some(fact) = forge_fact(facts.standing, page.as_ref()) {
                hand(pull_requests, "812", Observed::Forge(fact));
            }
        }
        hand(deploys, "web", Observed::Watch(reading(seen_by_the_watch).state));
        if let Some(fact) = fleet_fact(jobs, job) {
            hand(jobs, "98632ca8", Observed::Fleet(fact));
        }
        recorded
    };
    let running = JobSeen::default();

    // Nothing has been said yet by the tracker or the forge, so only the two Fleet knows move.
    let first = pass(&mut held, ReadingState::Running, &running, "t1");
    let who: Vec<_> = first.iter().map(|one| (one.instance.lifecycle().as_str(), one.moved.to.clone())).collect();
    assert_eq!(who, [("deploy", in_state("rolling_out")), ("job", in_state("running"))]);

    tracker.now_knows(ticket("Code Review"));
    forge.now_pull_request(Some(pull_request(PullRequestStanding::Open)));
    forge.now_pull_request_page(Some(page(ReviewStanding::Required, &["ana"])));
    let second = pass(&mut held, ReadingState::Running, &running, "t2");
    let who: Vec<_> = second.iter().map(|one| (one.instance.key().to_string(), one.moved.to.clone())).collect();
    assert_eq!(
        who,
        [("ARM-41".to_string(), in_state("in_review")), ("812".to_string(), in_state("reviewers_on"))],
        "my-store's tracker calls In review Code Review, and the forge shows reviewers asked"
    );
    assert_eq!(second[0].seen, "Code Review", "a move keeps the words that brought it");
    assert_eq!(second[1].at, "t2");

    // The same things seen again record nothing.
    assert!(pass(&mut held, ReadingState::Running, &running, "t3").is_empty());

    // A forge that will not show the page says nothing of an open pull request.
    forge.now_pull_request_page(None);
    assert!(pass(&mut held, ReadingState::Running, &running, "t4").is_empty());

    forge.now_pull_request_page(Some(page(ReviewStanding::Approved, &[])));
    forge.now_pull_request(Some(pull_request(PullRequestStanding::Merged)));
    let landed = JobSeen { landed: true, ended: true, ..JobSeen::default() };
    let last = pass(&mut held, ReadingState::Passed, &landed, "t5");
    let who: Vec<_> = last.iter().map(|one| (one.instance.lifecycle().as_str(), one.moved.to.clone())).collect();
    assert_eq!(
        who,
        [("pr", in_state("merged")), ("deploy", in_state("healthy")), ("job", in_state("landed"))],
        "a merge is seen at the forge, a rollout at its watch and a landing in Fleet's events"
    );
}
