//! The claim of #1059's refusal: a branch that appends a migration or moves
//! the protocol minor with no need on record is refused with what to run; one
//! with the need recorded passes; one touching neither passes untouched; and a
//! major move is outside needs.

use super::repo::TempRepo;
use crate::needs::Needs;
use crate::undeclared::undeclared;

const MIGRATIONS: &str = "crates/store/src/migrations.rs";
const PROTOCOL: &str = "protocol-version.toml";

fn list(entries: &[&str]) -> String {
    let mut out = String::from("pub const MIGRATIONS: &[&str] = &[\n");
    for entry in entries {
        out.push_str(&format!("    {entry},\n"));
    }
    out.push_str("];\n");
    out
}

fn toml(major: u32, minor: u32) -> String {
    format!("# minor = 99 in a comment\nmajor = {major}\nminor = {minor}\n")
}

/// `main` holds V1, V2 and 23.36; `work` is cut from it.
fn repo() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.write(MIGRATIONS, &list(&["V1", "// a note", "crate::a::V2"]));
    repo.write(PROTOCOL, &toml(23, 36));
    repo.commit_everything("base");
    repo.git(&["branch", "work"]);
    repo.git(&["checkout", "-q", "work"]);
    repo
}

fn answer(repo: &TempRepo, needs: &Needs) -> Option<String> {
    undeclared(repo.root(), "main", "work", needs).expect("git read")
}

fn needs_of(repo: &TempRepo) -> Needs {
    Needs::of(repo.root()).expect("needs")
}

#[test]
fn an_appended_migration_with_no_need_is_refused_naming_what_to_run() {
    let repo = repo();
    repo.write(MIGRATIONS, &list(&["V1", "crate::a::V2", "crate::b::V3"]));
    repo.commit_everything("add");
    let said = answer(&repo, &needs_of(&repo)).expect("refused");
    assert!(said.contains(&format!("armada need {MIGRATIONS} \"a new migration\"")));
    assert!(
        said.contains(MIGRATIONS) && said.contains(PROTOCOL),
        "{said}"
    );
    assert!(said.contains("keeps its place"), "{said}");
}

#[test]
fn a_changed_minor_with_no_need_is_refused() {
    let repo = repo();
    repo.write(PROTOCOL, &toml(23, 38));
    repo.commit_everything("bump");
    let said = answer(&repo, &needs_of(&repo)).expect("refused");
    assert!(said.contains(&format!("armada need {PROTOCOL} \"a minor\"")));
    assert!(
        !said.contains(&format!("armada need {MIGRATIONS}")),
        "{said}"
    );
}

#[test]
fn a_declared_need_passes() {
    let repo = repo();
    repo.write(MIGRATIONS, &list(&["V1", "crate::a::V2", "crate::b::V3"]));
    repo.write(PROTOCOL, &toml(23, 37));
    repo.commit_everything("both");
    let needs = needs_of(&repo);
    needs
        .declare("work", MIGRATIONS, "a new migration")
        .expect("declared");
    assert!(answer(&repo, &needs)
        .expect("the minor is not")
        .contains(PROTOCOL));
    needs
        .declare("work", PROTOCOL, "a minor")
        .expect("declared");
    assert_eq!(answer(&repo, &needs), None);
}

#[test]
fn a_branch_touching_neither_passes_and_so_does_a_comment_or_reorder() {
    let repo = repo();
    repo.write(MIGRATIONS, &list(&["crate::a::V2", "V1", "// edited note"]));
    repo.write("other.rs", "x");
    repo.commit_everything("unrelated");
    assert_eq!(answer(&repo, &needs_of(&repo)), None);
}

#[test]
fn a_major_move_is_outside_needs() {
    let repo = repo();
    repo.write(PROTOCOL, &toml(24, 0));
    repo.commit_everything("major");
    assert_eq!(answer(&repo, &needs_of(&repo)), None);
}

#[test]
fn a_base_that_moved_on_is_not_the_branchs_change() {
    let repo = repo();
    repo.write("other.rs", "x");
    repo.commit_everything("branch work");
    repo.git(&["checkout", "-q", "main"]);
    repo.write(PROTOCOL, &toml(23, 40));
    repo.commit_everything("main bumps");
    assert_eq!(answer(&repo, &needs_of(&repo)), None);
}
