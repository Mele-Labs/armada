//! `start_mod_session`: a Mod Job, a hosted Job on Armada's own repository for modding Armada
//! itself, started on what the person pointed at. `docs/concepts/mods.md`, *A Mod Job*.
//!
//! **The first message is a pinned note's**, sent by `send_session_note`'s road with the Mod brief
//! ahead of his words, so the turn runs under the rules any message is under.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ModSessionStarted, SendSessionNote, StartModSession, StartSession, WireError};

use super::serving::title_of;
use super::Marked;
use crate::daemon::Fleet;
use crate::retro::ARMADAS_OWN;

/// This Fleet does not serve Armada's own repository, so there is nowhere to start the Job. A 422.
const MOD_ARMADA_NOT_SERVED: &str = "fleet.mod_armada_not_served";
/// A Mod Job's title where neither the start nor his words give one.
const UNTITLED: &str = "Mod";

/// What a Mod Job is told ahead of the note. **Sent as it stands**, so it is short and plain.
const BRIEF: &str = "This is a Mod Job: the owner pressed Mod on a spot in Armada, and the change is to Armada itself. \
Load the `armada-mods` skill and reach for a mod first: a theme, a layout, words, a Fleet rule or a tracker. \
A mod applies live once `validate_mod` passes, and the owner switches it off from this Job if he does not like it. \
Where the ask needs more than a mod can say, tell the owner so, change Armada's source on this Job's branch, \
then put `scripts/preview --restart` in waiting_for as a run item, so he can bring it in.";

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
        let StartModSession { title, model, effort, note } = start;
        let manifest = self.armadas_own().ok_or_else(|| {
            Refusal::Unacceptable(WireError::raised(
                MOD_ARMADA_NOT_SERVED,
                format!(
                    "this Fleet does not serve Armada's own repository (`{ARMADAS_OWN}`), so there is \
                     nowhere to start a Mod Job. Serving it, then pressing Mod again, starts one"
                ),
                self.run_id(),
            ))
        })?;
        let title = title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty())
            .or_else(|| title_of(&note.said))
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
        };
        let session_id = self.started_hosted(begun, Marked::Mod).await?.id;
        // **Not an error**: the Job exists, so the answer is 200 and `send_failed` says why
        // nothing was sent.
        let sent = Arc::clone(&self)
            .note_sent_after(SendSessionNote { session_id: session_id.clone(), note }, Some(BRIEF))
            .await;
        let send_failed = sent.err().map(|refused| refused.error().message.clone());
        Ok(ModSessionStarted { session_id, title, send_failed })
    }
}
