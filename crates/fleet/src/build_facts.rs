//! What the session ledger does with a Session's build facts: the plan and the road are heard into the
//! build record, and the road is entered on the timeline, where the record reads it back from.
//! `docs/concepts/session.md`, *The build record*. The rules are `build_record`'s, the store glue
//! `build_keeping`'s; this is the ledger's side of them.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use std::collections::BTreeMap;

use ipc::{AttachmentState, ReportedTask, SessionFact, TimelineKind, TimelineTone};
use store::{Holder, KeptAttachment, KeptSession, Store, WriteError};

use crate::build_keeping::build_heard;
use crate::build_record::Observed;
use crate::daemon::Fleet;
use crate::timeline::{Fact, Subject};

/// A subagent as `workers` shows it: `Type: what it was asked`.
fn worker_label(detail: &BTreeMap<String, String>) -> String {
    let said = |key: &str| detail.get(key).map(String::as_str).unwrap_or_default();
    [said("type"), said("description")].iter().filter(|part| !part.is_empty()).cloned().collect::<Vec<_>>().join(": ")
}

/// The subagents a Session has running, each as `Type: what it was asked`.
pub(crate) fn workers_of(attachments: &[ipc::Attachment]) -> Vec<String> {
    attachments
        .iter()
        .filter(|one| one.kind == "subagent" && one.state == AttachmentState::Standing)
        .map(|one| worker_label(&one.detail))
        .collect()
}

/// A task list with each subagent's `agentId` turned into its worker label, which is what the pieces
/// keep. The attachment outlives the subagent, so a finished one still resolves; an id with no
/// attachment (heard before it was attached) stays the id and is resolved on the next list.
fn labelled(mut tasks: Vec<ReportedTask>, held: &[KeptAttachment]) -> Vec<ReportedTask> {
    for task in &mut tasks {
        let Some(agent) = task.by.take() else { continue };
        let label = held
            .iter()
            .find(|one| one.kind == "subagent" && one.target == agent)
            .map(|one| worker_label(&one.detail))
            .filter(|label| !label.is_empty());
        task.by = Some(label.unwrap_or(agent));
    }
    tasks
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
    /// A Session's plan or road fact, heard. Answers whether the row shows it. A commit or a push is
    /// entered on the timeline first, so the road is there when the row is next built.
    pub(crate) fn build_told(&self, store: &mut Store, session: &KeptSession, fact: SessionFact, now: &str) -> Result<bool, WriteError> {
        let heard = match fact {
            SessionFact::Tasks { tasks } => {
                Observed::Tasks(labelled(tasks, &store.attachments_of(&Holder::session(&session.id))?))
            }
            SessionFact::PlanLeft => Observed::PlanLeft,
            SessionFact::Committed { branch, commit, subject } => {
                self.road_noted(session, TimelineKind::SessionCommitted, "Committed", branch, now);
                let named = crate::plan_keeping::commit_heard(store, &session.id, &commit, &subject, now)?;
                return Ok(build_heard(store, &session.id, &Observed::Committed, now)? | named);
            }
            SessionFact::Pushed { branch } => {
                self.road_noted(session, TimelineKind::SessionPushed, "Pushed", branch, now);
                Observed::Committed
            }
            _ => return Ok(false),
        };
        build_heard(store, &session.id, &heard, now)
    }

    /// A Session committed or pushed: entered on the timeline, where the build record reads its road from.
    fn road_noted(&self, session: &KeptSession, kind: TimelineKind, says: &str, branch: Option<String>, now: &str) {
        self.timeline().note(
            Fact::new(now, kind, Subject::Session(session.id.clone()), TimelineTone::Idle, says)
                .titled(session.title.clone())
                .detail(branch)
                .in_repository(session.manifest_id.clone())
                .live(),
        );
    }

    /// A Session showed a page. The road reads a walk off the timeline: the `walk` item on its list is
    /// not entered there.
    pub(crate) fn walk_noted(&self, id: &str, record: &ipc::SessionRecord, page: &str) {
        self.timeline().note(
            Fact::new(self.timeline().now(), TimelineKind::SessionWalked, Subject::Session(id.to_string()), TimelineTone::Idle, "Walk shown")
                .titled(record.title.clone())
                .detail(Some(page.to_string()))
                .in_repository(record.manifest_id.as_ref().map(|one| one.as_str().to_string()))
                .live(),
        );
    }
}
