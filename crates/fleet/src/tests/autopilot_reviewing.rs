//! What Autopilot answered, as the person reviews it: listed until kept, kept, or corrected by a
//! message to the Job, and a correction read by the helper the next time it decides.

use api::{Autopilots, HostedSessions};
use ipc::{AutopilotDecision, OverrideAutopilotDecision, ReviewAutopilotDecision};

use super::autopilot_decisions::{item, keep, rig_calling, started, told, SURE_OF_THE_FIRST};

#[tokio::test]
async fn an_answer_is_listed_for_review_until_the_person_keeps_it() {
    let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;
    rig.fleet.decision_pass().await.expect("passed");

    let listed = rig.fleet.get_autopilot_decisions().await.expect("listed").to_review;
    let [one] = &listed[..] else { panic!("one answer: {listed:?}") };
    assert_eq!(one.id, format!("{}:a", id.as_str()));
    assert_eq!(one.manifest_id, rig.manifest);
    assert_eq!(one.session_id.as_ref(), Some(&id));
    assert_eq!((one.kind, one.asked.as_str(), one.chose.as_str()), (AutopilotDecision::Recommended, "Which clock?", "Frozen (Recommended)"));
    assert_eq!(one.because, "it is the one the agent marked");

    let ghost = rig.fleet.review_autopilot_decision(ReviewAutopilotDecision { id: "nope".into() }).await;
    assert!(matches!(ghost, Err(api::Refusal::IllegalMove(_))));
    let kept = rig.fleet.review_autopilot_decision(ReviewAutopilotDecision { id: one.id.clone() }).await.expect("kept");
    assert!(kept.to_review.is_empty());
    assert!(rig.fleet.get_autopilot_decisions().await.expect("listed").to_review.is_empty());
}

#[tokio::test]
async fn a_correction_goes_to_the_job_and_is_read_by_the_helper_next_time() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;
    rig.fleet.decision_pass().await.expect("passed");
    let row = rig.fleet.get_autopilot_decisions().await.expect("listed").to_review[0].id.clone();

    let blank = std::sync::Arc::clone(&rig.fleet)
        .override_autopilot_decision(OverrideAutopilotDecision { id: row.clone(), text: "  ".into() })
        .await;
    assert!(matches!(blank, Err(api::Refusal::Unacceptable(_))));
    let ghost = std::sync::Arc::clone(&rig.fleet)
        .override_autopilot_decision(OverrideAutopilotDecision { id: "nope".into(), text: "x".into() })
        .await;
    assert!(matches!(ghost, Err(api::Refusal::IllegalMove(_))));

    let state = std::sync::Arc::clone(&rig.fleet)
        .override_autopilot_decision(OverrideAutopilotDecision { id: row.clone(), text: "Use the fake one".into() })
        .await
        .expect("corrected");

    assert!(state.to_review.is_empty(), "a corrected answer is reviewed");
    assert_eq!(told(&rig, &id).await.last().unwrap(), "Re: Which clock?\nUse the fake one");
    let kept = rig.fleet.store().lock().await.autopilot_decision(&row).unwrap().unwrap();
    assert_eq!(kept.corrected.as_deref(), Some("Use the fake one"));

    keep(&rig, &id, &[item("b", "Which port?", &["80 (Recommended)", "8080"])]).await;
    rig.fleet.decision_pass().await.expect("passed");
    let asked = judge.asked();
    let port = asked.iter().find(|one| one.contains("Which port?")).expect("the port was put to the helper");
    assert!(port.contains("corrected: Use the fake one"), "the helper reads the correction");
}
