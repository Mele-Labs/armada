//! Installing a tool Fleet found missing, and setting it up where it has no project, when a person
//! asks. Setting `bd` up is what `scripts/beads-setup` does, in the repository's main checkout.
//!
//! **Homebrew is looked for in its own places before the PATH.** Fleet is started by Bridge, and a
//! program launched from the Dock does not carry the shell's PATH, so `brew` may not be on it.

use std::io::ErrorKind;
use std::path::Path;

use adapter_traits::{Tool, ToolInstalled};

use crate::beads_tracker::main_checkout;
use crate::delivery::said;
use crate::jira_tracker::Run;

/// Where Homebrew puts itself: Apple silicon, then Intel, then whatever the PATH holds.
const BREW: [&str; 3] = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew", "brew"];

/// The one command that installs `tool`: a formula name is not a thing the wire chooses.
fn formula(tool: Tool) -> &'static str {
    match tool {
        Tool::Bd => "beads",
    }
}

/// `tool` where it is absent, then set up for the repository at `in_repo`. **Idempotent**: a tool
/// that is there is not installed again, and a project that is there is left as it is.
pub(crate) fn installed<R: Run>(run: &R, tool: Tool, in_repo: &Path) -> ToolInstalled {
    if !present(run, tool) {
        let brewed = brewed(run, tool);
        if !brewed.done {
            return brewed;
        }
    }
    set_up(run, tool, &main_checkout(in_repo))
}

fn present<R: Run>(run: &R, tool: Tool) -> bool {
    !matches!(run.run(tool.name(), &["--version".to_string()], None), Err(why) if why.kind() == ErrorKind::NotFound)
}

fn brewed<R: Run>(run: &R, tool: Tool) -> ToolInstalled {
    let args = ["install".to_string(), formula(tool).to_string()];
    for brew in BREW {
        match run.run(brew, &args, None) {
            Ok(output) if output.status.success() => {
                return ToolInstalled { done: true, said: format!("`{}` is installed", tool.name()) };
            }
            Ok(output) => {
                let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let said = match said.is_empty() {
                    true => format!("`brew install {}` exited {}", formula(tool), output.status),
                    false => said,
                };
                return ToolInstalled { done: false, said };
            }
            Err(why) if why.kind() == ErrorKind::NotFound => continue,
            Err(why) => return ToolInstalled { done: false, said: why.to_string() },
        }
    }
    ToolInstalled {
        done: false,
        said: format!("Homebrew is not available, so `{}` was not installed", tool.name()),
    }
}

fn set_up<R: Run>(run: &R, tool: Tool, main: &Path) -> ToolInstalled {
    match tool {
        Tool::Bd => beads_set_up(run, main),
    }
}

/// Where `bd` and `git` each failed, in words.
fn stopped(did: &str, why: &std::io::Result<std::process::Output>) -> Option<ToolInstalled> {
    let said = match why {
        Err(why) => format!("{did}: {why}"),
        Ok(output) if !output.status.success() => match said(output) {
            said if said.is_empty() => format!("{did} exited {}", output.status),
            said => said,
        },
        Ok(_) => return None,
    };
    Some(ToolInstalled { done: false, said })
}

/// `scripts/beads-setup` after the install. **Never `bd init`**, and never a bootstrap where origin
/// holds no `refs/dolt/data`: that would start a second, empty backlog beside the first.
fn beads_set_up<R: Run>(run: &R, main: &Path) -> ToolInstalled {
    let at = main.display().to_string();
    let bd = |args: &[&str]| {
        let mut all = vec!["-C".to_string(), at.clone()];
        all.extend(args.iter().map(|one| one.to_string()));
        run.run("bd", &all, None)
    };
    let git = |args: &[&str]| {
        let mut all = vec!["-C".to_string(), at.clone()];
        all.extend(args.iter().map(|one| one.to_string()));
        run.run("git", &all, None)
    };

    if let Some(stop) = stopped("`bd metrics off`", &bd(&["metrics", "off"])) {
        return stop;
    }
    // Beads files are this machine's copy of the backlog, not part of the code.
    if let Err(why) = excluded(&main.join(".git/info/exclude")) {
        return ToolInstalled { done: false, said: format!("git could not be told to ignore the beads files: {why}") };
    }
    if main.join(".beads/metadata.json").is_file() {
        return ToolInstalled { done: true, said: format!("{at} already has a beads database, leaving it as it is") };
    }
    match git(&["ls-remote", "--exit-code", "origin", "refs/dolt/data"]) {
        Ok(output) if output.status.success() => {}
        Ok(_) => {
            return ToolInstalled {
                done: false,
                said: "origin holds no beads data (refs/dolt/data), so there is no backlog to pull; nothing was set up rather than starting an empty one beside it".to_string(),
            };
        }
        Err(why) => return ToolInstalled { done: false, said: format!("`git ls-remote` did not run: {why}") },
    }
    if let Some(stop) = stopped("`bd bootstrap`", &bd(&["bootstrap", "--yes"])) {
        return stop;
    }
    // Only the owner reads the database; beads' own folder is private.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(main.join(".beads"), std::fs::Permissions::from_mode(0o700));
    }
    // Beads reads its role from git config: maintainer means this clone pushes to origin itself.
    if let Some(stop) = stopped("`git config beads.role`", &git(&["config", "beads.role", "maintainer"])) {
        return stop;
    }
    ToolInstalled { done: true, said: format!("beads is set up in {at} from origin") }
}

/// The beads files added to the common directory's `info/exclude`, each once.
fn excluded(exclude: &Path) -> std::io::Result<()> {
    let held = std::fs::read_to_string(exclude).unwrap_or_default();
    let mut text = held.clone();
    for pattern in ["/.beads/", "/.beads.gate.lock"] {
        if !held.lines().any(|line| line == pattern) {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(pattern);
            text.push('\n');
        }
    }
    match text == held {
        true => Ok(()),
        false => {
            if let Some(dir) = exclude.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(exclude, text)
        }
    }
}
