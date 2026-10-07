//! A step's entry baseline is kept per Job and step, read back as it was
//! written, and dropped when the step advances.

use adapter_traits::Footprint;

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{job_id, open, TempDir};

fn held() -> Footprint {
    Footprint::of(vec![
        ("src/b.rs".to_string(), "bbbb".to_string()),
        ("src/a.rs".to_string(), "aaaa".to_string()),
    ])
}

#[test]
fn a_baseline_reads_back_as_it_was_written() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01BASE");
    let job = job_id("01BASE");

    assert_eq!(store.step_baseline(&job, &step_id()).unwrap(), None);
    store.keep_step_baseline(&job, &step_id(), &held()).unwrap();
    assert_eq!(store.step_baseline(&job, &step_id()).unwrap(), Some(held()));
}

#[test]
fn a_worktree_that_held_nothing_is_an_answer_and_not_an_absence() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01BASE");
    let job = job_id("01BASE");

    store
        .keep_step_baseline(&job, &step_id(), &Footprint::nothing())
        .unwrap();
    assert_eq!(
        store.step_baseline(&job, &step_id()).unwrap(),
        Some(Footprint::nothing())
    );
}

#[test]
fn a_forgotten_baseline_is_gone_and_forgetting_none_is_fine() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01BASE");
    let job = job_id("01BASE");

    store.forget_step_baseline(&job, &step_id()).unwrap();
    store.keep_step_baseline(&job, &step_id(), &held()).unwrap();
    store.forget_step_baseline(&job, &step_id()).unwrap();
    assert_eq!(store.step_baseline(&job, &step_id()).unwrap(), None);
}
