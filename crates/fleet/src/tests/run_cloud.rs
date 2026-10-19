//! How a repository's runs are kept in the cloud: Armada's central store as the profile the
//! repository is bound to, and a bucket the person owns. Both are written and read through the
//! seam the other places use, so a dropped line and a failed write behave as they do there.
//!
//! The store is `FakeStore` and the bucket is `FakeBucket`; neither reaches a network.

use std::sync::Arc;

use adapter_traits::{RunRow, Store};
use api::{Commands, Queries};
use ipc::{DropRunFact, RunCloud, RunFactKind, RunFactState, RunMemory, RunMemoryPlace, RunPlace};
use store::KeptRun;
use testkit::{FakeBucket, FakeStore};

use super::red_claim::{machine, Machine, Planted, SLUG};
use super::run_memory::{commands, ran, read, running, turn_over};
use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use crate::red_claim::repository_key_of;
use crate::run_memory::places;
use crate::session_host::Heard;

const BUCKET: &str = "s3://shop-runs/armada";

fn kept(command: &str, kind: &str, last_ran: u64) -> KeptRun {
    KeptRun {
        workspace: ".".into(),
        kind: kind.into(),
        command: command.into(),
        port: None,
        succeeded: true,
        last_ran,
        source: "jobs".into(),
        kept_in: None,
        why: None,
    }
}

fn row(command: &str, kind: &str, last_ran: u64) -> RunRow {
    RunRow {
        workspace: ".".into(),
        kind: kind.into(),
        command: command.into(),
        port: None,
        succeeded: true,
        last_ran,
        source: "jobs".into(),
    }
}

fn cloud(root: &str, form: RunCloud) -> RunMemoryPlace {
    RunMemoryPlace { repository: root.to_string(), place: RunPlace::Cloud, synced_folder: None, cloud: Some(form) }
}

fn refusal_code(refused: &api::Refusal) -> &str {
    match refused {
        api::Refusal::Unacceptable(error) => &error.code,
        other => panic!("not a 422: {other:?}"),
    }
}

/// A machine signed in to the account, whose repository has `facts` learnt.
async fn signed_in(shared: &FakeStore, facts: &[KeptRun]) -> (Machine, String) {
    let machine = machine("mac-a", shared, false).await;
    let root = super::main_fix::root(&machine.fleet).await;
    machine.fleet.get_run_memory(root.clone()).await.expect("started");
    machine.fleet.store().lock().await.merge_run_facts(&root, facts).expect("learnt");
    (machine, root)
}

fn held(shared: &FakeStore) -> Vec<RunRow> {
    shared.runs_of(&repository_key_of(SLUG).expect("a key"))
}

#[tokio::test]
async fn choosing_the_store_writes_what_is_remembered_under_the_repositorys_hash_and_says_where() {
    let shared = FakeStore::new();
    let (machine, root) = signed_in(&shared, &[kept("cargo test", "tests", 100)]).await;

    let chosen = machine.fleet.set_run_memory_place(cloud(&root, RunCloud::Store {})).await.expect("chosen");
    assert_eq!((chosen.place, chosen.cloud.clone()), (RunPlace::Cloud, Some(RunCloud::Store {})));

    assert_eq!(held(&shared), vec![row("cargo test", "tests", 100)]);
    assert_eq!(machine.fleet.get_run_memory_place(root.clone()).await.expect("read"), chosen);
    let memory = read_memory(&machine, &root).await;
    let run = &memory.workspaces[0].runs[0];
    assert_eq!((run.state, run.kept_in), (RunFactState::Kept, Some(RunPlace::Cloud)));
}

async fn read_memory(machine: &Machine, root: &str) -> RunMemory {
    machine.fleet.get_run_memory(root.to_string()).await.expect("read")
}

#[tokio::test]
async fn what_another_machine_put_in_the_store_is_merged_in_before_the_write_and_stays() {
    let shared = FakeStore::new();
    shared.view().put_runs(&repository_key_of(SLUG).expect("a key"), &[row("cargo clippy", "lint", 50)]).expect("another machine");
    let (machine, root) = signed_in(&shared, &[kept("cargo test", "tests", 100)]).await;

    machine.fleet.set_run_memory_place(cloud(&root, RunCloud::Store {})).await.expect("chosen");

    let mut commands: Vec<String> = held(&shared).into_iter().map(|run| run.command).collect();
    commands.sort();
    assert_eq!(commands, ["cargo clippy", "cargo test"]);
    let learnt: Vec<String> = read_memory(&machine, &root).await.workspaces[0].runs.iter().map(|run| run.fact.command.clone()).collect();
    assert_eq!(learnt.len(), 2, "{learnt:?}");
}

#[tokio::test]
async fn a_dropped_line_leaves_the_store_and_is_not_merged_back() {
    let shared = FakeStore::new();
    let (machine, root) = signed_in(&shared, &[kept("cargo test", "tests", 100), kept("cargo clippy", "lint", 90)]).await;
    machine.fleet.set_run_memory_place(cloud(&root, RunCloud::Store {})).await.expect("chosen");

    let after = machine
        .fleet
        .drop_run_fact(DropRunFact { repository: root.clone(), workspace: ".".into(), kind: RunFactKind::Tests, command: "cargo test".into() })
        .await
        .expect("dropped");

    assert_eq!(commands(&after), vec![(".".to_string(), RunFactKind::Lint, "cargo clippy".to_string())]);
    assert_eq!(held(&shared), vec![row("cargo clippy", "lint", 90)]);
}

#[tokio::test]
async fn a_store_that_goes_away_keeps_the_facts_waiting_and_the_next_write_brings_them() {
    let shared = FakeStore::new();
    let (machine, root) = signed_in(&shared, &[kept("cargo test", "tests", 100)]).await;
    machine.fleet.set_run_memory_place(cloud(&root, RunCloud::Store {})).await.expect("chosen");

    machine.fleet.store().lock().await.merge_run_facts(&root, &[kept("pnpm test", "tests", 300)]).expect("learnt");
    machine.view.set_unreachable(true);
    machine.fleet.run_memory_written(&root, None).await;

    let waiting = read_memory(&machine, &root).await;
    let runs = &waiting.workspaces[0].runs;
    assert_eq!(runs.len(), 2, "nothing is dropped for the store being down");
    let late = runs.iter().find(|run| run.fact.command == "pnpm test").expect("the new fact");
    assert_eq!(late.state, RunFactState::Waiting);
    assert!(late.why.as_deref().is_some_and(|why| why.contains("could not be reached")), "{:?}", late.why);
    assert_eq!(held(&shared).len(), 1, "the store still holds what it held");

    machine.view.set_unreachable(false);
    machine.fleet.run_memory_written(&root, None).await;

    let kept_now = read_memory(&machine, &root).await;
    assert!(kept_now.workspaces[0].runs.iter().all(|run| run.state == RunFactState::Kept));
    assert_eq!(held(&shared).len(), 2);
}

#[tokio::test]
async fn a_store_that_cannot_be_reached_or_a_machine_signed_out_refuses_the_place_and_the_old_one_stays() {
    let shared = FakeStore::new();
    let (machine, root) = signed_in(&shared, &[]).await;
    machine.view.set_unreachable(true);

    let down = machine.fleet.set_run_memory_place(cloud(&root, RunCloud::Store {})).await.expect_err("down");
    assert_eq!(refusal_code(&down), crate::run_memory::CLOUD_UNREACHABLE);
    assert_eq!(machine.fleet.get_run_memory_place(root.clone()).await.expect("read").place, RunPlace::ThisMac);

    let rig = rig_placing(None);
    let out = rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Store {})).await.expect_err("signed out");
    assert_eq!(refusal_code(&out), crate::run_memory::CLOUD_UNSIGNED);
    assert_eq!(rig.fleet.get_run_memory_place(rig.root.clone()).await.expect("read").place, RunPlace::ThisMac);
}

fn bucketed(bucket: &Arc<FakeBucket>) -> Rig {
    let bucket = Arc::clone(bucket);
    rig_fitted(move |fittings| fittings.reaching = Arc::new(Planted::with_bucket(bucket)))
}

#[tokio::test]
async fn a_job_learns_a_command_and_the_bucket_holds_it_in_the_kit_layout_without_a_press() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let chosen = rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(BUCKET.into()))).await.expect("chosen");
    assert_eq!(chosen.cloud, Some(RunCloud::Bucket(BUCKET.into())));
    let id = running(&rig).await;

    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "ok")));
    turn_over(&rig, &id).await;
    eventually(|| async { bucket.object(&places::object_for(places::FOLDER, &rig.root)).is_some() }).await;

    assert_eq!(bucket.names(), vec![places::object_for(places::FOLDER, &rig.root)]);
    let text = String::from_utf8(bucket.object(&places::object_for(places::FOLDER, &rig.root)).expect("an object")).expect("text");
    assert!(text.contains("cargo test") && text.contains("kind = \"tests\""), "{text}");
    eventually(|| async { read(&rig).await.workspaces.first().is_some_and(|one| one.runs[0].kept_in == Some(RunPlace::Cloud)) }).await;
}

#[tokio::test]
async fn an_object_another_machine_wrote_is_merged_in_and_a_dropped_line_leaves_the_object() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    let other = places::encode("/elsewhere/shop", &[kept("cargo clippy", "lint", 50)]).expect("encoded");
    bucket.plant(&places::object_for(places::FOLDER, &rig.root), other.as_bytes());

    rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(BUCKET.into()))).await.expect("chosen");
    assert_eq!(commands(&read(&rig).await), vec![(".".to_string(), RunFactKind::Lint, "cargo clippy".to_string())]);

    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Ran(ran("t1", "cargo test", "ok")));
    turn_over(&rig, &id).await;
    eventually(|| async { String::from_utf8_lossy(&bucket.object(&places::object_for(places::FOLDER, &rig.root)).unwrap_or_default()).contains("cargo test") }).await;
    let text = String::from_utf8(bucket.object(&places::object_for(places::FOLDER, &rig.root)).expect("an object")).expect("text");
    assert!(text.contains("cargo clippy"), "what was there stays: {text}");

    rig.fleet
        .drop_run_fact(DropRunFact { repository: rig.root.clone(), workspace: ".".into(), kind: RunFactKind::Lint, command: "cargo clippy".into() })
        .await
        .expect("dropped");
    let text = String::from_utf8(bucket.object(&places::object_for(places::FOLDER, &rig.root)).expect("an object")).expect("text");
    assert!(!text.contains("cargo clippy") && text.contains("cargo test"), "{text}");
}

#[tokio::test]
async fn a_bucket_the_key_may_not_write_keeps_the_facts_waiting_with_the_reason_and_writes_when_it_may() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(BUCKET.into()))).await.expect("chosen");
    rig.fleet.store().lock().await.merge_run_facts(&rig.root, &[kept("cargo test", "tests", 100)]).expect("learnt");

    bucket.set_read_only(true);
    rig.fleet.run_memory_written(&rig.root, None).await;
    let waiting = read(&rig).await;
    let run = &waiting.workspaces[0].runs[0];
    assert_eq!(run.state, RunFactState::Waiting);
    assert!(run.why.as_deref().is_some_and(|why| why.contains("refused")), "{:?}", run.why);
    let before = String::from_utf8(bucket.object(&places::object_for(places::FOLDER, &rig.root)).expect("the choice wrote an empty set")).expect("text");
    assert!(!before.contains("cargo test"), "{before}");

    bucket.set_read_only(false);
    rig.fleet.run_memory_written(&rig.root, None).await;
    let after = String::from_utf8(bucket.object(&places::object_for(places::FOLDER, &rig.root)).expect("an object")).expect("text");
    assert!(after.contains("cargo test"), "{after}");
    assert_eq!(read(&rig).await.workspaces[0].runs[0].state, RunFactState::Kept);
}

#[tokio::test]
async fn a_bucket_that_is_down_unsigned_or_not_an_address_refuses_the_place_and_the_old_one_stays() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    bucket.set_unreachable(true);
    let down = rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(BUCKET.into()))).await.expect_err("down");
    assert_eq!(refusal_code(&down), crate::run_memory::CLOUD_UNREACHABLE);

    let signed_out = rig_placing(None);
    let unsigned = signed_out.fleet.set_run_memory_place(cloud(&signed_out.root, RunCloud::Bucket(BUCKET.into()))).await.expect_err("unsigned");
    assert_eq!(refusal_code(&unsigned), crate::run_memory::CLOUD_UNSIGNED);

    for bad in ["", "shop-runs", "https://shop-runs/x", "s3://ab/x", "gs://shop-runs/../x"] {
        let refused = rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(bad.into()))).await.expect_err(bad);
        assert_eq!(refusal_code(&refused), crate::run_memory::PLACE_REFUSED, "{bad:?}");
    }
    let nowhere = RunMemoryPlace { repository: rig.root.clone(), place: RunPlace::Cloud, synced_folder: None, cloud: None };
    let refused = rig.fleet.set_run_memory_place(nowhere).await.expect_err("no where");
    assert_eq!(refusal_code(&refused), crate::run_memory::PLACE_REFUSED);

    assert_eq!(rig.fleet.get_run_memory_place(rig.root.clone()).await.expect("read").place, RunPlace::ThisMac);
    assert!(bucket.names().is_empty());
}

#[tokio::test]
async fn reading_what_is_remembered_does_not_call_the_cloud() {
    let bucket = Arc::new(FakeBucket::new());
    let rig = bucketed(&bucket);
    rig.fleet.set_run_memory_place(cloud(&rig.root, RunCloud::Bucket(BUCKET.into()))).await.expect("chosen");
    let calls = bucket.calls().len();

    read(&rig).await;
    read(&rig).await;

    assert_eq!(bucket.calls().len(), calls);
}
