//! Paths as the write table names them.

/// A path as the repository names it. The harness may spell it absolute or from the home
/// directory.
pub(crate) fn repository_relative(path: &str, worktree: &str, home: &str) -> String {
    let whole = match path.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home.trim_end_matches('/')),
        None => path.to_string(),
    };
    let root = format!("{}/", worktree.trim_end_matches('/'));
    match whole.strip_prefix(&root) {
        Some(inside) => inside.to_string(),
        None => whole,
    }
}
