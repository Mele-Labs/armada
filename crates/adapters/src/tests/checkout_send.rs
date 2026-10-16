//! A served checkout's uncommitted files, committed from objects and pushed,
//! against real git and a bare remote. The forge half, opening the pull
//! request, needs a forge and is not run here: the commit and the push are the
//! halves that could touch the owner's files.

use std::path::{Path, PathBuf};

use adapter_traits::CheckoutToSend;

use crate::checkout_send::committed;
use crate::manifest_edit::pushed;
use crate::tests::repo::TempRepo;

fn git_in(dir: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=a@b.invalid", "-C"])
        .arg(dir)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&run.stderr));
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// A checkout and its bare remote, with `armada.yml`, `old.rs` and `run.sh` pushed on `main`,
/// and a second clone standing in for the forge's merges.
fn a_checkout_with_a_remote() -> (TempRepo, PathBuf, PathBuf) {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.commit_one("armada.yml", "version: 1\n", "a manifest");
    repo.commit_one("old.rs", "fn old() {}\n", "an old file");
    repo.commit_one("run.sh", "#!/bin/sh\n", "a script");
    repo.git(&["push", "--quiet", "--set-upstream", "origin", "main"]);
    let elsewhere = repo.root().with_extension("elsewhere");
    git_in(repo.root(), &["clone", "--quiet", &bare.to_string_lossy(), &elsewhere.to_string_lossy()]);
    (repo, bare, elsewhere)
}

/// Main moves on the forge: `path` committed with `text` and pushed, then fetched here as a reading of main does.
fn main_moves(repo: &TempRepo, elsewhere: &Path, path: &str, text: &str) {
    std::fs::write(elsewhere.join(path), text).expect("writes");
    git_in(elsewhere, &["add", "--", path]);
    git_in(elsewhere, &["commit", "--quiet", "-m", "main moved"]);
    git_in(elsewhere, &["push", "--quiet", "origin", "main"]);
    repo.git(&["fetch", "--quiet", "origin"]);
}

fn sending(paths: &[&str]) -> CheckoutToSend {
    CheckoutToSend {
        base: String::from("main"),
        paths: paths.iter().map(|path| path.to_string()).collect(),
        message: String::from("armada.yml"),
        branch: String::from("armada/checkout-1a2b3c4d"),
    }
}

/// What a person standing in the checkout would see, every way git can say it, and the files themselves.
fn standing(repo: &TempRepo) -> (String, String, String, String, Vec<String>) {
    let files = ["armada.yml", "old.rs", "run.sh", "src/staged.rs", "loose.rs"]
        .iter()
        .map(|file| std::fs::read_to_string(repo.root().join(file)).unwrap_or_else(|_| String::from("<gone>")))
        .collect();
    (
        repo.git(&["rev-parse", "HEAD"]),
        repo.git(&["status", "--porcelain=v1", "--untracked-files=all"]),
        repo.git(&["ls-files", "--stage"]),
        repo.git(&["branch", "--format=%(refname)"]),
        files,
    )
}

/// The whole of the owner's side: a changed file, a deleted one, work staged and untracked beside them.
#[test]
fn the_commit_sits_on_origins_tip_with_the_working_content_and_the_checkout_does_not_move() {
    let (repo, bare, elsewhere) = a_checkout_with_a_remote();
    main_moves(&repo, &elsewhere, "theirs.rs", "fn theirs() {}\n");
    repo.write("armada.yml", "version: 1\nauto_merge: checks-pass\n");
    std::fs::remove_file(repo.root().join("old.rs")).expect("deleted by hand");
    repo.write("src/staged.rs", "staged");
    repo.git(&["add", "--", "src/staged.rs"]);
    repo.write("loose.rs", "untracked");
    let before = standing(&repo);
    let tip = repo.git(&["rev-parse", "origin/main"]);

    let commit = committed(&repo.root_str(), &sending(&["armada.yml", "old.rs"])).expect("committed");

    assert_eq!(repo.git(&["rev-parse", &format!("{commit}^")]), tip, "the parent is origin's tip, not the checkout's HEAD");
    assert_ne!(tip, repo.head_str(), "the checkout is behind, so the two differ");
    assert_eq!(repo.git(&["show", &format!("{commit}:armada.yml")]), "version: 1\nauto_merge: checks-pass");
    assert!(repo.git(&["ls-tree", "-r", "--name-only", &commit]).lines().all(|name| name != "old.rs"), "the deletion is in it");
    assert_eq!(repo.git(&["show", &format!("{commit}:theirs.rs")]), "fn theirs() {}", "what main gained stays");
    assert_eq!(repo.git(&["show", &format!("{commit}:run.sh")]), "#!/bin/sh", "a file not listed is main's");
    assert!(repo.git(&["ls-tree", "-r", "--name-only", &commit]).lines().all(|name| name != "src/staged.rs" && name != "loose.rs"), "only the listed paths go");
    assert_eq!(repo.git(&["log", "-1", "--format=%s", &commit]), "armada.yml");
    assert_eq!(standing(&repo), before, "HEAD, status, index, branches and the files are as they were");

    pushed(&repo.root_str(), &commit, "armada/checkout-1a2b3c4d").expect("pushed");
    assert_eq!(git_in(&bare, &["rev-parse", "refs/heads/armada/checkout-1a2b3c4d"]), commit);
    assert_eq!(git_in(&bare, &["rev-parse", "refs/heads/main"]), tip, "main on the remote did not move");
    assert_eq!(standing(&repo), before, "and pushing left them so");
}

#[test]
fn a_script_stays_executable() {
    let (repo, _bare, _elsewhere) = a_checkout_with_a_remote();
    repo.write("run.sh", "#!/bin/sh\necho hi\n");
    std::fs::set_permissions(repo.root().join("run.sh"), std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("chmod");

    let commit = committed(&repo.root_str(), &sending(&["run.sh"])).expect("committed");

    assert!(repo.git(&["ls-tree", &commit, "run.sh"]).starts_with("100755"), "{}", repo.git(&["ls-tree", &commit, "run.sh"]));
}

/// The base moved under the hold: the commit must not put the owner's older version over what landed.
#[test]
fn a_path_main_changed_to_something_else_is_refused_and_nothing_is_written() {
    let (repo, _bare, elsewhere) = a_checkout_with_a_remote();
    main_moves(&repo, &elsewhere, "armada.yml", "version: 2\n");
    repo.write("armada.yml", "version: 1\nauto_merge: checks-pass\n");
    let before = standing(&repo);

    let refused = committed(&repo.root_str(), &sending(&["armada.yml"])).expect_err("refused");

    assert!(refused.said.contains("armada.yml"), "{}", refused.said);
    assert_eq!(standing(&repo), before);
}

/// A file that is main's own content is left out; where that is every file, there is nothing to send.
#[test]
fn files_that_already_equal_main_are_left_out_and_all_of_them_is_nothing_to_send() {
    let (repo, _bare, elsewhere) = a_checkout_with_a_remote();
    main_moves(&repo, &elsewhere, "armada.yml", "version: 2\n");
    repo.write("armada.yml", "version: 2\n");

    let refused = committed(&repo.root_str(), &sending(&["armada.yml"])).expect_err("nothing differs");
    assert!(refused.said.contains("already holds"), "{}", refused.said);

    repo.write("run.sh", "#!/bin/sh\necho changed\n");
    let commit = committed(&repo.root_str(), &sending(&["armada.yml", "run.sh"])).expect("the other file goes");
    assert_eq!(repo.git(&["diff", "--name-only", &format!("{commit}^"), &commit]), "run.sh");
}

#[test]
fn a_path_git_cannot_send_is_refused() {
    let (repo, _bare, _elsewhere) = a_checkout_with_a_remote();
    repo.write("dir/inner.rs", "x");

    let refused = committed(&repo.root_str(), &sending(&["dir"])).expect_err("a directory is not a file");

    assert!(refused.said.contains("dir"), "{}", refused.said);
}
