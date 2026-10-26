//! Installing a tool Fleet found missing, when a person asks.
//!
//! **Homebrew is looked for in its own places before the PATH.** Fleet is started by Bridge, and a
//! program launched from the Dock does not carry the shell's PATH, so `brew` may not be on it.

use std::io::ErrorKind;

use adapter_traits::{Tool, ToolInstalled};

use crate::jira_tracker::Run;

/// Where Homebrew puts itself: Apple silicon, then Intel, then whatever the PATH holds.
const BREW: [&str; 3] = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew", "brew"];

/// The one command that installs `tool`: a formula name is not a thing the wire chooses.
fn formula(tool: Tool) -> &'static str {
    match tool {
        Tool::Bd => "beads",
    }
}

pub(crate) fn installed<R: Run>(run: &R, tool: Tool) -> ToolInstalled {
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
