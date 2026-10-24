//! Writing a Job's approved plan into the beads issue it holds. The steps are `plan_mirror`'s;
//! this runs them against the repository's backlog and keeps what each one wrote.
//! `docs/concepts/session/the-plan.md`.
//!
//! **Run by the backlog's look, under its lock**, so the writes land in the copy and the same look
//! syncs them: there is no second loop. A step that fails stops that Job's steps, is said once, and
//! is tried again [`SYNC_EVERY`](crate::backlog::SYNC_EVERY) later. **A row is kept the moment its
//! bead exists**, before anything syncs, so a retry finds the bead and does not file another.
//! Nothing here can refuse an approval, a task's progress or a merge notice: they never wait on it.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Backlog, BeadState, Delivery, NotTracked, Vcs, WorkProduct};
use store::KeptBead;

use crate::backlog_closing::beads_issues_held;
use crate::backlog_syncing::Kept;
use crate::daemon::Fleet;
use crate::plan_mirror::{steps, Step, FINISHED, REMOVED};
use crate::repositories::Served;

pub(crate) type Door = Arc<dyn Backlog + Send + Sync>;

/// One Job's plan to mirror.
struct Work {
    session: String,
    issue: String,
    held: Vec<KeptBead>,
    steps: Vec<Step>,
}

/// `call` on the backlog, off the async thread.
pub(crate) async fn writing<T: Send + 'static>(
    backlog: &Door,
    call: impl FnOnce(&dyn Backlog) -> Result<T, NotTracked> + Send + 'static,
) -> Result<T, NotTracked> {
    let backlog = Arc::clone(backlog);
    tokio::task::spawn_blocking(move || call(backlog.as_ref()))
        .await
        .unwrap_or_else(|why| Err(NotTracked::of("writing to the backlog", why.to_string())))
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
    /// Make the beads issue of each Job in `served` say what its plan says. What went wrong, one
    /// line each. **A Job with no beads issue mirrors nothing.** Only the first issue a Job holds
    /// has its plan under it.
    pub(crate) async fn plans_mirrored(
        &self,
        served: &Served,
        backlog: &Door,
        kept: &mut Kept,
        now: Duration,
    ) -> Vec<String> {
        let mut failed = Vec::new();
        let mut works = Vec::new();
        {
            let store = self.store().lock().await;
            let mut seen = std::collections::BTreeSet::new();
            for held in beads_issues_held(&store, served.manifest().id().as_str()) {
                let key = format!("mirror:{}", held.session);
                if !seen.insert(held.session.clone()) || !kept.due(&key, now) {
                    continue;
                }
                let read = store.session_plan(&held.session).and_then(|plan| {
                    store.plan_beads(&held.session, &held.issue).map(|rows| (plan, rows))
                });
                match read {
                    Ok((Some(plan), rows)) => {
                        let todo = steps(&plan, &rows);
                        if !todo.is_empty() {
                            works.push(Work { session: held.session, issue: held.issue, held: rows, steps: todo });
                        }
                    }
                    Ok((None, _)) => {}
                    Err(why) => {
                        kept.failed_at.insert(key, now);
                        failed.push(format!("reading the plan of job {} did not happen: {why:?}", short(&held.session)));
                    }
                }
            }
        }
        for work in works {
            let key = format!("mirror:{}", work.session);
            let session = work.session.clone();
            match self.mirrored(served.root(), backlog, work).await {
                Ok(()) => {
                    kept.failed_at.remove(&key);
                }
                Err(why) => {
                    kept.failed_at.insert(key, now);
                    failed.push(format!(
                        "mirroring the plan of job {} did not happen: {}: {}",
                        short(&session),
                        why.doing,
                        why.said
                    ));
                }
            }
        }
        failed
    }

    /// Run `work`'s steps in order, keeping each row as its write lands. Stops at the first failure.
    async fn mirrored(&self, root: &str, backlog: &Door, work: Work) -> Result<(), NotTracked> {
        let Work { session, issue, held, steps } = work;
        let mut rows: BTreeMap<String, KeptBead> = held.into_iter().map(|row| (row.plan_id.clone(), row)).collect();
        let kept = |why: store::WriteError| NotTracked::of("keeping the mirror of the plan", format!("{why:?}"));
        for step in steps {
            let root = root.to_string();
            match step {
                Step::Create { id, parent, title } => {
                    let under = match &parent {
                        Some(group) => rows.get(group).map(|row| row.bead.clone()),
                        None => Some(issue.clone()),
                    }
                    .ok_or_else(|| NotTracked::of("filing a task under the issue", "its group has no bead"))?;
                    let said = title.clone();
                    let bead = writing(backlog, move |door| door.create_child(&root, &under, &said)).await?;
                    let row = KeptBead {
                        session_id: session.clone(),
                        issue: issue.clone(),
                        plan_id: id.clone(),
                        bead,
                        title,
                        status: BeadState::Open.word().to_string(),
                        blocked_by: String::new(),
                    };
                    self.store().lock().await.keep_plan_bead(&row).map_err(kept)?;
                    rows.insert(id, row);
                }
                Step::Retitle { id, title } => {
                    let Some(row) = rows.get_mut(&id) else { continue };
                    let (bead, said) = (row.bead.clone(), title.clone());
                    writing(backlog, move |door| door.retitle(&root, &bead, &said)).await?;
                    row.title = title;
                    self.store().lock().await.keep_plan_bead(row).map_err(kept)?;
                }
                Step::Order { id, after } => {
                    let Some(row) = rows.get(&id).cloned() else { continue };
                    let behind = after.as_ref().and_then(|group| rows.get(group)).map(|row| row.bead.clone());
                    let was = rows.get(&row.blocked_by).map(|row| row.bead.clone());
                    if after.is_some() && behind.is_none() {
                        return Err(NotTracked::of("ordering the issues", "the group before has no bead"));
                    }
                    let bead = row.bead.clone();
                    writing(backlog, move |door| {
                        if let Some(was) = was {
                            door.unblock(&root, &bead, &was)?;
                        }
                        behind.map_or(Ok(()), |behind| door.block(&root, &bead, &behind))
                    })
                    .await?;
                    let row = rows.get_mut(&id).expect("held above");
                    row.blocked_by = after.unwrap_or_default();
                    self.store().lock().await.keep_plan_bead(row).map_err(kept)?;
                }
                Step::Set { id, state } => {
                    let Some(row) = rows.get_mut(&id) else { continue };
                    let bead = row.bead.clone();
                    writing(backlog, move |door| door.put_state(&root, &bead, state, FINISHED)).await?;
                    row.status = state.word().to_string();
                    self.store().lock().await.keep_plan_bead(row).map_err(kept)?;
                }
                Step::Retire { id } => {
                    let Some(row) = rows.get(&id).cloned() else { continue };
                    if row.status != BeadState::Closed.word() {
                        let bead = row.bead.clone();
                        writing(backlog, move |door| door.put_state(&root, &bead, BeadState::Closed, REMOVED)).await?;
                    }
                    self.store().lock().await.forget_plan_bead(&session, &issue, &id).map_err(kept)?;
                    rows.remove(&id);
                }
            }
        }
        Ok(())
    }
}

/// The start of a Job's id, as the claim's actor names it.
pub(crate) fn short(id: &str) -> String {
    id.chars().take(8).collect()
}
