//! How a native walk's build is made and kept: the tree, the seed, the queue and what stays.
//! `crates/fleet/src/walk_builds.rs`. What a build does to a walk's row is `native_walk.rs`'s.

use std::path::{Path, PathBuf};

use crate::walk_builds::{head_of, pruned_for_test, tree_at_for_test, Ask, Place, WalkBuilds};

fn git(at: &Path, args: &[&str]) -> String {
    let done = std::process::Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null"])
        .args(args)
        .current_dir(at)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8_lossy(&done.stdout).trim().to_string()
}

fn repository(at: &Path) -> String {
    std::fs::create_dir_all(at).expect("a folder");
    git(at, &["init", "--quiet"]);
    std::fs::write(at.join("one.txt"), "one").expect("a file");
    git(at, &["add", "."]);
    git(at, &["commit", "--quiet", "-m", "one"]);
    git(at, &["rev-parse", "HEAD"])
}

fn ask(session: &str, url: &str, commit: &str) -> Ask {
    let place = Place { key: "armada".into(), slot: PathBuf::from("/nowhere"), preview: PathBuf::from("/nowhere") };
    Ask { session: session.into(), url: url.into(), commit: commit.into(), place, answer: None }
}

#[test]
fn a_new_tree_is_a_worktree_at_the_commit_with_its_target_cloned_from_the_preview_and_moves_to_the_next_commit() {
    let scratch = crate::tests::tmp::TempDir::new();
    let slot = scratch.path().join("slot");
    let first = repository(&slot);
    let preview = scratch.path().join("preview");
    std::fs::create_dir_all(preview.join("crates/bridge-native/target/release")).expect("a target");
    std::fs::write(preview.join("crates/bridge-native/target/release/seed"), "built").expect("a seed");
    let place = Place { key: "armada".into(), slot: slot.clone(), preview };
    let root = scratch.path().join("walk-builds/armada");

    let tree = tree_at_for_test(&root, &place, &first).expect("made");

    assert_eq!(head_of(&tree).expect("a head"), first);
    assert_eq!(std::fs::read_to_string(tree.join("crates/bridge-native/target/release/seed")).ok().as_deref(), Some("built"), "seeded");
    assert!(git(&slot, &["worktree", "list"]).contains("walk-builds/armada/tree"), "a worktree of the Job's repository");

    std::fs::write(slot.join("two.txt"), "two").expect("a file");
    git(&slot, &["add", "."]);
    git(&slot, &["commit", "--quiet", "-m", "two"]);
    let second = git(&slot, &["rev-parse", "HEAD"]);
    let again = tree_at_for_test(&root, &place, &second).expect("moved");

    assert_eq!((again, head_of(&tree).expect("a head")), (tree.clone(), second));
    assert!(tree.join("crates/bridge-native/target/release/seed").is_file(), "the warm target stays");
}

#[test]
fn a_repository_with_no_preview_to_seed_from_builds_from_a_cold_target() {
    let scratch = crate::tests::tmp::TempDir::new();
    let slot = scratch.path().join("slot");
    let first = repository(&slot);
    let place = Place { key: "armada".into(), slot, preview: scratch.path().join("no-preview") };

    let tree = tree_at_for_test(&scratch.path().join("walk-builds/armada"), &place, &first).expect("made");

    assert!(!tree.join("crates/bridge-native/target").exists());
}

#[test]
fn the_newest_three_builds_of_a_repository_stay_and_the_one_just_made_is_never_pruned() {
    let scratch = crate::tests::tmp::TempDir::new();
    for (n, name) in ["a", "b", "c", "d", "e"].into_iter().enumerate() {
        let dir = scratch.path().join(name);
        std::fs::create_dir_all(&dir).expect("a build");
        std::fs::File::create(dir.join("bridge-native")).expect("a binary");
        let at = std::time::SystemTime::now() - std::time::Duration::from_secs(100 - n as u64);
        std::fs::File::open(&dir).expect("opens").set_modified(at).expect("aged");
    }
    // `a` is the oldest, and is the one just made.
    pruned_for_test(scratch.path(), "a");

    let mut kept: Vec<String> = std::fs::read_dir(scratch.path()).unwrap().map(|one| one.unwrap().file_name().to_string_lossy().into_owned()).collect();
    kept.sort();
    assert_eq!(kept, ["a", "d", "e"]);
}

#[test]
fn a_newer_commit_of_a_job_and_address_replaces_the_one_waiting_and_builds_go_one_at_a_time() {
    let queue = WalkBuilds::default();
    queue.asked(ask("job-1", "armada://walk/a", "c1"));
    queue.asked(ask("job-1", "armada://walk/b", "c1"));
    queue.asked(ask("job-1", "armada://walk/a", "c2"));

    let first = queue.next_for_test().expect("one is next");
    assert_eq!((first.url.as_str(), first.commit.as_str()), ("armada://walk/b", "c1"), "the replaced ask went, the newer took its place at the back");
    assert!(queue.next_for_test().is_none(), "one at a time: nothing starts while one runs");
    queue.done_for_test();
    let second = queue.next_for_test().expect("then the next");
    assert_eq!((second.url.as_str(), second.commit.as_str()), ("armada://walk/a", "c2"));
}

#[test]
fn an_ask_for_the_commit_that_is_running_is_not_queued_again() {
    let queue = WalkBuilds::default();
    queue.asked(ask("job-1", "armada://walk/a", "c1"));
    queue.next_for_test().expect("running");

    queue.asked(ask("job-1", "armada://walk/a", "c1"));
    queue.done_for_test();

    assert!(queue.next_for_test().is_none());
}
