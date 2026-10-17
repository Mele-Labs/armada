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

/// Paths a terminal's mod named from its `cwd`, as the worktree names them. A path that climbs out
/// of the directory, or is absolute, is not one the mod should have sent and is dropped.
pub(crate) fn inside_worktree(worktree: &str, cwd: &str, paths: &[String]) -> Vec<String> {
    let from = cwd.trim_end_matches('/').strip_prefix(worktree.trim_end_matches('/'));
    let Some(prefix) = from.map(|rest| rest.trim_matches('/')) else {
        return Vec::new();
    };
    paths
        .iter()
        .filter(|path| !path.starts_with('/') && !path.split('/').any(|part| part == ".."))
        .map(|path| match prefix.is_empty() {
            true => path.trim_start_matches("./").to_string(),
            false => format!("{prefix}/{}", path.trim_start_matches("./")),
        })
        .collect()
}
