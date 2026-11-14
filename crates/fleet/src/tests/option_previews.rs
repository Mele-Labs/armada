//! The claim, written before the code: **an agent's question that waits on a person as an `ask:<call>` item carries
//! each option's description and its preview, and leaves each out where the agent sent none.**

use std::sync::Arc;

use api::{HostedSessions, Sessions};

use super::document_answers::record_of;
use super::session_host::{eventually, rig_placing};
use super::session_plan::running;

const WITH_PREVIEWS: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t1","input":{"questions":[
  {"question":"Which look?","header":"Look","multiSelect":false,"options":[
    {"label":"Amber edge","description":"Matches the other marks.","preview":".chip {\n  border-color: amber;\n}"},
    {"label":"Grey","description":""}]}]}}"#;

#[tokio::test]
async fn an_asked_option_with_a_description_and_a_preview_waits_carrying_both() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let body = ipc::decode("an ask", WITH_PREVIEWS.as_bytes()).expect("an ask");
    let held = tokio::spawn(async move { fleet.session_permission(&session, body).await });
    eventually(|| async { Arc::clone(&rig.fleet).get_session(id.clone()).await.is_ok_and(|got| got.session.hosted.is_some_and(|hosted| hosted.asked.is_some())) }).await;

    let record = record_of(&rig, &id).await;
    let item = record.waiting_for.iter().find(|one| one.id.starts_with("ask:")).expect("the question waits");
    assert_eq!(item.options[0].label, "Amber edge");
    assert_eq!(item.options[0].description.as_deref(), Some("Matches the other marks."));
    assert_eq!(item.options[0].preview.as_deref(), Some(".chip {\n  border-color: amber;\n}"));
    assert_eq!((item.options[1].description.as_deref(), item.options[1].preview.as_deref()), (None, None), "an option that sent neither carries neither");
    held.abort();
}
