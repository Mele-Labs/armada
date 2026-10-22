//! A form's edit, committed and pushed from objects, against real git and a
//! bare remote. The forge half, opening the pull request, needs a forge and is
//! not run here: an edit sent to a branch already open takes the same commit
//! and the same push, and stops before it.

use std::path::Path;

use adapter_traits::{ManifestEditToSend, PullRequestFor};

use crate::manifest_edit::{goes_by_pull_request, on_the_base, send};
use crate::GhForge;
use crate::tests::repo::TempRepo;

const WAS: &str = "version: 1\nid: kept\n# by hand\nbase: main\n";
const NOW: &str = "version: 1\nid: kept\n# by hand\nbase: main\nauto_merge: checks-pass\n";

fn git_in(dir: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(
        run.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// A checkout whose remote holds `manifest` on `main`, and the bare remote.
fn behind_its_remote(manifest: &str) -> (TempRepo, std::path::PathBuf) {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.commit_one(manifest, WAS, "a manifest");
    repo.git(&["push", "--quiet", "--set-upstream", "origin", "main"]);
    (repo, bare)
}

fn path_of(repo: &TempRepo, file: &str) -> String {
    repo.root().join(file).to_string_lossy().into_owned()
}

fn sending(repo: &TempRepo, file: &str, parent: &str, text: &str) -> ManifestEditToSend {
    ManifestEditToSend {
        file: path_of(repo, file),
        text: text.to_string(),
        parent: parent.to_string(),
        message: String::from("armada.yml: auto_merge checks-pass"),
        branch: String::from("armada/manifest-1"),
        to: PullRequestFor::Update {
            branch: String::from("armada/manifest-1"),
            pull_request: String::from("https://forge.invalid/armada/pull/9"),
        },
    }
}

/// What a person standing in the checkout would see, every way git can say it.
fn standing(repo: &TempRepo) -> (String, String, String, String) {
    (
        repo.git(&["rev-parse", "HEAD"]),
        repo.git(&["status", "--porcelain=v1", "--untracked-files=all"]),
        repo.git(&["ls-files", "--stage"]),
        repo.git(&["branch", "--format=%(refname)"]),
    )
}

#[test]
fn an_edit_reaches_the_remote_branch_and_the_checkout_does_not_move() {
    let (repo, bare) = behind_its_remote("armada.yml");
    // Work in progress of every kind: staged, untracked, and the Manifest
    // itself edited by hand and not committed.
    repo.write("src/staged.rs", "staged");
    repo.git(&["add", "--", "src/staged.rs"]);
    repo.write("src/loose.rs", "untracked");
    repo.write("armada.yml", "# edited by hand, uncommitted\n");
    let before = standing(&repo);

    let file = path_of(&repo, "armada.yml");
    let base = on_the_base(&repo.root_str(), "main", &file).expect("the base is read");
    assert_eq!(base.text, WAS, "the remote's text, not the edited file's");
    assert_eq!(base.commit, git_in(&bare, &["rev-parse", "refs/heads/main"]));

    let sent = send(&repo.root_str(), &sending(&repo, "armada.yml", &base.commit, NOW), &GhForge)
        .expect("the edit goes out");

    assert_eq!(
        git_in(&bare, &["rev-parse", "refs/heads/armada/manifest-1"]),
        sent.commit
    );
    assert_eq!(
        git_in(&bare, &["diff", "--name-only", &base.commit, &sent.commit]),
        "armada.yml",
        "only the Manifest changed"
    );
    assert_eq!(
        git_in(&bare, &["show", &format!("{}:armada.yml", sent.commit)]),
        NOW.trim_end()
    );
    assert_eq!(
        git_in(&bare, &["rev-parse", &format!("{}^", sent.commit)]),
        base.commit
    );
    assert_eq!(
        git_in(&bare, &["log", "-1", "--format=%s%n%an", &sent.commit]),
        "armada.yml: auto_merge checks-pass\narmada",
        "the message names the key, and the author is the repository's own user"
    );
    assert_eq!(sent.pull_request, "https://forge.invalid/armada/pull/9");

    assert_eq!(
        standing(&repo),
        before,
        "HEAD, index, tree and branches are as they were"
    );
    assert_eq!(
        std::fs::read_to_string(repo.root().join("armada.yml")).unwrap(),
        "# edited by hand, uncommitted\n",
        "the file on disk is not rewritten"
    );
    assert_eq!(
        std::fs::read_to_string(repo.root().join("src/loose.rs")).unwrap(),
        "untracked"
    );
}

/// The forge half is not reachable here, so opening the pull request fails, and
/// the branch the push made must not be left on the remote with nothing on it.
#[test]
fn a_pull_request_that_will_not_open_takes_the_pushed_branch_off_the_remote() {
    let (repo, bare) = behind_its_remote("armada.yml");
    let base = on_the_base(&repo.root_str(), "main", &path_of(&repo, "armada.yml")).unwrap();
    let mut opening = sending(&repo, "armada.yml", &base.commit, NOW);
    opening.to = PullRequestFor::Open { base: String::from("main") };

    let refused = send(&repo.root_str(), &opening, &GhForge);

    assert!(refused.is_err(), "no forge here to open it");
    assert_eq!(
        git_in(&bare, &["branch", "--list", "armada/manifest-*"]),
        "",
        "the pushed branch is gone"
    );
}

#[test]
fn a_second_edit_stacks_on_the_branch_the_first_made() {
    let (repo, bare) = behind_its_remote("armada.yml");
    let base = on_the_base(&repo.root_str(), "main", &path_of(&repo, "armada.yml")).unwrap();
    let first = send(&repo.root_str(), &sending(&repo, "armada.yml", &base.commit, NOW), &GhForge).unwrap();
    let again = format!("{NOW}review_gate: never\n");

    let second = send(
        &repo.root_str(),
        &sending(&repo, "armada.yml", &first.commit, &again),
        &GhForge,
    )
    .unwrap();

    assert_eq!(
        git_in(&bare, &["rev-parse", "refs/heads/armada/manifest-1"]),
        second.commit
    );
    assert_eq!(
        git_in(
            &bare,
            &["rev-list", "--count", &format!("{}..{}", base.commit, second.commit)]
        ),
        "2"
    );
}

#[test]
fn a_manifest_in_a_workspace_is_replaced_at_its_own_path() {
    let (repo, bare) = behind_its_remote("web/armada.yml");
    let base = on_the_base(&repo.root_str(), "main", &path_of(&repo, "web/armada.yml"))
        .expect("read by its path from the root");
    assert_eq!(base.text, WAS);

    let sent = send(
        &repo.root_str(),
        &sending(&repo, "web/armada.yml", &base.commit, NOW),
        &GhForge,
    )
    .unwrap();
    assert_eq!(
        git_in(&bare, &["diff", "--name-only", &base.commit, &sent.commit]),
        "web/armada.yml"
    );
}

#[test]
fn a_tracked_manifest_in_a_repository_with_an_origin_goes_by_pull_request() {
    let (repo, _bare) = behind_its_remote("armada.yml");
    assert!(goes_by_pull_request(&repo.root_str(), &path_of(&repo, "armada.yml")));
}

#[test]
fn an_ignored_manifest_is_the_file_on_disk() {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.commit_one(".gitignore", "armada.yml\n", "ignore it");
    repo.write("armada.yml", WAS);
    assert!(!goes_by_pull_request(&repo.root_str(), &path_of(&repo, "armada.yml")));
}

#[test]
fn an_untracked_manifest_is_the_file_on_disk() {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.write("armada.yml", WAS);
    assert!(!goes_by_pull_request(&repo.root_str(), &path_of(&repo, "armada.yml")));
}

#[test]
fn a_repository_with_no_origin_is_the_file_on_disk() {
    let repo = TempRepo::with_a_commit();
    repo.commit_one("armada.yml", WAS, "a manifest");
    assert!(!goes_by_pull_request(&repo.root_str(), &path_of(&repo, "armada.yml")));
}
