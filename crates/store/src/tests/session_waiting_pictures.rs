//! The pictures an agent's item offers its options as are kept with the item through a reopen of the file, open
//! and closed, and a row kept before them reads as having none.

use crate::tests::{open, TempDir};
use crate::{KeptClosed, KeptWaiting};

const PICTURES: &str = r#"[{"image":"/files/a.png","width":10,"height":20},null]"#;

fn item(pictures: &str) -> KeptWaiting {
    KeptWaiting {
        item_id: "header".into(),
        text: "Which header?".into(),
        since: "2026-11-13T09:00:00.000Z".into(),
        act: None,
        options: vec!["Wide".into(), "Words only".into()],
        pictures: pictures.into(),
        steps: Vec::new(),
        approves: false,
    }
}

#[test]
fn the_pictures_of_an_item_are_kept_open_closed_and_reopened() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_waiting_for("s1", &[item(PICTURES), KeptWaiting { item_id: "plain".into(), ..item("") }]).unwrap();
    drop(store);
    let mut store = open(&dir);
    let read = store.waiting_for("s1").unwrap();
    assert_eq!(read.iter().map(|one| one.pictures.as_str()).collect::<Vec<_>>(), [PICTURES, ""]);

    let closed = KeptClosed {
        item_id: "header".into(),
        text: "Which header?".into(),
        source: "agent".into(),
        act: None,
        options: vec!["Wide".into(), "Words only".into()],
        pictures: PICTURES.into(),
        steps: Vec::new(),
        since: "2026-11-13T09:00:00.000Z".into(),
        closed_at: "2026-11-13T10:00:00.000Z".into(),
        approves: false,
    };
    store.keep_closed_waiting("s1", &closed).unwrap();
    drop(store);
    let mut store = open(&dir);
    assert_eq!(store.closed_waiting("s1").unwrap(), [closed]);

    store.reopen_waiting("s1", "header", Some(&item(PICTURES))).unwrap();
    store.replace_waiting_for("s1", &[]).unwrap();
    store.reopen_waiting("s1", "header", Some(&item(PICTURES))).unwrap();
    assert_eq!(store.waiting_for("s1").unwrap()[0].pictures, PICTURES);
}
