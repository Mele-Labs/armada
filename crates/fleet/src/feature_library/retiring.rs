//! The owner's answer to a proposal to retire a feature. `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! **Only a feature the librarian proposed for Retire is answered**, so Fleet's proposal stays the one way in. Retire takes
//! the feature out of the catalog and leaves its links where they are. Keep holds until a later check finds other evidence.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AnswerRetire, RetireAnswered, WireError};
use store::upkeep_kind;

use super::reading::Loaded;
use crate::daemon::Fleet;

/// A feature the librarian has not proposed for Retire.
const NOT_PROPOSED: &str = "fleet.retire_not_proposed";

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
    /// `answer_retire`. **Nothing is kept on a refusal.**
    pub(super) async fn retire_answered(&self, caller: Option<api::Caller>, answer: AnswerRetire) -> Result<RetireAnswered, Refusal> {
        self.owner_or_operator_called(caller)?;
        let (project, slug) = (answer.project.trim().to_string(), answer.slug.trim().to_string());
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        let feature = loaded.feature_named(&slug).ok_or_else(|| self.feature_unknown(&slug))?.slug.clone();
        let proposed = store.upkeep_in(&project).map_err(|why| self.ledger_fault(why))?.iter().any(|mark| mark.slug == feature && mark.kind != upkeep_kind::PLANNED);
        if !proposed {
            return Err(Refusal::IllegalMove(WireError::raised(NOT_PROPOSED, format!("the librarian has not proposed to retire `{feature}`"), self.run_id())));
        }
        if answer.retire {
            store.retire_feature(&project, &feature).map_err(|why| self.ledger_fault(why))?;
        } else {
            store.keep_retire_proposal(&project, &feature, &at).map_err(|why| self.ledger_fault(why))?;
        }
        Ok(RetireAnswered { project, slug: feature, retired: answer.retire })
    }
}
