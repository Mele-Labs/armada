//! What jq prints for a pull request's files, read as every file's patch and as one file's.
//! The forge is never called: what is under test is which record is the file and what its
//! absence means.

use adapter_traits::{FilePatch, PatchedFile};

use crate::pull_request_patch::{folded, listed};

fn lines(records: &[&str]) -> Vec<String> {
    records.iter().map(|it| it.to_string()).collect()
}

#[test]
fn a_patch_keeps_its_newlines() {
    let said = lines(&[
        "a.rs\t+\t@@ -1 +1 @@\\n-old\\n+new",
        "b.rs\t+\t@@ -2 +2 @@\\n-x\\n+y",
    ]);
    assert_eq!(
        folded(listed(&said), "b.rs"),
        FilePatch::Patch("@@ -2 +2 @@\n-x\n+y".into())
    );
}

#[test]
fn a_file_with_no_patch_is_not_empty() {
    let said = lines(&["logo.png\t-\t", "empty.txt\t+\t"]);
    assert_eq!(folded(listed(&said), "logo.png"), FilePatch::NoPatch);
    assert_eq!(folded(listed(&said), "empty.txt"), FilePatch::Patch(String::new()));
}

#[test]
fn an_unlisted_path_is_no_such_file() {
    assert_eq!(
        folded(listed(&lines(&["a.rs\t+\t@@ -1 +1 @@"])), "z.rs"),
        FilePatch::NoSuchFile
    );
}

#[test]
fn a_tab_in_a_patch_stays_one_field() {
    let said = lines(&["Makefile\t+\t@@ -1 +1 @@\\n+\\tcc"]);
    assert_eq!(
        folded(listed(&said), "Makefile"),
        FilePatch::Patch("@@ -1 +1 @@\n+\tcc".into())
    );
}

#[test]
fn every_file_is_listed_in_the_forges_order() {
    let said = lines(&[
        "z.rs\t+\t@@ -1 +1 @@\\n-a\\n+b",
        "logo.png\t-\t",
        "not a record",
        "a.rs\t+\t",
    ]);
    assert_eq!(
        listed(&said),
        vec![
            PatchedFile { path: "z.rs".into(), patch: Some("@@ -1 +1 @@\n-a\n+b".into()) },
            PatchedFile { path: "logo.png".into(), patch: None },
            PatchedFile { path: "a.rs".into(), patch: Some(String::new()) },
        ]
    );
}
