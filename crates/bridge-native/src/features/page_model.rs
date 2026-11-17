//! What a feature's page draws, worked out from the page and the asks Fleet read: the sections down the
//! side with the line under each, and what the open one holds. Nothing here touches the screen or the wire.

use chrono::{DateTime, Local, Utc};
use ipc::{Ask, AskKind, AskState, FeatureLink, FeatureLinkKind, FeaturePage, FeatureSectionKind, Instant};

/// The sections, in the order the catalog gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl Section {
    pub const ALL: [Section; 10] = [Section::Who, Section::Code, Section::Test, Section::Designs, Section::Bugs, Section::Changes, Section::Ideas, Section::Runs, Section::Observe, Section::Debt];

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
    Muted,
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

/// What the open section holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Rows(Vec<Row>),
    Code(Code),
    /// None where nobody wrote the section.
    Written(Option<Written>),
}

// ---- time ------------------------------------------------------------------------------------

pub fn when(at: &Instant) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Utc))
}

/// `15 Nov`.
pub fn day(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%-d %b").to_string()
}

fn instant(at: &Instant) -> DateTime<Utc> {
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
fn sentence(word: &str) -> String {
    let mut chars = word.trim().chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn dated(words: &str, at: DateTime<Utc>) -> String {
    if words.is_empty() { day(at) } else { format!("{words}, {}", day(at)) }
}

fn ask_row(ask: &Ask) -> Row {
    let (glyph, hue, said) = match ask.state {
        AskState::Building => ("circle-dot", Hue::Running, "Open"),
        AskState::Handed => ("circle", Hue::Handed, "Handed"),
        AskState::Backlog => ("circle-dashed", Hue::Muted, "Backlog"),
        AskState::Built => ("circle-check", Hue::Done, "Fixed"),
        AskState::Dropped => ("circle-x", Hue::Muted, "Dropped"),
    };
    Row { key: ask.ask_id.clone(), glyph, hue, said: said.into(), title: ask.title.clone(), meta: dated(said, instant(&ask.state_at)), on: false }
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
    Row { key: link.target.clone(), glyph, hue, said: if said.is_empty() { "Open".into() } else { said.clone() }, title: titled(link), meta: dated(&said, instant(&link.at)), on: false }
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
    Row { key: link.target.clone(), glyph: "circle", hue, said: if said.is_empty() { "Job".into() } else { said.clone() }, title: titled(link), meta: dated(&said, instant(&link.at)), on }
}

/// One Job a row: the ones on the feature now first, then the newest.
fn who(page: &FeaturePage) -> Vec<Row> {
    let now: Vec<&str> = page.feature.jobs_now.iter().map(|job| job.session_id.as_str()).collect();
    let mut rows: Vec<Row> = page.feature.jobs_now.iter().map(|job| Row { key: job.session_id.as_str().to_string(), glyph: "circle", hue: Hue::Running, said: "On it now".into(), title: job.title.clone(), meta: String::new(), on: true }).collect();
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

fn designs(page: &FeaturePage) -> Vec<Row> {
    links_of(page, &[FeatureLinkKind::Sketch, FeatureLinkKind::Page, FeatureLinkKind::Walk, FeatureLinkKind::Draft])
        .into_iter()
        .map(|link| {
            let (glyph, said) = glyph_of(link.kind);
            Row { key: link.target.clone(), glyph, hue: Hue::Muted, said: said.into(), title: titled(link), meta: when(&link.at).map(day).unwrap_or_default(), on: false }
        })
        .collect()
}

/// The asks of the section's kinds, and for bugs the ones a Job filed as links, newest first, as the chip keeps them.
fn asked(section: Section, page: &FeaturePage, asks: &[Ask], chip: Chip, looked: Option<DateTime<Utc>>) -> Vec<Row> {
    let mut found: Vec<(DateTime<Utc>, Row)> = Vec::new();
    for ask in asks.iter().filter(|ask| section.kinds().contains(&ask.kind)) {
        let touched = instant(&ask.state_at).max(instant(&ask.opened_at));
        if chip.keeps(Standing::of(ask.state), touched, looked) {
            found.push((instant(&ask.opened_at), ask_row(ask)));
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
pub fn body(section: Section, page: &FeaturePage, asks: Option<&[Ask]>, chip: Chip, looked: Option<DateTime<Utc>>) -> Body {
    match section {
        Section::Who => Body::Rows(who(page)),
        Section::Designs => Body::Rows(designs(page)),
        Section::Bugs | Section::Changes | Section::Ideas => Body::Rows(asked(section, page, asks.unwrap_or_default(), chip, looked)),
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
