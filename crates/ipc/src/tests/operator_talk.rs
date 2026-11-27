//! A standing order on the wire and in the store is only ever the one form, a hold on paths.

use crate::operator_talk::{OperatorHoldUntil, OperatorOrderForm};
use crate::SessionId;

#[test]
fn a_hold_reads_back_as_itself_and_anything_else_is_no_form() {
    let form = OperatorOrderForm::HoldPaths { manifest_id: "01REPO".into(), paths: vec!["apps/cockpit".into()], job: SessionId::carried("01JOB"), until: OperatorHoldUntil::Pushes };
    assert_eq!(OperatorOrderForm::read(&form.kept()), Some(form));
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"be_careful","words":"please"}"#), None);
    assert_eq!(OperatorOrderForm::read("hold everything"), None);
}

#[test]
fn the_forms_that_went_with_the_levels_read_as_no_form() {
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"role_off","role":"file","manifest_id":"01REPO"}"#), None);
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"always_raise","what":"note"}"#), None);
}

#[test]
fn an_operator_hold_answered_before_its_paths_were_sent_still_reads() {
    let older = r#"{"id":4,"manifest_id":"01REPO","said":"Hold apps/cockpit until Build pushes","at":"2026-11-10T09:00:00.000Z"}"#;
    let hold = crate::decode::<crate::operator_page::OperatorHold>("a hold", older.as_bytes()).expect("reads");
    assert_eq!((hold.paths.is_empty(), hold.job, hold.until), (true, None, None));
}

#[test]
fn a_talk_carries_his_files_and_tags_and_one_sent_without_them_still_reads() {
    use crate::operator_talk::TalkToOperator;
    use crate::{SessionTag, SessionUpload, TagKind};
    let talk = TalkToOperator {
        words: "This toast is wrong".into(),
        about: Some("turn:31".into()),
        attachments: vec![SessionUpload { name: "shot.png".into(), media_type: "image/png".into(), data: "iVBORw0K".into() }],
        mentions: vec![SessionTag { kind: TagKind::Issue, id: "12".into(), title: "Toasts".into(), url: None }],
    };
    let sent = serde_json::to_string(&talk).expect("writes");
    assert_eq!(crate::decode::<TalkToOperator>("a talk", sent.as_bytes()).expect("reads"), talk);
    let older = crate::decode::<TalkToOperator>("a talk", br#"{"words":"Which Jobs are running?"}"#).expect("reads");
    assert_eq!((older.attachments.is_empty(), older.mentions.is_empty()), (true, true));
    let bare = serde_json::to_string(&TalkToOperator { words: "hi".into(), about: None, attachments: Vec::new(), mentions: Vec::new() }).expect("writes");
    assert_eq!(bare, r#"{"words":"hi"}"#, "an empty list is left off the wire");
}
