//! Fleet's side of the Judge's four operations: the one read-only call over the checkout Fleet serves,
//! the reading kept for the artifact it read, and the person's acts on what it found.
//! `docs/concepts/judge.md`; what the call is shown and how its answer is read is `judge`, and what each
//! kind of artifact is read into is `judge_content`.
//!
//! **No bay**: an artifact is read against the repository as it stands, which is the served checkout, so
//! nothing is leased and nothing is given back. Read-only by construction all the same:
//! `Reading::checkout` gives the call the three read tools and no other.
//!
//! **Only a press starts one.** An ask answers at once with the reading `reading`; the call goes on a
//! task of its own, and the Job's `session.changed` is published when it starts, when it ends, when a
//! note is settled and when the notes are opened.

use std::num::NonZeroU8;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Reading, Vcs, WorkProduct};
use api::{JudgeReadings, Refusal};
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{
    AskJudge, DismissJudgeNote, JudgeNoteHow, JudgeReading, ManifestId, MarkJudgeRead, SendJudgeNote, SendSessionMessage,
    SessionId, SessionRow, SessionVoice, WireError,
};
use store::{KeptHosting, KeptJudgeNote, KeptJudgeReading, KeptSession};

use crate::daemon::Fleet;
use crate::judge::{brief, notes_read, Shown};
use crate::judge_keeping::{how_word, kind_of, kind_word, message, note_kept, reading_of, DIED, DONE, FAILED, READING};
use crate::review_runs::asked_and_costed;
use crate::session_host::NO_SUCH_SESSION;

/// The Job holds no reading of that id: it was never asked, or asked again for the same artifact. A 404.
const NO_SUCH_JUDGE_READING: &str = "fleet.no_such_judge_reading";
/// The reading holds no note of that id. A 404.
const NO_SUCH_JUDGE_NOTE: &str = "fleet.no_such_judge_note";

/// Turns the call may take, its reads and its answer together: a pull request run's forty, since an
/// artifact is checked against as many files as a change is.
const READ_TURNS: NonZeroU8 = NonZeroU8::new(40).expect("forty is not zero");

/// What marks a reading going in this process, in the set an agent run's keys share.
fn going_key(session: &str, kind: &str, artifact: &str) -> String {
    format!("judge:{session}:{kind}:{artifact}")
}

/// What the call answered, or why it did not, and what it cost either way.
type Read = Result<(Vec<KeptJudgeNote>, Option<u64>), (String, Option<u64>)>;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `ask_judge`. A reading going for the artifact is answered whatever the press said; a finished one
    /// is replaced by a fresh reading with a fresh `id`.
    pub(crate) async fn judge_asked(self: Arc<Self>, ask: AskJudge) -> Result<JudgeReading, Refusal> {
        let id = ask.session_id.as_str().trim().to_string();
        let (session, _) = self.judged_job(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let (kind, artifact) = (kind_word(ask.artifact.kind), ask.artifact.id.trim().to_string());
        let key = going_key(&id, kind, &artifact);
        let kept = self.store().lock().await.judge_reading_of(&id, kind, &artifact).map_err(|why| self.ledger_fault(why))?;
        if let Some(mut kept) = kept.filter(|kept| kept.state == READING) {
            if self.review_going().lock().await.contains(&key) {
                return Ok(reading_of(&kept));
            }
            kept = self.died(kept);
            self.judge_kept(&id, &kept).await;
        }
        if !self.review_going().lock().await.insert(key.clone()) {
            // Another press started it between the read and here.
            let kept = self.store().lock().await.judge_reading_of(&id, kind, &artifact).ok().flatten();
            if let Some(kept) = kept {
                return Ok(reading_of(&kept));
            }
        }
        let reading = KeptJudgeReading {
            id: self.mint().ulid().as_str().to_lowercase(),
            kind: kind.to_string(),
            artifact_id: artifact,
            title: ask.artifact.title.trim().to_string(),
            prompt: ask.prompt.trim().to_string(),
            state: READING.to_string(),
            asked_at: self.now().as_str().to_string(),
            finished_at: None,
            failed: None,
            read: false,
            cost_micros: None,
            notes: Vec::new(),
        };
        self.judge_kept(&id, &reading).await;
        let answer = reading_of(&reading);
        tokio::spawn(async move { self.judge_driven(id, reading, key).await });
        Ok(answer)
    }

    /// `send_judge_note`: the note goes to the agent as the person's message, then it is settled. **Sent
    /// before it is settled**, so a send that fails leaves it waiting.
    pub(crate) async fn judge_note_sent(self: Arc<Self>, send: SendJudgeNote) -> Result<JudgeReading, Refusal> {
        let (id, kept, at) = self.judge_note_held(&send.session_id, &send.reading, &send.note).await?;
        let note = &kept.notes[at];
        let settling = self.hosts().settling(&id, &format!("judge:{}:{}", kept.id, note.id));
        if note.settled.is_some() || settling.is_none() {
            return Ok(reading_of(&kept));
        }
        let sent = SendSessionMessage {
            session_id: SessionId::carried(&id),
            text: message(&kept, note),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        Arc::clone(&self).send_known(None, sent).await?;
        self.judge_note_settled(&id, &kept, at, JudgeNoteHow::Sent).await
    }

    /// `dismiss_judge_note`: the note is settled and the agent is told nothing.
    pub(crate) async fn judge_note_dismissed(self: Arc<Self>, dismiss: DismissJudgeNote) -> Result<JudgeReading, Refusal> {
        let (id, kept, at) = self.judge_note_held(&dismiss.session_id, &dismiss.reading, &dismiss.note).await?;
        let settling = self.hosts().settling(&id, &format!("judge:{}:{}", kept.id, kept.notes[at].id));
        if kept.notes[at].settled.is_some() || settling.is_none() {
            return Ok(reading_of(&kept));
        }
        self.judge_note_settled(&id, &kept, at, JudgeNoteHow::Dismissed).await
    }

    /// `mark_judge_read`: the person opened the notes. The agent is told nothing.
    pub(crate) async fn judge_marked_read(self: Arc<Self>, read: MarkJudgeRead) -> Result<JudgeReading, Refusal> {
        let id = read.session_id.as_str().trim().to_string();
        self.judged_job(&id).await?;
        let kept = {
            let mut store = self.store().lock().await;
            let held = store.mark_judge_read(&id, &read.reading).map_err(|why| self.ledger_fault(why))?;
            match held {
                true => store.judge_reading(&id, &read.reading).map_err(|why| self.ledger_fault(why))?,
                false => None,
            }
        };
        let kept = kept.ok_or_else(|| self.judge_refused(NO_SUCH_JUDGE_READING, &format!("no reading {} in this Job", read.reading)))?;
        let _ = self.published_hosted(&id).await;
        Ok(reading_of(&kept))
    }

    /// The Job's readings on its record. One the store says is reading that no task of this process holds
    /// died with the Fleet that ran it, and reads as failed until a press keeps it so.
    pub(crate) fn judge_readings_on_record(&self, id: &str, kept: &[KeptJudgeReading]) -> Vec<JudgeReading> {
        kept.iter()
            .map(|one| {
                // A set held this instant is a reading being started or ended, so it counts as going.
                let going = || {
                    self.review_going().try_lock().map_or(true, |going| going.contains(&going_key(id, &one.kind, &one.artifact_id)))
                };
                match one.state == READING && !going() {
                    true => reading_of(&self.died(one.clone())),
                    false => reading_of(one),
                }
            })
            .collect()
    }

    /// The call, from the brief to the kept answer. **Whatever happens the reading ends**, so nothing is
    /// left reading.
    async fn judge_driven(self: Arc<Self>, id: String, mut reading: KeptJudgeReading, key: String) {
        match self.judge_read(&id, &reading).await {
            Ok((notes, cost)) => {
                reading.state = DONE.to_string();
                reading.notes = notes;
                reading.cost_micros = cost;
            }
            Err((said, cost)) => {
                reading.state = FAILED.to_string();
                reading.failed = Some(said);
                reading.cost_micros = cost;
            }
        }
        reading.finished_at = Some(self.now().as_str().to_string());
        self.judge_kept(&id, &reading).await;
        self.review_going().lock().await.remove(&key);
    }

    /// One call on `models.review`, shown the person's words, the artifact's and the repository's rules.
    async fn judge_read(&self, id: &str, reading: &KeptJudgeReading) -> Read {
        let spent = |said: &str| (said.to_string(), None);
        let reviewing = self.reviewing().map_err(|why| (format!("no agent could be asked: {why:?}"), None))?;
        let (session, hosting) = self.judged_job(id).await.map_err(|_| spent("the Job is gone"))?;
        let served = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map_err(|_| spent("the repository is not served"))?;
        let rows = self.rows_of(id).await.map_err(|_| spent("the thread would not read"))?;
        let mut his = rows.iter().filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        });
        let request = his.next().unwrap_or_default();
        let later: Vec<String> = his.collect();
        let artifact = ipc::JudgeArtifact {
            kind: kind_of(&reading.kind),
            id: reading.artifact_id.clone(),
            title: reading.title.clone(),
        };
        let content = self.judge_content(id, &session, &hosting, &served, &rows, &artifact).await.map_err(|why| spent(&why))?;
        let cap = self.tuned(|dials| dials.standing_rules_bytes);
        let standing = crate::judging::standing(served.manifest(), served.root(), cap).told(&self.prompts().wording());
        let asked = brief(&Shown { request: &request, later: &later, artifact: &content, prompt: &reading.prompt, standing: &standing });

        let ask = Ask::put(reviewing.model.clone(), &asked, reviewing.environment.clone())
            .and_then(|ask| Ok(ask.reading(Reading::checkout(served.root(), READ_TURNS)?)))
            .map_err(|why| (format!("the question could not be put: {why:?}"), None))?;
        let (answered, micros) = asked_and_costed(&reviewing, &ask).await;
        let answer = answered.map_err(|why| {
            // The full output is Fleet's to keep, and the card gets one line of it.
            let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a Judge's call did not answer")
                .with_field("job", FieldValue::Str(id.to_string()))
                .with_field("artifact", FieldValue::Str(format!("{} {}", reading.kind, reading.artifact_id)))
                .with_field("why", FieldValue::Str(why.to_string()));
            crate::transcript::note_aloud(&envelope);
            (why.in_a_line("judge"), micros)
        })?;
        let root = served.root().to_string();
        let file = move |path: &str| std::fs::read_to_string(Path::new(&root).join(path)).ok();
        let notes = notes_read(&answer, &asked, &content, &file).map_err(|why| (format!("the answer was refused: {why}"), micros))?;
        Ok((notes.iter().map(note_kept).collect(), micros))
    }

    /// Keep the reading and publish the Job. **A store that will not take it does not stop the reading**:
    /// the Job is published from what the store holds, and a press reads it again.
    async fn judge_kept(&self, id: &str, kept: &KeptJudgeReading) {
        let _ = self.store().lock().await.keep_judge_reading(id, kept);
        let _ = self.published_hosted(id).await;
    }

    fn died(&self, mut kept: KeptJudgeReading) -> KeptJudgeReading {
        kept.state = FAILED.to_string();
        kept.failed = Some(DIED.to_string());
        kept.finished_at = Some(self.now().as_str().to_string());
        kept
    }

    /// The Job a Judge is asked about: one Fleet hosts, or it is unknown.
    async fn judged_job(&self, id: &str) -> Result<(KeptSession, KeptHosting), Refusal> {
        let store = self.store().lock().await;
        let found = store.session(id).and_then(|session| Ok((session, store.hosting(id)?))).map_err(|why| self.ledger_fault(why))?;
        match found {
            (Some(session), Some(hosting)) => Ok((session, hosting)),
            _ => Err(self.judge_refused(NO_SUCH_SESSION, &format!("no Job Fleet hosts is named {id}"))),
        }
    }

    /// The reading the Job holds, and where in it the note is.
    async fn judge_note_held(&self, session: &SessionId, reading: &str, note: &str) -> Result<(String, KeptJudgeReading, usize), Refusal> {
        let id = session.as_str().trim().to_string();
        self.judged_job(&id).await?;
        let kept = self.store().lock().await.judge_reading(&id, reading).map_err(|why| self.ledger_fault(why))?;
        let kept = kept.ok_or_else(|| self.judge_refused(NO_SUCH_JUDGE_READING, &format!("no reading {reading} in this Job")))?;
        let at = kept
            .notes
            .iter()
            .position(|one| one.id == note)
            .ok_or_else(|| self.judge_refused(NO_SUCH_JUDGE_NOTE, &format!("no note {note} in this reading")))?;
        Ok((id, kept, at))
    }

    async fn judge_note_settled(&self, id: &str, kept: &KeptJudgeReading, at: usize, how: JudgeNoteHow) -> Result<JudgeReading, Refusal> {
        let now = self.now().as_str().to_string();
        let settled = {
            let mut store = self.store().lock().await;
            store.settle_judge_note(id, &kept.id, &kept.notes[at].id, how_word(how), &now).map_err(|why| self.ledger_fault(why))?;
            store.judge_reading(id, &kept.id).map_err(|why| self.ledger_fault(why))?
        };
        let _ = self.published_hosted(id).await;
        Ok(reading_of(settled.as_ref().unwrap_or(kept)))
    }

    fn judge_refused(&self, code: &'static str, said: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(code, said, self.run_id()))
    }
}

impl<H, V, W> JudgeReadings for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn ask_judge(self: Arc<Self>, ask: AskJudge) -> Result<JudgeReading, Refusal> {
        self.judge_asked(ask).await
    }

    async fn send_judge_note(self: Arc<Self>, send: SendJudgeNote) -> Result<JudgeReading, Refusal> {
        self.judge_note_sent(send).await
    }

    async fn dismiss_judge_note(self: Arc<Self>, dismiss: DismissJudgeNote) -> Result<JudgeReading, Refusal> {
        self.judge_note_dismissed(dismiss).await
    }

    async fn mark_judge_read(self: Arc<Self>, read: MarkJudgeRead) -> Result<JudgeReading, Refusal> {
        self.judge_marked_read(read).await
    }
}
