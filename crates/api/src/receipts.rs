//! What an agent is told after it acts on its own Session through the door.
//!
//! The row grows with every attachment, 10 KB at the median and past 60 KB for a long Session, and
//! an agent pastes each answer into its context for good. The receipt is the session id and what
//! the act set, so it weighs the same on the first call and the thousandth. Bridge's route answers
//! the row as it always did: the door alone sets [`crate::door::DoorCalled`].

use ipc::{SessionRecord, WaitingSource};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Receipt<'a> {
    session_id: &'a str,
    #[serde(flatten)]
    set: Set<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Set<'a> {
    WaitingFor {
        waiting_for: Vec<Item<'a>>,
    },
    Window {
        window: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        takes: Vec<&'a str>,
    },
    Asked {
        #[serde(skip_serializing_if = "Option::is_none")]
        asked: Option<&'a str>,
    },
    Done {
        done: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        done_since: Option<&'a str>,
    },
}

#[derive(Serialize)]
struct Item<'a> {
    id: &'a str,
    text: &'a str,
}

impl<'a> Receipt<'a> {
    /// The agent's own items as Fleet kept them: one of Fleet's that names the same target wins, and
    /// the agent's is then not among them.
    pub(crate) fn waiting_for(record: &'a SessionRecord) -> Self {
        let waiting_for = record
            .waiting_for
            .iter()
            .filter(|item| item.source == WaitingSource::Agent)
            .map(|item| Item {
                id: &item.id,
                text: &item.text,
            })
            .collect();
        Receipt::of(record, Set::WaitingFor { waiting_for })
    }

    pub(crate) fn window(record: &'a SessionRecord, show: &'a ipc::ShowWindow) -> Self {
        let set = Set::Window {
            window: &show.url,
            title: show.title.as_deref(),
            takes: show
                .variations
                .iter()
                .map(|take| take.name.as_str())
                .collect(),
        };
        Receipt::of(record, set)
    }

    /// The newest open question at the place the agent named.
    pub(crate) fn asked(record: &'a SessionRecord, ask: &ipc::AskInWindow) -> Self {
        let asked = record
            .waiting_for
            .iter()
            .rev()
            .filter(|item| item.source == WaitingSource::WindowAsk)
            .find(|item| {
                item.place.as_ref().is_some_and(|place| {
                    place.url == ask.url.trim() && place.selector == ask.selector.trim()
                })
            })
            .map(|item| item.id.as_str());
        Receipt::of(record, Set::Asked { asked })
    }

    pub(crate) fn done(record: &'a SessionRecord) -> Self {
        let done_since = record.done_since.as_ref().map(|at| at.as_str());
        Receipt::of(
            record,
            Set::Done {
                done: done_since.is_some(),
                done_since,
            },
        )
    }

    fn of(record: &'a SessionRecord, set: Set<'a>) -> Self {
        Receipt {
            session_id: record.id.as_str(),
            set,
        }
    }
}
