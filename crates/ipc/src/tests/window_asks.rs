//! `ask_in_window` on the wire: the call the agent sends, the row the thread keeps and the item
//! Waiting on you reads, each as Bridge's mirror in `packages/protocol` spells it.

use serde_json::json;

use crate::{AskInWindow, SessionRow, WaitingItem};

#[test]
fn the_call_reads_with_only_what_is_required_and_writes_nothing_absent() {
    let ask: AskInWindow = crate::decode(
        "a question to ask in the window",
        br#"{"question":"Beside or under?","url":"http://localhost:5173/","selector":"main > h1"}"#,
    )
    .unwrap();
    assert!(ask.options.is_empty() && ask.text.is_none() && ask.session_id.is_none());
    let back = serde_json::to_value(&ask).unwrap();
    assert_eq!(back, json!({"question":"Beside or under?","url":"http://localhost:5173/","selector":"main > h1"}));
}

#[test]
fn the_row_and_the_item_round_trip_with_their_place() {
    let row = json!({
        "kind": "window_ask", "id": "r1", "at": "2026-10-11T09:00:00Z",
        "question": "Beside or under?",
        "options": [{"label": "Beside", "description": "One line"}, {"label": "Under"}],
        "place": {"url": "http://localhost:5173/f", "selector": "main > h1", "text": "Findings"},
        "window": "http://localhost:5173/"
    });
    let read: SessionRow = serde_json::from_value(row.clone()).unwrap();
    assert!(matches!(&read, SessionRow::WindowAsk { answer: None, .. }));
    assert_eq!(read.id(), "r1");
    assert_eq!(serde_json::to_value(&read).unwrap(), row);

    let item = json!({
        "id": "window_ask:r1", "text": "Beside or under?", "since": "2026-10-11T09:00:00Z",
        "source": "window_ask", "act": {"kind": "walk", "target": "http://localhost:5173/"},
        "options": [{"label": "Beside", "description": "One line"}, {"label": "Under"}],
        "place": {"url": "http://localhost:5173/f", "selector": "main > h1", "text": "Findings"}
    });
    let read: WaitingItem = serde_json::from_value(item.clone()).unwrap();
    assert_eq!(serde_json::to_value(&read).unwrap(), item);
}
