//! Paired phones: kept across a reopen, and unpairing takes the push subscription too.

use crate::tests::{open, TempDir};
use crate::Device;

fn phone(id: &str) -> Device {
    Device {
        id: id.into(),
        name: "Nick's phone".into(),
        public_key: vec![1, 2, 3],
        created_at: 100,
        last_seen_at: None,
    }
}

#[test]
fn a_device_survives_a_reopen_and_remembers_when_it_was_seen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_device(&phone("a")).expect("added");
    store.touch_device("a", 250).expect("touched");
    drop(store);
    let store = open(&dir);
    let kept = store.paired_device("a").expect("reads").expect("there");
    assert_eq!(kept.last_seen_at, Some(250));
    assert_eq!(store.paired_devices().expect("reads").len(), 1);
}

#[test]
fn removing_a_device_removes_its_subscription() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_device(&phone("a")).expect("added");
    store.add_device(&phone("b")).expect("added");
    for id in ["a", "b"] {
        store
            .conn
            .execute(
                "INSERT INTO pocket_push_subscriptions VALUES (?1, 'https://push', 'k', 'a')",
                (id,),
            )
            .expect("subscribed");
    }
    assert!(store.remove_device("a").expect("removed"));
    assert!(!store.remove_device("a").expect("removed"));
    assert_eq!(store.paired_device("a").expect("reads"), None);
    let left: i64 = store
        .conn
        .query_row("SELECT count(*) FROM pocket_push_subscriptions", [], |r| r.get(0))
        .expect("counted");
    assert_eq!(left, 1);
}
