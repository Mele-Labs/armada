//! The claim, written before the code: **an action a page declares with `<armada-action>` runs one of three acts at
//! his press, read off the kept revision: a message sends the Job what he pressed, the row's text and his words; a
//! start starts a Job at once on the action's brief and tells the page's Job; an answer answers the Job's open ask
//! with the option. Every press is kept on the page. An act off the list is named to the Job when the page is kept
//! and refused at the press, and an action the revision does not hold is refused.**
//! `crates/ipc/operations/press_page_action.toml`.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{PageAct, PagePress, PressDid, PressPageAction, SessionId, SessionRow, SessionVoice, SetWaitingFor, WaitingInput, WaitingOption};

use super::session_host::{eventually, rig_placing, Rig};

/// A Job with its process up, and where the stand-in holds it.
async fn a_job(rig: &Rig) -> (SessionId, usize) {
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    let at = rig.stand_in.starts().iter().position(|one| one.session == id.as_str()).unwrap();
    rig.stand_in.init(at);
    (id, at)
}

/// A review page whose one row holds `actions`.
fn review(actions: &str) -> String {
    format!(
        "<html><head><title>Flaky tests</title></head><body><h1>Flaky tests</h1><table>\
         <tr><th>Test</th><th></th></tr><tr><td>lease_clock drifts</td><td>{actions}</td></tr></table></body></html>"
    )
}

/// The Job writes `html` as its page, and Fleet keeps it as revision 1.
async fn page_holding(rig: &Rig, id: &SessionId, at: usize, html: &str) -> String {
    let file = std::path::Path::new(&rig.root).join(".armada/documents/flaky.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, html).unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +1")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async { rig.fleet.get_job_artifacts(id.clone()).await.unwrap().iter().any(|one| one.id == path) }).await;
    path
}

fn pressing(id: &SessionId, path: &str, action: &str, words: Option<&str>) -> PressPageAction {
    PressPageAction { session_id: id.clone(), artifact: path.into(), revision: 1, action: action.into(), words: words.map(str::to_string) }
}

async fn pressed(rig: &Rig, press: PressPageAction) -> Result<PagePress, Refusal> {
    Arc::clone(&rig.fleet).press_page_action(press).await
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

async fn presses(rig: &Rig, id: &SessionId, path: &str) -> Vec<PagePress> {
    rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap().presses
}

#[tokio::test]
async fn a_message_press_sends_the_job_the_label_the_row_and_his_words_and_is_kept_on_the_page() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &review(r#"<armada-action id="discuss-1" act="message" label="Discuss" item="row-1"></armada-action>"#)).await;

    let press = pressed(&rig, pressing(&id, &path, "discuss-1", Some("Is it the clock?"))).await.expect("pressed");

    assert_eq!((press.act, press.did.clone(), press.item.as_deref()), (PageAct::Message, PressDid::Sent, Some("row-1")));
    let told = sent(&rig.rows(&id).await).pop();
    assert_eq!(told.as_deref(), Some("He pressed \"Discuss\" on row-1 in \"Flaky tests\".\nlease_clock drifts\nIs it the clock?"));
    assert_eq!(presses(&rig, &id, &path).await, [press]);
}

#[tokio::test]
async fn a_start_press_starts_a_job_at_once_on_the_composed_brief_and_tells_the_pages_job() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let action = r#"<armada-action id="look-1" act="start" label="Investigate" item="row-1" brief="Find why it drifts."></armada-action>"#;
    let path = page_holding(&rig, &id, at, &review(action)).await;

    let press = pressed(&rig, pressing(&id, &path, "look-1", Some("Start with the clock."))).await.expect("pressed");

    let PressDid::Started { job } = press.did.clone() else { panic!("a Job started: {:?}", press.did) };
    assert_ne!(job, id);
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the new Job").session;
    assert!(record.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).is_none(), "his start runs at once, not held");
    let brief = "Find why it drifts.\n\nFrom \"Flaky tests\", row-1: lease_clock drifts\n\nStart with the clock.";
    eventually(|| async { sent(&rig.rows(&job).await).first().map(String::as_str) == Some(brief) }).await;
    let short: String = job.as_str().chars().take(8).collect();
    assert_eq!(sent(&rig.rows(&id).await).pop(), Some(format!("He started Job {short} from \"Investigate\" on row-1.")));
    assert_eq!(presses(&rig, &id, &path).await, [press]);
}

#[tokio::test]
async fn an_answer_press_answers_the_jobs_open_ask_with_the_option() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let option = |label: &str| WaitingOption { label: label.into(), description: None, picture: None };
    let question = WaitingInput {
        id: "q1".into(),
        text: "Quarantine lease_clock?".into(),
        act: None,
        options: vec![option("Quarantine"), option("Keep it")],
        approves: false,
        steps: Vec::new(),
    };
    let record = rig.fleet.waiting_for(None, SetWaitingFor { items: vec![question], session_id: Some(id.clone()) }).await.expect("set");
    let ask = record.waiting_for.iter().find(|one| one.text.starts_with("Quarantine")).expect("the ask waits").id.clone();
    let action = format!(r#"<armada-action id="keep-1" act="answer" label="Keep" item="row-1" ask="{ask}" option="Keep it"></armada-action>"#);
    let path = page_holding(&rig, &id, at, &review(&action)).await;

    let press = pressed(&rig, pressing(&id, &path, "keep-1", None)).await.expect("pressed");

    assert_eq!(press.did, PressDid::Answered { item: ask.clone() });
    let waiting = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.waiting_for;
    assert!(waiting.iter().all(|one| one.id != ask), "the ask is answered: {waiting:?}");
    // Said as his own pick of the option says it.
    let told = sent(&rig.rows(&id).await).pop().unwrap_or_default();
    assert!(told.starts_with("Re: Quarantine lease_clock?\nKeep it\n"), "{told}");
    // Answered, it is no longer open.
    let again = pressed(&rig, pressing(&id, &path, "keep-1", None)).await.expect_err("refused");
    assert_eq!(again.error().code, "fleet.page_ask_not_open");
}

#[tokio::test]
async fn an_act_off_the_list_is_named_when_the_page_is_kept_and_refused_at_the_press() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &review(r#"<armada-action id="merge-1" act="merge" label="Merge" item="row-1"></armada-action>"#)).await;

    let told = || async {
        rig.rows(&id).await.into_iter().find_map(|row| match row {
            SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" => Some(text),
            _ => None,
        })
    };
    eventually(|| async { told().await.is_some() }).await;
    let told = told().await.unwrap();
    assert!(told.starts_with(&format!("In the page {path}, action \"Merge\" will not run: act \"merge\" is not one a page runs.")), "{told}");

    let refused = pressed(&rig, pressing(&id, &path, "merge-1", None)).await.expect_err("refused");
    assert_eq!(refused.error().code, "fleet.page_act_refused");
    assert_eq!(
        refused.error().message,
        "a page runs start, answer and message only. Anything that lands, merges, deletes or stops goes to the Job as a message"
    );
    assert!(presses(&rig, &id, &path).await.is_empty());
}

#[tokio::test]
async fn an_action_the_pressed_revision_does_not_hold_is_refused() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, &review(r#"<armada-action id="discuss-1" act="message" label="Discuss" item="row-1"></armada-action>"#)).await;
    let before = sent(&rig.rows(&id).await).len();

    let refused = pressed(&rig, pressing(&id, &path, "discuss-9", None)).await.expect_err("refused");

    assert_eq!(refused.error().code, "fleet.page_action_unknown");
    assert!(refused.error().message.contains("discuss-1"), "names the ones it holds: {}", refused.error().message);
    assert_eq!(sent(&rig.rows(&id).await).len(), before, "nothing sent");
    assert!(presses(&rig, &id, &path).await.is_empty());
}
