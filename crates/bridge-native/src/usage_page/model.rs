//! What the Usage page shows, worked out from Fleet's waste reading (`ipc::UsageWaste`) and nothing there touches the screen.
//! `Held` is what the page holds while a reading is asked for: the span on the switch, the rows last read, and why a read failed.

use std::collections::{HashMap, HashSet};

use ipc::{AdjustChange, AdjustSetting, SettingValue, UsageAdjustments, UsageWaste, WasteKind};

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

    /// Whose burn the window is: `today's`, `this week's`.
    pub fn whose(self) -> &'static str {
        match self {
            Span::Day => "today's",
            Span::Week => "this week's",
            Span::Fortnight => "this fortnight's",
        }
    }

    /// What a share is a share of, at the head of a column: `Share of today`.
    pub fn share_head(self) -> &'static str {
        match self {
            Span::Day => "Share of today",
            Span::Week => "Share of the week",
            Span::Fortnight => "Share of the fortnight",
        }
    }

    /// The head of the column of what a change would save: `Saves a week`.
    pub fn saves_head(self) -> &'static str {
        match self {
            Span::Day => "Saves a day",
            Span::Week => "Saves a week",
            Span::Fortnight => "Saves a fortnight",
        }
    }

    /// The word Fleet's route takes: `GET /usage/waste?window=`, and `GET /usage/adjustments?window=`.
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

/// A share of the burn, 0 to 1, as a person reads it: `15%`, and under ten percent one decimal, `5.5%`.
pub fn share_said(share: f64) -> String {
    let percent = (share * 100.).max(0.);
    if percent == 0. {
        "0%".into()
    } else if percent < 9.95 {
        format!("{percent:.1}%")
    } else {
        format!("{}%", percent.round() as u64)
    }
}

/// A share with its decimal always, so a before and a since read against each other: `12.3%`.
pub fn share_exact(share: f64) -> String {
    format!("{:.1}%", (share * 100.).max(0.))
}

/// A stamp as a day and a month in `zone`, `days` later: `2026-11-17T23:20:00Z` is `18 Nov` in Berlin. A stamp this cannot read is said as it came.
pub fn date_in<Z: chrono::TimeZone>(at: &str, days: u64, zone: &Z) -> String
where
    Z::Offset: std::fmt::Display,
{
    chrono::DateTime::parse_from_rfc3339(at)
        .ok()
        .and_then(|stamp| stamp.with_timezone(zone).checked_add_days(chrono::Days::new(days)))
        .map(|day| day.format("%-d %b").to_string())
        .unwrap_or_else(|| at.to_string())
}

/// The day a stamp falls on in this Mac's zone, `days` later.
pub fn date_said(at: &str, days: u64) -> String {
    date_in(at, days, &chrono::Local)
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
    /// The kind's share of the window's burn.
    pub share: String,
    /// What the figure's tooltip says: how many times it happened.
    pub tip: String,
}

/// One line of a kind's table of Jobs.
#[derive(Clone, Debug, PartialEq)]
pub struct JobLine {
    pub id: String,
    pub title: String,
    /// The Job's share of the window's burn the kind took.
    pub share: String,
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
        (self.waste.total_units > 0.).then(|| format!("{} of {} burn was waste you can fix", share_said(self.waste.waste_units / self.waste.total_units), self.span.whose()))
    }

    /// A figure of units as a share of the window's whole burn, 0 to 1.
    fn share_of(&self, units: f64) -> f64 {
        if self.waste.total_units > 0. { units / self.waste.total_units } else { 0. }
    }

    pub fn rows(&self) -> Vec<KindRow> {
        self.waste
            .kinds
            .iter()
            .map(|kind| {
                let look = look_of(&kind.kind);
                KindRow { kind: kind.kind.clone(), title: kind.title.clone(), glyph: look.glyph, hue: look.hue, share: share_said(self.share_of(kind.units)), tip: times_said(u64::from(kind.occurrences), look) }
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
                share: share_said(self.share_of(job.units)),
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

/// The key of a Job's row in the open kind's table, among the rows the keys move between.
pub fn job_key(job: &str) -> String {
    format!("job:{job}")
}

/// The Job a row key names, where it names one.
pub fn job_of_key(key: &str) -> Option<&str> {
    key.strip_prefix("job:")
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

    /// The panel shuts, and the keys go back to the kind's row where they were on one of its Jobs.
    pub fn close(&mut self) -> bool {
        let closed = self.opened.take();
        if closed.is_some() && self.cursor.as_deref().is_some_and(|cursor| job_of_key(cursor).is_some()) {
            self.cursor = closed.clone();
        }
        closed.is_some()
    }

    /// A new reading: the cursor falls back to the first row where its row is gone, and a panel whose kind is gone shuts.
    /// `stops` are the rows the keys move between, settings first and then kinds; `kinds` are the ones that open a panel.
    pub fn keep(&mut self, stops: &[&str], kinds: &[&str]) {
        if !self.cursor.as_deref().is_some_and(|cursor| stops.contains(&cursor)) {
            self.cursor = stops.first().map(|first| first.to_string());
        }
        if !self.opened.as_deref().is_some_and(|opened| kinds.contains(&opened)) {
            self.opened = None;
        }
    }
}

/// A waste kind's title from the reading, or its id spelled out where the reading is not held or does not name it.
pub fn title_of(board: Option<&Board>, kind: &str) -> String {
    board.and_then(|board| board.kind(kind)).map(|found| found.title.clone()).unwrap_or_else(|| {
        let spelled = kind.replace('_', " ");
        let mut letters = spelled.chars();
        letters.next().map_or_else(String::new, |first| first.to_uppercase().chain(letters).collect())
    })
}

/// What a settings row says under it, where it says something.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// Fleet's reason a change was not taken.
    Failed(String),
    /// Saved, and in force once Fleet starts again.
    Restart,
}

/// One row of the table of settings.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingRow {
    pub key: String,
    pub label: String,
    /// The label of the value in force, or of the one being applied.
    pub now: String,
    /// The value shown in `now` is waiting on Fleet.
    pub pending: bool,
    /// The labels of the values it may be changed to, and the one the dropdown shows.
    pub choices: Vec<String>,
    pub chosen: usize,
    /// The title of the waste kind it cuts.
    pub cuts: String,
    /// `5.5%`, or `-` where Fleet cannot say.
    pub saves: String,
    /// The share's tooltip: the value it is priced at.
    pub saves_tip: String,
    pub can_apply: bool,
    pub note: Option<Note>,
}

/// Which way the waste a change cuts moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    Down,
    Up,
    Same,
}

impl Moved {
    /// The chip's registry glyph, ink and wash: the passed hue for a fall, the awaiting review hue for a rise, and a quiet chip with no glyph for neither.
    pub fn mark(self) -> (Option<&'static str>, Token, Token) {
        match self {
            Moved::Down => (Some("trending-down"), token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
            Moved::Up => (Some("trending-up"), token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG),
            Moved::Same => (None, token::FG_MUTED, token::BG_SUNKEN),
        }
    }
}

/// What a change came to once a day had passed, or when it will.
#[derive(Clone, Debug, PartialEq)]
pub enum After {
    /// The share since the change, and the difference from before it in a chip.
    Since { share: String, moved: Moved, chip: String, tip: String },
    /// A day has not passed: `Ready 20 Nov`.
    Ready { said: String, tip: String },
}

/// One row of the changes already made: what was changed and when, the waste it cuts before, and the same after.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeRow {
    /// `Compact a Job at: 400k → 200k`.
    pub change: String,
    /// The day it came into force, in this Mac's zone: `19 Nov`.
    pub date: String,
    /// `12.3% of your burn`: the share of the week before the change.
    pub before: String,
    /// The title of the waste kind the setting cuts.
    pub kind: String,
    pub after: After,
}

/// What the person did on the settings rows and has not been read back yet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edits {
    /// The value a dropdown was left on.
    chosen: HashMap<String, SettingValue>,
    /// Sent, and waiting on Fleet.
    applying: HashMap<String, SettingValue>,
    /// Saved: the value shown as in force until the next reading says so.
    settled: HashMap<String, SettingValue>,
    failed: HashMap<String, String>,
    restart: HashSet<String>,
}

/// What the page holds of the settings that change the burn: the last reading, the span it is for, and the edits on it.
/// A reading that fails leaves the last one where it is, and none read leaves the panels undrawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Changing {
    read: Option<(Span, UsageAdjustments)>,
    edits: Edits,
}

impl Changing {
    /// Fleet's answer for `span`. An answer for a span the switch has left is dropped, and a refused one keeps what was read.
    /// A reading ends the wait on a saved value: the reading holds it now.
    pub fn arrived(&mut self, span: Span, shown: Span, answer: Result<UsageAdjustments, String>) {
        if span != shown {
            return;
        }
        if let Ok(adjustments) = answer {
            self.read = Some((span, adjustments));
            self.edits.settled.clear();
        }
    }

    /// The span the reading is for.
    pub fn span(&self) -> Option<Span> {
        self.read.as_ref().map(|(span, _)| *span)
    }

    fn setting(&self, key: &str) -> Option<&AdjustSetting> {
        self.read.as_ref()?.1.settings.iter().find(|one| one.key == key)
    }

    /// The keys, in the order drawn.
    pub fn keys(&self) -> Vec<&str> {
        self.read.as_ref().map(|(_, read)| read.settings.iter().map(|one| one.key.as_str()).collect()).unwrap_or_default()
    }

    /// The value in force, with a saved one that has not been read back yet.
    fn now_of(&self, one: &AdjustSetting) -> SettingValue {
        self.edits.settled.get(&one.key).cloned().unwrap_or_else(|| one.now.clone())
    }

    /// Where the dropdown stands: what was picked, else the suggestion when it is not what is in force, else what is.
    fn chosen_of(&self, one: &AdjustSetting) -> usize {
        let now = self.now_of(one);
        let at = |value: &SettingValue| one.choices.iter().position(|choice| choice.value == *value);
        self.edits
            .chosen
            .get(&one.key)
            .and_then(at)
            .or_else(|| one.suggested.as_ref().filter(|suggested| **suggested != now).and_then(at))
            .or_else(|| at(&now))
            .unwrap_or(0)
    }

    fn label_of(one: &AdjustSetting, value: &SettingValue) -> String {
        one.choices.iter().find(|choice| choice.value == *value).map(|choice| choice.label.clone()).unwrap_or_else(|| match value {
            SettingValue::Text(text) => text.clone(),
            SettingValue::Integer(number) => number.to_string(),
            other => format!("{other:?}"),
        })
    }

    /// The rows of the table, with the waste's titles for what each cuts.
    pub fn rows(&self, board: Option<&Board>) -> Vec<SettingRow> {
        let Some((_, read)) = &self.read else { return Vec::new() };
        read.settings
            .iter()
            .map(|one| {
                let now = self.now_of(one);
                let chosen = self.chosen_of(one);
                let applying = self.edits.applying.get(&one.key);
                let note = match (self.edits.failed.get(&one.key), self.edits.restart.contains(&one.key)) {
                    (Some(why), _) => Some(Note::Failed(why.clone())),
                    (None, true) => Some(Note::Restart),
                    (None, false) => None,
                };
                SettingRow {
                    key: one.key.clone(),
                    label: one.label.clone(),
                    now: Self::label_of(one, applying.unwrap_or(&now)),
                    pending: applying.is_some(),
                    choices: one.choices.iter().map(|choice| choice.label.clone()).collect(),
                    chosen,
                    cuts: title_of(board, &one.cuts),
                    saves: one.saves_share.map(share_said).unwrap_or_else(|| "-".into()),
                    saves_tip: one.suggested.as_ref().map(|suggested| format!("Share of the burn saved at {}", Self::label_of(one, suggested))).unwrap_or_default(),
                    can_apply: applying.is_none() && one.choices.get(chosen).is_some_and(|choice| choice.value != now),
                    note,
                }
            })
            .collect()
    }

    /// The rows of the changes made, newest first, with the waste's titles for what each cut.
    pub fn changes(&self, board: Option<&Board>) -> Vec<ChangeRow> {
        self.changes_in(board, &chrono::Local)
    }

    /// As [`Self::changes`], with the days read in `zone`.
    pub fn changes_in<Z: chrono::TimeZone>(&self, board: Option<&Board>, zone: &Z) -> Vec<ChangeRow>
    where
        Z::Offset: std::fmt::Display,
    {
        let Some((_, read)) = &self.read else { return Vec::new() };
        read.changes.iter().map(|change| change_row(change, board, zone)).collect()
    }

    /// A pick in a setting's dropdown, by the place of its choice. It clears the reason the last try failed.
    pub fn choose(&mut self, key: &str, at: usize) {
        let Some(value) = self.setting(key).and_then(|one| one.choices.get(at)).map(|choice| choice.value.clone()) else { return };
        self.edits.chosen.insert(key.to_string(), value);
        self.edits.failed.remove(key);
    }

    /// Apply: the value the dropdown stands on, shown as pending in this frame. None where it is what is in force or one is already on its way.
    pub fn apply(&mut self, key: &str) -> Option<SettingValue> {
        let one = self.setting(key)?;
        if self.edits.applying.contains_key(key) {
            return None;
        }
        let value = one.choices.get(self.chosen_of(one))?.value.clone();
        if value == self.now_of(one) {
            return None;
        }
        self.edits.failed.remove(key);
        self.edits.applying.insert(key.to_string(), value.clone());
        Some(value)
    }

    /// Fleet took the change: the value stands as in force until the reading says so, and where it applies after a restart the row says that.
    pub fn saved(&mut self, key: &str, restart: bool) {
        let Some(value) = self.edits.applying.remove(key) else { return };
        self.edits.chosen.remove(key);
        self.edits.settled.insert(key.to_string(), value);
        if restart {
            self.edits.restart.insert(key.to_string());
        }
    }

    /// Fleet did not take it: the old value is back in the row, the dropdown stays where it was, and Fleet's reason is said.
    pub fn refused(&mut self, key: &str, why: String) {
        if self.edits.applying.remove(key).is_some() {
            self.edits.failed.insert(key.to_string(), why);
        }
    }

    /// Fleet started again: what waited on it is in force.
    pub fn restarted(&mut self) {
        self.edits.restart.clear();
    }

    /// The labels of a setting's choices and the one in force, for the dropdown.
    pub fn menu(&self, key: &str) -> Vec<(String, bool)> {
        let Some(one) = self.setting(key) else { return Vec::new() };
        let on = self.chosen_of(one);
        one.choices.iter().enumerate().map(|(at, choice)| (choice.label.clone(), at == on)).collect()
    }
}

fn change_row<Z: chrono::TimeZone>(change: &AdjustChange, board: Option<&Board>, zone: &Z) -> ChangeRow
where
    Z::Offset: std::fmt::Display,
{
    let kind = title_of(board, &change.cuts);
    let before = share_exact(change.before_share);
    let after = match change.since_share {
        Some(since) => {
            let now = share_exact(since);
            // The difference of the figures as drawn, so the chip agrees with them.
            let points = (since * 1000.).round() / 10. - (change.before_share * 1000.).round() / 10.;
            let moved = if points < -0.04 { Moved::Down } else if points > 0.04 { Moved::Up } else { Moved::Same };
            let chip = match moved {
                Moved::Down => format!("down {:.1} points", -points),
                Moved::Up => format!("up {:.1} points", points),
                Moved::Same => "no change".to_string(),
            };
            After::Since { share: now.clone(), moved, chip, tip: format!("{kind}: {before} of your burn in the week before, {now} since") }
        }
        None => After::Ready { said: format!("Ready {}", date_in(&change.at, 1, zone)), tip: "After a day, this compares the waste's share since the change with the week before".into() },
    };
    ChangeRow { change: format!("{}: {} → {}", change.label, change.from, change.to), date: date_in(&change.at, 0, zone), before: format!("{before} of your burn"), kind, after }
}
