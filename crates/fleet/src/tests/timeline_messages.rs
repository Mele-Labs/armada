//! `session.messaged`: one Job's message delivered to another is one entry naming both, and a
//! person's or Helm's message is none.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use ipc::{SessionId, SessionTurn, TimelineEvent, TimelineKind, TimelineSubject};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::address_of;

async fn messaged(rig: &Rig) -> Vec<TimelineEvent> {
    rig.fleet.timeline().settled().await;
    rig.fleet
        .fleet_timeline(ipc::TimelineAsk::default())
        .await
        .expect("a timeline")
        .events
        .into_iter()
        .filter(|one| one.kind == TimelineKind::SessionMessaged)
        .collect()
}

fn send_message(to: &SessionId, text: &str) -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: "SendMessage".into(),
        call: "toolu_1".into(),
        detail: CallDetail::of(&format!("to {}: {text}", address_of(to.as_str()))),
    }]
}

#[tokio::test]
async fn a_message_delivered_between_two_hosted_jobs_is_one_entry_naming_both() {
    let rig = rig_placing(None);
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let a = rig.start_titled(Some("Reader")).await;
    let b = rig.start().await;
    rig.send(&a, "hello a").await;
    rig.send(&b, "hello b").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "a ready");
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "b ready");
    eventually(|| async { rig.turn(&b).await == SessionTurn::Idle }).await;
    assert!(messaged(&rig).await.is_empty(), "a person's messages enter none");

    rig.stand_in.says(0, send_message(&b, "please review"));
    eventually(|| async { !messaged(&rig).await.is_empty() }).await;
    eventually(|| async { rig.rows(&b).await.len() == 3 }).await;

    let entries = messaged(&rig).await;
    assert_eq!(entries.len(), 1, "{entries:?}");
    let entry = &entries[0];
    assert_eq!(entry.subject, TimelineSubject::Session { id: a.clone() });
    assert_eq!(entry.to.as_ref(), Some(&b));
    assert_eq!(entry.title.as_deref(), Some("Reader"));
    assert_eq!(entry.says, "Messaged: please review");
}

#[tokio::test]
async fn a_message_from_a_person_or_from_helm_enters_no_message_entry() {
    let rig = rig_placing(None);
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let b = rig.start().await;
    rig.send(&b, "hello b").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "b ready");
    eventually(|| async { rig.turn(&b).await == SessionTurn::Idle }).await;

    let helm = rig.ask("check on the reader").await;
    assert!(helm.helm);
    rig.stand_in.init(1);
    rig.stand_in.says(1, send_message(&b, "status?"));
    // B's next turn is woken by Helm, which is past where the entry would have been written.
    rig.stand_in.init(0);
    eventually(|| async {
        matches!(rig.turn(&b).await, SessionTurn::Working { woken_by: Some(by) } if by.id == helm.id.as_str())
    })
    .await;

    assert!(messaged(&rig).await.is_empty());
}
