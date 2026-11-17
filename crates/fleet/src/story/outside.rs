//! An ask said outside a Job: words to the Operator, a note on a screen, dictation. The Operator reads them and files each that
//! asks for something, under the features it belongs to. `docs/concepts/job-story.md`, *An ask from outside a Job*.
//!
//! **The same ask rows, with no Job.** `session_id` and `said_in` are empty, the words are kept as a message of that empty
//! Job under the key `note:<id>` or the ask's own id, and the ask opens in the backlog since nothing builds it yet.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Ask, FileAsk};
use store::{ask_state, message_kind, sorted_by, KeptAsk, KeptMessage};

use super::asks::ask_wire;
use super::sorting;
use crate::daemon::Fleet;

const WORDS_BLANK: &str = "fleet.ask_words_blank";
use crate::feature_library::PROJECT_UNKNOWN as PROJECT;

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
    /// `file_ask`. **Nothing is kept on a refusal**, and a note already filed answers the ask it made.
    pub(crate) async fn ask_filed(&self, caller: Option<api::Caller>, file: FileAsk) -> Result<Ask, Refusal> {
        self.operator_called(caller)?;
        let project = file.project.trim().to_string();
        let words = file.words.trim().to_string();
        if words.is_empty() {
            return Err(self.hosted_refusal(WORDS_BLANK, "an ask needs his words, and these are blank"));
        }
        if !self.projects_held().running().any(|one| one.id().as_str() == project) {
            return Err(self.hosted_refusal(PROJECT, &format!("no project is named `{project}`")));
        }
        let key = file.note.as_deref().map(str::trim).filter(|note| !note.is_empty()).map(|note| format!("note:{note}"));
        let mut store = self.store().lock().await;
        if let Some(key) = &key {
            let filed = store.message_sorted("", key).map_err(|why| self.ledger_fault(why))?.and_then(|one| one.ask_id);
            if let Some(kept) = filed.map(|id| store.ask(&id)).transpose().map_err(|why| self.ledger_fault(why))?.flatten() {
                return ask_wire(&store, &kept).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.story_not_found(super::asks::NO_SUCH_ASK, format!("no ask is named {}", kept.ask_id)));
            }
        }
        let mut slugs: Vec<String> = Vec::new();
        for slug in file.features.iter().map(|slug| slug.trim()).filter(|slug| !slug.is_empty()) {
            let held = crate::feature_library::slug_held(&store, &project, slug).map_err(|why| self.ledger_fault(why))?;
            let held = held.ok_or_else(|| self.feature_unknown(slug))?;
            if !slugs.contains(&held) {
                slugs.push(held);
            }
        }
        let now = self.now().as_str().to_string();
        let ask_id = self.mint().ulid().as_str().to_lowercase();
        let title = file.title.as_deref().map(str::trim).filter(|title| !title.is_empty()).map(|title| sorting::cut(title, sorting::TITLE_CHARS)).unwrap_or_else(|| sorting::title_from(&words));
        let ask = KeptAsk {
            ask_id: ask_id.clone(),
            session_id: String::new(),
            manifest_id: None,
            project: Some(project),
            kind: sorting::ask_kind_word(file.kind).to_string(),
            title,
            state: ask_state::BACKLOG.to_string(),
            building: None,
            opened_at: now.clone(),
            state_at: now.clone(),
        };
        store.keep_ask(&ask).map_err(|why| self.ledger_fault(why))?;
        store.keep_ask_features(&ask_id, &slugs).map_err(|why| self.ledger_fault(why))?;
        store
            .keep_message(&KeptMessage {
                session_id: String::new(),
                row_id: key.unwrap_or_else(|| ask_id.clone()),
                kind: message_kind::NEW_ASK.to_string(),
                ask_id: Some(ask_id),
                words,
                at: now.clone(),
                sorted_by: sorted_by::FLEET.to_string(),
                sorted_at: now,
            })
            .map_err(|why| self.ledger_fault(why))?;
        ask_wire(&store, &ask).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.story_not_found(super::asks::NO_SUCH_ASK, format!("no ask is named {}", ask.ask_id)))
    }
}
