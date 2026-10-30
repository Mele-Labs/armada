//! What the phone may see of a Session.
//!
//! **An allowlist, built field by field**, never by serialising Fleet's record and
//! removing what is unwanted, so a field Fleet adds tomorrow does not reach the
//! phone until somebody writes it here. There is no field for a transcript, a file, a message, a working directory or an
//! environment value, and so none to fill. A Terminal Session has an ask only
//! while Fleet holds its question: [`held_ask`] judges that, and both the row
//! and the answer route read it there.

use ipc::{
    AskedChoice, AskedQuestion, HelmCallAnswer, HelmCallInFlight, SessionOrigin, SessionRecord,
    SessionStanding, SessionState, SessionTurn, WaitingItem, WaitingSource,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PhoneChoice {
    pub label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PhoneQuestion {
    pub question: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<PhoneChoice>,
}

#[derive(Debug, Serialize)]
pub struct PhoneAsk {
    /// What an answer names.
    pub ask_id: String,
    /// The tool reached for, where the ask is permission to run one.
    pub tool: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Empty for a permission ask; the answer is then one of `offers`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<PhoneQuestion>,
    /// `allow_once` and `refuse`. The offer to remember a rule writes the
    /// owner's settings and is not made from a phone.
    pub offers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct PhoneSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// `hosted` or `terminal`.
    pub kind: &'static str,
    pub waiting: bool,
    /// A turn is running, whether or not it has gone quiet.
    pub working: bool,
    /// Instants, so the phone counts the ages itself.
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    /// A hosted Session's, or a Terminal Session's while Fleet holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<PhoneAsk>,
    /// Held on its start card until the owner begins it. Neither waiting nor working meanwhile.
    pub held: bool,
    /// The Job that started this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<PhoneLink>,
    /// What its last answer had in its window, and that window. Absent until reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PhoneLink {
    pub id: String,
    pub title: String,
}

/// The ask a Session is held on. **A Terminal Session's counts only while Fleet
/// still holds it and its mod is listening**: `asked_at` plus `holding_for_seconds`, against `now` in Unix
/// seconds. After that the question is in the terminal and nowhere else.
pub fn held_ask(record: &SessionRecord, now: i64) -> Option<&HelmCallInFlight> {
    match record.origin {
        SessionOrigin::Bridge => record.hosted.as_ref()?.asked.as_ref(),
        SessionOrigin::Terminal => {
            let terminal = record.terminal.as_ref()?;
            // A mod that is not listening will never collect the answer.
            let asked = terminal.asked.as_ref().filter(|_| terminal.listening)?;
            let since = asked.asked_at.to_domain().epoch_millis()?;
            let until = since.saturating_add(i64::try_from(asked.holding_for_seconds).ok()?.saturating_mul(1000));
            (now.saturating_mul(1000) < until).then_some(asked)
        }
    }
}

fn question(asked: &AskedQuestion) -> PhoneQuestion {
    PhoneQuestion {
        question: asked.question.clone(),
        header: asked.header.clone(),
        multi_select: asked.multi_select,
        options: asked.options.iter().map(choice).collect(),
    }
}

fn choice(offered: &AskedChoice) -> PhoneChoice {
    PhoneChoice {
        label: offered.label.clone(),
        description: offered.description.clone(),
    }
}

fn ask(held: &HelmCallInFlight) -> PhoneAsk {
    PhoneAsk {
        ask_id: held.call.clone(),
        tool: held.tool.clone(),
        detail: held.detail.clone(),
        questions: held.questions.iter().map(question).collect(),
        offers: held
            .offers
            .iter()
            .filter_map(|offer| match offer {
                HelmCallAnswer::AllowOnce => Some("allow_once"),
                HelmCallAnswer::Refuse => Some("refuse"),
                // The phone has no folder picker.
                HelmCallAnswer::AllowAndRemember
                | HelmCallAnswer::AlwaysAllow
                | HelmCallAnswer::AlwaysBlock
                | HelmCallAnswer::AllowForSession
                | HelmCallAnswer::AllowLikeThisForSession => None,
            })
            .collect(),
    }
}

/// What a live Job owes the person besides an ask, as Bridge counts it (`stateOf` in
/// `session-state.ts`): a slot choice or the delivery question holds the agent whatever its
/// turn says; while a turn runs or is stuck nothing else is owed yet; otherwise every item
/// but the `done` Fleet derives from the turn.
fn owed(record: &SessionRecord) -> Vec<&WaitingItem> {
    if record.state != SessionState::Live {
        return Vec::new();
    }
    let holds = |one: &&WaitingItem| matches!(one.source, WaitingSource::SlotChoice | WaitingSource::Delivery);
    let items = || record.waiting_for.iter().filter(|one| one.source != WaitingSource::Done);
    let at_work = matches!(record.standing, Some(SessionStanding::Working | SessionStanding::Stuck))
        || matches!(record.hosted.as_ref().map(|h| &h.turn), Some(SessionTurn::Working { .. }));
    if at_work {
        items().filter(holds).collect()
    } else {
        items().collect()
    }
}

impl PhoneSession {
    pub fn of(record: &SessionRecord, repository: Option<&str>, now: i64) -> PhoneSession {
        let hosted = record.origin == SessionOrigin::Bridge;
        let held = record.hosted.as_ref().is_some_and(|facts| facts.start.is_some());
        // A Terminal Session is waiting when its mod says it asked, held or not.
        let waiting_on = match record.origin {
            SessionOrigin::Bridge => held_ask(record, now),
            SessionOrigin::Terminal => record.terminal.as_ref().and_then(|t| t.asked.as_ref()),
        }
        .filter(|_| record.state == SessionState::Live);
        let owed = owed(record);
        let since = waiting_on
            .map(|asked| asked.asked_at.as_str())
            .or_else(|| owed.iter().map(|one| &one.since).min().map(|at| at.as_str()))
            .filter(|_| !held);
        PhoneSession {
            id: record.id.as_str().to_string(),
            title: record.title.clone(),
            repository: repository.map(str::to_string),
            kind: if hosted { "hosted" } else { "terminal" },
            waiting: since.is_some(),
            working: !held && matches!(record.standing, Some(SessionStanding::Working | SessionStanding::Stuck)),
            started_at: record.started_at.as_str().to_string(),
            ended_at: record.ended_at.as_ref().map(|at| at.as_str().to_string()),
            waiting_since: since.map(str::to_string),
            ask: held_ask(record, now)
                .filter(|_| record.state == SessionState::Live)
                .map(ask),
            held,
            from: record.started.as_ref().and_then(|started| started.by.as_ref()).map(|by| PhoneLink {
                id: by.id.as_str().to_string(),
                title: by.title.clone(),
            }),
            context_tokens: record.usage.context_tokens,
            context_window: record.usage.context_window,
        }
    }
}
