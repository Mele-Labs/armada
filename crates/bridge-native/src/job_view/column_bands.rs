//! The band over a Job's right column: one card that holds the step line (`column_strip.rs`) and, under a hairline, Needs you.
//! A band is not a tab. Its x, on the step line, hides it for every Job, and + lists it hidden under Bands to bring it back as
//! Step and Needs you. The hide is one preference of the machine, kept in a file beside the Jobs list's own
//! (`native-column-bands`) and read once when the view is made; a machine that hid the strip or Needs you before hides the band.
//! The two halves are drawn by `strip_parts` and `needs_parts`, apart, and joined only in `column_bands`.
//!
//! Needs you is one list under three chips. Open is what Fleet's Needs you holds for the Job, Done what he closed from it
//! and Deferred what the agent kept for later. It stands on Open unless he pressed another chip, and never falls to Done or
//! Deferred by itself: with nothing open, the head and its chips are drawn with no rows under them, and only where Done or
//! Deferred holds a row. The chip he pressed stands for that Job until he presses another, or until a new item waits on him.

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
    Column,
}

impl Band {
    pub(super) const ALL: [Band; 1] = [Band::Column];

    /// What the file and the + menu's entry name it by.
    pub(super) fn id(self) -> &'static str {
        "column"
    }

    /// The entry's id in + (an entry's id is `&'static str`).
    pub(super) fn entry(self) -> &'static str {
        "band:column"
    }

    pub(super) fn word(self) -> &'static str {
        "Step and Needs you"
    }

    pub(super) fn glyph(self) -> &'static str {
        "milestone"
    }

    /// What the x says it does.
    fn hides(self) -> &'static str {
        "Hide Step and Needs you for every Job"
    }
}

/// The bands he hid, for every Job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Hidden(BTreeSet<Band>);

impl Hidden {
    /// The file's lines: each is the id of a band that is hidden. A line that names no band is left out. The strip and Needs
    /// were two bands once, so a machine that hid either has hidden the one band now.
    pub(super) fn parse(text: &str) -> Hidden {
        Hidden(text.lines().filter(|line| matches!(line.trim(), "column" | "strip" | "needs")).map(|_| Band::Column).collect())
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

/// The chip Needs you stands on, or None where it is not drawn. It is drawn while any of the three lists holds a row. It stands
/// on the chip he pressed, else on Open, even where Open is empty: it never falls to Done or Deferred on its own.
pub(super) fn standing(pressed: Option<Chip>, held: Held) -> Option<Chip> {
    if !held.open && !held.done && !held.deferred {
        return None;
    }
    Some(pressed.unwrap_or(Chip::Open))
}

/// A chip he pressed, with the items that were open then. An open item that is not among them is new, and takes the press back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pressed {
    pub chip: Chip,
    pub open: BTreeSet<String>,
}

/// The chip he pressed that still stands: none once an item waits on him that was not open when he pressed it.
pub(super) fn kept(pressed: Option<&Pressed>, open_now: &BTreeSet<String>) -> Option<Chip> {
    pressed.filter(|pressed| open_now.is_subset(&pressed.open)).map(|pressed| pressed.chip)
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

/// The band card over a pane, as two blocks with the column's gap between them: the card and the pane in its own frame under it.
/// The caller sizes the pair; the pane takes the height the card leaves.
pub(super) fn stacked(look: &Look, pane_id: &str, card: AnyElement, pane: AnyElement) -> gpui::Stateful<gpui::Div> {
    div().id(SharedString::from(format!("column-bands:{pane_id}"))).min_h_0().flex().flex_col().gap(look.length(token::GAP_PANEL)).child(card).child(pane)
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

    /// The band over the column, for a pane that holds it: one card of its own with the step line, and under a hairline Needs you. With
    /// nothing waiting on him the card is the step line alone; with no step to say and nothing waiting, there is no card.
    pub(super) fn column_bands(&self, job: &SharedString, pane: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.bands_pane(pane) || !self.band_shown(Band::Column) {
            return None;
        }
        let strip = self.strip_for(job, cx);
        let steps = strip.as_ref().map(|strip| self.strip_parts(job, strip, look, cx));
        // The x stands on the step line; with none, it stands on Needs you's head.
        let needs = self.needs_parts(job, steps.is_none(), look, cx);
        if steps.is_none() && needs.is_none() {
            return None;
        }
        let hairline = (steps.is_some() && needs.is_some()).then(|| div().flex_none().border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)));
        Some(
            labelled(
                band_frame(look, SharedString::from(format!("band-column:{job}"))).children(steps.into_iter().flatten()).children(hairline).children(needs.into_iter().flatten()),
                Role::Group,
                Band::Column.word(),
            )
            .into_any_element(),
        )
    }

    /// What each chip of Needs you holds for the Job.
    pub(super) fn needs_held(&self, id: &SharedString, cx: &gpui::App) -> Held {
        let Some(job) = self.record(id, cx) else { return Held::default() };
        Held {
            open: !self.things(id, cx).0.is_empty() || !self.for_you_beyond_rows(id, cx).is_empty(),
            done: !job.closed.is_empty(),
            deferred: job.later.iter().any(|later| !later.confirmed && !self.later_settled(&later.id)),
        }
    }

    /// The ids of what is open for the Job: the rows of Needs you's Open chip.
    fn open_ids(&self, id: &SharedString, cx: &gpui::App) -> BTreeSet<String> {
        let things = self.things(id, cx).0.into_iter().map(|thing| match thing {
            Thing::Ask(ask) => ask.call.call.clone(),
            Thing::Lapsed(lapsed) => lapsed.id.clone(),
            Thing::Row(row) => row.id.clone(),
            Thing::Sheet(item) => item.id,
        });
        things.chain(self.for_you_beyond_rows(id, cx).into_iter().map(|item| item.id.clone())).collect()
    }

    /// Needs you: the head with its chips, and the x where `hides` says the step line has none, then the list under the chip that
    /// stands, which is none where that list is empty. None where none of the three lists holds a row.
    fn needs_parts(&self, id: &SharedString, hides: bool, look: &Look, cx: &Context<Self>) -> Option<Vec<AnyElement>> {
        let open = self.open_ids(id, cx);
        let pressed = kept(self.pages.get(id).and_then(|page| page.side.needs_chip.as_ref()), &open);
        let held = self.needs_held(id, cx);
        let chip = standing(pressed, held)?;
        // With the panel up, a question is the card in front of the thread; the band does not draw it a second time.
        let up = self.needs_up(id, cx);
        let rows: Vec<AnyElement> = listed(
            chip,
            || self.things(id, cx).0.iter().filter(|thing| !(up && matches!(thing, Thing::Row(row) if super::needs_card::on_card(row)))).map(|thing: &Thing| self.thing_row(id, thing, look, cx)).chain(self.for_you_beyond_row_views(id, look, cx)).collect(),
            || self.closed_rows(id, look, cx),
            || self.deferred_rows(id, true, look, cx),
        );
        let chips = Chip::ALL.into_iter().map(|one| {
            let on = one == chip;
            let accent = look.color(token::ACCENT);
            let (ink, ground, edge) = if on { (accent, look.color(token::BG_RAISED).blend(accent.opacity(0.14)), accent) } else { (look.color(token::FG_MUTED), look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)) };
            let (job, open) = (id.clone(), open.clone());
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
                            page.side.needs_chip = Some(Pressed { chip: one, open: open.clone() });
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
            .children(hides.then(|| self.band_hide(id, Band::Column, look, cx)));
        let list = (!rows.is_empty()).then(|| div().flex().flex_col().gap(look.length(token::SPACE_1)).children(rows).into_any_element());
        Some(std::iter::once(head.into_any_element()).chain(list).collect())
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

    /// Hiding the band for one Job hides it for the next, and the choice is there after a restart.
    #[test]
    fn a_hidden_band_stays_hidden_across_runs() {
        let path = temp("hidden");
        let _ = std::fs::remove_file(&path);
        assert_eq!(read_hidden_from(&path), Hidden::default(), "nothing kept, nothing hidden");
        let mut hidden = Hidden::default();
        assert!(hidden.set(Band::Column, true));
        assert!(!hidden.set(Band::Column, true), "hiding it again changes nothing");
        write_hidden_to(&path, &hidden);
        let back = read_hidden_from(&path);
        assert!(back.has(Band::Column));
        assert_eq!(back.hidden().collect::<Vec<_>>(), [Band::Column], "+ lists exactly what was hidden");
        // Brought back from +, it is kept shown.
        let mut shown = back;
        assert!(shown.set(Band::Column, false));
        write_hidden_to(&path, &shown);
        assert_eq!(read_hidden_from(&path), Hidden::default());
        let _ = std::fs::remove_file(&path);
    }

    /// A machine that hid the strip or Needs you when they were two bands hides the one band now, and a line naming neither is left out.
    #[test]
    fn an_earlier_hide_of_either_band_hides_the_one_band() {
        for kept in ["strip\n", "needs\n", "needs\nstrip\n", "column\n", "something else\n\n strip \n"] {
            assert!(Hidden::parse(kept).has(Band::Column), "{kept:?} hides it");
        }
        assert_eq!(Hidden::parse("something else\n\n"), Hidden::default());
        assert_eq!(Hidden::parse("needs\nstrip\n").spell(), "column\n", "it is kept as the one band");
        assert_eq!(Band::Column.word(), "Step and Needs you", "+ offers it back as one entry");
    }

    #[test]
    fn needs_you_is_drawn_only_while_something_is_held_under_a_chip() {
        assert_eq!(standing(None, Held::default()), None);
        assert_eq!(standing(Some(Chip::Done), Held::default()), None, "nothing under any chip, nothing drawn");
        assert_eq!(standing(None, Held { open: true, ..Held::default() }), Some(Chip::Open));
        assert_eq!(standing(None, Held { done: true, ..Held::default() }), Some(Chip::Open), "what he closed alone draws the head, on Open");
        // A chip he pressed stands, even over one that holds more.
        assert_eq!(standing(Some(Chip::Done), Held { open: true, done: true, deferred: false }), Some(Chip::Done));
    }

    /// With nothing open and two deferred, the band stands on Open and Open lists no row.
    #[test]
    fn nothing_open_and_two_deferred_stands_on_open_with_no_rows() {
        let held = Held { deferred: true, ..Held::default() };
        let chip = standing(None, held).expect("Deferred holds a row, so the head is drawn");
        assert_eq!(chip, Chip::Open, "never Deferred by itself");
        let rows = listed(chip, Vec::<&str>::new, || panic!("Done was made"), || panic!("Deferred was made"));
        assert!(rows.is_empty());
    }

    /// A press on Deferred lists the deferred rows, and the press stands while nothing new waits on him.
    #[test]
    fn a_press_on_deferred_lists_them_and_stands() {
        let held = Held { deferred: true, ..Held::default() };
        let pressed = Pressed { chip: Chip::Deferred, open: BTreeSet::new() };
        let chip = standing(kept(Some(&pressed), &BTreeSet::new()), held).expect("drawn");
        assert_eq!(chip, Chip::Deferred);
        assert_eq!(listed(chip, || vec!["open"], || vec!["done"], || vec!["a", "b"]), ["a", "b"]);
    }

    /// An item that waits on him and was not open at the press brings the band back to Open.
    #[test]
    fn a_new_open_item_returns_the_band_to_open() {
        let held = Held { open: true, deferred: true, ..Held::default() };
        let pressed = Pressed { chip: Chip::Deferred, open: ["old".to_string()].into() };
        let same = ["old".to_string()].into();
        assert_eq!(standing(kept(Some(&pressed), &same), held), Some(Chip::Deferred), "the same open items keep the press");
        let fewer = BTreeSet::new();
        assert_eq!(standing(kept(Some(&pressed), &fewer), held), Some(Chip::Deferred), "one answered is not a new one");
        let more = ["old".to_string(), "new".to_string()].into();
        assert_eq!(standing(kept(Some(&pressed), &more), held), Some(Chip::Open), "a new one takes the press back");
        assert_eq!(kept(None, &more), None);
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
