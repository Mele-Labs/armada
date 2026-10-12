//! What #2105 put on the wire for a refusal the Judges agreed on: how many times a step went back,
//! of how many, and a member's risk reason. Each is absent where there is nothing to say.

use crate::{Judged, JudgesSentBack};

#[test]
fn a_step_never_sent_back_carries_nothing_and_one_sent_back_carries_the_limit() {
    assert_eq!(JudgesSentBack::of(0), None);
    assert_eq!(
        JudgesSentBack::of(1),
        Some(JudgesSentBack {
            times: 1,
            of: core_model::AGREED_RETRIES
        })
    );
    let wire = crate::encode(&JudgesSentBack::of(2).expect("sent back")).expect("encodes");
    assert_eq!(wire, r#"{"times":2,"of":2}"#);
}

#[test]
fn a_members_risk_reason_crosses_on_its_judgment_and_is_left_out_where_it_gave_none() {
    let judgment = |risky: Option<&str>| core_model::Judgment {
        criterion_id: core_model::CriterionId::new("c1"),
        member: Some(1),
        verdict: core_model::JudgeVerdict::NotMet,
        expected: Some("the bound narrowed".to_string()),
        produced: Some("the bound widened".to_string()),
        consequence: Some("a reader gets one row too many".to_string()),
        brief_path: None,
        cited: None,
        given: None,
        risky: risky.map(str::to_string),
    };

    let risky = Judged::of(1, &judgment(Some("it drops the only copy of the log")));
    let redo = Judged::of(1, &judgment(None));

    assert_eq!(
        risky.risky.as_deref(),
        Some("it drops the only copy of the log")
    );
    let encoded = |judged: &Judged| crate::encode(judged).expect("encodes");
    assert!(encoded(&risky).contains(r#""risky":"it drops the only copy of the log""#));
    assert!(!encoded(&redo).contains("risky"), "absent, never null");
}
