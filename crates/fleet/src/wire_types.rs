//! Which public wire types a pull request's patches change. `docs/concepts/fleet.md`, *Telling the
//! Jobs that read a wire type*.
//!
//! **Read from the patches alone**, so no checkout of the pull request is needed. A hunk's header
//! names the last top-level line before it, and a changed line is a member of the type whose
//! declaration is nearest above it. A type a hunk cannot place is left out rather than guessed at.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::PatchedFile;

/// Where the wire types are written. Tests under it are not the wire.
const WIRE: &str = "crates/ipc/src/";
const WIRE_TESTS: &str = "crates/ipc/src/tests/";

/// One `pub struct`, `pub enum` or `pub type` of the wire with a changed line in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TypeChange {
    pub(crate) name: String,
    declared: bool,
    dropped: bool,
    added: BTreeSet<String>,
    removed: BTreeSet<String>,
}

impl TypeChange {
    /// What changed, in bare facts: the members added, removed or changed by name, else `changed`.
    pub(crate) fn said(&self) -> String {
        if self.declared && !self.dropped {
            return "added".to_string();
        }
        if self.dropped && !self.declared {
            return "removed".to_string();
        }
        let join = |names: Vec<&String>| names.into_iter().cloned().collect::<Vec<_>>().join(", ");
        let both: Vec<&String> = self.added.intersection(&self.removed).collect();
        let added: Vec<&String> = self.added.difference(&self.removed).collect();
        let removed: Vec<&String> = self.removed.difference(&self.added).collect();
        let mut parts = Vec::new();
        if !added.is_empty() {
            parts.push(format!("adds {}", join(added)));
        }
        if !removed.is_empty() {
            parts.push(format!("removes {}", join(removed)));
        }
        if !both.is_empty() {
            parts.push(format!("changes {}", join(both)));
        }
        if parts.is_empty() {
            "changed".to_string()
        } else {
            parts.join("; ")
        }
    }
}

/// The wire types the patches change, by name.
pub(crate) fn changed_types(files: &[PatchedFile]) -> Vec<TypeChange> {
    let mut found: BTreeMap<String, TypeChange> = BTreeMap::new();
    for file in files {
        let wire = file.path.starts_with(WIRE) && file.path.ends_with(".rs") && !file.path.starts_with(WIRE_TESTS);
        if let (true, Some(patch)) = (wire, file.patch.as_deref()) {
            one_patch(patch, &mut found);
        }
    }
    found.into_values().collect()
}

fn one_patch(patch: &str, found: &mut BTreeMap<String, TypeChange>) {
    let mut seen_hunk = false;
    let mut carried: Option<String> = None;
    let mut first = false;
    let mut current: Option<String> = None;
    let mut attribute_changed = false;
    for line in patch.lines() {
        if let Some(header) = line.strip_prefix("@@") {
            seen_hunk = true;
            first = true;
            current = None;
            attribute_changed = false;
            carried = header.split_once("@@").and_then(|(_, context)| declared(context.trim_start()));
            continue;
        }
        if !seen_hunk || line.starts_with('\\') {
            continue;
        }
        let (marker, content) = match line.chars().next() {
            Some(marker @ ('+' | '-' | ' ')) => (marker, &line[1..]),
            _ => (' ', line),
        };
        let changed = marker != ' ';
        // The header names the nearest line above the hunk, which is the type only where the hunk
        // starts inside its body.
        if std::mem::take(&mut first) && content.starts_with([' ', '\t']) {
            current = carried.take();
        }
        if let Some(name) = declared(content) {
            if changed {
                let one = found.entry(name.clone()).or_insert_with(|| TypeChange { name: name.clone(), ..TypeChange::default() });
                if marker == '+' {
                    one.declared = true;
                } else {
                    one.dropped = true;
                }
                attribute_changed = false;
            } else if attribute_changed {
                found.entry(name.clone()).or_insert_with(|| TypeChange { name: name.clone(), ..TypeChange::default() });
                attribute_changed = false;
            }
            current = if content.trim_end().ends_with(';') { None } else { Some(name) };
        } else if content.starts_with('}') {
            current = None;
            attribute_changed = false;
        } else if content.starts_with('#') {
            attribute_changed |= changed;
        } else if content.starts_with(char::is_whitespace) {
            let (Some(name), true) = (current.as_ref(), changed) else {
                continue;
            };
            let said = content.trim();
            if said.is_empty() || said.starts_with("//") {
                continue;
            }
            let one = found.entry(name.clone()).or_insert_with(|| TypeChange { name: name.clone(), ..TypeChange::default() });
            let indent = content.len() - content.trim_start().len();
            if indent == 4 || (content.starts_with('\t') && !content.starts_with("\t\t")) {
                if let Some(member) = member_of(said) {
                    if marker == '+' { one.added.insert(member) } else { one.removed.insert(member) };
                }
            }
        } else if !content.is_empty() && !content.starts_with("//") {
            current = None;
            attribute_changed = false;
        }
    }
}

/// The name of a `pub struct`, `pub enum` or `pub type` that a line, at its left edge, declares.
fn declared(line: &str) -> Option<String> {
    let rest = ["pub struct ", "pub enum ", "pub type "].iter().find_map(|word| line.strip_prefix(word))?;
    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    (!name.is_empty()).then_some(name)
}

/// The field or variant a body line at the first level names. `None` for an attribute or a bracket.
fn member_of(line: &str) -> Option<String> {
    let line = line.strip_prefix("pub ").unwrap_or(line);
    let name: String = line.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    (!name.is_empty() && !line.starts_with('#')).then_some(name)
}
