//! Rule: a list file declared `merge=union` holds entries and nothing else.
//!
//! `.gitattributes` is the one place a repository declares its list files, so
//! the declaration is read from there rather than from a second list. Git then
//! keeps both sides of a conflict in one: two branches that each append a line
//! merge with no conflict, and `armada land`'s candidate worktree does the same
//! because it runs plain `git merge`. Decided 2 Oct 2026 for #1059.
//!
//! **Keeping both sides is only right where every line stands alone.** A file
//! that mixes entries with other code interleaves two edits unseen, so the gate
//! names any line that is not an entry. **A deletion is not covered**: both
//! sides of a deletion are kept too, so two branches deleting different entries
//! put them back. `packages/protocol/src/pending.ts` is such a file, and is
//! not declared.

use std::fs;
use std::path::Path;

use crate::Report;

#[cfg(test)]
mod tests;

/// The paths `.gitattributes` gives `merge=union`.
fn declared(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let path = parts.next().filter(|p| !p.starts_with('#'))?;
            parts
                .any(|attr| attr == "merge=union")
                .then(|| path.to_string())
        })
        .collect()
}

/// The 1-based lines of `text` that are not entries of a list file named
/// `name`, by what that file's extension lists. A kind with no rule is line 0.
fn not_entries(name: &str, text: &str) -> Vec<usize> {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mut bad = Vec::new();
    let mut in_block = false;
    let mut in_tables = false;
    for (at, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let entry = match ext {
            "ts" => line.is_empty() || line.starts_with("//") || is_export_all(line),
            "css" => {
                if in_block || line.starts_with("/*") {
                    in_block = !line.contains("*/");
                    true
                } else {
                    line.is_empty() || (line.starts_with("@import ") && line.ends_with(';'))
                }
            }
            "toml" => {
                if line.starts_with('[') {
                    in_tables = true;
                }
                in_tables || line.is_empty() || line.starts_with('#')
            }
            "rs" if name.ends_with("migration_list.rs") => {
                // A header, then one `Migration::` entry a line, then `];`.
                if line.starts_with("Migration::") {
                    in_tables = true;
                }
                !in_tables
                    || line.is_empty()
                    || line.starts_with("//")
                    || line.starts_with("Migration::") && line.ends_with("),")
                    || line == "];"
            }
            "rs" => line.is_empty() || line.starts_with("//") || is_mod(line),
            _ => return vec![0],
        };
        if !entry {
            bad.push(at + 1);
        }
    }
    bad
}

fn is_export_all(line: &str) -> bool {
    line.starts_with("export * from ") && line.ends_with(';')
}

fn is_mod(line: &str) -> bool {
    let rest = ["pub(crate) ", "pub(super) ", "pub "]
        .iter()
        .find_map(|p| line.strip_prefix(p))
        .unwrap_or(line);
    rest.strip_prefix("mod ")
        .and_then(|r| r.strip_suffix(';'))
        .is_some_and(|name| name.chars().all(|c| c.is_alphanumeric() || c == '_'))
}

/// Every declared list file exists and holds entries only.
pub fn a_list_file_holds_only_entries(root: &Path) -> Report {
    let mut report = Report::new("every declared list file holds only entries");
    let Ok(attributes) = fs::read_to_string(root.join(".gitattributes")) else {
        return report;
    };
    for path in declared(&attributes) {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            report.fail(format!(
                "{path} — declared `merge=union` in .gitattributes, and not there"
            ));
            continue;
        };
        for line in not_entries(&path, &text) {
            report.fail(match line {
                0 => format!(
                    "{path} — declared `merge=union`, and the gate has no entry rule for its kind"
                ),
                n => format!(
                    "{path}:{n} — not an entry. Keeping both sides of a merge would interleave \
                     it with another edit unseen: move it out of the list file, or take the \
                     file out of .gitattributes"
                ),
            });
        }
    }
    report
}
