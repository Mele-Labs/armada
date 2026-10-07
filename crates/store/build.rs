//! Turns `migrations/*.sql` into `FROM_FILES`, in file-name order, for
//! `src/migrations.rs` to include. `docs/practices/store-migrations.md`.

#[path = "src/migration_files.rs"]
mod migration_files;

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let dir = root.join("migrations");
    println!("cargo:rerun-if-changed={}", dir.display());
    let files = migration_files::read_dir(&dir).unwrap_or_else(|why| panic!("migrations/: {why}"));
    let mut out = String::from("pub(crate) const FROM_FILES: &[Migration] = &[\n");
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.path.display());
        let kind = if file.breaking {
            "breaking"
        } else {
            "additive"
        };
        writeln!(
            out,
            "    Migration::{kind}({:?}, include_str!({:?})),",
            file.name,
            file.path.display().to_string()
        )
        .expect("a string");
    }
    out.push_str("];\n");
    let target = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets it"));
    std::fs::write(target.join("file_migrations.rs"), out).expect("the generated list");
}
