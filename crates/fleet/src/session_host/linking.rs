//! A Session that hands a group of its plan to a Session of its own, and follows it. Since the
//! linked Sessions. `docs/concepts/session.md`, *Linked Sessions*.
//!
//! **A group rides on both link rows** (`detail.group` on the child's `started_by` and the parent's
//! `started_to`), so each ledger serves it and no table holds it. **The brief is Fleet's**: the
//! group's tasks word for word, then the parent's handoff, so the child needs nothing read back from
//! the parent. The agent's own `start_another_session` holds that brief for the person; the person's
//! `hand_to_session` sends it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{HandToSession, ManifestId, SendSessionMessage, SessionHeld, SessionRow, SessionVoice, StartSession};
use store::Holder;

use super::agent_starting::Words;
use crate::daemon::Fleet;

/// On a link row: the id of the parent's plan group the child was handed.
pub(crate) const GROUP: &str = "group";

const GROUP_UNKNOWN: &str = "fleet.group_unknown";
const GROUP_HANDED: &str = "fleet.group_handed";
const STARTED_TO: &str = "started_to";

/// How many of the parent's last messages go in the handoff, and how much of each.
const RECENT_ROWS: usize = 6;
const RECENT_CHARS: usize = 400;

fn cut(text: &str) -> String {
    let text = text.trim();
    match text.char_indices().nth(RECENT_CHARS) {
        Some((at, _)) => format!("{}…", text[..at].trim_end()),
        None => text.to_string(),
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
    /// The title and the brief of handing `group` of `parent`'s plan to a Session: the group's tasks
    /// word for word, its paths, then the parent's handoff. Refused 422 where the plan holds no such
    /// group and 409 where the group has a Session already.
    pub(super) async fn group_brief(&self, parent: &str, group: &str) -> Result<(String, String), Refusal> {
        let (plan, held) = {
            let store = self.store().lock().await;
            (
                store.session_plan(parent).map_err(|why| self.ledger_fault(why))?,
                store.attachments_of(&Holder::session(parent)).map_err(|why| self.ledger_fault(why))?,
            )
        };
        let found = plan.as_ref().and_then(|plan| plan.groups.iter().find(|one| one.id == group));
        let Some(found) = found else {
            return Err(self.hosted_refusal(
                GROUP_UNKNOWN,
                &format!("the plan of this job holds no group `{group}`; record_plan names each group's id"),
            ));
        };
        if held.iter().any(|one| one.kind == STARTED_TO && one.detail.get(GROUP).is_some_and(|handed| handed == group)) {
            return Err(Refusal::IllegalMove(ipc::WireError::raised(
                GROUP_HANDED,
                format!("the group `{group}` is handed to a job already"),
                self.run_id(),
            )));
        }
        let mut brief = format!("Group: {}\n\nTasks:\n", found.title);
        for task in &found.tasks {
            brief.push_str(&format!("- {}\n", task.text));
        }
        if !found.paths.is_empty() {
            brief.push_str(&format!("\nPaths: {}\n", found.paths.join(", ")));
        }
        brief.push_str(&format!("\n{}", self.handoff_of(parent).await?));
        Ok((found.title.clone(), brief))
    }

    /// What a Session handed work starts knowing of the one that handed it: its title and id, where
    /// it works, its open pull requests and the last of what was said in its thread.
    pub(super) async fn handoff_of(&self, parent: &str) -> Result<String, Refusal> {
        let (session, held) = {
            let store = self.store().lock().await;
            (
                store.session(parent).map_err(|why| self.ledger_fault(why))?,
                store.attachments_of(&Holder::session(parent)).map_err(|why| self.ledger_fault(why))?,
            )
        };
        let title = session.and_then(|one| one.title).unwrap_or_else(|| parent.to_string());
        let mut out = format!("Handoff from {title} ({parent})");
        for one in held.iter().filter(|one| one.state == store::AttachmentState::Standing) {
            match one.kind.as_str() {
                "slot" => out.push_str(&format!("\nSlot {}", one.target)),
                "branch" => out.push_str(&format!("\nBranch {}", one.target)),
                "pr" => out.push_str(&format!("\nPull request #{}", one.target)),
                _ => {}
            }
        }
        let rows = self.rows_of(parent).await?;
        let said: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                SessionRow::Message { from, text, .. } if !text.trim().is_empty() => Some(match from {
                    SessionVoice::You => format!("You: {}", cut(text)),
                    SessionVoice::Agent => format!("Agent: {}", cut(text)),
                    SessionVoice::Session { title, .. } => format!("{title}: {}", cut(text)),
                }),
                _ => None,
            })
            .collect();
        if !said.is_empty() {
            out.push_str("\n\nRecent thread:");
            for line in said.iter().skip(said.len().saturating_sub(RECENT_ROWS)) {
                out.push_str(&format!("\n{line}"));
            }
        }
        Ok(out)
    }

    /// `hand_to_session`: the person hands `group` of a Session's plan to a Session of its own. **The
    /// brief is sent, not held**, since the press is the person's; a send that is refused leaves it
    /// held, where the new Session's waiting item offers it.
    pub(crate) async fn group_handed(self: Arc<Self>, hand: HandToSession) -> Result<SessionHeld, Refusal> {
        let parent = hand.session_id.as_str().to_string();
        let group = hand.group.trim().to_string();
        let (_, hosting) = self.session_and_hosting(&parent).await?;
        let (title, brief) = self.group_brief(&parent, &group).await?;
        let made = self
            .start_session(StartSession {
                manifest_id: ManifestId::carried(&hosting.manifest_id),
                title: Some(title.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
            })
            .await?;
        let held = self.held_for_person(&parent, made, Words { title, message: brief.clone() }, false, Some(&group), true).await?;
        let _ = Arc::clone(&self)
            .send_session_message(SendSessionMessage {
                session_id: held.session_id.clone(),
                text: brief,
                attachments: Vec::new(),
                mentions: Vec::new(),
                aim: None,
            })
            .await;
        Ok(held)
    }
}
