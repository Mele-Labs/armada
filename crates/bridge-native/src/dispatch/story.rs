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
    /// The title of its area, read beside it and searched with it. Empty on a project's row.
    pub area: SharedString,
    /// The feature's slug, which a start names; on a project's row, Fleet's id for the project.
    pub slug: SharedString,
    pub kind: Kind,
}

/// What a row of the feature blank stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Feature,
    /// A row of his projects, offered while no project is chosen: taking it chooses the project and does not fill the blank.
    Project,
    /// An area of the project, for a change to something the catalog does not hold: its title fills the blank, and the slug is the area's.
    Area,
}

/// The area pending features are listed under, which is no area of the catalog.
const PENDING_AREA: &str = "pending";

/// A project's features as the list offers them, in the library's order, then its areas. A feature the Operator has yet to bring to
/// him is no feature of the catalog, and the area it waits under is no area.
pub fn catalog_of(library: &ipc::FeatureLibrary) -> Vec<FeatureChoice> {
    let features = library.areas.iter().flat_map(|area| {
        area.features.iter().filter(|row| row.state != ipc::FeatureState::Pending).map(|row| FeatureChoice { title: row.title.clone().into(), area: area.title.clone().into(), slug: row.slug.clone().into(), kind: Kind::Feature })
    });
    let areas = library.areas.iter().filter(|area| area.slug != PENDING_AREA).map(|area| FeatureChoice { title: area.title.clone().into(), area: SharedString::default(), slug: area.slug.clone().into(), kind: Kind::Area });
    features.chain(areas).collect()
}

/// A row of the feature list: the feature, or the kind's own extra row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRow {
    pub title: SharedString,
    /// The area, empty on an extra row.
    pub area: SharedString,
    /// Fleet's id for the project, where the row is one of his projects.
    pub project: Option<SharedString>,
    /// The row is an area of the project, not a feature: the list names it so where it draws the area.
    pub area_row: bool,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// A row of his projects was taken, by Fleet's id: the sheet chooses it, and the blank goes on listing, now its features.
    Project(SharedString),
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

    /// The feature of the catalog the first feature blank names, by its title.
    fn named<'a>(&self, catalog: &'a [FeatureChoice]) -> Option<&'a FeatureChoice> {
        let named = self.feature()?;
        catalog.iter().find(|one| one.kind == Kind::Feature && one.title == named)
    }

    /// The area of the catalog the first feature blank names, by its title, where no feature has that title.
    fn named_area<'a>(&self, catalog: &'a [FeatureChoice]) -> Option<&'a FeatureChoice> {
        let named = self.feature().filter(|_| self.named(catalog).is_none())?;
        catalog.iter().find(|one| one.kind == Kind::Area && one.title == named)
    }

    /// The words a start sends: the sentence, and a last line `Feature: <slug>` where the blank holds a feature of the catalog,
    /// which Fleet hands the Job the page of. Where it holds an area, the line names the area and tells the Job to name the
    /// feature itself. Words typed that are none of them add no line.
    pub fn brief(&self, catalog: &[FeatureChoice]) -> String {
        let words = self.words();
        if words.is_empty() {
            return words;
        }
        match (self.named(catalog), self.named_area(catalog)) {
            (Some(one), _) => format!("{words}\nFeature: {}", one.slug),
            (None, Some(area)) => format!("{words}\nArea: {} ({}). Its feature is not in the catalogue: name one and write it as the Feature: line of your commits.", area.title, area.slug),
            _ => words,
        }
    }

    /// The line under the sentence: what it starts, and the feature's page where a feature of the catalog is named.
    pub fn starts(&self, catalog: &[FeatureChoice]) -> String {
        let kind = self.kind();
        let says = if self.choice == 1 { kind.later.unwrap_or(kind.starts) } else { kind.starts };
        match self.feature() {
            Some(title) if self.named(catalog).is_some() => format!("Starts: {says} · {title} page"),
            Some(title) if self.named_area(catalog).is_some() => format!("Starts: {says} · {title} area"),
            _ => format!("Starts: {says}"),
        }
    }

    /// The rows of the feature list for the blank that has the keys: the kind's extra row, then the catalog's features
    /// whose title or area holds what was typed. None where the keys are not on a feature blank.
    pub fn list(&self, catalog: &[FeatureChoice]) -> Option<Vec<ListRow>> {
        let Piece::Feature(extra) = self.piece() else { return None };
        let typed = self.values[self.at].trim().to_lowercase();
        let rows = extra
            .iter()
            .map(|title| ListRow { title: (*title).into(), area: SharedString::default(), project: None, area_row: false })
            .chain(catalog.iter().map(|one| ListRow { title: one.title.clone(), area: one.area.clone(), project: (one.kind == Kind::Project).then(|| one.slug.clone()), area_row: one.kind == Kind::Area }))
            .filter(|row| typed.is_empty() || row.title.to_lowercase().contains(&typed) || row.area.to_lowercase().contains(&typed))
            .take(LIST_ROWS)
            .collect();
        Some(rows)
    }

    /// The feature list is up: the keys are on a feature blank, which has rows, and Esc has not put it away.
    pub fn list_open(&self, catalog: &[FeatureChoice]) -> bool {
        !self.shut && self.list(catalog).is_some_and(|rows| !rows.is_empty())
    }

    /// Take the marked row into the blank, and go on to the next one. A project's row fills nothing and stays: the blank is
    /// to list that project's features, and the project's id is given back for the sheet to choose.
    fn take(&mut self, catalog: &[FeatureChoice]) -> Option<SharedString> {
        let row = self.list(catalog).and_then(|rows| rows.into_iter().nth(self.row));
        if let Some(project) = row.as_ref().and_then(|row| row.project.clone()) {
            self.values[self.at].clear();
            let at = self.at;
            self.stand(at);
            return Some(project);
        }
        if let Some(row) = row {
            self.values[self.at] = row.title.to_string();
        }
        match self.after() {
            Some(next) => self.stand(next),
            None => self.shut = true,
        }
        None
    }

    /// A row of the list pressed: the project it chose, where it was one.
    pub fn take_row(&mut self, row: usize, catalog: &[FeatureChoice]) -> Option<SharedString> {
        self.row = row;
        self.take(catalog)
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
                match self.take(catalog) {
                    Some(project) => Keyed::Project(project),
                    None => Keyed::Changed,
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    const BUG: usize = 0;
    const IDEA: usize = 2;

    fn catalog() -> Vec<FeatureChoice> {
        let one = |title: &str, area: &str| FeatureChoice { title: title.to_string().into(), area: area.to_string().into(), slug: title.to_lowercase().into(), kind: Kind::Feature };
        vec![one("Login", "Account"), one("Checkout", "Payments"), one("Billing", "Payments")]
    }

    /// One key with no modifiers, as the sheet hands it on.
    fn press(story: &mut Story, key: &str, catalog: &[FeatureChoice]) -> Keyed {
        story.key(key, false, false, false, None, catalog)
    }

    /// Letters typed one key at a time into the blank that has the keys.
    fn write(story: &mut Story, text: &str, catalog: &[FeatureChoice]) {
        for letter in text.chars() {
            let letter = letter.to_string();
            story.key(&letter, false, false, false, Some(&letter), catalog);
        }
    }

    fn titles(rows: Option<Vec<ListRow>>) -> Vec<String> {
        rows.expect("the keys are on a feature blank").into_iter().map(|row| row.title.to_string()).collect()
    }

    #[test]
    fn openings_of_wh_are_when_i_what_if_and_why_is_whatever_the_capitals() {
        let named = |typed: &str| openings(typed).into_iter().map(|at| kinds()[at].opening).collect::<Vec<_>>();
        assert_eq!(named("wh"), ["When I", "What if", "Why is"]);
        assert_eq!(named("WH"), ["When I", "What if", "Why is"]);
        assert_eq!(named("wHe"), ["When I"]);
        assert_eq!(named("when i"), ["When I"]);
        assert!(named("When I clicked").is_empty(), "words past the opening match none");
        assert!(named("xyz").is_empty());
        assert!(named("").is_empty());
        assert!(named("when\nI").is_empty(), "words that span lines match none");
    }

    #[test]
    fn tab_and_enter_go_on_to_the_next_blank_and_shift_tab_goes_back() {
        for go in ["tab", "enter"] {
            let mut story = Story::new(BUG);
            assert_eq!(story.at, 0, "the caret starts in the first blank");
            assert_eq!(press(&mut story, go, &[]), Keyed::Taken);
            assert_eq!(story.piece(), &Piece::Feature(None), "{go} goes to the feature blank");
            assert_eq!(press(&mut story, go, &[]), Keyed::Taken);
            assert_eq!(story.piece(), &Piece::Blank("what happens"), "{go} goes to the last blank");
            assert_eq!(story.at, 4);
        }
        let mut story = Story::new(BUG);
        story.go_to(4);
        assert_eq!(story.at, 4);
        assert_eq!(story.key("tab", true, false, false, None, &[]), Keyed::Taken);
        assert_eq!(story.at, 2);
        assert_eq!(story.key("tab", true, false, false, None, &[]), Keyed::Taken);
        assert_eq!(story.at, 0);
        assert_eq!(story.key("tab", true, false, false, None, &[]), Keyed::Leave { back: true }, "⇧⇥ before the first blank leaves the sentence");
        story.go_to(4);
        assert_eq!(press(&mut story, "tab", &[]), Keyed::Leave { back: false }, "⇥ past the last blank leaves the sentence");
        assert_eq!(press(&mut story, "enter", &[]), Keyed::Taken, "↵ past the last blank stays");
        assert_eq!(story.at, 4);
    }

    #[test]
    fn backspace_in_an_empty_first_blank_gives_back_the_opening_and_esc_gives_plain_words() {
        let mut story = Story::new(BUG);
        write(&mut story, "ab", &[]);
        assert_eq!(press(&mut story, "backspace", &[]), Keyed::Changed, "a letter is taken back first");
        assert_eq!(story.values[0], "a");
        assert_eq!(press(&mut story, "backspace", &[]), Keyed::Changed);
        assert_eq!(press(&mut story, "backspace", &[]), Keyed::Opening, "nothing left in the first blank: plain words, opening kept");
        assert_eq!(press(&mut story, "escape", &[]), Keyed::Plain);
        // Further on, an empty blank steps back instead.
        story.go_to(2);
        assert_eq!(press(&mut story, "backspace", &[]), Keyed::Taken);
        assert_eq!(story.at, 0);
    }

    #[test]
    fn the_feature_blank_filters_the_catalog_by_title_or_area_and_enter_picks_the_marked_row() {
        let catalog = catalog();
        let mut story = Story::new(BUG);
        story.go_to(2);
        assert_eq!(titles(story.list(&catalog)), ["Login", "Checkout", "Billing"], "nothing typed: every feature");
        write(&mut story, "LOG", &catalog);
        assert_eq!(titles(story.list(&catalog)), ["Login"], "by title, whatever the capitals");
        story.values[2].clear();
        write(&mut story, "pay", &catalog);
        assert_eq!(titles(story.list(&catalog)), ["Checkout", "Billing"], "by the title of its area");
        assert_eq!(press(&mut story, "down", &catalog), Keyed::Taken);
        assert_eq!(story.row, 1);
        assert_eq!(press(&mut story, "enter", &catalog), Keyed::Changed);
        assert_eq!(story.values[2], "Billing", "↵ took the marked row");
        assert_eq!(story.at, 4, "and went on to the next blank");

        // Nothing typed, the mark moved: ↵ still takes the row.
        let mut story = Story::new(BUG);
        story.go_to(2);
        press(&mut story, "down", &catalog);
        assert_eq!(press(&mut story, "enter", &catalog), Keyed::Changed);
        assert_eq!(story.values[2], "Checkout");
    }

    #[test]
    fn words_compose_the_sentence_with_the_blanks_filled_and_the_unfilled_ones_dropped() {
        let mut story = Story::new(BUG);
        assert_eq!(story.words(), "", "no blank filled: no words, not even the opening");
        story.values[0] = "click Pay".into();
        assert_eq!(story.words(), "When I click Pay");
        story.values[4] = "it crashes".into();
        assert_eq!(story.words(), "When I click Pay, it crashes", "the empty feature blank takes its text with it");
        story.values[2] = "Checkout".into();
        assert_eq!(story.words(), "When I click Pay in Checkout, it crashes");
        story.values[0].clear();
        assert_eq!(story.words(), "When I Checkout, it crashes", "the first blank left empty: the next words follow the opening");

        let mut story = Story::new(IDEA);
        story.values[0] = "Armada".into();
        story.values[2] = "sing".into();
        assert_eq!(story.words(), "What if Armada could sing? now");
        story.choice = 1;
        assert_eq!(story.words(), "What if Armada could sing? later");
    }

    #[test]
    fn the_brief_ends_with_the_page_of_a_feature_of_the_catalog_and_typed_words_add_no_line() {
        let catalog = catalog();
        let mut story = Story::new(BUG);
        story.values[0] = "click Pay".into();
        assert_eq!(story.brief(&catalog), "When I click Pay", "no feature named");
        story.values[2] = "Checkout".into();
        assert_eq!(story.brief(&catalog), "When I click Pay in Checkout\nFeature: checkout");
        story.values[2] = "the pay button".into();
        assert_eq!(story.brief(&catalog), "When I click Pay in the pay button", "words typed that are no title add no line");
        story.values[2] = "Checkout".into();
        assert_eq!(story.brief(&[]), "When I click Pay in Checkout", "a catalog not yet in names nothing");
        let project = FeatureChoice { title: "Checkout".into(), area: SharedString::default(), slug: "storefront".into(), kind: Kind::Project };
        assert_eq!(story.brief(&[project]), "When I click Pay in Checkout", "a project's row is no feature");
    }

    fn library_catalog() -> Vec<FeatureChoice> {
        catalog_of(&crate::features::mock::library())
    }

    #[test]
    fn the_catalog_lists_areas_after_features_and_never_the_pending_one() {
        let catalog = library_catalog();
        let kinds: Vec<Kind> = catalog.iter().map(|one| one.kind).collect();
        let first_area = kinds.iter().position(|kind| *kind == Kind::Area).expect("areas are listed");
        assert!(kinds[..first_area].iter().all(|kind| *kind == Kind::Feature) && kinds[first_area..].iter().all(|kind| *kind == Kind::Area), "features, then areas");
        let areas: Vec<(&str, &str)> = catalog.iter().filter(|one| one.kind == Kind::Area).map(|one| (one.title.as_ref(), one.slug.as_ref())).collect();
        assert_eq!(areas, [("Running a Job", "running-a-job"), ("Job thread and terminal", "job-thread-and-terminal"), ("Pull requests and review", "pull-requests-and-review")], "no pending area");
        assert!(catalog.iter().all(|one| one.title != "Agent notes on a diff"), "a pending feature is still no feature");
    }

    #[test]
    fn area_rows_filter_by_typed_words_and_are_marked_as_areas() {
        let catalog = library_catalog();
        let mut story = Story::new(BUG);
        story.go_to(2);
        let rows = story.list(&catalog).expect("on a feature blank");
        assert!(rows.iter().filter(|row| row.area_row).count() == 3 && rows.iter().rev().take(3).all(|row| row.area_row), "area rows come last");
        write(&mut story, "pull requests", &catalog);
        let rows = story.list(&catalog).expect("on a feature blank");
        assert_eq!(
            rows.iter().map(|row| (row.title.to_string(), row.area_row)).collect::<Vec<_>>(),
            [("Pull request panel".to_string(), false), ("Reviews".to_string(), false), ("Pull requests and review".to_string(), true)],
            "its features match by their area, and the area by its own title"
        );
        let mut story = Story::new(BUG);
        story.go_to(2);
        write(&mut story, "thread", &catalog);
        assert_eq!(titles(story.list(&catalog)), ["Thread and composer", "Terminal Jobs", "Sessions without a Job", "Job thread and terminal"], "features, then the area");
    }

    #[test]
    fn taking_an_area_fills_its_title_and_the_brief_and_starts_line_name_it() {
        let catalog = library_catalog();
        let mut story = Story::new(BUG);
        story.values[0] = "click Pay".into();
        story.go_to(2);
        write(&mut story, "running", &catalog);
        let at = story.list(&catalog).expect("on a feature blank").iter().position(|row| row.area_row).expect("the area is listed");
        assert_eq!(story.take_row(at, &catalog), None, "an area is no project");
        assert_eq!(story.values[2], "Running a Job");
        assert_eq!(story.at, 4, "taking an area goes on, as taking a feature does");
        assert_eq!(
            story.brief(&catalog),
            "When I click Pay in Running a Job\nArea: Running a Job (running-a-job). Its feature is not in the catalogue: name one and write it as the Feature: line of your commits."
        );
        assert_eq!(story.starts(&catalog), format!("{} · Running a Job area", story.starts(&[])));
    }

    #[test]
    fn a_feature_still_gives_its_slug_and_typed_words_give_no_line_beside_areas() {
        let catalog = library_catalog();
        let mut story = Story::new(BUG);
        story.values[0] = "click Pay".into();
        story.values[2] = "Start a Job".into();
        assert_eq!(story.brief(&catalog), "When I click Pay in Start a Job\nFeature: start-a-job");
        assert!(story.starts(&catalog).ends_with("· Start a Job page"));
        story.values[2] = "the pay button".into();
        assert_eq!(story.brief(&catalog), "When I click Pay in the pay button");
        assert!(!story.starts(&catalog).contains('·'));
    }

    #[test]
    fn a_project_row_taken_fills_nothing_stays_in_the_blank_and_names_the_project() {
        let projects = vec![
            FeatureChoice { title: "Storefront".into(), area: SharedString::default(), slug: "storefront".into(), kind: Kind::Project },
            FeatureChoice { title: "Docs".into(), area: SharedString::default(), slug: "docs".into(), kind: Kind::Project },
        ];
        let mut story = Story::new(BUG);
        story.go_to(2);
        write(&mut story, "do", &projects);
        assert_eq!(titles(story.list(&projects)), ["Docs"]);
        assert_eq!(press(&mut story, "enter", &projects), Keyed::Project("docs".into()));
        assert_eq!(story.values[2], "", "the project is no feature");
        assert_eq!(story.at, 2, "the blank stays, to list the project's features");

        let mut story = Story::new(BUG);
        story.go_to(2);
        assert_eq!(story.take_row(0, &projects), Some("storefront".into()), "a row pressed does the same");
        assert_eq!(story.at, 2);
    }
}
