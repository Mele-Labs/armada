//! What a Job's Loadout says and what each row may do, with no drawing: Classic's `loadout.ts`, `mcpActsOf`,
//! `StateMark`, `useMcpGroup`'s act-wins rule and `loadout-kit.ts`'s match against Kit's inventory.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Local, Utc};
use ipc::time_format::TimeFormat;
use ipc::{
    KitInventory, LoadoutItem, LoadoutKind, LoadoutSource, LoadoutState, LoadoutSummary, LoadoutTurn, LoadoutUse, LoadoutUseKind, McpServerAct, McpServerState,
    RemoveKitPiece, SessionMcpServer, SetupItem, WhatWasRead,
};

use crate::look::{token, Token};

/// What a kind's list draws before Show more.
pub(super) const BOUND: usize = 40;

/// How often an open Loadout asks a hosted Job's process for its MCP servers again (Classic's `MCP_EVERY_MS`).
pub(super) const MCP_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// What the agent knows, in the order the tab lists them.
pub(super) const KNOWN: [LoadoutKind; 5] = [LoadoutKind::Skill, LoadoutKind::Subagent, LoadoutKind::McpServer, LoadoutKind::Tool, LoadoutKind::MemoryFile];
/// What it was told.
pub(super) const TOLD: [LoadoutKind; 3] = [LoadoutKind::OutputStyle, LoadoutKind::Rules, LoadoutKind::Injection];

/// A kind as the heading over its items.
pub(super) fn kind_said(kind: LoadoutKind) -> &'static str {
    match kind {
        LoadoutKind::Skill => "Skills",
        LoadoutKind::Subagent => "Subagents",
        LoadoutKind::McpServer => "MCP servers",
        LoadoutKind::Tool => "Tools",
        LoadoutKind::MemoryFile => "Memory files",
        LoadoutKind::OutputStyle => "Output style",
        LoadoutKind::Rules => "Job rules",
        LoadoutKind::Injection => "Injections",
    }
}

/// A kind's glyph and hue, on its heading and at the head of each of its rows, so a glance tells kinds apart.
pub(super) fn kind_look(kind: LoadoutKind) -> (&'static str, Token) {
    match kind {
        LoadoutKind::Skill => ("sparkles", token::KIND_STEP),
        LoadoutKind::Subagent => ("split", token::KIND_GATE),
        LoadoutKind::McpServer => ("server", token::KIND_SETUP),
        LoadoutKind::Tool => ("wrench", token::KIND_DELIVERY),
        LoadoutKind::MemoryFile => ("file-text", token::ACCENT),
        LoadoutKind::OutputStyle => ("palette", token::KIND_GATE),
        LoadoutKind::Rules => ("scale", token::KIND_STEP),
        LoadoutKind::Injection => ("message-square-quote", token::ACCENT),
    }
}

/// The item kind a turn's use stands for.
pub(super) fn kind_of_use(kind: LoadoutUseKind) -> LoadoutKind {
    match kind {
        LoadoutUseKind::Skill => LoadoutKind::Skill,
        LoadoutUseKind::Tool => LoadoutKind::Tool,
        LoadoutUseKind::Subagent => LoadoutKind::Subagent,
    }
}

/// Where an item or a use came from, as a person reads it: `Kit`, `Armada`, a plugin's or a server's own name.
pub(super) fn source_said(source: LoadoutSource, from: Option<&str>) -> String {
    match (source, from) {
        (LoadoutSource::Plugin | LoadoutSource::Mcp, Some(from)) if !from.is_empty() => from.to_string(),
        _ => match source {
            LoadoutSource::Kit => "Kit",
            LoadoutSource::Armada => "Armada",
            LoadoutSource::Repository => "Repository",
            LoadoutSource::Plugin => "Plugin",
            LoadoutSource::Mcp => "MCP",
            LoadoutSource::BuiltIn => "Built-in",
            LoadoutSource::Managed => "Managed",
        }
        .to_string(),
    }
}

const SOURCE_ORDER: [LoadoutSource; 7] =
    [LoadoutSource::Kit, LoadoutSource::Armada, LoadoutSource::Repository, LoadoutSource::Plugin, LoadoutSource::Mcp, LoadoutSource::BuiltIn, LoadoutSource::Managed];

/// The sources the items and uses name, each once, Kit first, and plugins and servers by name inside theirs.
pub(super) fn sources_of(items: &[LoadoutItem], uses: &[&LoadoutUse]) -> Vec<String> {
    let all: Vec<(LoadoutSource, Option<&str>)> = items.iter().map(|one| (one.source, one.from.as_deref())).chain(uses.iter().map(|one| (one.source, one.from.as_deref()))).collect();
    SOURCE_ORDER
        .iter()
        .flat_map(|source| {
            let mut said: Vec<String> = all.iter().filter(|one| one.0 == *source).map(|one| source_said(one.0, one.1)).collect();
            said.sort_by_key(|one| one.to_lowercase());
            said.dedup();
            said
        })
        .collect()
}

// ---- last used, search, filter and sort ------------------------------------------------------

/// One row of a kind's group: an item the agent reported, or a use that matches none, so nothing used goes unlisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub item: LoadoutItem,
    /// A turn of this session used it.
    pub used: bool,
    /// The start of the latest turn that used it, where its instant reads.
    pub last: Option<DateTime<Utc>>,
}

/// The use's row among `rows`: the one of its kind and name, and where two share both, the one from its source.
fn row_of_use(rows: &[Row], one: &LoadoutUse) -> Option<usize> {
    let kind = kind_of_use(one.kind);
    let named: Vec<usize> = rows.iter().enumerate().filter(|(_, row)| row.item.kind == kind && row.item.name == one.name).map(|(at, _)| at).collect();
    if named.len() < 2 {
        return named.first().copied();
    }
    let same = |at: &&usize, from: bool| rows[**at].item.source == one.source && (!from || rows[**at].item.from == one.from);
    named.iter().find(|at| same(at, true)).or_else(|| named.iter().find(|at| same(at, false))).or(named.first()).copied()
}

/// Every reported item as a row with when it was last used, then each use that matches no item as a row of its own.
pub(super) fn rows_of(items: &[LoadoutItem], turns: &[&LoadoutTurn]) -> Vec<Row> {
    let mut rows: Vec<Row> = items.iter().map(|item| Row { item: item.clone(), used: false, last: None }).collect();
    for turn in turns {
        for one in &turn.used {
            let at = [one.turn_started_at.as_str(), turn.started_at.as_str()].into_iter().find_map(|at| DateTime::parse_from_rfc3339(at).ok()).map(|at| at.with_timezone(&Utc));
            let found = row_of_use(&rows, one).unwrap_or_else(|| {
                let item = LoadoutItem { kind: kind_of_use(one.kind), name: one.name.clone(), source: one.source, from: one.from.clone(), state: None, tokens: None };
                rows.push(Row { item, used: false, last: None });
                rows.len() - 1
            });
            let row = &mut rows[found];
            row.used = true;
            row.last = row.last.max(at);
        }
    }
    rows
}

/// When a row was last used: the clock today, the day before that, `never` where no turn used it.
pub(super) fn last_used_said(row: &Row, now: DateTime<Local>, format: TimeFormat) -> String {
    match row.last.map(|at| at.with_timezone(&Local)) {
        Some(at) if at.date_naive() == now.date_naive() => crate::settings::time::time_of(at, TimeFormat { seconds: false, ..format }),
        Some(at) if at.year() == now.year() => at.format("%b %-d").to_string(),
        Some(at) => at.format("%b %-d %Y").to_string(),
        None if row.used => "this session".into(),
        None => "never".into(),
    }
}

/// How each group's rows are ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Sort {
    #[default]
    Name,
    LastUsed,
}

impl Sort {
    pub const ALL: [Sort; 2] = [Sort::Name, Sort::LastUsed];

    pub fn said(self) -> &'static str {
        match self {
            Sort::Name => "Name",
            Sort::LastUsed => "Last used",
        }
    }

    /// A menu row's id, since a menu's ids live as long as the app.
    pub fn id(self) -> &'static str {
        match self {
            Sort::Name => "sort-name",
            Sort::LastUsed => "sort-last-used",
        }
    }
}

/// What narrows every group: the source picked, the words searched for, and Used this session.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Narrow<'a> {
    pub source: Option<&'a str>,
    pub query: &'a str,
    pub used_only: bool,
}

impl Narrow<'_> {
    /// Whether anything narrows the list, so an empty group says nothing matched rather than nothing reported.
    pub fn any(&self) -> bool {
        self.source.is_some() || !self.query.trim().is_empty() || self.used_only
    }

    /// Whether a name is one the search finds: what was typed, in any case, anywhere in it.
    pub fn finds(&self, name: &str) -> bool {
        let query = self.query.trim().to_lowercase();
        query.is_empty() || name.to_lowercase().contains(&query)
    }

    /// Whether a row from `source` stays.
    pub fn keeps(&self, name: &str, source: &str, used: bool) -> bool {
        self.source.is_none_or(|picked| picked == source) && self.finds(name) && (used || !self.used_only)
    }
}

/// One kind's rows under the narrowing, in the order picked; the groups keep theirs.
pub(super) fn kept_rows<'a>(rows: &'a [Row], kind: LoadoutKind, narrow: Narrow<'_>, sort: Sort) -> Vec<&'a Row> {
    let mut kept: Vec<&Row> = rows.iter().filter(|row| row.item.kind == kind && narrow.keeps(&row.item.name, &source_said(row.item.source, row.item.from.as_deref()), row.used)).collect();
    let by_name = |a: &&Row, b: &&Row| a.item.name.to_lowercase().cmp(&b.item.name.to_lowercase());
    match sort {
        Sort::Name => kept.sort_by(by_name),
        // Latest first; a use whose instant did not read before never used.
        Sort::LastUsed => kept.sort_by(|a, b| b.last.cmp(&a.last).then(b.used.cmp(&a.used)).then_with(|| by_name(a, b))),
    }
    kept
}

/// Which item a Loadout row stands for, wherever it is listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ItemRef {
    pub kind: LoadoutKind,
    pub name: String,
    pub source: LoadoutSource,
    pub from: Option<String>,
}

impl ItemRef {
    pub fn of(item: &LoadoutItem) -> ItemRef {
        ItemRef { kind: item.kind, name: item.name.clone(), source: item.source, from: item.from.clone() }
    }

    /// One identity for an item wherever it is listed, so a use and the item it used are the same row.
    pub fn key(&self) -> String {
        format!("{}:{}:{}:{}", kind_wire(self.kind), source_wire(self.source), self.from.as_deref().unwrap_or(""), self.name)
    }

    /// The item a key names, as an item's tab keeps it between runs. None where the key is not one.
    pub fn of_key(key: &str) -> Option<ItemRef> {
        let mut parts = key.splitn(4, ':');
        let (kind, source, from, name) = (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        let kind = KNOWN.into_iter().chain(TOLD).find(|one| kind_wire(*one) == kind)?;
        let source = SOURCES.into_iter().find(|one| source_wire(*one) == source)?;
        (!name.is_empty()).then(|| ItemRef { kind, name: name.to_string(), source, from: (!from.is_empty()).then(|| from.to_string()) })
    }
}

/// Every source an item can come from.
const SOURCES: [LoadoutSource; 7] =
    [LoadoutSource::Kit, LoadoutSource::Armada, LoadoutSource::Repository, LoadoutSource::Plugin, LoadoutSource::Mcp, LoadoutSource::BuiltIn, LoadoutSource::Managed];

pub(super) fn kind_wire(kind: LoadoutKind) -> &'static str {
    match kind {
        LoadoutKind::Skill => "skill",
        LoadoutKind::Subagent => "subagent",
        LoadoutKind::McpServer => "mcp_server",
        LoadoutKind::Tool => "tool",
        LoadoutKind::MemoryFile => "memory_file",
        LoadoutKind::OutputStyle => "output_style",
        LoadoutKind::Rules => "rules",
        LoadoutKind::Injection => "injection",
    }
}

pub(super) fn source_wire(source: LoadoutSource) -> &'static str {
    match source {
        LoadoutSource::Kit => "kit",
        LoadoutSource::Armada => "armada",
        LoadoutSource::Repository => "repository",
        LoadoutSource::Plugin => "plugin",
        LoadoutSource::Mcp => "mcp",
        LoadoutSource::BuiltIn => "built_in",
        LoadoutSource::Managed => "managed",
    }
}

/// The Loadout item a live server stands for, so a press opens the same panel the agent's report would (`mcpRefOf`).
pub(super) fn mcp_ref_of(name: &str, items: &[LoadoutItem]) -> ItemRef {
    match items.iter().find(|one| one.kind == LoadoutKind::McpServer && one.name == name) {
        Some(reported) => ItemRef { kind: LoadoutKind::McpServer, name: name.to_string(), source: reported.source, from: reported.from.clone() },
        None => ItemRef { kind: LoadoutKind::McpServer, name: name.to_string(), source: LoadoutSource::Mcp, from: Some(name.to_string()) },
    }
}

/// The tools an MCP server gave, in the order the Loadout lists them: its own by `from`, or by the `mcp__<server>__` prefix.
pub(super) fn tools_of<'a>(server: &ItemRef, items: &'a [LoadoutItem]) -> Vec<&'a LoadoutItem> {
    let named = server.from.as_deref().unwrap_or(&server.name);
    let prefix = format!("mcp__{}__", server.name);
    items.iter().filter(|one| one.kind == LoadoutKind::Tool && (one.from.as_deref() == Some(named) || one.name.starts_with(&prefix))).collect()
}

/// A tool's state as its row in an MCP server's panel says it.
pub(super) fn tool_state_said(state: Option<LoadoutState>) -> &'static str {
    match state {
        Some(LoadoutState::Loaded) => "Loaded",
        Some(LoadoutState::Deferred) => "Deferred",
        _ => "",
    }
}

/// What moves when the Job reports a new list or a new use: a change re-reads the open Loadout.
pub(super) fn summary_key(summary: Option<&LoadoutSummary>) -> String {
    summary.map_or_else(String::new, |summary| {
        let latest = summary.latest_turn.as_ref();
        format!("{}/{}/{}/{}", summary.knows, summary.told, latest.map_or("", |turn| turn.started_at.as_str()), latest.map_or(0, |turn| turn.used.len()))
    })
}

/// Whether a message is `/mcp` alone in a hosted Job, which opens the Loadout on its MCP servers instead of going to the
/// agent (`opensMcp`). A terminal Job's goes as typed, since its own terminal draws the CLI's screen.
pub(super) fn opens_mcp(hosted: bool, text: &str, files: usize) -> bool {
    hosted && files == 0 && text.trim() == "/mcp"
}

/// A SKILL.md or an agent file opens on its front matter, which markdown would draw as a rule and a heading: it is
/// handed over as the fenced block it reads as (`fencedFrontMatter`).
pub(super) fn fenced_front_matter(body: &str) -> String {
    let lines: Vec<&str> = body.split('\n').collect();
    if lines.first().map(|line| line.trim_end_matches('\r')) != Some("---") {
        return body.to_string();
    }
    let Some(close) = lines.iter().enumerate().skip(2).find(|(_, line)| line.trim_end() == "---").map(|(at, _)| at) else { return body.to_string() };
    let inner = lines[1..close].join("\n");
    let inner = inner.trim_end_matches('\r');
    let longest = inner.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(3));
    format!("{fence}yaml\n{inner}\n{fence}\n\n{}", lines[close + 1..].join("\n"))
}

// ---- MCP servers ---------------------------------------------------------------------------

/// Every act a server's state takes, the face's first. No sign-out: the CLI has none.
pub(super) fn mcp_acts_of(state: McpServerState, signed_in: bool) -> &'static [McpServerAct] {
    use McpServerAct::*;
    match state {
        McpServerState::Connected => &[TurnOff, Reconnect],
        McpServerState::Failed => &[Reconnect, TurnOff],
        McpServerState::NeedsSignIn if signed_in => &[Reconnect, SignIn, TurnOff],
        McpServerState::NeedsSignIn => &[SignIn, Reconnect, TurnOff],
        McpServerState::Off => &[TurnOn],
        McpServerState::Connecting => &[],
    }
}

/// An act's word on its button, and while it is out.
pub(super) fn act_said(act: McpServerAct) -> (&'static str, &'static str) {
    match act {
        McpServerAct::Reconnect => ("Reconnect", "Reconnecting"),
        McpServerAct::TurnOn => ("Turn on", "Turning on"),
        McpServerAct::TurnOff => ("Turn off", "Turning off"),
        McpServerAct::SignIn => ("Sign in", "Signing in"),
    }
}

/// A menu row's id for an act, since a menu's ids live as long as the app.
pub(super) fn act_id(act: McpServerAct) -> &'static str {
    match act {
        McpServerAct::Reconnect => "reconnect",
        McpServerAct::TurnOn => "turn_on",
        McpServerAct::TurnOff => "turn_off",
        McpServerAct::SignIn => "sign_in",
    }
}

pub(super) fn act_of_id(id: &str) -> Option<McpServerAct> {
    [McpServerAct::Reconnect, McpServerAct::TurnOn, McpServerAct::TurnOff, McpServerAct::SignIn].into_iter().find(|act| act_id(*act) == id)
}

/// What the CLI said of a server: its tool count, or the words of a failure.
pub(super) fn server_said(server: &SessionMcpServer) -> String {
    match server.state {
        McpServerState::Connected if server.tools == 1 => "1 tool".into(),
        McpServerState::Connected => format!("{} tools", server.tools),
        McpServerState::Failed => server.error.clone().unwrap_or_else(|| "failed".into()),
        McpServerState::Connecting => "connecting".into(),
        McpServerState::NeedsSignIn => "needs sign-in".into(),
        McpServerState::Off => "off".into(),
    }
}

/// A state a row marks, from the agent's report, the live CLI, or a skill removed from this Mac.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mark {
    Connected,
    Failed,
    Connecting,
    NeedsSignIn,
    Off,
    Removed,
}

impl Mark {
    pub fn of_report(state: Option<LoadoutState>) -> Option<Mark> {
        match state? {
            LoadoutState::Connected => Some(Mark::Connected),
            LoadoutState::Failed => Some(Mark::Failed),
            LoadoutState::Pending => Some(Mark::Connecting),
            LoadoutState::Deferred | LoadoutState::Loaded => None,
        }
    }

    pub fn of_live(state: McpServerState) -> Mark {
        match state {
            McpServerState::Connected => Mark::Connected,
            McpServerState::Failed => Mark::Failed,
            McpServerState::Connecting => Mark::Connecting,
            McpServerState::NeedsSignIn => Mark::NeedsSignIn,
            McpServerState::Off => Mark::Off,
        }
    }

    /// Its registry glyph, its tooltip, and the hue on the glyph.
    pub fn look(self) -> (&'static str, &'static str, Token) {
        match self {
            Mark::Connected => ("plug", "Connected", token::STATUS_COMPLETED_SUCCESS),
            Mark::Failed => ("unplug", "Failed", token::STATUS_COMPLETED_FAILED),
            Mark::Connecting => ("clock", "Connecting", token::STATUS_RUNNING),
            Mark::NeedsSignIn => ("log-in", "Needs sign-in", token::STATUS_AWAITING_REVIEW),
            Mark::Off => ("toggle-left", "Off", token::FG_SUBTLE),
            Mark::Removed => ("trash", "Removed", token::FG_MUTED),
        }
    }
}

/// **An act's answer wins** over a read of a hosted Job's servers: a read that answers while an act is out, or that was
/// sent before an act landed, is dropped, since the process reports a server mid-reconnect until the act settles (`lands`).
#[derive(Clone, Debug, Default)]
pub(super) struct McpGate {
    asked: u64,
    landed: u64,
    out: u32,
}

impl McpGate {
    /// A read goes out: its number.
    pub fn ask(&mut self) -> u64 {
        self.asked += 1;
        self.asked
    }

    /// Whether read `seq`'s answer is taken.
    pub fn lands(&mut self, seq: u64) -> bool {
        if self.out > 0 || seq < self.landed {
            return false;
        }
        self.landed = seq;
        true
    }

    pub fn act_sent(&mut self) {
        self.out += 1;
    }

    /// The act answered; one that was taken makes every read sent before it stale.
    pub fn act_answered(&mut self, taken: bool) {
        self.out = self.out.saturating_sub(1);
        if taken {
            self.landed = self.asked;
        }
    }
}

// ---- skills Kit holds ----------------------------------------------------------------------

/// What a skill row can be asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SkillAct {
    Edit,
    Remove,
    RemovePlugin,
    Restore,
}

/// The plugin a skill came with, by its name without the marketplace, and the skills Remove plugin takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PluginActs {
    pub name: String,
    pub uninstall: Option<String>,
    pub takes: Vec<String>,
}

/// What Kit's inventory lets one skill row do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SkillActs {
    /// The SKILL.md Edit opens.
    pub edits: Option<String>,
    /// One of the person's own: Remove puts its folder in the Trash.
    pub removes: bool,
    pub plugin: Option<PluginActs>,
}

/// A skill row Kit holds: what it offers, the name Kit knows it by, and the bodies Remove and Remove plugin send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KitSkillMatch {
    pub acts: SkillActs,
    /// Its name and plugin as the inventory spells them, which Edit opens it by.
    pub kit_name: String,
    pub kit_plugin: Option<String>,
    pub remove: Option<RemoveKitPiece>,
    pub remove_plugin: Option<RemoveKitPiece>,
}

fn items_of<'a>(setup: &'a KitInventory, kind: &str) -> &'a [SetupItem] {
    match setup.kinds.iter().find(|one| one.kind == kind).map(|one| &one.read) {
        Some(WhatWasRead::Read { items, .. }) => items,
        _ => &[],
    }
}

/// The name a row has in the inventory, and the plugin it came with: a plugin's skill is spelt `plugin:name` in the
/// Loadout and `name` in the inventory.
fn inventory_name_of(item: &ItemRef) -> Option<(String, Option<String>)> {
    match item.source {
        LoadoutSource::Kit => Some((item.name.clone(), None)),
        LoadoutSource::Plugin => {
            let plugin = item.from.clone().filter(|from| !from.is_empty()).or_else(|| item.name.split(':').next().map(str::to_string)).filter(|plugin| !plugin.is_empty())?;
            let name = item.name.strip_prefix(&format!("{plugin}:")).unwrap_or(&item.name).to_string();
            Some((name, Some(plugin)))
        }
        _ => None,
    }
}

/// The one installed plugin a skill came with. None where no plugin, or two, have that name.
fn plugin_of(plugin: &str, setup: &KitInventory) -> Option<(PluginActs, RemoveKitPiece)> {
    let found: Vec<&SetupItem> = items_of(setup, "plugins").iter().filter(|one| one.name.split('@').next().unwrap_or(&one.name) == plugin).collect();
    let [one] = found.as_slice() else { return None };
    let takes = items_of(setup, "skills").iter().filter(|skill| skill.plugin.as_deref() == Some(plugin)).map(|skill| skill.name.clone()).collect();
    Some((PluginActs { name: plugin.to_string(), uninstall: one.uninstall.clone(), takes }, RemoveKitPiece { kind: ipc::KitPieceKind::Plugin, name: one.name.clone() }))
}

fn match_of(item: &ItemRef, setup: &KitInventory) -> Option<KitSkillMatch> {
    if item.kind != LoadoutKind::Skill {
        return None;
    }
    let (name, plugin) = inventory_name_of(item)?;
    let found: Vec<&SetupItem> = items_of(setup, "skills").iter().filter(|one| one.name == name && one.plugin == plugin).collect();
    let [one] = found.as_slice() else { return None };
    let path = crate::kit::source_path(&one.source, &setup.home)?;
    let edits = Some(format!("{path}/SKILL.md"));
    // A plugin's own skill goes with its plugin and never alone.
    if let Some(plugin) = plugin {
        let brought = plugin_of(&plugin, setup);
        let (plugin_acts, remove_plugin) = brought.map_or((None, None), |(acts, body)| (Some(acts), Some(body)));
        return Some(KitSkillMatch { acts: SkillActs { edits, removes: false, plugin: plugin_acts }, kit_name: name, kit_plugin: Some(plugin), remove: None, remove_plugin });
    }
    Some(KitSkillMatch {
        acts: SkillActs { edits, removes: true, plugin: None },
        kit_name: name.clone(),
        kit_plugin: None,
        remove: Some(RemoveKitPiece { kind: ipc::KitPieceKind::Skill, name }),
        remove_plugin: None,
    })
}

/// What each skill row offers, by [`ItemRef::key`]. A row Kit does not hold is absent (`skillActsByKit`).
pub(super) fn skill_acts_by_kit(items: &[LoadoutItem], setup: &KitInventory) -> HashMap<String, KitSkillMatch> {
    items.iter().map(ItemRef::of).filter_map(|item| match_of(&item, setup).map(|found| (item.key(), found))).collect()
}

/// The rows Remove plugin takes: every row Kit matched to that plugin.
pub(super) fn rows_of_plugin(matched: &HashMap<String, KitSkillMatch>, plugin: &RemoveKitPiece) -> Vec<String> {
    let mut rows: Vec<String> = matched.iter().filter(|(_, one)| one.remove_plugin.as_ref().is_some_and(|body| body.name == plugin.name)).map(|(key, _)| key.clone()).collect();
    rows.sort();
    rows
}

/// What one skill row offers on its face and behind its caret, as Classic's `SkillActs` lays them out. `removed` is
/// `Some(restorable)` once it is gone from this Mac.
pub(super) fn skill_face(acts: &SkillActs, removed: Option<bool>) -> (Option<SkillAct>, Vec<SkillAct>) {
    if let Some(restorable) = removed {
        return (restorable.then_some(SkillAct::Restore), Vec::new());
    }
    if acts.edits.is_none() {
        return (None, Vec::new());
    }
    let mut menu = Vec::new();
    if acts.removes {
        menu.push(SkillAct::Remove);
    }
    if acts.plugin.is_some() {
        menu.push(SkillAct::RemovePlugin);
    }
    (Some(SkillAct::Edit), menu)
}

/// The word an act out shows on the face.
pub(super) fn skill_underway(act: SkillAct) -> &'static str {
    match act {
        SkillAct::Remove | SkillAct::RemovePlugin => "Removing",
        SkillAct::Restore => "Restoring",
        SkillAct::Edit => "Edit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: LoadoutKind, name: &str, source: LoadoutSource, from: Option<&str>) -> LoadoutItem {
        LoadoutItem { kind, name: name.into(), source, from: from.map(str::to_string), state: None, tokens: None }
    }

    fn setup_item(name: &str, source: &str, plugin: Option<&str>, uninstall: Option<&str>) -> SetupItem {
        SetupItem { name: name.into(), says: None, source: source.into(), style: None, plugin: plugin.map(str::to_string), uninstall: uninstall.map(str::to_string), server: None }
    }

    fn inventory(skills: Vec<SetupItem>, plugins: Vec<SetupItem>) -> KitInventory {
        let kind = |kind: &str, items| ipc::SetupKindRow { kind: kind.into(), read: WhatWasRead::Read { items, unreadable: Vec::new() } };
        KitInventory { harness: "Harness".into(), home: "/Users/a/.harness".into(), present: true, kinds: vec![kind("skills", skills), kind("plugins", plugins)] }
    }

    #[test]
    fn each_mcp_state_offers_its_acts_with_the_face_first_as_mcp_acts_of() {
        use McpServerAct::*;
        assert_eq!(mcp_acts_of(McpServerState::Connected, false), [TurnOff, Reconnect]);
        assert_eq!(mcp_acts_of(McpServerState::Failed, false), [Reconnect, TurnOff]);
        assert_eq!(mcp_acts_of(McpServerState::NeedsSignIn, false), [SignIn, Reconnect, TurnOff]);
        assert_eq!(mcp_acts_of(McpServerState::NeedsSignIn, true), [Reconnect, SignIn, TurnOff], "once its sign-in page opened, Reconnect is its face");
        assert_eq!(mcp_acts_of(McpServerState::Off, false), [TurnOn]);
        assert!(mcp_acts_of(McpServerState::Connecting, false).is_empty());
        assert_eq!(act_said(SignIn), ("Sign in", "Signing in"));
        assert_eq!(act_said(Reconnect), ("Reconnect", "Reconnecting"));
    }

    #[test]
    fn a_read_answering_while_an_act_is_out_or_sent_before_it_landed_is_dropped() {
        let mut gate = McpGate::default();
        let first = gate.ask();
        assert!(gate.lands(first), "a read with nothing out lands");
        let before = gate.ask();
        gate.act_sent();
        let during = gate.ask();
        assert!(!gate.lands(during), "a read answering while the act is out is dropped");
        gate.act_answered(true);
        assert!(!gate.lands(before), "a read sent before the act landed is dropped");
        let after = gate.ask();
        assert!(gate.lands(after), "a read sent after it lands");
        // A refused act takes nothing from the reads.
        let mut gate = McpGate::default();
        let sent = gate.ask();
        gate.act_sent();
        gate.act_answered(false);
        assert!(gate.lands(sent));
    }

    #[test]
    fn a_kit_skill_offers_edit_then_remove_and_a_plugins_skill_remove_plugin_taking_all_its_rows() {
        let items = vec![
            item(LoadoutKind::Skill, "tidy", LoadoutSource::Kit, None),
            item(LoadoutKind::Skill, "docs:write", LoadoutSource::Plugin, Some("docs")),
            item(LoadoutKind::Skill, "docs:read", LoadoutSource::Plugin, Some("docs")),
            item(LoadoutKind::Skill, "repo-only", LoadoutSource::Repository, None),
        ];
        let setup = inventory(
            vec![setup_item("tidy", "skills/tidy", None, None), setup_item("write", "plugins/docs/skills/write", Some("docs"), None), setup_item("read", "plugins/docs/skills/read", Some("docs"), None)],
            vec![setup_item("docs@market", "plugins/docs", None, Some("harness plugin uninstall docs@market"))],
        );
        let matched = skill_acts_by_kit(&items, &setup);
        assert_eq!(matched.len(), 3, "a repository's skill is not Kit's to change");
        let tidy = &matched[&ItemRef::of(&items[0]).key()];
        assert_eq!(tidy.acts.edits.as_deref(), Some("/Users/a/.harness/skills/tidy/SKILL.md"));
        assert_eq!(skill_face(&tidy.acts, None), (Some(SkillAct::Edit), vec![SkillAct::Remove]));
        assert_eq!(skill_face(&tidy.acts, Some(true)), (Some(SkillAct::Restore), vec![]), "once removed, Restore alone");
        assert_eq!(skill_face(&tidy.acts, Some(false)), (None, vec![]), "nothing restores what the Trash no longer holds");
        let write = &matched[&ItemRef::of(&items[1]).key()];
        assert_eq!(skill_face(&write.acts, None), (Some(SkillAct::Edit), vec![SkillAct::RemovePlugin]));
        assert_eq!(write.acts.plugin.as_ref().map(|plugin| plugin.takes.clone()), Some(vec!["write".to_string(), "read".to_string()]));
        let body = write.remove_plugin.clone().expect("Remove plugin's body");
        assert_eq!(body.name, "docs@market");
        let mut taken = vec![ItemRef::of(&items[1]).key(), ItemRef::of(&items[2]).key()];
        taken.sort();
        assert_eq!(rows_of_plugin(&matched, &body), taken);
    }

    #[test]
    fn mcp_alone_in_a_hosted_job_opens_the_loadout_and_anything_else_is_a_message() {
        assert!(opens_mcp(true, "/mcp", 0));
        assert!(opens_mcp(true, " /mcp ", 0));
        assert!(opens_mcp(true, "  /mcp \n", 0));
        assert!(!opens_mcp(false, "/mcp", 0));
        assert!(!opens_mcp(true, "/mcp", 1), "with a file it is a message");
        for message in ["/mcp list", "/mcpx", "hi /mcp", "use /mcp"] {
            assert!(!opens_mcp(true, message, 0), "{message} goes to the agent");
        }
    }

    #[test]
    fn sources_are_listed_kit_first_and_plugins_by_name() {
        let items = vec![
            item(LoadoutKind::Skill, "b:x", LoadoutSource::Plugin, Some("beta")),
            item(LoadoutKind::Skill, "y", LoadoutSource::Kit, None),
            item(LoadoutKind::Skill, "a:x", LoadoutSource::Plugin, Some("alpha")),
            item(LoadoutKind::Tool, "Read", LoadoutSource::BuiltIn, None),
        ];
        assert_eq!(sources_of(&items, &[]), ["Kit", "alpha", "beta", "Built-in"]);
    }

    #[test]
    fn a_skills_front_matter_reads_as_a_yaml_block_and_a_body_without_any_is_kept() {
        let read = fenced_front_matter("---\nname: tidy\ndescription: Tidies.\n---\n\n# Tidy\n");
        assert_eq!(read, "```yaml\nname: tidy\ndescription: Tidies.\n```\n\n\n# Tidy\n");
        assert_eq!(fenced_front_matter("# Tidy\n---\n"), "# Tidy\n---\n", "a rule further down is not front matter");
        assert_eq!(fenced_front_matter("---\nname: x\n"), "---\nname: x\n", "front matter never closed is left alone");
        assert!(fenced_front_matter("---\nsays: ```\n---\nbody").starts_with("````yaml\n"), "the fence is longer than any run inside");
    }

    fn turn(at: &str, used: Vec<(LoadoutUseKind, &str, LoadoutSource, Option<&str>)>) -> LoadoutTurn {
        let at = ipc::Instant::carried(at.to_string());
        let used = used
            .into_iter()
            .map(|(kind, name, source, from)| LoadoutUse { turn_started_at: at.clone(), kind, name: name.into(), source, from: from.map(str::to_string), model: None, by: None, count: None })
            .collect();
        LoadoutTurn { started_at: at, used }
    }

    fn row(kind: LoadoutKind, name: &str, last: Option<&str>) -> Row {
        let last = last.map(|at| DateTime::parse_from_rfc3339(at).expect("an instant").with_timezone(&Utc));
        Row { item: item(kind, name, LoadoutSource::Kit, None), used: last.is_some(), last }
    }

    #[test]
    fn a_use_finds_its_row_by_kind_and_name_and_a_use_of_nothing_listed_gets_a_row_of_its_own() {
        let items = vec![
            item(LoadoutKind::Skill, "tidy", LoadoutSource::Kit, None),
            item(LoadoutKind::Tool, "tidy", LoadoutSource::BuiltIn, None),
            item(LoadoutKind::Skill, "write", LoadoutSource::Plugin, Some("alpha")),
            item(LoadoutKind::Skill, "write", LoadoutSource::Plugin, Some("beta")),
            item(LoadoutKind::Subagent, "explorer", LoadoutSource::Armada, None),
        ];
        let early = turn("2026-11-17T09:00:00Z", vec![(LoadoutUseKind::Skill, "tidy", LoadoutSource::Kit, None), (LoadoutUseKind::Tool, "Bash", LoadoutSource::BuiltIn, None)]);
        let late = turn(
            "2026-11-17T10:30:00Z",
            vec![(LoadoutUseKind::Skill, "tidy", LoadoutSource::Repository, None), (LoadoutUseKind::Skill, "write", LoadoutSource::Plugin, Some("beta")), (LoadoutUseKind::Tool, "Bash", LoadoutSource::BuiltIn, None)],
        );
        let rows = rows_of(&items, &[&early, &late]);
        let at = |at: &str| Some(DateTime::parse_from_rfc3339(at).expect("an instant").with_timezone(&Utc));
        assert_eq!(rows.len(), 6, "Bash is listed nowhere and gets one row, however often it was used");
        assert_eq!((rows[0].used, rows[0].last), (true, at("2026-11-17T10:30:00Z")), "the skill's own name, whatever source the use says, and its latest turn");
        assert!(!rows[1].used, "a tool of the same name is another row");
        assert!(!rows[2].used, "of two skills with one name, the use's source picks the other");
        assert_eq!(rows[3].last, at("2026-11-17T10:30:00Z"));
        assert!(!rows[4].used);
        assert_eq!((rows[5].item.kind, rows[5].item.name.as_str(), rows[5].item.source), (LoadoutKind::Tool, "Bash", LoadoutSource::BuiltIn));
        assert_eq!(rows[5].last, at("2026-11-17T10:30:00Z"));
        // A use whose instant does not read is still a use.
        let unread = turn("not an instant", vec![(LoadoutUseKind::Subagent, "explorer", LoadoutSource::Armada, None)]);
        let rows = rows_of(&items, &[&unread]);
        assert_eq!((rows[4].used, rows[4].last), (true, None));
    }

    #[test]
    fn last_used_says_the_clock_today_the_day_before_and_never() {
        use chrono::TimeZone;
        let now = Local.with_ymd_and_hms(2026, 11, 18, 14, 2, 9).single().expect("a time");
        let used_at = |at: DateTime<Local>| Row { item: item(LoadoutKind::Skill, "tidy", LoadoutSource::Kit, None), used: true, last: Some(at.with_timezone(&Utc)) };
        let twelve = TimeFormat { hour12: true, seconds: true, date: true };
        let today = Local.with_ymd_and_hms(2026, 11, 18, 9, 43, 46).single().expect("a time");
        assert_eq!(last_used_said(&used_at(today), now, twelve), "9:43 AM", "today is its clock alone, without seconds or day");
        assert_eq!(last_used_said(&used_at(today), now, TimeFormat { hour12: false, seconds: true, date: false }), "09:43", "in the clock he set");
        let yesterday = Local.with_ymd_and_hms(2026, 11, 17, 23, 59, 0).single().expect("a time");
        assert_eq!(last_used_said(&used_at(yesterday), now, twelve), "Nov 17");
        let last_year = Local.with_ymd_and_hms(2025, 12, 30, 10, 0, 0).single().expect("a time");
        assert_eq!(last_used_said(&used_at(last_year), now, twelve), "Dec 30 2025");
        let mut never = used_at(today);
        (never.used, never.last) = (false, None);
        assert_eq!(last_used_said(&never, now, twelve), "never");
        never.used = true;
        assert_eq!(last_used_said(&never, now, twelve), "this session");
    }

    #[test]
    fn search_source_and_used_this_session_narrow_each_kind_and_sort_orders_within_it() {
        let mut rows = vec![
            row(LoadoutKind::Skill, "release-notes", None),
            row(LoadoutKind::Skill, "Docs:write", Some("2026-11-17T09:00:00Z")),
            row(LoadoutKind::Skill, "tidy", Some("2026-11-17T10:30:00Z")),
            row(LoadoutKind::Skill, "audit", None),
            row(LoadoutKind::Tool, "Read", Some("2026-11-17T11:00:00Z")),
        ];
        rows[3].item.source = LoadoutSource::Repository;
        let names = |kept: Vec<&Row>| kept.into_iter().map(|row| row.item.name.clone()).collect::<Vec<_>>();
        let all = Narrow::default();
        assert!(!all.any());
        assert_eq!(names(kept_rows(&rows, LoadoutKind::Skill, all, Sort::Name)), ["audit", "Docs:write", "release-notes", "tidy"], "by name in any case, Tools apart");
        assert_eq!(names(kept_rows(&rows, LoadoutKind::Skill, all, Sort::LastUsed)), ["tidy", "Docs:write", "audit", "release-notes"], "latest first, never used last by name");
        let searched = Narrow { query: " DOCS ", ..all };
        assert!(searched.any());
        assert_eq!(names(kept_rows(&rows, LoadoutKind::Skill, searched, Sort::Name)), ["Docs:write"], "any case, anywhere in the name");
        let used = Narrow { used_only: true, ..all };
        assert_eq!(names(kept_rows(&rows, LoadoutKind::Skill, used, Sort::Name)), ["Docs:write", "tidy"]);
        let repository = Narrow { source: Some("Repository"), ..all };
        assert_eq!(names(kept_rows(&rows, LoadoutKind::Skill, repository, Sort::Name)), ["audit"]);
        assert!(kept_rows(&rows, LoadoutKind::Skill, Narrow { used_only: true, ..repository }, Sort::Name).is_empty(), "every narrowing holds at once");
    }

    #[test]
    fn an_items_key_names_it_again_after_a_restart() {
        let skill = ItemRef { kind: LoadoutKind::Skill, name: "docs:write-page".into(), source: LoadoutSource::Plugin, from: Some("docs".into()) };
        let tool = ItemRef { kind: LoadoutKind::Tool, name: "Read".into(), source: LoadoutSource::BuiltIn, from: None };
        for item in [skill, tool] {
            assert_eq!(ItemRef::of_key(&item.key()), Some(item.clone()), "a name with a colon in it too");
        }
        for not_one in ["", "skill:kit:", "skill:kit", "wand:kit::tidy", "skill:shelf::tidy"] {
            assert_eq!(ItemRef::of_key(not_one), None, "{not_one:?}");
        }
    }

    #[test]
    fn every_glyph_the_loadout_draws_is_in_the_icon_set() {
        use gpui::AssetSource;
        let has = |name: &str| crate::assets::Assets.load(&crate::assets::icon_path(name)).ok().flatten().is_some();
        let kinds = KNOWN.into_iter().chain(TOLD).map(|kind| kind_look(kind).0);
        let marks = [Mark::Connected, Mark::Failed, Mark::Connecting, Mark::NeedsSignIn, Mark::Off, Mark::Removed].map(|mark| mark.look().0);
        for name in kinds.chain(marks).chain(["layers", "chevron-down", "x", "search", "arrow-up-down"]) {
            assert!(has(name), "{name} is in the icon set");
        }
    }
}
