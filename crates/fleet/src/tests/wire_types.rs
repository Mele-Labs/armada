//! Which wire types a pull request's patches change, read from hunks alone.

use adapter_traits::PatchedFile;

use crate::wire_types::changed_types;

fn file(path: &str, patch: &str) -> PatchedFile {
    PatchedFile { path: path.into(), patch: Some(patch.into()), blob: None }
}

fn said(files: &[PatchedFile]) -> Vec<(String, String)> {
    changed_types(files).into_iter().map(|one| (one.name.clone(), one.said())).collect()
}

#[test]
fn a_field_added_and_one_removed_inside_a_struct_are_named() {
    let patch = "@@ -10,7 +10,7 @@ pub struct SessionRow {\n     pub id: String,\n-    pub old: u8,\n+    pub notice: Option<String>,\n     pub at: String,\n }\n";
    let found = said(&[file("crates/ipc/src/hosted_sessions.rs", patch)]);
    assert_eq!(found, vec![("SessionRow".to_string(), "adds notice; removes old".to_string())]);
}

#[test]
fn a_hunk_that_crosses_two_types_is_split_between_them() {
    let patch = "@@ -1,9 +1,9 @@ pub struct A {\n     pub x: u8,\n-    pub y: u8,\n+    pub z: u8,\n }\n \n pub enum B {\n-    One,\n+    Two,\n }\n";
    let found = said(&[file("crates/ipc/src/a.rs", patch)]);
    assert_eq!(
        found,
        vec![
            ("A".to_string(), "adds z; removes y".to_string()),
            ("B".to_string(), "adds Two; removes One".to_string()),
        ]
    );
}

#[test]
fn a_hunk_that_starts_after_a_types_closing_bracket_is_not_that_type() {
    let patch = "@@ -20,5 +20,5 @@ pub struct A {\n }\n \n-fn old() {}\n+fn new() {}\n";
    assert!(said(&[file("crates/ipc/src/a.rs", patch)]).is_empty());
}

#[test]
fn a_new_type_a_removed_type_and_a_changed_alias_are_each_named() {
    let patch = "@@ -1,3 +1,6 @@\n+pub struct Fresh {\n+    pub a: u8,\n+}\n-pub struct Gone {\n-}\n-pub type Id = u8;\n+pub type Id = u16;\n";
    let found = said(&[file("crates/ipc/src/a.rs", patch)]);
    assert_eq!(
        found,
        vec![
            ("Fresh".to_string(), "added".to_string()),
            ("Gone".to_string(), "removed".to_string()),
            ("Id".to_string(), "changed".to_string()),
        ]
    );
}

#[test]
fn comments_tests_and_files_outside_the_wire_change_nothing() {
    let doc = "@@ -1,3 +1,3 @@ pub struct A {\n     pub x: u8,\n-    /// old words\n+    /// new words\n }\n";
    let patch = "@@ -1,3 +1,3 @@ pub struct A {\n     pub x: u8,\n-    pub y: u8,\n+    pub z: u8,\n }\n";
    assert!(said(&[file("crates/ipc/src/a.rs", doc)]).is_empty());
    assert!(said(&[file("crates/ipc/src/tests/a.rs", patch)]).is_empty());
    assert!(said(&[file("crates/fleet/src/a.rs", patch)]).is_empty());
    assert!(said(&[file("crates/ipc/operations/a.toml", patch)]).is_empty());
}

#[test]
fn a_changed_attribute_above_a_type_changes_it() {
    let patch = "@@ -5,4 +5,4 @@\n }\n \n-#[serde(rename_all = \"snake_case\")]\n+#[serde(rename_all = \"kebab-case\")]\n pub enum Kind {\n     A,\n }\n";
    let found = said(&[file("crates/ipc/src/a.rs", patch)]);
    assert_eq!(found, vec![("Kind".to_string(), "changed".to_string())]);
}
