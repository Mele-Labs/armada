//! The tracked files of a checkout that hold any of some runs of words, as written and ignoring
//! case. What `send_session_note` reads the screen's words against.
//!
//! **`git grep -F`, so no new dependency** and only what the repository tracks: nothing ignored,
//! nothing built, nothing a dependency manager fetched.

use std::path::Path;

use crate::delivery::{read_in, said};

/// The most files a search answers, in git's path order. A run as common as a word would
/// otherwise have every file of a large repository read.
pub const MOST_FILES: usize = 400;

/// Git would not answer. The note is still sent, with no candidates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotSearched(pub String);

impl std::fmt::Display for NotSearched {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&self.0)
    }
}

/// Every tracked file in `checkout` holding one of `runs`, relative to it, at most
/// [`MOST_FILES`]. Binary files are passed over.
pub fn files_holding(checkout: &Path, runs: &[String]) -> Result<Vec<String>, NotSearched> {
    if runs.is_empty() {
        return Ok(Vec::new());
    }
    let mut args: Vec<&str> = vec!["grep", "-l", "-z", "-I", "-F", "-i", "--no-color"];
    for run in runs {
        args.push("-e");
        args.push(run);
    }
    let ran = read_in(&checkout.to_string_lossy(), "git", &args)
        .map_err(|why| NotSearched(format!("git grep could not run: {why}")))?;
    // One is git grep's answer for nothing found.
    match ran.status.code() {
        Some(0) => {}
        Some(1) => return Ok(Vec::new()),
        _ => return Err(NotSearched(format!("git grep: {}", said(&ran)))),
    }
    Ok(String::from_utf8_lossy(&ran.stdout)
        .split('\0')
        .filter(|name| !name.is_empty())
        .take(MOST_FILES)
        .map(str::to_string)
        .collect())
}
