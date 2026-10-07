//! Reads `crates/store/migrations/`: one `.sql` file a migration, so two
//! branches that each add one never touch the same file. Shared by `build.rs`,
//! which turns the directory into the list at compile time, and by the tests.
//! Std only, because a build script has no dependencies.

use std::fs;
use std::path::{Path, PathBuf};

pub struct FileMigration {
    /// `module.slug`, the part of the file name after the timestamp.
    pub name: String,
    /// A `-- breaking` line in the leading comments; additive without one.
    pub breaking: bool,
    pub path: PathBuf,
}

/// The files of `dir` ordered by file name, which is by timestamp. A missing
/// directory is an empty list. A file that does not follow the naming is an
/// error, so a misnamed migration fails the build instead of being skipped.
pub fn read_dir(dir: &Path) -> Result<Vec<FileMigration>, String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sql"))
        .collect();
    files.sort();
    let mut found: Vec<FileMigration> = Vec::new();
    for path in files {
        let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name = name_of(file)?;
        if found.iter().any(|m| m.name == name) {
            return Err(format!("{file}: the name {name} is used by another file"));
        }
        let sql = fs::read_to_string(&path).map_err(|e| format!("{file}: {e}"))?;
        let breaking = sql
            .lines()
            .take_while(|l| l.trim().is_empty() || l.trim_start().starts_with("--"))
            .any(|l| l.trim() == "-- breaking");
        found.push(FileMigration {
            name,
            breaking,
            path,
        });
    }
    Ok(found)
}

/// `20261007T0130Z-main_ci.state.sql` is named `main_ci.state`.
fn name_of(file: &str) -> Result<String, String> {
    let bad = |why: &str| {
        format!("{file}: {why}; a migration file is <UTC yyyymmddThhmmZ>-<module.slug>.sql")
    };
    let stem = file.strip_suffix(".sql").ok_or_else(|| bad("not .sql"))?;
    let (stamp, name) = stem.split_once('-').ok_or_else(|| bad("no timestamp"))?;
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let stamped = stamp.len() == 14
        && digits(&stamp[..8])
        && &stamp[8..9] == "T"
        && digits(&stamp[9..13])
        && stamp.ends_with('Z');
    if !stamped {
        return Err(bad("the timestamp is not yyyymmddThhmmZ"));
    }
    let shaped = name.contains('.')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
    if !shaped {
        return Err(bad("the name is not lowercase module.slug"));
    }
    Ok(name.to_string())
}
