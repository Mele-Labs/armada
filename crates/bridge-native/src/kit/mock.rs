//! Kit on a Fleet that is not there: generic placeholder pieces, so a walk or a preview draws the screen
//! with no Fleet. It answers Remove, Restore and the project's switch from memory, as Fleet would from disk.

use std::collections::{BTreeMap, HashMap};

use ipc::{HomeItem, HomeKind, HomeKindSwitch, KitAgainst, KitInventory, KitLink, KitLinkRow, KitLinkedMac, KitMark, KitServerRow, KitSide, SetupItem, SetupKindRow, WhatWasRead};
use serde_json::json;

use super::model::{Gone, Kind, Row, Scope};
use super::sync::Cross;
use super::wire::Saved;
use crate::settings::wire::Refused;

/// The mock's home for the harness.
pub const HOME: &str = "/Users/user/.claude";

/// Where the mock's person keeps their things, for a path to be written as `~/...`.
pub const USER: &str = "/Users/user";

pub fn scope() -> Scope {
    Scope { name: "armada".into(), repositories: vec!["armada".into()] }
}

fn item(name: &str, says: &str, source: &str, plugin: Option<&str>) -> SetupItem {
    SetupItem { name: name.into(), says: Some(says.into()), source: source.into(), style: None, plugin: plugin.map(str::to_string), uninstall: None, server: None }
}

fn row(kind: &str, items: Vec<SetupItem>) -> SetupKindRow {
    SetupKindRow { kind: kind.into(), read: WhatWasRead::Read { items, unreadable: Vec::new() } }
}

pub fn inventory() -> KitInventory {
    let mut style = item("Concise", "Short answers, no preamble", "/Users/user/.claude/output-styles/concise.md", None);
    style.style = Some(ipc::OutputStyleFacts { from: "person".into(), applied: "active".into(), plugin: None });
    let mut other = item("Explainer", "Walks through the why of a change", "/Users/user/.claude/output-styles/explainer.md", None);
    other.style = Some(ipc::OutputStyleFacts { from: "person".into(), applied: "available".into(), plugin: None });
    let mut armada = item("Armada", "The house voice: short, plain, no filler", "/Users/user/.claude/plugins/armada/output-styles/armada.md", None);
    armada.style = Some(ipc::OutputStyleFacts { from: "plugin".into(), applied: "available".into(), plugin: Some("armada@armada-local".into()) });
    let mut docs_here = item("design-docs", "design-docs-mcp", "/Users/user/.claude/plugins/design-kit/.mcp.json", Some("design-kit@design-kit"));
    docs_here.says = Some("design-docs-mcp".into());
    let mut ci = item("ci-logs", "ci-logs-mcp", "/work/armada/.mcp.json", None);
    ci.server = Some(ipc::ServerFacts { from: "repository".into(), folder: "/work/armada".into() });
    let mut plugin = item("design-kit@design-kit", "version 1.4.0", "/Users/user/.claude/plugins/design-kit", None);
    plugin.uninstall = Some("claude plugin uninstall design-kit@design-kit".into());
    let mut notes = item("release-notes", "Writes release notes from merged pull requests", "/Users/user/.claude/skills/release-notes", None);
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
                    item("review-pr", "Reviews a pull request against the house rules", "/Users/user/.claude/skills/review-pr", None),
                    item("pdf-tools", "Reads and fills PDF forms", "/Users/user/.claude/skills/pdf-tools", None),
                    item("deploy-notes", "Writes the notes a deploy goes out with", "/Users/user/.claude/skills/deploy-notes", None),
                    item("tracker-sync", "Files a ticket for each new issue", "/Users/user/.claude/skills/tracker-sync", None),
                    item("frontend-design", "Builds a screen in the repository's own look", "/Users/user/.claude/plugins/design-kit/skills/frontend-design", Some("design-kit")),
                    item("deploy-check", "Checks a deploy before it is announced", "/work/armada/.claude/skills/deploy-check", None),
                    item("long-notes", "A long file, for scrolling", "/work/armada/.claude/skills/long-notes", None),
                ],
            ),
            row("plugins", vec![plugin]),
            row(
                "sub_agents",
                vec![
                    item("code-reviewer", "Reads a diff the way the house rules ask", "/Users/user/.claude/agents/code-reviewer.md", None),
                    item("test-writer", "Writes the test a change is missing", "/Users/user/.claude/agents/test-writer.md", None),
                ],
            ),
            row("commands", vec![item("ship", "Merges the open pull request and tags it", "/Users/user/.claude/commands/ship.md", None)]),
            row("agent_file", vec![item("CLAUDE.md", "42 lines, opening How we work", "/Users/user/.claude/CLAUDE.md", None)]),
            row("output_styles", vec![style, other, armada]),
            row("mcp_servers", vec![item("linear", "linear.app", "", None), docs_here, ci]),
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

/// A long skill: sixty-odd lines, the description running well past any pane's width.
pub const LONG_NOTES_ID: &str = "skills:/work/armada/.claude/skills/long-notes:long-notes";

fn long_notes() -> String {
    let mut text = String::from("---\nname: long-notes\ndescription: Use when a person asks for the long way round of anything, and keep going well past the point where a pane would have run out of width, because a line that long has to wrap inside the pane and not run off its right edge\n---\n\n");
    for n in 1..=60 {
        text.push_str(&format!("Step {n}: read the next part, and write down what it says before going on.\n"));
    }
    text
}

pub struct MockKit {
    pub inventory: KitInventory,
    pub servers: Vec<KitServerRow>,
    pub skills_off: BTreeMap<String, Vec<String>>,
    pub files: HashMap<String, String>,
    trash: Vec<(String, SetupItem)>,
    /// Kit over the Mac link, as `get_kit_link` would answer.
    pub link: KitLink,
    /// Rows of a kind switched off, back when it is switched on.
    stash: Vec<KitLinkRow>,
    /// Each call over the link, in the order it was sent.
    pub sent: Vec<Cross>,
    /// A call Fleet refuses, for a test of a refusal part way.
    pub refuse: Option<Cross>,
}

impl MockKit {
    pub fn new() -> MockKit {
        let mut skills_off = BTreeMap::new();
        skills_off.insert("armada".to_string(), vec!["pdf-tools".to_string()]);
        let files = HashMap::from([
            ("skills:/Users/user/.claude/skills/review-pr:review-pr".to_string(), REVIEW_PR.to_string()),
            (LONG_NOTES_ID.to_string(), long_notes()),
            (RELEASE_NOTES_ID.to_string(), RELEASE_NOTES_HERE.to_string()),
        ]);
        let link = [(KIT_UNLINKED, Linked::Unlinked), (KIT_IN_STEP, Linked::InStep)].into_iter().find(|(name, _)| crate::mock::scenario_is(name)).map_or(Linked::Apart, |(_, linked)| linked);
        MockKit { inventory: inventory(), servers: servers(), skills_off, files, trash: Vec::new(), link: link_of(linked_now(link)), stash: Vec::new(), sent: Vec::new(), refuse: None }
    }

    /// The same, with Kit over the Mac link standing as `linked` says.
    #[cfg(test)]
    pub fn linked(linked: Linked) -> MockKit {
        MockKit { link: link_of(linked_now(linked)), ..MockKit::new() }
    }

    /// One call over the link, as Fleet answers it: taken, or refused in Fleet's words.
    pub fn send(&mut self, cross: &Cross) -> Result<(), String> {
        self.sent.push(cross.clone());
        if self.refuse.as_ref() == Some(cross) {
            return Err("Fleet could not write the link's switches on this Mac.".into());
        }
        match cross {
            Cross::Kind(kind, on) => self.cross_kind(*kind, *on),
            Cross::Item(item, on) => self.cross_item(item.clone(), *on),
        }
        Ok(())
    }

    /// Kit's switch for a kind: switched off, its rows leave the link's read, and come back when it is on again.
    pub fn cross_kind(&mut self, kind: HomeKind, on: bool) {
        for one in self.link.kinds.iter_mut().filter(|one| one.kind == kind) {
            one.on = on;
        }
        if on {
            let (back, kept): (Vec<KitLinkRow>, Vec<KitLinkRow>) = std::mem::take(&mut self.stash).into_iter().partition(|row| row.kind == kind);
            self.stash = kept;
            let back: Vec<KitLinkRow> = back.into_iter().filter(|row| !self.link.rows.iter().any(|one| one.kind == row.kind && one.name == row.name)).collect();
            self.link.rows.extend(back);
        } else {
            let alone = self.link.items.clone();
            let (gone, kept): (Vec<KitLinkRow>, Vec<KitLinkRow>) = std::mem::take(&mut self.link.rows).into_iter().partition(|row| row.kind == kind && !alone.iter().any(|item| item.kind == kind && item.name == row.name));
            self.link.rows = kept;
            self.stash.extend(gone);
        }
    }

    /// One item switched by itself: while its kind is off, it crosses alone and is read as the same on each Mac.
    pub fn cross_item(&mut self, item: HomeItem, on: bool) {
        self.link.items.retain(|one| *one != item);
        let kind_on = self.link.kinds.iter().any(|one| one.kind == item.kind && one.on);
        if on {
            self.link.items.push(item.clone());
            if !self.link.rows.iter().any(|row| row.kind == item.kind && row.name == item.name) {
                let against = self.link.macs.iter().map(|mac| same(&mac.mac)).collect();
                self.link.rows.push(link_row(item.kind, &item.name, None, against));
            }
        } else if !kind_on {
            self.link.rows.retain(|row| !(row.kind == item.kind && row.name == item.name));
        }
    }

    /// A clash settled: the side kept is on both Macs, so the row reads the same. Keeping the other Mac's writes it
    /// over this Mac's file.
    pub fn settle(&mut self, mac: &str, item: &HomeItem, keep: ipc::KitKeep, file: &str) {
        let Some(row) = self.link.rows.iter_mut().find(|row| row.kind == item.kind && row.name == item.name) else { return };
        for against in row.against.iter_mut().filter(|one| one.mac == mac && one.mark == ipc::KitMark::Clash) {
            if keep == ipc::KitKeep::There {
                if let Some(there) = &against.there {
                    self.files.insert(file.to_string(), there.text.clone());
                }
            }
            *against = same(mac);
        }
    }

    /// A skill this mock removed before the screen drew, so the first look shows one amber and struck through.
    pub fn removed_first(&mut self) -> Gone {
        let name = "old-triage";
        let item = item(name, "Sorts new issues by how long they have waited", "/Users/user/.claude/skills/old-triage", None);
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

    /// A piece's file as the mock's disk holds it: what was set for it, else its own words. A piece a server or an
    /// allowed command is has none.
    pub fn file(&self, row: &Row) -> Result<ipc::KitFile, Refused> {
        if row.kind.file().is_none() {
            return Err(Refused { key: None, said: "This piece has no file.".into(), code: Some("fleet.kit_file_none".into()) });
        }
        let text = self.files.get(&row.id).cloned().unwrap_or_else(|| row.says.clone().unwrap_or_default());
        Ok(ipc::KitFile { path: row.opens().unwrap_or_default(), text, at: ipc::Instant::carried("2026-11-16T09:00:00Z"), plugin: row.plugin_id.clone() })
    }

    /// A save as Fleet guards it: refused for a plugin's piece, and refused where the disk no longer holds what the
    /// edit started from.
    pub fn save(&mut self, row: &Row, read: &str, text: &str) -> Result<ipc::KitFile, Saved> {
        if let Some(plugin) = &row.comes_with {
            return Err(Saved { said: format!("It comes with the {plugin} plugin, so it is read-only here."), code: Some("fleet.kit_file_from_plugin".into()), on_disk: None });
        }
        let now = self.file(row).map_err(|why| Saved::said(why.said))?;
        if now.text != read {
            return Err(Saved { said: "The file changed on disk since you opened it.".into(), code: Some("fleet.kit_file_moved".into()), on_disk: Some(now.text) });
        }
        self.files.insert(row.id.clone(), text.to_string());
        self.file(row).map_err(|why| Saved::said(why.said))
    }

    pub fn reach(&mut self, name: &str, on: bool) {
        if let Some(server) = self.servers.iter_mut().find(|one| one.name == name) {
            server.manifest = ipc::ManifestReach::from_wire(if on { "extended" } else { "restricted" });
            server.resolves = on;
        }
    }
}

/// The scenarios that show Kit with no Mac linked, and with every row the same.
pub const KIT_UNLINKED: &str = "kit-unlinked";
pub const KIT_IN_STEP: &str = "kit-in-step";

/// How the mock's Kit stands over the Mac link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linked {
    Unlinked,
    InStep,
    /// A row in every mark, and one clash.
    Apart,
}

/// The linked Mac's id and name.
pub const WORK_MAC: (&str, &str) = ("mac-work", "Work Mac");

/// The skill that clashes, as Kit lists it.
pub const RELEASE_NOTES_ID: &str = "skills:/Users/user/.claude/skills/release-notes:release-notes";

const RELEASE_NOTES_HERE: &str = "---\nname: release-notes\ndescription: Writes release notes from merged pull requests\n---\n\nGroup the changes by area.\nSay what the diff cannot.\nOne line per change.";
const RELEASE_NOTES_THERE: &str = "---\nname: release-notes\ndescription: Writes release notes from merged pull requests\n---\n\nGroup the changes by area.\nOne line per change.\nWrap at 72.";

/// What a credential refusal says: the host, never the value.
pub const REFUSED: &str = "Carries a token for api.tracker.example, so it stays on this Mac";

/// The times the mock's link reads, from now: when Work Mac last crossed, and when each side of the clash changed.
struct Times {
    crossed: ipc::Instant,
    here: ipc::Instant,
    there: ipc::Instant,
}

fn linked_now(linked: Linked) -> (Linked, Times) {
    let now = chrono::Utc::now();
    let at = |minutes: i64| ipc::Instant::carried((now - chrono::Duration::minutes(minutes)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    (linked, Times { crossed: at(2), here: at(14 * 60), there: at(3 * 60) })
}

fn same(mac: &str) -> KitAgainst {
    KitAgainst { mac: mac.into(), mark: KitMark::Same, here: None, there: None }
}

fn link_row(kind: HomeKind, name: &str, says: Option<&str>, against: Vec<KitAgainst>) -> KitLinkRow {
    KitLinkRow { kind, name: name.into(), repository: None, says: says.map(str::to_string), source: None, key_here: None, mod_kind: None, refused: None, against }
}

fn link_of((linked, times): (Linked, Times)) -> KitLink {
    // Sub agents are off with one item crossing by itself, so their switch stands at some.
    let on = [HomeKind::Skills, HomeKind::Commands, HomeKind::GlobalInstructions];
    let mac = WORK_MAC.0;
    let macs = match linked {
        Linked::Unlinked => Vec::new(),
        _ => vec![KitLinkedMac { mac: mac.into(), name: WORK_MAC.1.into(), crossed_at: Some(times.crossed.clone()), kinds: on.to_vec(), items: Vec::new() }],
    };
    let side = |name: &str, text: &str, at: &ipc::Instant| KitSide { mac_name: name.into(), text: text.into(), changed_at: Some(at.clone()) };
    let apart = |mark: KitMark, here: &str, there: &str| match linked {
        Linked::Unlinked => None,
        Linked::InStep => Some(same(mac)),
        Linked::Apart => Some(KitAgainst { mac: mac.into(), mark, here: Some(side("This Mac", here, &times.here)), there: Some(side(WORK_MAC.1, there, &times.there)) }),
    };
    let mark = |mark: KitMark| match linked {
        Linked::Unlinked => None,
        Linked::InStep => Some(same(mac)),
        Linked::Apart => Some(KitAgainst { mac: mac.into(), mark, here: None, there: None }),
    };
    let mut refused = link_row(HomeKind::Skills, "tracker-sync", Some("Files a ticket for each new issue"), mark(KitMark::NewHere).into_iter().collect());
    refused.refused = Some(REFUSED.into());
    let mut rows = vec![
        link_row(HomeKind::Skills, "release-notes", Some("Writes release notes from merged pull requests"), apart(KitMark::Clash, RELEASE_NOTES_HERE, RELEASE_NOTES_THERE).into_iter().collect()),
        link_row(HomeKind::Skills, "review-pr", Some("Reviews a pull request against the house rules"), mark(KitMark::Same).into_iter().collect()),
        link_row(HomeKind::Skills, "pdf-tools", Some("Reads and fills PDF forms"), apart(KitMark::Differs, "Reads and fills PDF forms.", "Reads, fills and signs PDF forms.").into_iter().collect()),
        link_row(HomeKind::Skills, "deploy-notes", Some("Writes the notes a deploy goes out with"), mark(KitMark::NewHere).into_iter().collect()),
        refused,
        link_row(HomeKind::Plugins, "design-kit@design-kit", None, mark(KitMark::Same).into_iter().collect()),
        link_row(HomeKind::GlobalInstructions, "CLAUDE.md", None, apart(KitMark::Differs, "How we work", "How we work, and why").into_iter().collect()),
        link_row(HomeKind::SubAgents, "code-reviewer", Some("Reads a diff the way the house rules ask"), mark(KitMark::Same).into_iter().collect()),
        link_row(HomeKind::Commands, "ship", Some("Merges the open pull request and tags it"), mark(KitMark::NewHere).into_iter().collect()),
    ];
    if linked == Linked::Apart {
        rows.insert(5, link_row(HomeKind::Skills, "pricing-review", Some("Checks a price change against the margin rules"), mark(KitMark::OnlyThere).into_iter().collect()));
    }
    KitLink {
        kinds: HomeKind::ALL.iter().map(|kind| HomeKindSwitch { kind: *kind, on: on.contains(kind) }).collect(),
        items: vec![HomeItem { kind: HomeKind::SubAgents, name: "code-reviewer".into(), repository: None }],
        macs,
        rows,
        unreadable: Vec::new(),
    }
}
