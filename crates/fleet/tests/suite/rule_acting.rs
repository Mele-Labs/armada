//! What a rule planned is done through the doors Fleet has, tried again up to the rule's cap, and
//! the owner or the agent is told where it still fails. **A watch a rule starts for a deploy is the
//! deploy's**: its readings move the instance the Job took on with it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use api::HostedSessions;
use core_model::{ForgeFact, Observed};
use ipc::{SessionId, SessionRow};
use store::rule_act_standing as standing;

use super::repositories::Planted;
use super::session_host::{rig_fitted, Rig};

const DEPLOYS_ON_APPROVAL: &str = "version: 1
rule_id: deploy_on_approval
name: Follow the web deploy
when: { enters: { lifecycle: pr, state: approved } }
do:
  - start_watch: { recipe: qa-deploy, values: { sha: abc }, deploy: web }
  - tell_agent: { message: \"Deploying.\" }
";

fn qa_deploy() -> config::WatchRecipeWritten {
    config::WatchRecipeWritten::in_repository(
        PathBuf::from(".armada/watches/qa.yml"),
        "name: qa-deploy\ncommand: [/bin/cat, \"{sha}\"]\nevery: 2m\n".to_string(),
    )
}

pub(super) fn a_rig() -> Rig {
    let planted = Arc::new(Planted::nothing());
    planted.will_offer_recipes(vec![qa_deploy()]);
    rig_fitted(move |fittings| fittings.locating = planted)
}

pub(super) fn save(rig: &Rig, name: &str, text: &str) {
    let folder = Path::new(&rig.fleet.host().kit_home).join("rules");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(folder.join(name), text).expect("a rule file");
}

pub(super) async fn approved(rig: &Rig, job: &SessionId) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "pr", "812", &[], &at).expect("taken on");
    assert!(rig.fleet.lifecycle_seen(job.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await.is_some());
}

pub(super) async fn acts(rig: &Rig, job: &SessionId) -> Vec<(String, String, u32)> {
    let kept = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    kept.into_iter().map(|one| (one.text, one.state, one.attempts)).collect()
}

pub(super) async fn notices_in(rig: &Rig, job: &SessionId) -> Vec<(Option<String>, String)> {
    let rows = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").rows;
    rows.into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice, text, .. } => Some((notice, text)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_watch_for_a_deploy_is_started_followed_and_its_readings_move_the_deploy() {
    let rig = a_rig();
    save(&rig, "deploy.yml", DEPLOYS_ON_APPROVAL);
    let job = rig.start().await;
    approved(&rig, &job).await;
    assert_eq!(
        acts(&rig, &job).await,
        [
            ("Start the web deploy watch".to_string(), standing::PLANNED.to_string(), 0),
            ("Tell the agent: Deploying.".to_string(), standing::PLANNED.to_string(), 0),
        ]
    );

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(
        acts(&rig, &job).await.iter().map(|one| (one.1.as_str(), one.2)).collect::<Vec<_>>(),
        [(standing::DONE, 1), (standing::DONE, 1)]
    );
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| one.2).collect::<Vec<_>>(), [1, 1], "a done act is not tried again");

    let (watch, followed) = {
        let store = rig.fleet.store().lock().await;
        let watches = store.watches_of(job.as_str()).expect("reads");
        assert_eq!(watches.len(), 1, "one watch, for the one deploy");
        let held = store.lifecycle_instances_of(job.as_str()).expect("reads");
        let deploy = held.iter().find(|one| one.lifecycle == "deploy").expect("the Job took the deploy on");
        (watches[0].watch.id.clone(), deploy.watch_id.clone())
    };
    assert_eq!(followed, watch, "the deploy follows the watch the rule started");

    rig.fleet
        .watch_read(&watch, store::KeptWatchTaken::Read { state: "running".into(), finished: false, says: String::new(), link: None, parts: Vec::new() })
        .await;
    let moves = rig.fleet.store().lock().await.lifecycle_moves_of(job.as_str()).expect("reads");
    let deploy: Vec<(&str, &str)> = moves.iter().filter(|one| one.lifecycle == "deploy").map(|one| (one.from.as_str(), one.to.as_str())).collect();
    assert!(deploy.iter().any(|one| one.1 == "in:rolling_out"), "the watch's reading reached the deploy: {deploy:?}");

    assert!(
        notices_in(&rig, &job).await.iter().any(|one| one.0.as_deref() == Some("lifecycle_rule") && one.1.contains("Deploying.")),
        "the agent was told, as a lifecycle rule's word"
    );
}

pub(super) fn failing(then: &str, retries: u32) -> String {
    format!(
        "version: 1\nrule_id: watch_nothing\nname: Watch a recipe nobody wrote\nwhen: {{ enters: {{ lifecycle: pr, state: approved }} }}\ndo:\n  - start_watch: {{ recipe: nope, values: {{}} }}\non_failure: {{ retries: {retries}, then: {then} }}\n"
    )
}

#[tokio::test]
async fn an_act_that_keeps_failing_is_tried_up_to_the_rules_cap_and_then_asks_the_owner() {
    let rig = a_rig();
    save(&rig, "watch.yml", &failing("ask_owner", 2));
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let kept = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].state.as_str(), kept[0].attempts), (standing::STUCK, 3), "the first try and two retries");
    assert!(kept[0].said.starts_with("Start the nope watch failed: "), "{}", kept[0].said);
    assert!(kept[0].said.contains("nope"), "it says what failed: {}", kept[0].said);

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(rig.fleet.store().lock().await.rule_act(kept[0].id).expect("reads").expect("there").attempts, 3, "a stuck act waits for the owner");
}

#[tokio::test]
async fn an_act_whose_rule_says_to_tell_the_agent_tells_it_what_failed_and_is_done_with() {
    let rig = a_rig();
    save(&rig, "watch.yml", &failing("tell_agent", 0));
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let kept = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    assert_eq!((kept[0].state.as_str(), kept[0].attempts), (standing::TOLD, 1));
    let told = notices_in(&rig, &job).await;
    assert!(
        told.iter().any(|one| one.0.as_deref() == Some("lifecycle_rule") && one.1.starts_with("Start the nope watch failed: ")),
        "{told:?}"
    );
}

#[tokio::test]
async fn an_act_planned_before_fleet_stopped_is_done_when_it_comes_back_and_once() {
    let rig = a_rig();
    save(
        &rig,
        "tell.yml",
        "version: 1\nrule_id: tell_on_approval\nname: Tell on approval\nwhen: { enters: { lifecycle: pr, state: approved } }\ndo:\n  - tell_agent: { message: \"Approved.\" }\n",
    );
    let job = rig.start().await;
    approved(&rig, &job).await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| one.1.as_str()).collect::<Vec<_>>(), [standing::PLANNED]);

    let rig = rig.restarted(|_| {});
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| (one.1.as_str(), one.2)).collect::<Vec<_>>(), [(standing::DONE, 1)]);
    let told = notices_in(&rig, &job).await.into_iter().filter(|one| one.1.contains("Approved.")).count();
    assert_eq!(told, 1, "told once");
}

#[tokio::test]
async fn a_job_and_a_draft_a_rule_acts_on_are_started_and_proposed_through_their_own_doors() {
    let rig = a_rig();
    save(
        &rig,
        "hand_off.yml",
        "version: 1\nrule_id: hand_off\nname: Hand off\nwhen: { enters: { lifecycle: pr, state: approved } }\nautonomy: acts\ndo:\n  - start_job: { prompt: Look at the deploy }\n  - draft: { template: 'Ready for {pr}', on: { entered: true } }\n",
    );
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(
        acts(&rig, &job).await,
        [
            ("Start a Job: Look at the deploy".to_string(), standing::DONE.to_string(), 1),
            ("Draft a message on PR #812: Ready for PR #812".to_string(), standing::DONE.to_string(), 1),
        ]
    );
    let rows = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").rows;
    assert!(rows.iter().any(|row| matches!(row, SessionRow::Started { session, .. } if session.title == "Look at the deploy")), "the Job is the parent's");
    assert!(
        rows.iter().any(|row| matches!(row, SessionRow::Draft { title, text, settled: None, .. } if title == "Message on PR #812" && text == "Ready for PR #812")),
        "the draft waits for the owner to say its words are right"
    );
}

/// A rig whose repository holds the `bug` and `feature` workflows.
fn a_rig_with_workflows() -> Rig {
    rig_fitted(|fittings| {
        fittings.locating = Arc::new(Planted::nothing());
        fittings.starting().workflows =
            ["bug", "feature"].into_iter().map(super::daemon::workflow_named).map(|one| (one.id().clone(), one)).collect();
    })
}

fn starts_on(workflow: &str) -> String {
    format!("version: 1\nrule_id: hand_off\nname: Hand off\nwhen: {{ enters: {{ lifecycle: pr, state: approved }} }}\nautonomy: acts\ndo:\n  - start_job: {{ workflow: {workflow}, prompt: Look at the deploy }}\n")
}

async fn started_from(rig: &Rig, job: &SessionId) -> Vec<SessionId> {
    let rows = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").rows;
    rows.into_iter().filter_map(|row| match row {
        SessionRow::Started { session, .. } => Some(session.id),
        _ => None,
    }).collect()
}

#[tokio::test]
async fn a_job_a_rule_starts_follows_the_workflow_the_rule_names() {
    let rig = a_rig_with_workflows();
    save(&rig, "hand_off.yml", &starts_on("bug"));
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| one.1.as_str()).collect::<Vec<_>>(), [standing::DONE]);
    let started = started_from(&rig, &job).await;
    assert_eq!(started.len(), 1, "one Job");
    let record = Arc::clone(&rig.fleet).get_session(started[0].clone()).await.expect("the new Job").session;
    assert_eq!(record.workflow.as_deref(), Some("bug"));
}

#[tokio::test]
async fn a_rule_naming_a_workflow_the_repository_does_not_hold_starts_no_job() {
    let rig = a_rig_with_workflows();
    save(&rig, "hand_off.yml", &starts_on("nowhere"));
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_ne!(acts(&rig, &job).await[0].1, standing::DONE);
    assert!(started_from(&rig, &job).await.is_empty(), "nothing is started");
}
