//! Sub projects, written before the code: **the Jobs he picked are kept under his name and goal with the features whose
//! live Jobs held one of them at the press**, refused where the name, the goal, the Jobs or their project is wrong, listed
//! oldest first and dropped on Ungroup. **Every one has an orchestrator**, a Job titled with the goal, and its Goal met?
//! card folds it away on Agree. `docs/concepts/feature-catalog.md`, *Sub projects*.

use std::sync::Arc;

use api::{Features, HostedSessions, Refusal, Sessions};
use ipc::{AllDone, AnswerWaiting, CloseSession, DismissWaiting, SessionId, SessionRow, StartSubProject, TurnIntoOrchestrator, UngroupSubProject, WaitingItem, WaitingSource};
use store::{AttachmentState, Holder, KeptAttachment, KeptSubProject};

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, held_slot, seeded, wrote, VOTING};

const GOAL: &str = "Votes rank takes on the page";

fn start(project: &str, name: &str, jobs: &[&SessionId]) -> StartSubProject {
    StartSubProject { project: project.to_string(), name: name.to_string(), goal: GOAL.to_string(), jobs: jobs.iter().map(|job| (*job).clone()).collect() }
}

fn code(refusal: Refusal) -> (&'static str, String) {
    match refusal {
        Refusal::Unacceptable(why) => ("422", why.code),
        Refusal::NotFound(why) => ("404", why.code),
        Refusal::IllegalMove(why) => ("409", why.code),
        other => panic!("refused otherwise: {other:?}"),
    }
}

/// A project with its features seeded and one Job at work on `voting-on-takes`.
async fn at_work(rig: &Rig) -> (String, SessionId) {
    let project = a_project(rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let job = an_agent_for(rig, &project).await;
    wrote(rig, &job, "crates/pages/voting/ranks.rs").await;
    held_slot(rig, &job, "4").await;
    (project, job)
}

async fn listed(rig: &Rig, project: Option<&str>, ended: bool) -> ipc::SubProjects {
    Arc::clone(&rig.fleet).list_sub_projects(project.map(String::from), ended).await.expect("listed")
}

async fn said(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

async fn goal_card(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("kept").session;
    record.waiting_for.into_iter().find(|one| one.source == WaitingSource::GoalMet)
}

async fn slots_of(rig: &Rig, id: &SessionId) -> Vec<String> {
    let held = rig.fleet.store().lock().await.attachments_of(&Holder::session(id.as_str())).expect("read");
    held.into_iter().filter(|one| one.kind == "slot" && one.state == AttachmentState::Standing).map(|one| one.target).collect()
}

fn answer(job: &SessionId, item: &str, choice: u32) -> AnswerWaiting {
    AnswerWaiting { session_id: job.clone(), item_id: item.to_string(), choice: Some(choice), text: None, mode: None, fixed: None, extras: Default::default() }
}

fn dismiss(job: &SessionId, item: &str) -> DismissWaiting {
    DismissWaiting { session_id: job.clone(), item_id: item.to_string(), reason: None, misread: false }
}

async fn says_done(rig: &Rig, job: &SessionId) {
    rig.fleet.all_done(None, AllDone { session_id: Some(job.clone()), summary: None }).await.expect("done");
}

/// Pull request `number` of `job`, merged as the forge describes it.
async fn merged(rig: &Rig, job: &SessionId, number: u64, title: &str) {
    let now = rig.fleet.now().as_str().to_string();
    let row = KeptAttachment {
        holder: Holder::session(job.as_str()),
        kind: crate::pull_requesting::PR.into(),
        manifest_id: rig.manifest.as_str().to_string(),
        target: number.to_string(),
        state: AttachmentState::Spent,
        detail: [("state".to_string(), "merged".to_string()), ("title".to_string(), title.to_string())].into(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&row, false).expect("attached");
}

#[tokio::test]
async fn a_sub_project_keeps_his_jobs_in_his_order_and_the_features_they_are_on_now() {
    let rig = rig(None);
    let (project, voting) = at_work(&rig).await;
    let idle = an_agent_for(&rig, &project).await;

    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "  Rank votes ", &[&idle, &voting])).await.expect("started");
    assert_eq!(kept.name, "Rank votes", "trimmed");
    assert_eq!(kept.goal, GOAL);
    assert_eq!(kept.project, project);
    assert_eq!(kept.jobs, [idle, voting], "in his order");
    assert_eq!(kept.features, [VOTING], "page-actions has no Job on it");
    assert!(!kept.id.is_empty());

    assert_eq!(listed(&rig, Some(&project), false).await.sub_projects, [kept]);
}

#[tokio::test]
async fn a_sub_project_with_no_name_no_goal_or_no_jobs_is_refused_and_nothing_is_kept() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let no_name = Arc::clone(&rig.fleet).start_sub_project(start(&project, "   ", &[&job])).await.expect_err("refused");
    assert_eq!(code(no_name), ("422", "fleet.sub_project_no_name".to_string()));
    let no_goal = StartSubProject { goal: "  ".into(), ..start(&project, "Rank votes", &[&job]) };
    let no_goal = Arc::clone(&rig.fleet).start_sub_project(no_goal).await.expect_err("refused");
    assert_eq!(code(no_goal), ("422", "fleet.sub_project_no_goal".to_string()));
    let no_jobs = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[])).await.expect_err("refused");
    assert_eq!(code(no_jobs), ("422", "fleet.sub_project_no_jobs".to_string()));
    assert!(listed(&rig, None, false).await.sub_projects.is_empty());
}

#[tokio::test]
async fn a_job_fleet_does_not_hold_is_refused_404() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let unknown = SessionId::carried("no-such-job");
    let refused = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job, &unknown])).await.expect_err("refused");
    assert_eq!(code(refused), ("404", "fleet.no_such_session".to_string()));
    assert!(listed(&rig, None, false).await.sub_projects.is_empty());
}

#[tokio::test]
async fn jobs_of_two_projects_are_refused_409() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let other = a_project(&rig).await;
    let elsewhere = an_agent_for(&rig, &other).await;
    let refused = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job, &elsewhere])).await.expect_err("refused");
    assert_eq!(code(refused), ("409", "fleet.sub_project_across_projects".to_string()));
    assert!(listed(&rig, None, false).await.sub_projects.is_empty());
}

#[tokio::test]
async fn sub_projects_are_listed_by_project_oldest_first_and_every_project_without_one() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let other = a_project(&rig).await;
    let elsewhere = an_agent_for(&rig, &other).await;
    let first = Arc::clone(&rig.fleet).start_sub_project(start(&project, "First", &[&job])).await.expect("started");
    let second = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Second", &[&job])).await.expect("started");
    let theirs = Arc::clone(&rig.fleet).start_sub_project(start(&other, "Theirs", &[&elsewhere])).await.expect("started");
    assert!(theirs.features.is_empty(), "the other project holds no feature");

    let names = |listed: ipc::SubProjects| listed.sub_projects.into_iter().map(|one| one.name).collect::<Vec<_>>();
    assert_eq!(names(listed(&rig, Some(&project), false).await), [first.name.clone(), second.name.clone()]);
    assert_eq!(names(listed(&rig, Some(&other), false).await), [theirs.name.clone()]);
    assert_eq!(names(listed(&rig, None, false).await), [first.name, second.name, theirs.name]);
}

#[tokio::test]
async fn ungroup_drops_the_sub_project_and_leaves_its_jobs_and_an_unknown_id_is_404() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");

    rig.fleet.ungroup_sub_project(UngroupSubProject { id: kept.id.clone() }).await.expect("ungrouped");
    assert!(listed(&rig, Some(&project), false).await.sub_projects.is_empty());
    let library = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    let row = library.areas[0].features.iter().find(|one| one.slug == VOTING).expect("the feature");
    assert_eq!(row.jobs_now.iter().map(|one| one.session_id.clone()).collect::<Vec<_>>(), [job], "the Job is untouched");

    let again = rig.fleet.ungroup_sub_project(UngroupSubProject { id: kept.id }).await.expect_err("refused");
    assert_eq!(code(again), ("404", "fleet.no_such_sub_project".to_string()));
}

#[tokio::test]
async fn a_sub_project_starts_its_orchestrator_titled_with_his_goal_and_briefed_with_his_jobs() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;

    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");
    let orchestrator = kept.orchestrator.clone().expect("an orchestrator");
    assert!(!kept.jobs.contains(&orchestrator), "it is not one of his Jobs");
    let record = Arc::clone(&rig.fleet).get_session(orchestrator.clone()).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some(GOAL), "titled with his goal");
    let brief = said(&rig, &orchestrator).await.into_iter().next().expect("its brief");
    for part in ["\"Rank votes\"".to_string(), format!("Goal: {GOAL}"), format!("- Add rank votes ({})", job.as_str()), "all_done".to_string()] {
        assert!(brief.contains(&part), "the brief carries {part}: {brief}");
    }
}

#[tokio::test]
async fn a_sub_project_kept_without_an_orchestrator_gets_one_on_the_first_read_and_only_once() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let old = KeptSubProject {
        id: "sp-old".into(),
        project: project.clone(),
        name: "Rank votes".into(),
        goal: "Rank votes".into(),
        jobs: vec![job.as_str().to_string()],
        features: Vec::new(),
        orchestrator: None,
        started_at: rig.fleet.now().as_str().to_string(),
        not_yet_at: None,
        ended_at: None,
    };
    rig.fleet.store().lock().await.put_sub_project(&old).expect("kept");

    let first = listed(&rig, Some(&project), false).await.sub_projects;
    let orchestrator = first[0].orchestrator.clone().expect("started on the read");
    let record = Arc::clone(&rig.fleet).get_session(orchestrator.clone()).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some("Rank votes"), "its name is its goal");
    let again = listed(&rig, None, false).await.sub_projects;
    assert_eq!(again[0].orchestrator, Some(orchestrator), "the same one");

    // One whose start was tried and failed is never tried again.
    let tried = KeptSubProject { id: "sp-tried".into(), ..old };
    rig.fleet.store().lock().await.put_sub_project(&tried).expect("kept");
    let at = rig.fleet.now().as_str().to_string();
    assert!(rig.fleet.store().lock().await.ask_sub_project_orchestrator("sp-tried", &at).expect("asked"));
    let read = listed(&rig, Some(&project), false).await.sub_projects;
    assert_eq!(read.iter().find(|one| one.id == "sp-tried").expect("listed").orchestrator, None, "tried once");
}

#[tokio::test]
async fn a_running_job_turned_into_an_orchestrator_hands_its_work_to_a_first_child() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let turn = TurnIntoOrchestrator { project: project.clone(), session_id: job.clone(), goal: format!(" {GOAL} ") };

    let kept = Arc::clone(&rig.fleet).turn_into_orchestrator(turn.clone()).await.expect("turned");
    assert_eq!(kept.orchestrator.as_ref(), Some(&job));
    assert_eq!(kept.goal, GOAL);
    assert_eq!(kept.name, "Add rank votes", "the Job's title");
    assert_eq!(kept.features, [VOTING]);
    let [child] = kept.jobs.as_slice() else { panic!("one first child: {:?}", kept.jobs) };
    let record = Arc::clone(&rig.fleet).get_session(child.clone()).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some("Add rank votes"));
    assert_eq!(slots_of(&rig, child).await, ["4"], "the child holds the slot");
    assert!(slots_of(&rig, &job).await.is_empty(), "the orchestrator holds none");
    let to_child = said(&rig, child).await.into_iter().next().expect("its first message");
    for part in [format!("carries on the work of \"Add rank votes\" ({})", job.as_str()), "Handoff from Add rank votes".to_string(), "Slot 4".to_string()] {
        assert!(to_child.contains(&part), "the child is told {part}: {to_child}");
    }
    let to_job = said(&rig, &job).await.join("\n");
    for part in [format!("Your own work moved to \"Add rank votes\" ({})", child.as_str()), format!("Goal: {GOAL}")] {
        assert!(to_job.contains(&part), "the Job is told {part}: {to_job}");
    }
    assert_eq!(listed(&rig, Some(&project), false).await.sub_projects, [kept.clone()]);

    let twice = Arc::clone(&rig.fleet).turn_into_orchestrator(turn).await.expect_err("orchestrates already");
    assert_eq!(code(twice), ("409", "fleet.sub_project_not_turnable".to_string()));
    let unknown = TurnIntoOrchestrator { project: project.clone(), session_id: SessionId::carried("no-such-job"), goal: GOAL.into() };
    assert_eq!(code(Arc::clone(&rig.fleet).turn_into_orchestrator(unknown).await.expect_err("unknown")), ("404", "fleet.no_such_session".to_string()));
    let no_goal = TurnIntoOrchestrator { project, session_id: child.clone(), goal: " ".into() };
    assert_eq!(code(Arc::clone(&rig.fleet).turn_into_orchestrator(no_goal).await.expect_err("no goal")), ("422", "fleet.sub_project_no_goal".to_string()));
}

#[tokio::test]
async fn the_orchestrators_done_asks_goal_met_and_agree_folds_the_sub_project_into_history() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");
    let orchestrator = kept.orchestrator.clone().expect("an orchestrator");
    assert!(goal_card(&rig, &orchestrator).await.is_none(), "the Job is at work and the orchestrator said nothing");

    says_done(&rig, &orchestrator).await;
    assert!(goal_card(&rig, &orchestrator).await.is_some(), "its done asks");
    merged(&rig, &job, 412, "Votes rank takes").await;
    let card = goal_card(&rig, &orchestrator).await.expect("Goal met?");
    assert_eq!(card.text, "Goal met?");
    assert_eq!(card.id, format!("goal_met:{}", kept.id));
    assert_eq!(card.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Agree", "Not yet"]);
    let met = card.goal_met.clone().expect("the goal and what landed");
    assert_eq!((met.sub_project.as_str(), met.goal.as_str()), (kept.id.as_str(), GOAL));
    assert_eq!(met.landed.iter().map(|one| (one.number, one.title.as_str())).collect::<Vec<_>>(), [(412, "Votes rank takes")]);

    Arc::clone(&rig.fleet).answer_waiting(answer(&orchestrator, &card.id, 0)).await.expect("agreed");
    assert!(listed(&rig, Some(&project), false).await.sub_projects.is_empty(), "folded away");
    let history = listed(&rig, Some(&project), true).await.sub_projects;
    assert_eq!(history.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [kept.id.as_str()], "kept in history");
    assert!(history[0].ended_at.is_some());
    assert!(goal_card(&rig, &orchestrator).await.is_none());
    let again = Arc::clone(&rig.fleet).answer_waiting(answer(&orchestrator, &card.id, 0)).await.expect_err("settled");
    assert_eq!(code(again).0, "409");
}

#[tokio::test]
async fn not_yet_keeps_it_open_until_the_orchestrator_is_done_again() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");
    let orchestrator = kept.orchestrator.clone().expect("an orchestrator");
    says_done(&rig, &orchestrator).await;
    let card = goal_card(&rig, &orchestrator).await.expect("Goal met?");

    Arc::clone(&rig.fleet).answer_waiting(answer(&orchestrator, &card.id, 1)).await.expect("not yet");
    assert_eq!(listed(&rig, Some(&project), false).await.sub_projects.len(), 1, "still open");
    assert!(goal_card(&rig, &orchestrator).await.is_none(), "held back while the same done stands");

    // The orchestrator works on, and says done again later than his Not yet.
    let mut store = rig.fleet.store().lock().await;
    store.clear_done(orchestrator.as_str()).expect("at work");
    store.keep_done(orchestrator.as_str(), "2999-01-01T00:00:00Z", None).expect("done again");
    drop(store);
    assert!(goal_card(&rig, &orchestrator).await.is_some(), "asked again");
}

#[tokio::test]
async fn every_job_merged_asks_goal_met_without_the_orchestrators_word() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");
    let orchestrator = kept.orchestrator.clone().expect("an orchestrator");
    assert!(goal_card(&rig, &orchestrator).await.is_none());

    merged(&rig, &job, 412, "Votes rank takes").await;
    let card = goal_card(&rig, &orchestrator).await.expect("Goal met?");
    assert_eq!(card.goal_met.expect("carried").landed.iter().map(|one| one.number).collect::<Vec<_>>(), [412]);
}

#[tokio::test]
async fn every_job_ended_asks_goal_met_without_the_orchestrators_word_and_it_is_not_dismissed() {
    let rig = rig(None);
    let (project, job) = at_work(&rig).await;
    let kept = Arc::clone(&rig.fleet).start_sub_project(start(&project, "Rank votes", &[&job])).await.expect("started");
    let orchestrator = kept.orchestrator.clone().expect("an orchestrator");
    assert!(goal_card(&rig, &orchestrator).await.is_none());

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("ended");
    let card = goal_card(&rig, &orchestrator).await.expect("Goal met?");
    assert!(card.goal_met.clone().expect("carried").landed.is_empty(), "nothing landed");
    let refused = Arc::clone(&rig.fleet).dismiss_waiting(dismiss(&orchestrator, &card.id)).await.expect_err("answered, not dismissed");
    assert_eq!(code(refused).0, "422");
    let words = AnswerWaiting { choice: None, text: Some("done?".into()), ..answer(&orchestrator, &card.id, 0) };
    assert_eq!(code(Arc::clone(&rig.fleet).answer_waiting(words).await.expect_err("a choice only")).0, "422");
}
