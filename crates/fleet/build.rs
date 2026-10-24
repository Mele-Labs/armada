//! Lists the test modules, so adding one is adding a file, and compiles in the built-in mods.
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
use std::path::{Path, PathBuf};

fn main() {
    built_in_mods();
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

/// Where the built-in mods are, relative to this crate.
const BUILT_IN: &str = "../../packages/mods";
/// The files a mod can hold, `crate::mods`' four names. Anything else in a folder is not compiled in.
const MOD_FILES: [&str; 4] = ["mod.toml", "theme.css", "layout.json", "tracker.toml"];

/// Every folder under `packages/mods/`, its files as bytes, into `$OUT_DIR/built_in_mods.rs` for
/// `crate::mods::built_in`. **Written as literals, never as paths**, so a target directory carried
/// to another worktree holds no path into the old one; what they say is checked where they are
/// listed. A missing or empty folder is no built-in mod.
fn built_in_mods() {
    let place = Path::new(BUILT_IN);
    // Watched only where it is there: a missing path is stale on every build, and would rebuild Fleet each time.
    if place.is_dir() {
        println!("cargo:rerun-if-changed={BUILT_IN}");
    }
    let mut folders: Vec<(String, PathBuf)> = fs::read_dir(place)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
        .filter(|(name, _)| !name.starts_with('.'))
        .collect();
    folders.sort();
    let mut out = String::from("&[\n");
    for (name, folder) in folders {
        out.push_str(&format!("    ({name:?}, &[\n"));
        for file in MOD_FILES {
            if let Ok(bytes) = fs::read(folder.join(file)) {
                out.push_str(&format!("        ({file:?}, &b\"{}\"[..]),\n", bytes.escape_ascii()));
            }
        }
        out.push_str("    ]),\n");
    }
    out.push(']');
    let file = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("built_in_mods.rs");
    fs::write(file, out).expect("OUT_DIR is writable");
}
