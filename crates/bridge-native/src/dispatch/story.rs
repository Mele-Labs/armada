//! A sentence being written into the dispatch box: the kind taken, what a hand typed into each blank, and which blank
//! has the keys. This file is its state and its keys; `story_view.rs` draws it and `sheet.rs` carries it out.
//!
//! A blank takes letters at its end and nothing else: no caret moves inside one, so a blank is a word to retype and
//! not a field to edit. That keeps the keys few: ⇥ and ↵ go on, ⇧⇥ goes back, ⌫ takes a letter or steps back.

use gpui::SharedString;

use super::story_kinds::{kinds, Piece, StoryKind};

/// The most rows the feature list holds. A project's catalog is long, and the person narrows it by typing.
pub const LIST_ROWS: usize = 40;

/// One feature of the project's catalog, as the list offers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureChoice {
    pub title: SharedString,
    /// The title of its area, read beside it and searched with it.
    pub area: SharedString,
}

/// A project's features as the list offers them, in the library's order. A feature the Operator has yet to bring to him is no
/// feature of the catalog.
pub fn catalog_of(library: &ipc::FeatureLibrary) -> Vec<FeatureChoice> {
    library
        .areas
        .iter()
        .flat_map(|area| area.features.iter().filter(|row| row.state != ipc::FeatureState::Pending).map(|row| FeatureChoice { title: row.title.clone().into(), area: area.title.clone().into() }))
        .collect()
}

/// A row of the feature list: the feature, or the kind's own extra row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRow {
    pub title: SharedString,
    /// The area, empty on an extra row.
    pub area: SharedString,
}

/// The kinds whose opening begins with what has been typed, in the order the file lists them. Capitals do not matter,
/// and words that run past every opening, or span lines, match none.
pub fn openings(typed: &str) -> Vec<usize> {
    let typed = typed.trim_start();
    if typed.is_empty() || typed.contains('\n') {
        return Vec::new();
    }
    let typed = typed.to_lowercase();
    kinds().iter().enumerate().filter(|(_, kind)| kind.opening.to_lowercase().starts_with(&typed)).map(|(at, _)| at).collect()
}

/// What a key did to the sentence, for the sheet to carry out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyed {
    /// Not a key the sentence takes: it goes on to the sheet.
    Ignored,
    /// Taken, and nothing the sheet needs to hear.
    Taken,
    /// A blank's words changed: the draft and the repository read follow.
    Changed,
    /// ⌘↵.
    Start,
    /// Esc with nothing open: the sentence turns to plain words.
    Plain,
    /// ⌫ in the first blank, with nothing in it: back to plain text, the opening kept.
    Opening,
    /// ⇥ past the last blank, or ⇧⇥ before the first: the sheet's own order goes on.
    Leave { back: bool },
    /// ⌘V: the sheet reads the clipboard into the blank.
    Paste,
}

#[derive(Clone)]
pub struct Story {
    pub kind: usize,
    /// What each piece holds: the words of a blank, nothing for text and choices.
    pub values: Vec<String>,
    /// The piece that has the keys: always a blank or a choice.
    pub at: usize,
    /// Which word of a choice stands: 0 for the first.
    pub choice: usize,
    /// The row marked in the feature list.
    pub row: usize,
    /// ↓ or ↑ moved the mark, so ↵ takes the row even with nothing typed.
    pub moved: bool,
    /// Esc put the list away until a letter is typed or the keys move on.
    pub shut: bool,
}

impl Story {
    pub fn new(kind: usize) -> Story {
        let pieces = &kinds()[kind].pieces;
        let first = pieces.iter().position(|piece| !matches!(piece, Piece::Text(_))).unwrap_or(0);
        Story { kind, values: vec![String::new(); pieces.len()], at: first, choice: 0, row: 0, moved: false, shut: false }
    }

    pub fn kind(&self) -> &'static StoryKind {
        &kinds()[self.kind]
    }

    pub fn piece(&self) -> &'static Piece {
        &self.kind().pieces[self.at]
    }

    /// The word of the choice that stands.
    pub fn chosen(&self, first: &'static str, second: &'static str) -> &'static str {
        if self.choice == 0 {
            first
        } else {
            second
        }
    }

    fn stand(&mut self, at: usize) {
        self.at = at;
        self.row = 0;
        self.moved = false;
        self.shut = false;
    }

    /// The next piece a hand can type into, or none after the last.
    fn after(&self) -> Option<usize> {
        let pieces = &self.kind().pieces;
        (self.at + 1..pieces.len()).find(|&at| !matches!(pieces[at], Piece::Text(_)))
    }

    fn before(&self) -> Option<usize> {
        let pieces = &self.kind().pieces;
        (0..self.at).rev().find(|&at| !matches!(pieces[at], Piece::Text(_)))
    }

    /// A blank was pressed: it has the keys.
    pub fn go_to(&mut self, at: usize) {
        if at < self.values.len() && !matches!(self.kind().pieces[at], Piece::Text(_)) {
            self.stand(at);
        }
    }

    /// Whether a hand has typed into any blank.
    pub fn has_words(&self) -> bool {
        self.kind().pieces.iter().zip(&self.values).any(|(piece, value)| matches!(piece, Piece::Blank(_) | Piece::Feature(_)) && !value.trim().is_empty())
    }

    /// The sentence as words: the opening, then each blank that holds something with the text before it. A blank left
    /// empty leaves its text out, and so does the end, which stays only where something follows the opening.
    pub fn words(&self) -> String {
        if !self.has_words() {
            return String::new();
        }
        let kind = self.kind();
        let mut out = kind.opening.to_string();
        let mut glue = String::new();
        for (piece, value) in kind.pieces.iter().zip(&self.values) {
            match piece {
                Piece::Text(text) => glue.push_str(text),
                Piece::Choice(first, second) => {
                    out.push_str(&glue);
                    out.push_str(self.chosen(first, second));
                    glue.clear();
                }
                Piece::Blank(_) | Piece::Feature(_) => {
                    let value = value.trim();
                    if !value.is_empty() {
                        // The first words follow the opening with a space, whatever stood before them.
                        out.push_str(if out == kind.opening { " " } else { &glue });
                        out.push_str(value);
                    }
                    glue.clear();
                }
            }
        }
        out.push_str(glue.trim_end());
        out
    }

    /// The words typed into the first feature blank.
    pub fn feature(&self) -> Option<&str> {
        self.kind().pieces.iter().zip(&self.values).find_map(|(piece, value)| (matches!(piece, Piece::Feature(_)) && !value.trim().is_empty()).then_some(value.trim()))
    }

    /// The line under the sentence: what it starts, and the feature's page where a feature of the catalog is named.
    pub fn starts(&self, catalog: &[FeatureChoice]) -> String {
        let kind = self.kind();
        let says = if self.choice == 1 { kind.later.unwrap_or(kind.starts) } else { kind.starts };
        match self.feature().filter(|named| catalog.iter().any(|one| one.title == *named)) {
            Some(title) => format!("Starts: {says} · {title} page"),
            None => format!("Starts: {says}"),
        }
    }

    /// The rows of the feature list for the blank that has the keys: the kind's extra row, then the catalog's features
    /// whose title or area holds what was typed. None where the keys are not on a feature blank.
    pub fn list(&self, catalog: &[FeatureChoice]) -> Option<Vec<ListRow>> {
        let Piece::Feature(extra) = self.piece() else { return None };
        let typed = self.values[self.at].trim().to_lowercase();
        let rows = extra
            .iter()
            .map(|title| ListRow { title: (*title).into(), area: SharedString::default() })
            .chain(catalog.iter().map(|one| ListRow { title: one.title.clone(), area: one.area.clone() }))
            .filter(|row| typed.is_empty() || row.title.to_lowercase().contains(&typed) || row.area.to_lowercase().contains(&typed))
            .take(LIST_ROWS)
            .collect();
        Some(rows)
    }

    /// The feature list is up: the keys are on a feature blank, which has rows, and Esc has not put it away.
    pub fn list_open(&self, catalog: &[FeatureChoice]) -> bool {
        !self.shut && self.list(catalog).is_some_and(|rows| !rows.is_empty())
    }

    /// Take the marked row into the blank, and go on to the next one.
    fn take(&mut self, catalog: &[FeatureChoice]) {
        if let Some(row) = self.list(catalog).and_then(|rows| rows.into_iter().nth(self.row)) {
            self.values[self.at] = row.title.to_string();
        }
        match self.after() {
            Some(next) => self.stand(next),
            None => self.shut = true,
        }
    }

    /// A row of the list pressed.
    pub fn take_row(&mut self, row: usize, catalog: &[FeatureChoice]) {
        self.row = row;
        self.take(catalog);
    }

    /// The letters a blank takes, as typed.
    pub fn typed(&mut self, text: &str) {
        if matches!(self.piece(), Piece::Blank(_) | Piece::Feature(_)) {
            self.values[self.at].push_str(text);
            self.row = 0;
            self.moved = false;
            self.shut = false;
        }
    }

    /// One key, with the modifiers held and the character it types.
    pub fn key(&mut self, key: &str, shift: bool, platform: bool, other: bool, typed: Option<&str>, catalog: &[FeatureChoice]) -> Keyed {
        if platform && key == "enter" {
            return Keyed::Start;
        }
        if platform && key == "v" {
            return if matches!(self.piece(), Piece::Choice(..)) { Keyed::Taken } else { Keyed::Paste };
        }
        // A chord of the sheet's or the window's goes on: ⌘N, ⌥1, ⌃-anything.
        if platform || other {
            return Keyed::Ignored;
        }
        let list = self.list_open(catalog);
        let value_held = !self.values[self.at].trim().is_empty();
        match key {
            "escape" if list => {
                self.shut = true;
                Keyed::Taken
            }
            "escape" => Keyed::Plain,
            "enter" if shift => Keyed::Taken,
            "tab" if shift => match self.before() {
                Some(at) => {
                    self.stand(at);
                    Keyed::Taken
                }
                None => Keyed::Leave { back: true },
            },
            "tab" | "enter" if list && (value_held || self.moved) => {
                self.take(catalog);
                Keyed::Changed
            }
            "tab" | "enter" => match self.after() {
                Some(at) => {
                    self.stand(at);
                    Keyed::Taken
                }
                None if key == "tab" => Keyed::Leave { back: false },
                None => Keyed::Taken,
            },
            "down" | "up" if list => {
                let len = self.list(catalog).map_or(0, |rows| rows.len()).max(1);
                self.row = if key == "down" { (self.row + 1) % len } else { (self.row + len - 1) % len };
                self.moved = true;
                Keyed::Taken
            }
            "space" | "left" | "right" if matches!(self.piece(), Piece::Choice(..)) => {
                self.choice = 1 - self.choice;
                Keyed::Changed
            }
            "backspace" if matches!(self.piece(), Piece::Choice(..)) || self.values[self.at].is_empty() => match self.before() {
                Some(at) => {
                    self.stand(at);
                    Keyed::Taken
                }
                None => Keyed::Opening,
            },
            "backspace" => {
                self.values[self.at].pop();
                self.row = 0;
                self.moved = false;
                self.shut = false;
                Keyed::Changed
            }
            "left" | "right" | "up" | "down" | "home" | "end" => Keyed::Taken,
            _ => match typed.filter(|text| !text.chars().any(char::is_control)) {
                Some(_) if matches!(self.piece(), Piece::Choice(..)) => Keyed::Taken,
                Some(text) => {
                    self.typed(text);
                    Keyed::Changed
                }
                None => Keyed::Ignored,
            },
        }
    }
}
