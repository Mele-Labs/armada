//! The ideas the Operator kept for later: the model Next up lists them from, newest first, and what a press on one asks of
//! the page. A kept idea is drawn in Next up (`next_up.rs`) and, in front, as a card (`idea.rs`); this holds what Fleet keeps
//! (`list_parked_ideas`) and the page holds the wire.

use std::collections::HashSet;

use gpui::{Context, EventEmitter, SharedString};
use ipc::ParkedIdea;

/// What a press on a kept idea asks of the page.
#[derive(Clone, Debug, PartialEq)]
pub enum ParkedAsked {
    /// Send the idea as a dispatch, then let it go.
    Start(ParkedIdea),
    Remove(String),
}

impl EventEmitter<ParkedAsked> for ParkedIdeas {}

pub struct ParkedIdeas {
    /// What Fleet keeps, oldest first as it says it. `None` until read.
    ideas: Option<Vec<ParkedIdea>>,
    /// The ideas whose press is in flight.
    busy: HashSet<String>,
    /// Why a press did not come back done.
    refused: Option<SharedString>,
}

impl ParkedIdeas {
    pub fn new() -> Self {
        ParkedIdeas { ideas: None, busy: HashSet::new(), refused: None }
    }

    /// What Fleet keeps now.
    pub fn set(&mut self, ideas: Vec<ParkedIdea>, cx: &mut Context<Self>) {
        self.busy.retain(|id| ideas.iter().any(|idea| &idea.id == id));
        self.ideas = Some(ideas);
        cx.notify();
    }

    /// Newest first, which is not the order Fleet keeps them in.
    pub fn ideas(&self) -> Vec<&ParkedIdea> {
        let mut ideas: Vec<&ParkedIdea> = self.ideas.iter().flatten().collect();
        ideas.sort_by(|a, b| at_of(b).cmp(&at_of(a)));
        ideas
    }

    pub fn is_read(&self) -> bool {
        self.ideas.is_some()
    }

    /// A press is out for the idea; or it is back.
    pub fn busy(&mut self, id: &str, on: bool, cx: &mut Context<Self>) {
        if on {
            self.busy.insert(id.to_string());
            self.refused = None;
        } else {
            self.busy.remove(id);
        }
        cx.notify();
    }

    pub fn refuse(&mut self, why: String, cx: &mut Context<Self>) {
        self.refused = Some(why.into());
        cx.notify();
    }

    /// Whether a press is out for the idea.
    pub fn is_busy(&self, id: &str) -> bool {
        self.busy.contains(id)
    }

    /// Why the last press did not come back done.
    pub fn refused(&self) -> Option<SharedString> {
        self.refused.clone()
    }
}

/// What he sent: the request, else the words of the message that went with it.
pub(super) fn words_of(idea: &ParkedIdea) -> String {
    idea.dispatch
        .start
        .request
        .clone()
        .or_else(|| {
            idea.dispatch.then.iter().find_map(|then| match then {
                ipc::DispatchThen::Message(message) => Some(message.text.clone()),
                ipc::DispatchThen::Notes(_) => None,
            })
        })
        .unwrap_or_default()
}

fn at_of(idea: &ParkedIdea) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(&idea.at).map_or(chrono::DateTime::<chrono::Utc>::MIN_UTC, |at| at.to_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idea(id: &str, at: &str, words: &str) -> ParkedIdea {
        let wire = format!(r#"{{"id":"{id}","at":"{at}","dispatch":{{"manifest_id":"armada","request":"{words}"}}}}"#);
        ipc::decode("a parked idea", wire.as_bytes()).expect("a parked idea")
    }

    #[test]
    fn ideas_come_newest_first() {
        let mut list = ParkedIdeas::new();
        list.ideas = Some(vec![idea("a", "2026-11-11T09:00:00-05:00", "first"), idea("c", "2026-11-11T12:00:00-05:00", "third"), idea("b", "2026-11-11T10:00:00-05:00", "second")]);
        let order: Vec<_> = list.ideas().iter().map(|idea| idea.id.as_str()).collect();
        assert_eq!(order, ["c", "b", "a"]);
    }

    #[test]
    fn an_idea_reads_as_its_request_else_the_message_that_went_with_it() {
        assert_eq!(words_of(&idea("a", "2026-11-11T09:00:00-05:00", "Keep the scrollback")), "Keep the scrollback");
        let wire = r#"{"id":"b","at":"2026-11-11T09:00:00-05:00","dispatch":{"manifest_id":"armada","then":[{"message":{"text":"From the message"}}]}}"#;
        assert_eq!(words_of(&ipc::decode("a parked idea", wire.as_bytes()).expect("a parked idea")), "From the message");
    }
}
