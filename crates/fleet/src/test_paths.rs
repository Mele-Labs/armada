//! Which paths are a repository's tests, by convention. The one place the rail asks.

/// `*.test.*`, `*_test.*`, or anything under a `tests/` or `__tests__/` directory. **A fake is no
/// test**: a `fake/` or `fakes/` directory, or a file named `fake-*`, only lets a crate compile.
pub(crate) fn is_test_path(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let Some(file) = parts.pop() else { return false };
    if parts.iter().any(|dir| matches!(*dir, "fake" | "fakes")) || file.starts_with("fake-") || file.starts_with("fake_") {
        return false;
    }
    if parts.iter().any(|dir| matches!(*dir, "tests" | "__tests__")) {
        return true;
    }
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    file.contains(".test.") || stem.ends_with("_test")
}

/// A page a step writes as its own work, under `.armada/documents/`: not an edit of the repository.
pub(crate) fn is_step_output(path: &str) -> bool {
    path.starts_with(".armada/documents/") || path.contains("/.armada/documents/")
}
