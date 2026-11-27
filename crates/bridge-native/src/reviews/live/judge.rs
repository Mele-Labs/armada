//! Judge on a pull request, asked of Fleet: the judge types a Job's repository sees, the ask itself, a note sent or
//! dismissed, and the reading marked read. The reading comes back on the holding Job's record, not in these answers.

use gpui::{Context, Window};
use ipc::{JudgeArtifact, JudgeNoteHow, SessionId};
use serde::de::IgnoredAny;

use super::Verb;
use crate::reviews::ReviewsScreen;
use crate::reviews::judge::Holder;

impl ReviewsScreen {
    /// Read the types the pull request's Job sees, for the box that asks. A list read before stays drawn until the new one comes.
    pub(in crate::reviews) fn live_judge_types(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some((repository, project)) = self.judge_place(number, cx) else { return };
        let key = crate::judge_types::key(&repository, project.as_deref());
        let Some(port) = self.port(cx) else {
            self.judging.types.read.insert(key, Err("Fleet is not connected".to_string()));
            return;
        };
        let read = crate::judge_types::list(port, &repository, project.as_deref(), cx);
        let held = key.clone();
        self.judging.types.reading.insert(
            key,
            cx.spawn_in(window, async move |this, cx| {
                let got = read.await.map_err(|why| why.said);
                let _ = this.update(cx, |this, cx| {
                    this.judging.types.read.insert(held, got);
                    cx.notify();
                });
            }),
        );
    }

    /// Ask the holding Job's Judge to read the pull request. Fleet answers at once with the reading going; its
    /// record carries the rest.
    pub(in crate::reviews) fn live_judge_ask(&mut self, holder: Holder, artifact: JudgeArtifact, prompt: String, judge: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let number: u64 = artifact.id.parse().unwrap_or_default();
        let body = ipc::encode(&ipc::AskJudge { session_id: SessionId::carried(holder.id), artifact, prompt, judge }).ok();
        self.call(format!("judge:{number}"), Verb::Post, "/sessions/judge".into(), body, "JudgeReading", window, cx, move |this, answer: Result<IgnoredAny, String>, _, cx| match answer {
            Ok(_) => this.judge_asked_done(number, cx),
            Err(why) => this.judge_asked_refused(number, why, cx),
        });
    }

    /// Send a note to the Job as his message, or dismiss it. A refusal takes the note back.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::reviews) fn live_judge_settle(&mut self, number: u64, holder: &str, reading: &str, note: &str, how: JudgeNoteHow, window: &mut Window, cx: &mut Context<Self>) {
        let session_id = SessionId::carried(holder);
        let (reading, note) = (reading.to_string(), note.to_string());
        let (route, body) = match how {
            JudgeNoteHow::Sent => ("/sessions/judge/send", ipc::encode(&ipc::SendJudgeNote { session_id, reading: reading.clone(), note: note.clone() }).ok()),
            JudgeNoteHow::Dismissed => ("/sessions/judge/dismiss", ipc::encode(&ipc::DismissJudgeNote { session_id, reading: reading.clone(), note: note.clone() }).ok()),
        };
        let mark = format!("{reading}/{note}");
        self.call(format!("judge-note:{mark}"), Verb::Post, route.into(), body, "JudgeReading", window, cx, move |this, answer: Result<IgnoredAny, String>, _, cx| {
            if let Err(why) = answer {
                this.judging.settled.remove(&mark);
                this.judging.refused.insert(number, why);
                cx.notify();
            }
        });
    }

    /// Tell Fleet the reading's notes were opened.
    pub(in crate::reviews) fn live_judge_read(&mut self, holder: &str, reading: &str, cx: &mut Context<Self>) {
        let body = ipc::encode(&ipc::MarkJudgeRead { session_id: SessionId::carried(holder), reading: reading.to_string() }).ok();
        self.call_quiet(format!("judge-read:{reading}"), Verb::Post, "/sessions/judge/read".into(), body, "JudgeReading", cx, |_, _: Result<IgnoredAny, String>, _| {});
    }
}
