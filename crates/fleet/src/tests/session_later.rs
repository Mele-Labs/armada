//! The follow-ups an agent keeps for itself: held across its replies, on the row, settled by the
//! agent or by the person, and a hold on `all_done` while any is open. The sentence readers that find
//! them are not in force, so these keep sentences the way the reader would hand them over.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AllDone, SessionId, SessionRecord};

use super::session_host::{rig_placing, Rig};

const SLOW: &str = "I will file the slow fixture load once this lands.";
const DOCS: &str = "I will tidy the docs after the merge.";
const FIRST: &str = "2026-10-21T10:00:00.000Z";
const SECOND: &str = "2026-10-21T10:05:00.000Z";

async fn read(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

fn said(sentences: &[&str]) -> Vec<String> {
    sentences.iter().map(|one| one.to_string()).collect()
}

#[tokio::test]
async fn a_followup_survives_the_next_reply_and_the_same_sentence_is_one_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert!(read(&rig, &id).await.later.is_empty());

    assert!(rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), FIRST).await);
    // The next reply says a different one, and the first again.
    assert!(rig.fleet.later_kept(id.as_str(), &said(&[DOCS, SLOW]), SECOND).await);
    assert!(!rig.fleet.later_kept(id.as_str(), &said(&[SLOW]), SECOND).await, "nothing new");

    let later: Vec<(String, String, String)> =
        read(&rig, &id).await.later.into_iter().map(|one| (one.id, one.sentence, one.said_at.as_str().to_string())).collect();
    assert_eq!(
        later,
        [
            ("later-1".to_string(), SLOW.to_string(), FIRST.to_string()),
            ("later-2".to_string(), DOCS.to_string(), SECOND.to_string()),
        ]
    );
}
