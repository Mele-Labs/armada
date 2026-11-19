//! How his messages were sorted, for a walk, a picture and the tests, with no Fleet. It stands on the message rows of the
//! `job-view` fixture: the first Job's first words and the last six steps of the second Job. A change is kept the way Fleet keeps it, and
//! refused where it would take an ask that holds revisions or answers of its own.

use ipc::{Ask, AskKind, AskState, AskWords, Instant, MessageKind, MessageSorting, SessionId, SetMessageKind, SortedBy};

use super::store::Held;
use crate::settings::wire::Refused;

fn at(minute: u32) -> Instant {
    Instant::carried(format!("2026-11-09T13:{minute:02}:00Z"))
}

fn sorting(job: &str, row: &str, kind: MessageKind, ask_kind: Option<AskKind>, ask: Option<&str>, by: SortedBy, minute: u32) -> MessageSorting {
    MessageSorting {
        session_id: SessionId::carried(job),
        row_id: row.into(),
        kind,
        ask_kind,
        ask_id: ask.map(str::to_string),
        title: None,
        sorted_by: by,
        sorted_at: at(minute),
    }
}

fn ask(job: &str, id: &str, kind: AskKind, title: &str, state: AskState, row: &str, said: Vec<AskWords>) -> Ask {
    Ask {
        ask_id: id.into(),
        kind,
        title: title.into(),
        first_words: title.into(),
        first_row: row.into(),
        said_in: SessionId::carried(job),
        building: matches!(state, AskState::Building | AskState::Built).then(|| SessionId::carried(job)),
        features: Vec::new(),
        state,
        state_at: at(30),
        opened_at: at(1),
        said,
    }
}

fn words(job: &str, row: &str, kind: MessageKind) -> AskWords {
    AskWords { session_id: SessionId::carried(job), row_id: row.into(), kind, words: String::new(), at: at(40) }
}

/// The sortings and asks of Job `job`, none for a Job the fixture has no words for.
pub(crate) fn held(job: &str) -> Held {
    use AskKind::*;
    use MessageKind::*;
    use SortedBy::*;
    match job {
        "mock-01" => Held {
            sortings: vec![
                sorting(job, "m-001", NewAsk, Some(Bug), Some("ask-1"), Fleet, 2),
                sorting(job, "m-017", Answer, None, Some("ask-1"), Owner, 24),
            ],
            asks: vec![ask(job, "ask-1", Bug, "The merge line chip stays green when behind", AskState::Building, "m-001", vec![words(job, "m-017", Answer)])],
        },
        "mock-02" => Held {
            sortings: vec![
                sorting(job, "m-301", NewAsk, Some(ChangeRequest), Some("ask-1"), Fleet, 2),
                sorting(job, "m-306", NewAsk, Some(Idea), Some("ask-2"), Fleet, 3),
                sorting(job, "m-311", Revision, None, Some("ask-1"), Fleet, 4),
                sorting(job, "m-316", Answer, None, Some("ask-2"), Owner, 5),
                sorting(job, "m-321", Talk, None, None, Fleet, 6),
                sorting(job, "m-326", NewAsk, Some(Bug), Some("ask-3"), Fleet, 7),
                sorting(job, "m-331", NewAsk, Some(Fix), Some("ask-4"), Fleet, 8),
            ],
            asks: vec![
                ask(job, "ask-1", ChangeRequest, "Look at the router", AskState::Building, "m-301", vec![words(job, "m-311", Revision)]),
                ask(job, "ask-2", Idea, "Check the retry rules", AskState::Backlog, "m-306", vec![words(job, "m-316", Answer)]),
                ask(job, "ask-3", Bug, "The audit table builds on every call", AskState::Building, "m-326", Vec::new()),
                ask(job, "ask-4", Fix, "Rename the cache keys", AskState::Built, "m-331", Vec::new()),
            ],
        },
        _ => Held::default(),
    }
}

/// `set_message_kind` as Fleet takes it, on the held words: the sorting as kept, or Fleet's refusal.
pub(crate) fn apply(held: &mut Held, set: &SetMessageKind) -> Result<MessageSorting, Refused> {
    let refuse = |code: &str, said: &str| Refused { key: None, said: said.into(), code: Some(code.into()) };
    let at_row = held.sortings.iter().position(|one| one.row_id == set.row_id);
    // The ask this message opened, if it opened one.
    let opened = at_row.map(|at| &held.sortings[at]).filter(|one| one.kind == MessageKind::NewAsk).and_then(|one| one.ask_id.clone());
    if let Some(opened) = opened.as_deref().filter(|_| set.kind != MessageKind::NewAsk) {
        if held.asks.iter().any(|ask| ask.ask_id == opened && ask.said.iter().any(|words| words.row_id != set.row_id)) {
            return Err(refuse("fleet.ask_has_words", "the ask this message opened holds revisions or answers, so it cannot become something else"));
        }
    }
    let joined = match set.kind {
        MessageKind::Revision | MessageKind::Answer => {
            let Some(id) = set.ask_id.as_deref() else { return Err(refuse("fleet.ask_needed", "a revision or an answer names the ask it joins")) };
            if !held.asks.iter().any(|ask| ask.ask_id == id) {
                return Err(refuse("fleet.no_such_ask", "this Job holds no such ask"));
            }
            Some(id.to_string())
        }
        MessageKind::NewAsk => match opened.clone() {
            Some(id) => {
                if let Some(ask) = held.asks.iter_mut().find(|ask| ask.ask_id == id) {
                    ask.kind = set.ask_kind.unwrap_or(ask.kind);
                }
                Some(id)
            }
            None => {
                let id = format!("ask-{}", held.asks.len() + 1);
                let kind = set.ask_kind.unwrap_or(AskKind::ChangeRequest);
                held.asks.push(ask(set.session_id.as_str(), &id, kind, "New ask", AskState::Building, &set.row_id, Vec::new()));
                Some(id)
            }
        },
        MessageKind::Question | MessageKind::Talk => None,
    };
    if let Some(opened) = opened.filter(|id| set.kind != MessageKind::NewAsk || joined.as_deref() != Some(id)) {
        held.asks.retain(|ask| ask.ask_id != opened);
    }
    let kept = MessageSorting {
        session_id: set.session_id.clone(),
        row_id: set.row_id.clone(),
        kind: set.kind,
        ask_kind: (set.kind == MessageKind::NewAsk).then(|| set.ask_kind.unwrap_or(AskKind::ChangeRequest)),
        ask_id: joined,
        title: None,
        sorted_by: SortedBy::Owner,
        sorted_at: at(50),
    };
    match at_row {
        Some(at) => held.sortings[at] = kept.clone(),
        None => held.sortings.push(kept.clone()),
    }
    Ok(kept)
}
