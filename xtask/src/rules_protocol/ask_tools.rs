//! Every operation an agent can reach says where it stands in the message box's list of Armada's tools.
//!
//! The list (`crates/core-model/domain/ask_tools.toml`) is what a person picks from to ask the agent for a sketch, a
//! terminal or the feature catalog, and Fleet tells the agent which act to call. A new thing an agent can do that
//! nobody placed there is a thing he cannot ask for by name, and nothing would say so. So the operation file carries
//! `ask_tool`: a tool's id, or `none`, decided when the operation is added rather than left to a doc.
//!
//! It refuses four ways: an agent-reachable operation with no `ask_tool`, one naming a tool the list does not hold, a
//! tool with no hue token, glyph or line, and `ask_tool` on an operation no agent can reach. No `toml` crate: the
//! gate keeps no dependencies, and both files are one table an entry with one line a field.

use std::fs;
use std::path::Path;

use crate::Report;

use super::INVENTORY;

/// The list, one `[tools.<id>]` table a tool.
const REGISTRY: &str = "crates/core-model/domain/ask_tools.toml";
/// Where each tool's `--ask-*` hue is declared, dark and light.
const HUES: &[&str] = &["packages/tokens/src/ask-tools.css", "packages/tokens/src/light.css"];
/// One file a glyph, named for its key.
const GLYPHS: &str = "packages/icons/icons";
/// The `agent_access` values an agent can reach: the door's three tables.
const REACHABLE: &[&str] = &["Yes", "On your words", "Operator"];

/// Rule: every operation an agent can reach names its tool in the ask list, or `none`.
pub fn every_agent_tool_has_a_place_in_the_ask_list(root: &Path) -> Report {
    let mut report = Report::new("every operation an agent can reach has a place in the message box's tools, or says none");

    let Ok(inventory) = crate::read_entry_dirs(root, &[INVENTORY]) else {
        report.fail(format!("{INVENTORY} — the operation inventory itself"));
        return report;
    };
    let Ok(registry) = fs::read_to_string(root.join(REGISTRY)) else {
        report.fail(format!("{REGISTRY} — the list of tools itself"));
        return report;
    };
    let mut hues = Vec::new();
    for file in HUES {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            report.fail(format!("{file} — where the tools' hues are declared"));
            return report;
        };
        hues.push((*file, text));
    }
    let glyph = |key: &str| root.join(GLYPHS).join(format!("{key}.toml")).is_file();

    check(&inventory, &registry, &hues, &glyph, &mut report);
    report
}

/// Every check the rule makes, over the files as text.
pub(super) fn check(inventory: &str, registry: &str, hues: &[(&str, String)], glyph: &dyn Fn(&str) -> bool, report: &mut Report) {
    let tools = tools(registry);
    if tools.is_empty() {
        report.fail(format!("{REGISTRY} holds no `[tools.<id>]` table — a list of nothing passes every operation"));
        return;
    }
    for (id, fields) in &tools {
        let field = |key: &str| fields.iter().find_map(|(named, value)| (named == key).then_some(value.as_str())).unwrap_or("");
        let hue = field("hue");
        for (file, text) in hues {
            if hue.is_empty() || !text.contains(&format!("{hue}:")) {
                report.fail(format!(
                    "{REGISTRY} — tool `{id}` has hue `{hue}`, which {file} does not declare. Declare its `--ask-*` token \
                     in packages/tokens/src/ask-tools.css and light.css, and name it as `hue`"
                ));
            }
        }
        let key = field("glyph");
        if key.is_empty() || !glyph(key) {
            report.fail(format!(
                "{REGISTRY} — tool `{id}` has glyph `{key}`, which has no file in {GLYPHS}/. Name a glyph registered there"
            ));
        }
        if field("line").trim().is_empty() {
            report.fail(format!(
                "{REGISTRY} — tool `{id}` has no `line`, so Fleet tells the agent nothing when he asks for it. Write the \
                 sentence that says which act to call"
            ));
        }
    }

    let mut read = 0usize;
    for block in inventory.split("\n[operations.") {
        let Some((header, body)) = block.split_once(']') else {
            continue;
        };
        let name = header.trim().trim_matches('"');
        // The text before the first table is the first file's own header comment.
        if name.is_empty() || name.contains(char::is_whitespace) {
            continue;
        }
        read += 1;
        let file = format!("{INVENTORY}/{name}.toml");
        let access = field(body, "agent_access").unwrap_or_default();
        let reachable = REACHABLE.contains(&access.as_str());
        match (reachable, field(body, "ask_tool")) {
            (true, None) => report.fail(format!(
                "{file} — `{name}` is an operation an agent can reach and does not say where it stands in the message \
                 box's tools. Add `ask_tool = \"<id>\"` naming the tool in {REGISTRY} that covers it, add a row there if \
                 it is a new thing a person would ask for, or `ask_tool = \"none\"`"
            )),
            (false, Some(_)) => report.fail(format!(
                "{file} — `ask_tool` sits on `{name}`, which no agent can reach (`agent_access = \"{access}\"`). Delete \
                 it: the list offers only what an agent can do"
            )),
            (true, Some(value)) => {
                let ids = ids(&value);
                if ids.is_empty() || (ids.len() > 1 && ids.iter().any(|id| id == "none")) {
                    report.fail(format!(
                        "{file} — `ask_tool = {value}` on `{name}` is neither tool ids nor `none` alone. Name the tools \
                         in {REGISTRY} that cover it, or write `ask_tool = \"none\"`"
                    ));
                }
                for id in ids.iter().filter(|id| *id != "none") {
                    if !tools.iter().any(|(tool, _)| tool == id) {
                        report.fail(format!(
                            "{file} — `ask_tool` names `{id}`, which {REGISTRY} does not hold. Add `ask_tool = \"<id>\"` \
                             naming the tool there that covers it, add a row there if it is a new thing a person would \
                             ask for, or `ask_tool = \"none\"`"
                        ));
                    }
                }
            }
            (false, None) => {}
        }
    }
    if read == 0 {
        report.fail(format!("{INVENTORY} named no operation — a comparison of nothing"));
    }
}

/// Each `[tools.<id>]` table in file order, with its `key = "value"` lines.
fn tools(registry: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut tools: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in registry.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        if let Some(id) = line.strip_prefix("[tools.").and_then(|rest| rest.strip_suffix(']')) {
            tools.push((id.to_string(), Vec::new()));
        } else if let (Some((_, fields)), Some((key, value))) = (tools.last_mut(), line.split_once('=')) {
            fields.push((key.trim().to_string(), value.trim().trim_matches('"').to_string()));
        }
    }
    tools
}

/// One `key = value` out of a block, the value as written less its quotes on a plain string.
fn field(block: &str, key: &str) -> Option<String> {
    block
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(key)?.trim_start().strip_prefix('=').map(str::trim))
        .map(|value| if value.starts_with('[') { value.to_string() } else { value.trim_matches('"').to_string() })
}

/// The ids an `ask_tool` value names: one quoted id, or a list of them.
fn ids(value: &str) -> Vec<String> {
    value
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|id| id.trim().trim_matches('"').to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

#[cfg(test)]
mod tests;
