// Included by `build.rs` and by `xtask`; expects `fs` and `Path` in scope.

/// The wire surface, hashed: Fleet's DTOs, the operation inventory, the route
/// table and Bridge's mirror of the DTOs. The same list, normalisation and hash are written in
/// `apps/desktop/codegen/protocol-id.mjs`; change one and change the other.
///
/// Comment-only lines are dropped so editing prose does not move the ID.
/// A comment after code on the same line still counts.
fn protocol_id(root: &Path) -> String {
    let mut files = Vec::new();
    collect(root, "crates/ipc/src", &mut files, &|rel| {
        rel.ends_with(".rs") && !rel.contains("/tests/") && !rel.ends_with("/tests.rs")
    });
    // The route table: the method and path each operation is served on.
    collect(root, "crates/api/src/routes", &mut files, &|rel| rel.ends_with(".rs"));
    collect(root, "crates/ipc/operations", &mut files, &|rel| {
        rel.ends_with(".toml") && !rel.ends_with("/_header.toml")
    });
    collect(root, "packages/protocol/src", &mut files, &|rel| {
        rel.ends_with(".ts")
            && !rel.ends_with(".test.ts")
            && !rel.ends_with("/connection.ts")
    });
    files.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for rel in files {
        let text = fs::read_to_string(root.join(&rel))
            .unwrap_or_else(|e| panic!("{rel} is part of the wire surface and could not be read: {e}"));
        let comment = if rel.ends_with(".toml") { "#" } else { "//" };
        feed(rel.as_bytes());
        feed(b"\n");
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.trim_start().starts_with(comment) {
                continue;
            }
            feed(line.as_bytes());
            feed(b"\n");
        }
        feed(b"\0");
    }
    format!("{hash:016x}")
}

/// Every file under `dir` that `keep` accepts, as repo-relative `/` paths.
/// A directory that is missing is a panic: an empty surface hashes to a
/// constant, and a constant ID is a gate that never refuses.
fn collect(root: &Path, dir: &str, into: &mut Vec<String>, keep: &dyn Fn(&str) -> bool) {
    let entries = fs::read_dir(root.join(dir))
        .unwrap_or_else(|e| panic!("{dir} is part of the wire surface and could not be read: {e}"));
    for entry in entries.filter_map(|entry| entry.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = format!("{dir}/{name}");
        if entry.path().is_dir() {
            collect(root, &rel, into, keep);
        } else if keep(&rel) {
            into.push(rel);
        }
    }
}
