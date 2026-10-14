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
/// directory `git` is pointed at (`-C`, `--work-tree`, `--git-dir`) for anything that is not a
/// read. `cwd` is where the call started, and a path that climbs with a `..` is read against it.
/// **Not a shell parser**: a path built from a variable, a relative path with no `..` (bar
/// `armada worktree release`'s) and anything inside a script it runs are not seen. **A line that
/// runs an interpreter is read for every slot path in its text**, since what the interpreter
/// writes cannot be read from the line.
pub(super) fn written_by(line: &str, cwd: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut interpreted = false;
    // Where the line stands: `cwd` until a `cd`, then where the `cd` left it. A place this cannot
    // follow is empty, and a relative path is read against nothing.
    let mut cwd = cwd.to_string();
    for segment in line
        .replace("&&", ";")
        .replace("||", ";")
        .split([';', '|', '\n'])
    {
        let words: Vec<String> = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '\'' || c == '"').to_string())
            .collect();
        let bare: Vec<&str> = words
            .iter()
            .enumerate()
            .map(|(at, word)| match at {
                0 => word.trim_start_matches(['(', '{']),
                _ => word.as_str(),
            })
            .collect();
        if matches!(bare.first().copied(), Some("cd" | "pushd")) {
            cwd = after_cd(&bare, Some(&cwd)).unwrap_or_default();
            continue;
        }
        redirects(&words, &cwd, &mut found);
        let mut rest = words
            .iter()
            .skip_while(|word| word.contains('=') || *word == "env" || *word == "command");
        let Some(program) = rest.next() else { continue };
        let program = program.rsplit('/').next().unwrap_or(program);
        interpreted |= interprets(program);
        let arguments: Vec<&String> = rest.collect();
        let absolute = |found: &mut Vec<String>, from: &[&String]| {
            found.extend(
                from.iter()
                    .filter(|word| !word.starts_with('-'))
                    .filter_map(|word| resolved(word, &cwd)),
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
            "armada" => found.extend(released_by(&arguments, &cwd)),
            "git" => found.extend(git_writes_in(&arguments, &cwd)),
            _ => {}
        }
    }
    if interpreted {
        found.extend(slot_paths_in(line));
    }
    found
}

/// `path` as an absolute path: itself, or, where it begins with a `..`, read against `cwd`.
fn resolved(path: &str, cwd: &str) -> Option<String> {
    match path {
        absolute if absolute.starts_with('/') => Some(absolute.to_string()),
        climbing if cwd.starts_with('/') && (climbing == ".." || climbing.starts_with("../")) => {
            Some(normalised(&format!("{}/{climbing}", cwd.trim_end_matches('/'))))
        }
        _ => None,
    }
}

/// Programs that run code written in the line or in a file it names.
fn interprets(program: &str) -> bool {
    const OTHERS: &[&str] = &[
        "node", "nodejs", "deno", "bun", "ruby", "perl", "php", "lua", "osascript", "Rscript",
        "sh", "bash", "zsh", "dash",
    ];
    program.starts_with("python") || OTHERS.contains(&program)
}

/// Every path in `text` that is in a slot, wherever it sits: in a quote, a call, an argument.
fn slot_paths_in(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "/._-+@:~".contains(c)))
        .filter(|piece| piece.starts_with('/') && piece.contains("/.armada/slots/"))
        .map(str::to_string)
        .collect()
}

/// The directories a `git` is pointed at with `-C`, `--work-tree` or `--git-dir`, where its
/// subcommand is not a read. Read against `cwd` when they climb.
fn git_writes_in(arguments: &[&String], cwd: &str) -> Vec<String> {
    const TAKES_A_VALUE: &[&str] = &["-C", "-c", "--work-tree", "--git-dir", "--namespace"];
    let mut pointed = Vec::new();
    let mut subcommand = None;
    let mut words = arguments.iter().map(|word| word.as_str());
    while let Some(word) = words.next() {
        if TAKES_A_VALUE.contains(&word) {
            let value = words.next();
            if matches!(word, "-C" | "--work-tree" | "--git-dir") {
                pointed.extend(value);
            }
        } else if let Some(value) = word
            .strip_prefix("--work-tree=")
            .or_else(|| word.strip_prefix("--git-dir="))
        {
            pointed.push(value);
        } else if !word.starts_with('-') {
            subcommand = Some(word);
            break;
        }
    }
    if subcommand.is_some_and(|word| GIT_READS.contains(&word)) {
        return Vec::new();
    }
    pointed
        .into_iter()
        .filter_map(|path| match path.starts_with('/') {
            true => Some(path.to_string()),
            false if cwd.starts_with('/') => {
                Some(normalised(&format!("{}/{path}", cwd.trim_end_matches('/'))))
            }
            false => None,
        })
        .collect()
}

/// The slot `armada worktree release <path>` gives back, the path read against `cwd`. **Giving a
/// slot back writes it**, and it is whichever slot that is, not the caller's own. One that names
/// no path gives back the directory it runs in, which [`worked_in`] reads.
fn released_by(arguments: &[&String], cwd: &str) -> Option<String> {
    let mut words = arguments.iter().map(|word| word.as_str()).filter(|word| !word.starts_with('-'));
    if words.next() != Some("worktree") || words.next() != Some("release") {
        return None;
    }
    match words.next()? {
        path if path.starts_with('/') => Some(path.to_string()),
        path if cwd.starts_with('/') => Some(format!("{}/{path}", cwd.trim_end_matches('/'))),
        _ => None,
    }
}

/// Where a `cd` or `pushd` leaves a line that stood in `from`: an absolute target as it is, a
/// relative one read against `from`, so `cd a && cd b` ends in `a/b`. `None` when the target is
/// built from a variable, a `~` or a substitution, is `-` or is missing, or is relative and
/// `from` is not known; a line that has lost its place reads nothing relative after.
fn after_cd(words: &[&str], from: Option<&str>) -> Option<String> {
    let target = words.iter().skip(1).find(|word| !word.starts_with('-'))?;
    match target {
        word if word.contains(['$', '`', '~']) => None,
        word if word.starts_with('/') => Some(normalised(word)),
        word => from
            .filter(|base| base.starts_with('/'))
            .map(|base| normalised(&format!("{}/{word}", base.trim_end_matches('/')))),
    }
}

/// The absolute directories a shell line `cd`s or `pushd`s into and then runs something that is
/// not a read in, where `cwd` is the directory the call started in. **A `cd` moves every relative
/// write after it**, which no path in the line names, and a relative `cd` after another is read
/// from where the first left the line. A target built from a variable, a `~` or a
/// substitution is not followed, and neither is a subshell's end. A bare `armada worktree
/// release` with no `cd` before it gives back `cwd`.
pub(super) fn worked_in(line: &str, cwd: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut here = Some(cwd.to_string());
    let mut moved = false;
    for segment in line
        .replace("&&", ";")
        .replace("||", ";")
        .split([';', '|', '\n'])
    {
        let segment = segment.trim().trim_start_matches(['(', '{']).trim();
        let words: Vec<&str> = segment
            .split_whitespace()
            .map(|word| word.trim_matches(|c| c == '\'' || c == '"'))
            .collect();
        if matches!(words.first().copied(), Some("cd" | "pushd")) {
            here = after_cd(&words, here.as_deref());
            moved = true;
        } else if moved {
            if let Some(directory) = &here {
                if !adapters::reads_only(segment) && !found.contains(directory) {
                    found.push(directory.clone());
                }
            }
        } else if words.starts_with(&["armada", "worktree", "release"]) && words.len() == 3 {
            // Giving back the directory the call started in.
            if cwd.starts_with('/') && !found.iter().any(|one| one == cwd) {
                found.push(normalised(cwd));
            }
        }
    }
    found
}

/// The directory a call started in, where a line runs something that is not a read before it
/// `cd`s anywhere. **A working directory persists between a session's calls**, so a session that
/// went into a slot once is standing in it at every call after, with no path in the line to read.
pub(super) fn run_from(line: &str, cwd: &str) -> Option<String> {
    if !cwd.starts_with('/') {
        return None;
    }
    for segment in line
        .replace("&&", ";")
        .replace("||", ";")
        .split([';', '|', '\n'])
    {
        let segment = segment.trim().trim_start_matches(['(', '{']).trim();
        if matches!(segment.split_whitespace().next(), Some("cd" | "pushd")) {
            return None;
        }
        if !adapters::reads_only(segment) {
            return Some(normalised(cwd));
        }
    }
    None
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
fn redirects(words: &[String], cwd: &str, found: &mut Vec<String>) {
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
        if !target.starts_with("/dev/") {
            found.extend(resolved(target, cwd));
        }
    }
}
