//! Rule: a list is a directory with one file an entry, named for its key.
//!
//! GitHub merges a pull request without reading `.gitattributes`, so a
//! `merge=union` line protects nothing there, and two branches that each add a
//! line to one file conflict. An entry is a file instead: two branches adding
//! two entries touch two files. `docs/practices/list-files.md`.
//!
//! **The key is the file's name**, so a rule that finds an entry by its name
//! and a reader that finds it by its table agree. A file holding two tables, or
//! a table whose key is not the file's name, is the old shared file again under
//! a new path. A `merge=union` line is refused for the reason it is no longer
//! needed: it still reads as protection and gives none.

use std::fs;
use std::path::Path;

use crate::Report;

#[cfg(test)]
mod tests;

/// Where each list is kept, and the table prefix an entry's key follows.
const LISTS: &[(&str, &str)] = &[
    ("crates/ipc/operations", "operations"),
    ("packages/icons/icons", "icons"),
    ("packages/icons/conventions", "conventions"),
];

/// The keys of the top-level tables in `text`, a `[[...]]` sub-table not counted.
fn tables(prefix: &str, text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            line.strip_prefix('[')?
                .strip_prefix(prefix)?
                .strip_prefix('.')
        })
        .filter_map(|rest| rest.strip_suffix(']'))
        .filter(|key| !key.starts_with('['))
        .map(|key| key.trim_matches('"').to_string())
        .filter(|key| !key.ends_with(".usage"))
        .collect()
}

/// Every list entry is one file, named for the one table it holds.
pub fn a_list_is_a_directory_of_entries(root: &Path) -> Report {
    let mut report = Report::new("every list entry is one file named for its key");

    if let Ok(attributes) = fs::read_to_string(root.join(".gitattributes")) {
        for (n, line) in attributes.lines().enumerate() {
            if !line.starts_with('#') && line.contains("merge=union") {
                report.fail(format!(
                    ".gitattributes:{} — `merge=union` is ignored by a pull request on GitHub. \
                     Make the list a directory of entries, one file each",
                    n + 1
                ));
            }
        }
    }

    for (dir, prefix) in LISTS {
        let Ok(read) = fs::read_dir(root.join(dir)) else {
            report.fail(format!("{dir} — a list kept as a directory, and not there"));
            continue;
        };
        for file in read.filter_map(|e| e.ok()) {
            let name = file.file_name().to_string_lossy().to_string();
            let Some(stem) = name.strip_suffix(".toml") else {
                continue;
            };
            if stem == "_header" {
                continue;
            }
            let Ok(text) = fs::read_to_string(file.path()) else {
                report.fail(format!("{dir}/{name} — unreadable"));
                continue;
            };
            let keys = tables(prefix, &text);
            if keys != [stem.to_string()] {
                report.fail(format!(
                    "{dir}/{name} — holds `[{prefix}.<key>]` tables {keys:?}, and an entry is one \
                     file with the one table its name gives: `[{prefix}.{stem}]`"
                ));
            }
        }
    }
    report
}
