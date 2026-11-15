//! The notes of the build the owner runs: what each offered branch told him to look at, read for the branches the
//! running preview holds, marked looked at, and answered with his words. `docs/concepts/fleet.md`, *Offering a
//! branch for the preview*. The offers a note is kept from are `crate::previewing`.
//!
//! **A note outlives its offer**: the build he loaded still holds the branch after its Job ended or its tip
//! landed. Feedback goes to the Job where it is live, as his message, and starts a new Job where it has ended.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    BuildNote, BuildNoteFeedbackSent, BuildNotes, DispatchJob, DispatchMessage, DispatchThen, Event, ManifestId, MarkBuildNoteLookedAt,
    PreviewStep, SendBuildNoteFeedback, SendSessionMessage, SessionId, StartSession, WireError,
};
use store::{KeptBuildNote, SessionState};

use crate::daemon::Fleet;

const NOTE_UNKNOWN: &str = "fleet.build_note_unknown";
const WORDS_EMPTY: &str = "fleet.build_note_words_empty";
const NO_REPOSITORY: &str = "fleet.build_note_no_repository";

/// What the Job is told, led by which note it answers.
fn answer_text(note: &KeptBuildNote, words: &str) -> String {
    format!("On your preview note \"{}\": {words}", note.about)
}

/// What the Job is told when the owner marks its note looked at.
fn looked_text(note: &KeptBuildNote) -> String {
    format!("The owner looked at your preview note \"{}\" on {} at {}.", note.about, note.branch, note.tip.get(..7).unwrap_or(&note.tip))
}

/// What a new Job starts from where the note's Job has ended: his words, then what he was looking at.
fn started_brief(note: &KeptBuildNote, title: &str, words: &str) -> String {
    let mut brief = format!("His feedback on a preview note of the Job \"{title}\", which has ended:\n\n{words}\n\nThe note said: {}", note.about);
    if let Some(look) = &note.look {
        brief.push_str(&format!("\nWhat to look at: {look}"));
    }
    for (at, step) in note.steps.iter().enumerate() {
        let act = step.act.as_deref().map(|act| format!(" ({act})")).unwrap_or_default();
        brief.push_str(&format!("\n{}. {}{act}", at + 1, step.say));
    }
    brief.push_str(&format!("\n\nThe branch is {} at {}.", note.branch, note.tip));
    brief
}

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
    /// `get_build_notes`: the note of each (branch, tip) the running preview build holds, in the order the branches
    /// were offered. Empty, and no `commit`, off the preview.
    pub(crate) async fn build_notes_read(&self) -> BuildNotes {
        let Some((commit, tips)) = self.preview_build() else {
            return BuildNotes { commit: None, notes: Vec::new() };
        };
        let store = self.store().lock().await;
        let mut kept: Vec<KeptBuildNote> = tips.iter().filter_map(|(branch, tip)| store.build_note_of(branch, tip).ok().flatten()).collect();
        kept.sort_by(|one, other| (&one.offered_at, &one.branch).cmp(&(&other.offered_at, &other.branch)));
        let notes = kept
            .into_iter()
            .map(|one| {
                let session = store.session(&one.session_id).ok().flatten();
                BuildNote {
                    title: session.as_ref().and_then(|row| row.title.clone()).unwrap_or_else(|| one.session_id.clone()),
                    job_ended: session.as_ref().is_none_or(|row| row.state == SessionState::Ended),
                    session_id: SessionId::carried(one.session_id.as_str()),
                    looked_at: one.looked_at.is_some(),
                    branch: one.branch,
                    tip: one.tip,
                    about: one.about,
                    look: one.look,
                    steps: one.steps.into_iter().map(|step| PreviewStep { say: step.say, act: step.act }).collect(),
                }
            })
            .collect();
        BuildNotes { commit: Some(commit), notes }
    }

    /// Publish the notes where they differ from the ones last sent. The first answer after Fleet starts is sent.
    pub(crate) async fn build_notes_moved(&self) {
        let now = self.build_notes_read().await;
        let changed = {
            let mut sent = self.previewing().notes_sent.lock().expect("not poisoned");
            let changed = sent.as_ref() != Some(&now);
            if changed {
                *sent = Some(now.clone());
            }
            changed
        };
        if changed {
            self.publish(Event::BuildNotesChanged(now));
        }
    }

    /// `mark_build_note_looked_at`: keep the mark against the (branch, tip), and answer the notes now.
    pub(crate) async fn build_note_looked_at(self: &Arc<Self>, asked: MarkBuildNoteLookedAt) -> Result<BuildNotes, Refusal> {
        let at = self.now().as_str().to_string();
        let before = self.store().lock().await.build_note_of(&asked.branch, &asked.tip).map_err(|why| self.ledger_fault(why))?;
        let marked = self.store().lock().await.keep_build_note_looked_at(&asked.branch, &asked.tip, &at).map_err(|why| self.ledger_fault(why))?;
        if !marked {
            return Err(self.build_note_unknown(&asked.branch, &asked.tip));
        }
        // The note's Job is told the first time, where it is live, as it is told the rest of its preview news.
        if let Some(note) = before.filter(|note| note.looked_at.is_none()) {
            let live = self.store().lock().await.session(&note.session_id).ok().flatten().is_some_and(|row| row.state != SessionState::Ended);
            if live {
                let _ = self.told_news_by_fleet(&note.session_id, &looked_text(&note), ipc::Notice::BuildMoved).await;
            }
        }
        self.build_notes_moved().await;
        Ok(self.build_notes_read().await)
    }

    /// `send_build_note_feedback`: his words to the note's Job where it is live, else to a new Job started at once.
    pub(crate) async fn build_note_answered(self: &Arc<Self>, asked: SendBuildNoteFeedback) -> Result<BuildNoteFeedbackSent, Refusal> {
        let words = asked.words.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(WORDS_EMPTY, "words on the note: there are none to send"));
        }
        let (note, session) = {
            let store = self.store().lock().await;
            let note = store.build_note_of(&asked.branch, &asked.tip).map_err(|why| self.ledger_fault(why))?;
            let note = note.ok_or_else(|| self.build_note_unknown(&asked.branch, &asked.tip))?;
            let session = store.session(&note.session_id).map_err(|why| self.ledger_fault(why))?;
            (note, session)
        };
        let live = session.as_ref().is_some_and(|row| row.state != SessionState::Ended);
        if live {
            let terminal = self.terminal_session(&note.session_id).await?;
            let sent = SendSessionMessage {
                session_id: SessionId::carried(note.session_id.as_str()),
                text: answer_text(&note, &words),
                attachments: Vec::new(),
                mentions: Vec::new(),
                sketches: Vec::new(),
            };
            Arc::clone(self).send_known(terminal, sent).await?;
            return Ok(BuildNoteFeedbackSent { session_id: SessionId::carried(note.session_id.as_str()), started: false });
        }
        let title = session.as_ref().and_then(|row| row.title.clone()).unwrap_or_else(|| note.session_id.clone());
        let manifest = match session.and_then(|row| row.manifest_id) {
            Some(manifest) => manifest,
            None => self.preview_place().map(|(_, served)| served.manifest().id().as_str().to_string()).ok_or_else(|| {
                self.hosted_refusal(NO_REPOSITORY, "the repository the note's Job worked in: Fleet serves none with a preview")
            })?,
        };
        let brief = started_brief(&note, &title, &words);
        let start = StartSession {
            project: None,
            manifest_id: ManifestId::carried(&manifest),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: Some(brief.clone()),
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        let then = vec![DispatchThen::Message(DispatchMessage { text: brief, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
        let record = self.dispatch_started(DispatchJob { start, kind: None, then }).await?;
        Ok(BuildNoteFeedbackSent { session_id: record.id, started: true })
    }

    fn build_note_unknown(&self, branch: &str, tip: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NOTE_UNKNOWN, format!("no preview note is kept for {branch} at {tip}"), self.run_id()))
    }
}
