//! What the phone may see of a Session.
//!
//! **An allowlist, built field by field**, as [`crate::phone`] is for a Job. There
//! is no field for a transcript, a file, a message, a working directory or an
//! environment value, and so none to fill. A Terminal Session is built without
//! an ask: [`held_ask`] answers only for a hosted one, and both the row and the
//! answer route read it there.

use ipc::{
    AskedChoice, AskedQuestion, HelmCallAnswer, HelmCallInFlight, SessionOrigin, SessionRecord,
    SessionState,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    /// Hosted Sessions only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<PhoneAsk>,
}

/// The ask a hosted Session is held on. **Never a Terminal Session's**, which
/// Fleet may carry and the phone does not answer.
pub fn held_ask(record: &SessionRecord) -> Option<&HelmCallInFlight> {
    match record.origin {
        SessionOrigin::Bridge => record.hosted.as_ref()?.asked.as_ref(),
        SessionOrigin::Terminal => None,
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
                HelmCallAnswer::AllowAndRemember => None,
            })
            .collect(),
    }
}

impl PhoneSession {
    pub fn of(record: &SessionRecord, repository: Option<&str>) -> PhoneSession {
        let hosted = record.origin == SessionOrigin::Bridge;
        // A Terminal Session is waiting when its mod says it asked; that is the
        // whole of what the phone is told.
        let waiting_on = match record.origin {
            SessionOrigin::Bridge => held_ask(record),
            SessionOrigin::Terminal => record.terminal.as_ref().and_then(|t| t.asked.as_ref()),
        }
        .filter(|_| record.state == SessionState::Live);
        PhoneSession {
            id: record.id.as_str().to_string(),
            title: record.title.clone(),
            repository: repository.map(str::to_string),
            kind: if hosted { "hosted" } else { "terminal" },
            waiting: waiting_on.is_some(),
            waiting_since: waiting_on.map(|asked| asked.asked_at.as_str().to_string()),
            ask: held_ask(record)
                .filter(|_| record.state == SessionState::Live)
                .map(ask),
        }
    }
}
