//! `dispatch_job`'s body and answers, and the ideas kept for later, as Bridge's mirror in
//! `packages/protocol/src/job-dispatch.ts` spells them.

use serde_json::json;

use crate::operator_board::{DispatchWay, OperatorDispatched};
use crate::{DispatchJob, DispatchKind, DispatchThen, JobDispatched, ParkedIdea, ParkedIdeas, RemoveParkedIdea, SessionId};

#[test]
fn a_dispatch_is_a_start_flat_with_the_kind_he_gave_and_what_follows_it() {
    let body = br#"{"manifest_id":"armada","request":"Keep the scrollback","kind":"bug","then":[{"message":{"text":"Also the colour."}}]}"#;
    let dispatch: DispatchJob = crate::decode("a dispatch", body).unwrap();

    assert_eq!(dispatch.kind, Some(DispatchKind::Bug));
    assert_eq!(dispatch.start.request.as_deref(), Some("Keep the scrollback"));
    assert!(matches!(&dispatch.then[..], [DispatchThen::Message(message)] if message.text == "Also the colour."));
    let written = serde_json::to_value(&dispatch).unwrap();
    assert_eq!(written["kind"], json!("bug"));
    assert_eq!(written["then"], json!([{"message": {"text": "Also the colour."}}]));
}

#[test]
fn a_kind_left_out_is_written_as_nothing_and_every_kind_has_its_word() {
    let dispatch: DispatchJob = crate::decode("a dispatch", br#"{"manifest_id":"armada","request":"x"}"#).unwrap();
    assert_eq!(dispatch.kind, None);
    assert!(serde_json::to_value(&dispatch).unwrap().get("kind").is_none());
    assert!(serde_json::to_value(&dispatch).unwrap().get("then").is_none());

    for (kind, word) in [(DispatchKind::Idea, "idea"), (DispatchKind::Feature, "feature"), (DispatchKind::Bug, "bug")] {
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(word));
        assert_eq!(kind.as_wire(), word);
    }
    assert!(crate::decode::<DispatchJob>("a dispatch", br#"{"manifest_id":"armada","kind":"chore"}"#).is_err());
}

#[test]
fn the_answer_writes_only_the_one_it_holds() {
    assert_eq!(serde_json::to_value(JobDispatched { session: None, operator: Some("start:dispatch:01a".into()), parked: None }).unwrap(), json!({"operator": "start:dispatch:01a"}));
    assert_eq!(serde_json::to_value(JobDispatched { session: None, operator: None, parked: Some("start:dispatch:01b".into()) }).unwrap(), json!({"parked": "start:dispatch:01b"}));
}

#[test]
fn the_event_names_what_became_of_the_dispatch_in_a_word() {
    let parked = OperatorDispatched { dispatch_id: "start:dispatch:01b".into(), manifest_id: "armada".into(), way: DispatchWay::Parked, session_id: None, title: Some("Keep the scrollback".into()) };
    assert_eq!(serde_json::to_value(&parked).unwrap(), json!({"dispatch_id": "start:dispatch:01b", "manifest_id": "armada", "way": "parked", "title": "Keep the scrollback"}));
    let started = OperatorDispatched { way: DispatchWay::Started, session_id: Some(SessionId::carried("s1")), ..parked };
    assert_eq!(serde_json::to_value(&started).unwrap()["way"], json!("started"));
    for (way, word) in [(DispatchWay::Handed, "handed"), (DispatchWay::Asked, "asked")] {
        assert_eq!(serde_json::to_value(way).unwrap(), json!(word));
    }
}

#[test]
fn the_ideas_kept_for_later_round_trip_with_the_dispatch_whole() {
    let dispatch: DispatchJob = crate::decode("a dispatch", br#"{"manifest_id":"armada","request":"What if the rail showed time","kind":"idea"}"#).unwrap();
    let ideas = ParkedIdeas { ideas: vec![ParkedIdea { id: "start:dispatch:01b".into(), dispatch, at: "2026-11-12T09:00:00.000Z".into() }] };

    let written = serde_json::to_value(&ideas).unwrap();
    assert_eq!(written["ideas"][0]["id"], json!("start:dispatch:01b"));
    assert_eq!(written["ideas"][0]["at"], json!("2026-11-12T09:00:00.000Z"));
    assert_eq!(written["ideas"][0]["dispatch"]["kind"], json!("idea"));
    assert_eq!(serde_json::from_value::<ParkedIdeas>(written).unwrap(), ideas);
    assert_eq!(serde_json::to_value(RemoveParkedIdea { id: "start:dispatch:01b".into() }).unwrap(), json!({"id": "start:dispatch:01b"}));
}
