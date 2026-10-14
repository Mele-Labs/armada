//! The record a person carries when a Helm answer is bad. `#1367`.
//!
//! **The Session is real and the process behind it is the stand-in**, so what the record holds is
//! what a Session's own stream became on its thread, not a hand-built list.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent, Speaker};
use api::Conversations;
use ipc::{
    Cursor, EventTally, EventsSince, HelmDebugSaid, ManifestId, ThoughtKind, ThoughtSent,
    ThoughtTarget,
};

use super::session_host::{eventually, rig_placing};

#[tokio::test]
async fn the_record_holds_the_note_the_roster_and_the_thread() {
    let rig = rig_placing(None);
    let helm = rig.ask("why did you refuse that").await;
    rig.stand_in.init(0);
    rig.stand_in.says(
        0,
        vec![
            DroneEvent::Called {
                tool: "Bash".to_string(),
                call: "toolu_1".to_string(),
                detail: CallDetail::of("pnpm test"),
            },
            DroneEvent::Said {
                text: "I cannot answer that.".to_string(),
                by: Speaker::Drone,
            },
            DroneEvent::Ended {
                turns: 3,
                cost_micros: 24_300,
                refusals: 0,
            },
        ],
    );
    eventually(|| async { rig.helm_rows().await.len() == 3 }).await;
    Arc::clone(&rig.fleet)
        .record_thought_sent(
            ThoughtSent {
                text: "back off longer".to_string(),
                target: ThoughtTarget {
                    kind: ThoughtKind::Job,
                    id: "01JOB".to_string(),
                    title: "Retry failed charges".to_string(),
                    manifest_id: None,
                    slot: None,
                },
                attachments: Vec::new(),
                mentions: Vec::new(),
            },
            None,
        )
        .await
        .expect("recorded");

    let record = rig
        .fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record for the repository Fleet started in");

    assert!(
        record.note.starts_with("You are Helm, docked in Armada"),
        "the note the next process is started with: {}",
        record.note
    );
    assert!(
        record.tools.contains(&"list_jobs".to_string())
            && record.tools.contains(&"tell_session".to_string()),
        "the door's roster, by name: {:?}",
        record.tools
    );
    assert!(!record.tools.contains(&"undo_run".to_string()));
    assert_eq!(record.model, "default", "no model was chosen");
    assert_eq!(record.session.as_deref(), Some(helm.id.as_str()));
    assert_eq!(record.cut, 0);

    let said: Vec<&HelmDebugSaid> = record.thread.iter().map(|line| &line.said).collect();
    assert!(
        matches!(said[0], HelmDebugSaid::Asked { text } if text.text == "why did you refuse that"),
        "what was asked, first: {said:?}"
    );
    assert!(
        said.iter()
            .any(|one| matches!(one, HelmDebugSaid::Called { tool, detail } if tool == "Bash" && detail.contains("pnpm test"))),
        "the call it made: {said:?}"
    );
    assert!(
        said.iter().any(|one| matches!(one, HelmDebugSaid::Said { text } if text.text == "I cannot answer that.")),
        "what it said: {said:?}"
    );
    assert!(
        said.iter().any(|one| matches!(one, HelmDebugSaid::Sent { to, .. } if to == "Retry failed charges")),
        "where words went: {said:?}"
    );
}

#[tokio::test]
async fn a_record_is_taken_with_no_session_live() {
    let rig = rig_placing(None);
    let record = rig.fleet.get_helm_debug_info(None).await.expect("a record");
    assert!(record.session.is_none() && record.thread.is_empty());
    assert!(record.note.contains("docked in Armada"), "the note it would be started with");
}

#[tokio::test]
async fn the_record_reports_the_poll_the_session_was_answered() {
    let rig = rig_placing(None);
    rig.ask("hello").await;
    let manifest_id = rig
        .fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record")
        .manifest_id;

    assert!(
        rig.fleet.get_helm_debug_info(None).await.expect("a record").polled.is_none(),
        "a session that has not polled claims no window"
    );

    rig.fleet
        .helm_polled(
            ManifestId::carried(manifest_id.as_str()),
            EventsSince {
                from: Cursor::at(41),
                upto: Cursor::at(58),
                kinds: vec![EventTally {
                    kind: "job.state_changed".to_string(),
                    count: 9,
                }],
                missed: None,
            },
        )
        .await;

    let polled = rig
        .fleet
        .get_helm_debug_info(None)
        .await
        .expect("a record")
        .polled
        .expect("the window its last poll was answered");
    assert_eq!(polled.from.position(), 41);
    assert_eq!(polled.upto.position(), 58);
    assert_eq!(polled.kinds[0].count, 9);
}
