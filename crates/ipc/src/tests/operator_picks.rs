//! A card he pressed carries its pick, the Operator's steps and why it came back, and a page served before
//! picks reads as nothing picked and nothing worked on.

use crate::operator_page::OperatorPage;
use crate::{Instant, OperatorCard, OperatorCardKind, OperatorCardPicked, OperatorCardReturned, OperatorCardStep, ReportCard};

#[test]
fn a_picked_card_reads_back_as_itself() {
    let card = OperatorCard {
        kind: OperatorCardKind::Deal,
        talk_of: None,
        sessions: Vec::new(),
        picked: Some(OperatorCardPicked { option: "Start it".into(), at: Instant::carried("2026-11-14T09:00:00.000Z") }),
        steps: vec![OperatorCardStep { at: Instant::carried("2026-11-14T09:02:00.000Z"), said: "Started Build the pane".into() }],
        returned: Some(OperatorCardReturned { why: "Nothing settled in 15 min".into(), at: Instant::carried("2026-11-14T09:15:00.000Z") }),
    };
    let wire = serde_json::to_value(&card).expect("encodes");
    assert_eq!(wire["picked"], serde_json::json!({ "option": "Start it", "at": "2026-11-14T09:00:00.000Z" }));
    assert_eq!(wire["steps"], serde_json::json!([{ "at": "2026-11-14T09:02:00.000Z", "said": "Started Build the pane" }]));
    assert_eq!(wire["returned"], serde_json::json!({ "why": "Nothing settled in 15 min", "at": "2026-11-14T09:15:00.000Z" }));
    assert_eq!(crate::decode::<OperatorCard>("a card", wire.to_string().as_bytes()).expect("decodes"), card);
}

#[test]
fn a_card_and_a_page_from_before_picks_read_as_nothing_picked() {
    let card = crate::decode::<OperatorCard>("a card", br#"{"kind":"raise"}"#).expect("decodes");
    assert_eq!((&card.picked, card.steps.len(), &card.returned), (&None, 0, &None));
    let unpicked = serde_json::to_value(&card).expect("encodes");
    assert!(unpicked.get("picked").is_none() && unpicked.get("returned").is_none(), "absent, never null: {unpicked}");
    let page = crate::decode::<OperatorPage>("a page", br#"{"on":true,"turn":[],"digest":{}}"#).expect("decodes");
    assert!(page.working.is_empty());
}

#[test]
fn a_report_names_one_of_step_and_failed_and_leaves_the_other_out() {
    let report = ReportCard { card: "turn:4".into(), step: Some("Opened #41".into()), failed: None };
    assert_eq!(serde_json::to_value(&report).expect("encodes"), serde_json::json!({ "card": "turn:4", "step": "Opened #41" }));
}
