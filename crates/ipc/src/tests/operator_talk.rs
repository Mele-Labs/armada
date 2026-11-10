//! A standing order on the wire and in the store is only ever one of its forms.

use crate::operator_positions::OperatorRole;
use crate::operator_talk::{OperatorHoldUntil, OperatorOrderForm};
use crate::SessionId;

#[test]
fn each_form_reads_back_as_itself_and_anything_else_is_no_form() {
    for form in [
        OperatorOrderForm::RoleOff { role: OperatorRole::Answer, manifest_id: "01REPO".into(), today: true },
        OperatorOrderForm::HoldPaths { manifest_id: "01REPO".into(), paths: vec!["apps/cockpit".into()], job: SessionId::carried("01JOB"), until: OperatorHoldUntil::Pushes },
        OperatorOrderForm::AlwaysRaise { what: "question".into() },
    ] {
        assert_eq!(OperatorOrderForm::read(&form.kept()), Some(form));
    }
    assert_eq!(OperatorOrderForm::read(r#"{"kind":"be_careful","words":"please"}"#), None);
    assert_eq!(OperatorOrderForm::read("hold everything"), None);
}

#[test]
fn a_role_off_says_until_removed_unless_it_says_today() {
    let form = OperatorOrderForm::read(r#"{"kind":"role_off","role":"file","manifest_id":"01REPO"}"#).expect("a form");
    assert_eq!(form, OperatorOrderForm::RoleOff { role: OperatorRole::File, manifest_id: "01REPO".into(), today: false });
}

#[test]
fn a_line_names_a_raise_and_a_briefing_only_where_it_is_one_and_a_line_written_before_them_reads() {
    use crate::operator_talk::{BriefedChange, BriefedKind, BriefedPull, BriefedWaiting, OperatorBriefing, OperatorRaise, OperatorTalkLine};
    use crate::{Instant, OperatorRead, WaitingItem, WaitingSource};

    let plain: OperatorTalkLine = serde_json::from_str(r#"{"from_person":true,"text":"why?","at":"2026-11-10T09:00:00.000Z"}"#).expect("read");
    assert!(plain.raise.is_none() && plain.briefing.is_none());
    let wire = serde_json::to_value(&plain).expect("written");
    assert!(wire.get("raise").is_none() && wire.get("briefing").is_none());

    let item: WaitingItem = serde_json::from_str(r#"{"id":"plan","text":"Approve the plan?","since":"2026-11-10T08:00:00.000Z","source":"agent","options":[{"label":"Yes"}]}"#).expect("an item");
    let raise = OperatorRaise {
        session_id: Some(SessionId::carried("01JOB")),
        job: Some("Build the pane".into()),
        item,
        read: Some(OperatorRead { read: "It covers the pane only.".into(), pick: "Yes".into() }),
        settled_at: Some(Instant::carried("2026-11-10T09:30:00.000Z")),
    };
    assert_eq!(raise.item.source, WaitingSource::Agent);
    assert_eq!(OperatorRaise::read(&raise.kept()), Some(raise.clone()));
    let briefing = OperatorBriefing {
        since: vec![BriefedChange { kind: BriefedKind::Merged, session_id: SessionId::carried("01JOB"), job: "Build the pane".into(), pull: Some(BriefedPull { number: "3398".into(), title: "Take the views".into() }), said: None }],
        waiting: vec![BriefedWaiting { session_id: None, item_id: "suggestion:7".into() }],
        on: Vec::new(),
        top: None,
    };
    assert_eq!(OperatorBriefing::read(&briefing.kept()), Some(briefing.clone()));
    let line = OperatorTalkLine { from_person: false, text: "**Good morning.**".into(), at: "2026-11-10T09:00:00.000Z".into(), about: None, raise: Some(raise), briefing: Some(briefing) };
    let wire = serde_json::to_value(&line).expect("written");
    assert_eq!(wire["briefing"]["since"][0]["kind"], "merged");
    assert!(wire["briefing"].get("on").is_none() && wire["briefing"].get("top").is_none(), "an empty part is left out: {wire}");
    assert_eq!(wire["raise"]["settled_at"], "2026-11-10T09:30:00.000Z");
    assert_eq!(serde_json::from_value::<OperatorTalkLine>(wire).expect("read"), line);
}
