//! The mods on this machine, one row each: its kind, whether it is on, what is wrong with it if anything, and
//! Promote. Classic's `ModList` over `ModRow` and `ModRules` (`packages/surfaces/settings/src/ModList.tsx`,
//! `packages/components/src/compositions/ModRow/`, `ModRules/`). Theme lists the theme mods; All lists the theme in
//! force with the Fleet and tracker mods; Tracker and Fleet list theirs. A Fleet or tracker row is a taught one
//! (`fleet-taught.ts`): what it says comes from the file Fleet checked for it, and a refusal is a toast.
//!
//! **Not here:** the shared home's marks, which native does not read, and the rows of the kinds native does not
//! apply yet (tweak, layout, words, commands, tab, instrument), which come back with the mod-kit group.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{canvas, div, prelude::*, px, svg, Pixels, AnyElement, App, Context, Entity, FocusHandle, Focusable, Hsla, KeyDownEvent, Keystroke, Role, SharedString, Subscription, Window};
use ipc::{Condition, Does, FleetFile, FleetRule, ModChecked, ModKind, ModOrigin, ModPromoted, ModSummary, RuleScope, TrackerTry};

use super::overlay::SettingsOverlay;
use super::view::{card, ghost, ringed, switch_track, tip};
use crate::assets::icon_path;
use crate::job_view::{notice_name, ModPanel, ModPanelAsked, OpenJobFrom};
use crate::labels::labelled;
use crate::look::{token, Look};

/// A mod Armada ships is named `built-in:` and its folder's name; its title is the folder's name.
const BUILT_IN: &str = "built-in:";
pub const BUILT_IN_SAID: &str = "Ships with Armada";

/// What the mod changes, in words (`MOD_KIND_SAID`).
pub fn kind_said(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Theme => "Theme",
        ModKind::Tweak => "Theme tweak",
        ModKind::Layout => "Layout",
        ModKind::Words => "Words",
        ModKind::Fleet => "Fleet",
        ModKind::Tracker => "Tracker",
        ModKind::Commands => "Commands",
        ModKind::Tab => "Tab",
        ModKind::Instrument => "Instrument",
    }
}

/// The kind's glyph, by its name in the icon registry (`GLYPH`).
fn kind_glyph(kind: ModKind) -> &'static str {
    match kind {
        ModKind::Theme => "palette",
        ModKind::Tweak => "eclipse",
        ModKind::Layout => "panels-top-left",
        ModKind::Words => "link",
        ModKind::Fleet => "bell",
        ModKind::Tracker => "ticket",
        ModKind::Commands => "command",
        ModKind::Tab => "panel-top",
        ModKind::Instrument => "gauge",
    }
}

/// All sorts its rows by kind in this order (`KIND_ORDER`).
fn kind_rank(kind: ModKind) -> usize {
    [ModKind::Theme, ModKind::Tweak, ModKind::Layout, ModKind::Words, ModKind::Fleet, ModKind::Tracker, ModKind::Tab, ModKind::Instrument].iter().position(|one| *one == kind).unwrap_or(0)
}

/// Which list a page draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The theme mods, under the theme field, with no title of their own.
    Theme,
    /// The layout mods, under Layout's switches, with no title of their own.
    Layout,
    /// The theme in force, then every Fleet and tracker mod, the ones that are on first, with Create mod.
    All,
    /// The mods of one taught kind: Tracker or Fleet.
    Of(ModKind),
}

/// The list a page of Settings draws, where it draws one.
pub fn page_of(body: super::entries::Body) -> Option<Page> {
    match body {
        super::entries::Body::Theme => Some(Page::Theme),
        super::entries::Body::Mods(None) => Some(Page::All),
        super::entries::Body::Mods(Some(kind)) => Some(Page::Of(kind)),
        _ => None,
    }
}

/// One thing a mod says, under the part it belongs to (`TaughtFact`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub label: &'static str,
    pub value: String,
    /// A pattern, drawn in the mono face.
    pub code: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub part: &'static str,
    pub facts: Vec<Fact>,
}

/// What the file Fleet checked for a mod says, and the change of the mod that read was of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub at: String,
    pub parts: Vec<Part>,
    /// A Fleet mod's rules in one line, which its row says in place of the file's own description.
    pub line: Option<String>,
    /// A layout mod's `layout.json`, which Settings › Layout lays under his own choice.
    pub layout: Option<String>,
}

/// A tracker mod's try mark (`ModTried`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tried {
    Passed,
    Failed,
    Stale,
    Never,
}

/// A tracker mod's two marks: how its try went, in words, and whether it is valid (`trackerOf`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked {
    pub tried: Tried,
    pub words: String,
    pub valid: bool,
}

/// The hue a row in a list that holds a tracker is framed in (`standing`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Used,
    Waiting,
    Refused,
    Off,
}

/// One row as it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub name: String,
    pub title: String,
    pub kind: ModKind,
    pub enabled: bool,
    pub built_in: bool,
    /// The one plain fact under the title: what is wrong, the branch it was put on, or what a taught mod does.
    pub detail: Option<String>,
    pub promote_off: bool,
    /// The pull request Promote opened, by number, title and address, and the Job holding it, by id and title.
    pub pull: Option<(u64, String, String)>,
    pub job: Option<(String, String)>,
    pub tracker: Option<Tracked>,
    /// What the row opens to under it. Empty draws no chevron.
    pub parts: Vec<Part>,
    pub standing: Standing,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// `10 Nov`, in UTC, as `dayOf` says a day; what is not a time is said as it came.
fn day_of(at: &str) -> String {
    use chrono::Datelike;
    chrono::DateTime::parse_from_rfc3339(at).map_or_else(|_| at.to_string(), |day| {
        let day = day.with_timezone(&chrono::Utc);
        format!("{} {}", day.day(), MONTHS[day.month0() as usize])
    })
}

/// A tracker mod's marks off its row: a row with no try is stale where its file was edited after one.
fn tracked(row: &ModSummary, tried: Option<&TrackerTry>) -> Tracked {
    let (tried, words) = match tried {
        None if row.stale => (Tried::Stale, "Stale".to_string()),
        None => (Tried::Never, "Not tried".to_string()),
        Some(one) if one.passed => (Tried::Passed, format!("Tried {} on {}", day_of(one.at.as_str()), one.manifest_id.as_str())),
        Some(one) => (Tried::Failed, format!("Failed {}: {}", day_of(one.at.as_str()), one.reason.as_deref().unwrap_or("no reason given"))),
    };
    Tracked { tried, words, valid: row.valid }
}

/// The switch as pressed, until Fleet's list says something other than it said when it was pressed.
pub(super) fn switched_on(row: &ModSummary, held: &ModRows) -> bool {
    match held.switched.get(&row.name) {
        Some((was, now)) if *was == row.enabled => *now,
        _ => row.enabled,
    }
}

/// A theme mod's row: what is wrong with it, else the branch Promote put it on until its pull request is drawn.
fn own(row: &ModSummary, kind: ModKind, held: &ModRows) -> Listed {
    let built_in = row.origin == ModOrigin::BuiltIn;
    let title = row.name.strip_prefix(BUILT_IN).unwrap_or(&row.name).to_string();
    let promote = held.promoted.get(&row.name);
    let branch = promote.map(|done| done.branch.clone());
    let (pull, job) = match (&row.pull_request, promote) {
        (Some(open), _) => (Some((open.number, open.title.clone(), open.address.clone())), Some((open.session_id.as_str().to_string(), open.title.clone()))),
        (None, Some(done)) => (done.pull_request.as_ref().and_then(|address| number_of(address).map(|number| (number, done.title.clone(), address.clone()))), Some((done.session_id.as_str().to_string(), done.title.clone()))),
        (None, None) => (None, None),
    };
    let problem = (!row.valid).then(|| row.reason.clone().unwrap_or_else(|| "Did not pass its checks".into()));
    let enabled = switched_on(row, held);
    let standing = if problem.is_some() { Standing::Refused } else if enabled { Standing::Used } else { Standing::Off };
    Listed {
        name: row.name.clone(),
        title,
        kind,
        enabled,
        built_in,
        detail: problem.clone().or_else(|| branch.clone().filter(|_| pull.is_none())),
        promote_off: problem.is_some() || branch.is_some() || pull.is_some(),
        pull,
        job,
        tracker: None,
        parts: Vec::new(),
        standing,
    }
}

/// A taught mod's row (`taughtRow` over `fleet-taught.ts`): what it does, else why it did not pass, then what it
/// was taught on, then the branch Promote put it on.
fn taught(row: &ModSummary, kind: ModKind, held: &ModRows) -> Listed {
    let checked = held.checked.get(&row.name).filter(|_| row.valid);
    let description = if row.valid {
        checked.and_then(|one| one.line.clone()).or_else(|| row.description.clone()).unwrap_or_default()
    } else {
        row.reason.clone().unwrap_or_else(|| "Did not pass its checks".into())
    };
    let on = row.taught_on.clone().filter(|on| !on.is_empty()).map(|on| format!("Taught on {on}"));
    let line = [Some(description).filter(|said| !said.is_empty()), on].into_iter().flatten().collect::<Vec<_>>().join(". ");
    let branch = held.promoted.get(&row.name).map(|done| done.branch.clone());
    let detail = match &branch {
        Some(branch) => Some(format!("{line} ({branch})")),
        None => (!line.is_empty()).then_some(line),
    };
    let enabled = switched_on(row, held);
    let tracker = (kind == ModKind::Tracker).then(|| {
        // A try just answered stands until Fleet's list carries one as new.
        let ours = held.tried.get(&row.name).filter(|ours| row.tried.as_ref().is_none_or(|listed| listed.at.as_str() <= ours.at.as_str()));
        tracked(row, ours.or(row.tried.as_ref()))
    });
    let standing = match &tracker {
        Some(marks) if !marks.valid || marks.tried == Tried::Failed => Standing::Refused,
        _ if !enabled => Standing::Off,
        Some(marks) if marks.tried != Tried::Passed => Standing::Waiting,
        _ => Standing::Used,
    };
    Listed {
        name: row.name.clone(),
        title: row.name.clone(),
        kind,
        enabled,
        built_in: row.origin == ModOrigin::BuiltIn,
        detail,
        promote_off: branch.is_some(),
        pull: None,
        job: None,
        tracker,
        parts: checked.map(|one| one.parts.clone()).unwrap_or_default(),
        standing,
    }
}

/// The rows a page lists, in Classic's order.
pub fn listed(page: Page, mods: &[ModSummary], theme: &str, held: &ModRows) -> Vec<Listed> {
    let kind_of = |row: &ModSummary| row.kind.unwrap_or(ModKind::Theme);
    match page {
        Page::Theme => {
            let mut rows: Vec<Listed> = mods.iter().filter(|row| kind_of(row) == ModKind::Theme).map(|row| own(row, ModKind::Theme, held)).collect();
            // A mod Armada ships before a local one of the same name; the sort by title keeps that, being stable.
            rows.sort_by_key(|one| !one.built_in);
            rows.sort_by(|a, b| a.title.cmp(&b.title));
            rows
        }
        Page::Layout => {
            let mut rows: Vec<Listed> = mods.iter().filter(|row| kind_of(row) == ModKind::Layout).map(|row| own(row, ModKind::Layout, held)).collect();
            rows.sort_by(|a, b| a.title.cmp(&b.title));
            rows
        }
        Page::Of(kind) => {
            let mut rows: Vec<Listed> = mods.iter().filter(|row| kind_of(row) == kind).map(|row| taught(row, kind, held)).collect();
            rows.sort_by(|a, b| a.title.cmp(&b.title));
            rows
        }
        Page::All => {
            // A theme is applied once it is chosen, so the one in force is the one listed.
            let mut rows: Vec<Listed> = mods
                .iter()
                .filter_map(|row| match kind_of(row) {
                    ModKind::Theme if row.name == theme => Some(own(row, ModKind::Theme, held)),
                    kind @ (ModKind::Fleet | ModKind::Tracker) => Some(taught(row, kind, held)),
                    _ => None,
                })
                .collect();
            rows.sort_by(|a, b| b.enabled.cmp(&a.enabled).then(kind_rank(a.kind).cmp(&kind_rank(b.kind))).then(a.title.cmp(&b.title)));
            rows
        }
    }
}

/// A list that holds a tracker keeps the marks' and Try's columns in every row, and frames every row in its standing.
pub fn lined(page: Page, mods: &[ModSummary]) -> bool {
    matches!(page, Page::All | Page::Of(ModKind::Tracker)) && mods.iter().any(|row| row.kind == Some(ModKind::Tracker))
}

/// The number at the end of a pull request's address.
fn number_of(address: &str) -> Option<u64> {
    address.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

// ---- what a Fleet mod's file says (`taught-parts.ts`) -----------------------------------------

fn does_said(does: Does) -> &'static str {
    match does {
        Does::Tell => "Tell",
        Does::Quiet => "Quiet",
        Does::Batch => "Batch",
        Does::Call => "Call",
    }
}

/// What Fleet does in place of a rule's own, as a clause (`INSTEAD`).
fn instead_said(does: Does) -> &'static str {
    match does {
        Does::Tell => "tell",
        Does::Quiet => "stay quiet",
        Does::Batch => "batch",
        Does::Call => "call you",
    }
}

/// What a rule does, as `Also changes` words it (`teach.ts`'s `DOES`).
fn also_said(does: Does) -> &'static str {
    match does {
        Does::Batch => "batched",
        Does::Quiet => "quiet",
        Does::Tell => "told",
        Does::Call => "called to you",
    }
}

fn condition_said(condition: Condition) -> &'static str {
    match condition {
        Condition::BlocksMerge => "it blocks a merge",
        Condition::OpenPullRequest => "it has an open pull request",
        Condition::Working => "its agent is working",
        Condition::OwnerAway => "you are away",
    }
}

/// "Batch, and tell only when it blocks a merge" (`doesWords`).
fn does_words(rule: &FleetRule) -> String {
    std::iter::once(does_said(rule.does).to_string()).chain(rule.when.iter().map(|one| format!("{} only when {}", instead_said(one.does), condition_said(one.condition)))).collect::<Vec<_>>().join(", and ")
}

/// Where a rule reaches (`scopeOf`): a repository by its Manifest id.
fn scope_said(scope: &RuleScope) -> String {
    match scope {
        RuleScope::Everywhere => "Every repository".into(),
        RuleScope::Job(_) => "This Job".into(),
        RuleScope::Repo(id) => id.clone(),
    }
}

fn fleet_part(rule: &FleetRule) -> Part {
    let mut facts = vec![Fact { label: "Notice", value: notice_name(&rule.notice), code: false }];
    if let Some(matches) = &rule.matches {
        facts.push(Fact { label: "Matches", value: matches.clone(), code: true });
    }
    facts.push(Fact { label: "Does", value: does_words(rule), code: false });
    facts.push(Fact { label: "Where", value: scope_said(&rule.scope), code: false });
    Part { part: "Fleet", facts }
}

/// What a checked mod holds, as its row opens to. A mod that did not pass holds nothing here.
pub fn parts_of(checked: &ModChecked) -> Vec<Part> {
    if !checked.valid {
        return Vec::new();
    }
    checked.fleet.iter().flat_map(|file| file.rule.iter().map(fleet_part)).collect()
}

/// A Fleet file's rules in one line, capitalised: each notice, what is done with it and where, then each
/// condition that changes that (`fleetAlso`).
pub fn fleet_also(file: &FleetFile) -> Option<String> {
    let said: Vec<String> = file
        .rule
        .iter()
        .map(|one| {
            let reach = match one.scope {
                RuleScope::Everywhere => "for every repository",
                RuleScope::Job(_) => "for this Job",
                RuleScope::Repo(_) => "for this repository",
            };
            let notice = notice_name(&one.notice);
            let mut chars = notice.chars();
            let notice: String = chars.next().map_or_else(String::new, |first| first.to_lowercase().chain(chars).collect());
            std::iter::once(format!("{notice}, {} {reach}", also_said(one.does))).chain(one.when.iter().map(|then| format!("{} only when {}", also_said(then.does), condition_said(then.condition)))).collect::<Vec<_>>().join(", ")
        })
        .collect();
    let line = said.join("; ");
    let mut chars = line.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect())
}

// ---- the rows' own state ------------------------------------------------------------------------

/// A mod's panel opened from its row, beside the page.
pub struct Opened {
    pub name: String,
    pub view: Entity<ModPanel>,
    _asked: Subscription,
}

/// The mod rows' own state: the switches pressed while Fleet answers, what a Promote or a Try answered, what each
/// mod's checked file says, the row shown open, the panel opened, and each row's keys.
#[derive(Default)]
pub struct ModRows {
    pub switched: BTreeMap<String, (bool, bool)>,
    pub promoted: BTreeMap<String, ModPromoted>,
    pub promoting: Option<String>,
    pub refused: BTreeMap<String, String>,
    pub tried: BTreeMap<String, TrackerTry>,
    pub checked: BTreeMap<String, Checked>,
    asking: BTreeSet<String>,
    /// The row shown open to what it holds.
    pub opened: Option<String>,
    pub panel: Option<Opened>,
    /// Each row's keys, by mod name, at the indexes below.
    pub keys: BTreeMap<String, [FocusHandle; 7]>,
    /// Create mod, in All's header.
    pub create: Option<FocusHandle>,
    /// How wide a row's words column was drawn last frame. Every row of a list draws it alike, and the same
    /// whether the line under the name is beside it or under the row, so it decides which without moving itself.
    pub words_wide: std::rc::Rc<std::cell::Cell<Option<Pixels>>>,
}

/// Narrower than this, the words column holds the name alone and the line under it takes the row's width, so no
/// sentence is squeezed into a word a line.
const WORDS_LEAST: f32 = 240.0;

const OPEN: usize = 0;
const SWITCH: usize = 1;
const TRY: usize = 2;
const DETAILS: usize = 3;
const PROMOTE: usize = 4;
const PULL: usize = 5;
const JOB: usize = 6;

/// A theme or layout mod is the list's own; the other kinds are taught, and say a refusal as a toast.
fn is_taught(kind: ModKind) -> bool {
    !matches!(kind, ModKind::Theme | ModKind::Layout)
}

impl SettingsOverlay {
    pub fn mod_rows(&self, page: Page, cx: &App) -> Vec<Listed> {
        listed(page, &self.wire.mods(cx), &self.theme_word(cx), &self.mods)
    }

    /// Whether a press on the row opens the mod's panel: only where Fleet lists it, so it never opens on nothing.
    fn opens(&self, name: &str, cx: &App) -> bool {
        ModPanel::lists(name, self.wire.fleet().as_ref(), cx)
    }

    /// A row's own controls, in the order Tab and the arrows reach them; what Promote opened is not among them.
    fn row_controls(&self, one: &Listed, cx: &App) -> Vec<FocusHandle> {
        let Some(keys) = self.mods.keys.get(&one.name) else { return Vec::new() };
        let mut controls = Vec::new();
        if self.opens(&one.name, cx) {
            controls.push(keys[OPEN].clone());
        }
        controls.push(keys[SWITCH].clone());
        if one.tracker.as_ref().is_some_and(|marks| marks.valid) && self.live {
            controls.push(keys[TRY].clone());
        }
        if !one.parts.is_empty() {
            controls.push(keys[DETAILS].clone());
        }
        if !one.built_in && !one.promote_off && self.mods.promoting.is_none() {
            controls.push(keys[PROMOTE].clone());
        }
        controls
    }

    /// The stops of a page's rows, in the order Tab reaches them: Create mod first on All.
    pub fn mod_stops(&self, page: Page, cx: &App) -> Vec<FocusHandle> {
        let mut stops: Vec<FocusHandle> = self.mods.create.iter().filter(|_| page == Page::All).cloned().collect();
        for one in self.mod_rows(page, cx) {
            stops.extend(self.row_controls(&one, cx));
            let Some(keys) = self.mods.keys.get(&one.name) else { continue };
            if one.pull.is_some() {
                stops.push(keys[PULL].clone());
            }
            if one.job.is_some() {
                stops.push(keys[JOB].clone());
            }
        }
        stops
    }

    /// Make the keys of every row Fleet lists, once each, and Create mod's.
    pub fn mod_keys(&mut self, cx: &mut App) {
        if self.mods.create.is_none() {
            self.mods.create = Some(cx.focus_handle().tab_stop(true));
        }
        for row in self.wire.mods(cx) {
            self.mods.keys.entry(row.name).or_insert_with(|| std::array::from_fn(|_| cx.focus_handle().tab_stop(true)));
        }
    }

    /// Ask Fleet for the file it checked of each valid Fleet mod, once for each change of the mod.
    pub fn read_mods(&mut self, cx: &mut Context<Self>) {
        for row in self.wire.mods(cx) {
            if !matches!(row.kind, Some(ModKind::Fleet | ModKind::Layout)) || !row.valid {
                continue;
            }
            let at = row.changed_at.as_ref().map(|at| at.as_str().to_string()).unwrap_or_default();
            let asked = format!("{}@{at}", row.name);
            if self.mods.checked.get(&row.name).is_some_and(|one| one.at == at) || self.mods.asking.contains(&asked) {
                continue;
            }
            self.mods.asking.insert(asked.clone());
            let sent = self.wire.validate_mod(&row.name, cx);
            let name = row.name.clone();
            cx.spawn(async move |this, cx| {
                let answer = sent.await;
                let _ = this.update(cx, |this, cx| {
                    this.mods.asking.remove(&asked);
                    if let Ok(read) = answer {
                        let line = read.fleet.as_ref().and_then(fleet_also);
                        this.mods.checked.insert(name, Checked { at, parts: parts_of(&read), line, layout: read.layout.clone() });
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    /// The arrows on a row, as in a tree: → opens what it holds, ← closes it, ↑ and ↓ go to the same control in the
    /// row above or below. `true` where one was taken; ← on a row that is shut goes on to the page.
    pub fn mods_key(&mut self, page: Page, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || mods.shift || !matches!(key.key.as_str(), "left" | "right" | "up" | "down") {
            return false;
        }
        let rows = self.mod_rows(page, cx);
        let controls: Vec<Vec<FocusHandle>> = rows.iter().map(|one| self.row_controls(one, cx)).collect();
        let Some((row, at)) = controls.iter().enumerate().find_map(|(row, list)| list.iter().position(|one| one.is_focused(window)).map(|at| (row, at))) else { return false };
        let one = &rows[row];
        match key.key.as_str() {
            "right" if !one.parts.is_empty() && self.mods.opened.as_deref() != Some(&one.name) => self.mods.opened = Some(one.name.clone()),
            "left" if self.mods.opened.as_deref() == Some(&one.name) => self.mods.opened = None,
            "up" | "down" => {
                let next = if key.key == "up" { row.checked_sub(1) } else { Some(row + 1) };
                let Some(list) = next.and_then(|next| controls.get(next)) else { return false };
                let Some(to) = list.get(at).or(list.first()) else { return false };
                window.focus(to, cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Fleet's `set_mod_enabled`: the switch moves at once, and a refusal puts it back and says why.
    pub(super) fn switch_mod(&mut self, name: String, kind: ModKind, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(was) = self.wire.mods(cx).iter().find(|one| one.name == name).map(|one| one.enabled) else { return };
        self.mods.switched.insert(name.clone(), (was, on));
        self.mods.refused.remove(&name);
        let sent = self.wire.set_mod_enabled(&name, on, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                if let Err(why) = answer {
                    this.mods.switched.remove(&name);
                    if is_taught(kind) {
                        cx.defer(move |cx| crate::toasts::tell(why.said, cx));
                    } else {
                        // Taken from outside the overlay, the refusal is said where he is.
                        if !this.open {
                            let said = why.said.clone();
                            cx.defer(move |cx| crate::toasts::tell(said, cx));
                        }
                        this.mods.refused.insert(name, why.said);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Promote: Fleet starts a Job of its own that puts the mod into Armada in a pull request.
    fn promote_mod(&mut self, name: String, kind: ModKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.mods.promoting.is_some() {
            return;
        }
        self.mods.promoting = Some(name.clone());
        self.mods.refused.remove(&name);
        let sent = self.wire.promote_mod(&name, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.mods.promoting = None;
                match answer {
                    Ok(done) => {
                        this.mods.promoted.insert(name.clone(), done);
                        // Promote goes once it has been pressed: the keys go to the row's switch.
                        if let Some(keys) = this.mods.keys.get(&name) {
                            window.focus(&keys[SWITCH], cx);
                        }
                    }
                    Err(why) if is_taught(kind) => cx.defer(move |cx| crate::toasts::tell(why.said, cx)),
                    Err(why) => {
                        this.mods.refused.insert(name, why.said);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Try a tracker mod against the repository; how it went stands on the row until Fleet lists a newer try.
    fn try_mod(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let sent = self.wire.try_tracker_mod(&name, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(tried) => {
                        this.mods.tried.insert(name, tried);
                    }
                    Err(why) => cx.defer(move |cx| crate::toasts::tell(why.said, cx)),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Create mod: an empty Mod Job, started at once and opened, and Settings gets out of the way (`startEmptyModJob`).
    fn create_mod(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sent = self.wire.start_mod_session(cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(started) => {
                    let id = started.session_id.as_str().to_string();
                    if let Some(open) = cx.try_global::<OpenJobFrom>().cloned() {
                        cx.defer(move |cx| (open.0)(id, None, cx));
                    }
                    this.close(window, cx);
                }
                Err(why) => cx.defer(move |cx| crate::toasts::tell(format!("Mod Job not started: {}", why.said), cx)),
            });
        })
        .detach();
    }

    /// The mod's panel, beside the page, with the keys in it. Its Esc or close puts them back on the row; a Job
    /// opened from it closes Settings.
    fn open_panel(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let fleet = self.wire.fleet();
        let view = cx.new(|cx| ModPanel::new(name.clone(), fleet, cx));
        let asked = cx.subscribe_in(&view, window, |this, _, asked: &ModPanelAsked, window, cx| match asked {
            ModPanelAsked::Close => this.close_panel(window, cx),
            ModPanelAsked::OpenedJob => {
                this.mods.panel = None;
                this.close(window, cx);
            }
        });
        window.focus(&view.focus_handle(cx), cx);
        self.mods.panel = Some(Opened { name, view, _asked: asked });
        cx.notify();
    }

    pub fn close_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(opened) = self.mods.panel.take() else { return };
        if let Some(keys) = self.mods.keys.get(&opened.name) {
            window.focus(&keys[OPEN], cx);
        }
        cx.notify();
    }

    /// The Job holding a promote's pull request opens as a press on its row anywhere does.
    fn open_promote_job(id: &str, title: &str, window: &mut Window, cx: &mut App) {
        let Some(open) = cx.try_global::<crate::clash_mark::OpenHolder>().cloned() else { return };
        (open.0)(&ipc::ClashHolder { kind: ipc::ClashSide::Session, id: id.to_string(), label: title.to_string() }, window, cx);
    }

    /// A page's rows. Theme draws them in a box under its field, and nothing while there are none; All is a card
    /// titled All with Create mod, drawn with no rows; Tracker and Fleet are cards of their name.
    /// How wide a row is after its switch: Details, the shared home's two cells, and Promote. Settings › Layout keeps
    /// its puzzle column this wide, so its switches line up with the layout mods' under them.
    pub fn trailing_wide(look: &Look) -> Pixels {
        look.length(token::H_CONTROL) * 4.0 + look.length(token::SPACE_3) * 3.0
    }

    pub fn render_mods(&self, page: Page, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let mods = self.wire.mods(cx);
        let rows = listed(page, &mods, &self.theme_word(cx), &self.mods);
        let lined = lined(page, &mods);
        if rows.is_empty() && page != Page::All {
            return None;
        }
        let body = div().flex().flex_col().gap(look.length(token::SPACE_3)).children(rows.into_iter().map(|one| self.render_mod(one, lined, look, window, cx)));
        let titled = |title: &'static str, right: Option<AnyElement>, body: gpui::Div| labelled(card(look, SharedString::from(format!("card-mods-{title}")), title, right, vec![body.into_any_element()]), Role::Region, title).into_any_element();
        Some(match page {
            Page::Theme => labelled(
                div()
                    .id("card-mods-theme")
                    .flex()
                    .flex_col()
                    .p(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::SURFACE_CARD))
                    .child(body),
                Role::Group,
                "Theme mods",
            )
            .into_any_element(),
            Page::Layout => labelled(
                div()
                    .id("card-mods-layout")
                    .flex()
                    .flex_col()
                    // The Layout card's own sides, so the switches above and here share a column.
                    .px(look.length(token::PAD_CARD))
                    .py(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::SURFACE_CARD))
                    .child(body),
                Role::Group,
                "Layout mods",
            )
            .into_any_element(),
            Page::All => titled("All", Some(self.render_create(look, window, cx)), body),
            Page::Of(kind) => titled(kind_said(kind), None, body),
        })
    }

    fn render_create(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(focus) = self.mods.create.clone() else { return div().into_any_element() };
        let off = !self.live;
        let button = ghost(look, "mods-create", "Create mod", off)
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .track_focus(&focus)
            .when(!off, |button| {
                button.on_click(cx.listener(|this, _, window, cx| this.create_mod(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.create_mod(window, cx);
                        cx.stop_propagation();
                    }
                }))
            });
        labelled(ringed(look, button, &focus, window), Role::Button, "Create mod").into_any_element()
    }

    /// A press: a control's own pointer press, and Enter or Space on its keys.
    fn pressable(element: gpui::Stateful<gpui::Div>, cx: &Context<Self>, press: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static) -> gpui::Stateful<gpui::Div> {
        let by_key = press.clone();
        element.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                by_key(this, window, cx);
                cx.stop_propagation();
            }
        }))
    }

    fn render_mod(&self, one: Listed, lined: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let Some(keys) = self.mods.keys.get(&one.name).cloned() else { return div().into_any_element() };
        let off = !self.live;
        let control = look.length(token::H_CONTROL);
        let glyph16 = |name: &'static str, ink: Hsla| svg().path(icon_path(name)).flex_none().size(look.length(token::SPACE_4)).text_color(ink);
        let glyph = labelled(
            div().id(SharedString::from(format!("mod-kind-{}", one.name))).flex_none().flex().tooltip(tip(kind_said(one.kind))).child(glyph16(kind_glyph(one.kind), look.color(token::FG_MUTED))),
            Role::Image,
            kind_said(one.kind),
        );
        let under = self.mods.words_wide.get().is_some_and(|wide| wide < px(WORDS_LEAST));
        let line = |said: String| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(said);
        let wide = self.mods.words_wide.clone();
        let words = div()
            .relative()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            // Cut short where the column is narrow, so the whole name is on hover.
            .child(div().id(SharedString::from(format!("mod-name-{}", one.name))).min_w_0().truncate().tooltip(tip(one.title.clone())).child(SharedString::from(one.title.clone())))
            .children(one.detail.clone().filter(|_| !under).map(line))
            .child(
                canvas(
                    move |bounds, window, _| {
                        if wide.get() != Some(bounds.size.width) {
                            wide.set(Some(bounds.size.width));
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        // The name is how the keys open the mod's panel, where Fleet lists the mod; the switch alone toggles it.
        let text: AnyElement = if self.opens(&one.name, cx) {
            let name = one.name.clone();
            labelled(
                ringed(
                    look,
                    Self::pressable(div().id(SharedString::from(format!("mod-open-{}", one.name))).flex_1().min_w_0().rounded(look.length(token::RADIUS_SM)).track_focus(&keys[OPEN]).child(words), cx, move |this, window, cx| {
                        this.open_panel(name.clone(), window, cx)
                    }),
                    &keys[OPEN],
                    window,
                ),
                Role::Button,
                format!("Open {}", one.title),
            )
            .into_any_element()
        } else {
            words.flex_1().into_any_element()
        };
        let (name, kind, on) = (one.name.clone(), one.kind, one.enabled);
        let switch = div().id(SharedString::from(format!("mod-switch-{}", one.name))).flex_none().rounded_full().track_focus(&keys[SWITCH]).child(switch_track(look, on, off));
        let switch = if off { switch } else { Self::pressable(switch, cx, move |this, window, cx| this.switch_mod(name.clone(), kind, !on, window, cx)) };
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_4))
            .min_w_0()
            .flex_1()
            .child(text)
            .child(labelled(ringed(look, switch, &keys[SWITCH], window), Role::Switch, one.title.clone()));
        let cell = || div().flex_none().w(control).h(control).flex().items_center().justify_center();
        let act = |id: String, focus: &FocusHandle, icon: &'static str, quiet: bool| {
            cell()
                .id(SharedString::from(id))
                .rounded(look.length(token::RADIUS_MD))
                .track_focus(focus)
                .when(!quiet, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER))))
                .child(glyph16(icon, if quiet { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) }))
        };
        // The marks and Try, in a list that holds a tracker: kept empty on any other row, so the columns line up.
        let mark_wide = look.length(token::SPACE_4) + look.length(token::SPACE_2) + look.length(token::BORDER_WIDTH) * 2.0;
        let marks = lined.then(|| {
            let held = div().flex_none().w(mark_wide * 2.0 + look.length(token::SPACE_1)).flex().gap(look.length(token::SPACE_1));
            let Some(marks) = one.tracker.clone() else { return held.into_any_element() };
            let mark = |id: &str, tone: Hsla, icon: &'static str, said: String| {
                labelled(
                    div()
                        .id(SharedString::from(format!("mod-mark-{id}-{}", one.name)))
                        .flex_none()
                        .p(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(tone.opacity(0.4))
                        .bg(tone.opacity(0.16))
                        .tooltip(tip(said.clone()))
                        .child(glyph16(icon, tone)),
                    Role::Image,
                    said,
                )
            };
            let (tone, icon) = match marks.tried {
                Tried::Passed => (look.color(token::STATUS_COMPLETED_SUCCESS), "flask-conical"),
                Tried::Failed => (look.color(token::STATUS_COMPLETED_FAILED), "flask-conical"),
                Tried::Stale => (look.color(token::STATUS_AWAITING_REVIEW), "clock-alert"),
                Tried::Never => (look.color(token::STATUS_NOT_STARTED), "circle-dashed"),
            };
            let valid = if marks.valid { (look.color(token::STATUS_COMPLETED_SUCCESS), "shield-check", "Valid") } else { (look.color(token::STATUS_COMPLETED_FAILED), "shield-x", "Invalid") };
            held.child(mark("tried", tone, icon, marks.words.clone())).child(mark("valid", valid.0, valid.1, valid.2.to_string())).into_any_element()
        });
        let try_cell = lined.then(|| {
            let Some(marks) = one.tracker.as_ref() else { return cell().into_any_element() };
            let quiet = off || !marks.valid;
            let button = act(format!("mod-try-{}", one.name), &keys[TRY], "flask-conical", quiet).tooltip(tip(format!("Try {} against the repository", one.title)));
            let name = one.name.clone();
            let button = if quiet { button } else { Self::pressable(button, cx, move |this, window, cx| this.try_mod(name.clone(), window, cx)) };
            labelled(ringed(look, button, &keys[TRY], window), Role::Button, format!("Try {}", one.title)).into_any_element()
        });
        let opened = self.mods.opened.as_deref() == Some(one.name.as_str());
        let details: AnyElement = if one.parts.is_empty() {
            cell().into_any_element()
        } else {
            let name = one.name.clone();
            let button = act(format!("mod-details-{}", one.name), &keys[DETAILS], if opened { "chevron-down" } else { "chevron-right" }, false).tooltip(tip(if opened { "Hide the details" } else { "Show the details" }));
            let button = Self::pressable(button, cx, move |this, _, cx| {
                this.mods.opened = if this.mods.opened.as_deref() == Some(name.as_str()) { None } else { Some(name.clone()) };
                cx.notify();
            });
            labelled(ringed(look, button, &keys[DETAILS], window), Role::Button, format!("Details of {}", one.title)).into_any_element()
        };
        let promote: AnyElement = if one.built_in {
            labelled(cell().id(SharedString::from(format!("mod-shipped-{}", one.name))).tooltip(tip(BUILT_IN_SAID)).child(glyph16("package", look.color(token::FG_MUTED))), Role::Image, BUILT_IN_SAID).into_any_element()
        } else {
            let quiet = off || one.promote_off || self.mods.promoting.is_some();
            let button = act(format!("mod-promote-{}", one.name), &keys[PROMOTE], "git-pull-request-arrow", quiet).tooltip(tip(format!("Add {} to Armada in a pull request", one.title)));
            let (name, kind) = (one.name.clone(), one.kind);
            let button = if quiet { button } else { Self::pressable(button, cx, move |this, window, cx| this.promote_mod(name.clone(), kind, window, cx)) };
            labelled(ringed(look, button, &keys[PROMOTE], window), Role::Button, format!("Promote {}", one.title)).into_any_element()
        };
        let row = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .child(glyph)
            .child(head)
            .children(marks)
            .children(try_cell)
            .child(details)
            // The shared home's mark and its act: kept empty so Promote keeps its column.
            .child(cell())
            .child(cell())
            .child(promote);
        let chip = |id: String, focus: &FocusHandle| {
            div()
                .id(SharedString::from(id))
                .track_focus(focus)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
        };
        let promoted = (one.pull.is_some() || one.job.is_some()).then(|| {
            let pull = one.pull.clone().map(|(number, title, address)| {
                let open = address.clone();
                let element = chip(format!("mod-pull-{}", one.name), &keys[PULL])
                    .child(svg().path(icon_path("git-pull-request")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
                    .child(div().font_family(super::view::mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("#{number}"))))
                    .child(div().min_w_0().truncate().child(SharedString::from(title.clone())))
                    .on_click(move |_, _, cx| cx.open_url(&open))
                    .on_key_down(move |event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.open_url(&address);
                        }
                    });
                labelled(ringed(look, element, &keys[PULL], window), Role::Button, format!("#{number} {title}"))
            });
            let job = one.job.clone().map(|(id, title)| {
                let (open_id, open_title) = (id.clone(), title.clone());
                let element = chip(format!("mod-job-{}", one.name), &keys[JOB])
                    .child(svg().path(icon_path("box")).flex_none().size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
                    .child(div().min_w_0().truncate().child(SharedString::from(title.clone())))
                    .on_click(move |_, window, cx| Self::open_promote_job(&open_id, &open_title, window, cx))
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            Self::open_promote_job(&id, &title, window, cx);
                        }
                    });
                labelled(ringed(look, element, &keys[JOB], window), Role::Button, format!("Job {}", one.title))
            });
            div().flex().flex_col().pl(look.length(token::SPACE_4) + look.length(token::SPACE_3)).children(pull).children(job)
        });
        let rules = (opened && !one.parts.is_empty()).then(|| render_rules(&one.title, &one.parts, look));
        let refused = self.mods.refused.get(&one.name).cloned().map(|said| labelled(super::view::small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id(SharedString::from(format!("mod-refused-{}", one.name))), Role::Alert, said));
        // Under the name's line where the words column is narrow: from the name's start to the row's end.
        let below = one.detail.clone().filter(|_| under).map(|said| line(said).pl(look.length(token::SPACE_4) + look.length(token::SPACE_3)));
        let framed = div().id(SharedString::from(format!("mod-{}", one.name))).relative().flex().flex_col().gap(look.length(token::SPACE_1)).child(row).children(below).children(promoted).children(rules).children(refused);
        let framed = if lined { frame(framed, one.standing, look) } else { framed };
        labelled(framed, Role::Group, one.title.clone()).into_any_element()
    }
}

/// A row framed in its standing, as a Cockpit Job card is in its state: a heavier edge in the hue at its start, a
/// hairline taking a little of it, and the ground a little more.
fn frame(row: gpui::Stateful<gpui::Div>, standing: Standing, look: &Look) -> gpui::Stateful<gpui::Div> {
    let hue = match standing {
        Standing::Used => look.color(token::STATUS_COMPLETED_SUCCESS),
        Standing::Waiting => look.color(token::STATUS_AWAITING_REVIEW),
        Standing::Refused => look.color(token::STATUS_COMPLETED_FAILED),
        Standing::Off => look.color(token::FG_SUBTLE),
    };
    let edge = look.length(token::BORDER_WIDTH);
    let radius = look.length(token::RADIUS_MD);
    row.py(look.length(token::SPACE_2))
        .pr(look.length(token::SPACE_3))
        .pl(look.length(token::SPACE_3) + edge * 3.0)
        .rounded(radius)
        .border(edge)
        .border_l(px(0.0))
        .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)))
        .bg(look.color(token::BG_RAISED).blend(hue.opacity(0.08)))
        .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.0).rounded_l(radius).bg(hue))
}

/// What an opened row holds (`ModRules`): a head for each part, then a line for each fact, its label, its value.
fn render_rules(title: &str, parts: &[Part], look: &Look) -> AnyElement {
    let label_wide = parts.iter().flat_map(|part| part.facts.iter()).map(|fact| fact.label.len()).max().unwrap_or(0) as f32;
    let label_w = look.length(token::TEXT_SM) * (label_wide * 0.62);
    labelled(
        div()
            .id(SharedString::from(format!("mod-rules-{title}")))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .pt(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_1))
            .pl(look.length(token::SPACE_8))
            .children(parts.iter().map(|part| {
                labelled(
                    div()
                        .id(SharedString::from(format!("mod-part-{title}-{}", part.part)))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .min_w_0()
                        .child(div().text_size(look.length(token::TEXT_XS)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(part.part))
                        .children(part.facts.iter().map(|fact| {
                            div()
                                .flex()
                                .items_baseline()
                                .gap(look.length(token::SPACE_3))
                                .min_w_0()
                                .text_size(look.length(token::TEXT_SM))
                                .line_height(look.length(token::LEADING_SM))
                                .child(div().flex_none().min_w(label_w).text_color(look.color(token::FG_MUTED)).child(fact.label))
                                .child(div().min_w_0().text_color(look.color(token::FG_DEFAULT)).when(fact.code, |value| value.font_family(super::view::mono(look))).child(SharedString::from(fact.value.clone())))
                        })),
                    Role::Group,
                    part.part,
                )
            })),
        Role::Group,
        format!("Details of {title}"),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(json: &str) -> Vec<ModSummary> {
        ipc::decode::<ipc::ModList>("mods", json.as_bytes()).expect("decodes").mods
    }

    fn held() -> ModRows {
        ModRows::default()
    }

    #[test]
    fn a_theme_row_says_what_is_wrong_and_shipped_ones_cannot_be_promoted() {
        let mods = rows(r#"{"mods":[
            {"name":"dusk","kind":"theme","enabled":true,"valid":false,"reason":"theme.css names no block for dusk"},
            {"name":"built-in:paper","origin":"built_in","kind":"theme","enabled":false,"valid":true},
            {"name":"calm","kind":"fleet","enabled":true,"valid":true,"description":"Stopped rows lighter","taught_on":"the Jobs list"}
        ]}"#);
        let themes = listed(Page::Theme, &mods, "dark", &held());
        assert_eq!(themes.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["dusk", "paper"]);
        assert_eq!(themes[0].detail.as_deref(), Some("theme.css names no block for dusk"));
        assert!(themes[0].promote_off);
        assert!(themes[1].built_in);
        let fleet = listed(Page::Of(ModKind::Fleet), &mods, "dark", &held());
        assert_eq!(fleet[0].detail.as_deref(), Some("Stopped rows lighter. Taught on the Jobs list"));
    }

    #[test]
    fn a_pressed_switch_stands_until_fleet_says_otherwise() {
        let mods = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#);
        let pressed = ModRows { switched: [("dusk".to_string(), (true, false))].into_iter().collect(), ..held() };
        assert!(!listed(Page::Theme, &mods, "dark", &pressed)[0].enabled);
        let moved = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":false,"valid":true}]}"#);
        let other = ModRows { switched: [("dusk".to_string(), (false, true))].into_iter().collect(), ..held() };
        assert!(listed(Page::Theme, &moved, "dark", &other)[0].enabled);
    }

    #[test]
    fn what_promote_opened_goes_under_the_row_and_promote_is_off() {
        let mods = rows(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#);
        let done: ModPromoted = ipc::decode("ModPromoted", br#"{"name":"dusk","session_id":"01JOBAAAAAAAAAAAAAAAAAAAAA","title":"Add the dusk theme mod","branch":"armada/mod-dusk","commit":"abc","pull_request":"https://github.com/o/r/pull/42"}"#).expect("decodes");
        let promoted = ModRows { promoted: [("dusk".to_string(), done)].into_iter().collect(), ..held() };
        let one = &listed(Page::Theme, &mods, "dark", &promoted)[0];
        assert_eq!(one.pull.as_ref().map(|pull| pull.0), Some(42));
        assert_eq!(one.job.as_ref().map(|job| job.1.as_str()), Some("Add the dusk theme mod"));
        assert_eq!(one.detail, None);
        assert!(one.promote_off);
    }

    #[test]
    fn all_lists_the_theme_in_force_and_the_taught_mods_the_ones_on_first_by_kind() {
        let mods = rows(r#"{"mods":[
            {"name":"dusk","kind":"theme","enabled":true,"valid":true},
            {"name":"nord-mod","kind":"theme","enabled":true,"valid":true},
            {"name":"zed","kind":"tracker","enabled":true,"valid":true},
            {"name":"quiet","kind":"fleet","enabled":false,"valid":true},
            {"name":"alpha","kind":"fleet","enabled":true,"valid":true},
            {"name":"calm","kind":"tweak","enabled":true,"valid":true}
        ]}"#);
        let all = listed(Page::All, &mods, "dusk", &held());
        assert_eq!(all.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["dusk", "alpha", "zed", "quiet"]);
        assert!(lined(Page::All, &mods));
        assert!(!lined(Page::Of(ModKind::Fleet), &mods));
    }

    #[test]
    fn a_tracker_says_how_its_try_went_and_is_framed_by_it() {
        let mods = rows(r#"{"mods":[
            {"name":"passed","kind":"tracker","enabled":true,"valid":true,"tried":{"at":"2026-11-10T08:45:00Z","manifest_id":"armada","passed":true,"listed":3}},
            {"name":"failed","kind":"tracker","enabled":true,"valid":true,"tried":{"at":"2026-11-09T08:45:00Z","manifest_id":"armada","passed":false,"listed":0,"reason":"401 from the tracker"}},
            {"name":"stale","kind":"tracker","enabled":true,"valid":true,"stale":true},
            {"name":"never","kind":"tracker","enabled":false,"valid":true}
        ]}"#);
        let said: BTreeMap<String, (String, Standing)> = listed(Page::Of(ModKind::Tracker), &mods, "dark", &held()).into_iter().map(|one| (one.name, (one.tracker.expect("marks").words, one.standing))).collect();
        assert_eq!(said["passed"], ("Tried 10 Nov on armada".to_string(), Standing::Used));
        assert_eq!(said["failed"], ("Failed 9 Nov: 401 from the tracker".to_string(), Standing::Refused));
        assert_eq!(said["stale"], ("Stale".to_string(), Standing::Waiting));
        assert_eq!(said["never"], ("Not tried".to_string(), Standing::Off));
    }

    #[test]
    fn a_fleet_rule_is_said_as_its_row_and_its_details_say_it() {
        let checked: ModChecked = ipc::decode(
            "ModChecked",
            br#"{"name":"quiet-ci","valid":true,"problems":[],"fleet":{"rule":[{"notice":"pr_checks_failed","does":"batch","where":"everywhere","when":[{"if":"blocks_merge","does":"tell"}]}]}}"#,
        )
        .expect("decodes");
        let line = fleet_also(checked.fleet.as_ref().expect("a fleet file")).expect("a line");
        assert!(line.ends_with(", batched for every repository, told only when it blocks a merge"), "{line}");
        let parts = parts_of(&checked);
        let facts: Vec<(&str, &str)> = parts[0].facts.iter().map(|fact| (fact.label, fact.value.as_str())).collect();
        assert_eq!(facts[1..], [("Does", "Batch, and tell only when it blocks a merge"), ("Where", "Every repository")]);
        assert_eq!(facts[0].0, "Notice");
    }
}
