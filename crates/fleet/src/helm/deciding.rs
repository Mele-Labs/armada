//! What a Helm call is put to a person for, and what simply runs. `#1525`.
//!
//! **Fleet decides this and the person's own settings no longer do**, which
//! reverses `super::asking`'s old rule. The owner's three classes on 18 Sep
//! 2026 are the whole of it: destructive, pushes code to a shared space, writes
//! off this machine. `docs/concepts/helm.md`, *What Helm asks about*.
//!
//! **Their settings still decide what they decide.** The CLI reads them before
//! Fleet is called; what arrives here is the remainder.
//!
//! **Auto *unless*, so an unrecognised command runs.** The safety is in
//! recognising the risky set, and what runs unasked is on the record as
//! `HelmCallSettled::RanUnasked` — the audit trail is what catches a rule drawn
//! too wide.

use ipc::{AskingToRun, CallKind, ReachesTo};

use super::reading::{
    changes_only_held, command_of, kind_of, queries, reaches_into_owners_files, segment_only_reads, segment_writes, with_variables,
};

mod folders;
mod programs;
mod shell;

pub use folders::{guarded_in_a_session, pages_opened, Folders, KindAllowed};
pub(in crate::helm) use folders::{reaches_outside, read_refused};
pub(in crate::helm) use shell::{truncating_redirect, Slot};
use folders::{cd_stays, in_the_slot, inside_the_directory, Listed};
use programs::{fetching, forge, git, kills_only_its_own, publishing};
use shell::{shell, shell_guarded};

/// Why one call is being put to a person. **A reason and not a category**: it
/// is written to be read in an event, and on the card itself one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Because {
    /// It removes or overwrites something that does not come back.
    Destructive,
    /// It sends code somewhere other people read from.
    PushesToShared,
    /// It writes to something that is not this machine.
    WritesOffMachine,
    /// A session writes past its slot into the owner's own files, or reads a
    /// folder macOS asks him about, his credentials or the whole disk. **macOS
    /// asks him about those folders on Fleet's behalf** (the owner, 8 Oct 2026,
    /// after a `find /`); any other read runs (16 Oct 2026).
    ReachesOutside,
    /// Its shape could not be read. **The one reason about this module rather
    /// than about the call**: `eval`, a backtick and a command substitution can
    /// each carry anything, and those are the lines worth getting right.
    Unreadable,
}

impl Because {
    /// The reason in a person's words, for the event and for a card.
    pub fn said(&self) -> &'static str {
        match self {
            Because::Destructive => "it removes or overwrites something",
            Because::PushesToShared => "it sends code where other people read it",
            Because::WritesOffMachine => "it writes to something off this machine",
            Because::ReachesOutside => "it writes to your files outside its slot, or reads what macOS guards",
            Because::Unreadable => "what this line does cannot be read from it",
        }
    }
}

/// The built-in tools that write a file. **Not `adapters::CHANGES_THE_CHECKOUT`**,
/// which recognises a write for the event it publishes rather than to decide
/// whether to ask about one. The two would not move together.
const WRITES_A_FILE: &[&str] = &["Write", "Edit", "NotebookEdit"];

/// Whether one call is put to a person, and why. `None` runs it.
///
/// **The tool decides which question is asked.** A door tool is one of Armada's
/// own operations, a shell line is read as one, a write is read against the
/// path it names, and anything else by its name alone.
pub fn because(asking: &AskingToRun) -> Option<Because> {
    let tool = asking.tool_name.as_str();
    if let Some(operation) = tool.strip_prefix(&format!("mcp__{}__", ipc::door::SERVER)) {
        return door(operation, asking);
    }
    if tool == "Bash" {
        return shell(asking.detail().unwrap_or_default(), &[], Own::default());
    }
    if WRITES_A_FILE.contains(&tool) {
        return writing(asking);
    }
    named(tool)
}

/// [`because`] for a call a hosted session made, where `held` are the directories it may
/// write in: the slot it leased and every slot something it started leased (its subagents),
/// or the repository's root before its first lease. **The same three classes, read for a
/// session**: what it writes inside a slot it holds is its work, and a line is read through the
/// substitutions and heredocs it carries, since `git commit -m "$(cat <<'EOF'` is the ordinary
/// way to commit and is none of the three. Which slot is held is the lease ledger's to say;
/// whether a slot may be written at all is the write gate's.
pub fn because_in_a_session(asking: &AskingToRun, held: &[&str]) -> Option<Because> {
    because_in_a_session_owning(asking, held, Own::default())
}

/// What a session made or started beyond the slots it holds, which it may remove again.
/// **Its own and no one else's** (the owner, retro of session 9c74d82e): a dev server it
/// started, and a branch of its own the base already holds.
#[derive(Clone, Copy, Default)]
pub struct Own<'a> {
    /// Whether a process is one the session started, however deep. Absent: none is.
    pub started: Option<&'a dyn Fn(u32) -> bool>,
    /// Its branches whose every commit the base holds.
    pub merged_branches: &'a [&'a str],
}

impl Own<'_> {
    fn started(&self, pid: u32) -> bool {
        self.started.is_some_and(|started| started(pid))
    }

    fn merged(&self, branch: &str) -> bool {
        self.merged_branches.contains(&branch)
    }
}

/// [`because_in_a_session`] for a session that owns what it started, with no folder listed.
pub fn because_in_a_session_owning(
    asking: &AskingToRun,
    held: &[&str],
    own: Own,
) -> Option<Because> {
    match guarded_in_a_session(asking, held, own, Folders::default())? {
        Guarded::Because { because, .. } => Some(because),
        Guarded::Blocked { .. } => Some(Because::ReachesOutside),
    }
}

/// What the guard says about one hosted session's call, once the owner's folder list is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Guarded {
    /// Put to a person, or refused by the mode, for `because`. `reaching` is the absolute path past
    /// the slots it named first, which a card offers to the folder list. **None for a removal**: an
    /// allowed folder lets a write in and never an `rm`. `to` says whether a reach reads or writes,
    /// and is `None` where nothing was reached. `kind` is the program and script of the Bash
    /// segment that named `reaching`, which a card offers to allow for the job; `None` otherwise.
    Because { because: Because, reaching: Option<String>, to: Option<ReachesTo>, kind: Option<CallKind> },
    /// A folder the owner blocked holds what it names. Refused with no card.
    Blocked { folder: String },
}

/// One of Armada's own operations.
///
/// **A query never asks**, since a read changes nothing anywhere. A command
/// asks only where it is one of the three: writing a Studio note and starting
/// a local run are none of them.
fn door(operation: &str, asking: &AskingToRun) -> Option<Because> {
    const SHARED: &[&str] = &[
        "approve_review",
        "merge_pull_request",
        // A pull request by number is nobody's Job and the same act: the
        // merge, the draft taken out of draft, and the standing instruction
        // to merge. All three change what other people read.
        "merge_pull_request_by_number",
        "ready_pull_request",
        "enable_auto_merge",
        // A comment, an approval or a request for changes is read by everyone
        // watching the repository.
        "write_pull_request",
        "dispatch_studio_draft",
        "request_changes",
    ];
    const GONE: &[&str] = &[
        "kill_process",
        "kill_processes",
        "delete_branch",
        "delete_studio",
        "remove_studio_nodes",
        "reclaim_worktree",
        "undo_run",
        "undo_checkout_run",
        "restart_fleet",
        "save_manifest_file",
        "edit_manifest",
    ];
    const OFF_MACHINE: &[&str] = &[
        "file_finding_issue",
        "rerun_checks",
        "rerun_failed_checks",
        "investigate_failed_checks",
        "add_repository",
        "clone_repository",
        "create_project",
    ];
    if operation == "save_workflow" {
        return replacing_a_workflow(asking);
    }
    if SHARED.contains(&operation) {
        return Some(Because::PushesToShared);
    }
    if GONE.contains(&operation) {
        return Some(Because::Destructive);
    }
    if OFF_MACHINE.contains(&operation) {
        return Some(Because::WritesOffMachine);
    }
    None
}

/// `save_workflow` asks only where it says `overwrite`. **Replacing a
/// definition is the destructive act and a first save is not**: Fleet refuses a
/// save that would replace one unless the call carries `overwrite`, so the
/// word is the whole of the line between the two, and a call that omits it
/// cannot reach the case this guards.
fn replacing_a_workflow(asking: &AskingToRun) -> Option<Because> {
    let overwrite = asking
        .input
        .get("body")
        .and_then(|body| body.get("overwrite"))
        .and_then(|said| said.as_bool());
    (overwrite == Some(true)).then_some(Because::Destructive)
}

/// A built-in, or a tool from a server Armada knows nothing about.
///
/// **By the verb in its name, which is all there is.** `delete_page` says what
/// it does whoever wrote it, and under *auto unless* a name carrying none of
/// these runs.
fn named(tool: &str) -> Option<Because> {
    let lowered = tool.to_ascii_lowercase();
    const GONE: &[&str] = &["delete", "remove", "destroy", "purge", "revoke"];
    const SHARED: &[&str] = &["publish", "merge", "push"];
    const OFF_MACHINE: &[&str] = &["send", "upload", "deploy", "invite"];
    if GONE.iter().any(|word| lowered.contains(word)) {
        return Some(Because::Destructive);
    }
    if SHARED.iter().any(|word| lowered.contains(word)) {
        return Some(Because::PushesToShared);
    }
    if OFF_MACHINE.iter().any(|word| lowered.contains(word)) {
        return Some(Because::WritesOffMachine);
    }
    None
}

/// A write to the checkout, read against the path it names.
///
/// **An overwrite keeps its card and a new file does not.** The owner's 17 Sep
/// decision — Helm asks before it edits, because there is no undo — and his
/// later rule agree on an overwrite and differ only on a file that did not
/// exist, which takes nothing away. A path Fleet cannot read is an overwrite:
/// being unable to tell is not a reason to assume the harmless one.
fn writing(asking: &AskingToRun) -> Option<Because> {
    let path = asking
        .input
        .get("file_path")
        .or_else(|| asking.input.get("path"))
        .or_else(|| asking.input.get("notebook_path"))
        .and_then(|named| named.as_str());
    match path {
        Some(path) if !std::path::Path::new(path).exists() => None,
        _ => Some(Because::Destructive),
    }
}
