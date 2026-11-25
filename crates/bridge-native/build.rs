//! Reads `packages/tokens` when the crate builds, so the app carries no copy of a look value.
//!
//! It writes one file into `OUT_DIR`: the token, theme and catalogue sources as `include_str!`
//! constants, and the Plex fonts as `include_bytes!` constants. The fonts are WOFF2 in the
//! repository and gpui's text system reads TrueType, so they are unpacked here, into `OUT_DIR` and
//! nowhere else.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest directory"));
    let tokens = manifest.join("..").join("..").join("packages").join("tokens");
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets the output directory"));

    for watched in ["tokens.json", "src/light.css", "themes/index.json", "themes/css", "fonts"] {
        println!("cargo:rerun-if-changed={}", tokens.join(watched).display());
    }
    println!("cargo:rerun-if-changed=build.rs");

    let mut table = String::new();
    table.push_str("// Written by build.rs from packages/tokens. Not a file in the tree.\n\n");
    for (name, relative) in [
        ("TOKENS_JSON", "tokens.json"),
        ("LIGHT_CSS", "src/light.css"),
        ("CATALOGUE_INDEX", "themes/index.json"),
    ] {
        table.push_str(&format!(
            "pub const {name}: &str = include_str!({:?});\n",
            absolute(&tokens.join(relative))
        ));
    }

    table.push_str("\n/// One generated block per catalogue theme, in file-name order.\npub const CATALOGUE_CSS: &[&str] = &[\n");
    for file in sorted(&tokens.join("themes").join("css"), "css") {
        table.push_str(&format!("    include_str!({:?}),\n", absolute(&file)));
    }
    table.push_str("];\n");

    // A constant for each token in `tokens.json`, so a view names `token::BG_BASE` and a misspelling
    // is a compile error. The file is generated one key to a line; its `"name"` lines are read as
    // such, and a name that does not look like `--word-word` stops the build.
    let listed = fs::read_to_string(tokens.join("tokens.json")).expect("tokens.json is read");
    table.push_str("\n/// One constant per token in `packages/tokens/tokens.json`.\npub mod token {\n    use crate::look::Token;\n");
    let mut seen = Vec::new();
    for line in listed.lines() {
        let Some(name) = line.trim().strip_prefix("\"name\": \"").and_then(|rest| rest.strip_suffix("\",")) else { continue };
        assert!(
            name.starts_with("--") && name[2..].chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'),
            "{name} is not a token name this build can make a constant of"
        );
        let constant = name[2..].replace('-', "_").to_ascii_uppercase();
        assert!(!seen.contains(&constant), "{name} makes a constant that another token already made");
        table.push_str(&format!("    pub const {constant}: Token = Token({name:?});\n"));
        seen.push(constant);
    }
    table.push_str("}\n");

    let fonts = out.join("fonts");
    fs::create_dir_all(&fonts).expect("the font directory is made");
    table.push_str("\n/// IBM Plex, as TrueType.\npub const FONTS: &[(&str, &[u8])] = &[\n");
    for file in sorted(&tokens.join("fonts"), "woff2") {
        let stem = file.file_stem().and_then(|stem| stem.to_str()).expect("a font file has a name");
        let packed = fs::read(&file).expect("the font is read");
        let unpacked = wuff::decompress_woff2(&packed).unwrap_or_else(|why| panic!("{stem} did not unpack: {why:?}"));
        let target = fonts.join(format!("{stem}.ttf"));
        fs::write(&target, unpacked).expect("the font is written");
        table.push_str(&format!("    ({stem:?}, include_bytes!({:?})),\n", absolute(&target)));
    }
    table.push_str("];\n");

    fs::write(out.join("look_sources.rs"), table).expect("the source table is written");
}

fn absolute(path: &Path) -> String {
    path.canonicalize().unwrap_or_else(|why| panic!("{} is missing: {why}", path.display())).display().to_string()
}

/// The files of one extension in a directory, by name, so the table does not depend on read order.
fn sorted(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|why| panic!("{} is unreadable: {why}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect();
    found.sort();
    found
}
