//! `/api/live`: Fleet's event stream narrowed to Session changes, what the Operator did with a dispatch, and
//! that the acts waiting for a sleeping Mac changed.
//!
//! Every message Fleet sends is read as the type it is, and only those are written back out, as the
//! fields of [`Change`] and nothing else. `queued` carries no list: the phone reads `/api/queued` again.

use ipc::operator_board::OperatorDispatched;
use ipc::{Event, StreamMessage};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Change {
    /// `session`, `operator`, `queued`, or `resync` when the phone should read its lists again.
    pub change: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// For `operator`: what the Operator did with a dispatch it held.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatched: Option<OperatorDispatched>,
}

/// The Session change a Fleet message comes to, or `None` for anything else.
pub fn change(text: &str) -> Option<Change> {
    match ipc::decode::<StreamMessage>("a stream message", text.as_bytes()).ok()? {
        // A resync or a drop means the phone's lists may be behind.
        StreamMessage::Resync(_) | StreamMessage::Missed(_) => {
            Some(Change { change: "resync", session_id: None, dispatched: None })
        }
        StreamMessage::Event(delivered) => match delivered.event {
            Event::SessionChanged(record) => Some(Change {
                change: "session",
                session_id: Some(record.id.as_str().to_string()),
                dispatched: None,
            }),
            Event::OperatorDispatched(dispatched) => Some(Change { change: "operator", session_id: None, dispatched: Some(dispatched) }),
            Event::QueuedChanged(_) => Some(Change { change: "queued", session_id: None, dispatched: None }),
            _ => None,
        },
    }
}

/// One Server-Sent Event.
pub fn frame(change: &Change) -> Option<String> {
    Some(format!("data: {}\n\n", ipc::encode(change).ok()?))
}
