//! What a feature's page draws, worked out from the page and the asks Fleet read: the sections down the
//! side with the line under each, and what the open one holds. Nothing here touches the screen or the wire.

use std::collections::HashMap;

use chrono::{DateTime, Local, Utc};
use ipc::{Ask, AskKind, AskState, FeatureLink, FeatureLinkKind, FeaturePage, FeatureSectionKind, HealthMark, Instant, SessionId};

/// The titles of the Jobs Fleet holds, by id: what an ask's row names its Jobs by.
pub type Titles = HashMap<String, String>;

/// The sections, in the order the catalog gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Who,
    Code,
    Test,
    Designs,
    Bugs,
    Changes,
    Ideas,
    Runs,
    Observe,
    Debt,
    /// What the owner settled in the Jobs that shaped the feature.
    Decisions,
    /// The documents a Job read before it changed a file of the feature.
    Docs,
    /// The tickets a Job read before it changed a file of the feature.
    Tickets,
}

impl Section {
    pub const ALL: [Section; 13] = [
        Section::Who,
        Section::Code,
        Section::Test,
        Section::Designs,
        Section::Bugs,
        Section::Changes,
        Section::Ideas,
        Section::Runs,
        Section::Observe,
        Section::Debt,
        Section::Decisions,
        Section::Docs,
        Section::Tickets,
    ];

    /// The order the keys go through them: down the page's columns, as the cards stand.
    pub const NAV: [Section; 13] = [
        Section::Who,
        Section::Bugs,
        Section::Changes,
        Section::Ideas,
        Section::Decisions,
        Section::Test,
        Section::Runs,
        Section::Designs,
        Section::Docs,
        Section::Code,
        Section::Tickets,
        Section::Observe,
        Section::Debt,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Who => "Who worked on it",
            Section::Code => "Code",
            Section::Test => "How to test it",
            Section::Designs => "Designs",
            Section::Bugs => "Bugs and issues",
            Section::Changes => "Change requests",
            Section::Ideas => "Ideas",
            Section::Runs => "How it runs",
            Section::Observe => "How to observe it",
            Section::Debt => "Tech debt",
            Section::Decisions => "Decisions",
            Section::Docs => "Docs",
            Section::Tickets => "Tickets",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Section::Who => "who",
            Section::Code => "code",
            Section::Test => "test",
            Section::Designs => "designs",
            Section::Bugs => "bugs",
            Section::Changes => "changes",
            Section::Ideas => "ideas",
            Section::Runs => "runs",
            Section::Observe => "observe",
            Section::Debt => "debt",
            Section::Decisions => "decisions",
            Section::Docs => "docs",
            Section::Tickets => "tickets",
        }
    }

    /// Whether the section is a list of asks, which is where the filter chips are.
    pub fn is_asks(self) -> bool {
        matches!(self, Section::Bugs | Section::Changes | Section::Ideas)
    }

    /// The written section a Job keeps under this one, where it is one.
    fn written(self) -> Option<FeatureSectionKind> {
        match self {
            Section::Test => Some(FeatureSectionKind::Test),
            Section::Runs => Some(FeatureSectionKind::Runs),
            Section::Observe => Some(FeatureSectionKind::Observe),
            Section::Debt => Some(FeatureSectionKind::Debt),
            _ => None,
        }
    }

    /// The kinds of ask the section lists.
    fn kinds(self) -> &'static [AskKind] {
        match self {
            Section::Bugs => &[AskKind::Bug],
            Section::Changes => &[AskKind::ChangeRequest, AskKind::Fix],
            Section::Ideas => &[AskKind::Idea],
            _ => &[],
        }
    }
}

/// The chips above a list of asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    All,
    Open,
    Backlog,
    Fixed,
    Since,
}

impl Chip {
    pub const ALL: [Chip; 5] = [Chip::All, Chip::Open, Chip::Backlog, Chip::Fixed, Chip::Since];

    pub fn label(self) -> &'static str {
        match self {
            Chip::All => "All",
            Chip::Open => "Open",
            Chip::Backlog => "Backlog",
            Chip::Fixed => "Fixed",
            Chip::Since => "Since my last look",
        }
    }

    /// Whether the chip keeps an ask in `standing`, last touched at `touched`, given the time the feature was last looked at.
    /// `Dropped` is under All and Since alone. A feature never looked at has no time, so Since keeps every ask.
    pub fn keeps(self, standing: Standing, touched: DateTime<Utc>, looked: Option<DateTime<Utc>>) -> bool {
        match self {
            Chip::All => true,
            Chip::Open => standing == Standing::Open,
            Chip::Backlog => standing == Standing::Backlog,
            Chip::Fixed => standing == Standing::Fixed,
            Chip::Since => looked.is_none_or(|looked| touched > looked),
        }
    }
}

/// Where an ask stands, as the chips group it: `building` and `handed` are Open, `built` is Fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Open,
    Backlog,
    Fixed,
    Dropped,
}

impl Standing {
    pub fn of(state: AskState) -> Standing {
        match state {
            AskState::Building | AskState::Handed => Standing::Open,
            AskState::Backlog => Standing::Backlog,
            AskState::Built => Standing::Fixed,
            AskState::Dropped => Standing::Dropped,
        }
    }

    /// What a bug a Job filed says of itself, in the words a link carries its state in. A word it does not know is Open.
    fn of_link(state: &str) -> Standing {
        match state.trim().to_lowercase().as_str() {
            "fixed" | "built" | "closed" | "done" | "merged" | "resolved" => Standing::Fixed,
            "backlog" => Standing::Backlog,
            "dropped" | "wontfix" => Standing::Dropped,
            _ => Standing::Open,
        }
    }
}

/// The colour a row's glyph wears, by the hue it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hue {
    Running,
    Done,
    Handed,
    Bad,
    Warn,
    /// An ask in the backlog.
    NotStarted,
    Muted,
}

/// What an ask's row says beyond a basic row: his words, where he said them, where it stands, and the Job its Story opens.
#[derive(Clone, Debug, PartialEq)]
pub struct AskLine {
    pub ask_id: String,
    /// The first line: his words in quotes, or the ask's title where they run long.
    pub words: String,
    /// The second line is `before`, the Job's title as a link where Fleet holds one, and `after`.
    pub before: String,
    pub job: Option<String>,
    pub after: String,
    /// The Job a press on Story opens. Absent for an ask with no Job at all.
    pub story: Option<SessionId>,
    /// Where the ask stands, in a sentence, and the day it was said: the line under its title on the page.
    pub state_said: String,
    pub date: String,
}

/// One row of the open section: a Job, a design, an ask.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// What names it: an ask's id, a link's target.
    pub key: String,
    /// The registry glyph, and what its tooltip says.
    pub glyph: &'static str,
    pub hue: Hue,
    pub said: String,
    pub title: String,
    pub meta: String,
    /// A Job on the feature now, which draws the chip.
    pub on: bool,
    /// Set for an ask, which draws its own line and a Story link.
    pub ask: Option<AskLine>,
}

/// The line under a section's title in the rail.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub section: Section,
    pub summary: String,
    /// A written section is stale: the line is drawn in the awaiting review hue.
    pub stale: bool,
}

/// A written section as the pane shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub blocks: Vec<Block>,
    pub by: String,
    pub stale: Option<String>,
}

/// A run of lines of a written section: commands are drawn in monospace, the rest as a paragraph.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub text: String,
    pub command: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Code {
    pub paths: Vec<String>,
    pub entry_points: Vec<String>,
    /// Each file and the Jobs that wrote it.
    pub files: Vec<(String, u32)>,
}

/// A document or ticket a Job read: its row in the Docs and Tickets cards.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadRow {
    /// The registry glyph: a file for a path, a globe for an address, a ticket for a ticket.
    pub glyph: &'static str,
    pub title: String,
    /// What it was read by: the path, address or id.
    pub target: String,
    /// The result's first sentence, or the Job's own words where `by_job`.
    pub gist: String,
    pub by_job: bool,
    /// The names it gives that the code no longer has.
    pub drifted: Vec<String>,
    /// The address no longer answers.
    pub gone: bool,
    /// The last line: the Job that read it and the day.
    pub last: String,
}

/// One thing he settled, as the Decisions card draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionRow {
    pub question: String,
    pub choice: String,
    pub note: Option<String>,
    pub last: String,
}

/// What the open section holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Rows(Vec<Row>),
    Reads(Vec<ReadRow>),
    Decisions(Vec<DecisionRow>),
    Code(Code),
    /// None where nobody wrote the section.
    Written(Option<Written>),
}

// ---- the floor plan --------------------------------------------------------------------------

/// What the floor plan card holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Floor {
    /// The plan read, laid out by the card at the width it is drawn at.
    Plan(std::rc::Rc<super::floor_map::Laid>),
    /// No rooms to draw: the line that says why.
    Empty(&'static str),
}

/// Why the card is empty, and what fills it.
pub const NO_PLAN: &str = "No code map. It draws when the full page is read from a repository Fleet can open.";
pub const NO_ROOMS: &str = "No modules found. They draw from the repository's package manifests and folders.";

/// The code map with the feature's rooms lit: every module, lit or not.
pub fn floor(page: &FeaturePage) -> Floor {
    let Some(plan) = &page.floor_plan else { return Floor::Empty(NO_PLAN) };
    if plan.rooms.is_empty() {
        return Floor::Empty(NO_ROOMS);
    }
    Floor::Plan(std::rc::Rc::new(super::floor_map::Laid::new(plan.clone())))
}

// ---- the fold --------------------------------------------------------------------------------

/// How many of an ask section's rows show before the rest fold under one line.
pub const SHOWN: usize = 5;

/// What a place of the list holds: a row, or the line that folds the older ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Row(usize),
    Fold,
}

/// The places of a list of `count` rows. An ask section past [`SHOWN`] rows has the fold line after the fifth, and the
/// rest after it only while `unfolded`.
pub fn items(section: Section, count: usize, unfolded: bool) -> Vec<Item> {
    if !section.is_asks() || count <= SHOWN {
        return (0..count).map(Item::Row).collect();
    }
    let shown = if unfolded { count } else { SHOWN };
    let mut items: Vec<Item> = (0..SHOWN).map(Item::Row).collect();
    items.push(Item::Fold);
    items.extend((SHOWN..shown).map(Item::Row));
    items
}

// ---- time ------------------------------------------------------------------------------------

pub fn when(at: &Instant) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Utc))
}

/// `15 Nov`.
pub fn day(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%-d %b").to_string()
}

pub(crate) fn instant(at: &Instant) -> DateTime<Utc> {
    when(at).unwrap_or(DateTime::<Utc>::MIN_UTC)
}

// ---- the top strip ---------------------------------------------------------------------------

/// `Current as of #3720, 15 Nov`: the newest merge the feature is linked to. Empty where it has none.
pub fn current_as_of(page: &FeaturePage) -> Option<String> {
    let newest = page.links.iter().filter(|link| link.kind == FeatureLinkKind::Merge).max_by_key(|link| instant(&link.at))?;
    let number = newest.target.trim().trim_start_matches('#');
    let date = when(&newest.at).map(day);
    Some(match date {
        Some(date) => format!("Current as of #{number}, {date}"),
        None => format!("Current as of #{number}"),
    })
}

// ---- the rail --------------------------------------------------------------------------------

/// The first line of a written section, with the marks round a command taken off.
fn first_line(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
    line.trim_matches('`').trim().to_string()
}

fn basename(path: &str) -> &str {
    let path = path.split(':').next().unwrap_or(path);
    path.rsplit('/').next().unwrap_or(path)
}

fn links_of<'a>(page: &'a FeaturePage, kinds: &[FeatureLinkKind]) -> Vec<&'a FeatureLink> {
    let mut links: Vec<&FeatureLink> = page.links.iter().filter(|link| kinds.contains(&link.kind)).collect();
    links.sort_by_key(|link| std::cmp::Reverse(instant(&link.at)));
    links
}

fn newest_ask<'a>(asks: &'a [Ask], kinds: &[AskKind], keep: impl Fn(AskState) -> bool) -> Option<&'a Ask> {
    asks.iter().filter(|ask| kinds.contains(&ask.kind) && keep(ask.state)).max_by_key(|ask| instant(&ask.opened_at))
}

fn titled(link: &FeatureLink) -> String {
    if link.title.is_empty() { link.target.clone() } else { link.title.clone() }
}

fn summary(section: Section, page: &FeaturePage, asks: Option<&[Ask]>) -> (String, bool) {
    let plain = |text: String| (text, false);
    match section {
        Section::Who => {
            if let Some(now) = page.feature.jobs_now.first() {
                return plain(format!("Now: {}", now.title));
            }
            match links_of(page, &[FeatureLinkKind::Job]).first() {
                Some(job) => plain(match when(&job.at) {
                    Some(at) => format!("Last: {}, {}", titled(job), day(at)),
                    None => format!("Last: {}", titled(job)),
                }),
                None => plain(String::new()),
            }
        }
        Section::Code => {
            let mut names: Vec<&str> = page.files.iter().map(|file| basename(&file.path)).collect();
            if names.is_empty() {
                names = page.entry_points.iter().map(|point| basename(point)).collect();
            }
            plain(names.into_iter().filter(|name| !name.is_empty()).take(2).collect::<Vec<_>>().join(", "))
        }
        Section::Test => plain(written(page, FeatureSectionKind::Test).map(|section| first_line(&section.text)).unwrap_or_default()),
        Section::Designs => plain(links_of(page, &[FeatureLinkKind::Sketch, FeatureLinkKind::Page, FeatureLinkKind::Walk, FeatureLinkKind::Draft]).first().map(|link| titled(link)).unwrap_or_default()),
        Section::Bugs => plain(asks.and_then(|asks| newest_ask(asks, Section::Bugs.kinds(), |state| Standing::of(state) == Standing::Open)).map(|ask| format!("Open: {}", ask.title)).unwrap_or_default()),
        Section::Changes => {
            let asks = asks.unwrap_or_default();
            let kinds = Section::Changes.kinds();
            plain(if let Some(ask) = newest_ask(asks, kinds, |state| state == AskState::Building) {
                format!("Being built: {}", ask.title)
            } else if let Some(ask) = newest_ask(asks, kinds, |state| Standing::of(state) == Standing::Open) {
                format!("Open: {}", ask.title)
            } else {
                String::new()
            })
        }
        Section::Ideas => plain(asks.and_then(|asks| newest_ask(asks, Section::Ideas.kinds(), |state| state != AskState::Dropped)).map(|ask| ask.title.clone()).unwrap_or_default()),
        Section::Docs => plain(page.docs.iter().max_by_key(|read| instant(&read.at)).map(|read| read.title.clone()).unwrap_or_default()),
        Section::Tickets => plain(page.tickets.iter().max_by_key(|read| instant(&read.at)).map(|read| read.title.clone()).unwrap_or_default()),
        Section::Decisions => plain(page.decisions.iter().max_by_key(|decision| instant(&decision.at)).map(|decision| decision.question.clone()).unwrap_or_default()),
        Section::Runs | Section::Observe | Section::Debt => {
            let Some(section) = section.written().and_then(|kind| written(page, kind)) else { return plain(String::new()) };
            if let Some(since) = section.stale_since {
                return (format!("Stale since #{since}"), true);
            }
            let line = first_line(&section.text);
            if !line.is_empty() {
                return plain(line);
            }
            plain(when(&section.written_at).map(|at| format!("Written {}", day(at))).unwrap_or_default())
        }
    }
}

fn written(page: &FeaturePage, kind: FeatureSectionKind) -> Option<&ipc::FeatureSection> {
    page.sections.iter().find(|section| section.section == kind)
}

/// The rail: each section with its line. The ask sections stay empty until the asks are read.
pub fn entries(page: &FeaturePage, asks: Option<&[Ask]>) -> Vec<Entry> {
    Section::ALL
        .into_iter()
        .map(|section| {
            let (summary, stale) = summary(section, page, asks);
            Entry { section, summary, stale }
        })
        .collect()
}

// ---- the open section ------------------------------------------------------------------------

/// What a line of a written section is: a command where it starts like one a shell takes.
fn is_command(line: &str) -> bool {
    let line = line.trim();
    if line.len() > 2 && line.starts_with('`') && line.ends_with('`') {
        return true;
    }
    line.starts_with("$ ") || line.starts_with("./") || line.starts_with("scripts/") || ["armada ", "cargo ", "pnpm ", "npm ", "npx ", "git ", "just ", "curl "].iter().any(|start| line.starts_with(start))
}

/// The text as blocks: a run of commands is one block in monospace, lines between blank ones are a paragraph.
fn blocks(text: &str, commands: bool) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut open: Option<bool> = None;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            open = None;
            continue;
        }
        let command = commands && is_command(line);
        let text = if command { line.trim_start_matches("$ ").trim_matches('`').to_string() } else { line.to_string() };
        match (open, out.last_mut()) {
            (Some(was), Some(last)) if was == command => {
                last.text.push(if command { '\n' } else { ' ' });
                last.text.push_str(&text);
            }
            _ => out.push(Block { text, command }),
        }
        open = Some(command);
    }
    out
}

fn glyph_of(kind: FeatureLinkKind) -> (&'static str, &'static str) {
    match kind {
        FeatureLinkKind::Sketch => ("pencil-ruler", "Sketch"),
        FeatureLinkKind::Page => ("file-text", "Document"),
        FeatureLinkKind::Walk => ("footprints", "Walk"),
        _ => ("file-pen-line", "Draft"),
    }
}

/// A word with its first letter in capitals.
pub(crate) fn sentence(word: &str) -> String {
    let mut chars = word.trim().chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn dated(words: &str, at: DateTime<Utc>) -> String {
    if words.is_empty() { day(at) } else { format!("{words}, {}", day(at)) }
}

/// His words as the first line: in quotes while they fit on a line, the ask's title where they run long.
const LONG_WORDS: usize = 80;

fn ask_words(ask: &Ask) -> String {
    let words = ask.first_words.trim();
    if words.is_empty() || words.chars().count() > LONG_WORDS || words.contains('\n') {
        ask.title.clone()
    } else {
        format!("\"{words}\"")
    }
}

pub(crate) fn job_title(titles: &Titles, id: &SessionId) -> Option<String> {
    titles.get(id.as_str()).filter(|title| !title.is_empty()).cloned()
}

pub(crate) fn has_job(id: &SessionId) -> bool {
    !id.as_str().is_empty()
}

/// Where the ask stands, in a sentence. A Job Fleet holds no title for is not named, and never by its id.
fn state_sentence(ask: &Ask, titles: &Titles) -> String {
    let building = ask.building.as_ref().and_then(|id| job_title(titles, id));
    let named = |word: &str, alone: &str| building.as_ref().map_or_else(|| alone.to_string(), |title| format!("{word} {title}"));
    match ask.state {
        AskState::Handed => named("Handed to", "Handed"),
        AskState::Built => named("Fixed by", "Fixed"),
        AskState::Building if ask.building.as_ref().is_some_and(|id| *id != ask.said_in) => named("Building in", "Building"),
        AskState::Building => "Building".into(),
        AskState::Backlog => "Backlog".into(),
        AskState::Dropped => "Dropped".into(),
    }
}

/// The Job a press on the ask's Story opens: the one it was said in, and the one building it only where it was said in none.
pub fn story_of(ask: &Ask) -> Option<SessionId> {
    if has_job(&ask.said_in) {
        Some(ask.said_in.clone())
    } else {
        ask.building.clone().filter(has_job)
    }
}

fn ask_row(ask: &Ask, titles: &Titles) -> Row {
    let (glyph, hue, said) = match ask.state {
        AskState::Building => ("circle-dot", Hue::Running, "Open"),
        AskState::Handed => ("circle", Hue::Handed, "Handed"),
        AskState::Backlog => ("circle-dashed", Hue::NotStarted, "Backlog"),
        AskState::Built => ("circle-check", Hue::Done, "Fixed"),
        AskState::Dropped => ("circle-x", Hue::Muted, "Dropped"),
    };
    let said_in = has_job(&ask.said_in).then(|| job_title(titles, &ask.said_in)).flatten();
    let date = day(instant(&ask.opened_at));
    let before = if !has_job(&ask.said_in) {
        format!("You, {date}, to the Operator")
    } else if said_in.is_some() {
        format!("You, {date}, in ")
    } else {
        format!("You, {date}")
    };
    let state_said = state_sentence(ask, titles);
    let after = format!(". {state_said}");
    let meta = format!("{before}{}{after}", said_in.as_deref().unwrap_or_default());
    let line = AskLine { ask_id: ask.ask_id.clone(), words: ask_words(ask), before, job: said_in, after, story: story_of(ask), state_said, date };
    Row { key: ask.ask_id.clone(), glyph, hue, said: said.into(), title: ask.title.clone(), meta, on: false, ask: Some(line) }
}

fn bug_row(link: &FeatureLink) -> Row {
    let standing = Standing::of_link(&link.state);
    let (glyph, hue) = match standing {
        Standing::Open => ("circle", Hue::Running),
        Standing::Backlog => ("circle-dashed", Hue::Muted),
        Standing::Fixed => ("circle-check", Hue::Done),
        Standing::Dropped => ("circle-x", Hue::Muted),
    };
    let said = sentence(&link.state);
    Row { key: link.target.clone(), glyph, hue, said: if said.is_empty() { "Open".into() } else { said.clone() }, title: titled(link), meta: dated(&said, instant(&link.at)), on: false, ask: None }
}

fn job_row(link: &FeatureLink, on: bool) -> Row {
    let state = link.state.trim().to_lowercase();
    let hue = if on || matches!(state.as_str(), "running" | "working") {
        Hue::Running
    } else if matches!(state.as_str(), "done" | "merged" | "complete" | "completed" | "ended") {
        Hue::Done
    } else if state.contains("fail") {
        Hue::Bad
    } else {
        Hue::Muted
    };
    let said = sentence(&link.state);
    Row { key: link.target.clone(), glyph: "circle", hue, said: if said.is_empty() { "Job".into() } else { said.clone() }, title: titled(link), meta: dated(&said, instant(&link.at)), on, ask: None }
}

/// One Job a row: the ones on the feature now first, then the newest.
fn who(page: &FeaturePage) -> Vec<Row> {
    let now: Vec<&str> = page.feature.jobs_now.iter().map(|job| job.session_id.as_str()).collect();
    let mut rows: Vec<Row> = page.feature.jobs_now.iter().map(|job| Row { key: job.session_id.as_str().to_string(), glyph: "circle", hue: Hue::Running, said: "On it now".into(), title: job.title.clone(), meta: String::new(), on: true, ask: None }).collect();
    for link in links_of(page, &[FeatureLinkKind::Job]) {
        let on = now.contains(&link.target.as_str());
        let row = job_row(link, on);
        match rows.iter_mut().find(|held| held.key == link.target) {
            Some(held) => {
                // The Job is on it now and has a link: the link's state and date go on the row that stands first.
                held.meta = row.meta;
                if !link.title.is_empty() {
                    held.title = link.title.clone();
                }
            }
            None => rows.push(row),
        }
    }
    rows
}

/// The links the Designs card lists.
pub(crate) const DESIGN_KINDS: [FeatureLinkKind; 4] = [FeatureLinkKind::Sketch, FeatureLinkKind::Page, FeatureLinkKind::Walk, FeatureLinkKind::Draft];

fn designs(page: &FeaturePage) -> Vec<Row> {
    links_of(page, &DESIGN_KINDS)
        .into_iter()
        .map(|link| {
            let (glyph, said) = glyph_of(link.kind);
            Row { key: link.target.clone(), glyph, hue: Hue::Muted, said: said.into(), title: titled(link), meta: when(&link.at).map(day).unwrap_or_default(), on: false, ask: None }
        })
        .collect()
}

/// What a read was by, as the person would say it: the Job's title where Fleet holds one, never its id.
fn by_line(job: &str, at: &Instant, titles: &Titles) -> String {
    let title = titles.get(job).filter(|title| !title.is_empty());
    match (title, when(at).map(day)) {
        (Some(title), Some(date)) => format!("{title}, {date}"),
        (Some(title), None) => title.clone(),
        (None, Some(date)) => date,
        (None, None) => String::new(),
    }
}

/// Whether a read's target is an address on the web or an MCP server's, and not a path in the repository.
fn is_address(target: &str) -> bool {
    target.contains("://") || target.starts_with("mcp")
}

/// The docs or tickets a Job read, newest first.
fn reads(list: &[ipc::FeatureRead], ticket: bool, titles: &Titles) -> Vec<ReadRow> {
    let mut list: Vec<&ipc::FeatureRead> = list.iter().collect();
    list.sort_by_key(|read| std::cmp::Reverse(instant(&read.at)));
    list.into_iter()
        .map(|read| ReadRow {
            glyph: if ticket {
                "ticket"
            } else if is_address(&read.target) {
                "globe"
            } else {
                "file-text"
            },
            title: if read.title.is_empty() { read.target.clone() } else { read.title.clone() },
            target: read.target.clone(),
            gist: read.gist.clone(),
            by_job: read.gist_by_job && !read.gist.is_empty(),
            drifted: if ticket { Vec::new() } else { read.drifted.clone() },
            gone: read.state == "gone",
            last: by_line(&read.job, &read.at, titles),
        })
        .collect()
}

fn decisions(page: &FeaturePage, titles: &Titles) -> Vec<DecisionRow> {
    let mut list: Vec<&ipc::FeatureDecision> = page.decisions.iter().collect();
    list.sort_by_key(|decision| std::cmp::Reverse(instant(&decision.at)));
    list.into_iter()
        .map(|decision| DecisionRow {
            question: decision.question.clone(),
            choice: decision.choice.clone(),
            note: decision.note.as_deref().map(str::trim).filter(|note| !note.is_empty()).map(str::to_string),
            last: by_line(&decision.job, &decision.at, titles),
        })
        .collect()
}

// ---- the band's marks --------------------------------------------------------------------------

/// The chip a feature's health draws in the band: Watch or Poor, never Good, and the reasons behind it.
#[derive(Clone, Debug, PartialEq)]
pub struct Health {
    pub mark: HealthMark,
    pub word: &'static str,
    pub glyph: &'static str,
    /// The reasons' sentences, in Fleet's order.
    pub reasons: Vec<String>,
    /// The reasons on one line, and the chip's tooltip: what Fleet says, or what the mark means where it says nothing.
    pub line: String,
    pub tip: String,
}

/// The health chip, where the mark is not Good.
pub fn health(page: &FeaturePage) -> Option<Health> {
    let (word, glyph, means) = match page.feature.health.mark {
        HealthMark::Good => return None,
        HealthMark::Watch => ("Watch", "triangle-alert", "Something kept is worth a look"),
        HealthMark::Poor => ("Poor", "circle-alert", "A check failed, or flaky tests or open bugs piled up"),
    };
    let reasons: Vec<String> = page.feature.health.reasons.iter().map(|reason| reason.fact.trim().to_string()).filter(|fact| !fact.is_empty()).collect();
    let line = reasons.join(" · ");
    let tip = if line.is_empty() { means.to_string() } else { line.clone() };
    Some(Health { mark: page.feature.health.mark, word, glyph, reasons, line, tip })
}

/// What the Planned chip says on hover.
pub const PLANNED_TIP: &str = "No code yet. It becomes a normal feature at its first file.";

/// The line the retire proposal rests on, where the librarian proposes retiring the feature.
pub fn retire_evidence(page: &FeaturePage) -> Option<&str> {
    page.feature.retire.as_ref().map(|retire| retire.evidence.as_str())
}

/// The asks of the section's kinds, and for bugs the ones a Job filed as links, newest first, as the chip keeps them.
fn asked(section: Section, page: &FeaturePage, asks: &[Ask], chip: Chip, looked: Option<DateTime<Utc>>, titles: &Titles) -> Vec<Row> {
    let mut found: Vec<(DateTime<Utc>, Row)> = Vec::new();
    for ask in asks.iter().filter(|ask| section.kinds().contains(&ask.kind)) {
        let touched = instant(&ask.state_at).max(instant(&ask.opened_at));
        if chip.keeps(Standing::of(ask.state), touched, looked) {
            found.push((instant(&ask.opened_at), ask_row(ask, titles)));
        }
    }
    if section == Section::Bugs {
        for link in page.links.iter().filter(|link| link.kind == FeatureLinkKind::Bug) {
            let at = instant(&link.at);
            if chip.keeps(Standing::of_link(&link.state), at, looked) {
                found.push((at, bug_row(link)));
            }
        }
    }
    found.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    found.into_iter().map(|(_, row)| row).collect()
}

/// What the open section holds. `asks` is none while they are not read, which leaves an ask section with no row.
pub fn body(section: Section, page: &FeaturePage, asks: Option<&[Ask]>, chip: Chip, looked: Option<DateTime<Utc>>, titles: &Titles) -> Body {
    match section {
        Section::Who => Body::Rows(who(page)),
        Section::Designs => Body::Rows(designs(page)),
        Section::Bugs | Section::Changes | Section::Ideas => Body::Rows(asked(section, page, asks.unwrap_or_default(), chip, looked, titles)),
        Section::Docs => Body::Reads(reads(&page.docs, false, titles)),
        Section::Tickets => Body::Reads(reads(&page.tickets, true, titles)),
        Section::Decisions => Body::Decisions(decisions(page, titles)),
        Section::Code => Body::Code(Code { paths: page.paths.clone(), entry_points: page.entry_points.clone(), files: page.files.iter().map(|file| (file.path.clone(), file.jobs)).collect() }),
        Section::Test | Section::Runs | Section::Observe | Section::Debt => Body::Written(section.written().and_then(|kind| written(page, kind)).map(|text| {
            let when = when(&text.written_at).map(day);
            let by = match when {
                Some(date) => format!("Written by {}, {date}", text.job),
                None => format!("Written by {}", text.job),
            };
            Written { blocks: blocks(&text.text, section != Section::Debt), by, stale: text.stale_since.map(|since| format!("Stale since #{since}")) }
        })),
    }
}
