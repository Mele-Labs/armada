//! What each source says as a lifecycle reads it, what a Job's rows say it holds, and an instance
//! rebuilt from what was kept. None needs a Fleet, a forge or a tracker to run.

use std::collections::BTreeMap;

use adapter_traits::{
    FromOutside, IssueField, Mergeability, Omitted, PullRequestPageFacts, PullRequestStanding, ReadIssue, ReviewSaid,
    ReviewStanding, Reviewer, ReviewerKind, Said,
};
use core_model::{FleetFact, ForgeFact, HeldInstances, InstanceRef, Lifecycle, Observed, Standing, StateId, StepId};
use ipc::{Attachment, AttachmentState, Instant};
use store::KeptInstance;

use crate::lifecycles::holding::{read_instance, rebuilt, subject_of};
use crate::lifecycles::reading::{fleet_fact, forge_fact, tracker_word, JobSeen};
use crate::lifecycles::taking_on::{holds, landed, started_by, PullRequestHeld};

fn issue(closed: bool, status: Option<&str>) -> ReadIssue {
    let mut read = ReadIssue::bare("ARM-41".into(), FromOutside::verbatim("t"), FromOutside::verbatim("u"), closed);
    if let Some(status) = status {
        read.fields.push(IssueField { name: "Status".into(), values: vec![FromOutside::verbatim(status)] });
    }
    read
}

#[test]
fn a_tracker_word_is_the_status_it_names_and_else_its_own_open_or_closed() {
    assert_eq!(tracker_word(&issue(false, Some("Code Review"))), "Code Review");
    assert_eq!(tracker_word(&issue(true, Some(" Done "))), "Done", "a closed issue still has the status it names");
    assert_eq!(tracker_word(&issue(false, None)), "Open");
    assert_eq!(tracker_word(&issue(true, None)), "Closed");
    assert_eq!(tracker_word(&issue(false, Some("  "))), "Open", "a blank status names nothing");
}

fn page(review: ReviewStanding, reviewers: Vec<Reviewer>, conversation: Vec<Said>) -> PullRequestPageFacts {
    PullRequestPageFacts {
        base: "main".into(),
        author: Some("author".into()),
        created_at: None,
        additions: 0,
        deletions: 0,
        mergeable: Mergeability::Unknown,
        review,
        approvals: 0,
        conversation,
        checks: Vec::new(),
        files: Vec::new(),
        commits: Vec::new(),
        omitted: Omitted::default(),
        head: None,
        body: None,
        threads: Vec::new(),
        reviewers,
    }
}

fn reviewer(login: &str, verdict: Option<ReviewSaid>) -> Reviewer {
    Reviewer { login: login.into(), kind: ReviewerKind::Person, verdict, name: None, url: None, team: None }
}

fn comment(author: &str) -> Said {
    Said::Comment { author: author.into(), at: "t".into(), body: "b".into() }
}

#[test]
fn a_pull_request_shows_the_furthest_fact_that_holds_and_a_silent_forge_shows_none() {
    let open = |page: &PullRequestPageFacts| forge_fact(PullRequestStanding::Open, Some(page));
    let nobody = page(ReviewStanding::None, vec![], vec![]);
    assert_eq!(open(&nobody), Some(ForgeFact::Ready));
    let asked = page(ReviewStanding::Required, vec![reviewer("ana", None)], vec![]);
    assert_eq!(open(&asked), Some(ForgeFact::ReviewersOn));
    let looked = page(ReviewStanding::None, vec![reviewer("ana", Some(ReviewSaid::Commented))], vec![]);
    assert_eq!(open(&looked), Some(ForgeFact::InReview));
    let changes = page(ReviewStanding::ChangesRequested, vec![], vec![]);
    assert_eq!(open(&changes), Some(ForgeFact::InReview));
    let approved = page(ReviewStanding::Approved, vec![reviewer("ana", Some(ReviewSaid::Approved))], vec![comment("bo")]);
    assert_eq!(open(&approved), Some(ForgeFact::Approved), "approved outranks the reviewers still on it");

    assert_eq!(forge_fact(PullRequestStanding::Open, None), None, "silence says nothing");
    assert_eq!(forge_fact(PullRequestStanding::Draft, None), Some(ForgeFact::Draft));
    assert_eq!(forge_fact(PullRequestStanding::Merged, None), Some(ForgeFact::Merged));
    assert_eq!(forge_fact(PullRequestStanding::Closed, None), Some(ForgeFact::Closed));
}

#[test]
fn a_comment_is_a_look_only_when_somebody_but_the_author_left_it() {
    let own = page(ReviewStanding::None, vec![], vec![comment("author")]);
    assert_eq!(forge_fact(PullRequestStanding::Open, Some(&own)), Some(ForgeFact::Ready));
    let other = page(ReviewStanding::None, vec![], vec![comment("author"), comment("ana")]);
    assert_eq!(forge_fact(PullRequestStanding::Open, Some(&other)), Some(ForgeFact::InReview));
}

#[test]
fn the_fact_that_stands_for_a_job_is_the_furthest_one_its_lifecycle_has_a_state_for() {
    let job = Lifecycle::job();
    let seen = |landed, ended, opened| JobSeen { step: Some(StepId::new("plan")), opened_pull_request: opened, landed, ended };
    assert_eq!(fleet_fact(&job, &seen(false, false, false)), Some(FleetFact::Started));
    assert_eq!(fleet_fact(&job, &seen(false, false, true)), Some(FleetFact::Started), "the Job lifecycle has no state for a step or a pull request");
    assert_eq!(fleet_fact(&job, &seen(false, true, true)), Some(FleetFact::Ended));
    assert_eq!(fleet_fact(&job, &seen(true, true, true)), Some(FleetFact::Landed), "landing outranks the Job closing after it");
}

fn row(kind: &str, target: &str, state: AttachmentState, detail: &[(&str, &str)]) -> Attachment {
    Attachment {
        kind: kind.into(),
        manifest_id: None,
        target: target.into(),
        state,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect::<BTreeMap<_, _>>(),
        since: Instant::carried("t"),
        changed_at: Instant::carried("t"),
    }
}

#[test]
fn a_jobs_rows_name_its_tickets_and_the_pull_requests_each_is_for_and_a_row_given_back_is_not_taken() {
    let rows = [
        row("issue", "ARM-41", AttachmentState::Standing, &[("pull_requests", "812")]),
        row("issue", "ARM-42", AttachmentState::Standing, &[("pull_requests", "812,900")]),
        row("issue", "ARM-43", AttachmentState::GivenBack, &[("pull_requests", "812")]),
        row("pr", "812", AttachmentState::Standing, &[("state", "open")]),
        row("pr", "700", AttachmentState::GivenBack, &[]),
        row("branch", "armada/x", AttachmentState::Standing, &[]),
    ];
    let held = holds(&rows);
    assert_eq!(held.tickets, ["ARM-41", "ARM-42"]);
    assert_eq!(
        held.pull_requests,
        [PullRequestHeld { number: "812".into(), tickets: vec!["ARM-41".into(), "ARM-42".into()] }]
    );
    assert!(!landed(&rows));
    assert!(landed(&[row("pr", "812", AttachmentState::Spent, &[("state", "merged")])]));
    assert_eq!(started_by(&[row("started_by", "parent-1", AttachmentState::Spent, &[])]), Some("parent-1"));
    assert_eq!(started_by(&rows), None);
}

fn kept(lifecycle: &str, key: &str, links: &[(&str, &str)], seen: Option<(&str, &str)>) -> KeptInstance {
    KeptInstance {
        session_id: "s1".into(),
        lifecycle: lifecycle.into(),
        key: key.into(),
        links: links.iter().map(|(l, k)| (l.to_string(), k.to_string())).collect(),
        seen_source: seen.map(|one| one.0.to_string()).unwrap_or_default(),
        seen: seen.map(|one| one.1.to_string()).unwrap_or_default(),
        watch_id: String::new(),
        taken_on_at: "t0".into(),
        ended_at: String::new(),
        handed_to: String::new(),
    }
}

fn subject(held: &HeldInstances, lifecycle: &str, key: &str) -> InstanceRef {
    subject_of(held, lifecycle, key).expect("held")
}

#[test]
fn an_instance_rebuilt_from_what_was_kept_stands_where_it_did_and_a_second_read_records_nothing() {
    let lifecycles = Lifecycle::built_in();
    let rows = [
        kept("ticket", "ARM-41", &[], Some(("tracker", "Code Review"))),
        kept("pr", "812", &[("ticket", "ARM-41")], Some(("forge", "reviewers_on"))),
        kept("job", "child", &[("job", "parent")], None),
        kept("elsewhere", "x", &[], Some(("tracker", "Done"))),
    ];
    let mut held = rebuilt(&lifecycles, &rows);

    assert_eq!(held.all().len(), 3, "a thing of a lifecycle nobody resolves is left out");
    let pull = subject(&held, "pr", "812");
    assert_eq!(held.get(&pull).expect("held").standing(), &Standing::In(StateId::new("reviewers_on").expect("id")));
    assert_eq!(held.linked(&pull).len(), 1, "the pull request still knows its ticket");

    let again = read_instance(&mut held, &lifecycles, &pull, &Observed::Forge(ForgeFact::ReviewersOn), "t1");
    assert_eq!(again, None, "the state it was already in, after a restart, is not entered again");
    let ticket = subject(&held, "ticket", "ARM-41");
    assert_eq!(read_instance(&mut held, &lifecycles, &ticket, &Observed::Tracker("Code Review"), "t1"), None);
    let moved = read_instance(&mut held, &lifecycles, &ticket, &Observed::Tracker("Done"), "t2").expect("a new word");
    assert_eq!(
        (moved.moved.from, moved.moved.to, moved.seen.as_str(), moved.at.as_str()),
        (Standing::Unmapped("Code Review".into()), Standing::Unmapped("Done".into()), "Done", "t2"),
        "the ticket Armada carries lists no words, so the tracker's own are kept as said"
    );
    let child = subject(&held, "job", "child");
    let first = read_instance(&mut held, &lifecycles, &child, &Observed::Fleet(FleetFact::Started), "t3").expect("first read");
    assert_eq!(first.moved.from, Standing::Unread);
}
