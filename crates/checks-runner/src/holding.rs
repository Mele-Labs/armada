//! Which members of a workspace could hold a test of a given name, so a run of
//! one test compiles those and not the workspace.
//!
//! **Read off the source, not out of `cargo`.** Asking cargo which package has
//! a test of this name means building every test binary to list them, which is
//! the cost this exists to avoid. A member holds a name when a file in it
//! declares `fn <name>` or `mod <name>`; that over-includes (a commented-out
//! `fn`, a helper of the same name) and never under-includes, so the worst case
//! is a member compiled that did not need to be.
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
    let mut held: Vec<String> = members
        .filter_map(Result::ok)
        .filter(|member| member.path().is_dir())
        .filter(|member| declares(&member.path(), leaf))
        .filter_map(|member| member.file_name().into_string().ok())
        .collect();
    held.sort_unstable();
    held
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

/// Whether any `.rs` file under `dir` declares `leaf`. `target` and hidden
/// directories are not source.
fn declares(dir: &Path, leaf: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            return name != "target" && !name.starts_with('.') && declares(&path, leaf);
        }
        name.ends_with(".rs") && fs::read_to_string(&path).is_ok_and(|text| defines(&text, leaf))
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
