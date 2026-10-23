//! Teaching as the wire has it: read into a proposal, a link rule kept, listed and forgotten, and a
//! live Session told the note the person kept.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Teaching};
use ipc::{
    ForgottenTaughtLink, Instant, ReadTeaching, SaveTeaching, SavedTeaching, TaughtLinks, TeachingProposal, WireError,
};

use super::{link, INPUT_REFUSED, LINK_REFUSED};
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::run_memory::places::key_of;

/// An id that names no rule. A 404.
const NO_SUCH_LINK: &str = "fleet.no_such_taught_link";
/// The store would not read or keep it. A 500.
const FAILED: &str = "fleet.teaching_failed";

/// A kept rule as the wire has it. `root` is the repository it was read for, which a rule with a
/// repository belongs to.
fn wired(kept: store::TaughtLink, root: Option<&str>) -> ipc::TaughtLink {
    ipc::TaughtLink {
        id: kept.id.unwrap_or_default().to_string(),
        pattern: kept.pattern,
        address: kept.address,
        example: kept.example,
        repository: kept.repository.and(root.map(str::to_string)),
        made_at: Instant::carried(rfc3339_utc(kept.made_at)),
    }
}

impl<H, V, W> Teaching for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn read_teaching(&self, teach: ReadTeaching) -> Result<TeachingProposal, Refusal> {
        self.teaching_read(teach).await
    }

    async fn save_teaching(self: Arc<Self>, save: SaveTeaching) -> Result<SavedTeaching, Refusal> {
        let note = save.note.as_deref().map(str::trim).filter(|note| !note.is_empty());
        if save.link.is_none() && note.is_none() {
            return Err(Refusal::Unacceptable(WireError::raised(
                INPUT_REFUSED,
                "There is nothing to keep: no link rule and no note.",
                self.run_id(),
            )));
        }
        let root = save.repository.as_deref().map(|named| self.run_repository(named)).transpose()?;
        let kept = match &save.link {
            Some(rule) => {
                if let Some(why) = link::fault(&rule.pattern, &rule.address, &rule.example) {
                    return Err(Refusal::Unacceptable(WireError::raised(LINK_REFUSED, why, self.run_id())));
                }
                let mut row = store::TaughtLink {
                    id: None,
                    pattern: rule.pattern.clone(),
                    address: rule.address.clone(),
                    example: rule.example.clone(),
                    repository: root.as_deref().map(key_of),
                    made_at: self.now().epoch_millis().unwrap_or(0).max(0),
                };
                let id = self.store().lock().await.save_taught_link(&row).map_err(|why| self.teaching_fault(why))?;
                row.id = Some(id);
                Some(wired(row, root.as_deref()))
            }
            None => None,
        };
        // The rule is kept, so nobody to tell is not a failure: the Session ended, or Fleet hosts
        // none of that name. A Session mid-turn has the line queued behind it.
        if let (Some(session), Some(note)) = (&save.session_id, note) {
            let _ = self.told_news_by_fleet(session, &format!("From now on: {note}")).await;
        }
        Ok(SavedTeaching { link: kept })
    }

    async fn list_taught_links(&self, repository: Option<String>) -> Result<TaughtLinks, Refusal> {
        let root = repository.map(|named| self.run_repository(&named)).transpose()?;
        let key = root.as_deref().map(key_of).unwrap_or_default();
        let rules = self.store().lock().await.taught_links_for(&key).map_err(|why| self.teaching_fault(why))?;
        Ok(TaughtLinks { links: rules.into_iter().map(|rule| wired(rule, root.as_deref())).collect() })
    }

    async fn forget_taught_link(&self, link_id: String) -> Result<ForgottenTaughtLink, Refusal> {
        let missing = || {
            Refusal::NotFound(WireError::raised(
                NO_SUCH_LINK,
                format!("No link rule has the id {link_id}. It may have been forgotten."),
                self.run_id(),
            ))
        };
        let number: i64 = link_id.parse().map_err(|_| missing())?;
        let gone = self.store().lock().await.forget_taught_link(number).map_err(|why| self.teaching_fault(why))?;
        if !gone {
            return Err(missing());
        }
        Ok(ForgottenTaughtLink { id: link_id })
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
    fn teaching_fault(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(FAILED, why.to_string(), self.run_id()))
    }
}
