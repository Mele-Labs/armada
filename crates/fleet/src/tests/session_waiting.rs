//! What a session waits on the person for: the agent's list kept whole, Fleet's own items
//! merged in, and the one route that settles an item by where it came from.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerWaiting, HelmCallAnswer, HelmCallInFlight, Instant, ManifestId, RunOrNot, SessionMode,
    SessionRow, SessionVoice, SetWaitingFor, ShowWindow, TuneSession, WaitingAct, WaitingActKind,
    WaitingInput, WaitingMode, WaitingOption, WaitingSource,
};
use store::KeptWaiting;

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::waiting::merged;

fn kept(id: &str, kind: &str, target: &str) -> KeptWaiting {
    KeptWaiting {
        item_id: id.into(),
        text: format!("text of {id}"),
        since: "2026-10-09T00:00:00Z".into(),
        act: Some((kind.into(), target.into())),
        options: Vec::new(),
    }
}

fn card(call: &str, question: Option<&str>) -> HelmCallInFlight {
    let questions = match question {
        Some(question) => ipc::decode(
            "questions",
            format!(
                r#"[{{"question":"{question}","header":"Clock","multiSelect":false,
                  "options":[{{"label":"Fake","description":""}},{{"label":"Frozen","description":""}}]}}]"#
            )
            .as_bytes(),
        )
        .unwrap(),
        None => Vec::new(),
    };
    HelmCallInFlight {
        call: call.into(),
        manifest_id: ManifestId::carried("armada"),
        asked_at: Instant::carried("2026-10-09T00:01:00Z"),
        tool: "Bash".into(),
        detail: "rm -rf build".into(),
        truncated: false,
        length: None,
        rule: "Bash(rm:*)".into(),
        offers: vec![HelmCallAnswer::AllowOnce, HelmCallAnswer::Refuse],
        holding_for_seconds: 600,
        questions,
    }
}

#[test]
fn the_agents_items_come_first_and_fleets_own_follow_with_stable_ids() {
    let asked = card("c1", Some("Which clock?"));
    let windows = [super::super::session_host::waiting::Window {
        url: "http://localhost:5173/".into(),
        title: "Findings".into(),
        since: "2026-10-09T00:02:00Z".into(),
    }];
    let items = merged(vec![kept("a1", "run", "cargo test")], Some(&asked), &windows, false);
    let ids: Vec<_> = items.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["a1", "ask:c1", "walk:http://localhost:5173/"]);
    assert_eq!(items[0].source, WaitingSource::Agent);
    assert_eq!(items[1].source, WaitingSource::AskCard);
    assert_eq!(items[1].options.len(), 2, "one single-choice question has numbered choices");
    assert_eq!(items[2].source, WaitingSource::Walk);

    let permission = merged(Vec::new(), Some(&card("c2", None)), &[], false);
    assert_eq!(permission[0].id, "perm:c2");
    assert_eq!(permission[0].source, WaitingSource::Permission);
    assert_eq!(permission[0].text, "Bash rm -rf build");
    assert_eq!(permission[0].options[0].label, "Allow once");
}

#[test]
fn where_an_agent_item_names_fleets_target_fleets_stands_and_an_ended_session_holds_nothing() {
    let asked = card("c1", Some("Which clock?"));
    let items = merged(
        vec![kept("mine", "answer", "c1"), kept("other", "answer", "c9")],
        Some(&asked),
        &[],
        false,
    );
    let ids: Vec<_> = items.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["other", "ask:c1"], "the agent's duplicate of the open card is left off");
    assert!(merged(vec![kept("a", "run", "x")], Some(&asked), &[], true).is_empty());
}

fn set(id: &ipc::SessionId, items: &[(&str, &str)]) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: items
            .iter()
            .map(|(item, text)| WaitingInput {
                id: (*item).into(),
                text: (*text).into(),
                act: Some(WaitingAct {
                    kind: WaitingActKind::Run,
                    target: "cargo test".into(),
                }),
                options: vec![WaitingOption { label: "Yes".into() }],
            })
            .collect(),
    }
}

#[tokio::test]
async fn the_agents_list_is_replaced_whole_and_an_unchanged_id_keeps_when_it_began() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let first = rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    assert_eq!(first.waiting_for.len(), 2);
    assert!(first.waiting_for.iter().all(|one| one.source == WaitingSource::Agent));
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let second = rig
        .fleet
        .waiting_for(None, set(&id, &[("b", "Look at it again"), ("c", "New")]))
        .await
        .unwrap();
    let ids: Vec<_> = second.waiting_for.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["b", "c"], "a is gone: the list is replaced");
    let was = |record: &ipc::SessionRecord, item: &str| {
        record.waiting_for.iter().find(|one| one.id == item).unwrap().since.clone()
    };
    assert_eq!(was(&second, "b"), was(&first, "b"), "an unchanged id keeps its since");
    assert_ne!(was(&second, "c"), was(&first, "b"), "a new id begins now");
    assert_eq!(second.waiting_for[0].text, "Look at it again");
    let cleared = rig.fleet.waiting_for(None, set(&id, &[])).await.unwrap();
    assert!(cleared.waiting_for.is_empty());
    let nameless = rig
        .fleet
        .waiting_for(None, SetWaitingFor { items: Vec::new(), session_id: None })
        .await;
    assert!(nameless.is_err(), "a call that places no session sets nothing");
}

fn answer(id: &ipc::SessionId, item: &str) -> AnswerWaiting {
    AnswerWaiting {
        session_id: id.clone(),
        item_id: item.into(),
        choice: None,
        text: None,
        mode: None,
    }
}

async fn waiting_call(rig: &Rig, id: &ipc::SessionId) -> String {
    let call = std::sync::Mutex::new(String::new());
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked;
        match asked {
            Some(asked) => {
                *call.lock().unwrap() = asked.call;
                true
            }
            None => false,
        }
    })
    .await;
    call.into_inner().unwrap()
}

fn asking(body: &str) -> ipc::AskingToRun {
    ipc::decode("an ask", body.as_bytes()).expect("an ask")
}

const ONE_QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which clock?","header":"Clock","multiSelect":false,
   "options":[{"label":"Fake","description":""},{"label":"Frozen","description":""}]}]}}"#;

#[tokio::test]
async fn an_open_question_is_answered_through_the_ask_by_a_numbered_choice() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking(ONE_QUESTION)).await });
    let call = waiting_call(&rig, &id).await;
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(record.waiting_for[0].id, format!("ask:{call}"));
    assert_eq!(record.waiting_for[0].options[1].label, "Frozen");

    let mut nothing = answer(&id, &format!("ask:{call}"));
    assert!(Arc::clone(&rig.fleet).answer_waiting(nothing.clone()).await.is_err(), "an empty answer is refused");
    nothing.choice = Some(7);
    assert!(Arc::clone(&rig.fleet).answer_waiting(nothing).await.is_err(), "a choice that is not there");

    let mut pick = answer(&id, &format!("ask:{call}"));
    pick.choice = Some(1);
    let after = Arc::clone(&rig.fleet).answer_waiting(pick).await.unwrap();
    let _ = after;
    let RunOrNot::Allow { updated_input } = held.await.unwrap().unwrap() else {
        panic!("an answered question runs");
    };
    let gone = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(gone.waiting_for.is_empty(), "the card is answered, so the item is gone");
    assert_eq!(updated_input.pointer("/answers/Which clock?").and_then(|one| one.as_str()), Some("Frozen"));
}

#[tokio::test]
async fn a_permission_takes_a_choice_and_a_mode_alone_refuses_it_with_the_line() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet
        .tune_session(TuneSession { session_id: id.clone(), model: None, effort: None, mode: SessionMode::Ask })
        .await
        .unwrap();
    let ask = r#"{"tool_name":"Bash","input":{"command":"rm -rf build"}}"#;
    for (choice, mode) in [(Some(0), None), (None, Some(WaitingMode::Quick))] {
        let fleet = Arc::clone(&rig.fleet);
        let session = id.as_str().to_string();
        let held = tokio::spawn(async move { fleet.session_permission(&session, asking(ask)).await });
        let call = waiting_call(&rig, &id).await;
        let mut said = answer(&id, &format!("perm:{call}"));
        said.choice = choice;
        said.mode = mode;
        Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
        match held.await.unwrap().unwrap() {
            RunOrNot::Allow { .. } => assert_eq!(choice, Some(0), "choice 0 is Allow once"),
            RunOrNot::Deny { message } => {
                assert!(mode.is_some());
                assert!(message.contains("Take the reasonable path and keep moving"), "{message}");
            }
        }
    }
}

#[tokio::test]
async fn an_open_walk_window_waits_until_it_is_approved_and_an_agent_item_becomes_a_message() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5173/";
    let shown = rig
        .fleet
        .show_window(None, ShowWindow { url: url.into(), title: Some("Findings".into()), session_id: Some(id.clone()) })
        .await
        .unwrap();
    assert!(!shown.waiting_for.is_empty(), "{:?}", shown.attachments);
    assert_eq!(shown.waiting_for[0].id, format!("walk:{url}"));
    assert_eq!(shown.waiting_for[0].source, WaitingSource::Walk);

    Arc::clone(&rig.fleet).answer_waiting(answer(&id, &format!("walk:{url}"))).await.unwrap();
    let approved = rig.rows(&id).await.iter().any(|row| {
        matches!(row, SessionRow::Message { from: SessionVoice::You, text, .. } if text == &format!("Approved: {url}"))
    });
    assert!(approved, "the approval is the person's message");
    let now = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(now.waiting_for.is_empty(), "an approved walk stops waiting");

    rig.fleet.waiting_for(None, set(&id, &[("a1", "Run the tests")])).await.unwrap();
    let mut said = answer(&id, "a1");
    said.choice = Some(0);
    said.mode = Some(WaitingMode::Best);
    Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    let sent: Vec<String> = rig
        .rows(&id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect();
    let last = sent.last().unwrap();
    assert!(last.starts_with("Re: Run the tests\nYes\n"), "{last}");
    assert!(last.contains("Think it through and go with the best option"), "{last}");

    let missing = Arc::clone(&rig.fleet).answer_waiting({
        let mut said = answer(&id, "ghost");
        said.text = Some("hi".into());
        said
    }).await;
    assert!(matches!(missing, Err(api::Refusal::IllegalMove(_))), "an item nothing holds");
}
