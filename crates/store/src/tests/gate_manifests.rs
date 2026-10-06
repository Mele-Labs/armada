//! Replacing a Job's gating Manifests.

use core_model::{GateManifest, GateOutcome, ManifestId, NotRunReason};

use crate::tests::{created_at, job_id, open, top_level, ulid, TempDir};

fn gate(id: &str, outcome: GateOutcome) -> GateManifest {
    GateManifest {
        manifest_id: ManifestId::carried(ulid(id)),
        outcome,
    }
}

#[test]
fn replacing_the_gates_swaps_the_whole_set_and_keeps_the_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01REPLACEGATES");
    store.insert_job(&job, &created_at()).expect("stored");

    let now = [
        gate("01ZZ", GateOutcome::RanAndFailed),
        gate(
            "01AA",
            GateOutcome::DidNotRun(NotRunReason::PathConditionUnmet),
        ),
    ];
    store
        .replace_gate_manifests(&job_id("01REPLACEGATES"), &now)
        .expect("replaced");
    let read = store.load_job(&job_id("01REPLACEGATES")).expect("loads");
    assert_eq!(read.gate_manifests(), now);

    store
        .replace_gate_manifests(&job_id("01REPLACEGATES"), &[])
        .expect("cleared");
    let read = store.load_job(&job_id("01REPLACEGATES")).expect("loads");
    assert!(read.gate_manifests().is_empty());
}
