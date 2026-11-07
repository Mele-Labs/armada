//! Which Jobs a Job may `SendMessage`, asked by the mod before the tool runs. `message_reach`.
//!
//! **Its own tree and what the person named.** A Job writes to the Jobs that started it, to the
//! Jobs it started, and to a Job the person tagged in a message to it, whose address Fleet gave it
//! (`rows::mention_line`). Any other Job is the person's or the Operator's to reach, and the
//! Operator does it with `tell_session`, never with this tool.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{MessageReach, SessionRow, SessionVoice, TagKind};

use super::address_of;
use crate::daemon::Fleet;
use crate::lifecycles::taking_on::started_by;

/// How far up the starter chain the walk goes: past any chain a person builds, short of a cycle.
const HIGHEST: usize = 32;

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
    /// Whether the Job holding `caller`'s connection may write to `to`.
    pub(super) async fn message_reach_of(&self, caller: Caller, to: &str) -> Result<MessageReach, Refusal> {
        let Some(from) = self.session_holding(&caller) else {
            return Ok(MessageReach::Allowed {});
        };
        let target = {
            let store = self.store().lock().await;
            let hostings = store.hostings().map_err(|why| self.ledger_fault(why))?;
            hostings
                .into_iter()
                .map(|hosting| hosting.session_id)
                .find(|id| address_of(id) == to)
        };
        let Some(target) = target else {
            return Ok(MessageReach::Allowed {});
        };
        if self.reaches(&from, &target).await? {
            return Ok(MessageReach::Allowed {});
        }
        Ok(MessageReach::Refused { message: refused_to(to) })
    }

    /// Whether Job `from` may write to Job `target`. Fleet's own delivery asks it too, where the mod's hooks are skipped.
    pub(super) async fn reaches(&self, from: &str, target: &str) -> Result<bool, Refusal> {
        Ok(target == from
            || self.started_above(from).await?.iter().any(|id| id == target)
            || self.started_tree(from).await?.iter().any(|(id, _)| id == target)
            || self.named_by_the_person(from, target).await?)
    }

    /// The Jobs that started `id`, nearest first.
    async fn started_above(&self, id: &str) -> Result<Vec<String>, Refusal> {
        let store = self.store().lock().await;
        let mut chain: Vec<String> = Vec::new();
        let mut at = id.to_string();
        for _ in 0..HIGHEST {
            let Some(session) = store.session(&at).map_err(|why| self.ledger_fault(why))? else {
                break;
            };
            let row = self.ledger_row(&store, &session)?;
            let Some(parent) = started_by(&row.attachments).map(str::to_string) else {
                break;
            };
            if parent == id || chain.contains(&parent) {
                break;
            }
            chain.push(parent.clone());
            at = parent;
        }
        Ok(chain)
    }

    /// Whether the person tagged `target` in a message to `from`.
    async fn named_by_the_person(&self, from: &str, target: &str) -> Result<bool, Refusal> {
        Ok(self.rows_of(from).await?.iter().any(|row| match row {
            SessionRow::Message { from: SessionVoice::You, tags, .. } => {
                tags.iter().any(|tag| tag.kind == TagKind::Session && tag.id == target)
            }
            _ => false,
        }))
    }
}

/// Why a `SendMessage` to `to` is refused, as the hook and Fleet's own delivery both say it.
pub(super) fn refused_to(to: &str) -> String {
    format!("`SendMessage` to {to} is refused: a Job writes only to the Jobs that started it, the Jobs it started and the Jobs the person names to it")
}
