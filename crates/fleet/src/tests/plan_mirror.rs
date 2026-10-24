//! The claim, written before the code: **a Job's approved plan is mirrored into the beads issue it
//! holds, the beads follow what Fleet saw, the issue closes when the Job's pull request merges, and a
//! Job whose claim lost is told.** A fake backlog stands in for `bd`; the plan in Fleet stays the
//! source. `docs/concepts/tracker.md`, *Beads*, and `docs/concepts/session/the-plan.md`.

use std::sync::Arc;

use adapter_traits::{Backlog, FromOutside, RecentlyMerged};
use api::Sessions;
use ipc::{ApprovePlan, IssueReference, ReportedTaskStatus, SessionId};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{FakeBeads, FakeBeadsRemote};

use super::backlog::{actor, claims, machine, machine_reading, manifest};
use super::session_plan::{first, group, plan, recorded, task, tasks};
use super::session_host::Rig;
use crate::backlog::SYNC_EVERY;

const ON_MAIN: &str = "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\ntracker:\n  provider: beads\n";
const PULL: &str = "https://forge.invalid/armada/pull/1";

struct Boat {
    rig: Rig,
    copy: Arc<FakeBeads>,
    remote: FakeBeadsRemote,
    job: SessionId,
    issue: String,
}

/// A Fleet on a beads backlog, and a running Job that has not claimed the issue in it.
async fn unclaimed_reading(manifest: &'static str) -> Boat {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let rig = machine_reading("mac-a", &copy, manifest).await;
    let issue = copy.create("Mirror the plan");
    copy.sync("/").expect("pushed");
    let job = rig.start().await;
    rig.send(&job, "go").await;
    rig.stand_in.init(0);
    Boat { rig, copy, remote, job, issue }
}

/// [`unclaimed_reading`], and the Job claims the issue.
async fn boat_reading(manifest: &'static str) -> Boat {
    let boat = unclaimed_reading(manifest).await;
    claims(&boat.rig, &boat.job, &boat.issue).await.expect("claimed");
    boat
}

async fn boat() -> Boat {
    boat_reading(super::backlog::BEADS).await
}

impl Boat {
    async fn approves(&self, version: u32) {
        Arc::clone(&self.rig.fleet)
            .approve_plan(ApprovePlan { session_id: self.job.clone(), version })
            .await
            .expect("approved");
    }

    async fn looks(&self) -> Vec<String> {
        Arc::clone(&self.rig.fleet).backlogs_looked().await
    }

    /// Let the timer come due, then look.
    async fn looks_later(&self) -> Vec<String> {
        self.rig.fleet.tracking().keepings.later(SYNC_EVERY);
        self.looks().await
    }

    async fn plans_and_approves(&self) {
        recorded(&self.rig, first(&self.job)).await;
        self.approves(1).await;
        self.looks().await;
    }

    fn titles(&self, parent: &str) -> Vec<String> {
        self.copy.children(parent).iter().filter_map(|one| self.copy.bead(one)).map(|one| one.title).collect()
    }

    fn groups(&self) -> Vec<String> {
        self.copy.children(&self.issue)
    }

    /// The bead of task `at` of group `of`, in the order the plan holds them.
    fn task(&self, of: usize, at: usize) -> String {
        self.copy.children(&self.groups()[of])[at].clone()
    }

    fn status(&self, bead: &str) -> String {
        self.copy.bead(bead).expect("a bead").status
    }

    async fn told(&self, containing: &str) -> usize {
        let process = self.rig.stand_in.process_of(self.job.as_str()).expect("a process");
        let sent = process.sent.lock().unwrap();
        sent.iter().filter(|line| line.contains(containing)).count()
    }
}

#[tokio::test]
async fn an_approved_plan_is_filed_under_the_jobs_issue_each_group_blocking_the_next() {
    let boat = boat().await;
    recorded(&boat.rig, first(&boat.job)).await;
    boat.looks().await;
    assert!(boat.groups().is_empty(), "a plan nobody approved is not filed");

    boat.approves(1).await;
    boat.looks().await;

    assert_eq!(boat.titles(&boat.issue), ["Store", "Fleet"], "one child a group, in plan order");
    let [store, fleet] = boat.groups().try_into().expect("two groups");
    assert_eq!(boat.titles(&store), ["Add the tables", "Add the migration"], "one child a task, under its group");
    assert_eq!(boat.titles(&fleet), ["Record it"]);
    assert!(boat.copy.bead(&store).expect("a bead").blocked_by.is_empty());
    assert_eq!(boat.copy.bead(&fleet).expect("a bead").blocked_by, [store.clone()], "each group blocks the next");
    assert_eq!(boat.remote.children(&boat.issue), boat.groups(), "and the same look synced them");
    assert_eq!(boat.remote.bead(&boat.issue).expect("the issue").status, "in_progress", "the claim stands");
}

#[tokio::test]
async fn a_job_that_holds_no_beads_issue_mirrors_nothing() {
    let remote = FakeBeadsRemote::new();
    let copy = Arc::new(FakeBeads::on(&remote));
    let rig = machine("mac-a", &copy).await;
    let issue = copy.create("Somebody else's");
    copy.sync("/").expect("pushed");
    let job = rig.start().await;
    recorded(&rig, first(&job)).await;
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: job.clone(), version: 1 }).await.expect("approved");

    Arc::clone(&rig.fleet).backlogs_looked().await;

    assert!(copy.children(&issue).is_empty());
    assert_eq!(adapter_traits::Tracker::open_issues(&*copy, "/").expect("listed").len(), 1, "nothing was filed anywhere");
}

#[tokio::test]
async fn a_create_retried_after_a_failed_sync_files_no_second_bead() {
    let boat = boat().await;
    recorded(&boat.rig, first(&boat.job)).await;
    boat.approves(1).await;
    boat.copy.set_unreachable(true);
    let said = boat.looks().await;
    assert!(said.iter().any(|line| line.contains("did not happen")), "{said:?}");
    assert!(boat.remote.children(&boat.issue).is_empty(), "nothing reached the remote");
    let filed = boat.groups();
    assert_eq!(filed.len(), 2, "but the copy holds them");

    boat.copy.set_unreachable(false);
    boat.looks_later().await;

    assert_eq!(boat.groups(), filed, "the same beads");
    let all = adapter_traits::Tracker::open_issues(&*boat.copy, "/").expect("listed");
    assert_eq!(all.len(), 6, "the issue, two groups and three tasks, each once");
    assert_eq!(boat.remote.children(&boat.issue), filed, "and the retry synced them");
}

#[tokio::test]
async fn a_backlog_that_refuses_never_holds_up_approval_and_is_tried_again_at_the_next_timed_sync() {
    let boat = boat().await;
    recorded(&boat.rig, first(&boat.job)).await;
    boat.copy.set_refusing(true);

    boat.approves(1).await;
    let said = boat.looks().await;
    assert!(boat.groups().is_empty());
    assert!(said.iter().any(|line| line.contains("mirroring the plan")), "{said:?}");
    assert!(boat.looks().await.is_empty(), "a failure is said once");

    boat.copy.set_refusing(false);
    boat.looks().await;
    assert!(boat.groups().is_empty(), "and not tried before the timer");
    let said = boat.looks_later().await;

    assert_eq!(boat.titles(&boat.issue), ["Store", "Fleet"]);
    assert!(said.iter().any(|line| line.contains("went through again")), "{said:?}");
}

#[tokio::test]
async fn a_plan_recorded_again_keeps_matched_beads_renames_adds_and_closes_what_went_with_a_reason() {
    let boat = boat().await;
    boat.plans_and_approves().await;
    let (store, fleet) = (boat.groups()[0].clone(), boat.groups()[1].clone());
    let (tables, migration, record) = (boat.task(0, 0), boat.task(0, 1), boat.task(1, 0));

    let again = plan(
        &boat.job,
        vec![group(
            Some("g1"),
            "Store",
            "rust-engineer",
            &["crates/store"],
            &[(Some("t1"), "Add the plan tables"), (Some("t2"), "Add the migration"), (None, "Write the docs")],
        )],
    );
    recorded(&boat.rig, again).await;
    boat.looks().await;
    assert_eq!(boat.titles(&store), ["Add the tables", "Add the migration"], "a version nobody approved changes nothing");
    boat.approves(2).await;
    boat.looks().await;

    assert_eq!(boat.groups()[0], store, "the group keeps its bead");
    assert_eq!(boat.copy.children(&store)[..2], [tables.clone(), migration.clone()], "matched tasks keep theirs");
    assert_eq!(boat.copy.bead(&tables).expect("a bead").title, "Add the plan tables", "renamed");
    assert_eq!(boat.titles(&store).last().map(String::as_str), Some("Write the docs"), "a new task gets a bead");
    for gone in [&fleet, &record] {
        let bead = boat.copy.bead(gone).expect("kept, closed");
        assert_eq!((bead.status.as_str(), bead.reason.as_str()), ("closed", "removed from the plan"));
    }
    assert_eq!(boat.status(&migration), "open");
}

#[tokio::test]
async fn a_task_the_plan_moves_moves_its_bead_and_a_group_closes_with_its_last_task() {
    let boat = boat().await;
    boat.plans_and_approves().await;
    let (store, fleet) = (boat.groups()[0].clone(), boat.groups()[1].clone());
    let (tables, migration, record) = (boat.task(0, 0), boat.task(0, 1), boat.task(1, 0));
    let list = |a, b, c| {
        vec![task("Add the tables", a), task("Add the migration", b), task("Record it", c)]
    };
    use ReportedTaskStatus::{Completed, InProgress, Pending};

    tasks(&boat.rig, &boat.job, list(InProgress, Pending, Pending)).await;
    boat.looks().await;
    assert_eq!(boat.status(&tables), "in_progress", "working is in progress");
    assert_eq!(boat.status(&migration), "open");

    tasks(&boat.rig, &boat.job, list(Completed, Completed, Pending)).await;
    boat.looks().await;
    assert_eq!((boat.status(&tables), boat.status(&migration)), ("closed".into(), "closed".into()), "handed in is closed");
    assert_eq!(boat.status(&store), "closed", "a group closes when all its tasks are closed");
    assert_eq!(boat.copy.bead(&tables).expect("a bead").reason, "done in the plan");
    assert_eq!(boat.status(&fleet), "open");

    tasks(&boat.rig, &boat.job, list(Completed, Completed, Completed)).await;
    boat.looks().await;
    assert_eq!((boat.status(&record), boat.status(&fleet)), ("closed".into(), "closed".into()));
    assert_eq!(boat.remote.bead(&fleet).expect("pushed").status, "closed");

    tasks(&boat.rig, &boat.job, list(Completed, Completed, Pending)).await;
    boat.looks().await;
    assert_eq!((boat.status(&record), boat.status(&fleet)), ("open".into(), "open".into()), "back to open, and its group with it");
    assert_eq!(boat.status(&store), "closed");
}

#[tokio::test]
async fn an_item_of_the_agents_list_the_plan_does_not_hold_moves_no_bead() {
    let boat = boat().await;
    boat.plans_and_approves().await;

    tasks(&boat.rig, &boat.job, vec![task("Something else entirely", ReportedTaskStatus::Completed)]).await;
    boat.looks().await;

    let open: Vec<String> = boat
        .groups()
        .iter()
        .flat_map(|group| std::iter::once(group.clone()).chain(boat.copy.children(group)))
        .map(|one| boat.status(&one))
        .collect();
    assert!(open.iter().all(|status| status == "open"), "{open:?}");
}

#[tokio::test]
async fn a_merge_closes_the_issue_and_every_open_child_with_the_pull_requests_address() {
    let boat = boat_reading(ON_MAIN).await;
    boat.plans_and_approves().await;
    let tables = boat.task(0, 0);
    tasks(&boat.rig, &boat.job, vec![task("Add the tables", ReportedTaskStatus::Completed)]).await;
    boat.looks().await;
    let closed_by_plan = boat.copy.bead(&tables).expect("a bead").reason;

    let at = boat.rig.fleet.now().as_str().to_string();
    for (kind, target, state) in [("pr", "1", AttachmentState::Spent), ("branch", "armada/1", AttachmentState::GivenBack)] {
        boat.rig
            .fleet
            .store()
            .lock()
            .await
            .attach(
                &KeptAttachment {
                    holder: Holder::session(boat.job.as_str()),
                    kind: kind.into(),
                    manifest_id: manifest(&boat.rig).as_str().to_string(),
                    target: target.into(),
                    state,
                    detail: Default::default(),
                    since: at.clone(),
                    changed_at: at.clone(),
                },
                false,
            )
            .unwrap();
    }
    let forge = &boat.rig.fleet.vcs().main_ci;
    forge.followed_are(Some(Vec::new()));
    forge.recently_merged_are(Some(vec![RecentlyMerged {
        number: 1,
        title: FromOutside::verbatim("Mirror the plan"),
        branch: FromOutside::verbatim("armada/1"),
        url: FromOutside::verbatim(PULL),
        author: None,
        merged_at: FromOutside::verbatim(at.as_str()),
        commit: None,
    }]));
    let served = boat.rig.fleet.repositories().served().remove(0);
    boat.rig.fleet.notice_merged(&served).await;
    boat.rig.fleet.pulls_told(&served).await;
    assert_eq!(boat.status(&boat.issue), "in_progress", "the notice waits on no bd: nothing closed yet");

    boat.looks().await;

    let everything: Vec<String> = std::iter::once(boat.issue.clone())
        .chain(boat.groups())
        .chain(boat.groups().iter().flat_map(|group| boat.copy.children(group)))
        .collect();
    assert_eq!(everything.len(), 6);
    for one in &everything {
        assert_eq!(boat.status(one), "closed", "{one}");
    }
    assert_eq!(boat.copy.bead(&boat.issue).expect("the issue").reason, format!("merged in {PULL}"));
    assert_eq!(boat.copy.bead(&boat.groups()[1]).expect("a group").reason, format!("merged in {PULL}"));
    assert_eq!(boat.copy.bead(&tables).expect("a bead").reason, closed_by_plan, "one already closed keeps its reason");
    assert_eq!(boat.remote.bead(&boat.issue).expect("pushed").status, "closed");
}

#[tokio::test]
async fn a_job_that_ended_before_its_pull_request_merged_still_has_its_issue_closed_at_the_merge() {
    let boat = boat_reading(ON_MAIN).await;
    boat.plans_and_approves().await;
    let at = boat.rig.fleet.now().as_str().to_string();
    {
        let mut store = boat.rig.fleet.store().lock().await;
        for (kind, target) in [("pr", "1"), ("branch", "armada/1")] {
            store
                .attach(
                    &KeptAttachment {
                        holder: Holder::session(boat.job.as_str()),
                        kind: kind.into(),
                        manifest_id: manifest(&boat.rig).as_str().to_string(),
                        target: target.into(),
                        state: AttachmentState::Standing,
                        detail: Default::default(),
                        since: at.clone(),
                        changed_at: at.clone(),
                    },
                    false,
                )
                .unwrap();
        }
    }
    boat.looks().await;
    // The Job ends: everything it held is given back, its issue row too.
    boat.rig.fleet.store().lock().await.give_back(&Holder::session(boat.job.as_str()), None, &at).unwrap();

    let forge = &boat.rig.fleet.vcs().main_ci;
    forge.followed_are(Some(Vec::new()));
    forge.recently_merged_are(Some(vec![RecentlyMerged {
        number: 1,
        title: FromOutside::verbatim("Mirror the plan"),
        branch: FromOutside::verbatim("armada/1"),
        url: FromOutside::verbatim(PULL),
        author: None,
        // Older than the time a merge is news.
        merged_at: FromOutside::verbatim("2026-10-20T00:00:00.000Z"),
        commit: None,
    }]));
    let served = boat.rig.fleet.repositories().served().remove(0);
    boat.rig.fleet.notice_merged(&served).await;
    boat.rig.fleet.pulls_told(&served).await;
    boat.looks().await;

    for one in std::iter::once(boat.issue.clone()).chain(boat.groups()) {
        assert_eq!(boat.status(&one), "closed", "{one}");
    }
    assert_eq!(boat.copy.bead(&boat.issue).expect("the issue").reason, format!("merged in {PULL}"));
}

/// Another machine's Fleet claims the issue in a copy that synced before this Job's claim, and its
/// write is the later one.
async fn lost_to(boat: &Boat, other: &Arc<FakeBeads>, holder: &str) {
    other.claim("/", &boat.issue, holder).expect("claimed there");
    other.sync("/").expect("pushed");
    assert_eq!(boat.remote.holder(&boat.issue), Some(holder.to_string()), "the later write wins on the remote");
}

#[tokio::test]
async fn the_loser_of_a_claim_is_told_once_naming_the_holder_and_its_issue_row_is_dropped() {
    let boat = unclaimed_reading(super::backlog::BEADS).await;
    let other = Arc::new(FakeBeads::on(&boat.remote));
    other.sync("/").expect("heard of the issue, and not yet of a claim");
    claims(&boat.rig, &boat.job, &boat.issue).await.expect("this Fleet's claim, answered yes");
    assert_eq!(boat.remote.holder(&boat.issue), Some(actor("mac-a", &boat.job)));
    lost_to(&boat, &other, "mac-b/0a1b2c3d").await;

    boat.looks().await;
    assert_eq!(boat.told("mac-b/0a1b2c3d").await, 0, "the copy has not heard yet, and nothing is due");
    boat.looks_later().await;

    assert_eq!(boat.told("mac-b/0a1b2c3d").await, 1, "told, naming who holds it");
    let manifest = manifest(&boat.rig);
    let rows = boat.rig.fleet.store().lock().await.attachments_at("issue", &boat.issue, Some(manifest.as_str())).unwrap();
    assert!(rows.iter().all(|row| row.state != AttachmentState::Standing), "its issue row is dropped: {rows:?}");

    boat.looks_later().await;
    boat.looks_later().await;
    assert_eq!(boat.told("mac-b/0a1b2c3d").await, 1, "once");
}

#[tokio::test]
async fn a_job_that_only_named_an_issue_someone_else_holds_is_not_told_it_lost_a_claim() {
    let boat = boat().await;
    let named = boat.copy.create("Held on another machine");
    boat.copy.sync("/").expect("pushed");
    let other = Arc::new(FakeBeads::on(&boat.remote));
    other.sync("/").expect("heard");
    other.claim("/", &named, "mac-b/0a1b2c3d").expect("claimed there");
    other.sync("/").expect("pushed");
    {
        let mut store = boat.rig.fleet.store().lock().await;
        let now = boat.rig.fleet.now().as_str().to_string();
        let reference = IssueReference::read(&named).expect("a key");
        crate::session_issues::issue_held(
            &mut store,
            &Holder::session(boat.job.as_str()),
            manifest(&boat.rig).as_str(),
            &reference,
            &Default::default(),
            &now,
        )
        .expect("held");
    }

    boat.looks_later().await;

    assert_eq!(boat.told("mac-b/0a1b2c3d").await, 0);
    let rows = boat.rig.fleet.store().lock().await.attachments_at("issue", &named, Some(manifest(&boat.rig).as_str())).unwrap();
    assert!(rows.iter().all(|row| row.state == AttachmentState::Standing), "{rows:?}");
}
