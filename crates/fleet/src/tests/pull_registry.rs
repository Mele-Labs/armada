//! A pull request's owner in the central store, with two Fleets on one scripted store: a Session
//! on A claims a pull request and B's hub names it, B with the switch off names nothing, a store
//! that cannot be reached leaves the claim made and the field absent, a Job's own pull request is
//! registered as it opens, and a pull request that leaves the open list loses its row.
//!
//! **Here and not in `crates/acceptance`** for `red_claim`'s reason: the claim is a `Fleet` call
//! and the chip is read back off its hub.

use adapter_traits::{CiState, FromOutside, Holder, HolderKind, OpenPull, Store};
use api::{PullRequestPanel, Sessions};
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport};
use testkit::FakeStore;

use super::main_ci::{pull, run, the_hub, ONE};
use super::main_fix::root;
use super::pull_request_panel::a_page;
use super::red_claim::{machine, Machine, SLUG};
use crate::red_claim::owner_key_of;
use adapter_traits::{PullQueue, WatchedPull};

/// What the forge lists open: `numbers`, and the same as a watcher reads them.
fn open(machine: &Machine, numbers: &[u64]) {
    let forge = &machine.fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    forge.pulls_are(Some(numbers.iter().map(|number| pull(*number, Some(CiState::Passed), &[])).collect::<Vec<OpenPull>>()));
    forge.watched_are(Some(
        numbers
            .iter()
            .map(|number| WatchedPull {
                number: *number,
                branch: FromOutside::verbatim(format!("armada/{number}")),
                url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
                author: None,
                head: Some(ONE.to_string()),
                conflicting: false,
                queue: PullQueue::Outside,
                checks: Vec::new(),
            })
            .collect(),
    ));
}

async fn a_session(machine: &Machine, id: &str) {
    machine
        .fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Started { cwd: root(&machine.fleet).await, title: None, origin: SessionOrigin::Terminal, mod_version: None },
        })
        .await
        .unwrap();
}

async fn claims(machine: &Machine, id: &str, number: u64) -> Result<ipc::PullRequestClaimed, api::Refusal> {
    machine
        .fleet
        .claim_pull_request(None, ipc::ClaimPullRequest { number, session_id: Some(SessionId::carried(id)), job_id: None })
        .await
}

/// The row a pull request is under in the shared store.
fn row_of(store: &FakeStore, number: u64) -> Option<Holder> {
    let key = owner_key_of(SLUG, number).expect("a key");
    store.owners(&[key.clone()]).expect("reads").remove(&key)
}

async fn owner_in_hub(machine: &Machine, number: u64) -> Option<ipc::PullRequestOwner> {
    the_hub(&machine.fleet)
        .await
        .pull_requests
        .into_iter()
        .find(|one| one.number == number)
        .unwrap_or_else(|| panic!("#{number} is in the hub"))
        .owner
}

fn session_on(machine: &str, id: &str, this_machine: bool) -> ipc::PullRequestOwner {
    ipc::PullRequestOwner { kind: ipc::HolderKind::Session, id: id.into(), machine: machine.into(), this_machine }
}

#[tokio::test]
async fn a_session_claim_on_one_machine_is_an_owner_on_both_hubs() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    for each in [&a, &b] {
        open(each, &[30, 31]);
    }
    a_session(&a, "s-a").await;

    claims(&a, "s-a", 30).await.expect("claimed");
    let held = row_of(&shared, 30).expect("a row for #30");
    assert_eq!((held.kind, held.id.as_str(), held.machine.as_str()), (HolderKind::Session, "s-a", "mac-a"));
    assert!(row_of(&shared, 31).is_none(), "and none for the one nobody claimed");

    a.fleet.turn().await.unwrap();
    b.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&b, 30).await, Some(session_on("mac-a", "s-a", false)));
    assert_eq!(owner_in_hub(&a, 30).await, Some(session_on("mac-a", "s-a", true)));
    assert_eq!(owner_in_hub(&b, 31).await, None);
    assert_eq!(b.view.calls().iter().filter(|call| **call == "owners").count(), 1, "one read for the repository: {:?}", b.view.calls());
}

#[tokio::test]
async fn a_claim_is_drawn_on_the_claiming_machine_before_the_next_visit() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    open(&a, &[30]);
    a_session(&a, "s-a").await;
    a.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&a, 30).await, None);

    claims(&a, "s-a", 30).await.expect("claimed");
    assert_eq!(owner_in_hub(&a, 30).await, Some(session_on("mac-a", "s-a", true)));
}

#[tokio::test]
async fn a_pull_request_row_holds_a_hashed_key_and_no_name() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    open(&a, &[30]);
    a_session(&a, "s-a").await;
    claims(&a, "s-a", 30).await.expect("claimed");
    let key = owner_key_of(SLUG, 30).expect("a key");
    assert_eq!(key.as_str().len(), 64);
    assert!(!key.as_str().contains("armada") && !key.as_str().contains('/'));
}

#[tokio::test]
async fn a_repository_with_the_switch_off_registers_none_and_shows_none() {
    let shared = FakeStore::new();
    let (holder, quiet) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, false).await);
    for each in [&holder, &quiet] {
        open(each, &[30]);
    }
    a_session(&holder, "s-a").await;
    a_session(&quiet, "s-q").await;
    claims(&holder, "s-a", 30).await.expect("claimed");
    assert!(row_of(&shared, 30).is_some(), "a row stands in the store");

    quiet.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&quiet, 30).await, None, "another machine's row is not shown");
    claims(&quiet, "s-q", 30).await.expect("the claim itself is made");
    assert_eq!(row_of(&shared, 30).unwrap().id, "s-a", "and nothing of its own was written");
    assert_eq!(quiet.view.calls(), Vec::<&str>::new(), "the store was never asked");
}

#[tokio::test]
async fn an_owner_kept_from_an_earlier_reading_is_not_shown_once_the_switch_is_off() {
    let shared = FakeStore::new();
    let quiet = machine("mac-q", &shared, false).await;
    open(&quiet, &[30]);
    quiet.fleet.turn().await.unwrap();
    let root = root(&quiet.fleet).await;
    let kept = Holder::new(HolderKind::Session, "s-a", "mac-a", 1).expect("a holder");
    quiet.fleet.sweeping().lock().await.pulls.get_mut(&root).expect("read")[0].owner =
        Some(crate::pull_registry::PullOwner { holder: kept, here: false });

    assert_eq!(owner_in_hub(&quiet, 30).await, None);
}

#[tokio::test]
async fn a_store_that_cannot_be_reached_leaves_the_claim_made_and_the_field_absent() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    for each in [&a, &b] {
        open(each, &[30]);
    }
    a_session(&a, "s-a").await;
    a.view.set_unreachable(true);

    let claimed = claims(&a, "s-a", 30).await.expect("the claim is not the store's to refuse");
    assert_eq!(claimed.holder_id, "s-a");
    assert!(row_of(&shared, 30).is_none(), "no row was written");
    a.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&a, 30).await, None, "and the hub says nothing, with no error");

    a.view.set_unreachable(false);
    claims(&a, "s-a", 30).await.expect("claimed again");
    b.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&b, 30).await, Some(session_on("mac-a", "s-a", false)));
    b.view.set_unreachable(true);
    b.fleet.turn().await.unwrap();
    assert_eq!(owner_in_hub(&b, 30).await, None, "a store that stops answering takes the field away");
}

#[tokio::test]
async fn a_pull_request_that_leaves_the_open_list_loses_its_row() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    for each in [&a, &b] {
        open(each, &[30, 31]);
    }
    a_session(&a, "s-a").await;
    claims(&a, "s-a", 30).await.expect("claimed");
    claims(&a, "s-a", 31).await.expect("claimed");
    a.fleet.turn().await.unwrap();
    b.fleet.turn().await.unwrap();

    open(&b, &[31]);
    b.fleet.turn().await.unwrap();
    assert!(row_of(&shared, 30).is_none(), "#30 merged, so its row is gone");
    assert!(row_of(&shared, 31).is_some(), "#31 is still open");
    assert!(b.view.calls().contains(&"delete_owner"), "{:?}", b.view.calls());
}

#[tokio::test]
async fn a_forge_that_will_not_answer_deletes_nothing() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    open(&a, &[30]);
    a_session(&a, "s-a").await;
    claims(&a, "s-a", 30).await.expect("claimed");
    a.fleet.turn().await.unwrap();

    a.fleet.vcs().main_ci.pulls_are(None);
    a.fleet.turn().await.unwrap();
    assert!(row_of(&shared, 30).is_some());
}

#[tokio::test]
async fn the_pull_request_panels_page_carries_the_owner() {
    let shared = FakeStore::new();
    let (a, b) = (machine("mac-a", &shared, true).await, machine("mac-b", &shared, true).await);
    for each in [&a, &b] {
        open(each, &[30]);
    }
    a_session(&a, "s-a").await;
    claims(&a, "s-a", 30).await.expect("claimed");
    b.fleet.turn().await.unwrap();
    b.fleet.vcs().now_pull_request_page(Some(a_page()));
    let id = ipc::ManifestId::carried(b.fleet.repositories().served()[0].manifest().id().as_str());

    let page = b.fleet.get_pull_request_detail(id.clone(), 30).await.expect("read");
    assert_eq!(page.owner, Some(session_on("mac-a", "s-a", false)));
    let none = b.fleet.get_pull_request_detail(id, 99).await.expect("read");
    assert_eq!(none.owner, None);
}

#[tokio::test]
async fn a_jobs_pull_request_is_registered_as_it_opens() {
    use crate::tests::admitted::dispatched;
    use crate::tests::daemon::{a_proposal, diff_evidence, worktree_directory};
    use crate::tests::tools::submitted_by_the_one;

    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    let job = a.fleet.propose(a_proposal("fix the off-by-one in the log reader")).await.unwrap();
    worktree_directory(&a._home, &job);
    dispatched(&a.fleet, job.id()).await.unwrap();
    submitted_by_the_one(&a.fleet, diff_evidence()).await.unwrap();
    a.fleet.turn().await.unwrap();

    let held = row_of(&shared, 1).expect("the fake forge opens #1");
    assert_eq!((held.kind, held.id.as_str(), held.machine.as_str()), (HolderKind::Job, job.id().as_str(), "mac-a"));
}
