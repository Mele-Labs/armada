//! Two projects of one name made one: this Mac's own and a copy of another Mac's. `docs/concepts/project.md`,
//! *Sharing with his other Macs*.
//!
//! **Each Mac changes only what it holds.** The Mac that holds the kept project takes the gone one's tasks, members and
//! values (`absorb_run`), and the Mac that holds the gone one moves its Jobs, sends what the other was not given and
//! deletes the project (`give_up_run`). The press does its own part here and relays the other's as a `ProjectAct`, queued
//! where that Mac sleeps, and `project_copies` shows the merge on the wire at once.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Projects, Refusal};
use core_model::Member;
use ipc::{
    EditProject, ManifestId, MergeProjects, NewProjectIssue, NewProjectTask, Project, ProjectAct, ProjectRow, ProjectTask, ProjectTaskState,
    ValuesSet,
};
use store::{KeptProjectRow, KeptRowItem, KeptTaskState};

use crate::daemon::Fleet;
use crate::project_copies::row_id;
use crate::projects::{member_on_the_wire, put_in, rows_of, NO_SUCH_PROJECT, PulledBy, SaidBy};

/// The two projects are not one of this Mac's own and one copy. A 422.
const NOT_A_PAIR: &str = "fleet.merge_not_a_pair";
/// The two projects have different names. A 422.
const NAMES_DIFFER: &str = "fleet.merge_names_differ";
/// The project to keep is not shared with his other Macs. A 422.
const KEPT_NOT_SHARED: &str = "fleet.merge_kept_not_shared";
/// The Mac told to give its project up holds no copy of the one kept. A 422.
const KEPT_NOT_HELD: &str = "fleet.merge_kept_not_held";

/// The members of `held` with `added` joined: a repository already whole adds no folder of it, and a whole repository
/// takes the place of the folders of it.
fn joined(mut held: Vec<Member>, added: Member) -> Vec<Member> {
    if held.contains(&added) {
        return held;
    }
    let repository = added.repository().clone();
    match added.folder_of() {
        None => {
            held.retain(|one| one.repository() != &repository);
            held.push(added);
        }
        Some(_) if held.iter().any(|one| one.repository() == &repository && one.folder_of().is_none()) => {}
        Some(_) => held.push(added),
    }
    held
}

/// A row of a project as the Mac that keeps the other project takes it: **a held task comes over open**, since a Job on
/// the other Mac holds it and linking a Job to a task on another Mac is not built, and an asked one has no Job.
/// The links, by reference and number, are the second part.
pub(crate) fn kept_row_of(row: &ProjectRow) -> (KeptProjectRow, Vec<String>, Vec<u64>) {
    match row {
        ProjectRow::Task { id, manifest_id, title, notes, state, parent, issues, pulls, .. } => {
            let state = match state {
                ProjectTaskState::Open | ProjectTaskState::Held => KeptTaskState::Open,
                ProjectTaskState::Done => KeptTaskState::Done,
                ProjectTaskState::Asked => KeptTaskState::Asked(None),
            };
            let item = KeptRowItem::Task {
                title: title.clone(),
                notes: notes.clone(),
                state,
                parent: parent.clone(),
                issues: Vec::new(),
                pulls: Vec::new(),
            };
            let kept = KeptProjectRow { id: id.clone(), manifest_id: manifest_id.as_str().to_string(), item };
            (kept, issues.iter().map(|one| one.issue.clone()).collect(), pulls.iter().map(|one| one.number).collect())
        }
        ProjectRow::Issue { id, manifest_id, issue, .. } => {
            let kept = KeptProjectRow { id: id.clone(), manifest_id: manifest_id.as_str().to_string(), item: KeptRowItem::Issue { reference: issue.clone() } };
            (kept, Vec::new(), Vec::new())
        }
    }
}

/// The act that gives `row`, a row of a project that goes, to the Mac that holds `into`.
fn add_of(row: &KeptProjectRow, into: &str) -> ProjectAct {
    let manifest_id = ManifestId::carried(row.manifest_id.clone());
    match &row.item {
        KeptRowItem::Task { title, notes, parent: None, .. } => ProjectAct::AddProjectTask {
            task: NewProjectTask { manifest_id, project: Some(into.to_string()), title: title.clone(), notes: notes.clone() },
            id: Some(row.id.clone()),
        },
        KeptRowItem::Task { title, notes, parent: Some(parent), .. } => {
            ProjectAct::AddProjectSubtask { row: parent.clone(), task: ProjectTask { title: title.clone(), notes: notes.clone() }, id: Some(row.id.clone()) }
        }
        KeptRowItem::Issue { reference } => {
            ProjectAct::LinkProjectIssue { link: NewProjectIssue { manifest_id, project: Some(into.to_string()), issue: reference.clone() } }
        }
    }
}

/// One of the two, as this Mac has it.
enum Side {
    /// This Mac's own, by its name.
    Own(String),
    Copy(Project),
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
    /// `merge_projects`: the press, on whichever Mac the person is at. Refused with nothing changed: 404
    /// `fleet.no_such_project`, and 422 `fleet.merge_not_a_pair`, `fleet.merge_names_differ` and
    /// `fleet.merge_kept_not_shared`. **This Mac does its own part now and sends the other's**, so the one project
    /// stands here from the press whether the other Mac answers or sleeps.
    pub(crate) async fn projects_merged(&self, merge: MergeProjects) -> Result<ipc::Projects, Refusal> {
        let MergeProjects { kept, gone, from_gone } = merge;
        let own = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let copies = self.project_copies().await?;
        let find = |id: &str| -> Result<Side, Refusal> {
            if let Some(one) = own.projects.iter().find(|one| one.id == id) {
                return Ok(Side::Own(one.name.clone()));
            }
            copies
                .iter()
                .find(|one| one.id == id)
                .map(|one| Side::Copy(one.clone()))
                .ok_or_else(|| Refusal::NotFound(self.project_refusal(NO_SUCH_PROJECT, format!("there is no project {id}"))))
        };
        let (keeping, going) = (find(&kept)?, find(&gone)?);
        let refused = |code: &'static str, said: String| Refusal::Unacceptable(self.project_refusal(code, said));
        let not_a_pair = || refused(NOT_A_PAIR, String::from("Only a project of this Mac and a copy of another Mac's project can be merged."));
        let (kept_name, gone_name, kept_shared) = match (&keeping, &going) {
            (Side::Own(name), Side::Copy(copy)) => (name.as_str(), copy.name.as_str(), self.project_shared(&kept)),
            (Side::Copy(copy), Side::Own(name)) => (copy.name.as_str(), name.as_str(), copy.shared_with_macs),
            _ => return Err(not_a_pair()),
        };
        if kept_name.trim() != gone_name.trim() {
            return Err(refused(NAMES_DIFFER, format!("{kept_name} and {gone_name} have different names.")));
        }
        if !kept_shared {
            return Err(refused(KEPT_NOT_SHARED, format!("{kept_name} is not shared with other Macs. Turn sharing on to keep it in a merge.")));
        }
        match (keeping, going) {
            (Side::Own(_), Side::Copy(copy)) => {
                let Some(mac) = copy.mac.clone() else { return Err(not_a_pair()) };
                // The give-up first, so a Mac that refuses it leaves both projects as they were.
                let absorbed = copy.rows.iter().map(|one| row_id(one).to_string()).collect();
                self.act_on_copy_told(&mac, ProjectAct::GiveUp { gone, into: kept.clone(), absorbed }).await?;
                self.absorb_run(&kept, copy, &from_gone).await?;
                self.list_projects().await
            }
            (Side::Copy(copy), Side::Own(_)) => {
                let Some(mac) = copy.mac.clone() else { return Err(not_a_pair()) };
                let snapshot = self.own_snapshot(&gone).await?;
                let act = ProjectAct::Absorb { kept, gone: snapshot, from_gone };
                let (shown, answered) = self.act_on_copy_told(&mac, act.clone()).await?;
                match answered {
                    true => {
                        self.merge_finished(&mac, &act).await;
                        self.list_projects().await
                    }
                    false => Ok(shown),
                }
            }
            _ => Err(not_a_pair()),
        }
    }

    /// This Mac's own project `id` as a snapshot to send: its name, members, values and rows, as the Mac that keeps the
    /// other project reads them. No tracker or forge is asked, so a linked issue carries only its reference.
    async fn own_snapshot(&self, id: &str) -> Result<Project, Refusal> {
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let found = self.project_in(&kept, id)?;
        let file = self.project_read(id)?;
        let (said, pulled) = (SaidBy::new(), PulledBy::new());
        let rows = found.rows.iter().cloned().map(|row| crate::projects::row_on_the_wire(row, None, &said, &pulled)).collect();
        Ok(Project {
            id: id.to_string(),
            name: file.name().as_str().to_string(),
            rows,
            members: file.members().iter().map(member_on_the_wire).collect(),
            values: crate::values::values_set(file.values()),
            shared_with_macs: file.shared_with_macs(),
            ..Project::default()
        })
    }

    /// The Mac `mac` answered the `Absorb` this Mac sent for its own project: the gone project's part is now this
    /// Mac's to do. **Never refuses**: a part that fails leaves the project here, and the next merge does it again.
    pub(crate) async fn merge_finished(&self, mac: &str, act: &ProjectAct) {
        if let ProjectAct::Absorb { kept, gone, .. } = act {
            let absorbed: Vec<String> = gone.rows.iter().map(|one| row_id(one).to_string()).collect();
            let _ = self.give_up_run(&gone.id, kept, &absorbed, Some(mac)).await;
        }
    }

    /// `Absorb`, run on the Mac that holds `kept`: it takes `gone`'s members, the values it sets and every task it did
    /// not have, under their own ids. **Running it again adds nothing**: a row is kept once, a member joined once, and a
    /// value that is the same changes nothing. This Mac's own Jobs in `gone` move into `kept`.
    pub(crate) async fn absorb_run(&self, kept: &str, gone: Project, from_gone: &[String]) -> Result<ipc::Projects, Refusal> {
        let held = self.project_read(kept)?;
        let mut members = held.members().to_vec();
        for member in self.members_read(&gone.members)? {
            members = joined(members, member);
        }
        let values_now = crate::values::values_set(held.values()).unwrap_or_default();
        let values = ValuesSet::merged(&values_now, &gone.values.clone().unwrap_or_default(), from_gone);
        let (members_changed, values_changed) = (members != held.members(), values != values_now);
        if members_changed || values_changed {
            let edit = EditProject {
                members: members_changed.then(|| members.iter().map(member_on_the_wire).collect()),
                values: values_changed.then_some(values),
                ..EditProject::default()
            };
            Projects::edit_project(self, kept.to_string(), edit).await?;
        }
        let answered = self
            .projects_written_following(
                |store, before| {
                    let have: BTreeSet<&str> = rows_of(before).map(|one| one.id.as_str()).collect();
                    let mut repositories = Vec::new();
                    for row in gone.rows.iter().filter(|one| !have.contains(row_id(one))) {
                        let (row, issues, pulls) = kept_row_of(row);
                        store.add_project_row(kept, &row).map_err(|why| self.ledger_fault(why))?;
                        for issue in &issues {
                            store.link_task_issue(&row.id, issue).map_err(|why| self.ledger_fault(why))?;
                        }
                        for number in pulls {
                            store.link_task_pull(&row.id, number).map_err(|why| self.ledger_fault(why))?;
                        }
                        repositories.push(row.manifest_id);
                    }
                    Ok(put_in(kept, repositories))
                },
                false,
            )
            .await?;
        self.jobs_repointed(&gone.id, kept).await?;
        Ok(answered)
    }

    /// `GiveUp`, run on the Mac that holds `gone`: every row of it not in `absorbed` is sent to the Mac that holds `into`,
    /// this Mac's Jobs in it move into `into`, and the project is deleted with its file wherever Fleet keeps it, and its
    /// tasks. `via` is that Mac where the caller knows it, and otherwise the Mac whose copy of `into` this Mac holds.
    /// Refused, 422 `fleet.merge_kept_not_held`, with nothing changed **while this Mac holds no copy of `into`**: its
    /// project would go and nothing would stand in its place here.
    pub(crate) async fn give_up_run(&self, gone: &str, into: &str, absorbed: &[String], via: Option<&str>) -> Result<ipc::Projects, Refusal> {
        if !self.copy_held(into, None).await {
            let said = String::from("The project to keep is not on this Mac yet, so this Mac keeps its own. Merge again once it shows here.");
            return Err(Refusal::Unacceptable(self.project_refusal(KEPT_NOT_HELD, said)));
        }
        let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
        let unsent: Vec<KeptProjectRow> = self.project_in(&kept, gone)?.rows.iter().filter(|one| !absorbed.contains(&one.id)).cloned().collect();
        if !unsent.is_empty() {
            let holder = match via {
                Some(mac) => Some(mac.to_string()),
                None => self.copy_of_project(Some(into)).await?,
            };
            let Some(mac) = holder else {
                return Err(Refusal::NotFound(self.project_refusal(NO_SUCH_PROJECT, format!("there is no project {into}"))));
            };
            for row in &unsent {
                self.act_on_copy(&mac, add_of(row, into)).await?;
            }
        }
        self.jobs_repointed(gone, into).await?;
        // This runs on the Mac that holds `gone`, often long after the press, so a file a bucket would not let go is
        // logged, and the merge goes on.
        if let Some(left) = self.project_file_removed_everywhere(gone).await? {
            eprintln!("warn: the file of project {gone} was left in the bucket, as {}: {}", left.path, left.why);
        }
        self.projects_written_reading(|store, _| store.delete_project(gone).map(drop).map_err(|why| self.ledger_fault(why)), true).await
    }
}
