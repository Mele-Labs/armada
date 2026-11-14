//! Projects held from his other Macs: a read copy kept beside this Mac's own, and the edits made on it.
//! `docs/concepts/project.md`, *Sharing with his other Macs*.
//!
//! **The Mac that holds a project stays its authority.** What arrives is only what that Mac says, kept as it
//! was said and shown with its `mac`; a task act on a copy is sent to that Mac, which runs it with its own code
//! and answers its projects, and the answer is the copy. **Nothing here ticks a task**: a copy is replaced by
//! what the holder says, and a tick travels only as the person's own `tick_project_task`.
//!
//! **An edit made while that Mac cannot be reached is queued here**, shown on the copy at once, and sent when
//! it next speaks, oldest first. A copy is never sent on to a third Mac, and reaches Bridge by relay.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Projects, Refusal};
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{
    EditedTaskFields, Event, JobAct, MacAct, MoveProjectRow, Project, ProjectAct, ProjectActFromMac, ProjectRow, ProjectTask, ProjectTaskState,
    WireError,
};
use store::{Edited, FromMac, KeptQueuedAct};

use crate::daemon::Fleet;
use crate::mac_acts::Flushed;
use crate::other_macs::MAC_UNREADABLE;
use crate::projects::{NOT_A_TASK, NO_SUCH_ROW, TOO_DEEP};

/// A copy's project is changed in a way only the Mac that holds it may. A 409.
pub(crate) const PROJECT_HELD: &str = "fleet.other_mac_project_held";
/// A row id another Mac chose is not one this Fleet keeps rows under. A 422.
const ROW_ID_UNREADABLE: &str = "fleet.project_row_id_unreadable";

/// What waits for a Mac, read back: a task edit of one of its project copies, or an act on one of its Jobs.
struct Queued {
    seq: i64,
    mac: String,
    at: i64,
    act: Waiting,
}

enum Waiting {
    Project(ProjectAct),
    Job { id: String, act: JobAct },
}

/// The answer from the other Mac to a sent act.
enum Sent {
    Answered(ipc::Projects),
    Refused(Refusal),
    /// It could not be reached, or its Fleet was not answering: the edit is queued.
    Unreachable,
}

fn row_id(row: &ProjectRow) -> &str {
    match row {
        ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id,
    }
}

fn parent_of(row: &ProjectRow) -> Option<&str> {
    match row {
        ProjectRow::Task { parent, .. } => parent.as_deref(),
        ProjectRow::Issue { .. } => None,
    }
}

/// `act` shown on `project`, a copy of one that Mac holds: **a title, notes, a tick, a removal and a task added
/// show at once**; a link and a move show when the Mac has them. An add shows under the id it will be kept under.
fn overlaid(project: &mut Project, act: &ProjectAct) {
    match act {
        ProjectAct::AddProjectTask { task, id: Some(id) } if task.project.as_deref() == Some(project.id.as_str()) => {
            if !project.rows.iter().any(|one| row_id(one) == id) {
                project.rows.push(ProjectRow::Task {
                    id: id.clone(),
                    manifest_id: task.manifest_id.clone(),
                    title: task.title.trim().to_string(),
                    notes: task.notes.trim().to_string(),
                    state: ProjectTaskState::Open,
                    held_by: None,
                    parent: None,
                    issues: Vec::new(),
                    pulls: Vec::new(),
                });
            }
        }
        ProjectAct::AddProjectSubtask { row, task, id: Some(id) } => {
            let Some(parent) = project.rows.iter().position(|one| row_id(one) == row && parent_of(one).is_none()) else {
                return;
            };
            if project.rows.iter().any(|one| row_id(one) == id) {
                return;
            }
            let ProjectRow::Task { manifest_id, .. } = &project.rows[parent] else {
                return;
            };
            let subtask = ProjectRow::Task {
                id: id.clone(),
                manifest_id: manifest_id.clone(),
                title: task.title.trim().to_string(),
                notes: task.notes.trim().to_string(),
                state: ProjectTaskState::Open,
                held_by: None,
                parent: Some(row.clone()),
                issues: Vec::new(),
                pulls: Vec::new(),
            };
            let mut after = parent + 1;
            while project.rows.get(after).is_some_and(|one| parent_of(one) == Some(row.as_str())) {
                after += 1;
            }
            project.rows.insert(after, subtask);
        }
        ProjectAct::EditProjectTask { row, task, .. } => {
            for one in &mut project.rows {
                if let ProjectRow::Task { id, title, notes, .. } = one {
                    if id == row {
                        *title = task.title.trim().to_string();
                        *notes = task.notes.trim().to_string();
                    }
                }
            }
        }
        ProjectAct::TickProjectTask { row, tick } => {
            for one in &mut project.rows {
                if let ProjectRow::Task { id, state, held_by, .. } = one {
                    if id == row {
                        *state = if tick.done { ProjectTaskState::Done } else { ProjectTaskState::Open };
                        *held_by = None;
                    }
                }
            }
        }
        ProjectAct::RemoveProjectRow { row } => {
            project.rows.retain(|one| row_id(one) != row && parent_of(one) != Some(row.as_str()));
        }
        _ => {}
    }
}

/// A queued row read back as what it holds. `None` for one a later build wrote that this one cannot read.
fn waiting_of(kept: KeptQueuedAct) -> Option<Queued> {
    let act = match kept.kind.as_str() {
        "project" => Waiting::Project(ipc::decode("an edit queued for another Mac", kept.act.as_bytes()).ok()?),
        "job" => Waiting::Job { id: kept.id, act: ipc::decode("an act queued for another Mac", kept.act.as_bytes()).ok()? },
        _ => return None,
    };
    Some(Queued { seq: kept.seq, mac: kept.mac, at: kept.at, act })
}

/// Unlocks a Mac's flush when it ends, however it ends.
pub(crate) struct Flushing<'a> {
    pub(crate) held: &'a std::sync::Mutex<std::collections::BTreeSet<String>>,
    pub(crate) mac: String,
}

impl Drop for Flushing<'_> {
    fn drop(&mut self) {
        self.held.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&self.mac);
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
    pub(crate) fn now_ms(&self) -> i64 {
        self.now().epoch_millis().unwrap_or(0)
    }

    /// The person's own edit, here, now.
    pub(crate) fn edited_now(&self) -> Edited {
        Edited::Now(self.now_ms())
    }

    /// Every project held from his other Macs, as kept, each with its Mac, in the order each Mac said. An
    /// unreadable one is left out; the next replace from its Mac writes it again.
    pub(crate) async fn copies_held(&self) -> Result<Vec<Project>, Refusal> {
        let held = self.from_macs::<Project>(FromMac::Projects, "a project from another Mac").await?;
        Ok(held
            .into_iter()
            .map(|(kept, mut project)| {
                project.mac = Some(kept.mac);
                project
            })
            .collect())
    }

    /// Every edit and act still to be sent, for Mac `mac` or for all, in the one order, oldest first. One a later
    /// build wrote that this one cannot read is left out, and so is an act the Mac refused.
    async fn queued_acts(&self, mac: Option<&str>) -> Result<Vec<Queued>, Refusal> {
        let kept = self.store().lock().await.queued_acts(mac, true).map_err(|why| self.ledger_fault(why))?;
        Ok(kept.into_iter().filter_map(waiting_of).collect())
    }

    /// The copies, with what he edited on each while its Mac could not be reached shown on it.
    async fn project_copies(&self) -> Result<Vec<Project>, Refusal> {
        let mut copies = self.copies_held().await?;
        if copies.is_empty() {
            return Ok(copies);
        }
        for queued in self.queued_acts(None).await? {
            let Waiting::Project(act) = &queued.act else {
                continue;
            };
            for project in copies.iter_mut().filter(|one| one.mac.as_deref() == Some(queued.mac.as_str())) {
                overlaid(project, act);
            }
        }
        Ok(copies)
    }

    /// His projects on the wire: this Mac's own, then the copies his other Macs hold. **Every
    /// `projects.changed` and every answer carries both**, or Bridge would lose the copies when a project here
    /// changed.
    pub(crate) async fn projects_on_the_wire(&self, kept: store::KeptProjects) -> ipc::Projects {
        let mut projects = self.own_projects_on_the_wire(kept).await;
        if let Ok(copies) = self.project_copies().await {
            projects.projects.extend(copies);
        }
        projects
    }

    /// His projects as they stand, copies included, sent to Bridge alone and answered. **A relay, never a
    /// publish**: a copy is not this Mac's to tell its own listeners of.
    pub(crate) async fn projects_relayed(&self) -> Result<ipc::Projects, Refusal> {
        let merged = Projects::list_projects(self).await?;
        self.events().relay(Event::ProjectsChanged(merged.clone()));
        Ok(merged)
    }

    /// The Mac that holds the project `project` as a copy here, where it is one. **None where this Mac keeps a
    /// project by that id itself**, so another Mac can never stand in for one of this Mac's own.
    pub(crate) async fn copy_of_project(&self, project: Option<&str>) -> Result<Option<String>, Refusal> {
        let Some(project) = project.filter(|one| !one.is_empty()) else {
            return Ok(None);
        };
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        if kept.projects.iter().any(|one| one.id == project) {
            return Ok(None);
        }
        Ok(self.project_copies().await?.into_iter().find(|one| one.id == project).and_then(|one| one.mac))
    }

    /// The Mac that holds a copy with the row `row` in it, a row queued while it slept included. None where this
    /// Mac keeps a row by that id itself.
    pub(crate) async fn copy_of_row(&self, row: &str) -> Result<Option<String>, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        if kept.projects.iter().flat_map(|one| &one.rows).chain(&kept.loose).any(|one| one.id == row) {
            return Ok(None);
        }
        let copies = self.project_copies().await?;
        Ok(copies.into_iter().find(|one| one.rows.iter().any(|found| row_id(found) == row)).and_then(|one| one.mac))
    }

    /// What a copy refuses: `Only Mac <id> can change this project.`
    pub(crate) fn copy_refused(&self, mac: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(PROJECT_HELD, format!("Only Mac {mac} can change this project."), self.run_id()))
    }

    /// `row` is not on a copy: a Job link and a repository move are only the holder's.
    pub(crate) async fn row_not_held(&self, row: &str) -> Result<(), Refusal> {
        match self.copy_of_row(row).await? {
            Some(mac) => Err(self.copy_refused(&mac)),
            None => Ok(()),
        }
    }

    /// `project` is not a copy: its name, members, values, place, share switch and deletion are only the holder's.
    pub(crate) async fn project_not_held(&self, project: &str) -> Result<(), Refusal> {
        match self.copy_of_project(Some(project)).await? {
            Some(mac) => Err(self.copy_refused(&mac)),
            None => Ok(()),
        }
    }

    /// Where a move of `row` goes: the Mac that holds it where `row` is on a copy, and none where it is this
    /// Mac's own. **A row never moves between Macs**, so a move of a copy's row to a project that is not that
    /// Mac's, to none, or of this Mac's own row into a copy, is refused.
    pub(crate) async fn move_on_copy(&self, row: &str, to: &MoveProjectRow) -> Result<Option<String>, Refusal> {
        let holder = self.copy_of_row(row).await?;
        let into = self.copy_of_project(to.project.as_deref()).await?;
        match (holder, into) {
            (None, None) => Ok(None),
            (Some(mac), None) if to.project.is_none() => Ok(Some(mac)),
            (Some(mac), Some(into)) if mac == into => Ok(Some(mac)),
            (Some(mac), _) | (None, Some(mac)) => Err(self.copy_refused(&mac)),
        }
    }

    /// The id a row is kept under: the one given, where another Mac made it, or a new one. Whether this Fleet
    /// already keeps a row under it, which is an act already applied.
    pub(crate) async fn row_minted(&self, given: Option<String>) -> Result<(String, bool), Refusal> {
        let Some(id) = given else {
            return Ok((self.minted(), false));
        };
        if id.is_empty() || id.len() > 64 || !id.chars().all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_') {
            return Err(Refusal::Unacceptable(self.project_refusal(ROW_ID_UNREADABLE, "a row id is letters, digits, - and _")));
        }
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let held = kept.projects.iter().flat_map(|one| &one.rows).chain(&kept.loose).any(|one| one.id == id);
        Ok((id, held))
    }

    /// What `act` needs of the copy of Mac `mac`'s projects, so a refusal does not wait on a Mac that sleeps: the
    /// row is there, a task where it must be, and a title is not blank. The Mac answers the rest.
    async fn copy_act_checked(&self, mac: &str, act: &ProjectAct) -> Result<(), Refusal> {
        let copies = self.project_copies().await?;
        let rows: Vec<&ProjectRow> =
            copies.iter().filter(|one| one.mac.as_deref() == Some(mac)).flat_map(|one| one.rows.iter()).collect();
        let find = |row: &str| rows.iter().find(|one| row_id(one) == row).copied();
        let missing = |row: &str| Refusal::NotFound(self.project_refusal(NO_SUCH_ROW, format!("there is no row {row}")));
        let task_row = |row: &str| match find(row) {
            None => Err(missing(row)),
            Some(ProjectRow::Issue { .. }) => Err(Refusal::Unacceptable(
                self.project_refusal(NOT_A_TASK, format!("row {row} is a linked issue, whose state and title are its tracker's")),
            )),
            Some(one) => Ok(one),
        };
        let titled = |title: &str| match title.trim().is_empty() {
            true => Err(self.blank("a task needs a title")),
            false => Ok(()),
        };
        match act {
            ProjectAct::AddProjectTask { task, .. } => titled(&task.title),
            ProjectAct::AddProjectSubtask { row, task, .. } => {
                if parent_of(task_row(row)?).is_some() {
                    return Err(Refusal::Unacceptable(
                        self.project_refusal(TOO_DEEP, format!("row {row} is a subtask, and a subtask has none")),
                    ));
                }
                titled(&task.title)
            }
            ProjectAct::EditProjectTask { row, task, .. } => task_row(row).and_then(|_| titled(&task.title)),
            ProjectAct::TickProjectTask { row, .. }
            | ProjectAct::LinkTaskIssue { row, .. }
            | ProjectAct::UnlinkTaskIssue { row, .. }
            | ProjectAct::LinkTaskPull { row, .. }
            | ProjectAct::UnlinkTaskPull { row, .. } => task_row(row).map(drop),
            ProjectAct::RemoveProjectRow { row } | ProjectAct::MoveProjectRow { row, .. } => find(row).map(drop).ok_or_else(|| missing(row)),
            ProjectAct::LinkProjectIssue { .. } => Ok(()),
        }
    }

    /// Which of `task`'s fields differ from row `row` of Mac `mac`'s copy as it shows now, with what is queued on
    /// it: **only these are sent**, so an edit of the notes never sends the title it left as it was.
    async fn edited_fields(&self, mac: &str, row: &str, task: &ProjectTask) -> Result<EditedTaskFields, Refusal> {
        let copies = self.project_copies().await?;
        let shown = copies.iter().filter(|one| one.mac.as_deref() == Some(mac)).flat_map(|one| one.rows.iter()).find(|one| row_id(one) == row);
        Ok(match shown {
            Some(ProjectRow::Task { title, notes, .. }) => {
                EditedTaskFields { title: title != task.title.trim(), notes: notes != task.notes.trim() }
            }
            _ => EditedTaskFields { title: true, notes: true },
        })
    }

    /// `sent` handed to Mac `mac` through the link. **A Mac that cannot be reached, or whose Fleet is not
    /// answering, is told apart from one that refused**: the first is queued, the second is the answer.
    async fn send_project_act(&self, mac: &str, sent: &ProjectActFromMac) -> Result<Sent, Refusal> {
        let body = self.encoded(sent)?.into_bytes();
        let Ok(switch) = self.phone_switch() else {
            return Ok(Sent::Unreachable);
        };
        let Ok((status, bytes)) = switch.send_other_mac(mac, "/other_mac/project_act", body).await else {
            return Ok(Sent::Unreachable);
        };
        if (200..300).contains(&status) {
            return match ipc::decode("projects from another Mac", &bytes) {
                Ok(projects) => Ok(Sent::Answered(projects)),
                Err(why) => Err(Refusal::Fault(WireError::raised(
                    MAC_UNREADABLE,
                    format!("Mac {mac} sent projects this build cannot read: {why}"),
                    self.run_id(),
                ))),
            };
        }
        if status == 401 || status >= 500 {
            return Ok(Sent::Unreachable);
        }
        Ok(Sent::Refused(self.refusal_passed_on(mac, status, &bytes)))
    }

    /// Mac `mac`'s projects kept as `projects`, replacing all held from it. Whether the copy changed.
    async fn copies_kept(&self, mac: &str, projects: Vec<Project>) -> Result<bool, Refusal> {
        let before: Vec<Project> = self.copies_held().await?.into_iter().filter(|one| one.mac.as_deref() == Some(mac)).collect();
        let after: Vec<Project> = projects
            .into_iter()
            .map(|mut one| {
                one.mac = Some(mac.to_string());
                one
            })
            .collect();
        // Keyed by place first, so the copy lists in the order its Mac keeps them.
        let rows = after
            .iter()
            .enumerate()
            .map(|(at, one)| Ok((format!("{at:06}:{}", one.id), self.encoded(one)?)))
            .collect::<Result<Vec<_>, Refusal>>()?;
        self.store().lock().await.replace_from_mac(FromMac::Projects, mac, &rows).map_err(|why| self.ledger_fault(why))?;
        Ok(before != after)
    }

    /// Mac `mac`'s projects as it says them now, kept, and sent to Bridge where the copy changed.
    pub(crate) async fn copies_replaced(&self, mac: &str, projects: Vec<Project>) -> Result<(), Refusal> {
        if self.copies_kept(mac, projects).await? {
            self.projects_relayed().await?;
        }
        Ok(())
    }

    /// An act on a project Mac `mac` holds, which is the Mac's to run. Sent to it where it answers, and
    /// otherwise **queued and shown on the copy at once**, to be sent when it next speaks. Answers his projects.
    pub(crate) async fn act_on_copy(&self, mac: &str, mut act: ProjectAct) -> Result<ipc::Projects, Refusal> {
        self.copy_act_checked(mac, &act).await?;
        if let ProjectAct::EditProjectTask { row, task, fields } = &mut act {
            *fields = Some(self.edited_fields(mac, row, task).await?);
        }
        let at = self.now_ms();
        // Behind what is queued already, so the Mac hears them in the order he made them.
        let waiting = !self.queued_acts(Some(mac)).await?.is_empty();
        if !waiting {
            match self.send_project_act(mac, &ProjectActFromMac { act: act.clone(), at: Some(at) }).await? {
                Sent::Answered(answer) => {
                    self.copies_kept(mac, answer.projects).await?;
                    return self.projects_relayed().await;
                }
                Sent::Refused(refusal) => return Err(refusal),
                Sent::Unreachable => {}
            }
        }
        if let ProjectAct::AddProjectTask { id, .. } | ProjectAct::AddProjectSubtask { id, .. } = &mut act {
            *id = Some(self.minted());
        }
        let body = self.encoded(&act)?;
        self.store().lock().await.queue_project_act(mac, at, &body).map_err(|why| self.ledger_fault(why))?;
        self.projects_relayed().await
    }

    /// Whether edits wait for Mac `mac`.
    pub(crate) async fn project_acts_waiting(&self, mac: &str) -> bool {
        self.queued_acts(Some(mac)).await.is_ok_and(|queued| !queued.is_empty())
    }

    /// Send Mac `mac` what is queued for it, oldest first: each edit with when he made it, each act on a Job with
    /// the id it was made under. **An answer drops an edit, and so does a refusal**, said once; a Job act a Mac
    /// refuses stays with its why. A Mac that cannot be reached stops it, and the rest wait. The last answer to
    /// an edit is kept as the copy. One task for a Mac at a time.
    pub(crate) async fn project_acts_flushed(self: Arc<Self>, mac: String) {
        if !self.phone_held().flushing.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(mac.clone()) {
            return;
        }
        let _unlock = Flushing { held: &self.phone_held().flushing, mac: mac.clone() };
        let mut last = None;
        let mut progressed = false;
        let mut jobs_moved = false;
        loop {
            let Some(next) = self.queued_acts(Some(&mac)).await.ok().and_then(|queued| queued.into_iter().next()) else {
                break;
            };
            match next.act {
                Waiting::Project(act) => match self.send_project_act(&mac, &ProjectActFromMac { act, at: Some(next.at) }).await {
                    Ok(Sent::Answered(answer)) => last = Some(answer.projects),
                    Ok(Sent::Refused(refusal)) => {
                        let said = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a queued edit was refused by the Mac that holds its project, and dropped")
                            .with_field("mac", FieldValue::Str(mac.clone()))
                            .with_field("why", FieldValue::Str(refusal.error().message.clone()));
                        crate::transcript::note_aloud(&said);
                    }
                    Ok(Sent::Unreachable) | Err(_) => break,
                },
                Waiting::Job { id, act } => {
                    jobs_moved = true;
                    match self.job_act_flushed(&mac, next.seq, MacAct { id, act, at: next.at }).await {
                        Flushed::Sent => {}
                        Flushed::Kept => continue,
                        Flushed::Stopped => break,
                    }
                }
            }
            if self.store().lock().await.drop_project_act(next.seq).is_err() {
                break;
            }
            progressed = true;
        }
        if progressed {
            if let Some(projects) = last {
                let _ = self.copies_kept(&mac, projects).await;
            }
            let _ = self.projects_relayed().await;
        }
        if jobs_moved {
            self.queued_relayed().await;
        }
    }

    /// Start [`Fleet::project_acts_flushed`] on a task of its own where edits wait for Mac `mac`: the Mac that
    /// has just spoken is awake, and the call that heard it must not wait on the sending.
    pub(crate) async fn project_acts_flushing(self: &Arc<Self>, mac: &str) {
        if self.project_acts_waiting(mac).await {
            let (fleet, mac) = (Arc::clone(self), mac.to_string());
            tokio::spawn(async move { fleet.project_acts_flushed(mac).await });
        }
    }
}
