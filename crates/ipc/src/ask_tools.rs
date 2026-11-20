//! Armada's own tools a person asks the agent for, read from `crates/core-model/domain/ask_tools.toml` as built in. The
//! id is the wire value of a `tool` tag, so the one reader sits with the tag: Bridge draws the list from it and Fleet
//! says a tool's line from it, and neither keeps a copy.
//!
//! No TOML parser: `ipc` keeps none, and the file's shape is one `[tools.<id>]` table a tool with one quoted line a
//! field, as the gate's rule reads it.

use std::sync::OnceLock;

/// One tool: the id the tag carries, its name, what it gives, its glyph key, its `--ask-*` hue token and what Fleet
/// tells the agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskTool {
    pub id: &'static str,
    pub name: &'static str,
    pub says: &'static str,
    pub glyph: &'static str,
    pub hue: &'static str,
    pub line: &'static str,
}

/// Every tool, in the order the file lists them, which is the order the list draws them.
pub fn ask_tools() -> &'static [AskTool] {
    static TOOLS: OnceLock<Vec<AskTool>> = OnceLock::new();
    TOOLS.get_or_init(|| read(include_str!("../../core-model/domain/ask_tools.toml")))
}

/// The tool a tag's id names; none for an id the file does not hold.
pub fn ask_tool(id: &str) -> Option<&'static AskTool> {
    ask_tools().iter().find(|tool| tool.id == id)
}

fn read(registry: &'static str) -> Vec<AskTool> {
    let mut tables: Vec<(&'static str, Vec<(&'static str, &'static str)>)> = Vec::new();
    for line in registry.lines().map(str::trim) {
        if let Some(id) = line.strip_prefix("[tools.").and_then(|rest| rest.strip_suffix(']')) {
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
                    .unwrap_or_else(|| panic!("ask_tools.toml: `{id}` has no `{key}`"))
            };
            AskTool { id, name: field("name"), says: field("says"), glyph: field("glyph"), hue: field("hue"), line: field("line") }
        })
        .collect()
}
