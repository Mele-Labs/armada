//! A Session's row, turned into timeline facts as it is published.
//!
//! A Session row is replaced whole, so what changed is read against what this
//! saw last: the first sighting is its start, a waiting item that was not there
//! is an ask (or, where the agent listed it, a wait), one that left the agent's list is resolved,
//! a subagent attached or spent is its start or finish, and live turning to ended is its end.
//! That memory starts empty,
//! so a Session already live when Fleet starts is entered as started the first
//! time its row changes, with the start the row states.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use ipc::{AttachmentState, Event, SessionRecord, SessionState, TimelineKind, TimelineTone, WaitingSource};

use super::{bare, Fact, Subject, Timeline, SAYS};

/// What was last seen, so a repeat is not entered twice.
#[derive(Default)]
struct Seen {
    sessions: BTreeMap<String, SeenSession>,
}

#[derive(Default)]
struct SeenSession {
    ended: bool,
    items: BTreeSet<String>,
    /// What the agent listed itself, by id, so its leaving the list can say what it was.
    listed: BTreeMap<String, String>,
    /// Each subagent the row held, and the state it was last in.
    subagents: BTreeMap<String, AttachmentState>,
}

/// The tap on the event stream.
pub(super) struct Hearing {
    timeline: Timeline,
    seen: Mutex<Seen>,
}

impl Hearing {
    pub(super) fn new(timeline: Timeline) -> Hearing {
        Hearing {
            timeline,
            seen: Mutex::new(Seen::default()),
        }
    }

    /// Queue what this event is, if it is anything. Runs inside `publish`.
    pub(super) fn hear(&self, event: &Event) {
        let Ok(mut seen) = self.seen.lock() else {
            return;
        };
        for fact in self.facts(event, &mut seen) {
            self.timeline.note(fact);
        }
    }

    fn facts(&self, event: &Event, seen: &mut Seen) -> Vec<Fact> {
        match event {
            Event::SessionChanged(record) => Self::session(record, seen),
            _ => Vec::new(),
        }
    }

    fn session(record: &SessionRecord, seen: &mut Seen) -> Vec<Fact> {
        use TimelineKind as K;
        use TimelineTone as T;
        let id = record.id.as_str().to_string();
        let fact = |at: &ipc::Instant, kind, tone, says: String| {
            Fact::new(at.as_str(), kind, Subject::Session(id.clone()), tone, says)
                .titled(record.title.clone())
                .in_repository(record.manifest_id.as_ref().map(|id| id.as_str().to_string()))
        };
        let mut facts = Vec::new();
        let first = !seen.sessions.contains_key(&id);
        let before = seen.sessions.entry(id.clone()).or_default();
        if first {
            facts.push(fact(&record.started_at, K::SessionStarted, T::Run, "Started".to_string()).live());
        }
        for item in &record.waiting_for {
            if item.source == WaitingSource::Walk || before.items.contains(&item.id) {
                continue;
            }
            // What the agent listed is its own to say it waits on; the rest Fleet raised.
            facts.push(match item.source {
                WaitingSource::Agent => {
                    fact(&item.since, K::SessionWaiting, T::Ask, bare(&format!("Waiting: {}", item.text), SAYS))
                }
                _ => fact(&item.since, K::SessionAsked, T::Ask, bare(&format!("Asked: {}", item.text), SAYS)),
            });
        }
        before.items = record.waiting_for.iter().map(|item| item.id.clone()).collect();
        let listed: BTreeMap<String, String> = record
            .waiting_for
            .iter()
            .filter(|item| item.source == WaitingSource::Agent)
            .map(|item| (item.id.clone(), item.text.clone()))
            .collect();
        // An ended Session's list is cleared with it, which resolves nothing.
        if record.state == SessionState::Live {
            for (_, text) in before.listed.iter().filter(|(id, _)| !listed.contains_key(*id)) {
                let says = bare(&format!("Resolved: {text}"), SAYS);
                facts.push(fact(&record.last_seen_at, K::SessionResolved, T::Ok, says));
            }
        }
        before.listed = listed;
        for one in record.attachments.iter().filter(|one| one.kind == "subagent") {
            let spent = one.state == AttachmentState::Spent;
            let known = before.subagents.insert(one.target.clone(), one.state);
            // A Session already held subagents when Fleet first saw it: they began before this.
            if first {
                continue;
            }
            let said = |key: &str| one.detail.get(key).map(String::as_str).unwrap_or_default();
            let what = [said("type"), said("description")].iter().filter(|part| !part.is_empty()).cloned().collect::<Vec<_>>().join(": ");
            let subagent = |at: &ipc::Instant, tone, says: &str| {
                fact(at, K::SessionSubagent, tone, says.to_string()).detail(Some(what.clone()))
            };
            if known.is_none() {
                facts.push(subagent(&one.since, T::Run, "Subagent started").live());
            }
            // Its parent sent it more work: standing again, from when it started on it.
            if one.state == AttachmentState::Standing && known.is_some_and(|was| was != AttachmentState::Standing) {
                facts.push(subagent(&one.since, T::Run, "Subagent resumed").live());
            }
            if spent && known != Some(AttachmentState::Spent) {
                facts.push(subagent(&one.changed_at, T::Ok, "Subagent finished"));
            }
        }
        let ended = record.state == SessionState::Ended;
        if ended && !before.ended {
            let at = record.ended_at.as_ref().unwrap_or(&record.last_seen_at);
            facts.push(fact(at, K::SessionEnded, T::Idle, "Ended".to_string()).detail(record.end_reason.clone()).over());
        }
        before.ended = ended;
        facts
    }
}
