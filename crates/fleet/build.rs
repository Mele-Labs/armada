//! Lists the test modules, so adding one is adding a file.
//!
//! `src/tests/mod.rs` was one line a module, and two branches that each added a
//! test conflicted on it eleven times in three weeks. A pull request is merged by the
//! forge, which ignores `.gitattributes`, so a union merge could not be what kept
//! them apart. Every `src/tests/*.rs` is a module now, declared by
//! `src/tests/mods.inc`, which this writes and git ignores. A module `mod.rs`
//! declares itself (the few that are `pub(crate)`) is left to it.
//!
//! The list is written beside `mod.rs` rather than into `OUT_DIR`, because a
//! `mod x;` in an included file is looked for beside the included file: an
//! `OUT_DIR` copy would need an absolute path, which a target directory carried
//! to another worktree keeps pointing at the old tree.

use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/tests");
    let dir = Path::new("src/tests");
    let declared = fs::read_to_string(dir.join("mod.rs")).unwrap_or_default();
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("src/tests")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|file| file.strip_suffix(".rs").map(str::to_string))
        .filter(|name| name != "mod")
        .filter(|name| !declared.contains(&format!("mod {name};")))
        .collect();
    names.sort();
    let list: String = names.iter().map(|name| format!("mod {name};\n")).collect();
    let file = dir.join("mods.inc");
    // Only when it changed: a write moves the directory's mtime, which would
    // rerun this on the next build.
    if fs::read_to_string(&file).ok().as_deref() != Some(list.as_str()) {
        fs::write(&file, list).expect("src/tests is writable");
    }
}
