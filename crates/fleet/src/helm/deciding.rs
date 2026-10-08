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

use ipc::AskingToRun;

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
    /// A session reaches past its slot into the owner's own files or the
    /// whole disk. **macOS asks him about those folders on Fleet's behalf**,
    /// so the card comes first (the owner, 8 Oct 2026, after a `find /`).
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
            Because::ReachesOutside => "it reaches into your files outside its slot",
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
        return shell(asking.detail().unwrap_or_default(), None);
    }
    if WRITES_A_FILE.contains(&tool) {
        return writing(asking);
    }
    named(tool)
}

/// [`because`] for a call a hosted session made in `directory`, which is the
/// slot it leased. **The same three classes, read for a session**: what it
/// writes inside its own slot is its work, and a line is read through the
/// substitutions and heredocs it carries, since `git commit -m "$(cat <<'EOF'`
/// is the ordinary way to commit and is none of the three.
pub fn because_in_a_session(asking: &AskingToRun, directory: &str) -> Option<Because> {
    let tool = asking.tool_name.as_str();
    if WRITES_A_FILE.contains(&tool) {
        let path = asking
            .input
            .get("file_path")
            .or_else(|| asking.input.get("notebook_path"))
            .and_then(|named| named.as_str());
        if path.is_some_and(|path| inside_the_directory(path, directory)) {
            return None;
        }
        return because(asking);
    }
    if tool == "Bash" {
        let line = flattened(asking.detail().unwrap_or_default());
        // An escaped space is part of the path, as in `Application\ Support`.
        if line.replace("\\ ", "\u{0}").split_whitespace().any(reaches_outside) {
            return Some(Because::ReachesOutside);
        }
        return shell(&line, slot_of(directory));
    }
    let named = ["file_path", "path"]
        .iter()
        .filter_map(|key| asking.input.get(*key)?.as_str());
    if named.into_iter().any(reaches_outside) {
        return Some(Because::ReachesOutside);
    }
    because(asking)
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
fn reaches_outside(word: &str) -> bool {
    let word = word.rsplit('=').next().unwrap_or(word);
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.to_lowercase().contains("armada") {
        return false;
    }
    let root = word.trim_end_matches('/');
    (root.is_empty() && word.starts_with('/'))
        || matches!(root, "/Users" | "/Volumes")
        || word.starts_with('~')
        || word.contains("$HOME")
        || word.contains("${HOME}")
        || word.starts_with("/Users/")
        || word.starts_with("/Volumes/")
}

/// `directory` when it is a slot. Before its first lease a session runs in
/// the repository's root, the owner's own checkout, and nothing is taken away
/// there unasked.
fn slot_of(directory: &str) -> Option<&str> {
    directory.contains("/.armada/slots/").then_some(directory)
}

/// Whether `word`, a path a shell line removes or overwrites, is inside
/// `slot`. **Removing a file in the slot it holds is the session's own work**
/// (the owner, 8 Oct 2026); its root, a `..`, a `~` or a `$` is never read as
/// inside, and a relative path only where the line has not moved with `cd`.
fn in_the_slot(word: &str, slot: &str, relative: bool) -> bool {
    let word = word.trim_matches(|c| c == '\'' || c == '"');
    if word.is_empty() || word.contains('$') || word.starts_with('~') {
        return false;
    }
    if word.starts_with('/') {
        return inside_the_directory(word, slot)
            && word.trim_end_matches('/') != slot.trim_end_matches('/');
    }
    relative
        && !matches!(word.trim_end_matches('/'), "" | "." | "*")
        && inside_the_directory(word, slot)
}

/// Whether `path` is under `directory`, a relative path being read against it.
/// A `..` anywhere makes it unreadable as inside.
fn inside_the_directory(path: &str, directory: &str) -> bool {
    if path.split('/').any(|part| part == "..") {
        return false;
    }
    let directory = directory.trim_end_matches('/');
    !path.starts_with('/') || path == directory || path.starts_with(&format!("{directory}/"))
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
/// asks only where it is one of the three: pausing a Job, adding a task,
/// writing a Studio note and starting a local run are none of them.
fn door(operation: &str, asking: &AskingToRun) -> Option<Because> {
    const SHARED: &[&str] = &[
        "approve_dispatch",
        "approve_review",
        "merge_pull_request",
        // A pull request by number is nobody's Job and the same act: the
        // merge, the draft taken out of draft, and the standing instruction
        // to merge. All three change what other people read.
        "merge_pull_request_by_number",
        "ready_pull_request",
        "enable_auto_merge",
        "redispatch_job",
        "dispatch_studio_draft",
        "request_changes",
        // Two of a pilot's exits: the work goes on toward delivery, or a person
        // says finished work is finished with no gate run.
        "submit_for_verification",
        "attest_complete",
    ];
    const GONE: &[&str] = &[
        "kill_job",
        "kill_drone",
        "kill_one_drone",
        // Taking a Job over ends its Drone, and closing one as superseded ends
        // the Job.
        "take_over",
        "close_as_superseded",
        "kill_process",
        "kill_processes",
        "forget_job",
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
        "rerun_gate",
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
fn shell(line: &str, slot: Option<&str>) -> Option<Because> {
    let line = line.trim();
    if line.is_empty() {
        return Some(Because::Unreadable);
    }
    if line.contains('`') || line.contains("$(") {
        return Some(Because::Unreadable);
    }
    // Every class at once, and never worth reading further.
    if word_in(line, "sudo") || word_in(line, "doas") {
        return Some(Because::Destructive);
    }
    // **`&&` and `||` become separators and a lone `&` does not.** Splitting on
    // every `&` cuts `2>&1` in half and leaves a segment ending in a bare `>`,
    // which then reads as a truncating redirect — `cargo build 2>&1 | tail -5`
    // asked, as a destructive line, until a case caught it.
    let chained = line.replace("&&", ";").replace("||", ";");
    let segments: Vec<&str> = chained
        .split(|c| c == ';' || c == '|' || c == '\n')
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .collect();
    // A `cd` into the slot keeps a relative path inside it; one anywhere else,
    // or one that cannot be read, loses track of where the line is.
    let slot = slot.map(|slot| Slot {
        slot,
        relative: segments.iter().all(|said| {
            let mut words = said.split_whitespace();
            match words.next() {
                Some("cd" | "pushd") => words.next().is_some_and(|to| in_the_slot(to, slot, true)),
                Some("popd") => false,
                _ => true,
            }
        }),
    });
    segments.into_iter().find_map(|said| segment(said, slot))
}

/// Whether `word` appears in `line` as a word rather than inside a longer one,
/// so `evaluate.sh` is not an `eval`.
fn word_in(line: &str, word: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .any(|said| said == word)
}

/// The slot a session's line runs in, and whether a relative path in it is
/// still read against the slot.
#[derive(Clone, Copy)]
struct Slot<'a> {
    slot: &'a str,
    relative: bool,
}

impl Slot<'_> {
    fn holds(&self, word: &str) -> bool {
        in_the_slot(word, self.slot, self.relative)
    }
}

/// One segment of a shell line, already split from the rest.
fn segment(said: &str, slot: Option<Slot>) -> Option<Because> {
    if truncating_redirect(said)
        && !slot.is_some_and(|slot| {
            redirect_targets(said)
                .iter()
                .all(|target| slot.holds(target))
        })
    {
        return Some(Because::Destructive);
    }
    // The command is the first word that is not an assignment or a leading
    // `env`, since `FOO=1 rm -rf x` is an `rm`.
    let mut rest = said
        .split_whitespace()
        .skip_while(|word| word.contains('=') || *word == "env" || *word == "command")
        .peekable();
    let program = rest.next()?;
    let program = program.rsplit('/').next().unwrap_or(program);
    // **The shell builtins, and only in the position that makes them one.**
    // `pnpm --dir x exec vitest run` carries `exec` as a subcommand and is the
    // ordinary way this repository runs its tests; reading the word anywhere on
    // the line made every one of them ask.
    if program == "eval" || program == "exec" {
        return Some(Because::Unreadable);
    }
    let arguments: Vec<&str> = rest.collect();
    match program {
        "git" => return git(&arguments),
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
fn truncating_redirect(said: &str) -> bool {
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
fn git(arguments: &[&str]) -> Option<Because> {
    let subcommand = arguments.iter().find(|word| !word.starts_with('-'))?;
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
        "pr" | "issue" if matches!(verb, "close" | "delete" | "reopen" | "edit") => {
            Some(Because::Destructive)
        }
        "issue" if matches!(verb, "create" | "comment") => Some(Because::WritesOffMachine),
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
