//! What Helm's message box offers and runs on: the model and effort a person chose, the commands
//! `/` lists, and the file a message carried.
//!
//! **Offered live and never kept in the thread.** `facts` reaches whoever is watching and is read
//! again from the next `opened`, so a thread file or a replay never carries one.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, StoredFile};
use ipc::{HelmBoxFacts, HelmMessage, ManifestId, TuneHelm, WireError};

use super::conversation::{Conversation, ConversationKey};
use crate::daemon::Fleet;
use crate::session_host::uploads::ATTACHMENT_REFUSED;

/// An effort that is not `low`, `medium` or `high`. A 422.
const UNACCEPTABLE_EFFORT: &str = "fleet.unacceptable_effort";
/// The model and effort kept for a conversation would not read. A 500.
const HELM_TUNING_UNREADABLE: &str = "fleet.helm_tuning_unreadable";

/// Say the commands changed, to everyone watching. Called from the listener, which must not
/// block, so it reads only what the conversation holds.
pub(super) fn commands_changed(conversation: &Conversation, names: Vec<String>) {
    if !conversation.heard_commands(names.clone()) {
        return;
    }
    let (model, effort) = conversation.tuning();
    conversation
        .thread
        .offered_only(HelmMessage::Facts(HelmBoxFacts {
            model,
            effort,
            commands: names,
        }));
}

/// A blank string counts as absent.
fn chosen(word: Option<String>) -> Option<String> {
    word.map(|word| word.trim().to_string())
        .filter(|word| !word.is_empty())
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
    /// The model and effort chosen for a conversation, `None` for the machine's own, and the
    /// copy a listener publishes from brought up to date.
    pub(super) async fn helm_tuning(
        &self,
        key: &ConversationKey,
        conversation: &Conversation,
    ) -> Result<(Option<String>, Option<String>), String> {
        let (model, effort) = self
            .store()
            .lock()
            .await
            .helm_tuning(key.as_str())
            .map_err(super::unanswered::tuning_would_not_read)?;
        conversation.tuned_to(model.clone(), effort.clone());
        Ok((model, effort))
    }

    /// What the box offers now. **Commands are Helm's own once it has spoken**, and until then
    /// the last any Session read, as a Session's are.
    pub(super) async fn helm_facts(
        &self,
        key: &ConversationKey,
        conversation: &Conversation,
    ) -> Result<HelmBoxFacts, Refusal> {
        let (model, effort) = self
            .helm_tuning(key, conversation)
            .await
            .map_err(|why| self.tuning_fault(&why))?;
        let heard = conversation.commands();
        let commands = match heard.is_empty() {
            true => self.hosts().commands(),
            false => heard,
        };
        Ok(HelmBoxFacts {
            model,
            effort,
            commands,
        })
    }

    fn tuning_fault(&self, why: &str) -> Refusal {
        Refusal::Fault(WireError::raised(HELM_TUNING_UNREADABLE, why, self.run_id()))
    }

    /// `tune_helm`: keep the choice, and tell every watcher what the box now offers.
    pub(super) async fn helm_tuned(
        &self,
        tuned: TuneHelm,
        manifest_id: Option<ManifestId>,
    ) -> Result<HelmBoxFacts, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let model = chosen(tuned.model);
        let effort = chosen(tuned.effort);
        if let Some(word) = &effort {
            if crate::tuning::effort_of(word).is_none() {
                return Err(Refusal::Unacceptable(WireError::raised(
                    UNACCEPTABLE_EFFORT,
                    format!("`{word}` is not an effort. Use low, medium or high"),
                    self.run_id(),
                )));
            }
        }
        let conversation = self.helm().open(&key, served.records_root());
        self.store()
            .lock()
            .await
            .keep_helm_tuning(key.as_str(), model.as_deref(), effort.as_deref())
            .map_err(|why| self.tuning_fault(&format!("Helm's model and effort would not keep: {why}")))?;
        conversation.tuned_to(model, effort);
        let facts = self.helm_facts(&key, &conversation).await?;
        conversation
            .thread
            .offered_only(HelmMessage::Facts(facts.clone()));
        Ok(facts)
    }

    /// `get_helm_file`: what a message to Helm carried, found by the id its `asked` row names.
    pub(super) async fn helm_file(
        &self,
        manifest_id: Option<ManifestId>,
        file: String,
    ) -> Result<StoredFile, Refusal> {
        let (served, key) = self.helm_of(manifest_id.as_ref())?;
        let conversation = self.helm().open(&key, served.records_root());
        let (messages, _) = conversation.thread.taken();
        let sent = messages
            .into_iter()
            .find_map(|message| match message {
                HelmMessage::Asked(asked) => asked.files.into_iter().find(|one| one.id == file),
                _ => None,
            })
            .ok_or_else(|| {
                self.hosted_refusal(ATTACHMENT_REFUSED, "nothing was sent under that id")
            })?;
        self.stored_file(
            &key.uploads_in(&self.host().attachments_dir),
            &file,
            sent.media_type,
        )
    }
}
