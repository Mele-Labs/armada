//! What a shell command is for, from the first real command in it.
//!
//! A label for a turn's usage, never the command: `docs/concepts/session.md`, *Where the tokens went*.
//! Pure, so the table of cases in `tests/turn_lines.rs` is the whole of its specification.

use ipc::BashUse;

/// Words that read or search and change nothing.
const READ_SEARCH: &[&str] = &["grep", "sed", "cat", "rg", "ls", "wc", "head", "tail", "find", "awk", "jq"];
/// `git` subcommands that read.
const GIT_READS: &[&str] = &["grep", "show", "log", "diff", "ls-files"];
const SCRIPTS: &[&str] = &["python3", "python", "node"];
const BUILD_TEST: &[&str] = &["pnpm", "npx", "cargo", "taskpolicy", "time", "timeout"];
const LOOPS_WAITS: &[&str] = &["for", "while", "until", "sleep", "curl"];

/// The label for `command`.
pub fn classify_bash(command: &str) -> BashUse {
    let words = words_after_the_leading_cds(command);
    let Some((first, rest)) = words.split_first() else {
        return BashUse::Other;
    };
    let word = first.rsplit('/').next().unwrap_or(first);
    match word {
        "git" => match git_subcommand(rest) {
            Some(sub) if GIT_READS.contains(&sub) => BashUse::ReadSearch,
            _ => BashUse::Git,
        },
        "armada" if rest.iter().find(|one| !one.starts_with('-')) == Some(&"check") => BashUse::BuildTest,
        "armada" => BashUse::LoopsWaits,
        "gh" => BashUse::Forge,
        _ if READ_SEARCH.contains(&word) => BashUse::ReadSearch,
        _ if SCRIPTS.contains(&word) => BashUse::Scripts,
        _ if BUILD_TEST.contains(&word) => BashUse::BuildTest,
        _ if LOOPS_WAITS.contains(&word) => BashUse::LoopsWaits,
        _ => BashUse::Other,
    }
}

/// The command's words once every leading `cd <dir> &&` (or `;`) and every `NAME=value` prefix is gone.
fn words_after_the_leading_cds(command: &str) -> Vec<&str> {
    let mut rest = command.trim_start();
    while rest.starts_with("cd ") || rest == "cd" {
        let stop = rest.find(['&', ';', '\n']);
        let Some(at) = stop else {
            return Vec::new();
        };
        rest = rest[at..].trim_start_matches(['&', ';', '\n', ' ', '\t']);
    }
    let mut words = rest.split_whitespace().peekable();
    while words.next_if(|word| is_assignment(word)).is_some() {}
    words.collect()
}

fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// The word after `git` that names what it does, past `-C <dir>`, `-c <key=value>` and other flags.
fn git_subcommand<'a>(words: &[&'a str]) -> Option<&'a str> {
    let mut words = words.iter().copied();
    while let Some(word) = words.next() {
        match word {
            "-C" | "-c" | "--git-dir" | "--work-tree" => {
                words.next();
            }
            flag if flag.starts_with('-') => {}
            sub => return Some(sub),
        }
    }
    None
}
