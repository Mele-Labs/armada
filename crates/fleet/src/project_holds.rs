//! A Job holding the written tasks of a project it was started with. `docs/practices/protocol/projects.md`.
//!
//! **A task is held from the Job's start**, and only a written task: a linked issue row keeps no
//! state and is only named, so the Job's brief can carry it. Every move here publishes the
//! projects whole, as a person's own write does.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    Instant, TaskPullState, ProjectRowRef, SessionRecord, WaitingAct, WaitingActKind, WaitingItem, WaitingOption,
    WaitingSource,
};
use store::{KeptAsked, KeptProjects, KeptRowItem, SessionState, Store, LOOSE_PROJECT};

use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// A start names a row of another repository. A 422.
const OTHER_REPOSITORY: &str = "fleet.project_row_other_repository";

/// A Job to hold a task is another repository's. A 422.
const JOB_OTHER_REPOSITORY: &str = "fleet.project_job_other_repository";
/// A Job to hold a task has ended. A 422.
const JOB_ENDED: &str = "fleet.project_job_ended";

/// The ids of the subtasks of task `row`.
pub(crate) fn subtasks_of<'k>(kept: &'k KeptProjects, row: &'k str) -> impl Iterator<Item = String> + 'k {
    kept.projects
        .iter()
        .flat_map(|project| &project.rows)
        .chain(&kept.loose)
        .filter(move |sub| matches!(&sub.item, KeptRowItem::Task { parent: Some(of), .. } if of == row))
        .map(|sub| sub.id.clone())
}

/// The id of the ask about one project's tasks: `task_done:<project>`, and `task_done:loose` for the tasks in none.
const TASK_DONE: &str = "task_done:";
const LOOSE_ASK: &str = "loose";

/// What the person is asked about the tasks a Job finished in one project. Choice 0 and 1 are the
/// options in this order: `answer_waiting` takes the index.
pub(crate) fn task_done_item(asked: &KeptAsked) -> WaitingItem {
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None };
    let which = if asked.project_id == LOOSE_PROJECT { LOOSE_ASK } else { asked.project_id.as_str() };
    let id = format!("{TASK_DONE}{which}");
    let text = match asked.tasks.as_slice() {
        [(_, title)] => format!("Mark {title} done?"),
        _ if asked.project_id == LOOSE_PROJECT => "Mark the tasks done?".to_string(),
        _ => format!("Mark the {} tasks done?", asked.project_name),
    };
    WaitingItem {
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone() }),
        id,
        text,
        since: Instant::carried(&asked.since),
        source: WaitingSource::TaskDone,
        options: vec![option("Mark done"), option("Keep open")],
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
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
    /// The written tasks among `named`, by row id, once every row is known to be one of
    /// `manifest`'s. **All or nothing**: one row that is missing, or another repository's, refuses
    /// the start. An issue row passes and is not returned.
    pub(crate) async fn project_rows_checked(&self, manifest: &str, named: &[ProjectRowRef]) -> Result<Vec<String>, Refusal> {
        let store = self.store().lock().await;
        let kept = store.projects().map_err(|why| self.ledger_fault(why))?;
        let mut tasks = Vec::new();
        for one in named {
            let (_, row) = self.row_in(&kept, &one.row)?;
            if row.manifest_id != manifest {
                return Err(Refusal::Unacceptable(
                    self.project_refusal(OTHER_REPOSITORY, format!("row {} is another repository's", one.row)),
                ));
            }
            if matches!(row.item, store::KeptRowItem::Task { .. }) {
                tasks.push(one.row.clone());
            }
            // A task held with its subtasks: naming a parent names them all.
            tasks.extend(subtasks_of(&kept, &one.row));
        }
        let mut seen = std::collections::HashSet::new();
        tasks.retain(|one| seen.insert(one.clone()));
        Ok(tasks)
    }

    /// `session` may hold tasks of `manifest`: a Job Fleet has a record of, still running, in that repository.
    pub(crate) fn job_may_hold(&self, store: &Store, manifest: &str, session: &str) -> Result<(), Refusal> {
        let Some(job) = store.session(session).map_err(|why| self.ledger_fault(why))? else {
            return Err(Refusal::NotFound(self.project_refusal(NO_SUCH_SESSION, format!("no job is named {session}"))));
        };
        if job.manifest_id.as_deref() != Some(manifest) {
            return Err(Refusal::Unacceptable(
                self.project_refusal(JOB_OTHER_REPOSITORY, format!("job {session} is another repository's")),
            ));
        }
        if job.state == SessionState::Ended {
            return Err(Refusal::Unacceptable(self.project_refusal(JOB_ENDED, format!("job {session} has ended"))));
        }
        Ok(())
    }

    /// `session` holds `tasks`, by row id, from now, and his projects are published.
    pub(crate) async fn project_tasks_held(&self, session: &str, tasks: &[String]) {
        let held = self.store().lock().await.hold_project_tasks(session, tasks);
        if let Err(why) = held {
            self.session_warned(session, "a job's project tasks were not held", format!("{why:?}"));
            return;
        }
        self.projects_published().await;
    }

    /// His projects as they stand, published as `projects.changed`.
    pub(crate) async fn projects_published(&self) {
        let kept = self.store().lock().await.projects();
        let Ok(kept) = kept else {
            return;
        };
        let projects = self.projects_on_the_wire(kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects));
    }

    /// `answer_waiting` on a `task_done` item: choice 0 marks what the Job asked about in that
    /// project done, 1 opens it again. The agent is told nothing: it finished, and the Job may be gone.
    pub(crate) async fn task_done_chosen(
        &self,
        session: &str,
        item: &str,
        choice: Option<u32>,
    ) -> Result<SessionRecord, Refusal> {
        let done = match choice {
            Some(0) => true,
            Some(1) => false,
            _ => {
                return Err(self.hosted_refusal(
                    "fleet.session_waiting_empty",
                    "whether the tasks are done is answered by choosing Mark done or Keep open",
                ))
            }
        };
        let project = item.strip_prefix(TASK_DONE).unwrap_or(item);
        let project = if project == LOOSE_ASK { LOOSE_PROJECT } else { project };
        let record = {
            let mut store = self.store().lock().await;
            let kept = store.session(session).map_err(|why| self.ledger_fault(why))?;
            let kept = kept.ok_or_else(|| self.settled_already())?;
            let changed = store.settle_asked_project_tasks(session, project, done).map_err(|why| self.ledger_fault(why))?;
            if changed == 0 {
                return Err(self.settled_already());
            }
            self.ledger_row(&store, &kept)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        self.projects_published().await;
        Ok(record)
    }

    /// `session` said it is done, or its pull request merged: what it holds is asked about, and
    /// the Job and the projects are published. Nothing happens for a Job holding no task.
    pub(crate) async fn project_tasks_asked(&self, session: &str) {
        let at = self.now().as_str().to_string();
        match self.store().lock().await.ask_held_project_tasks(session, &at) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let _ = self.published_hosted(session).await;
        self.projects_published().await;
    }

    /// Pull request `number` merged: every Job that held it is asked about its held tasks. **Held, not
    /// standing**, as `issues_merged` reads it, and idempotent: a task already asked is not asked again.
    pub(crate) async fn tasks_asked_by_merge(&self, served: &crate::repositories::Served, number: u64) {
        let manifest = served.manifest().id().as_str().to_string();
        let holders: Vec<String> = self
            .store()
            .lock()
            .await
            .attachments_at("pr", &number.to_string(), Some(&manifest))
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.holder.kind == store::HolderKind::Session)
            .map(|row| row.holder.id)
            .collect();
        for session in holders {
            self.project_tasks_asked(&session).await;
        }
    }

    /// Pull request `number` merged: each task linked to it that is open, or held, is asked about, once.
    /// **Whoever opened it**: a person linked it, and nobody's Job need have. A Job holding a task keeps its
    /// ask on its list; a task with none is asked on the rows alone. The forge's answer for it is kept as
    /// merged. Whether any task was asked, in which case the projects are published.
    pub(crate) async fn tasks_asked_by_linked_pull(&self, manifest: &str, number: u64, title: &str) -> bool {
        self.tracking().pulls.keep(manifest, number, self.now(), (TaskPullState::Merged, title.to_string()));
        let at = self.now().as_str().to_string();
        let asked = self.store().lock().await.ask_pulled_project_tasks(manifest, number, &at);
        let Ok(asked) = asked else {
            return false;
        };
        if asked.is_empty() {
            return false;
        }
        let mut jobs: Vec<&String> = asked.iter().filter(|one| !one.is_empty()).collect();
        jobs.sort_unstable();
        jobs.dedup();
        for session in jobs {
            let _ = self.published_hosted(session).await;
        }
        self.projects_published().await;
        true
    }

    /// `session` is over: what it held and did not finish is open again. What it asked about stays asked.
    pub(crate) async fn project_tasks_freed(&self, session: &str) {
        match self.store().lock().await.free_held_project_tasks(session) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        self.projects_published().await;
    }
}
