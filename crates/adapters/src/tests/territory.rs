//! Where a repository's files are and what a slot has changed, against real git: the directories are
//! the base head's tree and the changes are the worktree's tree against its merge base.

use std::path::{Path, PathBuf};

use super::repo::TempRepo;
use crate::territory::{changed_since, directories_at, files_at, head_of, text_at, Directory, NotRead, Text};

fn dir(path: &str, files: u32, names: &[&str]) -> Directory {
    Directory { path: path.into(), files, names: names.iter().map(|name| name.to_string()).collect() }
}

/// A repository on `main`, and a worktree on `work` cut from it.
fn a_slot() -> (TempRepo, PathBuf) {
    let repo = TempRepo::with_a_commit();
    repo.commit_one("f.txt", "f\n", "add f");
    repo.commit_one("g.txt", "g\n", "add g");
    let slot = repo.root().join(".armada/slots/slot-1");
    repo.git(&["worktree", "add", "-q", &slot.to_string_lossy(), "-b", "work"]);
    (repo, slot)
}

/// The worktree's own git directory: where its index and any scratch copy of it would be.
fn git_dir(slot: &Path) -> PathBuf {
    let said = std::process::Command::new("git")
        .args(["-C", &slot.to_string_lossy(), "rev-parse", "--absolute-git-dir"])
        .output()
        .expect("git");
    PathBuf::from(String::from_utf8_lossy(&said.stdout).trim())
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .expect("a directory")
        .map(|one| one.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn the_directories_are_the_base_heads_tree_to_two_names_with_the_files_under_each() {
    let repo = TempRepo::empty();
    repo.write("README.md", "top\n");
    repo.write("crates/fleet/a.rs", "a\n");
    repo.write("crates/fleet/b.rs", "b\n");
    repo.write("crates/fleet/src/deep/c.rs", "c\n");
    repo.write("crates/ipc/d.rs", "d\n");
    repo.write("docs/e.md", "e\n");
    repo.commit_everything("the tree");
    // Not in the base head's tree, so not in the count.
    repo.write("crates/fleet/untracked.rs", "x\n");

    let head = head_of(repo.root(), "main").expect("main has a head");

    assert_eq!(head, repo.head_str());
    assert_eq!(
        directories_at(repo.root(), &head, 10),
        Ok(vec![
            dir("crates", 4, &["fleet/a.rs", "fleet/b.rs", "fleet/src/deep/c.rs", "ipc/d.rs"]),
            dir("crates/fleet", 3, &["a.rs", "b.rs", "src/deep/c.rs"]),
            dir("crates/ipc", 1, &["d.rs"]),
            dir("docs", 1, &["e.md"]),
        ]),
        "a file at the top is in no directory; a deeper one counts to the two names above it"
    );
}

#[test]
fn a_directory_names_its_first_files_in_gits_order_and_counts_them_all() {
    let repo = TempRepo::empty();
    // Git lists `a.rs` before `a/b.rs` (a dot sorts before a slash) and `a-z` before both.
    for file in ["d/b.rs", "d/a/b.rs", "d/a.rs", "d/a-z.rs", "d/c.rs"] {
        repo.write(file, "x\n");
    }
    repo.commit_everything("the tree");
    let head = head_of(repo.root(), "main").expect("main has a head");

    let read = directories_at(repo.root(), &head, 3).expect("read");

    assert_eq!(read[0], dir("d", 5, &["a-z.rs", "a.rs", "a/b.rs"]), "the first three, the count of all five");
}

#[test]
fn a_base_that_names_nothing_is_not_read() {
    let repo = TempRepo::with_a_commit();
    assert!(matches!(head_of(repo.root(), "nowhere"), Err(NotRead(_))));
}

#[test]
fn a_slots_changes_are_what_it_committed_edited_deleted_and_has_not_tracked_since_it_met_the_base() {
    let (repo, slot) = a_slot();
    // Main moves on after the slot was cut: what it changed is not the slot's.
    repo.commit_one("h.txt", "h\n", "add h on main");
    std::fs::create_dir_all(slot.join("src")).expect("a directory");
    std::fs::write(slot.join("src/a.rs"), "a\n").expect("written");
    repo.git(&["-C", &slot.to_string_lossy(), "add", "src/a.rs"]);
    repo.git(&["-C", &slot.to_string_lossy(), "commit", "-q", "-m", "add a"]);
    std::fs::write(slot.join("f.txt"), "edited\n").expect("written");
    std::fs::remove_file(slot.join("g.txt")).expect("removed");
    std::fs::write(slot.join("new.txt"), "new\n").expect("written");

    assert_eq!(
        changed_since(&slot, "main"),
        Ok(vec!["f.txt".into(), "g.txt".into(), "new.txt".into(), "src/a.rs".into()])
    );
}

#[test]
fn a_slot_that_changed_nothing_has_no_paths() {
    let (repo, slot) = a_slot();
    repo.commit_one("h.txt", "h\n", "add h on main");
    assert_eq!(changed_since(&slot, "main"), Ok(Vec::new()));
}

#[test]
fn reading_a_slots_changes_writes_no_index_of_it() {
    let (_repo, slot) = a_slot();
    std::fs::write(slot.join("f.txt"), "edited\n").expect("written");
    std::fs::write(slot.join("new.txt"), "new\n").expect("written");
    let gitdir = git_dir(&slot);
    let (index, held) = (std::fs::read(gitdir.join("index")).expect("an index"), names(&gitdir));

    let _ = changed_since(&slot, "main");

    assert_eq!(std::fs::read(gitdir.join("index")).expect("an index"), index, "its index is as it was");
    assert_eq!(names(&gitdir), held, "and no scratch copy of it is left beside it");
}

#[test]
fn the_files_and_a_files_text_are_the_trees_whatever_the_checkout_holds() {
    let repo = TempRepo::empty();
    repo.write("src/a.rs", "fn a() {}\n");
    repo.write("src/long.rs", &format!("{}é", "x".repeat(9)));
    repo.write("logo.png", "PNG\0\0");
    repo.commit_everything("the tree");
    let head = head_of(repo.root(), "main").expect("a head");
    // Edited after the commit, and a file the tree does not hold.
    repo.write("src/a.rs", "fn changed() {}\n");
    repo.write("untracked.rs", "x\n");

    assert_eq!(files_at(repo.root(), &head), Ok(vec!["logo.png".into(), "src/a.rs".into(), "src/long.rs".into()]));
    assert_eq!(
        text_at(repo.root(), &head, "src/a.rs", 100),
        Ok(Some(Text { text: "fn a() {}\n".into(), cut: false }))
    );
    assert_eq!(text_at(repo.root(), &head, "logo.png", 100), Ok(None), "a NUL byte is not text");
    assert!(text_at(repo.root(), &head, "untracked.rs", 100).is_err(), "not in the tree is not read");
    // 'é' is two bytes and starts at byte 9: a cut at 10 lands inside it.
    assert_eq!(
        text_at(repo.root(), &head, "src/long.rs", 10),
        Ok(Some(Text { text: "xxxxxxxxx".into(), cut: true }))
    );
}
