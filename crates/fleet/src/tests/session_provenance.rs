//! The claim, written before the code: **each version of a Job's artifacts records the person's latest
//! message and the plan task in progress when it was written, and Fleet works both out itself.**
//! `docs/concepts/session/what-is-kept.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::ReportedTaskStatus::{Completed, InProgress};
use ipc::{AttachmentVersion, CloseSession, SessionId, SessionRow, ShowSketch, StartBase, StartCondition};

use super::session_host::{rig_placing, Rig};
use super::session_after::{brief_after, BRIEF};
use super::session_plan::{first, recorded, task, tasks};

fn drawn(id: &SessionId, second: &str) -> ShowSketch {
    let scene = format!(
        r#"{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Store"}},{{"id":"b","kind":"box","x":300,"y":0,"title":"{second}"}}],"edges":[]}}"#
    );
    ipc::decode("a sketch to show", format!(r#"{{"title":"Flow","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes())
        .unwrap()
}

async fn versions(rig: &Rig, id: &SessionId) -> Vec<AttachmentVersion> {
    rig.fleet.get_attachment_history(id.clone()).await.expect("a history").versions
}

/// The thread row ids of the person's messages, oldest first.
async fn his(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .iter()
        .filter(|row| matches!(row, SessionRow::Message { from: ipc::SessionVoice::You, .. }))
        .map(|row| row.id().to_string())
        .collect()
}

#[tokio::test]
async fn a_version_follows_the_message_the_person_sent_last() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    rig.send(&id, "draw the flow").await;
    rig.fleet.show_sketch(None, drawn(&id, "Fleet")).await.unwrap();
    rig.send(&id, "now with the daemon").await;
    rig.fleet.show_sketch(None, drawn(&id, "Daemon")).await.unwrap();

    let messages = his(&rig, &id).await;
    assert_eq!(messages.len(), 2, "{messages:?}");
    let read: Vec<_> = versions(&rig, &id).await.into_iter().map(|one| one.from_message).collect();
    assert_eq!(read, [Some(messages[0].clone()), Some(messages[1].clone())]);
}

#[tokio::test]
async fn a_version_follows_the_task_that_began_last_and_none_once_nothing_is_in_progress() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    recorded(&rig, first(&id)).await;
    let (tables, migration) = ("Add the tables", "Add the migration");

    tasks(&rig, &id, vec![task(migration, InProgress)]).await;
    rig.fleet.show_sketch(None, drawn(&id, "A")).await.unwrap();
    // Earlier in the plan, but begun later.
    tasks(&rig, &id, vec![task(migration, InProgress), task(tables, InProgress)]).await;
    rig.fleet.show_sketch(None, drawn(&id, "B")).await.unwrap();
    tasks(&rig, &id, vec![task(migration, Completed), task(tables, InProgress)]).await;
    rig.fleet.show_sketch(None, drawn(&id, "C")).await.unwrap();
    tasks(&rig, &id, vec![task(migration, Completed), task(tables, Completed)]).await;
    rig.fleet.show_sketch(None, drawn(&id, "D")).await.unwrap();

    let read: Vec<_> = versions(&rig, &id).await.into_iter().map(|one| one.from_task).collect();
    let some = |task: &str| Some(task.to_string());
    assert_eq!(read, [some("t2"), some("t1"), some("t1"), None]);
}

#[tokio::test]
async fn the_brief_a_waiting_job_is_sent_when_its_wait_is_met_is_what_its_first_artifacts_follow() {
    let rig = super::agent_starts::rig(None);
    let agent = rig.a_waited_on_agent().await;
    let waiting = Arc::clone(&rig.fleet)
        .start_session_reading(brief_after(&rig, &agent, StartCondition::Ends, StartBase::Main))
        .await
        .expect("made to wait");
    rig.fleet.close_session(CloseSession { session_id: agent.clone() }).await.expect("closed");
    rig.eventually(&waiting.id, "the start", |one| one.waits.is_none()).await;
    let brief = rig
        .thread_has(&waiting.id, "the brief", |row| matches!(row, SessionRow::Message { text, .. } if text.ends_with(BRIEF)))
        .await;

    rig.fleet.show_sketch(None, drawn(&waiting.id, "Fleet")).await.unwrap();

    let history = rig.fleet.get_attachment_history(waiting.id.clone()).await.expect("a history");
    let read: Vec<_> = history.versions.into_iter().map(|one| one.from_message).collect();
    assert_eq!(read, [Some(brief.id().to_string())]);
}
