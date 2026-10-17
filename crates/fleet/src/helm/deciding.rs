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

use ipc::{AskingToRun, ReachesTo};

use super::reading::{
    changes_only_held, command_of, queries, reaches_into_owners_files, segment_only_reads, with_variables,
};

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
    /// and is `None` where nothing was reached.
    Because { because: Because, reaching: Option<String>, to: Option<ReachesTo> },
    /// A folder the owner blocked holds what it names. Refused with no card.
    Blocked { folder: String },
}

/// The owner's folder list, the folders this session was allowed until it ends, and the home a
/// `~` or `$HOME` stands for.
#[derive(Clone, Copy, Default)]
pub struct Folders<'a> {
    pub allow: &'a [String],
    pub block: &'a [String],
    pub session: &'a [String],
    pub home: &'a str,
}

/// What the list says about one path.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Listed {
    Allowed,
    Blocked(String),
    Unlisted,
}

impl Folders<'_> {
    /// **The deepest entry holding `path` wins, and a block wins a tie.** The session's own folders
    /// are read after the owner's list, so any block of his wins over them.
    fn listed(&self, path: &str) -> Listed {
        let Some(path) = resolved(path, "/") else {
            return Listed::Unlisted;
        };
        let holds = |entry: &String| resolved(entry, "/").is_some_and(|at| path.starts_with(&at));
        let deepest = |list: &[String]| {
            list.iter()
                .filter_map(|entry| {
                    let at = resolved(entry, "/")?;
                    path.starts_with(&at).then(|| (at.components().count(), entry.clone()))
                })
                .max_by_key(|(depth, _)| *depth)
        };
        match (deepest(self.allow), deepest(self.block)) {
            (Some((allowed, _)), Some((blocked, folder))) if blocked >= allowed => Listed::Blocked(folder),
            (None, Some((_, folder))) => Listed::Blocked(folder),
            (Some(_), _) => Listed::Allowed,
            _ if self.session.iter().any(holds) => Listed::Allowed,
            _ => Listed::Unlisted,
        }
    }

    /// `word` as an absolute path, `~` and `$HOME` read as the home, with `.` and `..` folded.
    /// `None` for a relative path or one spelled through another variable.
    fn absolute(&self, word: &str) -> Option<String> {
        let word = word.rsplit('=').next().unwrap_or(word);
        let word = word.trim_matches(|c| c == '\'' || c == '"');
        let whole = ["~", "${HOME}", "$HOME"]
            .iter()
            .find_map(|home| word.strip_prefix(home).filter(|rest| rest.is_empty() || rest.starts_with('/')))
            .map(|rest| format!("{}{rest}", self.home.trim_end_matches('/')))
            .unwrap_or_else(|| word.to_string());
        if !whole.starts_with('/') || whole.contains('$') {
            return None;
        }
        let mut parts: Vec<&str> = Vec::new();
        for part in whole.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                _ => parts.push(part),
            }
        }
        Some(format!("/{}", parts.join("/")))
    }
}

/// [`because_in_a_session_owning`], and the owner's folder list read first: a path in a blocked
/// folder is refused with no card, one in an allowed folder passes the reach, and anything else is
/// put to him with the path it named.
pub fn guarded_in_a_session(
    asking: &AskingToRun,
    held: &[&str],
    own: Own,
    folders: Folders,
) -> Option<Guarded> {
    let asked = |because: Because, reaching: Option<String>, to: ReachesTo| {
        Some(Guarded::Because { because, reaching, to: Some(to) })
    };
    let tool = asking.tool_name.as_str();
    if WRITES_A_FILE.contains(&tool) {
        let path = asking
            .input
            .get("file_path")
            .or_else(|| asking.input.get("notebook_path"))
            .and_then(|named| named.as_str());
        if path.is_some_and(|path| {
            in_scratch(path) || held.iter().any(|one| inside_the_directory(path, one))
        }) {
            return None;
        }
        // **Any write outside the slots and scratch is put to the owner** (16 Oct 2026), a new
        // file as much as an overwrite.
        let Some(reaching) = path.and_then(|path| folders.absolute(path)) else {
            return because(asking).map(|because| Guarded::Because { because, reaching: None, to: None });
        };
        return match folders.listed(&reaching) {
            Listed::Blocked(folder) => Some(Guarded::Blocked { folder }),
            Listed::Allowed => None,
            Listed::Unlisted => asked(
                because(asking).unwrap_or(Because::ReachesOutside),
                Some(reaching),
                ReachesTo::Write,
            ),
        };
    }
    if tool == "Bash" {
        let line = flattened(asking.detail().unwrap_or_default());
        return shell_guarded(&line, &slots_of(held), own, Some(folders));
    }
    let named = ["file_path", "path"]
        .iter()
        .filter_map(|key| asking.input.get(*key)?.as_str());
    let reads = matches!(tool, "Read" | "Grep" | "Glob" | "LS");
    let walks = matches!(tool, "Grep" | "Glob");
    let to = if reads { ReachesTo::Read } else { ReachesTo::Write };
    for path in named {
        if !(reaches_outside(path) && (!reads || read_refused(path, walks))) {
            continue;
        }
        let reaching = folders.absolute(path);
        match reaching.as_deref().map(|at| folders.listed(at)) {
            Some(Listed::Blocked(folder)) => return Some(Guarded::Blocked { folder }),
            Some(Listed::Allowed) => {}
            _ => return asked(Because::ReachesOutside, reaching, to),
        }
    }
    because(asking).map(|because| Guarded::Because { because, reaching: None, to: None })
}

/// The web pages a shell line opens in the owner's own browser: `open`,
/// `xdg-open` or `python -m webbrowser` with an `http(s)` address. A file or a
/// folder is not a page, and neither is a word that merely contains one.
pub fn pages_opened(line: &str) -> Vec<String> {
    let chained = flattened(line).replace("&&", ";").replace("||", ";");
    let mut pages = Vec::new();
    for segment in chained.split(|c| c == ';' || c == '|' || c == '\n') {
        let words: Vec<&str> = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '\'' || c == '"'))
            .collect();
        let opens = match words.first().copied() {
            Some("open" | "xdg-open") => true,
            Some(python) if python.starts_with("python") => {
                words.windows(2).any(|pair| pair == ["-m", "webbrowser"])
            }
            _ => false,
        };
        if opens {
            pages.extend(
                words
                    .iter()
                    .filter(|word| word.starts_with("http://") || word.starts_with("https://"))
                    .map(|word| word.to_string()),
            );
        }
    }
    pages
}

/// Whether a path a session names reaches into the owner's home, another
/// volume or the whole disk. **A path that names Armada is let through**: it
/// is most likely looking for something of Armada's (the owner, 8 Oct 2026).
/// The system's own folders (`/opt`, `/usr`, `/tmp`) are no one's files.
pub(super) fn reaches_outside(word: &str) -> bool {
    let word = word.rsplit('=').next().unwrap_or(word);
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.to_lowercase().contains("armada") {
        return false;
    }
    let root = word.trim_end_matches('/');
    // Only `/` itself: jq's `//` is an operator, not the disk.
    word == "/"
        || matches!(root, "/Users" | "/Volumes")
        || word.starts_with('~')
        || word.contains("$HOME")
        || word.contains("${HOME}")
        || word.starts_with("/Users/")
        || word.starts_with("/Volumes/")
}

/// Folders under the owner's home that macOS puts a privacy prompt in front of, and the ones
/// holding his credentials. Compared without case, as the disk does.
const GUARDED: &[&str] = &[
    "desktop",
    "documents",
    "downloads",
    "pictures",
    "movies",
    "music",
    "library/mobile documents",
    "library/mail",
    "library/messages",
    "library/safari",
    "library/calendars",
    "library/application support/addressbook",
    "library/keychains",
    ".ssh",
    ".aws",
    ".gnupg",
    ".netrc",
    ".config/gh",
];

/// Whether a read of `word`, a path [`reaches_outside`] named, is still refused: a folder
/// [`GUARDED`] names, another volume, or a search (`walks`) from the root, `/Users` or the
/// home, which reaches every one of them. **Every other read outside the slots runs** (the
/// owner, 16 Oct 2026: "reads are fine, writes are the ones we need to be careful of").
pub(super) fn read_refused(word: &str, walks: bool) -> bool {
    let word = word.rsplit('=').next().unwrap_or(word);
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.to_lowercase().contains("armada") {
        return false;
    }
    if word == "/Volumes" || word.starts_with("/Volumes/") {
        return true;
    }
    let under = ["~", "$HOME", "${HOME}"]
        .iter()
        .find_map(|home| word.strip_prefix(home))
        .map(str::to_string)
        .or_else(|| {
            let rest = word.strip_prefix("/Users/")?;
            Some(rest.find('/').map_or(String::new(), |at| rest[at..].to_string()))
        });
    let Some(under) = under else {
        return walks && matches!(word.trim_end_matches('/'), "" | "/Users");
    };
    let under = under.trim_matches('/').to_lowercase();
    if under.split('/').any(|part| part == "..") {
        return true;
    }
    (walks && under.is_empty())
        || GUARDED
            .iter()
            .any(|guarded| under == *guarded || under.starts_with(&format!("{guarded}/")))
}

/// The directories of `held` that are slots. Before its first lease a session runs in
/// the repository's root, the owner's own checkout, and nothing is taken away
/// there unasked.
fn slots_of<'a>(held: &[&'a str]) -> Vec<&'a str> {
    held.iter()
        .copied()
        .filter(|directory| directory.contains("/.armada/slots/"))
        .collect()
}

/// Whether `word`, a path a shell line removes or overwrites, is inside
/// one of `slots`. **Removing a file in a slot it holds is the session's own work**
/// (the owner, 8 Oct 2026); its root, a `..`, a `~` or a `$` is never read as
/// inside, and a relative path only where the line has not moved with `cd`.
fn in_the_slot(word: &str, slots: &[&str], relative: bool) -> bool {
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    // `$TMPDIR` is scratch space spelled by variable, and the one `$` read as inside.
    if let Some(under) = word.strip_prefix("$TMPDIR/").or_else(|| word.strip_prefix("${TMPDIR}/")) {
        return !under.is_empty() && !under.contains(['$', '~']) && !under.split('/').any(|part| part == "..");
    }
    if word.is_empty() || word.contains('$') || word.starts_with('~') {
        return false;
    }
    // Scratch space is no one's work: a patch kept in `/tmp` between two
    // steps asked, on every try (8 Oct 2026).
    if in_scratch(word) {
        return true;
    }
    let below = |slot: &&str| {
        inside_the_directory(word, slot) && resolved(word, slot) != resolved(slot, "/")
    };
    if word.starts_with('/') {
        return slots.iter().any(below);
    }
    relative && !matches!(word.trim_end_matches('/'), "" | "." | "*") && slots.iter().any(below)
}

/// Whether a `cd` to `to` lands in one of `slots`, its root included: unlike a
/// removal, standing at the root of a slot is the slot's own.
fn cd_stays(to: &str, slots: &[&str]) -> bool {
    let to = to.trim_matches(|c| c == '\'' || c == '"');
    !to.is_empty()
        && !to.contains(['$', '~'])
        && slots.iter().any(|slot| inside_the_directory(to, slot))
}

/// Where a session may write and remove besides its slot.
const SCRATCH: &[&str] = &[
    "/tmp/",
    "/private/tmp/",
    "/var/folders/",
    "/private/var/folders/",
];

/// Whether `path` is under `directory`, a relative path being read against it.
/// **Read where it lands**: a `..` and a symlink are followed first, so
/// `/tmp/../etc/hosts` is not scratch and `slot/a/../b` is the slot's own.
fn inside_the_directory(path: &str, directory: &str) -> bool {
    match (resolved(path, directory), resolved(directory, "/")) {
        (Some(path), Some(directory)) => path.starts_with(directory),
        _ => false,
    }
}

/// Whether `path` is an absolute path inside the system's scratch folders.
fn in_scratch(path: &str) -> bool {
    path.starts_with('/')
        && resolved(path, "/").is_some_and(|real| {
            let real = real.to_string_lossy();
            SCRATCH.iter().any(|scratch| real.starts_with(scratch))
        })
}

/// Where `path` lands, a relative one read against `base`: every `.` and `..`
/// folded away and every symlink that exists followed. The part of the path
/// that does not exist yet is read as written, so a write that creates a file
/// is placed by its directory. `None` where a `..` climbs past the root.
fn resolved(path: &str, base: &str) -> Option<std::path::PathBuf> {
    let whole = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{base}/{path}")
    };
    let mut at = std::path::PathBuf::from("/");
    let mut missing = false;
    for part in whole.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !at.pop() {
                    return None;
                }
                missing = false;
            }
            _ => {
                at.push(part);
                if !missing {
                    match std::fs::canonicalize(&at) {
                        Ok(real) => at = real,
                        Err(_) => missing = true,
                    }
                }
            }
        }
    }
    Some(at)
}

/// A shell line with its heredoc bodies cut out and its substitutions turned
/// into segments, so the commands inside them are read like any other.
/// `eval` and `exec` are still read where they stand.
fn flattened(line: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut ending: Option<String> = None;
    for each in line.lines() {
        if let Some(tag) = &ending {
            if each.trim() == tag {
                ending = None;
            }
            continue;
        }
        kept.push(each);
        if let Some((_, after)) = each.split_once("<<") {
            let tag: String = after
                .trim_start_matches('-')
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !tag.is_empty() {
                ending = Some(tag);
            }
        }
    }
    let kept = kept.join("\n");
    if kept.contains("$(") || kept.contains('`') {
        return kept.replace("$(", ";").replace(['`', ')'], ";");
    }
    kept
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

/// A shell line.
///
/// **Every segment, never the first word.** `cd crates && rm -rf target` has
/// `cd` at the front of it, and a classifier reading one word would run it.
fn shell(line: &str, slots: &[&str], own: Own) -> Option<Because> {
    match shell_guarded(line, slots, own, None)? {
        Guarded::Because { because, .. } => Some(because),
        Guarded::Blocked { .. } => Some(Because::ReachesOutside),
    }
}

/// [`shell`], and for a session (`folders`) whether a segment's words name the owner's own
/// files. **Read per segment, after the variables a line set are put back**, so `d=~/Music; ls $d`
/// is read as the `ls ~/Music` it is, and the `~` of an awk script is not read as a path at all.
fn shell_guarded(line: &str, slots: &[&str], own: Own, folders: Option<Folders>) -> Option<Guarded> {
    let only = |because: Because| Some(Guarded::Because { because, reaching: None, to: None });
    let line = line.trim();
    if line.is_empty() {
        return only(Because::Unreadable);
    }
    if line.contains('`') || line.contains("$(") {
        return only(Because::Unreadable);
    }
    // Every class at once, and never worth reading further.
    if word_in(line, "sudo") || word_in(line, "doas") {
        return only(Because::Destructive);
    }
    // **`&&` and `||` become separators and a lone `&` does not.** Splitting on
    // every `&` cuts `2>&1` in half and leaves a segment ending in a bare `>`,
    // which then reads as a truncating redirect — `cargo build 2>&1 | tail -5`
    // asked, as a destructive line, until a case caught it.
    let quiet = unquoted(line);
    // Same length as the line, so a cut in one is a cut in the other.
    let chained = quiet.replace("&&", " ;").replace("||", " ;");
    let (mut cut, mut quiet_parts, mut whole_parts) = (String::new(), Vec::new(), Vec::new());
    let mut whole = String::new();
    for (c, original) in chained.chars().zip(line.chars()) {
        if matches!(c, ';' | '|' | '\n') {
            quiet_parts.push(std::mem::take(&mut cut));
            whole_parts.push(std::mem::take(&mut whole));
        } else {
            cut.push(c);
            whole.push(original);
        }
    }
    quiet_parts.push(cut);
    whole_parts.push(whole);
    let kept: Vec<usize> = (0..quiet_parts.len())
        .filter(|at| !quiet_parts[*at].trim().is_empty())
        .collect();
    let segments: Vec<String> =
        with_variables(kept.iter().map(|at| quiet_parts[*at].trim()).collect());
    // **Read with its quotes**, which the segments above lose: a `/` left alone by a quoted `|`
    // is no path.
    let whole: Vec<String> =
        with_variables(kept.iter().map(|at| whole_parts[*at].trim()).collect());
    // The owner's files it names, whether the segment naming each only reads, and which of them
    // the list has not answered.
    let reached: Vec<(String, ReachesTo)> = match folders {
        Some(_) => whole
            .iter()
            .flat_map(|said| {
                let to = if segment_only_reads(said) { ReachesTo::Read } else { ReachesTo::Write };
                reaches_into_owners_files(said).into_iter().map(move |word| (word, to))
            })
            .collect(),
        None => Vec::new(),
    };
    let mut unlisted: Option<(Option<String>, ReachesTo)> = None;
    if let Some(folders) = folders {
        for (word, to) in &reached {
            let at = folders.absolute(word);
            match at.as_deref().map(|at| folders.listed(at)) {
                Some(Listed::Blocked(folder)) => return Some(Guarded::Blocked { folder }),
                Some(Listed::Allowed) => {}
                _ => {
                    unlisted.get_or_insert((at, *to));
                }
            }
        }
    }
    // A `cd` into the slot keeps a relative path inside it; one anywhere else,
    // or one that cannot be read, loses track of where the line is.
    let slot = (!slots.is_empty()).then(|| Slot {
        slots,
        relative: segments.iter().all(|said| {
            // `(cd x && ...)` moves the subshell, so its `(` is not the program.
            let mut words = said.split_whitespace().map(|word| word.trim_start_matches(['(', '{'])).filter(|word| !word.is_empty());
            match words.next() {
                Some("cd" | "pushd") => words.next().is_some_and(|to| cd_stays(to, slots)),
                Some("popd") => false,
                _ => true,
            }
        }),
    });
    // A redirect into a file the reach covers is that reach, put to the owner or let in by the
    // list. **A removal is not**: an allowed folder never turns an `rm` into a run.
    let covered: Vec<&str> = reached.iter().map(|(word, _)| word.trim_matches(|c| c == '\'' || c == '"')).collect();
    let rest = segments.iter().find_map(|said| segment(said, slot, own, &covered));
    match (rest, unlisted) {
        (Some(because @ (Because::Destructive | Because::Unreadable)), _) => only(because),
        (_, Some((reaching, to))) => {
            Some(Guarded::Because { because: Because::ReachesOutside, reaching, to: Some(to) })
        }
        (rest, None) => rest.map(|because| Guarded::Because { because, reaching: None, to: None }),
    }
}

/// `line` with every `;`, `|`, `&`, `<` and `>` inside quotes turned to a
/// space. **Quoted text is an argument, not shell**: a commit message's
/// `<noreply@...>` and a `sed 's|a|b|'` script asked, as an overwrite, on
/// every commit a Session made (8 Oct 2026).
fn unquoted(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    line.chars()
        .map(|c| {
            // `"a\|b"` is grep's alternation: an escaped `|` inside quotes is text as well.
            if escaped {
                escaped = false;
                return match quote {
                    Some(_) if matches!(c, ';' | '|' | '&' | '<' | '>' | '\n') => ' ',
                    _ => c,
                };
            }
            match quote {
                Some(q) if c == q => {
                    quote = None;
                    c
                }
                Some('"') if c == '\\' => {
                    escaped = true;
                    c
                }
                Some(_) if matches!(c, ';' | '|' | '&' | '<' | '>' | '\n') => ' ',
                Some(_) => c,
                None if c == '\\' => {
                    escaped = true;
                    c
                }
                None if c == '\'' || c == '"' => {
                    quote = Some(c);
                    c
                }
                None => c,
            }
        })
        .collect()
}

/// Whether `word` appears in `line` as a word rather than inside a longer one,
/// so `evaluate.sh` is not an `eval`.
fn word_in(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .any(|said| said == word)
}

/// The slots a session's line may write in, and whether a relative path in it is
/// still read against one of them.
#[derive(Clone, Copy)]
pub(super) struct Slot<'a> {
    pub(super) slots: &'a [&'a str],
    pub(super) relative: bool,
}

impl Slot<'_> {
    pub(super) fn holds(&self, word: &str) -> bool {
        in_the_slot(word, self.slots, self.relative)
    }
}

/// One segment of a shell line, already split from the rest.
fn segment(said: &str, slot: Option<Slot>, own: Own, covered: &[&str]) -> Option<Because> {
    if truncating_redirect(said)
        && !redirect_targets(said).iter().all(|target| {
            slot.is_some_and(|slot| slot.holds(target))
                || covered.contains(&target.trim_matches(|c| c == '\'' || c == '"'))
        })
    {
        return Some(Because::Destructive);
    }
    let (program, arguments) = command_of(said)?;
    // **The shell builtins, and only in the position that makes them one.**
    // `pnpm --dir x exec vitest run` carries `exec` as a subcommand and is the
    // ordinary way this repository runs its tests; reading the word anywhere on
    // the line made every one of them ask.
    if program == "eval" || program == "exec" {
        return Some(Because::Unreadable);
    }
    match program {
        "git" => return git(&arguments, slot, own),
        "gh" => return forge(&arguments),
        "curl" | "wget" | "http" => return fetching(&arguments),
        _ => {}
    }
    const GONE: &[&str] = &[
        "rm",
        "rmdir",
        "shred",
        "unlink",
        "dd",
        "mkfs",
        "truncate",
        "kill",
        "killall",
        "pkill",
        "chown",
        "chmod",
        "mv",
        "launchctl",
        "diskutil",
        "systemctl",
    ];
    const SHARED: &[&str] = &["scp", "rsync", "ssh", "sftp"];
    const OFF_MACHINE: &[&str] = &["aws", "gcloud", "az", "kubectl", "terraform", "op"];
    const REMOVES: &[&str] = &["rm", "rmdir", "unlink"];
    if REMOVES.contains(&program) {
        let operands: Vec<&&str> = arguments
            .iter()
            .filter(|word| !word.starts_with('-'))
            .collect();
        let held = slot.is_some_and(|slot| {
            !operands.is_empty() && operands.iter().all(|word| slot.holds(word))
        });
        if !held {
            return Some(Because::Destructive);
        }
    } else if program == "kill" && kills_only_its_own(&arguments, own) {
        // A process it started is its own work, as a file in its slot is.
    } else if matches!(program, "chmod" | "chown") {
        if !changes_only_held(program, &arguments, slot) {
            return Some(Because::Destructive);
        }
    } else if queries(program, &arguments) {
        // `launchctl print` and `diskutil list` change nothing.
    } else if GONE.contains(&program) {
        return Some(Because::Destructive);
    }
    if SHARED.contains(&program) {
        return Some(Because::PushesToShared);
    }
    if OFF_MACHINE.contains(&program) {
        return Some(Because::WritesOffMachine);
    }
    // **A `push` subcommand, whatever pushed it.** Written as a subcommand test
    // rather than a list of programs because the list would be a list of
    // vendors, which is `adapters`' to hold and not this module's.
    if arguments.iter().find(|word| !word.starts_with('-')) == Some(&"push") {
        return Some(Because::PushesToShared);
    }
    match publishing(program, &arguments) {
        true => Some(Because::PushesToShared),
        false => None,
    }
}

/// A `>` that is not `>>` and not `>&`. **It destroys what the file held**,
/// however ordinary the command in front of it.
pub(super) fn truncating_redirect(said: &str) -> bool {
    let written: Vec<char> = said.chars().collect();
    written.iter().enumerate().any(|(at, c)| {
        *c == '>'
            && at.checked_sub(1).and_then(|back| written.get(back)) != Some(&'>')
            && written.get(at + 1) != Some(&'>')
            && written.get(at + 1) != Some(&'&')
            && !into_a_device(&written[at + 1..])
    })
}

/// The paths each truncating `>` in `said` writes to.
fn redirect_targets(said: &str) -> Vec<&str> {
    let written: Vec<char> = said.chars().collect();
    let mut targets = Vec::new();
    for (at, c) in said.char_indices() {
        let index = said[..at].chars().count();
        if c == '>'
            && index.checked_sub(1).and_then(|back| written.get(back)) != Some(&'>')
            && written.get(index + 1) != Some(&'>')
            && written.get(index + 1) != Some(&'&')
            && !into_a_device(&written[index + 1..])
        {
            targets.push(said[at + 1..].split_whitespace().next().unwrap_or(""));
        }
    }
    targets
}

/// A redirect into `/dev/null`, `/dev/stderr` and the rest. **A device holds
/// nothing to destroy**, and `2>/dev/null` is on most lines a session runs.
fn into_a_device(after: &[char]) -> bool {
    let target: String = after.iter().collect();
    target.trim_start().starts_with("/dev/")
}

/// `git`, by its subcommand. **Reading history is not writing it**: `log`,
/// `status`, `diff`, `show` and `worktree list` run.
fn git(arguments: &[&str], slot: Option<Slot>, own: Own) -> Option<Because> {
    // `-C <path>` and `-c <key=value>` carry a value, which is not the verb.
    let mut words = arguments.iter();
    let subcommand = loop {
        let word = words.next()?;
        if matches!(*word, "-C" | "-c") {
            words.next();
        } else if !word.starts_with('-') {
            break word;
        }
    };
    // **A session in its own slot rewrites its own branch unasked**: a reset, a
    // checkout, a rebase or a forced tag there touches nothing but the work it
    // made (the owner, 8 Oct 2026). It is in one where it has not `cd`ed out of
    // its slots, or where `-C` names a slot it holds (16 Oct 2026); pointed
    // anywhere else it is read as before. The stash is every worktree's, and a
    // push, a deleted branch and a pruned repository are everyone's.
    const OWN_BRANCH: &[&str] = &[
        "reset",
        "clean",
        "checkout",
        "restore",
        "rebase",
        "am",
        "cherry-pick",
        "revert",
        "merge",
        "tag",
    ];
    let pointed: Vec<&str> = arguments
        .windows(2)
        .filter(|pair| pair[0] == "-C")
        .map(|pair| pair[1])
        .collect();
    let own_slot = slot.is_some_and(|slot| {
        let held = |path: &str| {
            let path = path.trim_matches(|c| c == '\'' || c == '"');
            !path.contains(['$', '~'])
                && (path.starts_with('/') || slot.relative)
                && slot.slots.iter().any(|one| inside_the_directory(path, one))
        };
        let other_tree = arguments
            .iter()
            .any(|word| word.starts_with("--git-dir") || word.starts_with("--work-tree"));
        !other_tree
            && match pointed.is_empty() {
                true => slot.relative,
                false => pointed.iter().all(|path| held(path)),
            }
    });
    if own_slot
        && OWN_BRANCH.contains(subcommand)
        && !(*subcommand == "tag" && arguments.iter().any(|w| matches!(*w, "-d" | "--delete")))
    {
        return None;
    }
    const GONE: &[&str] = &[
        "reset",
        "clean",
        "checkout",
        "restore",
        "rebase",
        "stash",
        "filter-branch",
        "gc",
        "prune",
        "am",
    ];
    if *subcommand == "branch" && deletes_only_its_own(arguments, own) {
        return None;
    }
    let said = |word: &str| arguments.iter().any(|argument| *argument == word);
    match *subcommand {
        "push" => Some(Because::PushesToShared),
        // Cutting a branch takes nothing away, and listing a stash reads it.
        "checkout" if said("-b") || said("-B") || said("--orphan") => None,
        "stash" if said("list") || said("show") => None,
        // Each of these takes a ref or a tree away, and the working copy does
        // not bring one back.
        "branch" | "tag" | "worktree" | "remote" | "submodule"
            if arguments.iter().any(|word| takes_something_away(word)) =>
        {
            Some(Because::Destructive)
        }
        word if GONE.contains(&word) => Some(Because::Destructive),
        _ => None,
    }
}

/// `kill` naming only processes the session started. **A signal first, then pids and nothing
/// else**: a negative pid is a process group, and `kill -9 -1` is every process the user owns.
fn kills_only_its_own(arguments: &[&str], own: Own) -> bool {
    let mut words = arguments.iter().copied();
    let mut pids = Vec::new();
    let mut first = true;
    while let Some(word) = words.next() {
        if matches!(word, "-s" | "-n") && first {
            words.next();
        } else if word.starts_with('-') && first && word.len() > 1 {
            // A signal: `-9`, `-TERM`, `-SIGKILL`.
            if !word[1..].chars().all(|c| c.is_ascii_alphanumeric()) {
                return false;
            }
        } else {
            match word.parse::<u32>() {
                Ok(pid) if pid > 1 => pids.push(pid),
                _ => return false,
            }
        }
        first = false;
    }
    !pids.is_empty() && pids.into_iter().all(|pid| own.started(pid))
}

/// `git branch -d` or `-D` naming only branches of the session's own that the base holds. **No
/// other flag**: `-r` is a remote's branch and `-m` a rename.
fn deletes_only_its_own(arguments: &[&str], own: Own) -> bool {
    let mut words = arguments.iter().copied().skip_while(|word| *word != "branch").skip(1);
    let mut deleting = false;
    let mut named = Vec::new();
    for word in &mut words {
        match word {
            "-d" | "-D" | "--delete" => deleting = true,
            flag if flag.starts_with('-') => return false,
            branch => named.push(branch),
        }
    }
    deleting && !named.is_empty() && named.into_iter().all(|branch| own.merged(branch))
}

fn takes_something_away(word: &str) -> bool {
    matches!(
        word,
        "-D" | "-d" | "--delete" | "-f" | "--force" | "remove" | "prune" | "rm"
    )
}

/// The code host's own CLI, by its subcommand pair. **A read of somebody
/// else's repository is still a read**: `view`, `list`, `diff`, `status` and
/// `checks` run.
fn forge(arguments: &[&str]) -> Option<Because> {
    let words: Vec<&str> = arguments
        .iter()
        .copied()
        .filter(|word| !word.starts_with('-'))
        .collect();
    let verb = words.get(1).copied().unwrap_or_default();
    match words.first().copied().unwrap_or_default() {
        "pr" | "release" if matches!(verb, "merge" | "create" | "ready" | "upload") => {
            Some(Because::PushesToShared)
        }
        "pr" | "issue" if matches!(verb, "close" | "delete" | "reopen") => {
            Some(Because::Destructive)
        }
        // An edit keeps what it replaced in the forge's history, so it is a write and
        // not a removal (the owner, 13 Oct 2026, after a rewrite of #2248 was refused).
        "issue" if matches!(verb, "create" | "comment" | "edit") => Some(Because::WritesOffMachine),
        "pr" if verb == "edit" => Some(Because::WritesOffMachine),
        "api"
            if arguments
                .iter()
                .any(|word| *word == "-X" || *word == "--method") =>
        {
            Some(Because::WritesOffMachine)
        }
        "repo" | "secret" | "auth" | "workflow" => Some(Because::WritesOffMachine),
        _ => None,
    }
}

/// `curl` and its kin. **A GET is a read.** Anything carrying a body, or naming
/// a method other than GET or HEAD, writes to somebody else's machine.
fn fetching(arguments: &[&str]) -> Option<Because> {
    let mut words = arguments.iter().copied().peekable();
    while let Some(word) = words.next() {
        let writes = match word {
            "-d" | "--data" | "--data-raw" | "--data-binary" | "-F" | "--form" | "-T"
            | "--upload-file" | "--json" => true,
            "-X" | "--request" => !matches!(
                words
                    .peek()
                    .copied()
                    .unwrap_or_default()
                    .to_ascii_uppercase()
                    .as_str(),
                "GET" | "HEAD"
            ),
            _ => false,
        };
        if writes {
            return Some(Because::WritesOffMachine);
        }
    }
    None
}

/// A package manager sending a build where other people install from.
/// **`publish` and nothing else** — `cargo build` and `pnpm test` are the
/// ordinary case, which is why this reads the subcommand and not the program.
fn publishing(program: &str, arguments: &[&str]) -> bool {
    const MANAGERS: &[&str] = &[
        "cargo", "npm", "pnpm", "yarn", "gem", "pip", "pip3", "twine",
    ];
    MANAGERS.contains(&program) && arguments.iter().any(|word| *word == "publish")
}
