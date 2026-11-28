//! The claim, written before the code: **each project has one Operator, and no Operator hears another project's
//! arrivals or words.** A repository two projects hold tells both and names the other; the machine's switch ends every
//! Operator and a project's own ends only its own; a repository in no project becomes one; and what a project sets for
//! its Operator is read where the Operator starts. `crates/ipc/operations/set_operator_settings.toml`.

use std::sync::Arc;

use api::Projects;
use ipc::{ManifestId, NewProject, OperatorRole, OperatorSettingsSet, OperatorToolsSet, ProjectMember, SetOperatorSettings};

use super::operator_projects_rig::{heard_by, operator_of, projects_rig, set_for};
use super::operator_session::switched;
use super::session_host::eventually;
use crate::operator::{Arrival, Arrived, Source};

pub(super) fn arrival_in(manifest: &ManifestId, id: &str, subject: &str) -> Arrival {
    Arrival {
        id: id.into(),
        arrived: Arrived::Report,
        source: Source::Agent,
        session_id: String::new(),
        manifest_id: manifest.as_str().to_string(),
        project: String::new(),
        subject: subject.into(),
        words: "The scrollback is lost on a resize.".into(),
    }
}

fn whole(manifest: &ManifestId) -> Option<Vec<ProjectMember>> {
    Some(vec![ProjectMember { manifest_id: manifest.clone(), folder: None }])
}

pub(super) async fn project_named(rig: &super::session_host::Rig, name: &str, manifest: &ManifestId) -> String {
    let added = rig.fleet.add_project(NewProject { name: name.into(), members: whole(manifest), place: None, values: None }).await.expect("added");
    added.projects.into_iter().find(|one| one.name == name).expect("the project").id
}

#[tokio::test]
async fn an_arrival_reaches_the_operator_of_the_project_that_holds_its_repository_and_never_another() {
    let (rig, second, _home) = projects_rig();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &second).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's Operator"), operator_of(&rig, &b).await.expect("B's Operator"));
    assert_ne!(for_a, for_b, "one each");
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;

    assert!(rig.fleet.hand_once(arrival_in(&rig.manifest, "report:1", "Scrollback is lost")).await, "handed");
    eventually(|| async { heard_by(&rig, &for_a, "Scrollback is lost") >= 1 }).await;
    assert_eq!(heard_by(&rig, &for_a, "Scrollback is lost"), 1, "A's Operator hears its own repository once");
    assert_eq!(heard_by(&rig, &for_b, "Scrollback is lost"), 0, "B's never does");
    assert!(!heard_by_text(&rig, &for_a).contains("shared with"), "a repository only A holds says nothing of sharing");
}

fn heard_by_text(rig: &super::session_host::Rig, session: &str) -> String {
    rig.stand_in.process_of(session).expect("running").sent.lock().unwrap().join("\n")
}

#[tokio::test]
async fn a_repository_two_projects_hold_tells_both_operators_once_and_names_the_other_project() {
    let (rig, _second, _home) = projects_rig();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &rig.manifest).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's"), operator_of(&rig, &b).await.expect("B's"));
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;

    assert!(rig.fleet.hand_once(arrival_in(&rig.manifest, "report:1", "Scrollback is lost")).await);
    assert!(rig.fleet.hand_once(arrival_in(&rig.manifest, "report:1", "Scrollback is lost")).await, "handed again");
    eventually(|| async { heard_by(&rig, &for_a, "Scrollback is lost") >= 1 && heard_by(&rig, &for_b, "Scrollback is lost") >= 1 }).await;
    assert_eq!(heard_by(&rig, &for_a, "Scrollback is lost"), 1, "told once each");
    assert_eq!(heard_by(&rig, &for_b, "Scrollback is lost"), 1);
    assert!(heard_by_text(&rig, &for_a).contains("shared with the project: Beta"), "{}", heard_by_text(&rig, &for_a));
    assert!(heard_by_text(&rig, &for_b).contains("shared with the project: Alpha"), "{}", heard_by_text(&rig, &for_b));
}

#[tokio::test]
async fn the_machine_switch_ends_every_operator_and_a_project_switch_ends_only_its_own() {
    let (rig, second, _home) = projects_rig();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &second).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    assert!(operator_of(&rig, &a).await.is_some() && operator_of(&rig, &b).await.is_some(), "both run");

    set_for(&rig, &a, OperatorSettingsSet { on: Some(false), ..Default::default() }).await;
    rig.fleet.operator_session_kept().await;
    assert!(operator_of(&rig, &a).await.is_none(), "A's own switch off ends A's");
    assert!(operator_of(&rig, &b).await.is_some(), "and leaves B's");

    set_for(&rig, &a, OperatorSettingsSet { on: Some(true), ..Default::default() }).await;
    rig.fleet.operator_session_kept().await;
    assert!(operator_of(&rig, &a).await.is_some(), "on again starts it again");

    switched(&rig, false).await;
    rig.fleet.operator_session_kept().await;
    assert!(operator_of(&rig, &a).await.is_none() && operator_of(&rig, &b).await.is_none(), "the machine's switch ends every one");
}

#[tokio::test]
async fn a_repository_in_no_project_becomes_a_project_named_for_it_and_gets_an_operator() {
    let (rig, _second, _home) = projects_rig();
    assert!(rig.fleet.list_projects().await.expect("listed").projects.is_empty(), "none to begin with");
    rig.fleet.operator_session_kept().await;
    assert!(rig.fleet.list_projects().await.expect("listed").projects.is_empty(), "switched off, Fleet makes none he did not");

    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let projects = rig.fleet.list_projects().await.expect("listed").projects;
    assert_eq!(projects.len(), 2, "one for each served repository: {projects:?}");
    let first = projects.iter().find(|one| one.members.iter().any(|member| member.manifest_id == rig.manifest)).expect("the first repository's project");
    assert_eq!(first.name, std::path::Path::new(&rig.root).file_name().expect("a folder").to_string_lossy(), "named for the repository");
    assert!(first.members.iter().all(|member| member.folder.is_none()), "the whole repository");
    for one in &projects {
        assert!(operator_of(&rig, &one.id).await.is_some(), "{} has its Operator", one.name);
    }

    rig.fleet.operator_session_kept().await;
    assert_eq!(rig.fleet.list_projects().await.expect("listed").projects.len(), 2, "a second tick makes no more");
}

#[tokio::test]
async fn a_project_reads_as_the_defaults_until_he_sets_a_value_and_the_role_sets_the_tools_under_his_own() {
    let (rig, _second, _home) = projects_rig();
    let a = project_named(&rig, "Alpha", &rig.manifest).await;
    let fresh = rig.fleet.operator_settings_of(&a).await;
    let (model, effort): (String, String) = {
        let now = rig.fleet.machine_settings().now();
        (now.get(config::settings::MODELS_OPERATOR), now.get(config::settings::EFFORT_OPERATOR))
    };
    assert_eq!((fresh.model.as_str(), fresh.effort.as_str()), (model.as_str(), effort.as_str()), "today's model and effort");
    assert_eq!(fresh.role, OperatorRole::Copilot);
    assert_eq!(fresh.decisions, ipc::OperatorDecisions::Cards);
    assert!(fresh.on, "on by default");
    assert_eq!((fresh.name.as_str(), fresh.output_style.as_str()), ("Operator", "armada:Operator"));
    assert!(fresh.accent.is_none() && fresh.words.is_empty() && fresh.prompt.is_empty());
    assert_eq!(fresh.tools, OperatorRole::Copilot.tools());
    assert!(fresh.tools.start_jobs && fresh.tools.merge && fresh.tools.end_jobs && fresh.tools.file_issues);

    set_for(&rig, &a, OperatorSettingsSet { role: Some(OperatorRole::Dispatcher), ..Default::default() }).await;
    let dispatcher = rig.fleet.operator_settings_of(&a).await.tools;
    assert_eq!((dispatcher.start_jobs, dispatcher.merge, dispatcher.end_jobs, dispatcher.file_issues), (true, false, true, true));

    set_for(&rig, &a, OperatorSettingsSet { role: Some(OperatorRole::Reviewer), ..Default::default() }).await;
    let reviewer = rig.fleet.operator_settings_of(&a).await.tools;
    assert_eq!((reviewer.start_jobs, reviewer.merge, reviewer.end_jobs, reviewer.file_issues), (false, false, false, false));

    let own = OperatorToolsSet { merge: Some(true), ..Default::default() };
    set_for(&rig, &a, OperatorSettingsSet { tools: Some(own), ..Default::default() }).await;
    let tools = rig.fleet.operator_settings_of(&a).await.tools;
    assert_eq!((tools.start_jobs, tools.merge, tools.end_jobs, tools.file_issues), (false, true, false, false), "his own switch wins over the preset's");

    let listed = rig.fleet.operator_settings_set(SetOperatorSettings { project: a.clone(), set: OperatorSettingsSet::default(), unset: vec!["tools.merge".into(), "role".into()] }).await.expect("unset");
    let one = listed.projects.iter().find(|one| one.project == a).expect("listed");
    assert_eq!((one.settings.role, one.settings.tools.merge, one.set.role), (OperatorRole::Copilot, true, None), "unset returns each to its default");
    assert_eq!(listed.styles.first().map(String::as_str), Some("armada:Operator"));

    let bad = rig.fleet.operator_settings_set(SetOperatorSettings { project: a.clone(), set: OperatorSettingsSet { accent: Some("red".into()), ..Default::default() }, unset: Vec::new() }).await.expect_err("not a hex colour");
    assert_eq!(bad.status(), 422);
    let none = rig.fleet.operator_settings_set(SetOperatorSettings { project: "no-such".into(), set: OperatorSettingsSet::default(), unset: Vec::new() }).await.expect_err("no such project");
    assert_eq!(none.status(), 404);
}

#[tokio::test]
async fn a_change_to_what_the_session_starts_with_hands_it_on_and_the_successor_runs_the_new_model() {
    let (rig, _second, _home) = projects_rig();
    let a = project_named(&rig, "Alpha", &rig.manifest).await;
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let old = operator_of(&rig, &a).await.expect("started");
    eventually(|| async { rig.stand_in.process_of(&old).is_some() }).await;

    // Nothing that is read on the next call hands it on.
    set_for(&rig, &a, OperatorSettingsSet { name: Some("Kit".into()), ..Default::default() }).await;
    assert_eq!(operator_of(&rig, &a).await.as_deref(), Some(old.as_str()), "a name applies on the next call");

    // What it runs on and its role are told at its start, so the session is handed on. The model is one the machine has
    // on: Fleet moves a session off a model that is switched off (`serving/living.rs`), whoever named it.
    let was = rig.stand_in.starts().into_iter().find(|one| one.session == old).and_then(|one| one.model);
    let other = rig.fleet.models_on().into_iter().find(|model| Some(model) != was.as_ref()).expect("a second model on");
    set_for(&rig, &a, OperatorSettingsSet { model: Some(other.clone()), role: Some(OperatorRole::Reviewer), ..Default::default() }).await;
    // The hand puts a `replaced_by` on the old session, which is how the other hand-on tests find the successor.
    let replaced = || async {
        let all = api::Sessions::list_sessions(rig.fleet.as_ref(), None, None, None).await.expect("listed").sessions;
        all.into_iter().find(|one| one.id.as_str() == old).and_then(|one| one.attachments.into_iter().find(|link| link.kind == "replaced_by")).map(|link| link.target)
    };
    let mut new = None;
    for _ in 0..500 {
        new = replaced().await;
        if new.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let all = rig.fleet.store().lock().await.live_operator_hostings().unwrap().into_iter().map(|h| (h.session_id, h.model, h.project)).collect::<Vec<_>>();
    let new = new.unwrap_or_else(|| panic!("the old session was replaced by none; live Operators {all:?}; starts {:?}", rig.stand_in.starts().iter().map(|s| (s.session.clone(), s.model.clone())).collect::<Vec<_>>()));
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == new) }).await;
    let start = rig.stand_in.starts().into_iter().find(|one| one.session == new).expect("its process");
    let hosting = rig.fleet.store().lock().await.hosting(&new).expect("read").expect("hosted");
    assert_eq!(start.model.as_deref(), Some(other.as_str()), "the successor runs the new model; hosting {hosting:?}; live {all:?}");
    assert!(start.operator.as_ref().is_some_and(|prompt| prompt.role == OperatorRole::Reviewer), "and its new role: {start:?}");
    assert!(hosting.operator && hosting.project.as_deref() == Some(a.as_str()), "still the project's Operator: {hosting:?}");
    assert_eq!(operator_of(&rig, &a).await.as_deref(), Some(new.as_str()), "and the only one that runs");

    // A change to where it asks decisions tells the live session, and hands nothing on.
    set_for(&rig, &a, OperatorSettingsSet { decisions: Some(ipc::OperatorDecisions::Chat), ..Default::default() }).await;
    eventually(|| async { heard_by(&rig, &new, "how you ask decisions") >= 1 }).await;
    assert_eq!(operator_of(&rig, &a).await.as_deref(), Some(new.as_str()), "decisions hand nothing on");
}

#[tokio::test]
async fn his_words_reach_the_operator_of_the_project_the_talk_names_and_no_other() {
    let (rig, second, _home) = projects_rig();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &second).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's"), operator_of(&rig, &b).await.expect("B's"));
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;

    let mut talk = super::operator_cards::said("Look at the reader", None);
    talk.project = Some(b.clone());
    Arc::clone(&rig.fleet).operator_talked(talk).await.expect("said");
    eventually(|| async { heard_by(&rig, &for_b, "Look at the reader") >= 1 }).await;
    assert_eq!(heard_by(&rig, &for_a, "Look at the reader"), 0, "A's Operator never hears words for B");

    // Two run and he named none: his words are kept and he is asked which, so neither Operator hears them yet.
    Arc::clone(&rig.fleet).operator_talked(super::operator_cards::said("Anyone?", None)).await.expect("kept");
    assert_eq!(heard_by(&rig, &for_a, "Anyone?") + heard_by(&rig, &for_b, "Anyone?"), 0);
    assert_eq!(rig.fleet.store().lock().await.operator_held_sends().expect("read").len(), 1);
}
