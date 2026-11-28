//! What Fleet's PreToolUse gate refuses an Operator's session, beyond what any hosted session is refused: a shell line that
//! does what its project switched off or holds until he has said so, and a file tool aimed at another project's checkout.
//! `crates/fleet/src/session_host/gate.rs` calls [`Fleet::operator_gated`] first for a session that is the Operator's.
//!
//! **A plain token match, and no more.** A line is cut at its separators, and a word `gh` or `bd` followed by the
//! words of an act is the act. It misses what a token match cannot see: a script or alias that runs the command, a variable that
//! holds a word of it, a program that calls the forge's API without `gh` (`curl`, `python`), and `git push` to a branch a
//! forge merges by itself. It also refuses a line that only names the act, such as an `echo`. The door is the strong
//! half: `merge_pull_request_by_number` and the starts are refused there by what the call is, not by a word.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{GateAnswer, OperatorActs, SessionGate};

use super::reaching::{said_needed, switch_off};
use crate::daemon::Fleet;

/// What a shell line does that a tool switch covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellAct {
    /// `gh pr merge`, or `gh api` on a merge path.
    Merge,
    /// `gh issue create`, `bd create`, `bd new`.
    FileIssue,
}

/// The words of `command` between its separators, each with its quotes and brackets taken off.
fn segments(command: &str) -> Vec<Vec<String>> {
    command
        .split(|c: char| matches!(c, ';' | '|' | '&' | '\n' | '(' | ')' | '`'))
        .map(|segment| segment.split_whitespace().map(|word| word.trim_matches(|c| matches!(c, '"' | '\'' | '{' | '}')).to_string()).filter(|word| !word.is_empty()).collect::<Vec<_>>())
        .filter(|words| !words.is_empty())
        .collect()
}

/// Whether `word` is a path of the forge's API that merges: it ends in `/merge`, or it is a mutation that merges.
fn merges_by_api(word: &str) -> bool {
    let path = word.split('?').next().unwrap_or_default();
    path.ends_with("/merge") || word.contains("mergePullRequest") || word.contains("enablePullRequestAutoMerge")
}

/// The acts `command` runs that a tool switch covers, in the order they appear.
pub(crate) fn shell_acts(command: &str) -> Vec<ShellAct> {
    let mut acts = Vec::new();
    for words in segments(command) {
        for (at, word) in words.iter().enumerate() {
            let program = word.rsplit('/').next().unwrap_or_default();
            if program != "gh" && program != "bd" {
                continue;
            }
            // The words after the program that are not flags, and not the value of the flag that names a repository.
            let mut after = words[at + 1..].iter();
            let mut named: Vec<&str> = Vec::new();
            while let Some(next) = after.next() {
                if matches!(next.as_str(), "-R" | "--repo") {
                    after.next();
                } else if !next.starts_with('-') {
                    named.push(next);
                }
            }
            match (program, named.as_slice()) {
                ("gh", ["pr", "merge", ..]) => acts.push(ShellAct::Merge),
                ("gh", ["api", rest @ ..]) if rest.iter().any(|word| merges_by_api(word)) => acts.push(ShellAct::Merge),
                ("gh", ["issue", "create", ..]) | ("bd", ["create" | "new", ..]) => acts.push(ShellAct::FileIssue),
                _ => {}
            }
        }
    }
    acts
}

/// `path` with `.` and `..` resolved by reading it, never by touching disk. A relative path is returned as it came.
fn normalised(path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_string();
    }
    let mut kept: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                kept.pop();
            }
            other => kept.push(other),
        }
    }
    format!("/{}", kept.join("/"))
}

/// The paths a file tool's call names, each made absolute against where the call stands.
fn paths_of(gate: &SessionGate) -> Vec<String> {
    if !matches!(gate.tool_name.as_str(), "Read" | "Grep" | "Glob" | "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
        return Vec::new();
    }
    let text = |key: &str| gate.tool_input.get(key).and_then(|said| said.as_str());
    let named = ["file_path", "notebook_path", "path"].into_iter().filter_map(text).map(str::to_string);
    // A Glob's pattern may itself be a path.
    let pattern = text("pattern").filter(|pattern| pattern.starts_with('/') && gate.tool_name == "Glob").map(str::to_string);
    named
        .chain(pattern)
        .map(|path| match path.starts_with('/') {
            true => normalised(&path),
            false if gate.cwd.starts_with('/') => normalised(&format!("{}/{path}", gate.cwd.trim_end_matches('/'))),
            false => path,
        })
        .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The deny for a call of the Operator `session` of `project` that its settings refuse, or none.
    pub(crate) async fn operator_gated(self: &Arc<Self>, gate: &SessionGate, session: &str, project: &str) -> Option<GateAnswer> {
        if gate.tool_name == "Bash" {
            let command = gate.tool_input.get("command").and_then(|command| command.as_str())?;
            return self.operator_shell_refused(command, project).await.map(GateAnswer::deny);
        }
        for path in paths_of(gate) {
            let Some(to) = self.operator_path_outside(project, &path) else { continue };
            let words = format!("{} {path}", if matches!(gate.tool_name.as_str(), "Read" | "Grep" | "Glob") { "read" } else { "change" });
            if let Some(why) = self.operator_held_across(session, project, &to, &gate.tool_name, &words).await {
                return Some(GateAnswer::deny(why));
            }
        }
        None
    }

    /// Why the shell line `command` is refused to `project`'s Operator, where a switch or its level refuses an act in it.
    async fn operator_shell_refused(&self, command: &str, project: &str) -> Option<String> {
        let acts = shell_acts(command);
        if acts.is_empty() {
            return None;
        }
        let settings = self.operator_settings_of(project).await;
        for act in acts {
            let (on, tool, what) = match act {
                ShellAct::Merge => (settings.tools.merge, "Merge pull requests", "merges a pull request"),
                ShellAct::FileIssue => (settings.tools.file_issues, "File issues", "files an issue"),
            };
            if !on {
                return Some(format!("That command {what}, which {}.", switch_off(tool)));
            }
            if settings.acts == Some(OperatorActs::OnlyWhenTold) && !self.operator_say_open(project) {
                return Some(format!("That command {what}, which {}.", said_needed(OperatorActs::OnlyWhenTold)));
            }
        }
        None
    }

    /// The project a path is in the checkout of, where `from` is not one of the projects that hold that repository.
    fn operator_path_outside(&self, from: &str, path: &str) -> Option<String> {
        let served = self.repositories().served();
        let inside = |root: &str| path == root.trim_end_matches('/') || path.starts_with(&format!("{}/", root.trim_end_matches('/')));
        let owner = served.iter().filter(|one| inside(one.root())).max_by_key(|one| one.root().len())?;
        self.operator_repository_outside(from, owner.manifest().id().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{shell_acts, ShellAct};

    #[test]
    fn a_merge_or_a_new_issue_is_read_in_a_line_and_nothing_else_is() {
        assert_eq!(shell_acts("gh pr merge 41 --squash"), [ShellAct::Merge]);
        assert_eq!(shell_acts("cd x && gh pr merge --auto -R o/r 41"), [ShellAct::Merge]);
        assert_eq!(shell_acts("gh api -X PUT repos/o/r/pulls/41/merge"), [ShellAct::Merge]);
        assert_eq!(shell_acts("bash -c \"gh pr merge 3\""), [ShellAct::Merge]);
        assert_eq!(shell_acts("gh issue create --title t"), [ShellAct::FileIssue]);
        assert_eq!(shell_acts("bd create \"a\" && bd new b"), [ShellAct::FileIssue, ShellAct::FileIssue]);
        assert!(shell_acts("gh pr view 41 && gh issue list && gh api repos/o/r/pulls/41").is_empty());
        assert!(shell_acts("bd list && bd show armada-a3f2 && git commit -m x").is_empty());
    }
}
