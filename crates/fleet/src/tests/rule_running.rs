//! A Job keeps the rules it started with, hears what happens to it, and has what they plan kept
//! once. **No forge, tracker or process is reached**: a reader's move is handed in by the test, as
//! `lifecycle_seen` is by the readers.

use std::path::Path;

use core_model::rule::JobEvent;
use core_model::{ForgeFact, Observed};
use ipc::{SessionId, SessionRecord};
use store::rule_act_standing as standing;

use std::sync::Arc;

use testkit::FakeJudge;

use super::session_host::{rig_placing, Rig};
use super::session_rail::{ruled_on, workflow};
use crate::lifecycles::copying::copy_of;

const TELLS_ON_APPROVAL: &str = "version: 1
rule_id: tell_on_approval
name: Tell the agent when the pull request is approved
when: { enters: { lifecycle: pr, state: approved } }
do:
  - tell_agent: { message: \"Approved. Merge it.\" }
";

const TELLS_ON_START: &str = "version: 1
rule_id: tell_on_start
name: Tell the agent when it starts
when: { job: started }
do:
  - tell_agent: { message: \"Go.\" }
";

fn save(rig: &Rig, name: &str, text: &str) {
    let folder = Path::new(&rig.fleet.host().kit_home).join("rules");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(folder.join(name), text).expect("a rule file");
}

async fn takes_on_a_pull_request(rig: &Rig, job: &SessionId) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet.store().lock().await.take_on_lifecycle_instance(job.as_str(), "pr", "812", &[], &at).expect("taken on");
}

async fn approved(rig: &Rig, job: &SessionId) -> bool {
    rig.fleet.lifecycle_seen(job.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await.is_some()
}

async fn planned(rig: &Rig, job: &SessionId) -> Vec<(String, String, String)> {
    let kept = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    kept.into_iter().map(|one| (one.rule_id, one.state, one.text)).collect()
}

#[tokio::test]
async fn a_job_keeps_the_rules_it_started_with_and_a_rule_saved_later_reaches_only_the_jobs_after_it() {
    let rig = rig_placing(None);
    let early = rig.start().await;
    save(&rig, "approved.yml", TELLS_ON_APPROVAL);
    let late = rig.start().await;
    save(&rig, "started.yml", TELLS_ON_START);

    let store = rig.fleet.store().lock().await;
    let rules = |job: &SessionId| copy_of(&store, job.as_str()).map(|copy| copy.rules.len());
    assert_eq!(rules(&early), Some(0), "started before the rule was saved: it has a copy, and the copy holds none");
    assert_eq!(rules(&late), Some(1), "the second file was saved after, and is in neither copy");
    drop(store);

    for job in [&early, &late] {
        takes_on_a_pull_request(&rig, job).await;
        assert!(approved(&rig, job).await, "the reader's move is kept either way");
    }
    assert!(planned(&rig, &early).await.is_empty());
    assert_eq!(
        planned(&rig, &late).await,
        [("tell_on_approval".to_string(), standing::PLANNED.to_string(), "Tell the agent: Approved. Merge it.".to_string())]
    );
}

#[tokio::test]
async fn the_job_started_event_is_heard_once_and_a_restart_replays_nothing() {
    let rig = rig_placing(None);
    save(&rig, "started.yml", TELLS_ON_START);
    save(&rig, "approved.yml", TELLS_ON_APPROVAL);
    let job = rig.start().await;
    takes_on_a_pull_request(&rig, &job).await;
    assert!(approved(&rig, &job).await);
    let kept = planned(&rig, &job).await;
    assert_eq!(kept.iter().map(|one| one.0.as_str()).collect::<Vec<_>>(), ["tell_on_start", "tell_on_approval"]);

    // The same events heard again plan nothing: by hand, and after Fleet came back.
    let again = core_model::rule::Occurrence::job(JobEvent::Started, "started");
    rig.fleet.rules_heard(job.as_str(), &again).await;
    assert_eq!(planned(&rig, &job).await, kept);
    let rig = rig.restarted(|_| {});
    rig.fleet.rules_caught_up().await;
    assert_eq!(planned(&rig, &job).await, kept, "the move was replayed, and every rule had seen it");
}

async fn record_of(rig: &Rig, job: &SessionId) -> SessionRecord {
    use api::HostedSessions;
    std::sync::Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").session
}

#[tokio::test]
async fn a_step_starting_and_ending_and_the_pull_request_opening_are_found_by_comparing_two_looks() {
    let rig = ruled_on(Arc::new(FakeJudge::saying("nothing")), workflow());
    save(
        &rig,
        "steps.yml",
        "version: 1\nrule_id: tell_on_steps\nname: Tell on steps\nwhen: { step_starts: implement }\ndo:\n  - tell_agent: { message: \"Second\" }\n",
    );
    save(
        &rig,
        "ended.yml",
        "version: 1\nrule_id: tell_on_leaving\nname: Tell on leaving\nwhen: { step_ends: plan }\ndo:\n  - tell_agent: { message: \"Left the first\" }\n",
    );
    save(
        &rig,
        "opened.yml",
        "version: 1\nrule_id: tell_on_opening\nname: Tell on opening\nwhen: { job: pull_request_opened }\nif:\n  - job: { step: implement }\ndo:\n  - tell_agent: { wake: true }\n",
    );
    let job = rig.start().await;
    let mut row = record_of(&rig, &job).await;
    row.workflow = Some("rail".to_string());

    row.workflow_step = Some(0);
    rig.fleet.rules_looked_at(&row).await;
    assert!(planned(&rig, &job).await.is_empty(), "entering the first step is not a rule's moment here");
    rig.fleet.rules_looked_at(&row).await;
    row.workflow_step = Some(1);
    rig.fleet.rules_looked_at(&row).await;
    rig.fleet.rules_looked_at(&row).await;
    let rules = |kept: Vec<(String, String, String)>| kept.into_iter().map(|one| one.0).collect::<Vec<_>>();
    assert_eq!(rules(planned(&rig, &job).await), ["tell_on_leaving", "tell_on_steps"], "the step it left, then the one it entered, once each");

    row.attachments.push(ipc::Attachment {
        kind: "pr".to_string(),
        manifest_id: None,
        target: "812".to_string(),
        state: ipc::AttachmentState::Standing,
        detail: Default::default(),
        since: ipc::Instant::carried("2026-10-29T10:00:00Z"),
        changed_at: ipc::Instant::carried("2026-10-29T10:00:00Z"),
    });
    rig.fleet.rules_looked_at(&row).await;
    rig.fleet.rules_looked_at(&row).await;
    assert_eq!(
        rules(planned(&rig, &job).await),
        ["tell_on_leaving", "tell_on_steps", "tell_on_opening"],
        "the opening is heard once, and its If reads the step the look left the Job on"
    );
}
