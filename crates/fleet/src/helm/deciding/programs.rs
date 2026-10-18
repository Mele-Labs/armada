//! The programs a segment can run that change something beyond the slot: `git`, the forge CLI, fetching and publishing.

use super::*;

/// `git`, by its subcommand. **Reading history is not writing it**: `log`,
/// `status`, `diff`, `show` and `worktree list` run.
pub(super) fn git(arguments: &[&str], slot: Option<Slot>, own: Own) -> Option<Because> {
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
pub(super) fn kills_only_its_own(arguments: &[&str], own: Own) -> bool {
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
pub(super) fn deletes_only_its_own(arguments: &[&str], own: Own) -> bool {
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

pub(super) fn takes_something_away(word: &str) -> bool {
    matches!(
        word,
        "-D" | "-d" | "--delete" | "-f" | "--force" | "remove" | "prune" | "rm"
    )
}

/// The code host's own CLI, by its subcommand pair. **A read of somebody
/// else's repository is still a read**: `view`, `list`, `diff`, `status` and
/// `checks` run.
pub(super) fn forge(arguments: &[&str]) -> Option<Because> {
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
pub(super) fn fetching(arguments: &[&str]) -> Option<Because> {
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
pub(super) fn publishing(program: &str, arguments: &[&str]) -> bool {
    const MANAGERS: &[&str] = &[
        "cargo", "npm", "pnpm", "yarn", "gem", "pip", "pip3", "twine",
    ];
    MANAGERS.contains(&program) && arguments.iter().any(|word| *word == "publish")
}
