//! The paths a person's commits changed, read from a real local history and nothing else.

use adapter_traits::{SlotPool, Vcs};

use crate::tests::repo::TempRepo;
use crate::GitVcs;

fn commit_as(repo: &TempRepo, author: &str, path: &str) {
    // Each commit its own contents, so a path committed twice is a change both times.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    repo.write(path, &format!("{n}\n"));
    repo.git(&["add", "--", path]);
    repo.git(&["commit", "-q", &format!("--author={author}"), "-m", path]);
}

fn pool(repo: &TempRepo) -> SlotPool {
    SlotPool::of(&repo.root().to_string_lossy(), 1, "main", Vec::new())
}

#[test]
fn the_paths_come_from_the_named_authors_and_the_repositorys_own_identity_newest_first_each_once() {
    let repo = TempRepo::with_a_commit();
    commit_as(&repo, "Ada Lovelace <ada@x.invalid>", "crates/fleet/a.rs");
    commit_as(&repo, "Bob <bob@x.invalid>", "apps/b.ts");
    commit_as(&repo, "Ada Lovelace <ada@x.invalid>", "docs/c.md");
    commit_as(&repo, "Ada Lovelace <ada@x.invalid>", "crates/fleet/a.rs");
    commit_as(&repo, "armada <armada@example.invalid>", "own.txt");

    let found = GitVcs::new().paths_touched_by(&pool(&repo), &["ada@x.invalid".to_string()], 300);
    assert_eq!(found, ["own.txt", "crates/fleet/a.rs", "docs/c.md"], "the configured identity too, newest first, once");

    let none = GitVcs::new().paths_touched_by(&pool(&repo), &["nobody".to_string()], 300);
    assert_eq!(none, ["own.txt"], "an author git has no commits for adds nothing");

    let capped = GitVcs::new().paths_touched_by(&pool(&repo), &["ada@x.invalid".to_string()], 1);
    assert_eq!(capped, ["own.txt"], "the last commits and no more");
}

#[test]
fn a_name_is_a_name_and_not_a_pattern() {
    let repo = TempRepo::with_a_commit();
    commit_as(&repo, "Ada <ada@x.invalid>", "a.rs");
    assert!(
        GitVcs::new().paths_touched_by(&pool(&repo), &[".*".to_string()], 300).iter().all(|path| path != "a.rs"),
        "a wildcard is not every author"
    );
}
