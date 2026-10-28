//! The owner's notes on a pull request's lines, kept so a Job can resolve one after a restart.

use crate::tests::{open, TempDir};
use crate::KeptPullRequestNote;

fn note(id: &str, number: u32, body: &str) -> KeptPullRequestNote {
    KeptPullRequestNote {
        id: id.to_string(),
        repository: "mele-labs/armada".to_string(),
        number,
        path: "crates/fleet/src/lib.rs".to_string(),
        line: 12,
        side: "right".to_string(),
        body: body.to_string(),
        status: "open".to_string(),
        sent_to: None,
        sent_title: None,
        sent_at: None,
        created_at: "2026-10-27T09:00:00Z".to_string(),
    }
}

#[test]
fn a_note_is_listed_for_its_pull_request_edited_removed_and_marked_sent_to_a_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_pull_request_note(&note("n1", 7, "Too tight")).unwrap();
    store.keep_pull_request_note(&note("n2", 7, "Rename this")).unwrap();
    store.keep_pull_request_note(&note("n3", 8, "Another pull request")).unwrap();

    let listed = store.pull_request_notes_of("mele-labs/armada", 7).unwrap();
    assert_eq!(listed.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["n1", "n2"]);

    let edited = store.edit_pull_request_note_body("n1", "Too tight on the left").unwrap().unwrap();
    assert_eq!(edited.body, "Too tight on the left");

    store.send_pull_request_notes(&["n1".to_string()], "session-a", "Fix the rail", "2026-10-27T10:00:00Z").unwrap();
    let sent = store.pull_request_note("n1").unwrap().unwrap();
    assert_eq!(
        (sent.sent_to.as_deref(), sent.sent_title.as_deref(), sent.sent_at.as_deref()),
        (Some("session-a"), Some("Fix the rail"), Some("2026-10-27T10:00:00Z"))
    );
    assert_eq!(store.pull_request_note("n2").unwrap().unwrap().sent_to, None, "only the ids named");

    assert_eq!(store.finish_pull_request_note("n1").unwrap().unwrap().status, "done");

    store.forget_pull_request_note("n2").unwrap();
    store.forget_pull_request_note("n2").unwrap();
    assert_eq!(store.pull_request_notes_of("mele-labs/armada", 7).unwrap().len(), 1);
    assert_eq!(store.pull_request_note("n2").unwrap(), None);
}
