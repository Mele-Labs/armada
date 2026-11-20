//! The bands over a Job's right column: the progress strip (`column_strip.rs`) and Needs you. A band is not a tab. Each has
//! an x that hides it for every Job, and + lists a hidden band under Bands to bring it back. The hide is one preference of
//! the machine, kept in a file beside the Jobs list's own (`native-column-bands`) and read once when the view is made.
//!
//! Needs you is one list under three chips. Open is what Fleet's Needs you holds for the Job, Done what he closed from it
//! and Deferred what the agent kept for later. It is drawn only while something waits on him, which is Open or Deferred
//! holding a row; Done alone draws nothing. The chip he pressed stands until he presses another.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};

use super::dock::Thing;
use super::rows::{age, glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A band over the column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Band {
    Strip,
    Needs,
}

impl Band {
    pub(super) const ALL: [Band; 2] = [Band::Strip, Band::Needs];

    /// What the file and the + menu's entry name it by.
    pub(super) fn id(self) -> &'static str {
        match self {
            Band::Strip => "strip",
            Band::Needs => "needs",
        }
    }

    /// The entry's id in + (an entry's id is `&'static str`).
    pub(super) fn entry(self) -> &'static str {
        match self {
            Band::Strip => "band:strip",
            Band::Needs => "band:needs",
        }
    }

    pub(super) fn word(self) -> &'static str {
        match self {
            Band::Strip => "Progress strip",
            Band::Needs => "Needs you",
        }
    }

    pub(super) fn glyph(self) -> &'static str {
        match self {
            Band::Strip => "milestone",
            Band::Needs => "user-check",
        }
    }

    /// What the x says it does.
    fn hides(self) -> &'static str {
        match self {
            Band::Strip => "Hide the strip for every Job",
            Band::Needs => "Hide Needs you for every Job",
        }
    }
}

/// The bands he hid, for every Job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Hidden(BTreeSet<Band>);

impl Hidden {
    /// The file's lines: each is the id of a band that is hidden. A line that names no band is left out.
    pub(super) fn parse(text: &str) -> Hidden {
        Hidden(text.lines().filter_map(|line| Band::ALL.into_iter().find(|band| band.id() == line.trim())).collect())
    }

    pub(super) fn spell(&self) -> String {
        self.0.iter().map(|band| format!("{}\n", band.id())).collect()
    }

    pub(super) fn has(&self, band: Band) -> bool {
        self.0.contains(&band)
    }

    /// Hide or show a band. Whether that changed anything.
    pub(super) fn set(&mut self, band: Band, hidden: bool) -> bool {
        if hidden {
            self.0.insert(band)
        } else {
            self.0.remove(&band)
        }
    }

    /// The bands that are hidden, in the order the bands stand.
    pub(super) fn hidden(&self) -> impl Iterator<Item = Band> + '_ {
        self.0.iter().copied()
    }
}

/// Where the choice is kept between runs, beside the list's own.
fn bands_file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-column-bands"))
}

pub(super) fn read_hidden_from(path: &Path) -> Hidden {
    Hidden::parse(&std::fs::read_to_string(path).unwrap_or_default())
}

pub(super) fn write_hidden_to(path: &Path, hidden: &Hidden) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, hidden.spell());
}

/// The bands he hid on earlier runs: none where nothing is kept.
pub(super) fn read_hidden() -> Hidden {
    bands_file().map(|path| read_hidden_from(&path)).unwrap_or_default()
}

/// One of Needs you's three lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Chip {
    Open,
    Done,
    Deferred,
}

impl Chip {
    pub(super) const ALL: [Chip; 3] = [Chip::Open, Chip::Done, Chip::Deferred];

    pub(super) fn word(self) -> &'static str {
        match self {
            Chip::Open => "Open",
            Chip::Done => "Done",
            Chip::Deferred => "Deferred",
        }
    }
}

/// Which of the three lists hold a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Held {
    pub open: bool,
    pub done: bool,
    pub deferred: bool,
}

impl Held {
    fn of(self, chip: Chip) -> bool {
        match chip {
            Chip::Open => self.open,
            Chip::Done => self.done,
            Chip::Deferred => self.deferred,
        }
    }
}

/// The chip Needs you stands on, or None where it is not drawn. It is drawn only while something waits on him: an Open or
/// a Deferred row. With no chip pressed it stands on the first of those that holds one; a chip he pressed stands.
pub(super) fn standing(pressed: Option<Chip>, held: Held) -> Option<Chip> {
    if !held.open && !held.deferred {
        return None;
    }
    pressed.or_else(|| [Chip::Open, Chip::Deferred].into_iter().find(|chip| held.of(*chip)))
}

/// What a chip lists: its own list and no other, which is the only one made.
pub(super) fn listed<T>(chip: Chip, open: impl FnOnce() -> Vec<T>, done: impl FnOnce() -> Vec<T>, deferred: impl FnOnce() -> Vec<T>) -> Vec<T> {
    match chip {
        Chip::Open => open(),
        Chip::Done => done(),
        Chip::Deferred => deferred(),
    }
}

/// A band's frame: a raised block with a hairline, in the column's gutter.
pub(super) fn band_frame(look: &Look, id: impl Into<gpui::ElementId>) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_RAISED))
}

impl JobView {
    /// Whether the column's bands are for this pane: the first pane of the column beside the thread.
    pub(super) fn bands_pane(&self, pane: &str) -> bool {
        let arrangement = self.arrangement();
        let thread = super::pane_tree::holder_of(&arrangement, "thread").map(|held| held.id.clone());
        super::panes_place::pane_for(&arrangement, super::view_tabs::ViewPane::Top).is_some_and(|top| top == pane && thread.as_deref() != Some(pane))
    }

    /// + in a pane of the column with the bands he hid listed first, under Bands, and the tabs under Tabs.
    pub(super) fn with_bands(&self, pane: &str, tabs: Vec<crate::dropdown::Entry>) -> Vec<crate::dropdown::Entry> {
        use crate::dropdown::Entry;
        let hidden: Vec<Band> = if self.bands_pane(pane) { self.bands.hidden().collect() } else { Vec::new() };
        if hidden.is_empty() {
            return tabs;
        }
        let bands = hidden.into_iter().map(|band| Entry::Item { id: band.entry(), label: band.word().into(), icon: Some(band.glyph()), danger: false, key: None, off: None });
        std::iter::once(Entry::Label("Bands")).chain(bands).chain(std::iter::once(Entry::Label("Tabs"))).chain(tabs).collect()
    }

    /// A pick in + that is a band coming back. Whether it was one.
    pub(super) fn band_picked(&mut self, entry: &str, cx: &mut Context<Self>) -> bool {
        let Some(band) = Band::ALL.into_iter().find(|band| band.entry() == entry) else { return false };
        self.set_band(band, false, cx);
        true
    }

    /// Whether the band is drawn: not one he hid.
    pub(super) fn band_shown(&self, band: Band) -> bool {
        !self.bands.has(band)
    }

    /// Hide or show a band for every Job, and keep it so.
    pub(super) fn set_band(&mut self, band: Band, hidden: bool, cx: &mut Context<Self>) {
        if !self.bands.set(band, hidden) {
            return;
        }
        // A mock run keeps nothing, as the list's own store does.
        if self.fleet.is_some() {
            if let Some(path) = bands_file() {
                write_hidden_to(&path, &self.bands);
            }
        }
        cx.notify();
    }

    /// The x on a band. It hides the band, and the keys go to the pane's + , which offers it back.
    pub(super) fn band_hide(&self, job: &SharedString, band: Band, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, pane) = (job.clone(), super::panes_place::pane_for(&self.arrangement(), super::view_tabs::ViewPane::Top));
        labelled(
            super::tabs::frame(look, SharedString::from(format!("band-hide:{}:{job}", band.id())), "x")
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .tooltip(tip(band.hides()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.set_band(band, true, cx);
                    if let Some(pane) = pane.clone() {
                        let handle = this.add_handle(&job, &pane, cx);
                        window.focus(&handle, cx);
                    }
                })),
            Role::Button,
            format!("Hide {}", band.word()),
        )
        .into_any_element()
    }

    /// The bands over the column, in the order they stand, for a pane that holds them. Nothing for a Job with none to draw.
    pub(super) fn column_bands(&self, job: &SharedString, pane: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.bands_pane(pane) {
            return None;
        }
        let strip = self.band_shown(Band::Strip).then(|| self.strip_for(job, cx)).flatten().map(|strip| self.column_strip(job, &strip, look, cx));
        let needs = self.band_shown(Band::Needs).then(|| self.column_needs(job, look, cx)).flatten();
        if strip.is_none() && needs.is_none() {
            return None;
        }
        Some(
            div()
                .id(SharedString::from(format!("column-bands:{job}")))
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_2))
                .bg(look.color(token::BG_SUNKEN))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_GLASS))
                .children(strip)
                .children(needs)
                .into_any_element(),
        )
    }

    /// What each chip of Needs you holds for the Job.
    fn needs_held(&self, id: &SharedString, cx: &Context<Self>) -> Held {
        let Some(job) = self.record(id, cx) else { return Held::default() };
        Held {
            open: !self.things(id, cx).0.is_empty(),
            done: !job.closed.is_empty(),
            deferred: job.later.iter().any(|later| !later.confirmed && !self.later_settled(&later.id)),
        }
    }

    /// Needs you: the head with its chips and x, then the list under the chip that stands. None where nothing waits on him.
    fn column_needs(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let pressed = self.pages.get(id).and_then(|page| page.side.needs_chip);
        let held = self.needs_held(id, cx);
        let chip = standing(pressed, held)?;
        let rows: Vec<AnyElement> = listed(
            chip,
            || self.things(id, cx).0.iter().map(|thing: &Thing| self.thing_row(id, thing, look, cx)).collect(),
            || self.closed_rows(id, look, cx),
            || self.deferred_rows(id, true, look, cx),
        );
        let chips = Chip::ALL.into_iter().map(|one| {
            let on = one == chip;
            let accent = look.color(token::ACCENT);
            let (ink, ground, edge) = if on { (accent, look.color(token::BG_RAISED).blend(accent.opacity(0.14)), accent) } else { (look.color(token::FG_MUTED), look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)) };
            let job = id.clone();
            labelled(
                div()
                    .id(SharedString::from(format!("needs-chip:{}:{id}", one.word())))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(edge)
                    .bg(ground)
                    .text_color(ink)
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(if on { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
                    .cursor_pointer()
                    .tab_index(0)
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .child(one.word())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(page) = this.pages.get_mut(&job) {
                            page.side.needs_chip = Some(one);
                        }
                        cx.notify();
                    })),
                Role::RadioButton,
                one.word(),
            )
        });
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_none().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Needs you"))
            .child(labelled(div().id(SharedString::from(format!("needs-chips:{id}"))).flex().items_center().gap(look.length(token::SPACE_1)).children(chips), Role::RadioGroup, "Needs you"))
            .child(div().flex_1())
            .child(self.band_hide(id, Band::Needs, look, cx));
        Some(
            labelled(
                band_frame(look, SharedString::from(format!("band-needs:{id}"))).child(head).child(div().flex().flex_col().gap(look.length(token::SPACE_1)).children(rows)),
                Role::Group,
                "Needs you",
            )
            .into_any_element(),
        )
    }

    /// What he closed from the Job's Needs you: its words and when.
    fn closed_rows(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let now = Local::now();
        job.closed
            .iter()
            .map(|closed| {
                let when = age(closed.at.as_str(), now);
                labelled(
                    div()
                        .id(SharedString::from(format!("needs-closed:{}", closed.item.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .min_h(look.length(token::H_ROW))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().id(SharedString::from(format!("needs-closed-mark:{}", closed.item.id))).flex_none().tooltip(tip("Done")).child(glyph(look, "circle-check", Size::Small, look.color(token::STEP_ADVANCED))))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(closed.item.text.clone())))
                        .children(when.map(|when| div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(when)))),
                    Role::ListItem,
                    closed.item.text.clone(),
                )
                .into_any_element()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("armada-column-bands-{}-{name}", std::process::id()))
    }

    /// Hiding a band for one Job hides it for the next, and the choice is there after a restart.
    #[test]
    fn a_hidden_band_stays_hidden_across_runs() {
        let path = temp("hidden");
        let _ = std::fs::remove_file(&path);
        assert_eq!(read_hidden_from(&path), Hidden::default(), "nothing kept, nothing hidden");
        let mut hidden = Hidden::default();
        assert!(hidden.set(Band::Strip, true));
        assert!(!hidden.set(Band::Strip, true), "hiding it again changes nothing");
        write_hidden_to(&path, &hidden);
        let back = read_hidden_from(&path);
        assert!(back.has(Band::Strip) && !back.has(Band::Needs));
        assert_eq!(back.hidden().collect::<Vec<_>>(), [Band::Strip], "+ lists exactly what was hidden");
        // Brought back from +, it is kept shown.
        let mut shown = back;
        assert!(shown.set(Band::Strip, false));
        write_hidden_to(&path, &shown);
        assert_eq!(read_hidden_from(&path), Hidden::default());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_kept_line_that_names_no_band_is_left_out() {
        assert_eq!(Hidden::parse("needs\nsomething else\n\n strip \n").hidden().collect::<Vec<_>>(), [Band::Strip, Band::Needs]);
    }

    #[test]
    fn needs_you_is_drawn_only_while_something_waits_on_him() {
        assert_eq!(standing(None, Held::default()), None);
        assert_eq!(standing(Some(Chip::Done), Held { done: true, ..Held::default() }), None, "what he closed alone asks nothing");
        assert_eq!(standing(None, Held { open: true, ..Held::default() }), Some(Chip::Open));
        assert_eq!(standing(None, Held { deferred: true, done: true, ..Held::default() }), Some(Chip::Deferred), "the first chip that holds a row, Done last");
        // A chip he pressed stands, even over one that holds more.
        assert_eq!(standing(Some(Chip::Done), Held { open: true, done: true, deferred: false }), Some(Chip::Done));
    }

    #[test]
    fn a_chip_lists_its_own_rows_and_no_other() {
        let pick = |chip| listed(chip, || vec!["a", "b"], || vec!["c"], || vec!["d", "e", "f"]);
        assert_eq!(pick(Chip::Open), ["a", "b"]);
        assert_eq!(pick(Chip::Done), ["c"]);
        assert_eq!(pick(Chip::Deferred), ["d", "e", "f"]);
        // The lists of the chips that were not pressed are never made.
        let made = std::cell::Cell::new(0);
        let counted = || {
            made.set(made.get() + 1);
            vec![1]
        };
        listed(Chip::Done, || panic!("Open was made"), counted, || panic!("Deferred was made"));
        assert_eq!(made.get(), 1);
    }
}
