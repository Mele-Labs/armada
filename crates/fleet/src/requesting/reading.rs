//! A request body read from a file the Job wrote, for [`SendRequest::body_file`](ipc::SendRequest).
//!
//! **Only from the Job's own places**: its scratch folder and its slot. A path is canonicalised
//! first, so `..` and a symlink out are judged by where they land and not by how they are spelled.

use std::io::Read;
use std::path::{Path, PathBuf};

/// The text of the file `named` names, where it is inside one of `roots` and at most `cap` bytes.
/// An `Err` says why not, and never quotes the file.
pub fn read_body(named: &str, roots: &[PathBuf], cap: usize) -> Result<String, String> {
    let named = named.trim();
    if named.is_empty() {
        return Err("`body_file` is empty".to_string());
    }
    let path = Path::new(named);
    if !path.is_absolute() {
        return Err(format!("`body_file` {named:?} is not an absolute path"));
    }
    let real = path.canonicalize().map_err(|why| format!("`body_file` {named:?} could not be read: {why}"))?;
    let inside = roots.iter().filter_map(|root| root.canonicalize().ok()).any(|root| real.starts_with(root));
    if !inside {
        return Err(format!("`body_file` {named:?} is outside the Job's scratch folder and slot"));
    }
    let file = std::fs::File::open(&real).map_err(|why| format!("`body_file` {named:?} could not be read: {why}"))?;
    let mut bytes = Vec::new();
    // One past the cap, so a file over it is told from one exactly at it.
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|why| format!("`body_file` {named:?} could not be read: {why}"))?;
    if bytes.len() > cap {
        return Err(format!("a request's body is at most {cap} bytes, and `body_file` {named:?} is more"));
    }
    String::from_utf8(bytes).map_err(|_| format!("`body_file` {named:?} is not text"))
}
