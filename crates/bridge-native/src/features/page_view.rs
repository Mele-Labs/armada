//! A feature's page drawn from what `page_model.rs` made of Fleet's read, in the owner's Dossier look: a band across the top
//! tinted with the area's hue, and under it cards in columns. Every card is always drawn: one with nothing in it says why and how it fills in. Every value is a token.

use gpui::{div, prelude::*, px, relative, AnyElement, ClipboardItem, Div, ElementId, FontWeight, Hsla, IntoElement, Pixels, Render, Role, SharedString, Stateful, Toggled, Window};

use super::asks::{self, Spot};
use super::read_rows;
use super::dossier::{button, hue_of_name, pill, pill_area, tint};
use super::page::{FeaturePageView, Load, Region, Retiring};
use super::floor_map::{Ink, Laid, HEIGHT, WIDTH};
use super::page_model::{Body, Chip, Code, DecisionRow, Floor, Item, ReadRow, Row, Section, Written};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::reviews::{card_inner_radius, panel_inner_radius};
use crate::settings::view::{bone, refusal, ring_colour, tip};
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

/// The cards in columns of `count` (`.col`), the way the take stands them at three: Jobs, asks and decisions, how it is tested and
/// runs, its designs and docs, then its code, tickets and what is stale or owed. In two or three columns the floor plan is not among them: it spans
/// the page under them, so the right column does not carry it.
fn columns(count: usize) -> Vec<Vec<Card>> {
    use Card::{Asks, Floor, Who};
    use Section::{Code, Debt, Decisions, Designs, Docs, Observe, Runs, Test, Tickets};
    let s = Card::Section;
    match count {
        3 => vec![vec![Who, Asks, s(Decisions)], vec![s(Test), s(Runs), s(Designs), s(Docs)], vec![s(Code), s(Tickets), s(Observe), s(Debt)]],
        2 => vec![vec![Who, Asks, s(Decisions), s(Designs), s(Docs)], vec![s(Test), s(Runs), s(Code), s(Tickets), s(Observe), s(Debt)]],
        _ => vec![vec![Who, Asks, s(Decisions), s(Test), s(Runs), s(Designs), s(Docs), s(Code), Floor, s(Tickets), s(Observe), s(Debt)]],
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
        "docs" => "No Job read a document before it changed a file of this feature.",
        "tickets" => "No Job read a ticket before it changed a file of this feature.",
        "decisions" => "Nothing you settled in a Job is filed under this feature.",
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
        let health = read.and_then(super::page_model::health);
        let planned = read.is_some_and(|page| page.feature.planned);
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
                            .flex_wrap()
                            .items_center()
                            .gap_x(look.length(token::SPACE_2))
                            .gap_y(look.length(token::SPACE_1))
                            .mt(look.length(token::SPACE_3) - px(2.))
                            .children(area.map(|area| labelled(pill_area(look, hue, area.clone()).id("feature-area"), Role::Label, format!("Area: {area}"))))
                            .children(current.map(|current| div().id("feature-current").text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap().child(SharedString::from(current))))
                            .children(planned.then(|| mark_chip(look, "feature-planned", "circle-dashed", "Planned", token::STATUS_NOT_STARTED, token::STATUS_NOT_STARTED_BG, super::page_model::PLANNED_TIP, "Planned".into())))
                            .children(health.as_ref().map(|health| {
                                let (ink, wash) = if health.mark == ipc::HealthMark::Poor { (token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG) } else { (token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG) };
                                mark_chip(look, "feature-health", health.glyph, health.word, ink, wash, &health.tip, format!("Health: {}", health.word))
                            }))
                            .children(health.filter(|health| !health.line.is_empty()).map(|health| {
                                labelled(div().id("feature-health-reasons").text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(health.line.clone())), Role::Label, health.line)
                            })),
                    ),
            )
            .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).child(start));
        // The hue rule follows the panel's curve: the band's own ground sits a rule's width inside a coloured one.
        labelled(div().id("feature-band").flex_none().rounded_t(outer).bg(look.color(hue)).pt(look.length(token::EDGE_ACTIVE)).child(band), Role::Region, "Feature band")
    }

    // ---- the proposal to retire --------------------------------------------------------------

    /// The row under the band where the librarian proposes retiring the feature (`.retire`): the question, the evidence it rests on,
    /// and the two buttons. A refusal stands on the row in Fleet's words. Once retired, the row says the feature left the catalog.
    fn render_retire(&self, look: &Look, cx: &mut gpui::Context<Self>) -> Option<AnyElement> {
        let Load::Read(page) = &self.page else { return None };
        let hue = token::STATUS_AWAITING_APPROVAL;
        let row = div()
            .id("feature-retire")
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .mb(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) + px(1.))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(hue))
            .bg(look.color(token::STATUS_AWAITING_APPROVAL_BG))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS));
        if self.retiring == Retiring::Retired {
            let said = "Retired. It left the catalog.";
            let row = row.child(glyph("circle-check", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_SUCCESS))).child(div().flex_1().min_w_0().text_color(look.color(token::FG_MUTED)).child(said));
            return Some(labelled(row, Role::Status, said).into_any_element());
        }
        let evidence = super::page_model::retire_evidence(page)?;
        let working = self.retiring == Retiring::Working;
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .gap_x(look.length(token::SPACE_2))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Retire this feature?"))
                    .child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(evidence.to_string()))),
            )
            .children(match &self.retiring {
                Retiring::Failed(why) => Some(labelled(div().id("feature-retire-refused").mt(look.length(token::SPACE_1)).child(refusal(look, why.clone())), Role::Label, why.clone())),
                _ => None,
            });
        let retire = labelled(
            button(look, "feature-retire-yes", "Retire", false, false, working, Some(&self.retire)).tooltip(tip("Take it out of the catalog")).on_click(cx.listener(|this, _, window, cx| {
                this.focus_region(Region::Retire, window, cx);
                this.answer_retire(true, window, cx);
            })),
            Role::Button,
            "Retire",
        );
        let keep = labelled(
            button(look, "feature-retire-no", "Keep", false, false, working, Some(&self.keep)).tooltip(tip("Keep it; the librarian asks again only if its evidence changes")).on_click(cx.listener(|this, _, window, cx| {
                this.focus_region(Region::Keep, window, cx);
                this.answer_retire(false, window, cx);
            })),
            Role::Button,
            "Keep",
        );
        let row = row
            .child(div().flex_none().self_start().mt(px(1.)).child(glyph("circle-question-mark", look.length(token::SPACE_4), look.color(hue))))
            .child(words)
            .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).child(retire).child(keep));
        Some(labelled(row, Role::Region, "Retire this feature?").into_any_element())
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
            if section == Section::Designs {
                this.open_design(ix, cx);
            }
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

    /// The docs or tickets a Job read, or the decisions he settled, as rows of a card. With none, the card says why.
    fn render_plain(&self, section: Section, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let count = match self.cards.get(&section) {
            Some(Body::Reads(rows)) => rows.len(),
            Some(Body::Decisions(rows)) => rows.len(),
            _ => 0,
        };
        if count == 0 {
            return empty_card(look, section.id(), section.title());
        }
        let ring = self.under_keys(&[section], window);
        let mut list = div().id(SharedString::from(format!("feature-rows-{}", section.id()))).w_full().flex().flex_col();
        if self.section == section {
            list = list.track_focus(&self.rows);
        }
        for at in 0..count {
            let picked = self.section == section && self.row_at == Some(at);
            let id = SharedString::from(format!("feature-row-{}-{at}", section.id()));
            let spot = Spot { picked, first: at == 0, bug: false, divider: None };
            let (row, name) = match self.cards.get(&section) {
                Some(Body::Reads(rows)) => {
                    let under = if section == Section::Docs { self.render_doc_flag(at, &rows[at], look, window, cx) } else { None };
                    (read_rows::read(look, id, &rows[at], spot, under), rows[at].title.clone())
                }
                Some(Body::Decisions(rows)) => (read_rows::decision(look, id, &rows[at], spot), rows[at].question.clone()),
                _ => continue,
            };
            let row = row.on_click(cx.listener(move |this, _, window, cx| {
                this.select(section, cx);
                this.pick_row(at, cx);
                this.focus_region(Region::Rows, window, cx);
            }));
            list = list.child(labelled(row, Role::ListBoxOption, name));
        }
        let title = if section == Section::Docs { self.docs_head(look, cx) } else { head(look, section.id(), section.title(), false, true).into_any_element() };
        let card = shell(look, SharedString::from(format!("feature-card-{}", section.id())), ring, false).child(title).child(labelled(list, Role::ListBox, section.title()));
        labelled(card, Role::Region, section.title()).into_any_element()
    }

    // ---- the floor plan ----------------------------------------------------------------------

    /// The code map as a card: every module a plot sized by its bytes, its files pills inside sized by theirs, the contracts the
    /// feature crosses as dashed doors, and the key under it. With nothing to draw it says why and what fills it.
    fn render_floor(&self, look: &Look, wide: bool) -> AnyElement {
        let map = match (&self.page, &self.floor) {
            (Load::Read(_), Floor::Plan(map)) => map.clone(),
            (Load::Read(_), Floor::Empty(said)) => return empty_card_saying(look, "floor", "Floor plan", said),
            _ => return empty_card_saying(look, "floor", "Floor plan", super::page_model::NO_PLAN),
        };
        let pad = look.length(token::SPACE_3) + px(2.);
        let body = div().flex().flex_col().gap(look.length(token::SPACE_2)).px(pad).py(look.length(token::SPACE_3)).child(floor_drawing(look, map, wide)).child(floor_key(look));
        let card = shell(look, "feature-card-floor", false, false).child(head(look, "floor", "Floor plan", false, true)).child(body);
        labelled(card, Role::Region, "Floor plan").into_any_element()
    }

    fn render_card(&self, card: Card, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        match card {
            Card::Who => self.render_who(look, window, cx),
            Card::Asks => self.render_asks(look, window, cx),
            Card::Floor => self.render_floor(look, false),
            Card::Section(Section::Designs) => self.render_designs(look, window, cx),
            Card::Section(section @ (Section::Docs | Section::Tickets | Section::Decisions)) => self.render_plain(section, look, window, cx),
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
        if count == 1 {
            return grid.into_any_element();
        }
        div().w_full().flex().flex_col().gap(gap).child(grid).child(self.render_floor(look, true)).into_any_element()
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
        let retire = self.render_retire(&look, cx);
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
                    .children(retire)
                    .child(content),
            )
    }
}

/// A chip in the band with a glyph before its word, filled with its state's wash: Watch, Poor, Planned. Its tooltip says why.
fn mark_chip(look: &Look, id: &'static str, name: &'static str, words: &'static str, ink: Token, wash: Token, why: &str, said: String) -> AnyElement {
    let ink = look.color(ink);
    let chip = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::H_BADGE) / 2.)
        .bg(look.color(wash))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .whitespace_nowrap()
        .tooltip(tip(why.to_string()))
        .child(glyph(name, look.length(token::SPACE_3), ink))
        .child(words);
    labelled(chip, Role::Status, said).into_any_element()
}

/// The colours the floor plan paints with, read from the look before the paint, which has none.
#[derive(Clone, Copy)]
struct FloorInks {
    plot: Hsla,
    edge: Hsla,
    lit_plot: Hsla,
    lit_edge: Hsla,
    pill: Hsla,
    lit_pill: Hsla,
    changing: Hsla,
    door: Hsla,
}

fn floor_inks(look: &Look) -> FloorInks {
    let raised = look.color(token::BG_RAISED);
    FloorInks {
        plot: raised,
        edge: look.color(token::BORDER_DEFAULT),
        lit_plot: tint(look, raised, token::STATUS_AWAITING_REVIEW, 0.07),
        lit_edge: look.color(token::STATUS_AWAITING_REVIEW),
        pill: tint(look, raised, token::BORDER_DEFAULT, 0.8),
        lit_pill: tint(look, raised, token::STATUS_AWAITING_REVIEW, 0.6),
        changing: look.color(token::STATUS_RUNNING),
        door: look.color(token::FG_DEFAULT),
    }
}

/// `text` in the mono face at `size`, cut with an ellipsis until it is no wider than `room`; none where not one letter fits.
fn shaped_within(window: &mut Window, text: &str, font: &gpui::Font, size: Pixels, colour: Hsla, room: Pixels) -> Option<gpui::ShapedLine> {
    let shape = |words: String, window: &mut Window| {
        let run = gpui::TextRun { len: words.len(), font: font.clone(), color: colour, background_color: None, underline: None, strikethrough: None };
        window.text_system().shape_line(words.into(), size, &[run], None)
    };
    let whole = shape(text.to_string(), window);
    if whole.width <= room {
        return Some(whole);
    }
    let letters: Vec<char> = text.chars().collect();
    (1..letters.len()).rev().map(|keep| letters[..keep].iter().collect::<String>() + "\u{2026}").find_map(|cut| {
        let line = shape(cut, window);
        (line.width <= room).then_some(line)
    })
}

/// The plots, pills, doors, names and door chips painted as quads, paths and shaped lines at the card's width, the mock's shape
/// kept: about 1500 rects a frame, laid out again only when the width moves.
/// Across the page (`wide`) it keeps the mock's height and takes the width; in a column it keeps the mock's shape.
pub(crate) fn floor_drawing(look: &Look, laid: std::rc::Rc<Laid>, wide: bool) -> AnyElement {
    let inks = floor_inks(look);
    // The names keep the mock's size, the smallest the tokens hold, whatever the width.
    let words_size = look.length(token::TEXT_SWITCH);
    let look_radius = look.length(token::RADIUS_SM);
    let (name_ink, lit_ink, chip_ground) = (look.color(token::FG_SUBTLE), look.color(token::STATUS_AWAITING_REVIEW), look.color(token::BG_BASE));
    let family = mono(look);
    let (body, heading) = (FontWeight(look.number(token::WEIGHT_BODY)), FontWeight(look.number(token::WEIGHT_HEADING)));
    let said = laid.said.clone();
    // The strip a plot's name sits in: the name's line and the room over and under it.
    let strip = f32::from(words_size * 1.2) + 3.;
    let paint = gpui::canvas(
        move |bounds, _, _| laid.at(f32::from(bounds.size.width), f32::from(bounds.size.height), strip),
        move |bounds, painted, window, cx| {
            let at = |x: f32, y: f32| gpui::point(bounds.origin.x + px(x), bounds.origin.y + px(y));
            let rect = |r: &super::floor_map::Rect| gpui::Bounds::new(at(r.x, r.y), gpui::size(px(r.w), px(r.h)));
            for plot in &painted.plots {
                let (ground, edge) = if plot.lit { (inks.lit_plot, inks.lit_edge) } else { (inks.plot, inks.edge) };
                window.paint_quad(gpui::quad(rect(&plot.rect), look_radius, ground, px(1.), edge, gpui::BorderStyle::Solid));
            }
            for pill in &painted.pills {
                let ink = match pill.ink {
                    Ink::Plain => inks.pill,
                    Ink::Lit => inks.lit_pill,
                    Ink::Changing => inks.changing,
                };
                window.paint_quad(gpui::fill(rect(&pill.rect), ink));
            }
            let name_size = words_size;
            for plot in &painted.plots {
                let font = gpui::Font { family: family.clone(), weight: if plot.lit { heading } else { body }, ..gpui::font(family.clone()) };
                let room = px((plot.rect.w - 8.).max(0.));
                if let Some(line) = shaped_within(window, &plot.name, &font, name_size, if plot.lit { lit_ink } else { name_ink }, room) {
                    let _ = line.paint(at(plot.rect.x + 5., plot.rect.y + 2.), name_size * 1.2, gpui::TextAlign::Left, None, window, cx);
                }
            }
            let chip_size = words_size;
            let chip_font = gpui::Font { family: family.clone(), weight: heading, ..gpui::font(family.clone()) };
            for door in &painted.doors {
                let mut line = gpui::PathBuilder::stroke(px(1.5)).dash_array(&[px(4.), px(3.)]);
                line.move_to(at(door.from.0, door.from.1));
                line.line_to(at(door.to.0, door.to.1));
                if let Ok(path) = line.build() {
                    window.paint_path(path, inks.door.opacity(0.85));
                }
                let dot = 3.;
                for end in [door.from, door.to] {
                    let centre = at(end.0, end.1);
                    window.paint_quad(gpui::fill(gpui::Bounds::new(gpui::point(centre.x - px(dot), centre.y - px(dot)), gpui::size(px(dot * 2.), px(dot * 2.))), inks.door).corner_radii(px(dot)));
                }
                // The chip at the middle names the module the door leads to.
                let Some(words) = shaped_within(window, &door.label, &chip_font, chip_size, inks.door, bounds.size.width) else { continue };
                let middle = at((door.from.0 + door.to.0) / 2., (door.from.1 + door.to.1) / 2.);
                let (wide, tall) = (words.width + chip_size, chip_size * 1.6);
                let chip = gpui::Bounds::new(gpui::point(middle.x - wide / 2., middle.y - tall / 2.), gpui::size(wide, tall));
                window.paint_quad(gpui::quad(chip, tall / 2., chip_ground, px(1.), inks.door, gpui::BorderStyle::Solid));
                let _ = words.paint(gpui::point(middle.x - words.width / 2., middle.y - chip_size * 0.6), chip_size * 1.2, gpui::TextAlign::Left, None, window, cx);
            }
        },
    )
    .absolute()
    .size_full();
    labelled(
        div().id("feature-floor-map").relative().w_full().map(|map| if wide { map.h(px(HEIGHT)) } else { map.aspect_ratio(WIDTH / HEIGHT) }).rounded(look.length(token::RADIUS_MD)).overflow_hidden().bg(look.color(token::BG_SUNKEN)).child(paint),
        Role::Image,
        said,
    )
    .into_any_element()
}

/// What the colours of the floor plan mean, in a line under it.
fn floor_key(look: &Look) -> AnyElement {
    let swatch = |colour: Hsla| div().flex_none().w(look.length(token::SPACE_3)).h(look.length(token::SPACE_2)).rounded(px(2.)).bg(colour);
    let entry = |mark: Option<AnyElement>, words: &'static str| div().flex().items_center().gap(look.length(token::SPACE_1)).whitespace_nowrap().children(mark).child(words);
    let inks = floor_inks(look);
    let contract = div().font_family(mono(look)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("- -").into_any_element();
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x(look.length(token::SPACE_3))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .text_color(look.color(token::FG_SUBTLE))
        .child(entry(Some(swatch(inks.lit_pill).into_any_element()), "this feature's files"))
        .child(entry(Some(swatch(inks.changing).into_any_element()), "changed by a Job now"))
        .child(entry(None, "small files share a pill"))
        .child(entry(Some(contract), "contract crossed"))
        .into_any_element()
}
