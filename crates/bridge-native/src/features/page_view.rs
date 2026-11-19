//! A feature's page drawn from what `page_model.rs` made of Fleet's read, in the owner's Dossier look: a band across the top
//! tinted with the area's hue, and under it cards in columns. Every card is always drawn: one with nothing in it says why and how it fills in. Every value is a token.

use gpui::{div, prelude::*, px, relative, AnyElement, ClipboardItem, Div, ElementId, FontWeight, Hsla, IntoElement, Pixels, Render, Role, SharedString, Stateful, Toggled, Window};

use super::asks::{self, Spot};
use super::dossier::{button, hue_of_name, pill, pill_area, tint};
use super::page::{FeaturePageView, Load, Region};
use super::page_model::{Body, Chip, Code, Door, Floor, Item, Plan, Room, Row, Section, Written};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::reviews::{card_inner_radius, panel_inner_radius};
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{glyph, mono, sans};

/// What a card holds: the Jobs on the feature, the asks of its three kinds together, or a section.
#[derive(Clone, Copy, PartialEq)]
enum Card {
    Who,
    Asks,
    /// The code map with the feature's rooms lit. Not a section: it has no place in the rail.
    Floor,
    Section(Section),
}

/// The sides of the page the cards stand in: the page's padding (`.grid`) either side.
fn side(look: &Look) -> Pixels {
    look.length(token::SPACE_6) + look.length(token::SPACE_1)
}

/// The cards in columns of `count` (`.col`), the way the take stands them at three: Jobs and asks, how it is tested and runs
/// and its designs, then its code and what is stale or owed.
fn columns(count: usize) -> Vec<Vec<Card>> {
    use Card::{Asks, Floor, Who};
    use Section::{Code, Debt, Designs, Observe, Runs, Test};
    let s = Card::Section;
    match count {
        3 => vec![vec![Who, Asks], vec![s(Test), s(Runs), s(Designs)], vec![s(Code), Floor, s(Observe), s(Debt)]],
        2 => vec![vec![Who, Asks, s(Designs)], vec![s(Test), s(Runs), s(Code), Floor, s(Observe), s(Debt)]],
        _ => vec![vec![Who, Asks, s(Test), s(Runs), s(Designs), s(Code), Floor, s(Observe), s(Debt)]],
    }
}

/// A card (`.card`) on the page's panel, a step under the panel's ground so it reads as a card: the hairline, the card shadow, and where the keys are on it the focus colour. The one that says what
/// runs now is tinted with the running hue (`.now`).
pub(super) fn shell(look: &Look, id: impl Into<ElementId>, ring: bool, running: bool) -> Stateful<Div> {
    let edge = if ring {
        ring_colour(look)
    } else if running {
        look.color(token::STATUS_RUNNING).opacity(0.38)
    } else {
        look.color(token::BORDER_SUBTLE)
    };
    let ground = if running { tint(look, look.color(token::BG_BASE), token::STATUS_RUNNING, 0.09) } else { look.color(token::BG_BASE) };
    div().id(id.into()).w_full().flex().flex_col().rounded(look.length(token::RADIUS_LG)).border(look.length(token::BORDER_WIDTH)).border_color(edge).bg(ground).shadow(look.shadows(token::SHADOW_CARD)).overflow_hidden()
}

/// A card's title (`h3`): on a ground a little under the card's own, or in the running hue for the card that says what runs.
pub(super) fn head(look: &Look, key: &str, title: &str, running: bool, first: bool) -> impl IntoElement {
    let (ground, ink, edge) = if running {
        (tint(look, look.color(token::BG_BASE), token::STATUS_RUNNING, 0.20), look.color(token::STATUS_RUNNING), look.color(token::STATUS_RUNNING).opacity(0.30))
    } else {
        (tint(look, look.color(token::BG_SUNKEN), token::BG_BASE, 0.60), look.color(token::FG_DEFAULT), look.color(token::BORDER_SUBTLE))
    };
    labelled(
        div()
            .id(SharedString::from(format!("feature-card-title-{key}")))
            .flex_none()
            .flex()
            .items_center()
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) + px(2.))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .when(first, |head| head.rounded_t(card_inner_radius(look)))
            .bg(ground)
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .child(SharedString::from(title.to_string())),
        Role::Heading,
        title.to_string(),
    )
}

/// The inside of a card that holds text (`.in`).
pub(super) fn inner(look: &Look) -> Div {
    div().flex().flex_col().gap(look.length(token::SPACE_3) - px(2.)).px(look.length(token::SPACE_3) + px(2.)).py(look.length(token::SPACE_3))
}

/// What an empty card says: why it is empty and how it fills in.
pub(super) fn said_empty(key: &str) -> &'static str {
    match key {
        "on" => "No Job is changing this feature now. A Job shows here while it edits the feature's files.",
        "who" => "No merged Job has changed this feature yet. Each merge that changes its files adds its Job here.",
        "asks" => "Nothing asked about this feature yet. Bugs, changes and ideas you send a Job or note on a screen file here.",
        "test" => "Not written yet. The next Job that changes this feature's files writes how to test it before it ends.",
        "runs" => "Not written yet. The next Job that changes this feature's files writes how it runs before it ends.",
        "designs" => "No sketch, page or walk names this feature yet. One shows here when a Job draws it for this feature.",
        "code" => "No files mapped to this feature yet. Files map in from the code graph and each commit's Feature line.",
        "observe" => "Not written yet. The next Job that changes this feature's files writes how to observe it before it ends.",
        _ => "None written. The next Job that changes this feature's files writes any debt it leaves.",
    }
}

/// A card with nothing in it: its title and one muted line under it. It has no rows, chips or buttons, so no key stops on it.
fn empty_card(look: &Look, key: &str, title: &str) -> AnyElement {
    empty_card_saying(look, key, title, said_empty(key))
}

/// The same, where the line is the model's.
fn empty_card_saying(look: &Look, key: &str, title: &str, said: &'static str) -> AnyElement {
    let line = labelled(
        div()
            .id(SharedString::from(format!("feature-empty-{key}")))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(said),
        Role::Label,
        said,
    );
    labelled(shell(look, SharedString::from(format!("feature-card-{key}")), false, false).child(head(look, key, title, false, true)).child(line), Role::Region, title.to_string()).into_any_element()
}

impl FeaturePageView {
    // ---- the band ----------------------------------------------------------------------------

    /// The hue of the feature's area: the one whoever opened the page gave, or one from the area's name.
    fn band_hue(&self) -> Token {
        self.hue.unwrap_or_else(|| match &self.page {
            Load::Read(page) => hue_of_name(&page.feature.area),
            _ => hue_of_name(""),
        })
    }

    /// The band over the cards (`.head`): the title, the line under it, the area and when the page was last current, and at the
    /// right the button that starts a Job on the feature.
    fn render_top(&self, look: &Look, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let hue = self.band_hue();
        let read = match &self.page {
            Load::Read(page) => Some(page),
            _ => None,
        };
        let title = match read {
            Some(page) => div().child(SharedString::from(page.feature.title.clone())).into_any_element(),
            None => bone(look, px(160.), look.length(token::LEADING_XL) - px(8.)).into_any_element(),
        };
        let area = match (read, &self.area) {
            (Some(_), Some(area)) => Some(area.clone()),
            (Some(page), None) => Some(page.feature.area.replace('-', " ")),
            _ => None,
        };
        let current = read.and_then(super::page_model::current_as_of);
        let summary = read.map(|page| page.feature.summary.clone()).filter(|summary| !summary.is_empty());
        let start = labelled(
            button(look, "feature-start", "Start a Job on it", true, true, read.is_none(), Some(&self.start)).tooltip(tip("Open the dispatch panel to start a Job")).on_click(cx.listener(|this, _, window, cx| {
                this.focus_region(Region::Start, window, cx);
                this.start_job(cx);
            })),
            Role::Button,
            "Start a Job on it",
        );
        let outer = if self.flush { panel_inner_radius(look) } else { px(0.) };
        let inner = (outer - look.length(token::EDGE_ACTIVE)).max(px(0.));
        let band = div()
            .id("feature-top")
            .flex_none()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_6))
            .px(side(look))
            .pt(look.length(token::SPACE_6) - px(2.))
            .pb(look.length(token::SPACE_4) + px(2.))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .rounded_t(inner)
            .bg(tint(look, look.color(token::BG_RAISED), hue, 0.08))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(labelled(
                        div()
                            .id("feature-title")
                            .text_size(look.length(token::TEXT_XL))
                            .line_height(look.length(token::LEADING_XL))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(title),
                        Role::Heading,
                        match read {
                            Some(page) => page.feature.title.clone(),
                            None => "Feature".into(),
                        },
                    ))
                    .children(summary.map(|summary| {
                        div()
                            .id("feature-summary")
                            .mt(look.length(token::SPACE_1) + px(2.))
                            .max_w(look.length(token::SPACE_12) * 12.)
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(SharedString::from(summary))
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .mt(look.length(token::SPACE_3) - px(2.))
                            .children(area.map(|area| labelled(pill_area(look, hue, area.clone()).id("feature-area"), Role::Label, format!("Area: {area}"))))
                            .children(current.map(|current| div().id("feature-current").text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap().child(SharedString::from(current)))),
                    ),
            )
            .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).child(start));
        // The hue rule follows the panel's curve: the band's own ground sits a rule's width inside a coloured one.
        labelled(div().id("feature-band").flex_none().rounded_t(outer).bg(look.color(hue)).pt(look.length(token::EDGE_ACTIVE)).child(band), Role::Region, "Feature band")
    }

    // ---- rows --------------------------------------------------------------------------------

    /// One row of a card. The keys are on it where `row_at` is its place and the card is the one they are on.
    fn render_row(&self, section: Section, ix: usize, row: &Row, first: bool, divider: Option<Hsla>, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let picked = self.section == section && self.row_at == Some(ix);
        let id = SharedString::from(format!("feature-row-{}-{ix}", section.id()));
        let spot = Spot { picked, first, bug: section == Section::Bugs, divider };
        let line = match (&row.ask, section) {
            (Some(ask), _) => asks::ask(
                look,
                id,
                row,
                ask,
                spot,
                cx.listener(move |this, _, _, cx| {
                    this.select(section, cx);
                    this.open_story(ix, cx);
                }),
            ),
            (None, Section::Bugs) => asks::filed(look, id, row, spot),
            (None, Section::Designs) => asks::design(look, id, row, spot),
            (None, _) => asks::job(look, id, row, spot),
        }
        .on_click(cx.listener(move |this, _, window, cx| {
            this.select(section, cx);
            this.pick_row(ix, cx);
            this.focus_region(Region::Rows, window, cx);
        }));
        labelled(line, Role::ListBoxOption, row.title.clone()).into_any_element()
    }

    /// The rows of one section, with the line that folds the older asks of a kind. The list tracks the focus where the keys are on
    /// this section's rows.
    fn render_rows(&self, section: Section, rows: &[Row], divider: Option<Hsla>, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let open = self.unfolded.contains(&section);
        let mut list = div().id(SharedString::from(format!("feature-rows-{}", section.id()))).w_full().flex().flex_col();
        if self.section == section {
            list = list.track_focus(&self.rows);
        }
        for (at, item) in super::page_model::items(section, rows.len(), open).into_iter().enumerate() {
            match item {
                Item::Fold => {
                    let picked = self.section == section && self.row_at == Some(at);
                    list = list.child(asks::fold(
                        look,
                        SharedString::from(format!("feature-row-{}-{at}", section.id())),
                        open,
                        picked,
                        cx.listener(move |this, _, window, cx| {
                            this.select(section, cx);
                            this.pick_row(at, cx);
                            this.focus_region(Region::Rows, window, cx);
                            this.toggle_fold(cx);
                        }),
                    ));
                }
                Item::Row(row) => {
                    if let Some(row) = rows.get(row) {
                        list = list.child(self.render_row(section, at, row, at == 0, divider, look, cx));
                    }
                }
            }
        }
        labelled(list, Role::ListBox, section.title()).into_any_element()
    }

    // ---- the cards ---------------------------------------------------------------------------

    fn rows_of(&self, section: Section) -> &[Row] {
        match self.cards.get(&section) {
            Some(Body::Rows(rows)) => rows,
            _ => &[],
        }
    }

    /// Whether the keys are on a card of one of `here`, which draws it in the focus colour.
    fn under_keys(&self, here: &[Section], window: &Window) -> bool {
        self.rail.is_focused(window) && here.contains(&self.section)
    }

    /// The Jobs on the feature now in a card of their own, and those that worked on it before under it. Both are drawn where
    /// there are none, each saying why.
    fn render_who(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let rows = self.rows_of(Section::Who);
        let ring = self.under_keys(&[Section::Who], window);
        let on = rows.iter().take_while(|row| row.on).count();
        let running_edge = look.color(token::STATUS_RUNNING).opacity(0.18);
        let mut who = div().id("feature-who").w_full().flex().flex_col().gap(look.length(token::SPACE_4));
        if self.section == Section::Who && !rows.is_empty() {
            who = who.track_focus(&self.rows);
        }
        if on > 0 {
            let mut list = div().w_full().flex().flex_col();
            for (at, row) in rows.iter().take(on).enumerate() {
                list = list.child(self.render_row(Section::Who, at, row, at == 0, Some(running_edge), look, cx));
            }
            who = who.child(labelled(shell(look, "feature-card-on", ring, true).child(head(look, "on", "On it now", true, true)).child(list), Role::Region, "On it now"));
        } else {
            who = who.child(empty_card(look, "on", "On it now"));
        }
        if rows.len() > on {
            let mut list = div().w_full().flex().flex_col();
            for (at, row) in rows.iter().enumerate().skip(on) {
                list = list.child(self.render_row(Section::Who, at, row, at == on, None, look, cx));
            }
            who = who.child(labelled(shell(look, "feature-card-who", ring, false).child(head(look, "who", Section::Who.title(), false, true)).child(list), Role::Region, Section::Who.title()));
        } else {
            who = who.child(empty_card(look, "who", Section::Who.title()));
        }
        who.into_any_element()
    }

    /// The filter chips of the asks, in a strip under the card's title.
    fn render_chips(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let focused = self.chips.is_focused(window);
        let mut chips = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        for chip in Chip::ALL {
            let on = self.chip == chip;
            let cursor = focused && self.chip_at == chip;
            let said = match chip {
                Chip::Since => Some(match self.looked {
                    Some(at) => format!("Asks changed after {}", super::page_model::day(at)),
                    None => "Not looked at before, so every ask shows".to_string(),
                }),
                _ => None,
            };
            let pill = div()
                .id(SharedString::from(format!("feature-chip-{}", chip.label().to_lowercase().replace(' ', "-"))))
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::H_BADGE) / 2.)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if cursor { ring_colour(look) } else { gpui::transparent_black() })
                .bg(if on { look.color(token::ACCENT_MUTED) } else { look.color(token::BG_HOVER) })
                .text_size(look.length(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
                .whitespace_nowrap()
                .cursor_pointer()
                .when(!on, |pill| pill.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.14))))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .when_some(said, |pill, said| pill.tooltip(tip(said)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.apply(chip, cx);
                    this.focus_region(Region::Chips, window, cx);
                }))
                .child(chip.label());
            chips = chips.child(labelled(pill, Role::Button, chip.label()));
        }
        div()
            .id("feature-chips")
            .track_focus(&self.chips)
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(chips.id("feature-chip-row"), Role::Group, "Show"))
    }

    /// The asks of the three kinds in one card, each kind under its own chip (`.lane-h`).
    fn render_asks(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let ring = self.under_keys(&[Section::Bugs, Section::Changes, Section::Ideas], window);
        let card = shell(look, "feature-card-asks", ring, false).child(head(look, "asks", "Asks", false, true));
        let body = match &self.asks {
            Load::Reading => {
                let mut bones = div().flex().flex_col();
                for at in 0..3 {
                    bones = bones.child(self.bone_row(look, at));
                }
                labelled(bones.id("feature-asks-loading"), Role::Image, "Loading").into_any_element()
            }
            Load::Failed(why) => self.not_read(look, why).into_any_element(),
            Load::Read(_) if !self.has_asks => return empty_card(look, "asks", "Asks"),
            Load::Read(_) => {
                let mut lanes = div().flex().flex_col();
                let mut first = true;
                for (section, words, hue) in [(Section::Bugs, "Bug", token::STATUS_COMPLETED_FAILED), (Section::Changes, "Change request", token::ACCENT), (Section::Ideas, "Idea", token::OPERATOR)] {
                    let rows = self.rows_of(section);
                    if rows.is_empty() {
                        continue;
                    }
                    let heading = labelled(
                        div()
                            .id(SharedString::from(format!("feature-lane-{}", section.id())))
                            .flex()
                            .items_center()
                            .px(look.length(token::SPACE_3) + px(2.))
                            .py(look.length(token::SPACE_2))
                            .when(!first, |lane| lane.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                            .child(pill(look, hue, words)),
                        Role::Heading,
                        words,
                    );
                    lanes = lanes.child(heading).child(self.render_rows(section, rows, None, look, cx));
                    first = false;
                }
                div().flex().flex_col().child(self.render_chips(look, window, cx)).child(lanes).into_any_element()
            }
        };
        labelled(card.child(body), Role::Region, "Asks").into_any_element()
    }

    fn bone_row(&self, look: &Look, at: usize) -> impl IntoElement {
        div()
            .id(SharedString::from(format!("feature-row-bone-{at}")))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) + px(1.))
            .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
            .child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3)))
            .child(div().flex().flex_col().gap(px(4.)).child(bone(look, px(180. + 40. * (at % 3) as f32), look.length(token::LEADING_XS) - px(6.))).child(bone(look, px(110.), look.length(token::LEADING_2XS) - px(6.))))
    }

    fn not_read(&self, look: &Look, why: &str) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
            .child(SharedString::from(why.to_string()))
    }

    fn render_designs(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let rows = self.rows_of(Section::Designs);
        if rows.is_empty() {
            return empty_card(look, "designs", Section::Designs.title());
        }
        let ring = self.under_keys(&[Section::Designs], window);
        let card = shell(look, "feature-card-designs", ring, false).child(head(look, "designs", Section::Designs.title(), false, true)).child(self.render_rows(Section::Designs, rows, None, look, cx));
        labelled(card, Role::Region, Section::Designs.title()).into_any_element()
    }

    /// The entry points, files and prefixes of the feature as mono rows with a hairline between them (`.path`). A file's figure is
    /// the Jobs that wrote it, in a column of its own.
    fn render_code(&self, code: &Code, look: &Look, window: &Window) -> AnyElement {
        let ring = self.under_keys(&[Section::Code], window);
        let path = |at: usize, text: &str, tag: AnyElement| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1) + px(2.))
                .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(text.to_string())))
                .child(tag)
        };
        let word = |text: &'static str| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(text).into_any_element();
        let mut at = 0;
        let mut rows = div().flex().flex_col();
        for point in &code.entry_points {
            rows = rows.child(path(at, point, word("entry")));
            at += 1;
        }
        for (file, jobs) in &code.files {
            let figure = div()
                .id(SharedString::from(format!("feature-file-jobs-{at}")))
                .flex_none()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .tooltip(tip(if *jobs == 1 { "Jobs that wrote this file: 1".to_string() } else { format!("Jobs that wrote this file: {jobs}") }))
                .child(SharedString::from(jobs.to_string()));
            rows = rows.child(path(at, file, labelled(figure, Role::Image, format!("Jobs that wrote {file}: {jobs}")).into_any_element()));
            at += 1;
        }
        for prefix in &code.paths {
            rows = rows.child(path(at, prefix, word("path")));
            at += 1;
        }
        let card = shell(look, "feature-card-code", ring, false).child(head(look, "code", Section::Code.title(), false, true)).child(div().px(look.length(token::SPACE_3) + px(2.)).py(look.length(token::SPACE_1)).child(rows));
        labelled(card, Role::Region, Section::Code.title()).into_any_element()
    }

    /// A written section: a command in a well with Copy at its right (`.well`), the rest as a paragraph, and who wrote it. A stale
    /// one has a band over its title in the awaiting review hue (`.stale-band`).
    fn render_written(&self, section: Section, written: &Written, look: &Look, window: &Window) -> AnyElement {
        let ring = self.under_keys(&[section], window);
        let mut body = inner(look).id(SharedString::from(format!("feature-body-{}", section.id())));
        if self.section == section && !self.wells().is_empty() {
            body = body.track_focus(&self.well);
        }
        let mut well = 0;
        for (at, block) in written.blocks.iter().enumerate() {
            body = body.child(if block.command {
                let keys = self.section == section && self.well.is_focused(window) && self.well_at == well;
                well += 1;
                let text = block.text.clone();
                let copy = labelled(
                    div()
                        .id(SharedString::from(format!("feature-copy-{}-{at}", section.id())))
                        .flex_none()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .cursor_pointer()
                        .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                        .tooltip(tip("Copy the command"))
                        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(text.clone())))
                        .child("Copy"),
                    Role::Button,
                    "Copy",
                );
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3) - px(2.))
                    .px(look.length(token::SPACE_3) - px(1.))
                    .py(look.length(token::SPACE_2) + px(1.))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_SUNKEN))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if keys { ring_colour(look) } else { look.color(token::BORDER_SUBTLE) })
                    .child(div().flex_1().min_w_0().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(block.text.clone())))
                    .child(copy)
            } else {
                div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(block.text.clone()))
            });
        }
        body = body.child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(written.by.clone())));
        let band = written.stale.clone().map(|stale| {
            labelled(
                div()
                    .id("feature-stale")
                    .flex_none()
                    .px(look.length(token::SPACE_3) + px(2.))
                    .py(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::STATUS_AWAITING_REVIEW).opacity(0.30))
                    .rounded_t(card_inner_radius(look))
                    .bg(tint(look, look.color(token::BG_BASE), token::STATUS_AWAITING_REVIEW, 0.14))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                    .child(SharedString::from(stale.clone())),
                Role::Image,
                stale,
            )
        });
        let first = band.is_none();
        let card = shell(look, SharedString::from(format!("feature-card-{}", section.id())), ring, false).children(band).child(head(look, section.id(), section.title(), false, first)).child(body);
        labelled(card, Role::Region, section.title()).into_any_element()
    }

    // ---- the floor plan ----------------------------------------------------------------------

    /// One tile (`room`): the module's name in mono. A lit one wears the feature's hue as a tinted fill and an edge, with its name at
    /// full weight; an unlit one is a step under the card's ground with quiet ink.
    fn render_room(&self, at: usize, room: &Room, hue: Token, look: &Look) -> AnyElement {
        let (ground, edge, ink) = if room.lit {
            (tint(look, look.color(token::BG_BASE), hue, 0.16), look.color(hue).opacity(0.60), look.color(token::FG_DEFAULT))
        } else {
            (look.color(token::BG_SUNKEN), look.color(token::BORDER_SUBTLE), look.color(token::FG_SUBTLE))
        };
        let tile = div()
            .id(SharedString::from(format!("feature-room-{at}")))
            .flex_none()
            .min_w_0()
            .max_w_full()
            .truncate()
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .font_weight(FontWeight(look.number(if room.lit { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
            .text_color(ink)
            .tooltip(tip(room.said.clone()))
            .child(SharedString::from(room.module.clone()));
        labelled(tile, Role::ListItem, format!("{}, {}", room.module, if room.lit { "in this feature" } else { "not in this feature" })).into_any_element()
    }

    /// One door (`from` to `to`) in fixed columns: the lit room, the glyph for the way in, the module it leads to, and the chip column.
    fn render_door(&self, at: usize, door: &Door, look: &Look) -> AnyElement {
        let (name, said, ink) = if door.contract {
            ("scroll-text", "Contract: the module it leads to carries a wire or API contract", token::ACCENT)
        } else {
            ("corner-down-right", "Depends on", token::FG_SUBTLE)
        };
        let way = labelled(
            div().id(SharedString::from(format!("feature-door-way-{at}"))).flex_none().flex().items_center().justify_center().w(look.length(token::SPACE_4)).tooltip(tip(said)).child(glyph(name, look.length(token::SPACE_3) + px(2.), look.color(ink))),
            Role::Image,
            said,
        );
        let chip = door.contract.then(|| labelled(div().id(SharedString::from(format!("feature-door-contract-{at}"))).flex_none().child(pill(look, token::ACCENT, "contract")), Role::Label, format!("Contract: {} to {}", door.from, door.to)));
        let word = |text: &str, ink: Hsla, grow: f32| div().min_w_0().flex_basis(relative(0.)).flex_grow(grow).truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS)).text_color(ink).child(SharedString::from(text.to_string()));
        labelled(
            div()
                .id(SharedString::from(format!("feature-door-{at}")))
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1) + px(2.))
                .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .child(word(&door.from, look.color(token::FG_DEFAULT), 1.2))
                .child(way)
                .child(word(&door.to, look.color(token::FG_MUTED), 1.))
                .child(div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_4)).flex().justify_end().children(chip)),
            Role::ListItem,
            format!("{} to {}", door.from, door.to),
        )
        .into_any_element()
    }

    /// The press under the grid that opens the unlit rooms of a long plan, and folds them again. The keys can be on it.
    fn render_floor_fold(&self, plan: &Plan, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let words = if self.floor_open { "Fewer modules".to_string() } else { format!("{} more modules", plan.hidden) };
        let keys = self.floor.is_focused(window);
        labelled(
            div()
                .id("feature-floor-fold")
                .track_focus(&self.floor)
                .tab_stop(true)
                .self_start()
                .ml(px(0.) - look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if keys { ring_colour(look) } else { gpui::transparent_black() })
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .cursor_pointer()
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .aria_expanded(self.floor_open)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus_region(Region::Floor, window, cx);
                    this.toggle_floor(cx);
                }))
                .child(SharedString::from(words.clone())),
            Role::Button,
            words,
        )
        .into_any_element()
    }

    /// The code map as a card: the rooms as a wrapping grid, the doors from the lit ones as rows under it. With nothing to draw it says
    /// why and what fills it.
    fn render_floor(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Load::Read(page) = &self.page else { return empty_card_saying(look, "floor", "Floor plan", super::page_model::NO_PLAN) };
        let plan = match super::page_model::floor(page, self.floor_open) {
            Floor::Empty(said) => return empty_card_saying(look, "floor", "Floor plan", said),
            Floor::Plan(plan) => plan,
        };
        let hue = self.band_hue();
        let pad = look.length(token::SPACE_3) + px(2.);
        let mut grid = div().id("feature-floor-rooms").flex().flex_wrap().gap(look.length(token::SPACE_2));
        for (at, room) in plan.rooms.iter().enumerate() {
            grid = grid.child(self.render_room(at, room, hue, look));
        }
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_3)).px(pad).py(look.length(token::SPACE_3)).child(labelled(grid, Role::List, "Modules"));
        if plan.folds {
            body = body.child(self.render_floor_fold(&plan, look, window, cx));
        }
        let card = shell(look, "feature-card-floor", false, false).child(head(look, "floor", "Floor plan", false, true)).child(body);
        let card = if plan.doors.is_empty() {
            card
        } else {
            let mut doors = div().id("feature-floor-doors").w_full().flex().flex_col();
            for (at, door) in plan.doors.iter().enumerate() {
                doors = doors.child(self.render_door(at, door, look));
            }
            card.child(
                div()
                    .flex()
                    .flex_col()
                    .px(pad)
                    .pb(look.length(token::SPACE_2))
                    .pt(look.length(token::SPACE_2))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Doors"))
                    .child(labelled(doors, Role::List, "Doors")),
            )
        };
        labelled(card, Role::Region, "Floor plan").into_any_element()
    }

    fn render_card(&self, card: Card, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        match card {
            Card::Who => self.render_who(look, window, cx),
            Card::Asks => self.render_asks(look, window, cx),
            Card::Floor => self.render_floor(look, window, cx),
            Card::Section(Section::Designs) => self.render_designs(look, window, cx),
            Card::Section(section) => match self.cards.get(&section) {
                Some(Body::Code(code)) if self.present(section) => self.render_code(code, look, window),
                Some(Body::Written(Some(written))) => self.render_written(section, written, look, window),
                _ => empty_card(look, section.id(), section.title()),
            },
        }
    }

    /// The cards, in as many columns as the page has room for: the first is a little wider (`1.15fr 1fr 1fr`).
    fn render_cards(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let gap = look.length(token::SPACE_4);
        let width = window.viewport_size().width - self.inset - side(look) * 2.;
        let least = look.length(token::SPACE_12) * 6.;
        let count = (((width + gap) / (least + gap)).floor() as usize).clamp(1, 3);
        let mut grid = div().w_full().flex().items_start().gap(gap);
        for (at, cards) in columns(count).into_iter().enumerate() {
            let mut column = div().min_w_0().flex_basis(relative(0.)).flex_grow(if at == 0 && count == 3 { 1.15 } else { 1. }).flex().flex_col().gap(gap);
            for card in cards {
                column = column.child(self.render_card(card, look, window, cx));
            }
            grid = grid.child(column);
        }
        grid.into_any_element()
    }

    fn render_loading(&self, look: &Look) -> AnyElement {
        let mut grid = div().w_full().flex().items_start().gap(look.length(token::SPACE_4));
        for column in 0..3 {
            let mut card = shell(look, SharedString::from(format!("feature-card-bone-{column}")), false, false).child(
                div().flex_none().px(look.length(token::SPACE_3) + px(2.)).py(look.length(token::SPACE_3)).border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(bone(look, px(96.), look.length(token::LEADING_XS) - px(6.))),
            );
            for at in 0..3 {
                card = card.child(self.bone_row(look, at));
            }
            grid = grid.child(div().min_w_0().flex_1().child(card));
        }
        labelled(grid.id("feature-pane-loading"), Role::Image, "Loading").into_any_element()
    }
}

impl Render for FeaturePageView {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let top = self.render_top(&look, cx);
        let content = match &self.page {
            Load::Reading => self.render_loading(&look),
            Load::Failed(why) => shell(&look, "feature-not-read", false, false).child(self.not_read(&look, why)).into_any_element(),
            Load::Read(_) => self.render_cards(&look, window, cx),
        };
        div()
            .id("feature-page-view")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| this.engage()))
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(top)
            .child(
                div()
                    .id("feature-cards")
                    .track_focus(&self.rail)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(side(&look))
                    .pt(look.length(token::SPACE_4) + px(2.))
                    .pb(look.length(token::SPACE_6))
                    .child(content),
            )
    }
}
