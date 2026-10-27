//! The owner's notes on a pull request's lines, kept so the Job a note was sent to can resolve it.
//! Bridge's four calls; the Job's is `session_host::note_resolving`. `docs/concepts/pull-review.md`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestId, NoteSide, NoteStatus, PullRequestNote, PullRequestNotes, SavePullRequestNote, SendPullRequestNotes,
    SessionId,
};
use store::KeptPullRequestNote;

use crate::daemon::Fleet;

/// A note that cannot be made, edited or sent. A 422.
pub(crate) const INVALID: &str = "fleet.pull_request_note_invalid";

/// The note as the wire has it.
pub(crate) fn note_read(row: &KeptPullRequestNote) -> PullRequestNote {
    PullRequestNote {
        id: row.id.clone(),
        repository: row.repository.clone(),
        number: row.number,
        path: row.path.clone(),
        line: row.line,
        side: if row.side == "left" { NoteSide::Left } else { NoteSide::Right },
        body: row.body.clone(),
        status: if row.status == "done" { NoteStatus::Done } else { NoteStatus::Open },
        sent_to: row.sent_to.as_deref().map(SessionId::carried),
        sent_title: row.sent_title.clone(),
        sent_at: row.sent_at.clone(),
        created_at: row.created_at.clone(),
    }
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
    fn note_invalid(&self, said: &str) -> Refusal {
        Refusal::Unacceptable(self.pr_refusal(INVALID, said))
    }

    /// The repository as the forge names it, for a manifest Fleet serves.
    async fn notes_repository(&self, manifest_id: &ManifestId) -> Result<String, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        Ok(self.walk_slug(served.root(), manifest_id).await)
    }

    async fn notes_of(&self, repository: &str, number: u32) -> Result<PullRequestNotes, Refusal> {
        let rows = self
            .store()
            .lock()
            .await
            .pull_request_notes_of(repository, number)
            .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
        Ok(PullRequestNotes { notes: rows.iter().map(note_read).collect() })
    }

    pub(crate) async fn pull_request_notes_listed(
        &self,
        manifest_id: ManifestId,
        number: u32,
    ) -> Result<PullRequestNotes, Refusal> {
        let repository = self.notes_repository(&manifest_id).await?;
        self.notes_of(&repository, number).await
    }

    /// Make a note, or change the body of one that is open and unsent.
    pub(crate) async fn pull_request_note_saved(
        &self,
        manifest_id: ManifestId,
        number: u32,
        save: SavePullRequestNote,
    ) -> Result<PullRequestNote, Refusal> {
        let repository = self.notes_repository(&manifest_id).await?;
        let body = save.body.trim().to_string();
        if body.is_empty() {
            return Err(self.note_invalid("a note needs something to say"));
        }
        let writing = |why| self.refusal(crate::adrift::Adrift::Writing(why));
        let mut store = self.store().lock().await;
        let kept = match save.id {
            Some(id) => {
                let found = store.pull_request_note(&id).map_err(writing)?;
                let Some(found) = found.filter(|found| found.repository == repository && found.number == number) else {
                    return Err(self.note_invalid(&format!("no note {id} on this pull request")));
                };
                if found.status != "open" || found.sent_to.is_some() {
                    return Err(self.note_invalid("a note already sent to a job cannot be edited"));
                }
                store.edit_pull_request_note_body(&id, &body).map_err(writing)?.unwrap_or(found)
            }
            None => {
                let (Some(path), Some(line), Some(side)) = (save.path, save.line, save.side) else {
                    return Err(self.note_invalid("a new note needs a path, a line and a side"));
                };
                let row = KeptPullRequestNote {
                    id: self.mint().ulid().as_str().to_lowercase(),
                    repository,
                    number,
                    path,
                    line,
                    side: if side == NoteSide::Left { "left" } else { "right" }.to_string(),
                    body,
                    status: "open".to_string(),
                    sent_to: None,
                    sent_title: None,
                    sent_at: None,
                    created_at: self.now().as_str().to_string(),
                };
                store.keep_pull_request_note(&row).map_err(writing)?
            }
        };
        Ok(note_read(&kept))
    }

    pub(crate) async fn pull_request_note_removed(
        &self,
        manifest_id: ManifestId,
        number: u32,
        id: String,
    ) -> Result<PullRequestNotes, Refusal> {
        let repository = self.notes_repository(&manifest_id).await?;
        {
            let mut store = self.store().lock().await;
            let writing = |why| self.refusal(crate::adrift::Adrift::Writing(why));
            // Only this pull request's: an id from another is left alone.
            let found = store.pull_request_note(&id).map_err(writing)?;
            if found.is_some_and(|found| found.repository == repository && found.number == number) {
                store.forget_pull_request_note(&id).map_err(writing)?;
            }
        }
        self.notes_of(&repository, number).await
    }

    /// Record that the notes named went to a session. **Sends nothing.**
    pub(crate) async fn pull_request_notes_sent(
        &self,
        manifest_id: ManifestId,
        number: u32,
        send: SendPullRequestNotes,
    ) -> Result<PullRequestNotes, Refusal> {
        let repository = self.notes_repository(&manifest_id).await?;
        {
            let mut store = self.store().lock().await;
            let writing = |why| self.refusal(crate::adrift::Adrift::Writing(why));
            for id in &send.note_ids {
                let found = store.pull_request_note(id).map_err(writing)?;
                if !found.is_some_and(|found| found.repository == repository && found.number == number) {
                    return Err(self.note_invalid(&format!("no note {id} on this pull request")));
                }
            }
            let at = self.now().as_str().to_string();
            store
                .send_pull_request_notes(&send.note_ids, send.session_id.as_str(), &send.title, &at)
                .map_err(writing)?;
        }
        self.notes_of(&repository, number).await
    }
}
