//! Which files a person has touched: the paths their commits changed on the default branch.
//! `docs/concepts/pull-review.md`, *Natural Search*.
//!
//! **Read-only and bounded**: one `git log` over the last few hundred of the person's commits.
//! The names git matches are given outright and as fixed strings, so a login with a dot in it is
//! not a pattern.

use crate::delivery::read_in;

/// The paths the last `commits` commits by any of `authors` (and by the repository's configured
/// identity) changed, newest first, each once. Nothing for a repository git will not read.
pub(crate) fn paths_touched_by(repo: &str, authors: &[String], commits: usize) -> Vec<String> {
    let mut who: Vec<String> = Vec::new();
    let configured = ["user.email", "user.name"]
        .into_iter()
        .filter_map(|key| config_value(repo, key));
    for name in authors.iter().cloned().chain(configured) {
        let name = name.trim().to_string();
        if !name.is_empty() && !who.iter().any(|had| had.eq_ignore_ascii_case(&name)) {
            who.push(name);
        }
    }
    if who.is_empty() || commits == 0 {
        return Vec::new();
    }
    let (branch, limit) = (default_branch(repo), format!("--max-count={commits}"));
    let mut args: Vec<String> = ["log", &branch, "--no-merges", "--regexp-ignore-case", "--fixed-strings", &limit]
        .map(str::to_string)
        .to_vec();
    args.extend(who.iter().map(|name| format!("--author={name}")));
    // An empty format and `-z`: only the NUL-separated paths of each commit are printed.
    args.extend(["--name-only", "-z", "--format=", "--"].map(str::to_string));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let Ok(run) = read_in(repo, "git", &args) else {
        return Vec::new();
    };
    if !run.status.success() {
        return Vec::new();
    }
    let mut paths: Vec<String> = Vec::new();
    for path in String::from_utf8_lossy(&run.stdout).split('\0') {
        let path = path.trim_matches(['\n', '\r']);
        if !path.is_empty() && !paths.iter().any(|had| had == path) {
            paths.push(path.to_string());
        }
    }
    paths
}

fn config_value(repo: &str, key: &str) -> Option<String> {
    let run = read_in(repo, "git", &["config", "--get", key]).ok()?;
    let value = String::from_utf8_lossy(&run.stdout).trim().to_string();
    (run.status.success() && !value.is_empty()).then_some(value)
}

/// What the remote names as its default, else `main` or `master` where one exists here, else
/// whatever is checked out.
fn default_branch(repo: &str) -> String {
    let remote = read_in(repo, "git", &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"]).ok();
    if let Some(run) = remote.filter(|run| run.status.success()) {
        let name = String::from_utf8_lossy(&run.stdout).trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    ["main", "master"]
        .into_iter()
        .find(|name| {
            read_in(repo, "git", &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")])
                .is_ok_and(|run| run.status.success())
        })
        .unwrap_or("HEAD")
        .to_string()
}
