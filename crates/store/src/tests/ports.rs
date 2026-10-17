//! A checkout's port span, and Fleet's listener's, told apart by the type
//! rather than by nullable columns a test would otherwise have to police.

use core_model::Timestamp;

use crate::tests::{open, TempDir};
use crate::{PortClaim, PortClaimant, Store, WriteError};

/// The repository whose main checkout claims in these cases.
fn root() -> String {
    String::from("/repos/storefront")
}

fn claimed_at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z")
}

/// The main checkout — the proof run after a merge, and a server started with
/// no Job, both draw from this — gets a claim of its own, keyed by neither a
/// Job nor a run.
#[test]
fn the_main_checkout_gets_a_claim_of_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Checkout(root()),
            base: 42000,
            width: 2,
            claimed_at: claimed_at(),
        })
        .expect("the main checkout's claim is recorded");

    let read = store
        .port_span_for_checkout(&root())
        .expect("the read succeeds")
        .expect("a claim is there");
    assert_eq!(read.claimant, PortClaimant::Checkout(root()));
    assert_eq!(read.base, 42000);

    let all = store.every_port_claim().expect("every claim is read");
    assert_eq!(all.len(), 1, "one claim, naming the main checkout");
    assert_eq!(all[0].claimant, PortClaimant::Checkout(root()));

    store
        .release_port_span(&PortClaimant::Checkout(root()))
        .expect("the main checkout's claim is released");
    assert!(
        store
            .every_port_claim()
            .expect("every claim is read")
            .is_empty(),
        "the main checkout's own release takes its claim, with no Job involved at all"
    );
}

/// A second claim for the main checkout is refused rather than silently
/// replacing it — the same rule a Job's own second claim is refused by.
#[test]
fn a_second_claim_for_the_main_checkout_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Checkout(root()),
            base: 42000,
            width: 2,
            claimed_at: claimed_at(),
        })
        .expect("the first claim is recorded");
    let refused = store.claim_port_span(&PortClaim {
        claimant: PortClaimant::Checkout(root()),
        base: 42010,
        width: 2,
        claimed_at: claimed_at(),
    });
    assert!(
        matches!(refused, Err(WriteError::Database(_))),
        "a second claim for the main checkout is a conflict, not a replacement: {refused:?}"
    );
}

/// Fleet's own listener — the one port Bridge connects to — gets a claim of
/// its own, keyed by neither a Job nor the main checkout.
#[test]
fn the_fleet_listener_gets_a_claim_of_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 43000,
            width: 1,
            claimed_at: claimed_at(),
        })
        .expect("Fleet's own claim is recorded");

    let read = store
        .port_span_for_fleet_listener()
        .expect("the read succeeds")
        .expect("a claim is there");
    assert_eq!(read.claimant, PortClaimant::FleetListener);
    assert_eq!(read.base, 43000);
    assert_eq!(read.width, 1, "one listener, one port");

    store
        .release_port_span(&PortClaimant::FleetListener)
        .expect("Fleet's own claim is released");
    assert!(
        store
            .every_port_claim()
            .expect("every claim is read")
            .is_empty(),
        "the release takes it with no Job and no main checkout involved"
    );
}

/// A second claim for Fleet's listener is refused rather than silently
/// replacing the first — **one Fleet per store**, which is what the partial
/// unique index says and what the runtime file says from the other side.
#[test]
fn a_second_claim_for_the_fleet_listener_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 43000,
            width: 1,
            claimed_at: claimed_at(),
        })
        .expect("the first claim is recorded");
    let refused = store.claim_port_span(&PortClaim {
        claimant: PortClaimant::FleetListener,
        base: 43010,
        width: 1,
        claimed_at: claimed_at(),
    });
    assert!(
        matches!(refused, Err(WriteError::Database(_))),
        "a second claim for one Fleet's listener is a conflict, not a replacement: {refused:?}"
    );
}

/// **Two repositories' main checkouts hold two spans**, and releasing one
/// leaves the other — the claim is keyed by the repository's root.
#[test]
fn two_repositories_main_checkouts_hold_separate_claims() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mailer = String::from("/repos/mailer");
    for (holder, base) in [(root(), 42_000), (mailer.clone(), 42_008)] {
        let claimant = PortClaimant::Checkout(holder);
        let claim = PortClaim {
            claimant,
            base,
            width: 8,
            claimed_at: claimed_at(),
        };
        store
            .claim_port_span(&claim)
            .expect("each checkout claims its own");
    }
    let base_of = |store: &Store, at: &str| {
        store
            .port_span_for_checkout(at)
            .expect("read")
            .map(|claim| claim.base)
    };
    assert_eq!(base_of(&store, &root()), Some(42_000));
    assert_eq!(base_of(&store, &mailer), Some(42_008));
    store
        .release_port_span(&PortClaimant::Checkout(mailer.clone()))
        .expect("released");
    assert_eq!(
        (base_of(&store, &root()), base_of(&store, &mailer)),
        (Some(42_000), None)
    );
}

/// **Two claimants, one table, told apart by the type.** A Fleet serving a
/// repository holds its listener's port *and* the main checkout's span, so
/// both coexist and each reads back as itself.
#[test]
fn the_main_checkout_and_the_fleet_listener_are_separate_claims() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for claim in [
        PortClaim {
            claimant: PortClaimant::Checkout(root()),
            base: 41004,
            width: 4,
            claimed_at: claimed_at(),
        },
        PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 41008,
            width: 1,
            claimed_at: claimed_at(),
        },
    ] {
        store
            .claim_port_span(&claim)
            .expect("each claim is written");
    }

    assert_eq!(store.every_port_claim().expect("read").len(), 2);
    assert_eq!(
        store
            .port_span_for_checkout(&root())
            .expect("read")
            .expect("a claim")
            .base,
        41004
    );
    assert_eq!(
        store
            .port_span_for_fleet_listener()
            .expect("read")
            .expect("a claim")
            .base,
        41008
    );
}
