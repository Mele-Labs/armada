//! The command palette (`list-needs-palette.md`, section 4): one box over the window that finds a Job, an act and
//! a page by a few letters, and runs the act on the row it is over. ⌘K opens it from anywhere; ⌘K on a row opens
//! that row's act panel.
//!
//! **A row is data and the box only draws it.** What the sections hold, in what order, and which acts a row has
//! is worked out by the functions at the top of this file, which the tests hold. Every act a Job takes goes to
//! Fleet as the Jobs list's menu sends it; the row says it is going in the frame the key lands, and a refusal is
//! said in the panel with Fleet's reason.
//!
//! The title bar's search field opens it with [`Palette::open`].

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{
    anchored, deferred, div, list, prelude::*, px, svg, Anchor, AnyElement, App, ClipboardItem, Context, Entity, FocusHandle, FontWeight, Keystroke, ListAlignment, ListState, MouseButton, Render, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{AttachmentState, ForkFrom, JobState, ManifestId, PauseSession, ResumeSession, SessionId, SessionRecord, SessionState, StartSession, StopSession};

use crate::assets::icon_path;
use crate::generated::actions::{Act as Registered, ActionScope, ACTIONS};
use crate::jobs::{deck_file, glyph_of, hue_of, JobsList};
use crate::keymap::{parse_chord, Act};
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look, Token};

// ---- what the palette holds ------------------------------------------------------------------

/// The blocks of results (`Palette.tsx`, `sections`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Section {
    Page,
    Context,
    Navigation,
    Commands,
    Run,
    Repositories,
    Jobs,
    Pulls,
    Issues,
    Tasks,
    Settings,
    Pinned,
    Recent,
}

/// Every block in the order the palette draws them, the empty ones included.
pub const SECTION_ORDER: [Section; 11] = [
    Section::Page,
    Section::Context,
    Section::Navigation,
    Section::Commands,
    Section::Run,
    Section::Repositories,
    Section::Jobs,
    Section::Pulls,
    Section::Issues,
    Section::Tasks,
    Section::Settings,
];

impl Section {
    /// The heading words; the focused block is headed by what is focused.
    fn title(self, on: Option<&str>) -> String {
        match self {
            Section::Page => "Page".into(),
            Section::Context => on.unwrap_or("Nothing focused").into(),
            Section::Navigation => "Navigation".into(),
            Section::Commands => "Commands".into(),
            Section::Run => "Run in the checkout".into(),
            Section::Repositories => "Repositories".into(),
            Section::Jobs => "Jobs".into(),
            Section::Pulls => "Pull requests".into(),
            Section::Issues => "Issues".into(),
            Section::Tasks => "Tasks".into(),
            Section::Settings => "Settings".into(),
            Section::Pinned => "Pinned".into(),
            Section::Recent => "Recent".into(),
        }
    }
}

/// What a Job row says beside its name: the repository and the state as a registry glyph with the words its tooltip
/// names it by.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub repo: Option<String>,
    pub state: Option<(&'static str, String, Token)>,
    /// A time fact beside the name, in his time format, and the words its tooltip names it by: when an ended
    /// Job ended.
    pub when: Option<(String, &'static str)>,
}

/// One row the palette can show.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `session:<id>` for a Job, `act:<id>` for a registered act.
    pub id: String,
    pub section: Section,
    pub label: String,
    pub icon: Option<&'static str>,
    pub shortcut: Option<String>,
    /// Words that find the row besides its label: a Job's id and its pull request.
    pub aliases: Vec<String>,
    pub meta: Option<Meta>,
    /// Found only while something is typed: the opening list leaves it out. An ended or quiet Job.
    pub typed_only: bool,
    /// Ranks after every row that is not ended, among rows of the same block.
    pub ended: bool,
}

impl Entry {
    /// The word flush right (`KIND`).
    pub fn kind(&self) -> &'static str {
        match self.id.split(':').next() {
            Some("session") => "Job",
            Some("act") => "Action",
            Some("pull") => "Pull request",
            Some("issue") => "Issue",
            Some("task") => "Task",
            Some("cmd") => "Command",
            Some("nav") => "Page",
            Some("set") => "Setting",
            Some("run") => "Check",
            Some("repo") => "Repository",
            _ => "",
        }
    }
}

/// The registered acts this build answers. The rest belong to screens the native app does not have yet, and the
/// palette leaves them out rather than draw a row that does nothing. `command_palette` is the act that opened it.
const ANSWERED: [&str; 9] = ["message_job", "needs_you", "needs_you_fold", "thread_yes", "thread_change", "thread_send_back", "thread_no", "deck_search", "new_terminal_job"];

fn act_entry(row: &Registered, section: Section) -> Entry {
    Entry {
        id: format!("act:{}", row.id),
        section,
        label: row.verb.to_string(),
        icon: row.glyph,
        shortcut: Some(row.shortcut.to_string()),
        aliases: Vec::new(),
        meta: None,
        typed_only: false,
        ended: false,
    }
}

/// The acts on what is focused (a Job open puts the thread's here) and the ones that work anywhere, which are
/// Navigation's.
pub fn registered(job_open: bool) -> Vec<Entry> {
    crate::keymap::in_palette()
        .filter(|row| ANSWERED.contains(&row.id))
        .filter_map(|row| match row.scope {
            ActionScope::Anywhere => Some(act_entry(row, Section::Navigation)),
            ActionScope::Thread if job_open => Some(act_entry(row, Section::Context)),
            ActionScope::Deck | ActionScope::List | ActionScope::ListAndDetail => Some(act_entry(row, Section::Context)),
            _ => None,
        })
        .collect()
}

/// The Jobs list's fold commands, offered where a Job has sub-Jobs. Electron's list does not fold, so the registry
/// has no row for them. Root Jobs only reads On while the switch is.
fn fold_commands(list: &JobsList) -> Vec<Entry> {
    let (any, root_only, _) = list.fold_facts();
    if !any {
        return Vec::new();
    }
    let command = |id: &str, label: &str, icon: &'static str, on: bool| Entry {
        id: format!("cmd:{id}"),
        section: Section::Commands,
        label: label.into(),
        icon: Some(icon),
        shortcut: None,
        aliases: Vec::new(),
        meta: on.then(|| Meta { repo: None, state: Some(("check", "On".into(), token::FG_MUTED)), when: None }),
        typed_only: false,
        ended: false,
    };
    vec![
        command("fold-all", "Fold all sub-Jobs", "list-tree", false),
        command("unfold-all", "Unfold all sub-Jobs", "list-tree", false),
        command("root-only", "Root Jobs only", "list-tree", root_only),
    ]
}

/// The open pull request a Job holds: its repository and number.
pub fn pull_of(job: &SessionRecord) -> Option<(ManifestId, u64)> {
    held_pull(job).filter(|(_, _, open)| *open).map(|(repo, number, _)| (repo, number))
}

/// The pull request a Job holds, open or not: its repository, number, and whether it is still open. The wire
/// marks an attachment standing while the pull request is open and spent once it is merged.
pub fn held_pull(job: &SessionRecord) -> Option<(ManifestId, u64, bool)> {
    let one = job.attachments.iter().find(|one| one.kind == "pr")?;
    let repo = one.manifest_id.clone().or_else(|| job.manifest_id.clone())?;
    Some((repo, one.target.trim_start_matches('#').parse().ok()?, one.state == AttachmentState::Standing))
}

/// What a Job's row cannot do. Electron leaves out `merge-pull` when the pull request held is not open; the
/// other acts all have a handler here, so they are never left out, bar Pause and Stop on an ended Job.
pub fn without_of(job: &SessionRecord) -> Vec<&'static str> {
    let mut without = match held_pull(job) {
        Some((_, _, false)) => vec!["merge-pull"],
        _ => Vec::new(),
    };
    // An ended Job has nothing to pause or stop, as the Jobs list's menu leaves it none.
    if job.state == SessionState::Ended {
        without.extend(["pause", "stop"]);
    }
    without
}

fn is_paused(job: &SessionRecord) -> bool {
    job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some())
}

fn is_snoozed(job: &SessionRecord) -> bool {
    job.snoozed_until.is_some()
}

/// A Job as a row. An ended or quiet Job is a row too, found only while something is typed; an ended one ranks
/// after the rest and says when it ended.
pub fn job_entry(job: &SessionRecord) -> Option<Entry> {
    let ended = job.state == SessionState::Ended || job.mark.is_some_and(|mark| mark.state == JobState::Ended);
    let state_of = if ended { JobState::Ended } else { job.mark?.state };
    let quiet = state_of == JobState::Quiet;
    let pull = pull_of(job);
    let state = if is_paused(job) && !ended {
        ("circle-pause", "Paused".to_string(), token::FG_MUTED)
    } else {
        let (icon, said) = glyph_of(state_of, pull.as_ref().map(|(_, number)| format!("#{number}")).as_deref());
        (icon, said.to_string(), hue_of(state_of))
    };
    let when = job.ended_at.as_ref().filter(|_| ended).and_then(|at| crate::settings::time::stamp_at(at.as_str())).map(|at| (at, "Ended"));
    Some(Entry {
        id: format!("session:{}", job.id.as_str()),
        section: Section::Jobs,
        label: crate::jobs::titled(job.title.as_deref()),
        icon: None,
        shortcut: None,
        aliases: std::iter::once(job.id.as_str().to_string()).chain(held_pull(job).iter().map(|(_, number, _)| format!("#{number}"))).collect(),
        meta: Some(Meta { repo: job.manifest_id.as_ref().map(|id| id.as_str().to_string()), state: Some(state), when }),
        typed_only: ended || quiet,
        ended,
    })
}

// ---- finding ---------------------------------------------------------------------------------

/// How well `query` finds an entry: lower is better, None is not found. A word the person gave a row, typed
/// whole, is best of all.
pub fn rank(query: &str, entry: &Entry, alias: Option<&str>) -> Option<u32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(100);
    }
    if alias.is_some_and(|word| word.to_lowercase() == query) {
        return Some(0);
    }
    let label = entry.label.to_lowercase();
    let mut worst = 0;
    for word in query.split_whitespace() {
        let here = if label.starts_with(word) {
            1
        } else if label.split(|c: char| !c.is_alphanumeric()).any(|part| part.starts_with(word)) {
            2
        } else if label.contains(word) {
            3
        } else if entry.aliases.iter().any(|one| one.to_lowercase().contains(word)) || alias.is_some_and(|one| one.to_lowercase().contains(word)) {
            4
        } else if entry.meta.as_ref().and_then(|meta| meta.repo.as_deref()).is_some_and(|repo| repo.to_lowercase().contains(word)) {
            5
        } else {
            return None;
        };
        worst = worst.max(here);
    }
    Some(worst)
}

/// What the empty query opens with (the setting *Palette opens with*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Opens {
    #[default]
    Pinned,
    Page,
    Everything,
}

impl Opens {
    fn word(self) -> &'static str {
        match self {
            Opens::Pinned => "pinned",
            Opens::Page => "page",
            Opens::Everything => "everything",
        }
    }

    fn of(word: &str) -> Opens {
        match word {
            "page" => Opens::Page,
            "everything" => Opens::Everything,
            _ => Opens::Pinned,
        }
    }
}

/// One line of the list: a heading, or the entry at this index.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Heading(String),
    Row(usize),
}

/// The lines for a query. With nothing typed, *Palette opens with* decides: Pinned then Recent, or the whole
/// list. A row found by a word he gave it, typed whole, comes first under no heading; then the blocks in order.
/// An ended or quiet Job is found only by what is typed, and an ended one comes after every other match.
pub fn lines_of(entries: &[Entry], query: &str, on: Option<&str>, opens: Opens, memory: &Memory) -> Vec<Line> {
    let mut lines = Vec::new();
    let typed = !query.trim().is_empty();
    if !typed && opens == Opens::Pinned {
        let find = |ids: &[String], skip: &[String]| -> Vec<usize> {
            ids.iter().filter(|id| !skip.contains(id)).filter_map(|id| entries.iter().position(|entry| &entry.id == id && !entry.typed_only)).collect()
        };
        let kept = find(&memory.pinned, &[]);
        let lately = find(&memory.recent, &memory.pinned);
        if !kept.is_empty() || !lately.is_empty() {
            for (section, rows) in [(Section::Pinned, kept), (Section::Recent, lately)] {
                if !rows.is_empty() {
                    lines.push(Line::Heading(section.title(on)));
                    lines.extend(rows.into_iter().map(Line::Row));
                }
            }
            return lines;
        }
    }
    // The fold commands are found by typing, so the list a person opens on keeps the blocks it has.
    let mut found: Vec<(u32, usize)> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| typed || !entry.id.starts_with("cmd:"))
        .filter_map(|(at, entry)| rank(query, entry, memory.aliases.get(&entry.id).map(String::as_str)).map(|rank| (rank, at)))
        .collect();
    if !typed {
        found.retain(|(_, at)| !entries[*at].typed_only);
    }
    let mut named: Vec<usize> = if typed { found.iter().filter(|(rank, _)| *rank == 0).map(|(_, at)| *at).collect() } else { Vec::new() };
    named.sort_by_key(|at| entries[*at].ended);
    lines.extend(named.iter().map(|at| Line::Row(*at)));
    found.retain(|(_, at)| !named.contains(at));
    for section in SECTION_ORDER {
        let mut rows: Vec<(u32, usize)> = found.iter().copied().filter(|(_, at)| entries[*at].section == section).collect();
        // The query ranks within a block; with nothing typed the block keeps its order.
        if typed {
            rows.sort_by_key(|(rank, at)| (entries[*at].ended, *rank, *at));
        }
        if !rows.is_empty() {
            lines.push(Line::Heading(section.title(on)));
            lines.extend(rows.into_iter().map(|(_, at)| Line::Row(at)));
        }
    }
    lines
}

// ---- acts ------------------------------------------------------------------------------------

/// What an act asks for before it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    Word,
    Chord,
}

/// One act of a row's ⌘K panel (`PaletteAction`).
#[derive(Clone, Debug, PartialEq)]
pub struct RowAct {
    pub id: &'static str,
    pub label: String,
    pub icon: Option<&'static str>,
    /// The chord that runs it while the palette is up, spelled as the registry spells one.
    pub chord: Option<&'static str>,
    /// A rule above it.
    pub apart: bool,
    /// The palette stays up after it.
    pub stays: bool,
    pub destructive: bool,
    pub takes: Option<Takes>,
    /// Why it cannot run now, said in its tooltip: the Job runs on another Mac. The act is listed, off.
    pub off: Option<String>,
}

fn act(id: &'static str, label: &str, icon: Option<&'static str>, chord: Option<&'static str>) -> RowAct {
    RowAct { id, label: label.into(), icon, chord, apart: false, stays: false, destructive: false, takes: None, off: None }
}

/// What a Job row's panel offers (`ownActs`). `without` is the acts nothing answers for this Job, by act id.
pub fn job_acts(paused: bool, snoozed: bool, pinned: bool, has_pull: bool, without: &[&str]) -> Vec<RowAct> {
    let apart = |one: RowAct| RowAct { apart: true, ..one };
    let all = [
        Some(act("open", "Open", None, Some("\u{21b5}"))),
        Some(if paused { act("pause", "Resume", Some("play"), Some("\u{2318}P")) } else { act("pause", "Pause", Some("pause"), Some("\u{2318}P")) }),
        // ⌥⌘Z: ⌘Z and ⇧⌘Z undo and redo a pane, and the bare Z a row answers is typing in here.
        Some(if snoozed { act("wake", "Wake now", Some("bell"), Some("\u{2325}\u{2318}Z")) } else { act("snooze", "Snooze", Some("bell-off"), Some("\u{2325}\u{2318}Z")) }),
        Some(act("fork", "Fork", Some("git-fork"), Some("\u{2325}\u{2318}F"))),
        has_pull.then(|| apart(act("open-pull", "Open pull request", Some("git-pull-request"), Some("\u{21e7}\u{2318}P")))),
        has_pull.then(|| act("merge-pull", "Merge pull request", Some("git-merge"), Some("\u{21e7}\u{2318}M"))),
        Some(apart(act("copy-id", "Copy Job id", Some("copy"), Some("\u{21e7}\u{2318}C")))),
        Some(RowAct { stays: true, ..apart(act("pin", if pinned { "Unpin" } else { "Pin" }, None, Some("\u{21e7}\u{2318}F"))) }),
        Some(RowAct { destructive: true, ..apart(act("stop", "Stop", Some("ban"), Some("\u{2318}Backspace"))) }),
    ];
    all.into_iter().flatten().filter(|one| !without.contains(&one.id)).collect()
}

/// The acts every row has after its own: a word and a chord to find it by (`actionsOf`). A destructive row has no
/// hotkey. `pinned` is Some for a row that has no Pin of its own.
pub fn memory_acts(has_alias: bool, has_hotkey: bool, destructive: bool, pinned: Option<bool>) -> Vec<RowAct> {
    let mut acts = Vec::new();
    if let Some(pinned) = pinned {
        acts.push(RowAct { apart: true, stays: true, ..act("pin", if pinned { "Unpin" } else { "Pin" }, None, Some("\u{21e7}\u{2318}F")) });
    }
    acts.push(RowAct { apart: pinned.is_none(), takes: Some(Takes::Word), ..act("set-alias", "Set alias", None, None) });
    if has_alias {
        acts.push(RowAct { stays: true, ..act("remove-alias", "Remove alias", None, None) });
    }
    if !destructive {
        acts.push(RowAct { takes: Some(Takes::Chord), ..act("record-hotkey", "Record hotkey", None, None) });
        if has_hotkey {
            acts.push(RowAct { stays: true, ..act("remove-hotkey", "Remove hotkey", None, None) });
        }
    }
    acts
}

/// Whether a keystroke is the chord, spelled as the registry spells one.
pub fn chord_is(chord: &str, keystroke: &Keystroke) -> bool {
    let Some(chord) = parse_chord(chord) else { return false };
    let Ok(want) = Keystroke::parse_with_os(&chord.keystroke(), gpui::OperatingSystem::Mac) else { return false };
    let (a, b) = (want.modifiers, keystroke.modifiers);
    want.key == keystroke.key && a.control == b.control && a.alt == b.alt && a.platform == b.platform && a.shift == b.shift
}

/// The chord a keystroke is, spelled for the registry and for the row it is kept on. A key with no ⌘ or ⌃
/// is typing and is no chord: ⌥ alone types a letter on this keyboard.
pub fn spelled(keystroke: &Keystroke) -> Option<String> {
    let mods = keystroke.modifiers;
    if !(mods.platform || mods.control) || matches!(keystroke.key.as_str(), "shift" | "control" | "alt" | "platform" | "function") {
        return None;
    }
    let key = match keystroke.key.as_str() {
        "enter" => "\u{21b5}".to_string(),
        "escape" => "Esc".to_string(),
        "backspace" => "Backspace".to_string(),
        "up" => "\u{2191}".to_string(),
        "down" => "\u{2193}".to_string(),
        "left" => "\u{2190}".to_string(),
        "right" => "\u{2192}".to_string(),
        other => other.to_uppercase(),
    };
    Some(format!(
        "{}{}{}{}{}",
        if mods.control { "\u{2303}" } else { "" },
        if mods.alt { "\u{2325}" } else { "" },
        if mods.shift { "\u{21e7}" } else { "" },
        if mods.platform { "\u{2318}" } else { "" },
        key
    ))
}

// ---- what is kept ----------------------------------------------------------------------------

/// What the person gave the palette: pins, what he used lately, a word and a chord for a row, and what the
/// empty query opens with. Kept in a file of its own beside the deck's, a line each: `pin <id>`, `recent <id>`,
/// `alias <id>\t<word>`, `hotkey <id>\t<chord>` and `opens <value>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Memory {
    pub pinned: Vec<String>,
    pub recent: Vec<String>,
    pub aliases: HashMap<String, String>,
    pub hotkeys: HashMap<String, String>,
    pub opens: Opens,
}

fn memory_file() -> Option<PathBuf> {
    deck_file().map(|deck| deck.with_file_name("native-palette"))
}

impl Memory {
    pub fn read() -> Memory {
        memory_file().and_then(|path| std::fs::read_to_string(path).ok()).map(|text| Memory::parse(&text)).unwrap_or_default()
    }

    pub fn parse(text: &str) -> Memory {
        let mut memory = Memory::default();
        for line in text.lines() {
            let Some((head, rest)) = line.split_once(' ') else { continue };
            match head {
                "pin" => memory.pinned.push(rest.to_string()),
                "recent" => memory.recent.push(rest.to_string()),
                "opens" => memory.opens = Opens::of(rest),
                "alias" | "hotkey" => {
                    if let Some((id, value)) = rest.split_once('\t') {
                        let book = if head == "alias" { &mut memory.aliases } else { &mut memory.hotkeys };
                        book.insert(id.to_string(), value.to_string());
                    }
                }
                _ => {}
            }
        }
        memory
    }

    pub fn text(&self) -> String {
        let mut lines = vec![format!("opens {}", self.opens.word())];
        lines.extend(self.pinned.iter().map(|id| format!("pin {id}")));
        lines.extend(self.recent.iter().map(|id| format!("recent {id}")));
        let mut aliases: Vec<_> = self.aliases.iter().collect();
        aliases.sort();
        lines.extend(aliases.into_iter().map(|(id, word)| format!("alias {id}\t{word}")));
        let mut hotkeys: Vec<_> = self.hotkeys.iter().collect();
        hotkeys.sort();
        lines.extend(hotkeys.into_iter().map(|(id, chord)| format!("hotkey {id}\t{chord}")));
        lines.join("\n")
    }

    pub fn toggle_pin(&mut self, id: &str) {
        match self.pinned.iter().position(|one| one == id) {
            Some(at) => {
                self.pinned.remove(at);
            }
            None => self.pinned.push(id.to_string()),
        }
    }

    /// A row used moves to the front of Recent, which holds twelve.
    pub fn used(&mut self, id: &str) {
        self.recent.retain(|one| one != id);
        self.recent.insert(0, id.to_string());
        self.recent.truncate(12);
    }
}

// ---- the box ---------------------------------------------------------------------------------

/// The act panel, while it is up over a row.
struct Panel {
    entry: usize,
    acts: Vec<RowAct>,
    cursor: usize,
    field: Entity<InputState>,
    _typed: Subscription,
    /// An act waiting for the word or the chord it takes.
    taking: Option<RowAct>,
    /// An act that went to Fleet and has not come back.
    going: Option<&'static str>,
    refused: Option<SharedString>,
}

/// A Stop waiting on the person's say.
struct Confirm {
    job: String,
    title: String,
}

struct Open {
    query: Entity<InputState>,
    _typed: Subscription,
    entries: Vec<Entry>,
    lines: Vec<Line>,
    state: ListState,
    cursor: usize,
    panel: Option<Panel>,
    confirm: Option<Confirm>,
    back_to: Option<FocusHandle>,
    on: Option<String>,
    /// The cursor is on the row that starts what was typed, at the foot of the matches.
    sends: bool,
}

/// What an act sends to Fleet.
enum Request {
    Post(&'static str, String),
    Merge(String),
    Address(String),
}

pub struct Palette {
    list: Entity<JobsList>,
    memory: Memory,
    remember: bool,
    open: Option<Open>,
    focus: FocusHandle,
    keeping: Option<gpui::Task<()>>,
    /// Hears every key while the palette is up.
    keys: Option<Subscription>,
}

/// The acts the panel's field narrows to.
fn panel_acts(panel: &Panel, cx: &App) -> Vec<RowAct> {
    let typed = panel.field.read(cx).value().to_lowercase();
    if panel.taking.is_some() || typed.trim().is_empty() {
        return panel.acts.clone();
    }
    panel.acts.iter().filter(|act| act.label.to_lowercase().contains(typed.trim())).cloned().collect()
}

impl Palette {
    pub fn new(list: Entity<JobsList>, cx: &mut Context<Self>) -> Self {
        let remember = list.read(cx).remembers();
        Palette { list, memory: if remember { Memory::read() } else { Memory::default() }, remember, open: None, focus: cx.focus_handle(), keeping: None, keys: None }
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.list.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// Write what the person gave the palette, off the main thread.
    fn keep(&mut self, cx: &mut Context<Self>) {
        if !self.remember {
            return;
        }
        let Some(path) = memory_file() else { return };
        let text = self.memory.text();
        self.keeping = Some(cx.spawn(async move |_, cx| {
            cx.background_executor()
                .spawn(async move {
                    if let Err(why) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text)) {
                        Note::warn("the palette's memory was not kept").field("why", why.to_string()).emit();
                    }
                })
                .await;
        }));
    }

    fn entries_now(&self, cx: &App) -> (Vec<Entry>, Option<String>) {
        let list = self.list.read(cx);
        let on = list.opened_id().and_then(|id| list.record(&id).map(|job| crate::jobs::titled(job.title.as_deref())));
        let mut entries = registered(on.is_some());
        // New Side Job only where a Side Job can be started.
        entries.retain(|entry| entry.id != "act:new_terminal_job" || crate::terminal::shells::starts(cx));
        entries.extend(fold_commands(list));
        entries.extend(list.records().iter().filter_map(job_entry));
        (entries, on)
    }

    /// Open the palette over whatever has the keys; Esc gives them back.
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.is_some() {
            return;
        }
        let back_to = window.focused(cx);
        let (entries, on) = self.entries_now(cx);
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search actions, jobs and settings"));
        let typed = cx.subscribe_in(&query, window, |this, query, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = query.read(cx).value().to_string();
                this.relist(&text, cx);
            }
        });
        let lines = lines_of(&entries, "", on.as_deref(), self.memory.opens, &self.memory);
        let state = ListState::new(lines.len(), ListAlignment::Top, px(200.));
        let cursor = lines.iter().position(|line| matches!(line, Line::Row(_))).unwrap_or(0);
        self.open = Some(Open { query: query.clone(), _typed: typed, entries, lines, state, cursor, panel: None, confirm: None, back_to, on, sends: false });
        query.update(cx, |query, cx| query.focus(window, cx));
        let (me, here) = (cx.entity().downgrade(), window.window_handle().window_id());
        self.keys = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != here {
                return;
            }
            let Some(me) = me.upgrade() else { return };
            if me.update(cx, |palette, cx| palette.on_keystroke(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        }));
        cx.notify();
    }

    /// Shut it and give the keys back to whoever had them.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.take() else { return };
        self.keys = None;
        match open.back_to {
            Some(handle) => window.focus(&handle, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    fn relist(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_mut() else { return };
        open.lines = lines_of(&open.entries, text, open.on.as_deref(), self.memory.opens, &self.memory);
        open.state.reset(open.lines.len());
        open.cursor = open.lines.iter().position(|line| matches!(line, Line::Row(_))).unwrap_or(0);
        open.sends = !text.trim().is_empty() && !open.lines.iter().any(|line| matches!(line, Line::Row(_)));
        open.panel = None;
        cx.notify();
    }

    /// The entry the cursor is over.
    fn under_cursor(&self) -> Option<(usize, Entry)> {
        let open = self.open.as_ref().filter(|open| !open.sends)?;
        match open.lines.get(open.cursor)? {
            Line::Row(at) => Some((*at, open.entries[*at].clone())),
            Line::Heading(_) => None,
        }
    }

    fn entry_at(&self, at: usize) -> Option<Entry> {
        self.open.as_ref().and_then(|open| open.entries.get(at).cloned())
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_mut() else { return };
        if let Some(panel) = open.panel.as_mut() {
            let shown = panel_acts(panel, cx).len().max(1);
            panel.cursor = (panel.cursor as isize + by).clamp(0, shown as isize - 1) as usize;
            cx.notify();
            return;
        }
        let rows: Vec<usize> = open.lines.iter().enumerate().filter(|(_, line)| matches!(line, Line::Row(_))).map(|(at, _)| at).collect();
        let offered = !open.query.read(cx).value().trim().is_empty();
        // The row that starts what was typed is the last stop, under the matches.
        if open.sends {
            if by < 0 && !rows.is_empty() {
                open.sends = false;
                open.cursor = *rows.last().unwrap_or(&0);
                open.state.scroll_to_reveal_item(open.cursor);
                cx.notify();
            }
            return;
        }
        if rows.is_empty() {
            return;
        }
        let here = rows.iter().position(|at| *at == open.cursor).unwrap_or(0) as isize;
        if offered && here + by >= rows.len() as isize {
            open.sends = true;
            cx.notify();
            return;
        }
        open.cursor = rows[(here + by).clamp(0, rows.len() as isize - 1) as usize];
        open.state.scroll_to_reveal_item(open.cursor);
        cx.notify();
    }

    // ---- the act panel -----------------------------------------------------------------------

    /// The acts of a row.
    fn acts_of(&self, entry: &Entry, cx: &App) -> Vec<RowAct> {
        let alias = self.memory.aliases.contains_key(&entry.id);
        let hotkey = self.memory.hotkeys.contains_key(&entry.id);
        let pinned = self.memory.pinned.contains(&entry.id);
        match entry.id.strip_prefix("session:") {
            Some(id) => {
                let list = self.list.read(cx);
                let Some(job) = list.record(id) else { return Vec::new() };
                let mut acts = job_acts(is_paused(job), is_snoozed(job), pinned, held_pull(job).is_some(), &without_of(job));
                // A Job on another Mac is read here: the acts that go to Fleet are listed, off, with why, but for Stop and
                // Merge, which Fleet carries to that Mac.
                for one in acts.iter_mut() {
                    let route = match one.id {
                        "pause" => "/sessions/pause",
                        "snooze" | "wake" => "/sessions/snooze",
                        "fork" => "/sessions/start",
                        "merge-pull" => "/pull_requests/:repository/:number/merge",
                        "stop" => "/sessions/stop",
                        _ => continue,
                    };
                    one.off = crate::pairing::refusal_to(job, route, cx).or(one.off.take());
                }
                acts.extend(memory_acts(alias, hotkey, false, None));
                acts
            }
            None => {
                let mut acts = vec![act("run", "Run", None, Some("\u{21b5}"))];
                acts.extend(memory_acts(alias, hotkey, false, Some(pinned)));
                acts
            }
        }
    }

    /// ⌘K on a row: its panel opens, or shuts.
    fn toggle_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.as_ref().is_some_and(|open| open.panel.is_some()) {
            self.shut_panel(window, cx);
            return;
        }
        let Some((at, entry)) = self.under_cursor() else { return };
        let acts = self.acts_of(&entry, cx);
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Search for actions"));
        let typed = cx.subscribe_in(&field, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(panel) = this.open.as_mut().and_then(|open| open.panel.as_mut()) {
                    panel.cursor = 0;
                    panel.refused = None;
                }
                cx.notify();
            }
        });
        if let Some(open) = self.open.as_mut() {
            open.panel = Some(Panel { entry: at, acts, cursor: 0, field: field.clone(), _typed: typed, taking: None, going: None, refused: None });
        }
        field.update(cx, |field, cx| field.focus(window, cx));
        cx.notify();
    }

    fn shut_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = self.open.as_mut() {
            open.panel = None;
            let query = open.query.clone();
            query.update(cx, |query, cx| query.focus(window, cx));
        }
        cx.notify();
    }

    /// Esc: a dialog first, then what a panel is taking, then the panel, then the palette.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_mut() else { return };
        if open.confirm.take().is_some() {
            cx.notify();
            return;
        }
        if let Some(panel) = open.panel.as_mut() {
            if panel.taking.take().is_some() {
                panel.refused = None;
                let field = panel.field.clone();
                field.update(cx, |field, cx| {
                    field.set_value("", window, cx);
                    field.set_placeholder("Search for actions", window, cx);
                });
                cx.notify();
            } else {
                self.shut_panel(window, cx);
            }
            return;
        }
        self.close(window, cx);
    }

    /// Return on the list runs the default act; in the panel it runs the one under the cursor.
    fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_ref() else { return };
        if open.confirm.is_some() {
            self.stop_now(window, cx);
            return;
        }
        if let Some(panel) = open.panel.as_ref() {
            let shown = panel_acts(panel, cx).get(panel.cursor).cloned();
            match (panel.taking.clone(), shown) {
                (Some(taking), _) if taking.takes == Some(Takes::Word) => self.take_word(taking, window, cx),
                (Some(_), _) => {}
                (None, Some(act)) => self.run_act(act, window, cx),
                (None, None) => {}
            }
            return;
        }
        if open.sends {
            self.start_typed(window, cx);
            return;
        }
        let Some((_, entry)) = self.under_cursor() else { return };
        let default = if entry.id.starts_with("session:") { "open" } else { "run" };
        if let Some(act) = self.acts_of(&entry, cx).into_iter().find(|one| one.id == default) {
            self.run_for(&entry, act, window, cx);
        }
    }

    /// What was typed goes to the dispatch sheet as its words (`one-bar.tsx` `onDispatch`).
    fn start_typed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_ref() else { return };
        let words = open.query.read(cx).value().trim().to_string();
        if words.is_empty() {
            return;
        }
        self.close(window, cx);
        crate::dispatch::screen::start_with(words, cx);
    }

    fn run_act(&mut self, act: RowAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.open.as_ref().and_then(|open| open.panel.as_ref()).and_then(|panel| self.entry_at(panel.entry)) else { return };
        self.run_for(&entry, act, window, cx);
    }

    /// Run an act on a row. The row says it is going in this frame; a refusal is said in the panel.
    fn run_for(&mut self, entry: &Entry, act: RowAct, window: &mut Window, cx: &mut Context<Self>) {
        if act.off.is_some() {
            return;
        }
        // An act that asks for a word or a chord turns the panel's field into that ask first.
        if let Some(takes) = act.takes {
            let placeholder = if takes == Takes::Word { "Type a word" } else { "Press a chord" };
            if self.open.as_ref().is_some_and(|open| open.panel.is_none()) {
                self.toggle_panel(window, cx);
            }
            if let Some(panel) = self.open.as_mut().and_then(|open| open.panel.as_mut()) {
                panel.taking = Some(act);
                panel.refused = None;
                let field = panel.field.clone();
                field.update(cx, |field, cx| {
                    field.set_value("", window, cx);
                    field.set_placeholder(placeholder, window, cx);
                    field.focus(window, cx);
                });
            }
            cx.notify();
            return;
        }
        let job = entry.id.strip_prefix("session:").map(str::to_string);
        match (act.id, job) {
            ("open", Some(job)) => {
                self.memory.used(&entry.id);
                self.keep(cx);
                self.close(window, cx);
                self.list.update(cx, |list, cx| list.open(job.into(), cx));
            }
            ("run", None) if entry.id.starts_with("cmd:") => {
                self.memory.used(&entry.id);
                self.keep(cx);
                self.close(window, cx);
                let command = entry.id.trim_start_matches("cmd:").to_string();
                self.list.update(cx, |list, cx| list.fold_command(&command, cx));
            }
            ("run", None) => {
                self.memory.used(&entry.id);
                self.keep(cx);
                let id = entry.id.strip_prefix("act:").and_then(|id| ACTIONS.iter().find(|row| row.id == id)).map(|row| row.id);
                let back = self.open.as_ref().and_then(|open| open.back_to.clone());
                self.close(window, cx);
                if let Some(id) = id {
                    // The act runs as its key would, from where the keys were.
                    window.defer(cx, move |window, cx| {
                        if let Some(back) = back {
                            window.focus(&back, cx);
                        }
                        window.dispatch_action(Box::new(Act { id, slot: 0, digit: None }), cx);
                    });
                }
            }
            ("pin", _) => {
                self.memory.toggle_pin(&entry.id);
                self.keep(cx);
                self.refresh(cx);
            }
            ("remove-alias", _) => {
                self.memory.aliases.remove(&entry.id);
                self.keep(cx);
                self.refresh(cx);
            }
            ("remove-hotkey", _) => {
                self.memory.hotkeys.remove(&entry.id);
                self.keep(cx);
                self.refresh(cx);
            }
            ("copy-id", Some(job)) => {
                cx.write_to_clipboard(ClipboardItem::new_string(job));
                self.memory.used(&entry.id);
                self.keep(cx);
                self.close(window, cx);
            }
            ("stop", Some(job)) => {
                let title = entry.label.clone();
                if let Some(open) = self.open.as_mut() {
                    open.confirm = Some(Confirm { job, title });
                }
                cx.notify();
            }
            (id @ ("pause" | "fork" | "open-pull" | "merge-pull"), Some(job)) => self.go(id, job, window, cx),
            // Snooze asks for its presets and Wake now posts at once: neither waits on the palette, which shuts.
            ("snooze", Some(job)) => {
                self.memory.used(&entry.id);
                self.keep(cx);
                self.close(window, cx);
                crate::dispatch::screen::snooze_ask(job.into(), entry.label.clone().into(), None, cx);
            }
            ("wake", Some(job)) => {
                self.memory.used(&entry.id);
                self.keep(cx);
                self.close(window, cx);
                self.list.update(cx, |list, cx| list.snooze(&job, None, cx));
            }
            _ => {}
        }
    }

    /// Redraw the panel's acts after the memory changed under it, and the rows that carry a word or a chord.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(at) = self.open.as_ref().and_then(|open| open.panel.as_ref()).map(|panel| panel.entry) else {
            cx.notify();
            return;
        };
        if let Some(entry) = self.entry_at(at) {
            let acts = self.acts_of(&entry, cx);
            if let Some(panel) = self.open.as_mut().and_then(|open| open.panel.as_mut()) {
                panel.acts = acts;
            }
        }
        cx.notify();
    }

    /// The word typed for an alias.
    fn take_word(&mut self, act: RowAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.open.as_ref().and_then(|open| open.panel.as_ref()) else { return };
        let word = panel.field.read(cx).value().trim().replace(['\t', '\n'], " ");
        let Some(entry) = self.entry_at(panel.entry) else { return };
        if act.id != "set-alias" || word.is_empty() {
            return;
        }
        // A word another row already answers to would find two.
        let taken = self.memory.aliases.iter().any(|(id, one)| id != &entry.id && one.to_lowercase() == word.to_lowercase());
        if taken {
            self.refused("Another row has that word".into(), cx);
            return;
        }
        self.memory.aliases.insert(entry.id.clone(), word);
        self.finish_taking(window, cx);
    }

    /// The chord pressed for a hotkey.
    fn take_chord(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.open.as_ref().and_then(|open| open.panel.as_ref()) else { return };
        let Some(entry) = self.entry_at(panel.entry) else { return };
        let Some(chord) = spelled(keystroke) else { return };
        let taken = self.memory.hotkeys.iter().any(|(id, one)| id != &entry.id && one == &chord) || ACTIONS.iter().any(|row| row.shortcut == chord);
        if taken {
            self.refused("That chord is in use".into(), cx);
            return;
        }
        self.memory.hotkeys.insert(entry.id.clone(), chord);
        self.finish_taking(window, cx);
    }

    fn finish_taking(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.keep(cx);
        if let Some(panel) = self.open.as_mut().and_then(|open| open.panel.as_mut()) {
            panel.taking = None;
            panel.refused = None;
            let field = panel.field.clone();
            field.update(cx, |field, cx| {
                field.set_value("", window, cx);
                field.set_placeholder("Search for actions", window, cx);
                field.focus(window, cx);
            });
        }
        self.refresh(cx);
    }

    /// Stop was confirmed.
    fn stop_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(confirm) = self.open.as_mut().and_then(|open| open.confirm.take()) else { return };
        self.go("stop", confirm.job, window, cx);
    }

    /// An act that goes to Fleet: the row reads it is going now, the palette shuts on the answer, and a refusal
    /// stays in the panel with Fleet's reason.
    fn go(&mut self, id: &'static str, job: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(record) = self.list.read(cx).record(&job).cloned() else { return };
        let session_id = SessionId::carried(job.clone());
        let paused = is_paused(&record);
        let pull = held_pull(&record).map(|(repo, number, _)| (repo, number));
        let said = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
        let request: Result<Request, String> = match id {
            "pause" if paused => said(ipc::encode(&ResumeSession { session_id })).map(|body| Request::Post("/sessions/resume", body)),
            "pause" => said(ipc::encode(&PauseSession { session_id })).map(|body| Request::Post("/sessions/pause", body)),
            "stop" => said(ipc::encode(&StopSession { session_id })).map(|body| Request::Post("/sessions/stop", body)),
            "fork" => match record.manifest_id.clone() {
                // A live Job is forked at a row of its thread; the whole conversation is for an ended one. Fleet's
                // refusal of a live Job without a row is said in the panel.
                Some(manifest_id) => {
                    let start = StartSession {
                        manifest_id,
                        title: None,
                        model: None,
                        effort: None,
                        mode: None,
                        fork: Some(ForkFrom { session_id, at: None }),
                        root: None,
                        request: None,
                        after: None,
                        held: false,
                        project_rows: Vec::new(),
                        project: None,
                        on_your_words: None,
                    };
                    said(ipc::encode(&start)).map(|body| Request::Post("/sessions/start", body))
                }
                None => Err("This Job has no repository to fork in".into()),
            },
            "open-pull" => pull.map(|(repo, number)| Request::Address(format!("/pull_requests/{}/{number}", repo.as_str()))).ok_or_else(|| "This Job holds no pull request".to_string()),
            "merge-pull" => pull.map(|(repo, number)| Request::Merge(format!("/pull_requests/{}/{number}/merge", repo.as_str()))).ok_or_else(|| "This Job holds no pull request".to_string()),
            _ => return,
        };
        self.memory.used(&format!("session:{job}"));
        self.keep(cx);
        // Pressed: the act's row dims and the refusal of an earlier press is gone.
        if let Some(panel) = self.open.as_mut().and_then(|open| open.panel.as_mut()) {
            panel.going = Some(id);
            panel.refused = None;
        }
        cx.notify();
        let request = match request {
            Ok(request) => request,
            Err(why) => return self.refused(why.into(), cx),
        };
        let Some(port) = self.port(cx) else {
            // Mock data has no Fleet: the act is taken as done.
            if self.list.read(cx).fleet().is_none() {
                self.close(window, cx);
            } else {
                self.refused("Fleet is not connected".into(), cx);
            }
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let answer: Result<Vec<u8>, String> = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = match request {
                        Request::Post(path, body) => runtime.block_on(crate::transport::post(port, path, body))?,
                        Request::Merge(path) => runtime.block_on(crate::transport::post_bare(port, &path))?,
                        Request::Address(path) => runtime.block_on(crate::transport::get(port, &path))?,
                    };
                    if (200..300).contains(&status) {
                        return Ok(bytes);
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(bytes) => {
                    if id == "open-pull" {
                        match ipc::decode::<ipc::PullRequestState>("PullRequestState", &bytes) {
                            Ok(state) => cx.open_url(&state.address),
                            Err(why) => return this.refused(why.to_string().into(), cx),
                        }
                    }
                    this.close(window, cx);
                    if id == "fork" {
                        if let Ok(forked) = ipc::decode::<SessionRecord>("SessionRecord", &bytes) {
                            let forked = forked.id.as_str().to_string();
                            this.list.update(cx, |list, cx| list.open(forked.into(), cx));
                        }
                    }
                }
                Err(why) => this.refused(why.into(), cx),
            });
        })
        .detach();
    }

    fn refused(&mut self, why: SharedString, cx: &mut Context<Self>) {
        if let Some(panel) = self.open.as_mut().and_then(|open| open.panel.as_mut()) {
            panel.going = None;
            panel.refused = Some(why);
        }
        cx.notify();
    }

    // ---- keys ----------------------------------------------------------------------------------

    /// A chord a row was given, pressed anywhere with the palette shut: it runs the row's default act.
    pub fn hotkey(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.open.is_some() {
            return false;
        }
        let Some(chord) = spelled(keystroke) else { return false };
        let Some(id) = self.memory.hotkeys.iter().find(|(_, one)| **one == chord).map(|(id, _)| id.clone()) else { return false };
        let (entries, _) = self.entries_now(cx);
        let Some(entry) = entries.into_iter().find(|entry| entry.id == id) else { return false };
        self.memory.used(&id);
        self.keep(cx);
        match entry.id.strip_prefix("session:") {
            Some(job) => self.list.update(cx, |list, cx| list.open(job.to_string().into(), cx)),
            None => {
                if let Some(command) = entry.id.strip_prefix("cmd:") {
                    self.list.update(cx, |list, cx| list.fold_command(command, cx));
                } else if let Some(row) = entry.id.strip_prefix("act:").and_then(|id| ACTIONS.iter().find(|row| row.id == id)) {
                    window.dispatch_action(Box::new(Act { id: row.id, slot: 0, digit: None }), cx);
                }
            }
        }
        true
    }

    /// A keystroke, before anything else hears it: whether the palette took it. The palette holds every key
    /// while it is up, so a chord the registry binds, such as ⌘K itself, is read here and not as that act.
    fn on_keystroke(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.open.is_none() {
            return false;
        }
        let key = keystroke.key.as_str();
        let mods = keystroke.modifiers;
        if self.open.as_ref().is_some_and(|open| open.confirm.is_some()) {
            match key {
                "escape" => self.escape(window, cx),
                "enter" => self.enter(window, cx),
                _ => {}
            }
            return true;
        }
        let taking_chord = self.open.as_ref().and_then(|open| open.panel.as_ref()).is_some_and(|panel| panel.taking.as_ref().is_some_and(|act| act.takes == Some(Takes::Chord)));
        if taking_chord && key != "escape" {
            if spelled(keystroke).is_some() {
                self.take_chord(keystroke, window, cx);
            }
            return !matches!(key, "shift" | "control" | "alt" | "platform" | "function");
        }
        // ⌘K on a row opens its panel, and on the panel shuts it.
        if mods.platform && !mods.shift && !mods.alt && key == "k" {
            self.toggle_panel(window, cx);
            return true;
        }
        // With words typed and no panel up, ⌘↵ starts them from any row.
        let offered = self.open.as_ref().is_some_and(|open| open.panel.is_none() && !open.query.read(cx).value().trim().is_empty());
        if offered && mods.platform && !mods.shift && !mods.alt && key == "enter" {
            self.start_typed(window, cx);
            return true;
        }
        // The chords run the act on the row under the cursor, with the panel up or shut.
        let target = match self.open.as_ref().and_then(|open| open.panel.as_ref()) {
            Some(panel) => self.entry_at(panel.entry),
            None => self.under_cursor().map(|(_, entry)| entry),
        };
        if let Some(entry) = target {
            let found = self.acts_of(&entry, cx).into_iter().find(|act| act.chord.is_some_and(|chord| chord != "\u{21b5}" && chord_is(chord, keystroke)));
            if let Some(act) = found {
                self.run_for(&entry, act, window, cx);
                return true;
            }
        }
        match key {
            "escape" => self.escape(window, cx),
            "down" => self.step(1, cx),
            "up" => self.step(-1, cx),
            "enter" => self.enter(window, cx),
            _ => return false,
        }
        true
    }
}

// ---- drawing ---------------------------------------------------------------------------------

/// The palette's foot (`ResultKeys`, `DispatchKeys`): each key after its word. With words typed, ⌘↵ starts them.
fn foot_keys(offered: bool, sends: bool) -> Vec<(&'static str, &'static str)> {
    if !offered {
        return vec![("Open", "\u{21b5}"), ("Actions", "\u{2318}K")];
    }
    let mut keys = vec![(if sends { "Dispatch" } else { "Open" }, "\u{21b5}"), ("Dispatch", "\u{2318}\u{21b5}")];
    if !sends {
        keys.push(("Actions", "\u{2318}K"));
    }
    keys
}

/// The list is taller than Electron's 400 px by this much, as the owner asked for a roomier palette.
const LIST_GROWS: f32 = 1.15;

/// The label with the typed span marked by contrast: the rest steps down to muted (`Marked`, `.armada-palette__mark`).
/// A row found another way, as by its alias, marks nothing.
fn marked(label: &str, typed: &str, look: &Look) -> gpui::StyledText {
    let found = (!typed.is_empty()).then(|| label.to_lowercase().find(&typed.to_lowercase())).flatten().filter(|at| label.is_char_boundary(*at) && label.is_char_boundary(*at + typed.len()));
    let Some(at) = found else { return gpui::StyledText::new(SharedString::from(label.to_string())) };
    let muted = gpui::HighlightStyle { color: Some(look.color(token::FG_MUTED)), ..Default::default() };
    let spans = [(0..at, muted), (at + typed.len()..label.len(), muted)].into_iter().filter(|(range, _)| !range.is_empty()).collect::<Vec<_>>();
    gpui::StyledText::new(SharedString::from(label.to_string())).with_highlights(spans)
}

fn glyph(look: &Look, name: &'static str, ink: gpui::Hsla) -> gpui::Svg {
    svg().path(icon_path(name)).flex_none().size(look.length(token::SPACE_4)).text_color(ink)
}

fn tip(said: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView {
    let said: SharedString = said.into();
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
}

/// A key chip.
fn kbd(look: &Look, text: &str) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_KBD))
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_b(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .text_color(look.color(token::FG_MUTED))
        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .child(SharedString::from(text.replace("Backspace", "\u{232b}")))
}

/// The row under the cursor: one neutral rounded fill, no bar, no accent.
fn on_row(look: &Look, row: gpui::Stateful<gpui::Div>, on: bool) -> gpui::Stateful<gpui::Div> {
    if on {
        row.bg(look.color(token::SURFACE_ROW_SELECTED))
    } else {
        row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
    }
}

impl Palette {
    fn render_line(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let Some(open) = self.open.as_ref() else { return div().into_any_element() };
        match open.lines.get(ix) {
            Some(Line::Heading(title)) => labelled(
                div()
                    .id(SharedString::from(format!("palette-heading:{title}")))
                    .w_full()
                    .h(look.length(token::SPACE_8) - look.length(token::SPACE_1))
                    .flex()
                    .items_center()
                    .px(look.length(token::SPACE_2))
                    // `.armada-palette__section` in the new look (`depth.css`): 2xs mono, heading weight, subtle.
                    .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(title.clone()),
                Role::Heading,
                title.clone(),
            )
            .into_any_element(),
            Some(Line::Row(at)) => {
                let entry = open.entries[*at].clone();
                self.render_row(ix, &entry, look, cx)
            }
            None => div().into_any_element(),
        }
    }

    fn render_row(&self, ix: usize, entry: &Entry, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(open) = self.open.as_ref() else { return div().into_any_element() };
        let on = open.cursor == ix;
        let on = on && !open.sends;
        let state = entry.meta.as_ref().and_then(|meta| meta.state.clone());
        let leading = match (state, entry.icon) {
            (Some((icon, said, hue)), _) => labelled(
                div().id(SharedString::from(format!("palette-state:{}", entry.id))).flex_none().tooltip(tip(said.clone())).child(glyph(look, icon, look.color(hue))),
                Role::Image,
                said,
            )
            .into_any_element(),
            (None, Some(icon)) => glyph(look, icon, look.color(token::FG_MUTED)).into_any_element(),
            (None, None) => div().flex_none().size(look.length(token::SPACE_4)).into_any_element(),
        };
        let alias = self.memory.aliases.get(&entry.id).cloned();
        let label = marked(&entry.label, open.query.read(cx).value().trim(), look);
        let id = entry.id.clone();
        let row = on_row(
            look,
            div()
                .id(SharedString::from(format!("palette-row:{}", entry.id)))
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .h(look.length(token::SPACE_8))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT)),
            on,
        )
        .child(leading)
        .child(div().flex_1().min_w_0().truncate().child(label))
        .children(alias.map(|word| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(word))))
        .children(entry.meta.as_ref().and_then(|meta| meta.when.clone()).map(|(time, said)| {
            div()
                .id(SharedString::from(format!("palette-when:{}", entry.id)))
                .flex_none()
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .tooltip(tip(said))
                .child(SharedString::from(time))
        }))
        .child(
            div()
                .flex_none()
                .w(look.length(token::SPACE_12) * 2.)
                .flex()
                .justify_end()
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(entry.kind()),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            if let Some(open) = this.open.as_mut() {
                if let Some(at) = open.lines.iter().position(|line| matches!(line, Line::Row(row) if open.entries[*row].id == id)) {
                    open.cursor = at;
                }
            }
            this.enter(window, cx);
        }));
        labelled(row, Role::ListBoxOption, entry.label.clone()).into_any_element()
    }

    /// What was typed, offered as the work itself (`DispatchRow`): the accent plus, the words in quotes, the kind.
    fn render_start(&self, typed: &str, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (said, kind) = ("Start a Job with", "Job");
        let name = format!("{said} \u{201c}{typed}\u{201d}");
        let row = div()
            .id("palette-start")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::SPACE_8))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .when(on, |row| row.bg(look.color(token::BG_HOVER)))
            .when(!on, |row| row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))))
            .child(glyph(look, "plus", look.color(token::ACCENT)))
            .child(div().flex_1().min_w_0().truncate().child({
                // The words in their quotes are the row's weight (`.armada-palette__words`).
                let words = gpui::HighlightStyle { font_weight: Some(FontWeight(look.number(token::WEIGHT_MEDIUM))), ..Default::default() };
                gpui::StyledText::new(SharedString::from(name.clone())).with_highlights([(said.len() + 1..name.len(), words)])
            }))
            .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(kind))
            .on_click(cx.listener(|this, _, window, cx| this.start_typed(window, cx)));
        div()
            .flex_none()
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(row, Role::ListBoxOption, name))
            .into_any_element()
    }

    fn render_panel(&self, panel: &Panel, entry: &Entry, look: &Look, cx: &Context<Self>) -> AnyElement {
        let acts = panel_acts(panel, cx);
        let rows = acts.iter().enumerate().map(|(at, shown)| {
            let on = panel.cursor == at;
            let going = panel.going == Some(shown.id);
            let ink = if shown.off.is_some() {
                token::FG_SUBTLE
            } else if shown.destructive {
                token::STATUS_COMPLETED_FAILED
            } else {
                token::FG_DEFAULT
            };
            let off = shown.off.clone().map(SharedString::from);
            let run = shown.clone();
            let held = match shown.id {
                "set-alias" => self.memory.aliases.get(&entry.id).cloned(),
                "record-hotkey" => self.memory.hotkeys.get(&entry.id).cloned(),
                _ => None,
            };
            let row = on_row(
                look,
                div()
                    .id(SharedString::from(format!("palette-act:{}", shown.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .h(look.length(token::SPACE_8))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(ink)),
                on,
            )
            .when(going, |row| row.opacity(0.5))
            .when_some(off, |row, why| row.tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(why.clone()).build(window, cx)))
            .child(match shown.icon {
                Some(icon) => glyph(look, icon, look.color(if shown.off.is_some() { token::FG_SUBTLE } else if shown.destructive { token::STATUS_COMPLETED_FAILED } else { token::FG_MUTED })).into_any_element(),
                None => div().flex_none().size(look.length(token::SPACE_4)).into_any_element(),
            })
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(shown.label.clone())))
            .children(held.map(|value| {
                div().flex_none().max_w_1_2().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(value))
            }))
            .children(shown.chord.map(|chord| {
                let cap = kbd(look, chord);
                if shown.off.is_some() { cap.text_color(look.color(token::FG_SUBTLE)) } else { cap }
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.run_act(run.clone(), window, cx)));
            div()
                .flex()
                .flex_col()
                .when(shown.apart && at > 0, |column| {
                    column.child(div().h(look.length(token::BORDER_WIDTH)).mx(look.length(token::SPACE_2)).my(look.length(token::SPACE_1)).bg(look.color(token::BORDER_SUBTLE)))
                })
                .child(labelled(row, Role::MenuItem, shown.label.clone()))
                .into_any_element()
        });
        let title = match &panel.taking {
            Some(taking) => format!("{} \u{b7} {}", taking.label, entry.label),
            None => entry.label.clone(),
        };
        div()
            .id("palette-actions")
            .absolute()
            .right(look.length(token::SPACE_2))
            .bottom(look.length(token::SPACE_8) + look.length(token::SPACE_1))
            .w(look.length(token::PALETTE_WIDTH) / 1.6)

            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
            .child(
                div()
                    .flex_none()
                    .px(look.length(token::SPACE_3))
                    .pt(look.length(token::SPACE_2))
                    .pb(look.length(token::SPACE_1))
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(SharedString::from(title)),
            )
            .child(labelled(
                div().id("palette-actions-list").flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_1)).pb(look.length(token::SPACE_1)).children(rows),
                Role::Menu,
                "Actions",
            ))
            .children(panel.refused.clone().map(|why| {
                labelled(
                    div()
                        .id("palette-refused")
                        .px(look.length(token::SPACE_3))
                        .pb(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::STATUS_ESCALATED))
                        .child(why),
                    Role::Status,
                    "Refused",
                )
            }))
            .child(div().flex_none().flex().items_center().border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(labelled(
                div()
                    .id("palette-actions-field")
                    .key_context(crate::keymap::TEXT_BOX)
                    .flex_1()
                    .h(look.length(token::H_MENU_ITEM))
                    .px(look.length(token::SPACE_3))
                    .flex()
                    .items_center()
                    .text_size(look.length(token::TEXT_SM))
                    .child(Input::new(&panel.field).appearance(false)),
                Role::SearchInput,
                "Search for actions",
            )))
            .into_any_element()
    }

    fn render_confirm(&self, confirm: &Confirm, look: &Look, cx: &Context<Self>) -> AnyElement {
        let sure = labelled(
            div()
                .id("palette-stop")
                .flex()
                .items_center()
                .justify_center()
                .h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .cursor_pointer()
                .bg(look.color(token::STATUS_COMPLETED_FAILED))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .child("Stop")
                .on_click(cx.listener(|this, _, window, cx| this.stop_now(window, cx))),
            Role::Button,
            "Stop",
        );
        let keep = labelled(
            div()
                .id("palette-keep")
                .flex()
                .items_center()
                .justify_center()
                .h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .cursor_pointer()
                .text_color(look.color(token::FG_MUTED))
                .text_size(look.length(token::TEXT_SM))
                .hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
                .child("Cancel")
                .on_click(cx.listener(|this, _, window, cx| this.escape(window, cx))),
            Role::Button,
            "Cancel",
        );
        div()
            .id("palette-confirm-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(look.color(token::SCRIM))
            .occlude()
            .child(labelled(
                div()
                    .id("palette-confirm")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    .w(look.length(token::SPACE_12) * 5.)
                    .max_w_full()
                    .p(look.length(token::SPACE_6))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_BASE))
                            .line_height(look.length(token::LEADING_BASE))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(format!("Stop {}?", confirm.title))),
                    )
                    .child(div().flex().items_center().justify_end().gap(look.length(token::SPACE_2)).child(keep).child(sure)),
                Role::Dialog,
                "Stop",
            ))
            .into_any_element()
    }
}

impl Render for Palette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(open) = self.open.as_ref() else { return div().into_any_element() };
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let body = if open.lines.is_empty() && !open.query.read(cx).value().trim().is_empty() {
            div().into_any_element()
        } else if open.lines.is_empty() {
            labelled(
                div()
                    .id("palette-empty")
                    .p(look.length(token::SPACE_4))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child("No match"),
                Role::Alert,
                "No match",
            )
            .into_any_element()
        } else {
            list(open.state.clone(), cx.processor(|this, ix: usize, _window, cx| this.render_line(ix, cx))).size_full().into_any_element()
        };
        // The box is as tall as its rows, up to the cap, as Electron's is.
        let lines: gpui::Pixels = open
            .lines
            .iter()
            .map(|line| match line {
                Line::Heading(_) => look.length(token::SPACE_8) - look.length(token::SPACE_1),
                Line::Row(_) => look.length(token::SPACE_8),
            })
            .sum();
        let typed = open.query.read(cx).value().trim().to_string();
        let start = (!typed.is_empty()).then(|| self.render_start(&typed, open.sends, look, cx));
        // The start row is pinned under the list, so a long list never hides it.
        let pinned = if start.is_some() { look.length(token::SPACE_8) + look.length(token::BORDER_WIDTH) } else { px(0.) };
        let tall = (!open.lines.is_empty() || start.is_some()).then(|| (lines + pinned + look.length(token::SPACE_2) * 2.).min(look.length(token::PALETTE_MAX_HEIGHT) * LIST_GROWS));
        // While a row's acts are open the list keeps eleven rows' room, or the panel's own where it needs more, so
        // the panel sits inside it under the field, as Electron's `[data-acting]` list does.
        let acting = open.panel.as_ref().map(|panel| {
            let acts = panel_acts(panel, cx);
            let rules = acts.iter().enumerate().filter(|(at, act)| act.apart && *at > 0).count() as f32;
            let rows = acts.len() as f32;
            look.length(token::SPACE_8) * (rows + 1.)
                + (look.length(token::BORDER_WIDTH) + look.length(token::SPACE_2)) * rules
                + look.length(token::H_MENU_ITEM)
                + look.length(token::SPACE_2)
        });
        let tall = match acting {
            Some(room) => Some(tall.unwrap_or_default().max(look.length(token::H_MENU_ITEM) * 11. + look.length(token::SPACE_4)).max(room + look.length(token::SPACE_2))),
            None => tall,
        };
        let panel = open.panel.as_ref().and_then(|panel| self.entry_at(panel.entry).map(|entry| self.render_panel(panel, &entry, look, cx)));
        let confirm = open.confirm.as_ref().map(|confirm| self.render_confirm(confirm, look, cx));
        let query = open.query.clone();
        let wide = (look.length(token::PALETTE_WIDTH) * 1.2).min(viewport.width - look.length(token::SPACE_6));
        let boxed = labelled(
            div()
                .id("palette-box")
                .relative()
                .occlude()
                .w(wide)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_HIGHLIGHT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::SPACE_12))
                        .px(look.length(token::SPACE_4))
                        .border_b(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(labelled(
                            div()
                                .id("palette-input")
                                .key_context(crate::keymap::TEXT_BOX)
                                .flex_1()
                                .text_size(look.length(token::TEXT_LG))
                                .line_height(look.length(token::LEADING_LG))
                                .child(Input::new(&query).appearance(false)),
                            Role::SearchInput,
                            "Search actions, jobs and settings",
                        )),
                )
                .child(labelled(
                    div()
                        .id("palette-list")
                        .flex_none()
                        .when_some(tall, |list, tall| list.h(tall))
                        .flex()
                        .flex_col()
                        .p(look.length(token::SPACE_2))
                        .child(div().flex_1().min_h_0().child(body))
                        .children(start),
                    Role::ListBox,
                    "Results",
                ))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_wrap()
                        .justify_end()
                        .gap_x(look.length(token::SPACE_4))
                        .gap_y(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .border_t(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .children(foot_keys(!query.read(cx).value().trim().is_empty() && open.panel.is_none(), open.sends).into_iter().map(|(said, key)| {
                            div().flex().items_center().gap(look.length(token::SPACE_1)).child(said).child(kbd(look, key))
                        })),
                )
                .children(panel)
                .children(confirm),
            Role::Dialog,
            "Command palette",
        );
        deferred(
            anchored().position(room.origin).anchor(Anchor::TopLeft).child(
                div()
                    .id("palette-layer")
                    .track_focus(&self.focus)
                    .key_context("palette")
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .justify_center()
                    .items_start()
                    .pt(viewport.height * 0.2)
                    .bg(look.color(token::SCRIM))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)))
                    .child(boxed),
            ),
        )
        .with_priority(2)
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-many").expect("the fixture decodes")
    }

    fn all() -> Vec<Entry> {
        let mut entries = registered(true);
        entries.extend(jobs().iter().filter_map(job_entry));
        entries
    }

    fn headings(lines: &[Line]) -> Vec<String> {
        lines.iter().filter_map(|line| if let Line::Heading(title) = line { Some(title.clone()) } else { None }).collect()
    }

    fn ids(acts: Vec<RowAct>) -> Vec<&'static str> {
        acts.into_iter().map(|act| act.id).collect()
    }

    #[test]
    fn the_blocks_stand_in_the_order_the_contract_gives() {
        assert_eq!(
            SECTION_ORDER.map(|section| section.title(Some("Fix the chip"))),
            ["Page", "Fix the chip", "Navigation", "Commands", "Run in the checkout", "Repositories", "Jobs", "Pull requests", "Issues", "Tasks", "Settings"],
        );
        assert_eq!(Section::Context.title(None), "Nothing focused");
    }

    #[test]
    fn everything_opens_with_the_focused_block_navigation_and_the_jobs() {
        let lines = lines_of(&all(), "", Some("Fix the chip"), Opens::Everything, &Memory::default());
        assert_eq!(headings(&lines), ["Fix the chip", "Navigation", "Jobs"]);
    }

    #[test]
    fn pinned_and_recent_open_first_and_a_pinned_row_is_not_recent_too() {
        let entries = all();
        let job = entries.iter().find(|entry| entry.section == Section::Jobs).expect("a job").id.clone();
        let act = entries.iter().find(|entry| entry.id == "act:message_job").expect("an act").id.clone();
        let memory = Memory { pinned: vec![job.clone()], recent: vec![job.clone(), act], ..Memory::default() };
        let lines = lines_of(&entries, "", None, Opens::Pinned, &memory);
        assert_eq!(headings(&lines), ["Pinned", "Recent"]);
        assert_eq!(lines.len(), 4);
        // Nothing pinned or used: the whole list opens.
        assert!(headings(&lines_of(&entries, "", None, Opens::Pinned, &Memory::default())).contains(&"Jobs".to_string()));
        // This page first shows the whole list even with pins.
        assert!(headings(&lines_of(&entries, "", None, Opens::Page, &memory)).contains(&"Jobs".to_string()));
    }

    #[test]
    fn a_word_given_to_a_row_and_typed_whole_puts_it_first_under_no_heading() {
        let entries = all();
        let job = entries.iter().find(|entry| entry.section == Section::Jobs).expect("a job").id.clone();
        let memory = Memory { aliases: HashMap::from([(job.clone(), "chip".to_string())]), ..Memory::default() };
        let lines = lines_of(&entries, "chip", None, Opens::Everything, &memory);
        assert!(matches!(&lines[0], Line::Row(at) if entries[*at].id == job));
    }

    #[test]
    fn a_job_row_carries_its_repository_and_its_state_and_a_quiet_job_is_found_only_when_typed_and_an_ended_one_last() {
        let entries = all();
        let row = entries.iter().find(|entry| entry.label == "Fix the merge line freshness chip").expect("listed");
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.repo.as_deref(), Some("armada"));
        assert!(meta.state.is_some());
        assert_eq!(row.kind(), "Job");
        assert!(row.aliases.iter().any(|one| one == "#3388"), "{:?}", row.aliases);
        assert!(!row.typed_only && !row.ended);
        let quiet = entries.iter().find(|entry| entry.label.contains("Retry webhooks")).expect("a quiet Job is a row");
        let ended = entries.iter().find(|entry| entry.label.contains("ended")).expect("an ended Job is a row");
        assert!(quiet.typed_only && !quiet.ended);
        assert!(ended.typed_only && ended.ended);
        let found = |query: &str| -> Vec<String> {
            lines_of(&entries, query, None, Opens::Everything, &Memory::default())
                .into_iter()
                .filter_map(|line| if let Line::Row(at) = line { Some(entries[at].label.clone()) } else { None })
                .collect()
        };
        assert!(found("retry").contains(&quiet.label), "typed, a quiet Job is found");
        assert!(found("ended").contains(&ended.label), "typed, an ended Job is found");
    }

    #[test]
    fn ended_and_quiet_jobs_are_not_in_the_opening_list_not_even_pinned() {
        let entries = all();
        let id_of = |word: &str| entries.iter().find(|entry| entry.label.contains(word)).expect("a row").id.clone();
        let (quiet, ended) = (id_of("Retry webhooks"), id_of("ended"));
        let memory = Memory { pinned: vec![ended.clone(), quiet.clone()], recent: vec![quiet.clone()], ..Memory::default() };
        for (opens, memory) in [(Opens::Everything, Memory::default()), (Opens::Page, Memory::default()), (Opens::Pinned, Memory::default()), (Opens::Pinned, memory)] {
            let lines = lines_of(&entries, "", None, opens, &memory);
            let shown: Vec<&str> = lines.iter().filter_map(|line| if let Line::Row(at) = line { Some(entries[*at].id.as_str()) } else { None }).collect();
            assert!(!shown.contains(&ended.as_str()) && !shown.contains(&quiet.as_str()), "{opens:?}: {shown:?}");
        }
    }

    fn titled(job: &SessionRecord, title: &str) -> SessionRecord {
        let mut job = job.clone();
        job.title = Some(title.into());
        job
    }

    #[test]
    fn a_quiet_job_ranks_among_the_live_matches_and_an_ended_one_after_them_all() {
        let all = jobs();
        let of = |state: &str| all.iter().find(|job| job.mark.is_some_and(|mark| format!("{:?}", mark.state).to_lowercase() == state)).expect("a Job in the fixture").clone();
        // The ended Job's name matches best and stands first; the quiet Job stands ahead of the live one.
        let entries: Vec<Entry> = [titled(&of("ended"), "zebra"), titled(&of("quiet"), "a zebra"), titled(&of("working"), "xzebra")].iter().filter_map(job_entry).collect();
        let order: Vec<&str> = lines_of(&entries, "zebra", None, Opens::Everything, &Memory::default())
            .into_iter()
            .filter_map(|line| if let Line::Row(at) = line { Some(entries[at].label.as_str()) } else { None })
            .map(|label| if label == "a zebra" { "quiet" } else if label == "zebra" { "ended" } else { "live" })
            .collect();
        assert_eq!(order, ["quiet", "live", "ended"]);
    }

    #[test]
    fn an_ended_job_reads_ended_in_its_hue_and_says_when_it_ended_and_a_quiet_one_says_no_time() {
        crate::settings::time::hold(r#"{"hour12":false,"seconds":false,"date":true}"#);
        let all = jobs();
        let ended = all.iter().find(|job| job.state == SessionState::Ended).expect("an ended Job");
        let row = job_entry(ended).expect("a row");
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.state, Some(("power-off", "Ended".to_string(), hue_of(JobState::Ended))));
        let at = chrono::DateTime::parse_from_rfc3339(ended.ended_at.as_ref().expect("an end").as_str()).expect("an instant").with_timezone(&chrono::Local);
        assert_eq!(meta.when, Some((at.format("%b %-d %H:%M").to_string(), "Ended")));
        let quiet = all.iter().find(|job| job.mark.is_some_and(|mark| mark.state == JobState::Quiet)).expect("a quiet Job");
        let row = job_entry(quiet).expect("a row");
        let meta = row.meta.as_ref().expect("meta");
        assert_eq!(meta.state.as_ref().map(|state| (state.0, state.1.as_str())), Some(("unplug", "Not reachable")));
        assert_eq!(meta.when, None);
        crate::settings::time::hold("");
    }

    #[test]
    fn a_paused_job_reads_paused() {
        let entries = all();
        let row = entries.iter().find(|entry| entry.label == "Pocket sign-in on the phone").expect("listed");
        assert_eq!(row.meta.as_ref().and_then(|meta| meta.state.as_ref()).map(|state| state.1.as_str()), Some("Paused"));
    }

    #[test]
    fn a_job_that_holds_a_pull_request_has_the_two_pull_acts_and_one_that_does_not_has_neither() {
        assert_eq!(ids(job_acts(false, false, false, true, &[])), ["open", "pause", "snooze", "fork", "open-pull", "merge-pull", "copy-id", "pin", "stop"]);
        assert_eq!(ids(job_acts(false, false, false, false, &[])), ["open", "pause", "snooze", "fork", "copy-id", "pin", "stop"]);
    }

    #[test]
    fn snooze_reads_wake_now_when_snoozed_and_keeps_its_chord() {
        let label = |snoozed: bool| job_acts(false, snoozed, false, false, &[]).into_iter().find(|act| matches!(act.id, "snooze" | "wake")).map(|act| (act.id, act.label, act.chord));
        assert_eq!(label(false), Some(("snooze", "Snooze".to_string(), Some("\u{2325}\u{2318}Z"))));
        assert_eq!(label(true), Some(("wake", "Wake now".to_string(), Some("\u{2325}\u{2318}Z"))));
    }

    #[test]
    fn pause_reads_resume_when_paused_and_pin_reads_unpin_when_pinned() {
        let acts = job_acts(true, false, true, false, &[]);
        let label = |id: &str| acts.iter().find(|act| act.id == id).map(|act| act.label.clone());
        assert_eq!(label("pause").as_deref(), Some("Resume"));
        assert_eq!(label("pin").as_deref(), Some("Unpin"));
    }

    #[test]
    fn an_act_nothing_answers_for_the_job_is_left_out() {
        let acts = job_acts(false, false, false, true, &["fork", "merge-pull"]);
        assert!(!acts.iter().any(|act| act.id == "fork" || act.id == "merge-pull"));
        assert!(acts.iter().any(|act| act.id == "open-pull"));
    }

    #[test]
    fn a_merged_pull_request_leaves_out_merge_but_keeps_open_and_the_number_finds_the_job() {
        let mut job = jobs().into_iter().next().expect("a Job");
        job.attachments.retain(|one| one.kind != "pr");
        assert!(held_pull(&job).is_none() && without_of(&job).is_empty());
        let mut held = job.attachments.first().cloned().expect("an attachment to copy");
        held.kind = "pr".into();
        held.target = "#412".into();
        held.manifest_id = job.manifest_id.clone();
        held.state = AttachmentState::Standing;
        job.attachments.push(held);
        assert!(without_of(&job).is_empty() && pull_of(&job).is_some());
        job.attachments.last_mut().expect("the pull").state = AttachmentState::Spent;
        assert_eq!(without_of(&job), ["merge-pull"]);
        assert!(pull_of(&job).is_none() && held_pull(&job).is_some_and(|(_, number, open)| number == 412 && !open));
        let acts = ids(job_acts(false, false, false, true, &without_of(&job)));
        assert!(acts.contains(&"open-pull") && !acts.contains(&"merge-pull"));
    }

    #[test]
    fn the_job_acts_keep_their_chords_their_rules_and_what_stays_up() {
        let acts = job_acts(false, false, false, true, &[]);
        let at = |id: &str| acts.iter().find(|act| act.id == id).expect(id).clone();
        assert_eq!(at("pause").chord, Some("\u{2318}P"));
        assert_eq!(at("fork").chord, Some("\u{2325}\u{2318}F"));
        assert!(at("open-pull").apart && at("copy-id").apart && at("pin").apart && at("stop").apart && !at("merge-pull").apart);
        assert!(at("pin").stays && !at("stop").stays);
        assert!(at("stop").destructive);
        for act in &acts {
            if let Some(chord) = act.chord {
                assert!(chord == "\u{21b5}" || parse_chord(chord).is_some(), "{} reads", act.id);
            }
        }
    }

    #[test]
    fn a_destructive_row_has_no_hotkey_and_a_set_alias_comes_before_a_remove() {
        assert_eq!(ids(memory_acts(true, true, false, None)), ["set-alias", "remove-alias", "record-hotkey", "remove-hotkey"]);
        assert_eq!(ids(memory_acts(false, false, true, None)), ["set-alias"]);
        assert_eq!(ids(memory_acts(false, false, false, Some(false))), ["pin", "set-alias", "record-hotkey"]);
    }

    #[test]
    fn a_chord_is_the_keystroke_it_spells() {
        let press = |text: &'static str| Keystroke::parse(text).expect("parses");
        assert!(chord_is("\u{2318}P", &press("cmd-p")));
        assert!(chord_is("\u{21e7}\u{2318}M", &press("shift-cmd-m")));
        assert!(chord_is("\u{2318}Backspace", &press("cmd-backspace")));
        assert!(!chord_is("\u{2318}P", &press("p")));
        assert!(!chord_is("\u{2318}P", &press("shift-cmd-p")));
        assert_eq!(spelled(&press("alt-cmd-f")).as_deref(), Some("\u{2325}\u{2318}F"));
        assert_eq!(spelled(&press("f")), None);
        assert!(chord_is(&spelled(&press("shift-cmd-backspace")).expect("a chord"), &press("shift-cmd-backspace")));
    }

    #[test]
    fn typing_finds_by_title_and_by_the_number_of_the_pull_request() {
        let entries = all();
        let found = |query: &str| {
            lines_of(&entries, query, None, Opens::Everything, &Memory::default())
                .into_iter()
                .filter_map(|line| if let Line::Row(at) = line { Some(entries[at].label.clone()) } else { None })
                .collect::<Vec<_>>()
        };
        assert!(found("freshness").contains(&"Fix the merge line freshness chip".to_string()));
        assert!(found("#3388").contains(&"Fix the merge line freshness chip".to_string()));
        assert!(found("sample-web").contains(&"Upgrade the router".to_string()));
        assert!(found("zzzz").is_empty());
    }

    #[test]
    fn the_registered_acts_split_into_the_focused_block_and_navigation() {
        let with_job = registered(true);
        assert!(with_job.iter().any(|entry| entry.section == Section::Context && entry.id == "act:thread_yes"));
        assert!(with_job.iter().any(|entry| entry.section == Section::Navigation && entry.id == "act:message_job"));
        assert!(!with_job.iter().any(|entry| entry.id == "act:command_palette"));
        assert!(!registered(false).iter().any(|entry| entry.id == "act:thread_yes"));
    }

    #[test]
    fn what_is_kept_moves_a_used_row_to_the_front_holds_twelve_and_reads_back() {
        let mut memory = Memory::default();
        for at in 0..15 {
            memory.used(&format!("a{at}"));
        }
        memory.used("a3");
        assert_eq!(memory.recent[0], "a3");
        assert_eq!(memory.recent.len(), 12);
        memory.toggle_pin("a3");
        memory.toggle_pin("a4");
        memory.toggle_pin("a3");
        assert_eq!(memory.pinned, ["a4"]);
        memory.aliases.insert("session:mock-01".into(), "chip".into());
        memory.hotkeys.insert("session:mock-01".into(), "\u{2325}\u{2318}1".into());
        memory.opens = Opens::Everything;
        assert_eq!(Memory::parse(&memory.text()), memory);
    }

    #[gpui::test]
    fn the_fold_commands_are_offered_where_a_job_has_sub_jobs_and_root_jobs_only_reads_on(cx: &mut gpui::TestAppContext) {
        let tree = crate::mock::jobs("jobs-tree").expect("the fixture decodes");
        let list = cx.new(|cx| JobsList::mock(&tree, cx));
        let labels = |list: &JobsList| fold_commands(list).iter().map(|entry| (entry.label.clone(), entry.meta.is_some(), entry.kind())).collect::<Vec<_>>();
        let said = |label: &str, on: bool| (label.to_string(), on, "Command");
        list.read_with(cx, |list, _| {
            assert_eq!(labels(list), [said("Fold all sub-Jobs", false), said("Unfold all sub-Jobs", false), said("Root Jobs only", false)]);
        });
        list.update(cx, |list, cx| list.fold_command("root-only", cx));
        list.read_with(cx, |list, _| assert_eq!(labels(list)[2], said("Root Jobs only", true)));
        // A list with no sub-Jobs offers none of them.
        let flat: Vec<SessionRecord> = tree.into_iter().map(|mut job| { job.attachments.retain(|one| one.kind != "started_by"); job }).collect();
        let flat = cx.new(|cx| JobsList::mock(&flat, cx));
        flat.read_with(cx, |list, _| assert!(fold_commands(list).is_empty()));
        // Nothing typed, they stay out of the list the palette opens on; a word finds them.
        let entries = list.read_with(cx, |list, _| fold_commands(list));
        let rows = |query: &str| lines_of(&entries, query, None, Opens::Everything, &Memory::default()).iter().filter(|line| matches!(line, Line::Row(_))).count();
        assert_eq!((rows(""), rows("fold") > 0, rows("root")), (0, true, 1));
    }
}
