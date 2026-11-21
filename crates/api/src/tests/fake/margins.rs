//! Margin threads as the fake answers them: a comment is taken and kept nowhere, and every page holds none. What
//! is kept, sent and written is `fleet`'s.

use std::sync::Arc;

use ipc::{
    ActOnMargin, AnswerThread, CommentMargin, GetMarginThreads, Instant, MarginAnchor, MarginMessage, MarginState, MarginThread,
    MarginThreads, MarginVoice, SendHeldMargin, SessionId, SuggestEdit, WireError,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Caller, Margins, Refusal};

fn none_on(session_id: SessionId, document: String) -> MarginThreads {
    MarginThreads { session_id, document, threads: Vec::new() }
}

fn the_thread(thread: String, said: String) -> MarginThread {
    MarginThread {
        id: thread,
        document: "quiet-hours.html".into(),
        anchor: MarginAnchor { quote: "Notices wait until eight".into(), block: None, section: Some("When notices wait".into()) },
        messages: vec![MarginMessage { by: MarginVoice::Agent, text: said, at: Instant::carried("2026-11-21T09:00:00.000Z"), held: false }],
        state: MarginState::Answered,
        gone: false,
        suggestion: None,
        outcome: None,
    }
}

impl Margins for FakeDaemon {
    async fn comment_margin(self: Arc<Self>, comment: CommentMargin) -> Result<MarginThreads, Refusal> {
        if comment.text.trim().is_empty() {
            return Err(Refusal::Unacceptable(WireError::raised("fleet.margin_empty", "a comment says something", run_id())));
        }
        Ok(none_on(comment.session_id, comment.document))
    }

    async fn send_held_margin(self: Arc<Self>, send: SendHeldMargin) -> Result<MarginThreads, Refusal> {
        Ok(none_on(send.session_id, send.document))
    }

    async fn act_on_margin_thread(self: Arc<Self>, act: ActOnMargin) -> Result<MarginThreads, Refusal> {
        Ok(none_on(act.session_id, "quiet-hours.html".into()))
    }

    async fn get_margin_threads(&self, get: GetMarginThreads) -> Result<MarginThreads, Refusal> {
        Ok(none_on(get.session_id, get.document))
    }

    async fn answer_thread(self: Arc<Self>, _caller: Option<Caller>, answer: AnswerThread) -> Result<MarginThread, Refusal> {
        Ok(the_thread(answer.thread, answer.text))
    }

    async fn suggest_edit(self: Arc<Self>, _caller: Option<Caller>, suggest: SuggestEdit) -> Result<MarginThread, Refusal> {
        Ok(the_thread(suggest.thread, format!("{} reads {}", suggest.struck, suggest.added)))
    }
}
