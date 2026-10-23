//! A pull request's owner in the central store, with two Fleets on one scripted store: a Session
//! on A claims a pull request and B's hub names it, B with the switch off names nothing, a store
//! that cannot be reached leaves the claim made and the field absent, a Job's own pull request is
//! registered as it opens, and a pull request that leaves the open list loses its row.
//!
//! **Here and not in `crates/acceptance`** for `red_claim`'s reason: the claim is a `Fleet` call
//! and the chip is read back off its hub.

use adapter_traits::{CiState, FromOutside, Holder, OpenPull, Store};
use api::Sessions;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport};
use testkit::FakeStore;

use super::main_ci::pull;
use super::main_ci::run;
use super::main_ci::ONE;
use super::main_fix::root;
use super::red_claim::{machine, Machine, SLUG};
use crate::red_claim::owner_key_of;
use adapter_traits::{PullQueue, FollowedPull};

/// What the forge lists open: `numbers`, and the same as a pull reading reads them.
fn open(machine: &Machine, numbers: &[u64]) {
    let forge = &machine.fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    forge.pulls_are(Some(numbers.iter().map(|number| pull(*number, Some(CiState::Passed), &[])).collect::<Vec<OpenPull>>()));
    forge.followed_are(Some(
        numbers
            .iter()
            .map(|number| FollowedPull {
                number: *number,
                branch: FromOutside::verbatim(format!("armada/{number}")),
                url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
                author: None,
                head: Some(ONE.to_string()),
                conflicting: false,
                queue: PullQueue::Outside,
                checks: Vec::new(),
                title: None,
                auto_merge: false,
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
        .claim_pull_request(None, ipc::ClaimPullRequest { number, session_id: Some(SessionId::carried(id)) })
        .await
}

/// The row a pull request is under in the shared store.
fn row_of(store: &FakeStore, number: u64) -> Option<Holder> {
    let key = owner_key_of(SLUG, number).expect("a key");
    store.owners(&[key.clone()]).expect("reads").remove(&key)
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

/// **A draft past the pull reading's hundred is claimed by its number**: the owner's four older drafts in
/// a busy repository were refused as not open (16 Oct 2026). One the forge says is closed is not.
#[tokio::test]
async fn an_open_pull_request_the_pull_reading_did_not_list_is_claimed_by_its_number() {
    let shared = FakeStore::new();
    let a = machine("mac-a", &shared, true).await;
    open(&a, &[30]);
    a_session(&a, "s-a").await;
    let facts = |standing| adapter_traits::PullRequestFacts {
        standing,
        branch: "nm/older-draft".into(),
        auto_merge: false,
        title: "An older draft".into(),
        url: "https://forge.invalid/armada/pull/77".into(),
        body: None,
        closes: Vec::new(),
    };

    a.fleet.vcs().now_pull_request(Some(facts(adapter_traits::PullRequestStanding::Draft)));
    claims(&a, "s-a", 77).await.expect("an open draft is claimed by its number");
    assert_eq!(row_of(&shared, 77).map(|held| held.id), Some("s-a".to_string()));

    a.fleet.vcs().now_pull_request(Some(facts(adapter_traits::PullRequestStanding::Closed)));
    let refused = claims(&a, "s-a", 78).await.expect_err("a closed one is not");
    assert!(format!("{refused:?}").contains("not an open pull request"), "{refused:?}");
}
