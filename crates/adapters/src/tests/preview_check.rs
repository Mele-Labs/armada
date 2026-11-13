//! The check an offer for the preview gets, against real git and a compiler planted in its place: what
//! each change needs compiled, how a failure is read, and how a set of tips is merged and tried.

use std::path::Path;
use std::sync::Mutex;

use super::repo::TempRepo;
use crate::preview_check::{check, compiles_for, files_named, first_error, Broke, Compile, Compiler, Verdict, CONFLICTS};

fn changed(files: &[&str]) -> Vec<Compile> {
    compiles_for(&files.iter().map(|file| file.to_string()).collect::<Vec<_>>())
}

#[test]
fn a_change_under_crates_or_xtask_or_a_cargo_file_compiles_the_workspace() {
    assert_eq!(changed(&["crates/fleet/src/a.rs"]), [Compile::Workspace]);
    assert_eq!(changed(&["xtask/src/main.rs"]), [Compile::Workspace]);
    assert_eq!(changed(&["Cargo.lock"]), [Compile::Workspace]);
}

#[test]
fn a_change_to_the_native_bridge_the_wire_or_the_tokens_compiles_the_native_bridge() {
    assert_eq!(changed(&["crates/bridge-native/src/a.rs"]), [Compile::Native]);
    assert_eq!(changed(&["crates/ipc/src/a.rs"]), [Compile::Workspace, Compile::Native]);
    assert_eq!(changed(&["packages/tokens/src/a.ts"]), [Compile::Native, Compile::Typescript]);
}

#[test]
fn a_change_under_apps_or_packages_runs_the_typecheck_and_docs_alone_compile_nothing() {
    assert_eq!(changed(&["apps/desktop/src/a.ts", "packages/shell/src/b.ts"]), [Compile::Typescript]);
    assert!(changed(&["docs/concepts/fleet.md", "scripts/preview"]).is_empty());
}

#[test]
fn the_first_error_line_and_the_files_it_names_are_read_off_a_compilers_output() {
    let rust = "   Compiling a v0.0.0\nwarning: unused\n  --> crates/a/src/x.rs:1:1\nerror[E0308]: mismatched types\n  --> crates/a/src/y.rs:12:5\n   |\nerror: could not compile `a` (lib) due to 1 previous error\n";
    assert_eq!(first_error(rust).as_deref(), Some("error[E0308]: mismatched types"));
    assert_eq!(files_named(rust), ["crates/a/src/x.rs", "crates/a/src/y.rs"]);

    let tsc = "packages/shell/src/layout.tsx(12,3): error TS2322: Type 'string' is not assignable to type 'number'.\n";
    assert_eq!(first_error(tsc).as_deref(), Some("packages/shell/src/layout.tsx(12,3): error TS2322: Type 'string' is not assignable to type 'number'."));
    assert_eq!(files_named(tsc), ["packages/shell/src/layout.tsx"]);
}

/// A compiler that fails wherever the tree holds `crates/x/broken.rs`, and writes down what it was asked.
#[derive(Default)]
struct Planted {
    asked: Mutex<Vec<Compile>>,
}

impl Compiler for Planted {
    fn compile(&self, tree: &Path, what: Compile) -> Result<(), Broke> {
        self.asked.lock().expect("not poisoned").push(what);
        if tree.join("crates/x/broken.rs").exists() {
            return Err(Broke { why: "error[E0308]: mismatched types".into(), files: vec!["crates/x/broken.rs".into()] });
        }
        Ok(())
    }
}

/// A branch off `base` with one file committed, and the tip it answers.
fn branch_with(repo: &TempRepo, base: &str, branch: &str, file: &str, contents: &str) -> String {
    repo.git(&["checkout", "-q", "-B", branch, base]);
    repo.commit_one(file, contents, branch);
    repo.git(&["rev-parse", "HEAD"])
}

#[test]
fn every_tip_that_merges_and_compiles_together_is_ready_after_one_compile() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    let one = branch_with(&repo, &base, "armada/one", "crates/x/one.rs", "1");
    let two = branch_with(&repo, &base, "armada/two", "crates/x/two.rs", "2");
    let compiler = Planted::default();

    let verdicts = check(repo.root(), &base, &[one, two], &compiler).expect("checked");

    assert_eq!(verdicts, [Verdict::Ready, Verdict::Ready]);
    assert_eq!(*compiler.asked.lock().expect("not poisoned"), [Compile::Workspace], "once, for both");
}

#[test]
fn a_set_that_does_not_compile_is_tried_one_at_a_time_and_only_the_breaking_tip_fails() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    let good = branch_with(&repo, &base, "armada/good", "crates/x/good.rs", "1");
    let bad = branch_with(&repo, &base, "armada/bad", "crates/x/broken.rs", "2");
    let also_good = branch_with(&repo, &base, "armada/also", "crates/x/also.rs", "3");
    let compiler = Planted::default();

    let verdicts = check(repo.root(), &base, &[good, bad, also_good], &compiler).expect("checked");

    assert_eq!(
        verdicts,
        [
            Verdict::Ready,
            Verdict::Failed(Broke { why: "error[E0308]: mismatched types".into(), files: vec!["crates/x/broken.rs".into()] }),
            Verdict::Ready,
        ]
    );
    assert_eq!(compiler.asked.lock().expect("not poisoned").len(), 4, "all three, then each of them on its own");
}

#[test]
fn a_tip_that_conflicts_with_an_earlier_one_fails_naming_the_files_and_the_rest_go_on() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    let first = branch_with(&repo, &base, "armada/first", "crates/x/shared.rs", "one");
    let second = branch_with(&repo, &base, "armada/second", "crates/x/shared.rs", "two");
    let third = branch_with(&repo, &base, "armada/third", "crates/x/third.rs", "3");
    let compiler = Planted::default();

    let verdicts = check(repo.root(), &base, &[first, second, third], &compiler).expect("checked");

    assert_eq!(verdicts[0], Verdict::Ready);
    assert_eq!(verdicts[1], Verdict::Failed(Broke { why: format!("{CONFLICTS} crates/x/shared.rs"), files: vec!["crates/x/shared.rs".into()] }));
    assert_eq!(verdicts[2], Verdict::Ready);
}

#[test]
fn the_kept_worktree_is_made_once_and_the_next_check_reuses_it() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    let one = branch_with(&repo, &base, "armada/one", "crates/x/one.rs", "1");
    let compiler = Planted::default();

    check(repo.root(), &base, &[one.clone()], &compiler).expect("checked");
    check(repo.root(), &base, &[one], &compiler).expect("checked again");

    let listed = repo.git(&["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("preview-check").count(), 1, "{listed}");
}
