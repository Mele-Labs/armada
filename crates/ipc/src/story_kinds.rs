//! The kinds of Job a person's words are, read from `crates/core-model/domain/story_kinds.toml` as built in, with the
//! workflow each starts on. Fleet reads it to start the Operator's Jobs on a kind's workflow, and the Operator is told
//! the list, so the sheet and the Operator start the same kind on the same workflow and neither keeps a copy.
//!
//! No TOML parser, as [`crate::ask_tools`] keeps none: one `[kinds.<id>]` table a kind, one quoted line a field.

use std::sync::OnceLock;

/// One kind, with what a start needs of it. The sentence, hue and glyph are the sheet's alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryKind {
    pub id: &'static str,
    pub name: &'static str,
    /// The words a person types, which lead the sentence.
    pub opening: &'static str,
    /// The workflow it starts on; none for a talk Job, which the file writes `none`.
    pub workflow: Option<&'static str>,
    /// What the Job it starts is, in a few words.
    pub starts: &'static str,
}

/// Every kind, in the order the file lists them.
pub fn story_kinds() -> &'static [StoryKind] {
    static KINDS: OnceLock<Vec<StoryKind>> = OnceLock::new();
    KINDS.get_or_init(|| read(include_str!("../../core-model/domain/story_kinds.toml")))
}

/// The kind an id names; none for an id the file does not hold.
pub fn story_kind(id: &str) -> Option<&'static StoryKind> {
    story_kinds().iter().find(|kind| kind.id == id)
}

fn read(registry: &'static str) -> Vec<StoryKind> {
    let mut tables: Vec<(&'static str, Vec<(&'static str, &'static str)>)> = Vec::new();
    for line in registry.lines().map(str::trim) {
        if let Some(id) = line.strip_prefix("[kinds.").and_then(|rest| rest.strip_suffix(']')) {
            tables.push((id, Vec::new()));
        } else if let (Some((_, fields)), Some((key, value))) = (tables.last_mut(), line.split_once('=')) {
            if !line.starts_with('#') {
                fields.push((key.trim(), value.trim().trim_matches('"')));
            }
        }
    }
    tables
        .into_iter()
        .map(|(id, fields)| {
            let field = |key: &str| {
                fields
                    .iter()
                    .find_map(|(named, value)| (*named == key).then_some(*value))
                    .unwrap_or_else(|| panic!("story_kinds.toml: `{id}` has no `{key}`"))
            };
            let workflow = field("workflow");
            StoryKind {
                id,
                name: field("name"),
                opening: field("opening"),
                workflow: (workflow != "none").then_some(workflow),
                starts: field("starts"),
            }
        })
        .collect()
}
