//! Which members of a workspace could hold a test of a given name, so a run of
//! one test compiles those and not the workspace.
//!
//! **Read off the source, not out of `cargo`.** Asking cargo which package has
//! a test of this name means building every test binary to list them, which is
//! the cost this exists to avoid. A member holds a name when a file of its test
//! source declares `fn <name>` or `mod <name>`, or is the module `<name>` itself.
//! Test source is a file with a test attribute or one under a `tests` module.
//! A plain `mod kit_files;` in `api` and `ipc` once had a `kit_files` run link
//! three test binaries for tests in one, and 274 of 882 passing agent runs of
//! 14 to 21 Nov 2026 built a member that ran nothing.
//!
//! Where no member holds the name in test source, as with tests a macro writes,
//! any declaration counts, so the worst case is still a member compiled that did
//! not need to be, never a test left out.
//!
//! The second Rust fact in this crate, beside [`reach`](crate::reach): what a
//! test's name looks like in source. [`narrow`](crate::narrowed) still knows
//! none.

use std::fs;
use std::path::Path;

/// Every member directly under `under` in `root` whose source declares a
/// function or module called the last segment of `test`, sorted.
///
/// `test` is whatever a person typed: `a_test`, `tests::servers::a_test`, or a
/// nextest line's `binary_id name` pair, of which the name is the last word. A
/// last segment that is not an identifier holds nothing, since no Rust source
/// declares one.
pub fn holders(root: &Path, under: &str, test: &str) -> Vec<String> {
    let Some(leaf) = leaf(test) else {
        return Vec::new();
    };
    let Ok(members) = fs::read_dir(root.join(under)) else {
        return Vec::new();
    };
    let mut members: Vec<(String, std::path::PathBuf)> = members
        .filter_map(Result::ok)
        .filter(|member| member.path().is_dir())
        .filter_map(|member| Some((member.file_name().into_string().ok()?, member.path())))
        .collect();
    members.sort_unstable();
    let holding = |in_tests: bool| -> Vec<String> {
        members
            .iter()
            .filter(|(_, dir)| declares(dir, leaf, in_tests, Down::default()))
            .map(|(name, _)| name.clone())
            .collect()
    };
    match holding(true) {
        none if none.is_empty() => holding(false),
        held => held,
    }
}

/// Whether `test` ends in the module `tests`, which every test file of a crate
/// sits under: the name selects them all, not one.
pub(crate) fn whole_suite(test: &str) -> bool {
    leaf(test) == Some("tests")
}

/// The identifier a name ends in, or [`None`] where it ends in anything else.
fn leaf(test: &str) -> Option<&str> {
    let word = test.split_whitespace().next_back()?;
    let leaf = word.rsplit("::").next()?;
    (!leaf.is_empty() && leaf.chars().all(identifier)).then_some(leaf)
}

fn identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether any `.rs` file under `dir` declares `leaf`, or, with `in_tests`,
/// whether a file of test source declares it or is the module `leaf` itself.
/// `target` and hidden directories are not source.
fn declares(dir: &Path, leaf: &str, in_tests: bool, down: Down) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            return name != "target"
                && !name.starts_with('.')
                && declares(
                    &path,
                    leaf,
                    in_tests,
                    Down {
                        named: down.named || name == leaf,
                        tests: down.tests || name == "tests",
                    },
                );
        }
        let Some(stem) = name.strip_suffix(".rs") else {
            return false;
        };
        let Ok(text) = fs::read_to_string(&path) else {
            return false;
        };
        if !in_tests {
            return defines(&text, leaf);
        }
        let test_source = has_test_attribute(&text) || down.tests || stem == "tests";
        test_source && (defines(&text, leaf) || down.named || stem == leaf)
    })
}

/// What the directories between a member and a file say about the file: one is
/// called the name asked for, or one is called `tests`.
#[derive(Clone, Copy, Default)]
struct Down {
    named: bool,
    tests: bool,
}

/// Whether `text` carries an attribute that makes a test: `#[test]`,
/// `#[tokio::test(...)]` and their kin, and not `#[cfg(test)]`.
fn has_test_attribute(text: &str) -> bool {
    text.match_indices("#[").any(|(at, _)| {
        let rest = &text[at + 2..];
        let path_len = rest.find(|c: char| !(identifier(c) || c == ':')).unwrap_or(rest.len());
        let path = &rest[..path_len];
        (path == "test" || path.ends_with("::test")) && rest[path_len..].starts_with([']', '('])
    })
}

/// Whether `text` has `fn leaf` or `mod leaf`, as whole words.
pub(crate) fn defines(text: &str, leaf: &str) -> bool {
    text.match_indices(leaf).any(|(at, _)| {
        let before = &text[..at];
        let word_ends = !text[at + leaf.len()..].starts_with(identifier);
        let keyword = before.trim_end();
        let spaced = keyword.len() < before.len();
        word_ends && spaced && (ends_with_word(keyword, "fn") || ends_with_word(keyword, "mod"))
    })
}

fn ends_with_word(text: &str, word: &str) -> bool {
    text.strip_suffix(word)
        .is_some_and(|rest| !rest.ends_with(identifier))
}
