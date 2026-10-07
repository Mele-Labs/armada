//! Where a path lands, as far as a hosted session's gate can tell: the main
//! checkout, one slot of the pool, or somewhere else. Since 23.71.
//!
//! **A slot lives under the main checkout's path** (`<root>/.armada/slots/`),
//! so "inside the repository" is not "the main checkout". `docs/concepts/
//! session.md`, *What the write gate covers*.

/// What a path is, against the repository's root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    Main,
    Slot(u32),
    Elsewhere,
}

/// `path` with `.` and `..` resolved by reading it, never by touching disk.
/// A relative path is returned as it came.
pub(super) fn normalised(path: &str) -> String {
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

pub(super) fn inside(path: &str, directory: &str) -> bool {
    let directory = directory.trim_end_matches('/');
    path == directory || path.starts_with(&format!("{directory}/"))
}

pub(super) fn place_of(path: &str, root: &str) -> Place {
    let path = normalised(path);
    let root = root.trim_end_matches('/');
    if !inside(&path, root) {
        return Place::Elsewhere;
    }
    let slots = format!("{root}/.armada/slots/");
    let number = path
        .strip_prefix(&slots)
        .and_then(|rest| rest.split('/').next())
        .and_then(|name| name.strip_prefix("slot-"))
        .and_then(|number| number.parse().ok());
    match number {
        Some(number) => Place::Slot(number),
        None => Place::Main,
    }
}

/// The absolute paths a shell line obviously writes: a redirect's target, the
/// arguments of a command whose job is to write, `sed -i`'s files, and the
/// directory `git -C` writes in. **Not a shell parser**: a path built from a
/// variable, a relative path and anything inside a script it runs are not seen.
pub(super) fn written_by(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    for segment in line
        .replace("&&", ";")
        .replace("||", ";")
        .split([';', '|', '\n'])
    {
        let words: Vec<String> = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '\'' || c == '"').to_string())
            .collect();
        redirects(&words, &mut found);
        let mut rest = words
            .iter()
            .skip_while(|word| word.contains('=') || *word == "env" || *word == "command");
        let Some(program) = rest.next() else { continue };
        let program = program.rsplit('/').next().unwrap_or(program);
        let arguments: Vec<&String> = rest.collect();
        let absolute = |found: &mut Vec<String>, from: &[&String]| {
            found.extend(
                from.iter()
                    .filter(|word| word.starts_with('/'))
                    .map(|word| word.to_string()),
            )
        };
        match program {
            "tee" | "cp" | "mv" | "install" | "ln" | "rsync" | "touch" | "mkdir" | "rm"
            | "rmdir" | "truncate" | "dd" | "chmod" => absolute(&mut found, &arguments),
            "sed" | "perl" | "gsed"
                if arguments
                    .iter()
                    .any(|word| word.starts_with("-i") || word.starts_with("-pi")) =>
            {
                absolute(&mut found, &arguments)
            }
            "git" => {
                let at = arguments.iter().position(|word| word.as_str() == "-C");
                if let Some(path) = at.and_then(|at| arguments.get(at + 1)) {
                    let subcommand = arguments
                        .iter()
                        .skip(at.unwrap_or(0) + 2)
                        .find(|word| !word.starts_with('-'));
                    if !subcommand.is_some_and(|word| GIT_READS.contains(&word.as_str())) {
                        found.push(path.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    found
}

const GIT_READS: &[&str] = &[
    "status",
    "log",
    "diff",
    "show",
    "rev-parse",
    "ls-files",
    "blame",
    "describe",
    "fetch",
    "branch",
    "remote",
    "worktree",
    "config",
    "grep",
    "ls-tree",
    "cat-file",
];

/// `>file`, `>> file`, `2>file`, `&>file`, and a `>` standing alone.
fn redirects(words: &[String], found: &mut Vec<String>) {
    for (at, word) in words.iter().enumerate() {
        let Some(arrow) = word.find('>') else {
            continue;
        };
        let after = word[arrow..]
            .trim_start_matches('>')
            .trim_start_matches('|');
        let target = if after.is_empty() {
            words.get(at + 1).map(String::as_str).unwrap_or_default()
        } else {
            after
        };
        if target.starts_with('/') && !target.starts_with("/dev/") {
            found.push(target.to_string());
        }
    }
}
