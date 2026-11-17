//! Asides as the fake answers them: a question is taken and kept nowhere. What runs, answers and is told is `fleet`'s.

use ipc::{AskAside, Aside, AsideList, AsideState, Instant, SessionId, TellAside, WireError};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Asides, Refusal};

fn at() -> Instant {
    Instant::carried("2026-11-15T09:00:00.000Z")
}

impl Asides for FakeDaemon {
    async fn ask_aside(self: std::sync::Arc<Self>, ask: AskAside) -> Result<Aside, Refusal> {
        if ask.question.trim().is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised("fleet.aside_blank", "a side question needs words", run_id())));
        }
        Ok(Aside {
            aside_id: "aside-1".into(),
            session_id: ask.session_id,
            question: ask.question,
            quote: ask.quote,
            answer: String::new(),
            state: AsideState::Asking,
            failed: None,
            asked_at: at(),
            answered_at: None,
            told_at: None,
            reads: Vec::new(),
        })
    }

    async fn list_asides(&self, session_id: SessionId) -> Result<AsideList, Refusal> {
        Ok(AsideList { session_id, asides: Vec::new() })
    }

    async fn tell_aside(self: std::sync::Arc<Self>, tell: TellAside) -> Result<Aside, Refusal> {
        Ok(Aside {
            aside_id: tell.aside_id,
            session_id: tell.session_id,
            question: "why a mutex?".into(),
            quote: None,
            answer: "Two tasks share it.".into(),
            state: AsideState::Answered,
            failed: None,
            asked_at: at(),
            answered_at: Some(at()),
            told_at: Some(at()),
            reads: Vec::new(),
        })
    }
}
