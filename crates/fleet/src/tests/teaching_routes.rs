//! The link rules a person taught, over their routes: kept per repository beside the every-repository
//! ones, forgotten for good, held to the rules a proposal is, and a note told to the live Session
//! they taught in, queued behind a turn it is in the middle of and never refused.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Teaching};
use ipc::{SaveTeaching, SessionId, TeachingLink};

use super::session_host::{eventually, rig_placing, Rig};

fn link(example: &str) -> TeachingLink {
    TeachingLink {
        pattern: r"[A-Z][A-Z0-9]+-\d+".to_string(),
        address: "https://myproject.jira.com/{match}".to_string(),
        example: example.to_string(),
    }
}

fn saving(link: Option<TeachingLink>, repository: Option<&str>) -> SaveTeaching {
    SaveTeaching { link, repository: repository.map(str::to_string), session_id: None, note: None }
}

fn code(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

async fn listed(rig: &Rig, repository: Option<&str>) -> Vec<(String, Option<String>)> {
    let read = rig.fleet.list_taught_links(repository.map(str::to_string)).await.expect("listed");
    read.links.into_iter().map(|one| (one.example, one.repository)).collect()
}

#[tokio::test]
async fn a_rule_is_kept_for_its_repository_and_a_rule_for_every_repository_is_listed_beside_it() {
    let rig = rig_placing(None);
    let root = rig.root.clone();
    let kept = Arc::clone(&rig.fleet).save_teaching(saving(Some(link("SHOP-1")), Some(&root))).await.expect("saved");
    let kept = kept.link.expect("the rule");
    assert_eq!((kept.pattern.as_str(), kept.repository.as_deref()), (r"[A-Z][A-Z0-9]+-\d+", Some(root.as_str())));
    Arc::clone(&rig.fleet).save_teaching(saving(Some(link("ALL-2")), None)).await.expect("saved");

    assert_eq!(
        listed(&rig, Some(&root)).await,
        [("SHOP-1".to_string(), Some(root.clone())), ("ALL-2".to_string(), None)]
    );
    assert_eq!(listed(&rig, None).await, [("ALL-2".to_string(), None)]);
    let (status, said) = code(rig.fleet.list_taught_links(Some("/nowhere".into())).await.expect_err("not served"));
    assert_eq!((status, said.as_str()), (404, "fleet.run_memory_repository_not_served"));
}

#[tokio::test]
async fn a_forgotten_rule_is_gone_and_forgetting_it_again_is_a_404() {
    let rig = rig_placing(None);
    let saved = Arc::clone(&rig.fleet).save_teaching(saving(Some(link("ALL-2")), None)).await.expect("saved");
    let id = saved.link.expect("the rule").id;

    assert_eq!(rig.fleet.forget_taught_link(id.clone()).await.expect("forgotten").id, id);
    assert!(listed(&rig, None).await.is_empty());
    for named in [id, "not-a-number".to_string()] {
        let (status, said) = code(rig.fleet.forget_taught_link(named).await.expect_err("none"));
        assert_eq!((status, said.as_str()), (404, "fleet.no_such_taught_link"));
    }
}

#[tokio::test]
async fn a_rule_that_cannot_work_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let mut bad = link("STORE-1234");
    bad.pattern = r"BLOG-\d+".to_string();
    let refused = Arc::clone(&rig.fleet).save_teaching(saving(Some(bad), None)).await.expect_err("refused");
    assert_eq!(code(refused), (422, "fleet.teaching_link_refused".to_string()));
    let mut insecure = link("STORE-1234");
    insecure.address = "ftp://myproject.example/{match}".to_string();
    assert!(Arc::clone(&rig.fleet).save_teaching(saving(Some(insecure), None)).await.is_err());
    let empty = Arc::clone(&rig.fleet).save_teaching(saving(None, None)).await.expect_err("nothing to keep");
    assert_eq!(code(empty), (422, "fleet.teaching_input_refused".to_string()));
    let elsewhere = Arc::clone(&rig.fleet).save_teaching(saving(Some(link("STORE-1")), Some("/nowhere"))).await;
    assert_eq!(code(elsewhere.expect_err("not served")).0, 404);
    assert!(listed(&rig, None).await.is_empty());
}

fn told(session: &SessionId, note: &str) -> SaveTeaching {
    SaveTeaching { session_id: Some(session.as_str().to_string()), note: Some(note.to_string()), ..saving(None, None) }
}

fn lines_to(rig: &Rig) -> Vec<String> {
    rig.stand_in.process(0).sent.lock().unwrap().clone()
}

#[tokio::test]
async fn an_idle_session_is_told_the_note_in_fleets_voice() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    let before = lines_to(&rig).len();

    Arc::clone(&rig.fleet).save_teaching(told(&id, "Propose ideas with a sketch.")).await.expect("saved");

    let lines = lines_to(&rig);
    assert_eq!(lines.len(), before + 1, "one turn was started");
    assert!(lines.last().unwrap().contains("From now on: Propose ideas with a sketch."));
}

#[tokio::test]
async fn a_session_in_the_middle_of_a_turn_has_the_note_queued_and_it_is_not_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    assert_eq!(rig.turn(&id).await, ipc::SessionTurn::Working { woken_by: None }, "the turn has not ended");
    let before = lines_to(&rig).len();
    let queued = rig.fleet.hosts().of(id.as_str()).state().queued;

    let saved = Arc::clone(&rig.fleet).save_teaching(told(&id, "Use the focused test command.")).await;
    assert!(saved.is_ok(), "{:?}", saved.err().map(code));

    let lines = lines_to(&rig);
    assert_eq!(lines.len(), before + 1, "the line was handed to the process behind the running turn");
    assert!(lines.last().unwrap().contains("From now on: Use the focused test command."));
    assert_eq!(rig.fleet.hosts().of(id.as_str()).state().queued, queued + 1, "counted as a queued turn, not dropped");
}

#[tokio::test]
async fn a_session_that_cannot_be_told_does_not_fail_the_save() {
    let rig = rig_placing(None);
    let gone = SessionId::carried("01NOSUCHSESSIONAAAAAAAAAA");
    let mut save = told(&gone, "Whatever.");
    save.link = Some(link("ALL-2"));
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved all the same");
    assert!(saved.link.is_some());

    let ended = rig.start().await;
    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: ended.clone() }).await.expect("closed");
    assert!(Arc::clone(&rig.fleet).save_teaching(told(&ended, "Whatever.")).await.is_ok());
}
