//! What the Usage page shows, worked out from Fleet's waste reading (`ipc::UsageWaste`) and nothing there touches the screen.
//! `Held` is what the page holds while a reading is asked for: the span on the switch, the rows last read, and why a read failed.

use std::collections::HashMap;

use ipc::{UsageWaste, WasteKind};

use crate::look::{token, Token};

/// The span a reading covers, shortest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    Day,
    Week,
    Fortnight,
}

impl Span {
    pub const ALL: [Span; 3] = [Span::Day, Span::Week, Span::Fortnight];

    /// The word on the switch.
    pub fn label(self) -> &'static str {
        match self {
            Span::Day => "Day",
            Span::Week => "Week",
            Span::Fortnight => "Fortnight",
        }
    }

    /// The word Fleet's route takes: `GET /usage/waste?window=`.
    pub fn id(self) -> &'static str {
        match self {
            Span::Day => "day",
            Span::Week => "week",
            Span::Fortnight => "fortnight",
        }
    }

    /// The key `usage_window`'s slot is on: Day, Week, Fortnight.
    pub fn at(slot: u8) -> Option<Span> {
        Span::ALL.get(slot as usize).copied()
    }
}

/// What a kind is drawn as: its registry glyph, its hue, and what one occurrence is called in its tooltip.
#[derive(Clone, Copy, Debug)]
pub struct KindLook {
    pub glyph: &'static str,
    pub hue: Token,
    pub one: &'static str,
    pub many: &'static str,
}

/// How a kind looks. A kind Fleet adds that this build does not know is a grey circle, still listed.
pub fn look_of(kind: &str) -> KindLook {
    let (glyph, hue, one, many) = match kind {
        "subagent_context" => ("split", token::SKETCH_ORANGE, "request", "requests"),
        "main_context" => ("message-square-text", token::SKETCH_AMBER, "request", "requests"),
        "cold_cache" => ("clock", token::SKETCH_BLUE, "wake", "wakes"),
        "repeat_reads" => ("scroll-text", token::SKETCH_TEAL, "read", "reads"),
        "big_results" => ("rows-3", token::SKETCH_GREEN, "result", "results"),
        "fresh_starts" => ("rotate-cw", token::SKETCH_GREY, "start", "starts"),
        _ => ("circle-dot", token::SKETCH_GREY, "time", "times"),
    };
    KindLook { glyph, hue, one, many }
}

/// Fleet's burn units as a person reads them: `1,116`, `123`, `31`, `1.9`. A hundred or more is rounded with a thousands
/// separator, ten to ninety-nine is rounded, and under ten keeps one decimal.
pub fn units_said(units: f64) -> String {
    let units = units.max(0.);
    if units >= 100. {
        let whole = units.round() as u64;
        let digits = whole.to_string();
        let mut said = String::new();
        for (at, digit) in digits.chars().enumerate() {
            if at > 0 && (digits.len() - at) % 3 == 0 {
                said.push(',');
            }
            said.push(digit);
        }
        said
    } else if units >= 9.95 {
        format!("{}", units.round() as u64)
    } else if units == 0. {
        "0".into()
    } else {
        format!("{units:.1}")
    }
}

/// A context size, which is tokens and not units: `870k`, `1.2M`.
pub fn tokens_said(tokens: u64) -> String {
    if tokens >= 999_500 {
        let millions = tokens as f64 / 1_000_000.;
        if millions >= 10. || (millions - millions.round()).abs() < 0.05 {
            format!("{}M", millions.round() as u64)
        } else {
            format!("{millions:.1}M")
        }
    } else if tokens >= 1_000 {
        format!("{}k", (tokens as f64 / 1_000.).round() as u64)
    } else {
        tokens.to_string()
    }
}

/// The kinds whose Jobs carry what caused them: the file read again, the tool whose result was large.
pub fn shows_causes(kind: &str) -> bool {
    matches!(kind, "repeat_reads" | "big_results")
}

/// The longest a cause is drawn before its middle is dropped.
const CAUSE_CHARS: usize = 40;

/// A cause as it is drawn: a path from the slot folder (`.armada/slots/slot-N/`) or the repository root on, a tool as it is,
/// and where it is still long, its middle left out.
pub fn cause_said(name: &str, roots: &[String]) -> String {
    middle(&from_root(name, roots), CAUSE_CHARS)
}

fn from_root<'a>(name: &'a str, roots: &[String]) -> &'a str {
    const SLOTS: &str = ".armada/slots/slot-";
    if let Some(at) = name.find(SLOTS) {
        let after = &name[at + SLOTS.len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with('/') {
            return &after[digits + 1..];
        }
    }
    roots
        .iter()
        .map(|root| root.trim_end_matches('/'))
        .filter(|root| !root.is_empty())
        .filter_map(|root| name.strip_prefix(root).and_then(|rest| rest.strip_prefix('/')))
        .min_by_key(|rest| rest.len())
        .unwrap_or(name)
}

/// `head…tail` of at most `max` characters, the tail the longer so the file's name stays.
fn middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let keep = max - 1;
    let head = keep / 3;
    let tail = keep - head;
    format!("{}…{}", chars[..head].iter().collect::<String>(), chars[chars.len() - tail..].iter().collect::<String>())
}

/// `41 requests`, and `1 request`.
pub fn times_said(count: u64, look: KindLook) -> String {
    format!("{count} {}", if count == 1 { look.one } else { look.many })
}

/// A Job's short id, for the Job with no title.
pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// One row of the list of kinds.
#[derive(Clone, Debug, PartialEq)]
pub struct KindRow {
    pub kind: String,
    pub title: String,
    pub glyph: &'static str,
    pub hue: Token,
    pub units: String,
    /// What the figure's tooltip says: how many times it happened.
    pub tip: String,
}

/// One line of a kind's table of Jobs.
#[derive(Clone, Debug, PartialEq)]
pub struct JobLine {
    pub id: String,
    pub title: String,
    pub units: String,
    /// Empty where Fleet recorded no peak.
    pub peak: String,
    /// The Job's largest cause, as drawn; empty for a kind that names none.
    pub cause: String,
    /// The cause's whole name, for its tooltip.
    pub cause_tip: String,
    pub tip: String,
}

/// A reading with its kinds and their Jobs put largest first, and the span it was asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    span: Span,
    waste: UsageWaste,
}

impl Board {
    pub fn new(mut waste: UsageWaste, span: Span) -> Board {
        waste.kinds.sort_by(|a, b| b.units.total_cmp(&a.units).then_with(|| a.title.cmp(&b.title)));
        for kind in &mut waste.kinds {
            kind.jobs.sort_by(|a, b| b.units.total_cmp(&a.units).then_with(|| a.session_id.cmp(&b.session_id)));
        }
        Board { span, waste }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    /// The line over the list. None where the window holds no burn: it has nothing to say.
    pub fn header(&self) -> Option<String> {
        (self.waste.total_units > 0.).then(|| format!("{} of {} units went to fixable waste", units_said(self.waste.waste_units), units_said(self.waste.total_units)))
    }

    pub fn rows(&self) -> Vec<KindRow> {
        self.waste
            .kinds
            .iter()
            .map(|kind| {
                let look = look_of(&kind.kind);
                KindRow { kind: kind.kind.clone(), title: kind.title.clone(), glyph: look.glyph, hue: look.hue, units: units_said(kind.units), tip: times_said(u64::from(kind.occurrences), look) }
            })
            .collect()
    }

    /// The kinds' ids in the order drawn.
    pub fn kinds(&self) -> Vec<&str> {
        self.waste.kinds.iter().map(|kind| kind.kind.as_str()).collect()
    }

    pub fn kind(&self, kind: &str) -> Option<&WasteKind> {
        self.waste.kinds.iter().find(|one| one.kind == kind)
    }

    /// The Jobs a kind hit, largest first, each with the title Bridge holds for it or its short id.
    pub fn lines_of(&self, kind: &str, titles: &HashMap<String, String>, roots: &[String]) -> Vec<JobLine> {
        let Some(found) = self.kind(kind) else { return Vec::new() };
        let look = look_of(kind);
        found
            .jobs
            .iter()
            .map(|job| JobLine {
                id: job.session_id.clone(),
                title: titles.get(&job.session_id).filter(|title| !title.is_empty()).cloned().unwrap_or_else(|| short_id(&job.session_id)),
                units: units_said(job.units),
                peak: job.peak_context.map(tokens_said).unwrap_or_default(),
                cause: job.causes.first().filter(|_| shows_causes(kind)).map(|cause| cause_said(&cause.name, roots)).unwrap_or_default(),
                cause_tip: job.causes.first().filter(|_| shows_causes(kind)).map(|cause| cause.name.clone()).unwrap_or_default(),
                tip: times_said(u64::from(job.occurrences), look),
            })
            .collect()
    }
}

/// What the page holds while Fleet is asked: the span the switch is on, the rows last read, and why the last read failed.
/// The switch moves in the frame it is pressed and the rows stay until the answer, so the page is never empty between two reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    span: Span,
    board: Option<Board>,
    failed: Option<String>,
}

/// What the page draws from what it holds.
#[derive(Debug, PartialEq)]
pub enum Shown<'a> {
    /// Nothing has been read yet: skeleton rows.
    Loading,
    /// The read Fleet refused or could not make, and what it said.
    Failed(&'a str),
    /// The rows; `pending` where they are for another span than the switch is on.
    Read { board: &'a Board, pending: bool },
}

impl Held {
    pub fn new(span: Span) -> Held {
        Held { span, board: None, failed: None }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    /// A press on the switch. False where it is already there, so nothing is asked again.
    pub fn pick(&mut self, span: Span) -> bool {
        if self.span == span {
            return false;
        }
        self.span = span;
        self.failed = None;
        true
    }

    /// Fleet's answer for `span`. An answer for a span the switch has left is dropped. A failed read keeps the rows where they
    /// are for the span shown, which a later read puts right; where they are for another span they are not drawn under the wrong switch.
    pub fn arrived(&mut self, span: Span, answer: Result<UsageWaste, String>) {
        if span != self.span {
            return;
        }
        match answer {
            Ok(waste) => {
                self.board = Some(Board::new(waste, span));
                self.failed = None;
            }
            Err(why) => {
                if self.board.as_ref().map(Board::span) != Some(span) {
                    self.failed = Some(why);
                }
            }
        }
    }

    /// Whether the last read for the span shown failed.
    pub fn failed(&self) -> bool {
        self.failed.is_some()
    }

    /// The rows read, whatever span they are for: the keys and the table work from them.
    pub fn board(&self) -> Option<&Board> {
        self.board.as_ref().filter(|_| self.failed.is_none())
    }

    pub fn shown(&self) -> Shown<'_> {
        match (&self.failed, &self.board) {
            (Some(why), _) => Shown::Failed(why),
            (None, None) => Shown::Loading,
            (None, Some(board)) => Shown::Read { board, pending: board.span() != self.span },
        }
    }
}

/// Where the keys are: the row of the list, and the kind whose panel is open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
    pub cursor: Option<String>,
    pub opened: Option<String>,
}

impl Selection {
    /// One row down or up, held at either end. The first move from nowhere lands on the first or the last row.
    pub fn step(&mut self, kinds: &[&str], by: isize) {
        if kinds.is_empty() {
            return;
        }
        let at = self.cursor.as_deref().and_then(|cursor| kinds.iter().position(|kind| *kind == cursor));
        let next = match at {
            Some(at) => (at as isize + by).clamp(0, kinds.len() as isize - 1) as usize,
            None if by > 0 => 0,
            None => kinds.len() - 1,
        };
        self.cursor = Some(kinds[next].to_string());
    }

    /// Enter, Space or a press: the panel of the row under the keys opens, or shuts where it is open.
    pub fn toggle(&mut self, kind: &str) {
        self.cursor = Some(kind.to_string());
        self.opened = if self.opened.as_deref() == Some(kind) { None } else { Some(kind.to_string()) };
    }

    pub fn close(&mut self) -> bool {
        self.opened.take().is_some()
    }

    /// A new reading: the cursor falls back to the first row where its kind is gone, and a panel whose kind is gone shuts.
    pub fn keep(&mut self, kinds: &[&str]) {
        if !self.cursor.as_deref().is_some_and(|cursor| kinds.contains(&cursor)) {
            self.cursor = kinds.first().map(|first| first.to_string());
        }
        if !self.opened.as_deref().is_some_and(|opened| kinds.contains(&opened)) {
            self.opened = None;
        }
    }
}
