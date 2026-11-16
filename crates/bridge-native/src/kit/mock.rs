//! Kit on a Fleet that is not there: generic placeholder pieces, so a walk or a preview draws the screen
//! with no Fleet. It answers Remove, Restore and the project's switch from memory, as Fleet would from disk.

use std::collections::{BTreeMap, HashMap};

use ipc::{KitInventory, KitServerRow, SetupItem, SetupKindRow, WhatWasRead};
use serde_json::json;

use super::model::{Gone, Kind, Row, Scope};

/// The mock's home for the harness.
pub const HOME: &str = "/Users/you/.claude";

/// Where the mock's person keeps their things, for a path to be written as `~/...`.
pub const USER: &str = "/Users/you";

pub fn scope() -> Scope {
    Scope { name: "armada".into(), repositories: vec!["armada".into()] }
}

fn item(name: &str, says: &str, source: &str, plugin: Option<&str>) -> SetupItem {
    SetupItem { name: name.into(), says: Some(says.into()), source: source.into(), style: None, plugin: plugin.map(str::to_string), uninstall: None }
}

fn row(kind: &str, items: Vec<SetupItem>) -> SetupKindRow {
    SetupKindRow { kind: kind.into(), read: WhatWasRead::Read { items, unreadable: Vec::new() } }
}

pub fn inventory() -> KitInventory {
    let mut style = item("Concise", "Short answers, no preamble", "/Users/you/.claude/output-styles/concise.md", None);
    style.style = Some(ipc::OutputStyleFacts { from: "person".into(), applied: "active".into(), plugin: None });
    let mut other = item("Explainer", "Walks through the why of a change", "/Users/you/.claude/output-styles/explainer.md", None);
    other.style = Some(ipc::OutputStyleFacts { from: "person".into(), applied: "available".into(), plugin: None });
    let mut plugin = item("design-kit@design-kit", "version 1.4.0", "/Users/you/.claude/plugins/design-kit", None);
    plugin.uninstall = Some("claude plugin uninstall design-kit@design-kit".into());
    let mut notes = item("release-notes", "Writes release notes from merged pull requests", "/Users/you/.claude/skills/release-notes", None);
    notes.plugin = None;
    KitInventory {
        harness: "Claude Code".into(),
        home: HOME.into(),
        present: true,
        kinds: vec![
            row(
                "skills",
                vec![
                    notes,
                    item("review-pr", "Reviews a pull request against the house rules", "/Users/you/.claude/skills/review-pr", None),
                    item("pdf-tools", "Reads and fills PDF forms", "/Users/you/.claude/skills/pdf-tools", None),
                    item("frontend-design", "Builds a screen in the repository's own look", "/Users/you/.claude/plugins/design-kit/skills/frontend-design", Some("design-kit")),
                    item("deploy-check", "Checks a deploy before it is announced", "/work/armada/.claude/skills/deploy-check", None),
                ],
            ),
            row("plugins", vec![plugin]),
            row("sub_agents", vec![item("code-reviewer", "Reads a diff the way the house rules ask", "/Users/you/.claude/agents/code-reviewer.md", None)]),
            row("commands", vec![item("ship", "Merges the open pull request and tags it", "/Users/you/.claude/commands/ship.md", None)]),
            row("agent_file", vec![item("CLAUDE.md", "42 lines, opening How we work", "/Users/you/.claude/CLAUDE.md", None)]),
            row("output_styles", vec![style, other]),
            row("mcp_servers", vec![item("linear", "linear.app", "", None)]),
            row("allowlist", vec![item("cargo check", "", "always allow", None), item("git status", "", "written by hand", None)]),
        ],
    }
}

pub fn servers() -> Vec<KitServerRow> {
    serde_json::from_value(json!([
        {"name": "docs", "address": {"transport": "http", "url": "https://docs.example.com/mcp"}, "drones": "yes", "resolves": true, "added_at": "2026-11-01T09:00:00Z", "by": "human"},
        {"name": "browser", "address": {"transport": "stdio", "command": "browser-mcp", "args": ["--headless"]}, "drones": "no", "resolves": false, "added_at": "2026-11-02T09:00:00Z", "by": "human"}
    ]))
    .expect("the mock servers decode")
}

/// What review-pr's file says, for the detail's pane.
const REVIEW_PR: &str = "---\nname: review-pr\ndescription: Reviews a pull request against the house rules\n---\n\nRead the diff first. Check each changed file against\nthe rules in docs/contracts before reading the tests.\n…";

pub struct MockKit {
    pub inventory: KitInventory,
    pub servers: Vec<KitServerRow>,
    pub skills_off: BTreeMap<String, Vec<String>>,
    pub files: HashMap<String, String>,
    trash: Vec<(String, SetupItem)>,
}

impl MockKit {
    pub fn new() -> MockKit {
        let mut skills_off = BTreeMap::new();
        skills_off.insert("armada".to_string(), vec!["pdf-tools".to_string()]);
        let files = HashMap::from([("skills:/Users/you/.claude/skills/review-pr:review-pr".to_string(), REVIEW_PR.to_string())]);
        MockKit { inventory: inventory(), servers: servers(), skills_off, files, trash: Vec::new() }
    }

    /// A skill this mock removed before the screen drew, so the first look shows one amber and struck through.
    pub fn removed_first(&mut self) -> Gone {
        let name = "old-triage";
        let item = item(name, "Sorts new issues by how long they have waited", "/Users/you/.claude/skills/old-triage", None);
        let row = super::model::read_kind(Kind::Skills, &KitInventory { kinds: vec![row("skills", vec![item.clone()])], ..inventory() }, &[]);
        let super::model::Read::Rows(mut rows) = row else { unreachable!("a skill reads as rows") };
        let row: Row = rows.remove(0);
        let trashed = format!("~/.Trash/{name}");
        self.trash.push((trashed.clone(), item));
        Gone { row, trashed: Some(trashed) }
    }

    fn skills(&mut self) -> Option<&mut Vec<SetupItem>> {
        self.inventory.kinds.iter_mut().find(|one| one.kind == "skills").and_then(|one| match &mut one.read {
            WhatWasRead::Read { items, .. } => Some(items),
            WhatWasRead::NotRead { .. } => None,
        })
    }

    /// Remove a skill of his own: its folder goes to the Trash.
    pub fn remove_skill(&mut self, name: &str) -> Option<String> {
        let items = self.skills()?;
        let at = items.iter().position(|one| one.name == name)?;
        let item = items.remove(at);
        let trashed = format!("~/.Trash/{name}");
        self.trash.push((trashed.clone(), item));
        Some(trashed)
    }

    pub fn restore(&mut self, trashed: &str) -> bool {
        let Some(at) = self.trash.iter().position(|(place, _)| place == trashed) else { return false };
        let (_, item) = self.trash.remove(at);
        if let Some(items) = self.skills() {
            items.push(item);
        }
        true
    }

    pub fn remove_plugin(&mut self, name: &str) {
        if let Some(one) = self.inventory.kinds.iter_mut().find(|one| one.kind == "plugins") {
            if let WhatWasRead::Read { items, .. } = &mut one.read {
                items.retain(|item| item.name != name);
            }
        }
    }

    pub fn reach(&mut self, name: &str, on: bool) {
        if let Some(server) = self.servers.iter_mut().find(|one| one.name == name) {
            server.manifest = ipc::ManifestReach::from_wire(if on { "extended" } else { "restricted" });
            server.resolves = on;
        }
    }
}
