//! `armada land preflight` — ready a branch to join the line, and stamp the
//! tree it was measured on.

use std::path::Path;

use adapters::undeclared::undeclared;

use crate::need::Needs;

use super::armada_cli::covers;
use super::dir::StateDir;
use super::env::Env;
use super::git::checked;
use super::repo::{changed_paths, current_branch, is_ancestor, merge_base, rev_parse};
use super::shell::gh_view;
use super::stamp::{write_stamp, PreflightStamp};
use super::stop::Refused;

/// What a clean preflight leaves behind, for the CLI to print.
pub struct Preflighted {
    pub branch: String,
    pub pull_request: Option<u64>,
    pub tree: String,
    pub checks: Vec<String>,
}

/// The branch, head and tree a stamp or a queue entry may be built from —
/// refused unless every byte is committed. Nothing needs pushing: the runner
/// reads the branch from this clone.
pub fn ready_tree(cwd: &Path, env: &Env) -> Result<(String, String, String), Refused> {
    let found = current_branch(cwd);
    let branch = match &found {
        Some(name) if name != &env.base => found.clone().unwrap(),
        _ => {
            return Err(Refused(format!(
                "check out the branch to land; this is {}",
                found.as_deref().unwrap_or("a detached HEAD")
            )));
        }
    };

    let dirty = checked(cwd, &["status", "--porcelain", "--untracked-files=all"])?;
    let dirty = String::from_utf8_lossy(&dirty.stdout).trim().to_string();
    if !dirty.is_empty() {
        return Err(Refused(format!(
            "the tree has uncommitted or untracked files, and a stamp would not \
             describe what lands — commit or remove them:\n{dirty}"
        )));
    }

    let head = rev_parse(cwd, "HEAD")?;
    let tree = rev_parse(cwd, "HEAD^{tree}")?;
    Ok((branch, head, tree))
}

pub fn preflight(cwd: &Path, env: &Env) -> Result<Preflighted, Refused> {
    let state = StateDir::resolve(cwd).map_err(|why| Refused(why.to_string()))?;
    if let Some((name, value)) = env.relative_binaries() {
        return Err(Refused(format!(
            "{name}={value} is relative, and the gate runs in another directory — give an absolute path"
        )));
    }
    let (branch, head, tree) = ready_tree(cwd, env)?;

    checked(cwd, &["fetch", "--quiet", &env.remote, &env.base])?;
    let upstream = format!("{}/{}", env.remote, env.base);
    if is_ancestor(cwd, &head, &upstream) {
        return Err(Refused(format!(
            "{branch} has nothing ahead of {upstream} — commit the work first"
        )));
    }
    // The cheapest refusal that needs the base: nothing below is worth running
    // for a branch that took a number it never declared. `#1059`.
    let needs = Needs::of(cwd).map_err(Refused)?;
    // **Still the files**: `armada land` is being retired for pull requests and
    // is not moved onto the ledger. A need declared through Fleet is not seen here.
    let declared: Vec<String> = needs
        .standing()
        .into_iter()
        .filter(|need| need.branch == branch)
        .map(|need| need.path)
        .collect();
    if let Some(said) = undeclared(cwd, &upstream, &branch, &declared).map_err(Refused)? {
        return Err(Refused(said));
    }
    let number = open_pull_request(cwd, env, &branch);

    let base = merge_base(cwd, "HEAD", &upstream)?;
    let changed = changed_paths(cwd, &base, &head)?;
    let hits = covers(&env.armada, cwd, &changed).map_err(|stopped| Refused(stopped.detail))?;

    write_stamp(
        &state,
        &PreflightStamp {
            branch: branch.clone(),
            head,
            tree: tree.clone(),
            base,
            pr: number,
            checks: hits.clone(),
        },
    )
    .map_err(|why| Refused(why.to_string()))?;

    Ok(Preflighted {
        branch,
        pull_request: number,
        tree,
        checks: hits,
    })
}

/// The pull request open for `branch` against the base, if there is one. A
/// pull request is optional, and a forge that cannot be asked means none.
fn open_pull_request(cwd: &Path, env: &Env, branch: &str) -> Option<u64> {
    let pr = gh_view(&env.gh, cwd, branch, "number,state,baseRefName")?;
    let open = pr.state.as_deref() == Some("OPEN");
    let against = pr.base_ref_name.as_deref() == Some(env.base.as_str());
    pr.number.filter(|_| open && against)
}
