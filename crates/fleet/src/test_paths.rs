//! Which paths are a repository's tests, by convention. The one place the rail asks.

/// `*.test.*`, `*_test.*`, or anything under a `tests/` or `__tests__/` directory.
pub(crate) fn is_test_path(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let Some(file) = parts.pop() else { return false };
    if parts.iter().any(|dir| matches!(*dir, "tests" | "__tests__")) {
        return true;
    }
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    file.contains(".test.") || stem.ends_with("_test")
}
