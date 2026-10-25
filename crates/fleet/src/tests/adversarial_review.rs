//! An adversarial review of a Job's plan through a real Fleet and the judge test double: what the call is
//! shown and where it reads, the review landing on the Job's plan, kept per version, and the person's two
//! acts on a finding. `docs/concepts/adversarial-review.md`; the pure half is
//! `crates/acceptance/tests/adversarial_review.rs`.

use std::sync::Arc;
use std::time::Duration;

use api::{AdversarialReviews, HostedSessions, Next, Refusal};
use ipc::{
    AdversarialReview, AdversarialReviewState, DismissFinding, Event, FindingAnchor, FindingHow, RecordPlan,
    ReviewSubject, SendFinding, SessionId, StartAdversarialReview,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use super::session_plan::{group, plan, recorded};

/// Every brief opens with it, so the double answers this call and no other.
const ASKED: &str = "You are reviewing a plan";
const REQUEST: &str = "Rotate the keys every night, and never leave two rotations on one day.";
const LATER: &str = "Keep the previous key readable for a day.";
const AGENT_SAID: &str = "I checked everything and the lock is already there.";

const KEYS_RS: &str = "crates/store/src/keys.rs";
const KEYS: &str = "impl Keys {\n    pub fn rotate(&mut self, day: Day) -> Result<(), Error> {\n        self.insert(day)\n    }\n}\n";

const FINDINGS: &str = r#"{"findings": [
    {"about": "Write the rotation behind a lock", "consequence": "Two rotations can land on one day.",
     "expected": "The rotation takes a lock the store holds.", "found": "The store has no lock.",
     "cites": ["crates/store/src/keys.rs:2"]},
    {"about": "Keep the keys", "consequence": "The new table ships without a migration.",
     "expected": "A migration file beside the table.", "found": "No task writes one.", "cites": []},
    {"about": "", "consequence": "Yesterday's key is unreadable after a rotation.",
     "expected": "The old key stays readable for a day.", "found": "keys.rs reads \"pub fn rotate mut self\" and keeps one key.",
     "cites": ["crates/store/src/keys.rs"]}
]}"#;

fn answered_with(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::answering(&[(ASKED, answer)]).costing(0.4));
    let held = Arc::clone(&judge);
    let rig = rig_fitted(|fittings| fittings.judge = held);
    let file = std::path::Path::new(&rig.root).join(KEYS_RS);
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
    std::fs::write(file, KEYS).expect("written");
    (rig, judge)
}

fn the_plan(id: &SessionId) -> RecordPlan {
    let mut keys = group(None, "Keep the keys", "rust-engineer", &["crates/store"], &[
        (None, "Add a key_rotations table with a unique day"),
        (None, "Write the rotation behind a lock"),
    ]);
    keys.gives = Some("Old keys stay readable for a day".into());
    plan(id, vec![keys, group(None, "Run it nightly", "rust-engineer", &["crates/fleet"], &[(None, "Schedule the rotation at midnight")])])
}

/// A Job the person asked something of, whose agent answered, who said more, and which recorded a plan.
async fn a_job_with_a_plan(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, REQUEST).await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, AGENT_SAID);
    eventually(|| async { rig.rows(&id).await.iter().any(|row| format!("{row:?}").contains(AGENT_SAID)) }).await;
    rig.send(&id, LATER).await;
    recorded(rig, the_plan(&id)).await;
    id
}

fn subject(id: &SessionId, version: u32) -> ReviewSubject {
    ReviewSubject::Plan { session_id: id.clone(), version }
}

async fn start(rig: &Rig, id: &SessionId, version: u32, again: bool) -> Result<AdversarialReview, Refusal> {
    Arc::clone(&rig.fleet).start_adversarial_review(StartAdversarialReview { subject: subject(id, version), again }).await
}

async fn on_the_plan(rig: &Rig, id: &SessionId) -> Option<AdversarialReview> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.plan.and_then(|plan| plan.adversarial)
}

/// The review on the Job's plan once it has ended.
async fn ended(rig: &Rig, id: &SessionId) -> AdversarialReview {
    eventually(|| async { on_the_plan(rig, id).await.is_some_and(|review| review.state != AdversarialReviewState::Running) }).await;
    on_the_plan(rig, id).await.expect("a review")
}

fn asked_of(judge: &FakeJudge) -> Vec<(String, Option<String>)> {
    judge.asked().into_iter().zip(judge.directories()).filter(|(asked, _)| asked.starts_with(ASKED)).collect()
}

fn told(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

fn code(refusal: Refusal) -> String {
    refusal.error().code.clone()
}

#[tokio::test]
async fn a_review_of_the_plan_answers_running_and_lands_done_with_each_finding_pinned() {
    let (rig, judge) = answered_with(FINDINGS);
    let id = a_job_with_a_plan(&rig).await;
    let mut watching = rig.fleet.events().subscribe();

    let started = start(&rig, &id, 1, false).await.expect("started");
    assert_eq!((started.state, started.subject.clone(), started.findings.len()), (AdversarialReviewState::Running, subject(&id, 1), 0));
    let review = ended(&rig, &id).await;

    assert_eq!(review.state, AdversarialReviewState::Done, "{:?}", review.failed);
    let pinned: Vec<&FindingAnchor> = review.findings.iter().map(|one| &one.about).collect();
    assert_eq!(pinned, [&FindingAnchor::Task { id: "t2".into() }, &FindingAnchor::Group { id: "g1".into() }, &FindingAnchor::Plan]);
    assert_eq!(review.cost_micros, Some(400_000), "what the vendor billed is kept with the review");
    assert!(review.finished_at.is_some() && review.failed.is_none());

    // One call, on models.review, in the checkout Fleet serves: shown the person's words and the plan's,
    // never the agent's, and no id.
    let asked = asked_of(&judge);
    assert_eq!(asked.len(), 1);
    let (brief, directory) = &asked[0];
    for shown in [REQUEST, LATER, "Write the rotation behind a lock", "Keep the keys", "Old keys stay readable for a day"] {
        assert!(brief.contains(shown), "the brief lacks {shown:?}:\n{brief}");
    }
    assert!(!brief.contains(AGENT_SAID), "the agent's words are withheld:\n{brief}");
    assert!(!brief.contains("t2") && !brief.contains("g1"), "no id is shown:\n{brief}");
    assert_eq!(directory.as_deref(), Some(rig.root.as_str()));
    assert_eq!(judge.models().last().map(String::as_str), Some("a-model"), "models.review");

    // The Job changed on the start and on the finish.
    let mut states = Vec::new();
    while states.last() != Some(&AdversarialReviewState::Done) {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::SessionChanged(record) = delivered.event {
                    if let Some(review) = record.plan.and_then(|plan| plan.adversarial).filter(|_| record.id == id) {
                        if states.last() != Some(&review.state) {
                            states.push(review.state);
                        }
                    }
                }
            }
            _ => panic!("the stream ended"),
        }
    }
    assert_eq!(states, [AdversarialReviewState::Running, AdversarialReviewState::Done]);

    // A kept review is answered as it stands; only again runs another call.
    assert_eq!(start(&rig, &id, 1, false).await.expect("kept"), review);
    assert_eq!(asked_of(&judge).len(), 1, "no second call");
    let again = start(&rig, &id, 1, true).await.expect("again");
    assert_eq!(again.state, AdversarialReviewState::Running);
    assert_eq!(ended(&rig, &id).await.findings.len(), 3);
    assert_eq!(asked_of(&judge).len(), 2);
}

#[tokio::test]
async fn a_start_is_refused_for_no_job_no_plan_or_a_version_that_is_not_current() {
    let (rig, _) = answered_with(FINDINGS);
    let nobody = SessionId::carried("no-such-job");
    assert_eq!(code(start(&rig, &nobody, 1, false).await.expect_err("no job")), "fleet.review_subject_unknown");

    let id = rig.start().await;
    assert_eq!(code(start(&rig, &id, 1, false).await.expect_err("no plan")), "fleet.review_subject_unknown");

    recorded(&rig, the_plan(&id)).await;
    assert_eq!(code(start(&rig, &id, 2, false).await.expect_err("not recorded yet")), "fleet.review_subject_stale");
}

#[tokio::test]
async fn an_answer_quoting_words_it_was_never_shown_fails_the_review_and_keeps_nothing() {
    let invented = FINDINGS.replace(r#"\"pub fn rotate mut self\""#, r#"\"the scheduler takes the lock first\""#);
    let (rig, _) = answered_with(&invented);
    let id = a_job_with_a_plan(&rig).await;

    start(&rig, &id, 1, false).await.expect("started");
    let review = ended(&rig, &id).await;
    assert_eq!(review.state, AdversarialReviewState::Failed);
    assert!(review.findings.is_empty(), "nothing is half kept");
    let failed = review.failed.expect("Fleet's words");
    assert!(failed.contains("the scheduler takes the lock first"), "{failed}");
    assert_eq!(review.cost_micros, Some(400_000), "what was spent is kept on a failed review too");
}

#[tokio::test]
async fn a_finding_is_sent_as_the_persons_message_or_dismissed_even_once_the_plan_moved_on() {
    let (rig, _) = answered_with(FINDINGS);
    let id = a_job_with_a_plan(&rig).await;
    start(&rig, &id, 1, false).await.expect("started");
    ended(&rig, &id).await;
    let act = |finding: &str| SendFinding { subject: subject(&id, 1), finding_id: finding.into() };

    let sent = Arc::clone(&rig.fleet).send_finding(act("f1")).await.expect("sent");
    assert_eq!(sent.findings[0].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Sent));
    eventually(|| async { told(&rig, "Two rotations can land on one day.") == 1 }).await;
    assert_eq!(told(&rig, "Write the rotation behind a lock"), 1, "the message names the task by its words");
    assert_eq!(told(&rig, "crates/store/src/keys.rs:2"), 1, "and what it cites");

    let dismissed = Arc::clone(&rig.fleet)
        .dismiss_finding(DismissFinding { subject: subject(&id, 1), finding_id: "f2".into() })
        .await
        .expect("dismissed");
    assert_eq!(dismissed.findings[1].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Dismissed));
    assert_eq!(told(&rig, "The new table ships without a migration."), 0, "the agent is told nothing");

    // Settled is answered as it stands, with nothing sent twice.
    let again = Arc::clone(&rig.fleet).send_finding(act("f2")).await.expect("as it stands");
    assert_eq!(again.findings[1].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Dismissed));
    assert_eq!(code(Arc::clone(&rig.fleet).send_finding(act("f9")).await.expect_err("no such finding")), "fleet.finding_unknown");

    // Recorded again: the review stays on the plan as it read version one, and still settles.
    let mut moved = the_plan(&id);
    moved.groups[1].tasks[0].title = "Schedule the rotation at one".into();
    let two = recorded(&rig, moved).await.plan.expect("a plan");
    assert_eq!((two.version, two.adversarial.as_ref().map(|review| review.subject.clone())), (2, Some(subject(&id, 1))));
    let late = Arc::clone(&rig.fleet).send_finding(act("f3")).await.expect("an out-of-date review still sends");
    assert_eq!(late.findings[2].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Sent));
    eventually(|| async { told(&rig, "the plan as a whole") == 1 }).await;
    assert_eq!(code(start(&rig, &id, 1, true).await.expect_err("an old version")), "fleet.review_subject_stale");
}
