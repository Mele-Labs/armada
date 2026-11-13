//! Moving running Jobs to a workflow's new version: **a publish pins each live Job it would touch to the text that Job was running, and
//! the person moves the Jobs he picks, each staying on its step by id or going to the next that survives.**
//! `docs/practices/protocol/protocol-23-97-moving-running-jobs-to-a-new-workflow-version.md`.
//!
//! The Jobs are real hosted Jobs on a Fleet whose folders are `Planted`, which keeps what a save is asked to write; a publish reads the
//! folders before its stale check and again after the save, so the rig answers both reads.

use std::sync::Arc;

use api::{Authoring, HostedSessions};
use ipc::{
    AffectedJob, ChangeSessionSteps, MigrateJobs, MigrateResult, PublishWorkflowDraft, PutWorkflowDraft, SessionId, StepChange, WorkflowId,
    WorkflowPublished,
};

use super::repositories::Planted;
use super::session_host::{eventually, rig_fitted, Rig};
use super::session_rail::{record, REQUEST};
use super::workflow_page::{at, carried_id, read, staged_in_repository, STAGED};
use crate::rail;
use crate::repositories::{Locating, SavedWorkflow};

const MOVED: &str = "moved this Job to the newest staged workflow";

/// `STAGED` with a step in front of Propose.
fn with_triage() -> String {
    STAGED.replace(
        "steps:\n  - id: propose",
        "steps:\n  - id: triage\n    label: Triage\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    advance_gate: auto\n  - id: propose",
    )
}

/// `STAGED` without the Propose step.
fn without_propose() -> String {
    let (from, to) = (STAGED.find("  - id: propose").expect("a step"), STAGED.find("  - id: plan").expect("a step"));
    format!("{}{}", &STAGED[..from], &STAGED[to..])
}

struct Migrating {
    rig: Rig,
    planted: Arc<Planted>,
}

/// A Fleet holding `STAGED` in the repository.
fn migrating() -> Migrating {
    let planted = Arc::new(Planted::nothing());
    let rig = rig_fitted(|fittings| fittings.locating = Arc::clone(&planted) as Arc<dyn Locating>);
    planted.will_read(read(staged_in_repository()));
    rig.fleet.workflows_changed(&rig.root);
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    Migrating { rig, planted }
}

fn staged() -> WorkflowId {
    WorkflowId::carried("staged")
}

fn ids(steps: Option<Vec<rail::RailStep>>) -> Vec<String> {
    steps.map(|steps| rail::step_ids(&steps)).expect("the steps resolve")
}

fn holding(text: &str) -> super::super::repositories::Catalogued {
    read(vec![config::Written::in_repository(at("repo", "staged.json"), text.to_string())])
}

impl Migrating {
    /// A live Job on its first step, following `staged`, whose process has run.
    async fn job(&self) -> SessionId {
        let rig = &self.rig;
        let id = rig.start().await;
        rig.send(&id, REQUEST).await;
        rig.stand_in.init(0);
        Arc::clone(&rig.fleet)
            .set_session_workflow(ipc::SetSessionWorkflow { session_id: id.clone(), workflow: Some("staged".into()) })
            .await
            .expect("set");
        eventually(|| async { !record(rig, &id).await.workflow_visits.is_empty() }).await;
        id
    }

    async fn put(&self, source: &str, text: &str) {
        let put = PutWorkflowDraft { workflow_id: staged(), source: source.into(), text: text.into(), ..Default::default() };
        self.rig.fleet.put_workflow_draft(put, None).await.expect("kept");
    }

    /// Publish the draft of `staged` in the repository, which holds `was`, as the folders then hold `text`.
    async fn publish(&self, was: &str, text: &str) -> WorkflowPublished {
        self.planted.will_save(Ok(SavedWorkflow { id: carried_id("staged"), file: "/repo/.armada/workflows/staged.json".into(), replaced: true }));
        self.planted.will_read(holding(was));
        self.planted.will_read_next(holding(text));
        let publish = PublishWorkflowDraft { workflow_id: staged(), source: "repository".into(), ..Default::default() };
        self.rig.fleet.publish_workflow_draft(publish, None).await.expect("published")
    }

    async fn drafted_and_published(&self, was: &str, text: &str) -> WorkflowPublished {
        self.put("repository", text).await;
        self.publish(was, text).await
    }

    async fn affected(&self) -> Vec<AffectedJob> {
        Arc::clone(&self.rig.fleet).get_affected_jobs(staged(), "repository".into(), None, None).await.expect("listed").jobs
    }

    async fn migrate(&self, ids: &[&SessionId]) -> Vec<MigrateResult> {
        let migrate = MigrateJobs {
            workflow_id: staged(),
            source: "repository".into(),
            project: None,
            session_ids: ids.iter().map(|id| (*id).clone()).collect(),
        };
        Arc::clone(&self.rig.fleet).migrate_jobs(migrate, None).await.expect("answered").results
    }

    fn steps_published(&self) -> Vec<String> {
        ids(self.rig.fleet.rail_steps(self.rig.manifest.as_str(), "staged"))
    }

    async fn steps_read_by(&self, id: &SessionId) -> Vec<String> {
        ids(self.rig.fleet.job_rail_steps(id.as_str(), self.rig.manifest.as_str(), "staged").await)
    }

    async fn own_copy(&self, id: &SessionId) -> Option<store::KeptWorkflowCopy> {
        self.rig.fleet.store().lock().await.job_workflow_copy(id.as_str()).expect("reads")
    }

    fn told(&self, id: &SessionId) -> usize {
        let process = self.rig.stand_in.process_of(id.as_str()).expect("a process");
        let sent = process.sent.lock().unwrap();
        sent.iter().filter(|line| line.contains(MOVED)).count()
    }

    async fn timeline_says(&self) -> Vec<String> {
        self.rig.fleet.timeline().settled().await;
        self.rig
            .fleet
            .fleet_timeline(ipc::TimelineAsk::default())
            .await
            .expect("a timeline")
            .events
            .into_iter()
            .filter(|one| one.kind.as_wire() == "session.workflow")
            .map(|one| one.says)
            .collect()
    }
}

const OLD: [&str; 4] = ["propose", "plan", "walk", "deliver"];

/// **A publish moves no Job**: the Job keeps reading the steps it was running while the repository serves the new ones, and the publish names it.
#[tokio::test]
async fn publishing_a_repository_draft_pins_a_live_job_that_follows_it_and_the_job_keeps_its_old_steps() {
    let rig = migrating();
    let id = rig.job().await;
    assert_eq!(record(&rig.rig, &id).await.workflow_step, Some(0));

    let published = rig.drafted_and_published(STAGED, &with_triage()).await;

    assert_eq!(published.pinned, [id.clone()]);
    assert_eq!(rig.steps_published(), ["triage", "propose", "plan", "walk", "deliver"], "the repository serves the new steps");
    assert_eq!(rig.steps_read_by(&id).await, OLD, "and the Job still reads the ones it was running");
    let copy = rig.own_copy(&id).await.expect("a copy is kept");
    assert!(copy.pinned && copy.text == STAGED, "a pin of the text it ran");
    assert_eq!(record(&rig.rig, &id).await.workflow_step, Some(0), "it stands on Propose still");
    let held = Arc::clone(&rig.rig.fleet).get_job_copy(Some(id.clone())).await.expect("read");
    assert!(!held.own, "a pin is not a change the Job made");
}

/// **A publish that changes nothing a repository runs pins nothing**: Kit's copy beats the repository's, so a repository copy published under
/// it changes nothing the Job reads.
#[tokio::test]
async fn a_publish_that_does_not_change_what_the_repository_runs_pins_nothing() {
    let rig = migrating();
    let under_kit = |repository: &str| {
        let mut written = vec![config::Written::in_repository(at("repo", "staged.json"), repository.to_string())];
        written.push(config::Written::in_kit(at("kit", "staged.json"), with_triage()));
        read(written)
    };
    rig.planted.will_read(under_kit(STAGED));
    rig.rig.fleet.workflows_changed(&rig.rig.root);
    let id = rig.job().await;
    assert_eq!(rig.steps_read_by(&id).await[0], "triage", "Kit's copy is the one it runs");

    let changed = format!("{STAGED}\n# a second thought");
    rig.put("repository", &changed).await;
    rig.planted.will_save(Ok(SavedWorkflow { id: carried_id("staged"), file: "/repo/.armada/workflows/staged.json".into(), replaced: true }));
    rig.planted.will_read(under_kit(STAGED));
    rig.planted.will_read_next(under_kit(&changed));
    let publish = PublishWorkflowDraft { workflow_id: staged(), source: "repository".into(), ..Default::default() };
    let published = rig.rig.fleet.publish_workflow_draft(publish, None).await.expect("published");

    assert!(published.pinned.is_empty(), "Kit's copy still wins, so nothing moved for the Job");
    assert!(rig.own_copy(&id).await.is_none(), "and the pin made before the save was given back");
    assert_eq!(rig.steps_read_by(&id).await[0], "triage");
}

/// **A Job with edits of its own is not pinned**, and is listed `edited` so he sees it.
#[tokio::test]
async fn a_job_with_edits_of_its_own_is_not_pinned_and_is_listed_edited() {
    let rig = migrating();
    let id = rig.job().await;
    let reword = StepChange::Reword { step: "plan".into(), label: Some("Plan it".into()), about: None, instructions: None };
    Arc::clone(&rig.rig.fleet).change_session_steps(ChangeSessionSteps { session_id: id.clone(), change: reword }).await.expect("edited");
    let own = rig.own_copy(&id).await.expect("its own copy");
    assert!(!own.pinned, "an edit keeps the copy as the Job's");

    let published = rig.drafted_and_published(STAGED, &with_triage()).await;

    assert!(published.pinned.is_empty());
    assert_eq!(rig.own_copy(&id).await, Some(own), "its copy is as it left it");
    let listed = rig.affected().await;
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].edited, listed[0].pinned), (true, false));
}

/// **Before a publish the list is the preview**: a draft that drops the step a Job stands on says the step does not survive.
#[tokio::test]
async fn affected_before_a_publish_says_whether_each_jobs_step_survives_the_draft() {
    let rig = migrating();
    let id = rig.job().await;

    rig.put("repository", &without_propose()).await;
    let listed = rig.affected().await;
    assert_eq!(listed.len(), 1);
    let job = &listed[0];
    assert_eq!((job.session_id.clone(), job.step_id.as_deref(), job.step_label.as_deref()), (id.clone(), Some("propose"), Some("Propose")));
    assert_eq!((job.survives, job.pinned, job.edited, job.on_version), (false, false, false, None));

    rig.put("repository", &with_triage()).await;
    assert!(rig.affected().await[0].survives, "a draft that keeps the step does");
}

/// **A move drops the pin**, so the Job reads the new steps and stays on its step by id, and its agent is told once.
#[tokio::test]
async fn migrating_a_pinned_job_moves_it_to_the_new_steps_on_the_same_step_and_tells_the_agent_once() {
    let rig = migrating();
    let id = rig.job().await;
    rig.drafted_and_published(STAGED, &with_triage()).await;
    let listed = rig.affected().await;
    assert_eq!((listed[0].pinned, listed[0].survives, listed[0].on_version), (true, true, Some(1)), "the pin is the text found as version 1");

    let results = rig.migrate(&[&id]).await;

    assert_eq!(results.len(), 1);
    let moved = &results[0];
    assert_eq!((moved.moved, moved.step_id.as_deref(), moved.step_label.as_deref(), moved.code.as_deref()), (true, Some("propose"), Some("Propose"), None));
    assert!(rig.own_copy(&id).await.is_none(), "the pin is gone");
    assert_eq!(rig.steps_read_by(&id).await, ["triage", "propose", "plan", "walk", "deliver"]);
    assert_eq!(record(&rig.rig, &id).await.workflow_step, Some(1), "Propose is the second step now, and the Job is on it");
    eventually(|| async { rig.told(&id) == 1 }).await;
    let line = rig.rig.stand_in.process_of(id.as_str()).unwrap().sent.lock().unwrap().iter().find(|line| line.contains(MOVED)).cloned().unwrap();
    assert!(line.replace("\\\"", "\"").contains("it is on the \"Propose\" step."), "{line}");
    assert_eq!(rig.timeline_says().await, ["workflow: moved to the newest staged"]);

    let again = rig.migrate(&[&id]).await;
    assert!(!again[0].moved && again[0].code.is_none(), "a Job that reads the published text already is not moved, and is not refused");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(rig.told(&id), 1, "and is told nothing twice");
}

/// **Where the step a Job stood on is gone, it goes to the next step the new version still has.**
#[tokio::test]
async fn migrating_a_job_whose_step_the_new_version_dropped_puts_it_on_the_next_surviving_step() {
    let rig = migrating();
    let id = rig.job().await;
    rig.drafted_and_published(STAGED, &without_propose()).await;
    let listed = rig.affected().await;
    assert_eq!((listed[0].step_id.as_deref(), listed[0].survives), (Some("propose"), false));

    let results = rig.migrate(&[&id]).await;

    assert_eq!((results[0].moved, results[0].step_id.as_deref()), (true, Some("plan")));
    assert_eq!(rig.steps_read_by(&id).await, ["plan", "walk", "deliver"]);
    let row = record(&rig.rig, &id).await;
    assert_eq!(row.workflow_step, Some(0), "Plan is the first step now");
    assert_eq!(row.workflow_visits.iter().map(|visit| visit.step).collect::<Vec<_>>(), [0], "and the visit kept is to it, the dropped step's gone");
    eventually(|| async { rig.told(&id) == 1 }).await;
    let said = rig.rig.stand_in.process_of(id.as_str()).unwrap().sent.lock().unwrap().iter().find(|line| line.contains(MOVED)).cloned().unwrap();
    assert!(said.replace("\\\"", "\"").contains("\"Plan\" step"), "{said}");
}

/// The landing rule, where a dropped step has nothing after it that survives.
#[test]
fn the_landing_is_the_first_later_step_the_new_version_has_else_its_last() {
    let steps = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
    let old = steps(&["a", "b", "c", "d"]);
    assert_eq!(crate::workflow_migrating::landing(&old, &steps(&["a", "c", "d"]), "b"), Some(1), "c is the first after b that is kept");
    assert_eq!(crate::workflow_migrating::landing(&old, &steps(&["d", "a"]), "b"), Some(0), "the order is the old one, so d comes after b");
    assert_eq!(crate::workflow_migrating::landing(&old, &steps(&["a", "b", "x"]), "d"), Some(2), "nothing after d survives: the new last step");
    assert_eq!(crate::workflow_migrating::landing(&old, &steps(&["x"]), "gone"), None, "a step the old version never had");
    assert_eq!(crate::workflow_migrating::landing(&old, &[], "b"), None, "a version with no steps");
}

/// **Each Job answers for itself**: an edited Job is refused with its code and one that reads the published text is left alone, and a Job
/// Fleet does not host or one that follows nothing is refused too, in the order asked.
#[tokio::test]
async fn migrate_refuses_an_edited_job_and_does_nothing_for_one_that_is_not_pinned() {
    let rig = migrating();
    let unpinned = rig.job().await;
    let results = rig.migrate(&[&unpinned]).await;
    assert!(!results[0].moved && results[0].code.is_none() && results[0].message.is_some(), "{:?}", results[0]);

    let reword = StepChange::Reword { step: "plan".into(), label: Some("Plan it".into()), about: None, instructions: None };
    Arc::clone(&rig.rig.fleet).change_session_steps(ChangeSessionSteps { session_id: unpinned.clone(), change: reword }).await.expect("edited");
    let own = rig.own_copy(&unpinned).await;
    let plain = rig.rig.start().await;
    let nowhere = SessionId::carried("nowhere");

    let results = rig.migrate(&[&unpinned, &nowhere, &plain]).await;

    let codes: Vec<(&str, Option<&str>)> = results.iter().map(|one| (one.session_id.as_str(), one.code.as_deref())).collect();
    assert_eq!(
        codes,
        [
            (unpinned.as_str(), Some("fleet.workflow_migrate_edited")),
            ("nowhere", Some("fleet.workflow_migrate_not_live")),
            (plain.as_str(), Some("fleet.workflow_migrate_not_following")),
        ]
    );
    assert!(results.iter().all(|one| !one.moved));
    assert_eq!(rig.own_copy(&unpinned).await, own, "its edits are kept");
    assert_eq!(rig.told(&unpinned), 0, "and the agent is told nothing");
}
