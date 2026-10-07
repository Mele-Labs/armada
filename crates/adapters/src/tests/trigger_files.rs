//! Trigger files are read from the base, not from what is on disk.

use crate::tests::repo::TempRepo;
use crate::triggers_on_base;

#[test]
fn the_files_are_the_bases_and_not_the_working_trees() {
    let repo = TempRepo::empty();
    let dir = repo.root().join(".armada/triggers");
    std::fs::create_dir_all(&dir).expect("a folder");
    std::fs::write(dir.join("tidy.yml"), "name: tidy\n").expect("a file");
    std::fs::write(dir.join("notes.txt"), "not a Trigger\n").expect("a file");
    repo.commit_everything("the first commit");
    std::fs::write(dir.join("tidy.yml"), "name: edited\n").expect("an edit");
    std::fs::write(dir.join("new.yml"), "name: new\n").expect("an uncommitted file");

    let found = triggers_on_base(repo.root(), None);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "name: tidy\n");
    assert!(found[0].0.ends_with("tidy.yml"));
}

#[test]
fn a_repository_with_no_folder_has_none() {
    let repo = TempRepo::with_a_commit();
    assert!(triggers_on_base(repo.root(), Some("HEAD")).is_empty());
}
