//! Where a failed Check's log says it failed, read once as the run ends. `ipc::CheckBlame`.
//!
//! **The formats are the tools' own**: rustc's `-->` under an `error`, a Rust panic's location,
//! vitest's `❯` frames and `FAIL` header, a node `at` frame, and tsc's `file(12,5): error` and
//! `file:12:5 - error`. A nextest summary names a test and no file; its panic lines are read.
//!
//! **A source file wins over a test file** wherever the log names both, so a vitest frame inside
//! the code under test, or a panic outside `tests/`, is what is blamed. Only a file that is in the
//! slot is kept: an absolute path elsewhere, a `..` path or one that is not on disk names nothing.

use std::path::{Path, PathBuf};

use ipc::CheckBlame;

/// How much of a log's end is read. A failure's report is at the end, and a huge log costs little.
pub const TAIL: usize = 64 * 1024;

/// One location the log names, in the order it names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spot {
    pub path: String,
    pub line: Option<u32>,
}

/// The file a failed run's log blames, relative to `slot`. `None` where it names none in the slot.
pub fn blame_of(log: &str, slot: &Path) -> Option<CheckBlame> {
    let mut place = Placer::new(slot);
    let placed: Vec<Spot> = spots(tail(log)).into_iter().filter_map(|spot| place.of(spot)).collect();
    let chosen = placed.iter().find(|one| !is_test(&one.path)).or_else(|| placed.first())?;
    // A header names the file without a line, and a frame below it names the line.
    let line = chosen.line.or_else(|| placed.iter().filter(|one| one.path == chosen.path).find_map(|one| one.line));
    Some(CheckBlame { path: chosen.path.clone(), line })
}

fn tail(log: &str) -> &str {
    if log.len() <= TAIL {
        return log;
    }
    let mut from = log.len() - TAIL;
    while !log.is_char_boundary(from) {
        from += 1;
    }
    &log[from..]
}

/// Every location the log names a failure at, as written, in order.
pub fn spots(log: &str) -> Vec<Spot> {
    let mut out = Vec::new();
    // rustc's `-->` follows a warning as often as an error, and only an error's is a failure.
    let mut in_error = false;
    for raw in log.lines() {
        let line = plain(raw);
        let text = unprefixed(&line).trim();
        if text.starts_with("error") {
            in_error = true;
        } else if text.starts_with("warning") {
            in_error = false;
        }
        if let Some(rest) = text.strip_prefix("--> ") {
            if in_error {
                out.extend(located(rest));
            }
        } else if let Some(at) = text.find("panicked at ") {
            out.extend(text[at + "panicked at ".len()..].split_whitespace().find_map(located));
        } else if text.contains("): error TS") || text.contains(" - error TS") {
            out.extend(text.split_whitespace().find_map(|word| tsc(word).or_else(|| located(word))));
        } else if let Some(at) = text.find("❯ ") {
            out.extend(text[at + "❯ ".len()..].split_whitespace().find_map(located));
        } else if let Some(rest) = text.strip_prefix("at ") {
            out.extend(rest.split_whitespace().find_map(located));
        } else if let Some(rest) = text.strip_prefix("FAIL ") {
            // vitest's header: the file, then ` > ` and the test's name.
            out.extend(rest.split_whitespace().next().and_then(|word| has_extension(word).then(|| Spot { path: word.to_string(), line: None })));
        }
    }
    out
}

/// A line without its colour codes.
fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for end in chars.by_ref() {
                if end.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A line without pnpm's `<package> <script>: ` prefix, which a recursive run writes before each.
fn unprefixed(line: &str) -> &str {
    let Some((head, rest)) = line.split_once(": ") else {
        return line;
    };
    let mut words = head.split(' ');
    match (words.next(), words.next(), words.next()) {
        (Some(package), Some(script), None) if package.contains('/') && !package.contains(':') && !script.is_empty() && !has_extension(package) => rest,
        _ => line,
    }
}

/// `path:line[:col]` in one word, wrapped in parentheses or quotes or not.
fn located(word: &str) -> Option<Spot> {
    let word = word.trim_start_matches(['(', '\'', '"', '`']).trim_end_matches([')', '\'', '"', '`', ',', ':', '.']);
    let word = word.strip_prefix("file://").unwrap_or(word);
    let mut from = 0;
    while let Some(i) = word[from..].find(':').map(|i| i + from) {
        let (path, rest) = (&word[..i], &word[i + 1..]);
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && has_extension(path) {
            return Some(Spot { path: path.to_string(), line: digits.parse().ok() });
        }
        from = i + 1;
    }
    None
}

/// tsc's `path(line,col):`.
fn tsc(word: &str) -> Option<Spot> {
    let (path, rest) = word.split_once('(')?;
    let line: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (has_extension(path) && !line.is_empty() && rest[line.len()..].starts_with(',')).then(|| Spot { path: path.to_string(), line: line.parse().ok() })
}

/// The last segment ends in `.ext`: a file, not a test's name or a crate.
fn has_extension(path: &str) -> bool {
    let last = path.rsplit('/').next().unwrap_or(path);
    match last.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) && !path.contains(char::is_whitespace),
        None => false,
    }
}

/// A test file by where it is or what it is called.
pub fn is_test(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let folder = path.split('/').rev().skip(1).any(|one| matches!(one, "tests" | "test" | "__tests__"));
    folder || name.contains(".test.") || name.contains(".spec.") || name.ends_with("_test.rs") || name == "tests.rs"
}

/// Turns a path as written into one relative to the slot, where it names a file there.
struct Placer<'a> {
    slot: &'a Path,
    real: Option<PathBuf>,
    /// The folders a tool may have written its paths from, found once: vitest's are relative to
    /// its package, not the repository.
    packages: Option<Vec<String>>,
}

impl<'a> Placer<'a> {
    fn new(slot: &'a Path) -> Placer<'a> {
        Placer { slot, real: slot.canonicalize().ok(), packages: None }
    }

    fn of(&mut self, spot: Spot) -> Option<Spot> {
        let relative = if spot.path.starts_with('/') {
            let path = Path::new(&spot.path);
            let inside = path.strip_prefix(self.slot).ok().or_else(|| self.real.as_deref().and_then(|real| path.strip_prefix(real).ok()))?;
            inside.to_str()?.to_string()
        } else {
            spot.path.trim_start_matches("./").to_string()
        };
        if relative.is_empty() || relative.split('/').any(|one| one == ".." || one == "node_modules") {
            return None;
        }
        if self.slot.join(&relative).is_file() {
            return Some(Spot { path: relative, line: spot.line });
        }
        let slot = self.slot;
        let packages = self.packages.get_or_insert_with(|| folders(slot));
        let mut found = packages.iter().filter(|one| slot.join(one).join(&relative).is_file());
        match (found.next(), found.next()) {
            (Some(one), None) => Some(Spot { path: format!("{one}/{relative}"), line: spot.line }),
            _ => None,
        }
    }
}

/// The slot's folders two deep, past hidden ones and build output.
fn folders(slot: &Path) -> Vec<String> {
    let skip = |name: &str| name.starts_with('.') || matches!(name, "node_modules" | "target" | "dist" | "out");
    let children = |dir: &Path| -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| !skip(name))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for top in children(slot) {
        for inner in children(&slot.join(&top)) {
            out.push(format!("{top}/{inner}"));
        }
        out.push(top);
    }
    out
}
