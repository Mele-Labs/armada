//! `start_mod_session`: a Mod Job, a hosted Job on Armada's own repository for modding Armada
//! itself, started on what the person pointed at. Where that repository is not served, the Job runs
//! in the mods folder and makes mods only. `docs/concepts/mods.md`, *A Mod Job*.
//!
//! **The first message is a pinned note's**, sent by `send_session_note`'s road with the Mod brief
//! ahead of his words, so the turn runs under the rules any message is under. Where he pointed at
//! nothing it is his words alone, sent by `send_session_message`'s road with the brief ahead.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ManifestId, ModSessionStarted, SendSessionMessage, SendSessionNote, StartModSession, StartSession};

use super::serving::title_of;
use super::Marked;
use crate::daemon::Fleet;

/// A start that names neither a note nor his words, both, or words with nothing in them. A 422.
const UNACCEPTABLE_MOD_START: &str = "fleet.unacceptable_mod_start";

/// A Mod Job's title where neither the start nor his words give one.
const UNTITLED: &str = "Mod";

/// What a Mod Job is told ahead of the note. **Sent as it stands**, so it is short and plain.
const BRIEF: &str = "This is a Mod Job: the owner pressed Mod on a spot in Armada, and the change is to Armada itself. \
Load the `armada:armada-mods` skill and reach for a mod first: a theme, a layout, words, a Fleet rule or a tracker. \
A mod applies live once `validate_mod` passes, and the owner switches it off from this Job if he does not like it. \
Where the ask needs more than a mod can say, tell the owner so, change Armada's source on this Job's branch, \
then put `scripts/preview --restart` in waiting_for as a run item, so he can bring it in.";
/// [`BRIEF`] for a Mod Job in the mods folder, where this Fleet does not serve Armada's own repository.
const MODS_FOLDER_BRIEF: &str = "This is a Mod Job in the mods folder: the owner pressed Mod on a spot in Armada, \
and this Fleet does not serve Armada's own repository, so the change is a mod. \
Load the `armada:armada-mods` skill and make a theme, a layout, words, a Fleet rule or a tracker. \
A mod applies live once `validate_mod` passes, and the owner switches it off from this Job if he does not like it. \
Armada's source is not here to change: where the ask needs more than a mod can say, tell the owner so.";

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
    /// Start a Mod Job and send it the brief and the note.
    pub(crate) async fn mod_job_started(self: Arc<Self>, start: StartModSession) -> Result<ModSessionStarted, Refusal> {
        let StartModSession { title, model, effort, note, said } = start;
        let said = said.map(|words| words.trim().to_string());
        let first = match (note, said) {
            (Some(note), None) => First::Note(note),
            (None, Some(words)) if !words.is_empty() => First::Words(words),
            (None, Some(_)) => {
                return Err(self.hosted_refusal(UNACCEPTABLE_MOD_START, "a Mod Job needs his words: `said` is empty"))
            }
            (None, None) => {
                return Err(self.hosted_refusal(UNACCEPTABLE_MOD_START, "a Mod Job needs a `note` or `said`, and has neither"))
            }
            (Some(_), Some(_)) => {
                return Err(self.hosted_refusal(UNACCEPTABLE_MOD_START, "a Mod Job takes a `note` or `said`, not both"))
            }
        };
        let (manifest, marked, brief) = match self.armadas_own() {
            Some(manifest) => (manifest, Marked::Mod, BRIEF),
            None => {
                let mods = self.mods_dir();
                std::fs::create_dir_all(&mods)
                    .map_err(|why| self.mod_fault(format!("the mods folder could not be made: {why}")))?;
                // Not read: a Job in the mods folder names no Manifest.
                (ManifestId::carried(""), Marked::ModsFolder, MODS_FOLDER_BRIEF)
            }
        };
        let title = title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty())
            .or_else(|| title_of(first.words()))
            .unwrap_or_else(|| UNTITLED.to_string());
        let begun = StartSession {
            manifest_id: manifest,
            title: Some(title.clone()),
            model,
            effort,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
        };
        let session_id = self.started_hosted(begun, marked).await?.id;
        // **Not an error**: the Job exists, so the answer is 200 and `send_failed` says why
        // nothing was sent.
        let sent = match first {
            First::Note(note) => Arc::clone(&self)
                .note_sent_after(SendSessionNote { session_id: session_id.clone(), note }, Some(brief))
                .await
                .map(|_| ()),
            First::Words(words) => Arc::clone(&self).words_sent_after(session_id.clone(), words, brief).await,
        };
        let send_failed = sent.err().map(|refused| refused.error().message.clone());
        Ok(ModSessionStarted { session_id, title, send_failed })
    }
}

/// What a Mod Job's first message is made of.
enum First {
    /// What he pointed at.
    Note(ipc::SessionNote),
    /// His words, trimmed and not empty.
    Words(String),
}

impl First {
    fn words(&self) -> &str {
        match self {
            First::Note(note) => &note.said,
            First::Words(words) => words,
        }
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
    /// His words as a message, with `brief` ahead of them in what the agent reads and the thread
    /// keeping the words alone: `note_sent_after` for a Mod Job pressed on nothing.
    async fn words_sent_after(
        self: Arc<Self>,
        session_id: ipc::SessionId,
        words: String,
        brief: &str,
    ) -> Result<(), Refusal> {
        let terminal = self.terminal_session(session_id.as_str()).await?;
        let instead = format!("{brief}\n\n{words}");
        let sent = SendSessionMessage { session_id, text: words, attachments: Vec::new(), mentions: Vec::new() };
        self.send_known_added(terminal, sent, None, Some(instead)).await.map(|_| ())
    }
}
