//! What a Job froze when it started: kept for the Job alone, replaced whole, and across a reopen.

use crate::tests::{open, TempDir};
use crate::KeptValue;

fn kept(key: &str, value: Option<&str>, layer: &str) -> KeptValue {
    KeptValue { key: key.into(), value: value.map(str::to_string), layer: layer.into() }
}

#[test]
fn a_job_s_values_are_kept_for_that_job_alone_replaced_whole_and_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.job_values("s1").expect("reads").is_empty(), "a Job that froze nothing has none");
    let first = [kept("pr_mode", Some("draft"), "project"), kept("review_model", None, "default"), kept("autonomy.forge_write", Some("asks"), "kit")];
    store.keep_job_values("s1", &first).expect("kept");
    store.keep_job_values("s2", &[kept("pr_mode", Some("ready"), "default")]).expect("kept");
    drop(store);

    let mut store = open(&dir);
    let mut expected = first.to_vec();
    expected.sort_by(|a, b| a.key.cmp(&b.key));
    assert_eq!(store.job_values("s1").expect("reads"), expected, "a key no layer set is kept as that");
    assert_eq!(store.job_values("s2").expect("reads"), [kept("pr_mode", Some("ready"), "default")]);
    store.keep_job_values("s1", &[kept("auto_merge", Some("always"), "repository")]).expect("kept again");
    assert_eq!(store.job_values("s1").expect("reads"), [kept("auto_merge", Some("always"), "repository")], "what was frozen is replaced, not added to");
    assert_eq!(store.job_values("s2").expect("reads").len(), 1);
}
