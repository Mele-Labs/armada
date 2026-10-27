//! What a session waits on the person for: the agent's list kept whole, Fleet's own items
//! merged in, and the one route that settles an item by where it came from.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerWaiting, DismissWaiting, HelmCallAnswer, HelmCallInFlight, Instant, ManifestId, RunOrNot, SessionMode,
    SessionRow, SessionVoice, SetWaitingFor, ShowWindow, TuneSession, WaitingAct, WaitingActKind,
    WaitingInput, WaitingMode, WaitingOption, WaitingSource, WindowFailed, WindowTake,
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
        steps: Vec::new(),
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
        whole: None,
        rule: "Bash(rm:*)".into(),
        offers: vec![HelmCallAnswer::AllowOnce, HelmCallAnswer::Refuse],
        holding_for_seconds: 600,
        questions,
        reaches: None,
        reaches_to: None,
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
    let items = merged(vec![kept("a1", "run", "cargo test")], Some(&asked), &windows, &[], false);
    let ids: Vec<_> = items.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["a1", "ask:c1", "walk:http://localhost:5173/"]);
    assert_eq!(items[0].source, WaitingSource::Agent);
    assert_eq!(items[1].source, WaitingSource::AskCard);
    assert_eq!(items[1].options.len(), 2, "one single-choice question has numbered choices");
    assert_eq!(items[2].source, WaitingSource::Walk);

    let permission = merged(Vec::new(), Some(&card("c2", None)), &[], &[], false);
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
        &[],
        false,
    );
    let ids: Vec<_> = items.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["other", "ask:c1"], "the agent's duplicate of the open card is left off");
    assert!(merged(vec![kept("a", "run", "x")], Some(&asked), &[], &[], true).is_empty());
}

#[test]
fn an_agent_walk_item_is_dropped_whatever_page_it_names_and_a_run_on_the_same_address_stays() {
    let items = merged(
        vec![
            kept("mine", "walk", "http://localhost:5173/one"),
            kept("other", "walk", "http://localhost:5173/three"),
            kept("run", "run", "http://localhost:5173/one"),
        ],
        None,
        &[],
        &[],
        false,
    );
    let ids: Vec<_> = items.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["run"], "a walk reaches the list only as the window Fleet lists");
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
                options: vec![WaitingOption { label: "Yes".into(), description: None }],
                steps: Vec::new(),
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

#[tokio::test]
async fn the_steps_an_agent_gives_an_item_reach_the_served_item_and_survive_a_read() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut call = set(&id, &[("oauth", "Register the OAuth app"), ("plain", "Look")]);
    call.items[0].steps = vec!["  Name it Armada ".into(), "".into(), "Press Register".into()];
    let record = rig.fleet.waiting_for(None, call).await.unwrap();
    let steps = |record: &ipc::SessionRecord, item: &str| {
        record.waiting_for.iter().find(|one| one.id == item).unwrap().steps.clone()
    };
    assert_eq!(steps(&record, "oauth"), ["Name it Armada", "Press Register"], "trimmed, and a blank line is no step");
    assert!(steps(&record, "plain").is_empty(), "an item without steps has none");

    let kept = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(steps(&kept, "oauth"), ["Name it Armada", "Press Register"], "kept in the store, not only echoed back");
    let wire = ipc::encode(&kept.waiting_for[1]).unwrap();
    assert!(!wire.contains("steps"), "an empty list is left off the wire");
}

fn answer(id: &ipc::SessionId, item: &str) -> AnswerWaiting {
    AnswerWaiting {
        session_id: id.clone(),
        item_id: item.into(),
        choice: None,
        text: None,
        mode: None,
        fixed: None,
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
                assert!(message.contains("finish the work and land it"), "{message}");
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
        .show_window(None, ShowWindow { url: url.into(), title: Some("Findings".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
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

#[tokio::test]
async fn a_pick_with_words_tells_the_agent_both() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a1", "Run the tests")])).await.unwrap();
    let mut said = answer(&id, "a1");
    said.choice = Some(0);
    said.text = Some("  but only the store ones  ".into());
    Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    let last = rig
        .rows(&id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .last()
        .unwrap();
    assert_eq!(last, "Re: Run the tests\nYes\nbut only the store ones\nThis settles the item, so Fleet took it off your waiting list. Do not add it back.");
}

/// One item as an agent words it, with no act: how the walks the owner saw on 19 Oct were sent.
fn said(id: &ipc::SessionId, item: &str, text: &str) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![WaitingInput { id: item.into(), text: text.into(), act: None, options: Vec::new(), steps: Vec::new() }],
    }
}

fn refused_for(answer: Result<ipc::SessionRecord, api::Refusal>) -> String {
    match answer {
        Err(api::Refusal::Unacceptable(error)) => error.message,
        other => panic!("expected the walk refused, got {other:?}"),
    }
}

#[tokio::test]
async fn an_agent_item_asking_him_to_walk_is_refused_while_no_window_is_shown() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let walk = said(&id, "walk-pr-panel-cache", "Walk it: open a pull request panel, close it, open it again.");
    let why = refused_for(rig.fleet.waiting_for(None, walk).await);
    assert!(why.contains("show_window"), "{why}");
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.is_empty(), "he sees no walk with nothing to walk: {:?}", ids(&record));
}

#[tokio::test]
async fn an_agent_item_asking_him_to_walk_is_refused_once_a_window_is_shown_and_fleets_stands() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5187/?walk=filesAsATree";
    show(&rig, &id, url).await;
    let worded = said(&id, "files-tree", "Walk the pull request Files tab at localhost:5187/?walk=filesAsATree, then say land it.");
    refused_for(rig.fleet.waiting_for(None, worded).await);
    let mut acted = set(&id, &[("again", "Look at the other page")]);
    acted.items[0].act = Some(WaitingAct { kind: WaitingActKind::Walk, target: "http://localhost:5173/never".into() });
    refused_for(rig.fleet.waiting_for(None, acted).await);
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(ids(&record), [format!("walk:{url}").as_str()], "one walk, Fleet's");
}

#[tokio::test]
async fn an_agent_walk_item_kept_before_the_rule_is_never_drawn_and_approve_clears_the_walk() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5201/?walk=delivery-question";
    show(&rig, &id, url).await;
    let kept = KeptWaiting {
        item_id: "walk-delivery-question".into(),
        text: "Walk the delivery question (window open, 4 steps), then say land it or what to change.".into(),
        since: "2026-10-19T17:59:35Z".into(),
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
    };
    rig.fleet.store().lock().await.replace_waiting_for(id.as_str(), &[kept]).unwrap();
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(ids(&record), [format!("walk:{url}").as_str()], "one walk, Fleet's");

    Arc::clone(&rig.fleet).answer_waiting(answer(&id, &format!("walk:{url}"))).await.unwrap();
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.is_empty(), "a walk he approved leaves nothing behind: {:?}", ids(&record));
}

async fn show(rig: &Rig, id: &ipc::SessionId, url: &str) -> ipc::SessionRecord {
    rig.fleet
        .show_window(None, ShowWindow { url: url.into(), title: Some(url.into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .unwrap()
}

async fn later() {
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
}

fn ids(record: &ipc::SessionRecord) -> Vec<&str> {
    record.waiting_for.iter().map(|one| one.id.as_str()).collect()
}

async fn say(rig: &Rig, id: &ipc::SessionId, text: &str) {
    rig.send(id, text).await;
}

#[tokio::test]
async fn an_older_walk_still_waits_beside_a_newer_one() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    show(&rig, &id, "http://localhost:5173/one").await;
    later().await;
    let record = show(&rig, &id, "http://localhost:5173/two").await;
    let mut waiting = ids(&record);
    waiting.sort();
    assert_eq!(waiting, ["walk:http://localhost:5173/one", "walk:http://localhost:5173/two"]);
}

#[tokio::test]
async fn a_message_does_not_settle_a_walk_only_his_approve_does() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    show(&rig, &id, "http://localhost:5173/one").await;
    later().await;
    say(&rig, &id, "Looks off, change the header").await;
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(ids(&record), ["walk:http://localhost:5173/one"], "a message settles nothing");
    say(&rig, &id, "Approved: http://localhost:5173/one").await;
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.is_empty(), "{:?}", ids(&record));
}

#[tokio::test]
async fn a_walk_with_no_newer_walk_and_no_message_still_stands() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    show(&rig, &id, "http://localhost:5173/one").await;
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(ids(&record), ["walk:http://localhost:5173/one"], "closing its window decides nothing");
}

fn takes(pairs: [(&str, &str); 2]) -> Vec<WindowTake> {
    pairs.map(|(name, url)| WindowTake { name: name.into(), url: url.into() }).into()
}

#[tokio::test]
async fn a_walk_shown_again_on_another_port_replaces_the_older_window() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let old = "http://localhost:5391/?walk=jobHandoff";
    let shown = ShowWindow {
        url: old.into(),
        session_id: Some(id.clone()),
        variations: takes([("Handoff works", old), ("Handoff fails", "http://localhost:5391/?walk=jobHandoffFails")]),
        ..Default::default()
    };
    rig.fleet.show_window(None, shown).await.unwrap();
    later().await;
    let new = "http://localhost:5393/?walk=jobHandoffFails";
    let again = ShowWindow {
        url: new.into(),
        session_id: Some(id.clone()),
        variations: takes([("Handoff fails", new), ("Handoff works", "http://localhost:5393/?walk=jobHandoff")]),
        ..Default::default()
    };
    let record = rig.fleet.show_window(None, again).await.unwrap();
    assert_eq!(ids(&record), [format!("walk:{new}")], "the same walks on a new server are one walk");

    later().await;
    say(&rig, &id, &format!("Approved A, Handoff fails: {new}")).await;
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.is_empty(), "{:?}", ids(&record));
}

#[tokio::test]
async fn a_walk_whose_page_cannot_load_stops_waiting_until_it_is_shown_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5391/?walk=jobHandoff";
    show(&rig, &id, url).await;
    let failed = |why: &str| WindowFailed { session_id: id.clone(), url: url.into(), why: why.into() };

    Arc::clone(&rig.fleet).window_failed(failed("HTTP 500")).await.unwrap();
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert_eq!(ids(&record), [format!("walk:{url}")], "a page that loads broken is still to be walked once fixed");

    Arc::clone(&rig.fleet).window_failed(failed("ERR_CONNECTION_REFUSED")).await.unwrap();
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.is_empty(), "nothing answers on that address: {:?}", ids(&record));

    later().await;
    let record = show(&rig, &id, url).await;
    assert_eq!(ids(&record), [format!("walk:{url}")], "shown again, it waits again");
}

#[tokio::test]
async fn a_dismissed_item_never_comes_back_and_nothing_is_sent_to_the_agent() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5173/one";
    show(&rig, &id, url).await;
    let before = rig.rows(&id).await.len();
    let gone = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: format!("walk:{url}"), reason: None, misread: false })
        .await
        .unwrap();
    assert!(gone.waiting_for.is_empty());
    let again = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap();
    assert!(again.session.waiting_for.is_empty(), "the derived item does not return on the next read");
    assert_eq!(again.rows.len(), before, "nothing was said to the agent");
    let twice = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: format!("walk:{url}"), reason: None, misread: false })
        .await;
    assert!(matches!(twice, Err(api::Refusal::IllegalMove(_))), "nothing holds it any more");
}

#[tokio::test]
async fn a_walk_he_dismissed_waits_again_when_the_job_shows_it_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5173/one";
    show(&rig, &id, url).await;
    Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: format!("walk:{url}"), reason: None, misread: false })
        .await
        .unwrap();
    later().await;
    let record = show(&rig, &id, url).await;
    assert_eq!(ids(&record), [format!("walk:{url}")], "shown again, a dismissed walk waits again");
}

#[tokio::test]
async fn a_dismissed_agent_item_is_gone_and_stays_gone_when_the_agent_says_it_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    let after = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: "a".into(), reason: None, misread: false })
        .await
        .unwrap();
    assert_eq!(ids(&after), ["b"]);
    let restated = rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    assert_eq!(ids(&restated), ["b"]);
}

#[tokio::test]
async fn an_answered_agent_item_leaves_the_list_at_once_and_stays_gone_when_restated() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    let mut said = answer(&id, "a");
    said.choice = Some(0);
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    assert_eq!(ids(&after), ["b"], "the answered item is off the record the answer returns");
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
    assert!(last.ends_with("This settles the item, so Fleet took it off your waiting list. Do not add it back."), "{last}");
    let restated = rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    assert_eq!(ids(&restated), ["b"]);
}

#[tokio::test]
async fn an_answer_pressed_again_while_the_first_is_delivered_reaches_the_agent_once() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    let press = || {
        let mut said = answer(&id, "a");
        said.text = Some("fix all of these".into());
        Arc::clone(&rig.fleet).answer_waiting(said)
    };
    // The store held while all three press, so each reads the item before the first has sent it.
    let held = rig.fleet.store().lock().await;
    let let_go = async move {
        tokio::task::yield_now().await;
        drop(held);
    };
    let (one, two, three, ()) = tokio::join!(press(), press(), press(), let_go);
    let refused = [&one, &two, &three].into_iter().filter(|each| each.is_err()).count();
    assert_eq!(refused, 2, "every press after the first is refused as settled already");
    assert_eq!(said_by_the_person(&rig, &id).await.len(), 1);
}

/// What the person has said to the session so far, oldest first.
async fn said_by_the_person(rig: &Rig, id: &ipc::SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn removal(id: &ipc::SessionId, item: &str, reason: Option<&str>) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.into(), reason: reason.map(str::to_string), misread: false }
}

#[tokio::test]
async fn removing_an_agent_item_with_a_reason_tells_the_agent_what_and_why() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    let after = Arc::clone(&rig.fleet).dismiss_waiting(removal(&id, "a", Some("  already ran them  "))).await.unwrap();
    assert_eq!(ids(&after), ["b"]);
    assert_eq!(said_by_the_person(&rig, &id).await, [r#"Owner removed "Run the tests": already ran them"#]);
}

#[tokio::test]
async fn removing_an_agent_item_without_a_reason_tells_the_agent_only_what() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, set(&id, &[("a", "Run the tests"), ("b", "Look")])).await.unwrap();
    Arc::clone(&rig.fleet).dismiss_waiting(removal(&id, "a", None)).await.unwrap();
    rig.fleet.waiting_for(None, set(&id, &[("b", "Look"), ("c", "Check")])).await.unwrap();
    Arc::clone(&rig.fleet).dismiss_waiting(removal(&id, "b", Some("   "))).await.unwrap();
    assert_eq!(said_by_the_person(&rig, &id).await, [r#"Owner removed "Run the tests""#, r#"Owner removed "Look""#]);
}

#[tokio::test]
async fn removing_a_done_fleet_derived_tells_the_agent_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let since = rig.fleet.all_done(None, ipc::AllDone { session_id: Some(id.clone()) }).await.unwrap().done_since.expect("when");
    let item = format!("done:{}", since.as_str());
    let before = said_by_the_person(&rig, &id).await;
    let gone = Arc::clone(&rig.fleet).dismiss_waiting(removal(&id, &item, Some("not yet"))).await.unwrap();
    assert!(gone.waiting_for.is_empty());
    assert_eq!(said_by_the_person(&rig, &id).await, before, "the reason has nobody to go to");
}
