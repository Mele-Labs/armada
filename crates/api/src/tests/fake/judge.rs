//! A Judge as the fake answers it, read off the one Job's record the test holds in `session_row`: its
//! `judge_readings` are what an ask finds and what a note is settled on. No call is made. What runs,
//! reads and fails is `fleet`'s.

use ipc::{
    AskJudge, DismissJudgeNote, JudgeNoteHow, JudgeNoteSettled, JudgeReading, JudgeState, MarkJudgeRead, SendJudgeNote,
    SessionId, SessionRecord, SessionState, WireError,
};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{JudgeReadings, Refusal};

fn at() -> ipc::Instant {
    ipc::Instant::carried("2026-10-31T09:00:00.000Z")
}

impl FakeDaemon {
    fn job_of(&self, id: &SessionId) -> Result<SessionRecord, Refusal> {
        let row = self.session_row.lock().expect("session row").clone().filter(|row| &row.id == id);
        row.ok_or_else(|| Refusal::NotFound(WireError::raised("fleet.no_such_session", "no such Job", run_id())))
    }

    /// Change one reading of the Job and answer it as the engine does.
    fn reading_changed(
        &self,
        id: &SessionId,
        reading: &str,
        change: impl FnOnce(&mut JudgeReading) -> Result<(), Refusal>,
    ) -> Result<JudgeReading, Refusal> {
        let mut row = self.job_of(id)?;
        let held = row.judge_readings.iter_mut().find(|held| held.id == reading).ok_or_else(|| {
            Refusal::NotFound(WireError::raised("fleet.no_such_judge_reading", format!("no reading {reading}"), run_id()))
        })?;
        change(held)?;
        let answer = held.clone();
        *self.session_row.lock().expect("session row") = Some(row);
        Ok(answer)
    }

    fn note_settled(&self, id: &SessionId, reading: &str, note: &str, how: JudgeNoteHow) -> Result<JudgeReading, Refusal> {
        self.reading_changed(id, reading, |held| {
            let found = held.notes.iter_mut().find(|found| found.id == note).ok_or_else(|| {
                Refusal::NotFound(WireError::raised("fleet.no_such_judge_note", format!("no note {note}"), run_id()))
            })?;
            if found.settled.is_none() {
                found.settled = Some(JudgeNoteSettled { how, at: at() });
            }
            Ok(())
        })
    }
}

impl JudgeReadings for FakeDaemon {
    async fn ask_judge(self: std::sync::Arc<Self>, ask: AskJudge) -> Result<JudgeReading, Refusal> {
        let row = self.job_of(&ask.session_id)?;
        if row.state == SessionState::Ended {
            return Err(Refusal::IllegalMove(WireError::raised("fleet.session_closed", "the Job ended", run_id())));
        }
        let held = row.judge_readings.iter().find(|held| held.artifact.kind == ask.artifact.kind && held.artifact.id == ask.artifact.id);
        if let Some(going) = held.filter(|held| held.state == JudgeState::Reading) {
            return Ok(going.clone());
        }
        Ok(JudgeReading {
            id: format!("r{}", row.judge_readings.len() + 1),
            artifact: ask.artifact,
            prompt: ask.prompt,
            state: JudgeState::Reading,
            asked_at: at(),
            finished_at: None,
            failed: None,
            read: false,
            notes: Vec::new(),
            cost_micros: None,
        })
    }

    async fn send_judge_note(self: std::sync::Arc<Self>, send: SendJudgeNote) -> Result<JudgeReading, Refusal> {
        self.note_settled(&send.session_id, &send.reading, &send.note, JudgeNoteHow::Sent)
    }

    async fn dismiss_judge_note(self: std::sync::Arc<Self>, dismiss: DismissJudgeNote) -> Result<JudgeReading, Refusal> {
        self.note_settled(&dismiss.session_id, &dismiss.reading, &dismiss.note, JudgeNoteHow::Dismissed)
    }

    async fn mark_judge_read(self: std::sync::Arc<Self>, read: MarkJudgeRead) -> Result<JudgeReading, Refusal> {
        self.reading_changed(&read.session_id, &read.reading, |held| {
            held.read = true;
            Ok(())
        })
    }
}
