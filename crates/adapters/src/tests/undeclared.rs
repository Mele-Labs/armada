//! The claim of #1059's refusal: a branch that moves the protocol minor with no
//! need on record is refused with what to run; one with the need recorded
//! passes; one touching nothing watched passes untouched; a major move is
//! outside needs; and a migration is not watched, since names do not collide.

use super::repo::TempRepo;
use crate::undeclared::undeclared;

const MIGRATIONS: &str = "crates/store/src/migration_list.rs";
const PROTOCOL: &str = "protocol-version.toml";

fn toml(major: u32, minor: u32) -> String {
    format!("# minor = 99 in a comment\nmajor = {major}\nminor = {minor}\n")
}

/// `main` holds 23.36; `work` is cut from it.
fn repo() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.write(PROTOCOL, &toml(23, 36));
    repo.commit_everything("base");
    repo.git(&["branch", "work"]);
    repo.git(&["checkout", "-q", "work"]);
    repo
}

/// What the caller says the branch has declared: Fleet reads its ledger for it
/// and `armada land` the files, and this function is told either way.
fn answer(repo: &TempRepo, declared: &[&str]) -> Option<String> {
    let declared: Vec<String> = declared.iter().map(|path| path.to_string()).collect();
    undeclared(repo.root(), "main", "work", &declared).expect("git read")
}

#[test]
fn an_appended_migration_needs_nothing() {
    let repo = repo();
    repo.write(MIGRATIONS, "Migration::additive(\"a.b\", crate::a::B),\n");
    repo.commit_everything("add");
    assert_eq!(answer(&repo, &[]), None);
}

#[test]
fn a_changed_minor_with_no_need_is_refused() {
    let repo = repo();
    repo.write(PROTOCOL, &toml(23, 38));
    repo.commit_everything("bump");
    let said = answer(&repo, &[]).expect("refused");
    assert!(said.contains(&format!("armada need {PROTOCOL} \"a minor\"")));
    assert!(!said.contains("a new migration"), "{said}");
}

#[test]
fn a_declared_need_passes() {
    let repo = repo();
    repo.write(PROTOCOL, &toml(23, 37));
    repo.commit_everything("bump");
    assert!(answer(&repo, &[]).is_some());
    assert_eq!(answer(&repo, &[PROTOCOL]), None);
    assert_eq!(
        answer(&repo, &[&format!("./{PROTOCOL}")]),
        None,
        "a path is read without its leading ./"
    );
}

#[test]
fn a_branch_touching_nothing_watched_passes() {
    let repo = repo();
    repo.write("other.rs", "x");
    repo.commit_everything("unrelated");
    assert_eq!(answer(&repo, &[]), None);
}

#[test]
fn a_major_move_is_outside_needs() {
    let repo = repo();
    repo.write(PROTOCOL, &toml(24, 0));
    repo.commit_everything("major");
    assert_eq!(answer(&repo, &[]), None);
}

#[test]
fn a_base_that_moved_on_is_not_the_branchs_change() {
    let repo = repo();
    repo.write("other.rs", "x");
    repo.commit_everything("branch work");
    repo.git(&["checkout", "-q", "main"]);
    repo.write(PROTOCOL, &toml(23, 40));
    repo.commit_everything("main bumps");
    assert_eq!(answer(&repo, &[]), None);
}
