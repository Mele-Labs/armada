//! What a live Job holds, read off its ledger, its plan, the files Fleet saw it write and its beads
//! items' assignees. **Read, never reported**: no call of the agent's adds to it.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{IssueReference, ManifestId};
use store::{AttachmentState, Holder};

use super::{person_holding, Wanted};
use crate::clashing::Who;
use crate::daemon::Fleet;
use crate::pull_requesting::PR;
use crate::session_issues::{CLAIMED_AS, ISSUE};

/// One live Job as a pass reads it.
#[derive(Clone, Debug, Default)]
pub(crate) struct JobRead {
    pub id: String,
    pub manifest_id: String,
    pub root: String,
    pub title: String,
    /// Its open items, each with the name it claimed it as in beads, where it did.
    pub items: BTreeMap<String, Option<String>>,
    /// Its plan's paths, whole-repository groups left out.
    pub planned: BTreeSet<String>,
    pub written: BTreeSet<String>,
    pub pulls_open: BTreeSet<u64>,
    pub pulls_ended: BTreeSet<u64>,
    /// Whether its tracker is beads, the one whose assignee counts as a person's held work.
    pub beads: bool,
}

impl JobRead {
    /// The items a person holds as their beads assignee, with that person, from `assignees` read by
    /// repository and reference.
    pub(crate) fn person_held(&self, assignees: &BTreeMap<(String, String), Vec<String>>) -> BTreeMap<String, String> {
        if !self.beads {
            return BTreeMap::new();
        }
        self.items
            .iter()
            .filter_map(|(item, own)| {
                let read = assignees.get(&(self.manifest_id.clone(), item.clone()))?;
                person_holding(read, own.as_deref()).map(|person| (item.clone(), person.clone()))
            })
            .collect()
    }

    /// What it holds for the store: its items no person holds, its files less those a merged or
    /// closed pull request ended, and its open pull requests.
    pub(crate) fn wanted(&self, person_held: &BTreeMap<String, String>, files_ended: &BTreeSet<String>) -> BTreeSet<Wanted> {
        let items = self.items.keys().filter(|item| !person_held.contains_key(*item)).map(|item| Wanted::item(item));
        let files = self.planned.union(&self.written).filter(|path| !files_ended.contains(*path)).map(|path| Wanted::file(path));
        let pulls = self.pulls_open.iter().map(|number| Wanted::pull_request(*number));
        items.chain(files).chain(pulls).collect()
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
    /// Every live Job in a repository Fleet serves, read for a pass.
    pub(crate) async fn jobs_read(&self) -> Vec<JobRead> {
        let mut read = Vec::new();
        {
            let store = self.store().lock().await;
            let Ok(ids) = store.live_session_ids() else { return Vec::new() };
            for id in ids {
                let Some(session) = store.session(&id).ok().flatten() else { continue };
                let Some(manifest_id) = session.manifest_id.clone() else { continue };
                let mut job = JobRead { id: id.clone(), manifest_id, title: session.title.unwrap_or_default(), ..JobRead::default() };
                for row in store.attachments_of(&Holder::session(&id)).unwrap_or_default() {
                    if row.manifest_id != job.manifest_id {
                        continue;
                    }
                    match (row.kind.as_str(), row.state) {
                        (ISSUE, AttachmentState::Standing) if row.detail.get("state").map(String::as_str) != Some("closed") => {
                            if let Some(issue) = IssueReference::read(&row.target) {
                                job.items.insert(issue.as_str().to_string(), row.detail.get(CLAIMED_AS).cloned());
                            }
                        }
                        (PR, AttachmentState::Standing) => job.pulls_open.extend(row.target.parse::<u64>().ok()),
                        (PR, _) => job.pulls_ended.extend(row.target.parse::<u64>().ok()),
                        _ => {}
                    }
                }
                if let Some(plan) = store.session_plan(&id).ok().flatten() {
                    job.planned = plan.groups.iter().flat_map(|group| group.paths.iter()).map(|path| path.trim().trim_end_matches('/').to_string()).filter(|path| !path.is_empty()).collect();
                }
                read.push(job);
            }
        }
        let mut served_read = Vec::with_capacity(read.len());
        for mut job in read {
            let Ok(served) = self.served_named(Some(&ManifestId::carried(&job.manifest_id))) else { continue };
            job.root = served.root().to_string();
            job.written = self.writes().written_by(&Who::Session(job.id.clone()));
            let project = self.project_of_job(&job.id).await;
            job.beads = self.backlog_for_job(&served, project.as_deref()).is_some();
            served_read.push(job);
        }
        served_read
    }

    /// The assignees of a beads item as the issue reading found them, kept for the next pass.
    pub(crate) fn assignees_seen(&self, manifest_id: &str, issue: &str, assignees: Vec<String>) {
        self.job_claims().with(|kept| {
            kept.assignees.insert((manifest_id.to_string(), issue.to_string()), assignees);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_person_held_item_is_waited_on_not_claimed_and_ended_files_stay_ended() {
        let job = JobRead {
            manifest_id: "armada".into(),
            items: BTreeMap::from([("armada-1".to_string(), None), ("armada-2".to_string(), Some("home/abc".to_string()))]),
            planned: BTreeSet::from(["crates/fleet".to_string()]),
            written: BTreeSet::from(["src/a.rs".to_string(), "src/b.rs".to_string()]),
            pulls_open: BTreeSet::from([7]),
            beads: true,
            ..JobRead::default()
        };
        let assignees = BTreeMap::from([(("armada".to_string(), "armada-1".to_string()), vec!["nick".to_string()])]);
        let held = job.person_held(&assignees);
        assert_eq!(held, BTreeMap::from([("armada-1".to_string(), "nick".to_string())]));
        let wanted = job.wanted(&held, &BTreeSet::from(["src/b.rs".to_string()]));
        assert_eq!(
            wanted,
            BTreeSet::from([Wanted::item("armada-2"), Wanted::file("crates/fleet"), Wanted::file("src/a.rs"), Wanted::pull_request(7)])
        );
        assert!(JobRead { beads: false, ..job }.person_held(&assignees).is_empty(), "only a beads assignee is a person's held work");
    }
}
