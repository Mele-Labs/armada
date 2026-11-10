//! What the dispatch sheet draws, as data. Electron works each of these out in TypeScript
//! (`docs/spikes/native-titlebar-inventory/dispatch-merge-clash.md`, section 5.1); here each is a
//! small struct a walk's mock fills and the wiring later maps Fleet's values into, so a view
//! reads one and works out none: which chip leads and carries a key, the six reading states and their
//! words, the wait sentence and its cycles, the menu rows for what is typed.

use gpui::SharedString;

/// What a dispatch starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Job,
    Autopilot,
}

impl Kind {
    /// The kind's word, which is also its tooltip and its tab (`DP/kinds.ts`).
    pub fn word(self) -> &'static str {
        match self {
            Kind::Job => "Job",
            Kind::Autopilot => "Autopilot",
        }
    }

    /// The kind's registry glyph, drawn on the line and in the strip.
    pub fn glyph(self) -> &'static str {
        match self {
            Kind::Job => "square-terminal",
            Kind::Autopilot => "plane",
        }
    }
}

/// How a repository chip stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Plain,
    /// The one the words name: an accent edge.
    Read,
    /// The one the request goes to: filled.
    Chosen,
}

/// A repository in the row.
#[derive(Clone, Debug)]
pub struct Chip {
    pub name: SharedString,
    /// The digit of its `dispatch_repository` key: the first nine chips carry one.
    pub key: Option<u8>,
    pub mark: Mark,
}

/// The six readings of which repository the words name (`DP/DispatchPanel.tsx:95-126`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    Idle,
    Detecting,
    Unresolved,
    Detected,
    Picked,
    Scoped,
}

impl Reading {
    /// What its tooltip says.
    pub fn tip(self) -> &'static str {
        match self {
            Reading::Idle => "Repository",
            Reading::Detecting => "Finding the repository",
            Reading::Unresolved => "No repository found",
            Reading::Detected => "Found in the request",
            Reading::Picked => "Picked by you",
            Reading::Scoped => "The one the rail is on",
        }
    }

    /// Its accessible name, the repository's name after it where one is chosen.
    pub fn name(self, chosen: Option<&str>) -> String {
        match (self, chosen) {
            (Reading::Detecting, _) => "Repository, finding".into(),
            (Reading::Unresolved, _) => "Repository, not found".into(),
            (Reading::Detected | Reading::Picked | Reading::Scoped, Some(name)) => format!("Repository {name}"),
            _ => "Repository".into(),
        }
    }

    /// Whether the mark is drawn in the accent (a repository stands) or muted.
    pub fn found(self) -> bool {
        matches!(self, Reading::Detected | Reading::Picked | Reading::Scoped)
    }
}

/// Which part of the wait sentence a hand has set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    On,
    When,
    Base,
    Needs,
}

impl Part {
    /// The tooltip each part carries (`DP/WaitChip.tsx:12-17`).
    pub fn tip(self) -> &'static str {
        match self {
            Part::On => "The Job it waits on",
            Part::When => "When it starts",
            Part::Base => "Where it starts from",
            Part::Needs => "What it needs",
        }
    }
}

/// One part of the sentence and what it cycles through.
#[derive(Clone, Debug)]
pub struct Cycle {
    pub value: SharedString,
    pub label: SharedString,
    /// `(value, label)` of each alternative, in the order a press goes through them.
    pub options: Vec<(SharedString, SharedString)>,
}

impl Cycle {
    pub fn of(value: &str, options: &[(&str, &str)]) -> Cycle {
        let label = options.iter().find(|(one, _)| *one == value).map(|(_, label)| *label).unwrap_or(value);
        Cycle {
            value: value.to_string().into(),
            label: label.to_string().into(),
            options: options.iter().map(|(value, label)| (value.to_string().into(), label.to_string().into())).collect(),
        }
    }

    /// One step through the alternatives, wrapping; `back` goes the other way.
    pub fn moved(&self, back: bool) -> Cycle {
        let at = self.options.iter().position(|(value, _)| *value == self.value).unwrap_or(0);
        let len = self.options.len().max(1);
        let to = if back { (at + len - 1) % len } else { (at + 1) % len };
        match self.options.get(to) {
            Some((value, label)) => Cycle { value: value.clone(), label: label.clone(), options: self.options.clone() },
            None => self.clone(),
        }
    }
}

/// A Job the brief can wait on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    pub id: SharedString,
    pub title: SharedString,
}

/// The sentence over the request: `Starts when [Job]'s [PR lands] · from [main] · needs [..]`.
#[derive(Clone, Debug)]
pub struct Wait {
    pub on: Offered,
    pub when: Cycle,
    pub base: Cycle,
    /// Absent where the brief asks for nothing.
    pub needs: Option<Cycle>,
    /// The parts a hand set: an accent edge and `, set by you` in the tooltip.
    pub his: Vec<Part>,
    /// The other Jobs the first part's menu offers.
    pub others: Vec<Offered>,
}

impl Wait {
    pub fn his(&self, part: Part) -> bool {
        self.his.contains(&part)
    }

    /// A tooltip: the part's own words, and that a hand set it.
    pub fn tip(&self, part: Part) -> String {
        format!("{}{}", part.tip(), if self.his(part) { ", set by you" } else { "" })
    }

    /// A part cycled by a hand.
    pub fn cycled(&mut self, part: Part, back: bool) {
        match part {
            Part::When => self.when = self.when.moved(back),
            Part::Base => self.base = self.base.moved(back),
            Part::Needs => {
                if let Some(needs) = &self.needs {
                    self.needs = Some(needs.moved(back));
                }
            }
            Part::On => return,
        }
        if !self.his.contains(&part) {
            self.his.push(part);
        }
    }

    /// The Job it waits on, chosen from the menu.
    pub fn waits_on(&mut self, id: &str) {
        let Some(at) = self.others.iter().position(|one| one.id == id) else { return };
        let was = std::mem::replace(&mut self.on, self.others[at].clone());
        self.others[at] = was;
        if !self.his.contains(&Part::On) {
            self.his.push(Part::On);
        }
    }
}

/// What a row of the `@` or `#` menu stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Job,
    File,
    PullRequest,
    Issue,
    Bug,
    Terminal,
}

impl Glyph {
    pub fn icon(self) -> &'static str {
        match self {
            Glyph::Job => "square-terminal",
            Glyph::File => "file",
            Glyph::PullRequest => "git-pull-request",
            Glyph::Issue => "ticket",
            Glyph::Bug => "bug",
            Glyph::Terminal => "terminal",
        }
    }
}

/// One row of a menu.
#[derive(Clone, Debug)]
pub struct Row {
    pub glyph: Glyph,
    pub name: SharedString,
    pub says: Option<SharedString>,
    pub standing: Option<SharedString>,
    /// What choosing it writes into the request.
    pub insert: SharedString,
}

/// Which menu is open, by its trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    At,
    Hash,
}

/// What the `@` and `#` menus can offer, and how a typed query narrows it
/// (`DP/SessionMenu.tsx:52-123`, `:321-352`): Jobs whose title holds the query, then up to
/// [`FILE_ROWS`] files once something follows the `@`; issues and pull requests, numbers first.
#[derive(Clone, Debug, Default)]
pub struct Mentions {
    pub jobs: Vec<Offered>,
    pub files: Vec<SharedString>,
    pub tickets: Vec<Ticket>,
}

pub const FILE_ROWS: usize = 5;
pub const NUMBERED_ROWS: usize = 6;

#[derive(Clone, Debug)]
pub struct Ticket {
    pub glyph: Glyph,
    pub id: SharedString,
    pub reference: SharedString,
    pub title: SharedString,
    pub standing: Option<SharedString>,
}

impl Mentions {
    /// The menu's label (`DP/SessionMenu.tsx:353-358`).
    pub fn label(trigger: Trigger, query: &str) -> &'static str {
        match trigger {
            Trigger::At if query.is_empty() => "Jobs",
            Trigger::At => "Tag a Job or a file",
            Trigger::Hash => "Tag an issue or pull request",
        }
    }

    pub fn rows(&self, trigger: Trigger, query: &str) -> Vec<Row> {
        let needle = query.to_lowercase();
        match trigger {
            Trigger::At => {
                let jobs = self.jobs.iter().filter(|one| one.title.to_lowercase().contains(&needle)).map(|one| Row {
                    glyph: Glyph::Job,
                    name: one.title.clone(),
                    says: None,
                    standing: None,
                    insert: format!("@{} ", one.title).into(),
                });
                let files = self
                    .files
                    .iter()
                    .filter(|_| !needle.is_empty())
                    .filter(|path| path.to_lowercase().contains(&needle))
                    .take(FILE_ROWS)
                    .map(|path| Row {
                        glyph: Glyph::File,
                        name: path.clone(),
                        says: None,
                        standing: None,
                        insert: format!("@{path} ").into(),
                    });
                jobs.chain(files).collect()
            }
            Trigger::Hash => {
                let numbered = !needle.is_empty() && needle.chars().all(|ch| ch.is_ascii_digit());
                let mut rows: Vec<&Ticket> = self
                    .tickets
                    .iter()
                    .filter(|one| {
                        if needle.is_empty() {
                            true
                        } else if numbered {
                            one.id.chars().all(|ch| ch.is_ascii_digit()) && one.id.starts_with(&needle)
                        } else {
                            !one.id.chars().all(|ch| ch.is_ascii_digit()) && one.reference.to_lowercase().starts_with(&needle)
                        }
                    })
                    .collect();
                // Numbers first, by descending id, then keys.
                rows.sort_by(|a, b| {
                    let (a_num, b_num) = (a.id.parse::<u64>().ok(), b.id.parse::<u64>().ok());
                    match (a_num, b_num) {
                        (Some(a), Some(b)) => b.cmp(&a),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.reference.cmp(&b.reference),
                    }
                });
                rows.into_iter()
                    .take(NUMBERED_ROWS)
                    .map(|one| Row {
                        glyph: one.glyph,
                        name: one.reference.clone(),
                        says: Some(one.title.clone()),
                        standing: one.standing.clone(),
                        insert: format!("{} {} ", one.reference, one.title).into(),
                    })
                    .collect()
            }
        }
    }
}

/// What Fleet read of an Autopilot sentence (`AutopilotProposal`, `quoted` included): the
/// instruction and the fields, each with the words it was read from where the sentence said any.
#[derive(Clone, Debug)]
pub struct AutopilotRead {
    pub instruction: SharedString,
    pub fields: Vec<AutopilotField>,
}

#[derive(Clone, Debug)]
pub struct AutopilotField {
    pub label: &'static str,
    pub value: SharedString,
    pub placeholder: &'static str,
    pub quoted: Option<SharedString>,
}

/// The model and effort lists, and what `Auto` stands for.
#[derive(Clone, Debug)]
pub struct Tuning {
    pub default_model: Option<SharedString>,
    pub models: Vec<SharedString>,
    pub efforts: Vec<SharedString>,
}

/// Everything the sheet is drawn from.
#[derive(Clone, Debug)]
pub struct Face {
    /// Whether the window is connected to Fleet; off, everything is disabled.
    pub connected: bool,
    /// The kinds offered, the default first.
    pub kinds: Vec<Kind>,
    /// Chips in the order they are shown, the first nine keyed.
    pub chips: Vec<Chip>,
    /// Repositories past the ninth, under More.
    pub more: Vec<Chip>,
    /// What the words name, or the rail's repository, before a hand has picked.
    pub reading: Reading,
    pub tuning: Tuning,
    /// Whether the queue for later is served.
    pub queue: bool,
    pub mentions: Mentions,
    /// The sentence a brief that waits on a Job reads as, once a read stands.
    pub wait: Option<Wait>,
    /// What Fleet read of an Autopilot sentence, once asked.
    pub read: Option<AutopilotRead>,
    /// What the Autopilot read was refused with, in Fleet's sentence.
    pub refused: Option<SharedString>,
}

impl Face {
    /// The word on the act: `Start` on a Job; on an Autopilot `Read it`, then `Start Autopilot`
    /// once a read stands (`DP/dispatching.tsx:694`).
    pub fn act(&self, kind: Kind) -> &'static str {
        match kind {
            Kind::Job => "Start",
            Kind::Autopilot if self.read.is_some() => "Start Autopilot",
            Kind::Autopilot => "Read it",
        }
    }

    /// The name of the chip the request goes to.
    pub fn chosen(&self) -> Option<&Chip> {
        self.chips.iter().chain(&self.more).find(|chip| chip.mark == Mark::Chosen)
    }

    /// A sheet opened again: nothing chosen by hand or read from earlier words stands. A scoped rail does.
    pub fn fresh(&mut self) {
        if self.reading == Reading::Scoped {
            return;
        }
        for chip in self.chips.iter_mut().chain(self.more.iter_mut()) {
            chip.mark = Mark::Plain;
        }
        self.reading = Reading::Idle;
    }

    /// The repository chosen by hand: it is `Chosen`, and the rest are not. The read one stays marked.
    pub fn pick(&mut self, name: &str) {
        for chip in self.chips.iter_mut().chain(self.more.iter_mut()) {
            if chip.mark == Mark::Chosen {
                chip.mark = Mark::Plain;
            }
            if chip.name == name {
                chip.mark = Mark::Chosen;
            }
        }
        self.reading = Reading::Picked;
    }
}

/// What a send leaves: the strip's title and kind (`DP/dispatching.tsx:507-510`).
#[derive(Clone, Debug)]
pub struct Sent {
    pub kind: Kind,
    pub title: SharedString,
    /// The repository it runs in, which the Open on the strip asks the Autopilot lane of.
    pub root: SharedString,
}
