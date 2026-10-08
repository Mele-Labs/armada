//! A question the agent puts to the person through its question tool, held at
//! the permission door and answered from the session's thread. Since 23.71.
//!
//! The door gets the call because the CLI asks it about every call it has no
//! rule for, `AskUserQuestion` among them (spike 29). What the door returns on
//! an allow is the input the tool runs with, and the tool reads its `answers`
//! map from there.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AnswerSessionAsk, AskingToRun, HelmCallAnswer, QuestionAnswer, RunOrNot, SessionId};

use super::session_host::{eventually, rig_placing, Rig};

const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_1","input":{"questions":[
  {"question":"Which toppings?","header":"Toppings","multiSelect":true,
   "options":[{"label":"cheese","description":"Melted"},{"label":"ham","description":"Cured"}]},
  {"question":"Which size?","header":"Size","multiSelect":false,
   "options":[{"label":"S","description":"Small"},{"label":"L","description":"Large"}]}]}}"#;

fn asking() -> AskingToRun {
    ipc::decode("an ask", ASK.as_bytes()).expect("an ask")
}

fn chosen(question: &str, labels: &[&str]) -> QuestionAnswer {
    QuestionAnswer {
        question: question.into(),
        chosen: labels.iter().map(|one| one.to_string()).collect(),
    }
}

async fn waiting_on(rig: &Rig, id: &SessionId) -> ipc::HelmCallInFlight {
    let found = std::sync::Mutex::new(None);
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet)
            .get_session(id.clone())
            .await
            .unwrap()
            .session
            .hosted
            .unwrap()
            .asked;
        *found.lock().unwrap() = asked.clone();
        asked.is_some()
    })
    .await;
    let asked = found.lock().unwrap().take();
    asked.expect("an ask is waiting")
}

async fn answer(
    rig: &Rig,
    id: &SessionId,
    call: &str,
    answer: HelmCallAnswer,
    answers: Vec<QuestionAnswer>,
) -> Result<(), api::Refusal> {
    Arc::clone(&rig.fleet)
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call: call.into(),
            answer,
            note: None,
            answers,
        })
        .await
        .map(|_| ())
}

#[tokio::test]
async fn a_question_waits_for_the_person_in_every_mode_and_its_answers_go_into_the_tool() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking()).await });

    let ask = waiting_on(&rig, &id).await;
    assert_eq!(ask.tool, "AskUserQuestion");
    assert_eq!(ask.detail, "Which toppings?");
    assert_eq!(ask.questions.len(), 2);
    assert!(ask.questions[0].multi_select && !ask.questions[1].multi_select);
    assert_eq!(ask.questions[0].options[1].label, "ham");
    assert_eq!(
        ask.offers,
        vec![HelmCallAnswer::AllowOnce, HelmCallAnswer::Refuse],
        "a question is answered or skipped, and no rule is remembered"
    );

    // One question unanswered: refused, and the ask stands.
    let half = answer(
        &rig,
        &id,
        &ask.call,
        HelmCallAnswer::AllowOnce,
        vec![chosen("Which toppings?", &["cheese"])],
    )
    .await;
    assert!(half.is_err());
    assert!(!held.is_finished());

    // An answer to a question that was not asked is refused too.
    let stray = answer(
        &rig,
        &id,
        &ask.call,
        HelmCallAnswer::AllowOnce,
        vec![
            chosen("Which toppings?", &["cheese"]),
            chosen("Which colour?", &["red"]),
        ],
    )
    .await;
    assert!(stray.is_err());

    // Several labels join as the tool reads them; "Other" is the typed words.
    answer(
        &rig,
        &id,
        &ask.call,
        HelmCallAnswer::AllowOnce,
        vec![
            chosen("Which toppings?", &["cheese", "ham"]),
            chosen("Which size?", &["Medium, if you have it"]),
        ],
    )
    .await
    .unwrap();
    let RunOrNot::Allow { updated_input } = held.await.unwrap().unwrap() else {
        panic!("an answered question runs");
    };
    let said = |question: &str| {
        updated_input
            .pointer(&format!("/answers/{question}"))
            .and_then(|one| one.as_str())
            .map(str::to_string)
    };
    assert_eq!(said("Which toppings?").as_deref(), Some("cheese, ham"));
    assert_eq!(
        said("Which size?").as_deref(),
        Some("Medium, if you have it")
    );
    assert!(
        updated_input.get("questions").is_some(),
        "the questions are carried as they came"
    );
}

#[tokio::test]
async fn a_skipped_question_tells_the_agent_so() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking()).await });
    let ask = waiting_on(&rig, &id).await;
    answer(&rig, &id, &ask.call, HelmCallAnswer::Refuse, Vec::new())
        .await
        .unwrap();
    assert!(matches!(
        held.await.unwrap().unwrap(),
        RunOrNot::Deny { .. }
    ));
}
