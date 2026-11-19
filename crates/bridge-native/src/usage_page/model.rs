//! What the Usage page shows, worked out from Fleet's waste reading and nothing there touches the screen.
//!
//! The types below mirror the reading Fleet will serve (`UsageWaste`). They are local until Fleet's route lands
//! in `ipc`; the page then takes that type and this file keeps the board, the units and the selection.

use std::collections::HashMap;

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

    /// The key `usage_window`'s slot is on: Day, Week, Fortnight.
    pub fn at(slot: u8) -> Option<Span> {
        Span::ALL.get(slot as usize).copied()
    }
}

/// The window a reading was made over, as Fleet names it.
#[derive(Clone, Debug, PartialEq)]
pub struct WasteWindow {
    pub id: Span,
    pub since: String,
    pub until: String,
}

/// One Job a kind of waste hit.
#[derive(Clone, Debug, PartialEq)]
pub struct WasteJob {
    pub session_id: String,
    pub units: u64,
    pub occurrences: u64,
    pub peak_context: Option<u64>,
}

/// One kind of waste, with the Jobs it hit.
#[derive(Clone, Debug, PartialEq)]
pub struct WasteKind {
    pub kind: String,
    pub title: String,
    pub fix: String,
    pub units: u64,
    pub occurrences: u64,
    pub jobs: Vec<WasteJob>,
}

/// Fleet's reading of where the burn was wasted over one window.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageWaste {
    pub until: String,
    pub window: WasteWindow,
    pub total_units: u64,
    pub waste_units: u64,
    pub kinds: Vec<WasteKind>,
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

/// Burn units as a person reads them: `131M`, `4.2M`, `870k`.
pub fn units_said(units: u64) -> String {
    if units >= 999_500 {
        let millions = units as f64 / 1_000_000.;
        if millions >= 10. || (millions - millions.round()).abs() < 0.05 {
            format!("{}M", millions.round() as u64)
        } else {
            format!("{millions:.1}M")
        }
    } else if units >= 1_000 {
        format!("{}k", (units as f64 / 1_000.).round() as u64)
    } else {
        units.to_string()
    }
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
    pub tip: String,
}

/// A reading with its kinds and their Jobs put largest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    waste: UsageWaste,
}

impl Board {
    pub fn new(mut waste: UsageWaste) -> Board {
        waste.kinds.sort_by(|a, b| b.units.cmp(&a.units).then_with(|| a.title.cmp(&b.title)));
        for kind in &mut waste.kinds {
            kind.jobs.sort_by(|a, b| b.units.cmp(&a.units).then_with(|| a.session_id.cmp(&b.session_id)));
        }
        Board { waste }
    }

    pub fn span(&self) -> Span {
        self.waste.window.id
    }

    /// The line over the list. None where the window holds no burn: it has nothing to say.
    pub fn header(&self) -> Option<String> {
        (self.waste.total_units > 0).then(|| format!("{} of {} went to fixable waste", units_said(self.waste.waste_units), units_said(self.waste.total_units)))
    }

    pub fn rows(&self) -> Vec<KindRow> {
        self.waste
            .kinds
            .iter()
            .map(|kind| {
                let look = look_of(&kind.kind);
                KindRow { kind: kind.kind.clone(), title: kind.title.clone(), glyph: look.glyph, hue: look.hue, units: units_said(kind.units), tip: times_said(kind.occurrences, look) }
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
    pub fn lines_of(&self, kind: &str, titles: &HashMap<String, String>) -> Vec<JobLine> {
        let Some(found) = self.kind(kind) else { return Vec::new() };
        let look = look_of(kind);
        found
            .jobs
            .iter()
            .map(|job| JobLine {
                id: job.session_id.clone(),
                title: titles.get(&job.session_id).filter(|title| !title.is_empty()).cloned().unwrap_or_else(|| short_id(&job.session_id)),
                units: units_said(job.units),
                peak: job.peak_context.map(units_said).unwrap_or_default(),
                tip: times_said(job.occurrences, look),
            })
            .collect()
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
