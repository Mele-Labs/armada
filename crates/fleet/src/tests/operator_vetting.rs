//! A Job's plan, pull request or walk, read by the Operator before it reaches his Your turn, as the Job's own
//! frozen `reviews` say: Off queues it as it was, Review queues it with the findings, and Review and approve
//! approves a plan found clean and nothing else. One call for each item, none for docs and tests alone.

use std::sync::{Arc, Mutex};

use api::{HostedSessions, Projects, Sessions};
use ipc::{
    EditProject, NewProject, ProjectMember, ReviewsSet, SessionId, StartSession, ValuesSet, WaitingAct, WaitingActKind, WaitingInput,
};
use testkit::FakeWorkProduct;

use super::operator_raises::{journal, page, pause, queued, seated, waiting};
use super::session_host::{rig_fitted, Rig};
use crate::operator::{keep_the_raises, Answering, Calling};
use crate::CallFailed;

const CODE: &str = "diff --git a/crates/fleet/src/values.rs b/crates/fleet/src/values.rs\n--- a/crates/fleet/src/values.rs\n+++ b/crates/fleet/src/values.rs\n@@ -1 +1 @@\n-old\n+new\n";
const DOCS: &str = "diff --git a/docs/concepts/project.md b/docs/concepts/project.md\n--- a/docs/concepts/project.md\n+++ b/docs/concepts/project.md\n@@ -1 +1 @@\n-old\n+new\n";
const PLAIN: &str = "item: 1\nread: A plain review.\npick: Approve the plan\n\njournal: Reviewed one item.";

/// A call that answers a review of work with `vet` and the review of the lot with a plain read, and keeps what it was asked.
struct Smart {
    vet: Option<&'static str>,
    asked: Mutex<Vec<String>>,
}

impl Smart {
    fn answering(vet: Option<&'static str>) -> Arc<Smart> {
        Arc::new(Smart { vet, asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }

    /// How many calls were reviews of work.
    fn vets(&self) -> usize {
        self.asked().iter().filter(|question| question.contains("verdict:")).count()
    }
}

impl Calling for Smart {
    fn ask(&self, question: String) -> Answering<'_> {
        let vetting = question.contains("verdict:");
        self.asked.lock().expect("not poisoned").push(question);
        let reply = if vetting { self.vet } else { Some(PLAIN) };
        Box::pin(async move { reply.map(str::to_string).ok_or(CallFailed::SaidNothing) })
    }
}

fn a_rig(patch: &str) -> Rig {
    let patch = patch.to_string();
    rig_fitted(move |fittings| fittings.work = FakeWorkProduct::changed(&["crates/fleet/src/values.rs"]).showing(&patch))
}

/// The Operator seated on `call` and switched on, its raises kept.
async fn on(rig: &Rig, call: &Arc<Smart>) {
    assert!(rig.fleet.operator_seated_with(call.clone()));
    super::operator_sources::switched(rig, "features.operator", true).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
}

fn reviews(plans: &str, pull_requests: &str, walks: &str) -> ValuesSet {
    let word = |word: &str| Some(word.to_string());
    ValuesSet { reviews: Some(ReviewsSet { plans: word(plans), pull_requests: word(pull_requests), walks: word(walks) }), ..ValuesSet::default() }
}

async fn project(rig: &Rig, values: ValuesSet) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: Some(values) };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

/// A Job in `project`, running, holding a branch to read.
async fn job_in(rig: &Rig, project: &str) -> SessionId {
    let start = StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some("Build the pane".to_string()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.send(&id, "plan it").await;
    rig.stand_in.init(0);
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_branch = Some("job-branch".to_string());
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    id
}

async fn plan_recorded(rig: &Rig, id: &SessionId, seen: bool) {
    rig.fleet.record_plan(None, super::session_rail::plan(id), seen).await.expect("recorded");
}

async fn pull_request_waits(rig: &Rig, id: &SessionId) {
    let act = WaitingAct { kind: WaitingActKind::ApprovePr, target: "#12".into() };
    let item = WaitingInput { approves: false, id: "pr".into(), text: "Review #12".into(), act: Some(act), options: Vec::new(), steps: Vec::new() };
    waiting(rig, id, vec![item]).await;
}

async fn plan_approved(rig: &Rig, id: &SessionId) -> bool {
    rig.fleet.store().lock().await.session_plan(id.as_str()).expect("read").expect("a plan").approved_at.is_some()
}

#[tokio::test]
async fn a_plan_of_a_job_whose_reviews_are_off_is_queued_with_the_one_review_of_the_lot() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: clean\nread: Fine."));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("off", "off", "off")).await).await;
    plan_recorded(&rig, &job, false).await;

    let one = queued(&rig, &job, "plan:1").await;

    assert_eq!(one.read.map(|read| read.read), Some("A plain review.".to_string()));
    assert_eq!(call.vets(), 0, "no review of the work is made");
    assert!(!plan_approved(&rig, &job).await);
}

#[tokio::test]
async fn review_queues_a_plan_with_its_findings_after_one_call_and_approves_nothing() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: findings\nread: It leaves out the tests.\npick: Send the plan back"));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("review", "off", "off")).await).await;
    plan_recorded(&rig, &job, false).await;

    let one = queued(&rig, &job, "plan:1").await;

    let read = one.read.expect("a read");
    assert_eq!((read.read.as_str(), read.pick.as_str()), ("Findings: It leaves out the tests.", "Send the plan back"));
    assert_eq!((call.vets(), call.asked().len()), (1, 1), "one call, and it is the review of the plan");
    let asked = &call.asked()[0];
    assert!(asked.contains("You get a plan.") && asked.contains("plan it"), "{asked}");
    assert!(!plan_approved(&rig, &job).await, "Review approves nothing");
}

#[tokio::test]
async fn a_diff_of_docs_alone_is_not_read_and_its_item_is_queued_with_no_read() {
    let rig = a_rig(DOCS);
    let call = Smart::answering(Some("verdict: clean\nread: Fine."));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("off", "review", "off")).await).await;
    pull_request_waits(&rig, &job).await;

    let one = queued(&rig, &job, "pr").await;
    pause().await;

    assert_eq!(one.read, None);
    assert!(call.asked().is_empty(), "no call is made: {:?}", call.asked());
}

#[tokio::test]
async fn a_pull_request_is_read_as_its_diff_against_the_brief() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: findings\nread: It leaves a stray file."));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("off", "review", "off")).await).await;
    pull_request_waits(&rig, &job).await;

    let one = queued(&rig, &job, "pr").await;

    assert_eq!(one.read.map(|read| read.read), Some("Findings: It leaves a stray file.".to_string()));
    let asked = &call.asked()[0];
    assert!(asked.contains("+new") && asked.contains("plan it"), "{asked}");
}

#[tokio::test]
async fn review_and_approve_approves_a_plan_found_clean_and_the_log_names_the_setting() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: clean\nread: It matches the brief.\npick: Approve the plan"));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("review_and_approve", "off", "off")).await).await;
    plan_recorded(&rig, &job, false).await;

    super::session_host::eventually(|| async { plan_approved(&rig, &job).await }).await;
    pause().await;

    assert!(page(&rig).await.turn.is_empty(), "an approved plan is never queued");
    assert_eq!(call.vets(), 1);
    assert!(journal(&rig).await.contains("Approved by Review and approve, plans"), "{}", journal(&rig).await);
    let handled = page(&rig).await.digest.handled.acts;
    assert!(handled.iter().any(|one| one.outcome == "By a setting: Review and approve, plans"), "{handled:?}");
}

#[tokio::test]
async fn review_and_approve_queues_a_plan_with_findings_and_approves_nothing() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: findings\nread: It touches the wrong crate."));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("review_and_approve", "off", "off")).await).await;
    plan_recorded(&rig, &job, false).await;

    let one = queued(&rig, &job, "plan:1").await;
    pause().await;

    assert_eq!(one.read.map(|read| read.read), Some("Findings: It touches the wrong crate.".to_string()));
    assert!(!plan_approved(&rig, &job).await);
}

#[tokio::test]
async fn a_clean_pull_request_whose_plan_marks_work_he_would_see_is_queued_for_his_walk() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: clean\nread: It does what was asked."));
    on(&rig, &call).await;
    let job = job_in(&rig, &project(&rig, reviews("off", "review_and_approve", "off")).await).await;
    plan_recorded(&rig, &job, true).await;
    pull_request_waits(&rig, &job).await;

    let one = queued(&rig, &job, "pr").await;

    let read = one.read.expect("a read");
    assert!(read.read.contains("waits for his walk"), "{read:?}");
    assert_eq!(read.pick, "Walk it first");
}

#[tokio::test]
async fn a_setting_changed_after_the_job_started_does_not_change_how_its_work_is_read() {
    let rig = a_rig(CODE);
    let call = Smart::answering(Some("verdict: findings\nread: It leaves out the tests."));
    on(&rig, &call).await;
    let shop = project(&rig, reviews("off", "off", "off")).await;
    let before = job_in(&rig, &shop).await;
    rig.fleet.edit_project(shop.clone(), EditProject { values: Some(reviews("review", "off", "off")), ..Default::default() }).await.expect("edited");
    let after = job_in(&rig, &shop).await;
    plan_recorded(&rig, &before, false).await;
    plan_recorded(&rig, &after, false).await;

    let started_off = queued(&rig, &before, "plan:1").await;
    let started_on = queued(&rig, &after, "plan:1").await;

    assert_eq!(started_off.read.map(|read| read.read), Some("A plain review.".to_string()), "started when reviews were off");
    assert_eq!(started_on.read.map(|read| read.read), Some("Findings: It leaves out the tests.".to_string()));
    assert_eq!(call.vets(), 1);
}
