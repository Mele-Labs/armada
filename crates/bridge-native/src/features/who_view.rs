//! Who's on what drawn from what `who_model.rs` made of Fleet's answer, in one of three layouts of the same data: a grid with the
//! features down and the people across, a block for each feature, and a card for each person. A feature that two or more people are
//! on has its name in the awaiting-review colour. Every value is a token.

use gpui::{div, prelude::*, px, AnyElement, Div, FontWeight, Role, SharedString, Window};

use super::dossier::selected_shadow;
use super::page::Load;
use super::page_view::{inner, shell};
use super::project_page::ProjectPageView;
use super::project_view::{not_read, said_line};
use super::who_model::{self, Cell, Mark, Person, Stop, WhoModel, WhoView};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::rows::{segment, segment_track, Pick};
use crate::settings::view::{ring_colour, tip};
use crate::titlebar::parts::{mono, sans};

/// What the keys stop on, in the order the layout draws it. `at` is the place of the stop `take` hands out next.
struct Walk {
    stops: Vec<Stop>,
    next: usize,
}

impl Walk {
    /// The place of the stop being drawn, which is what `WhoModel::stops` lists at that place.
    fn take(&mut self, drawn: &Stop) -> usize {
        debug_assert_eq!(self.stops.get(self.next), Some(drawn), "the layout draws a stop the model does not list there");
        self.next += 1;
        self.next - 1
    }
}

/// The width the feature's name has in the grid, the same for every row, so the people's columns line up.
fn name_width(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 3. + look.length(token::SPACE_6)
}

/// The chip's own words and what a tooltip says of it.
fn chip_words(mark: &Mark) -> (String, Option<String>) {
    match mark {
        Mark::Pull(pull) => (format!("#{}", pull.number), Some(if pull.draft { format!("Draft: {}", pull.title) } else { pull.title.clone() })),
        Mark::Job { title, .. } => ("Job".into(), title.clone()),
    }
}

/// What a screen reader says of a chip.
fn chip_name(mark: &Mark) -> String {
    match mark {
        Mark::Pull(pull) if pull.draft => format!("Draft pull request #{}, {}", pull.number, pull.title),
        Mark::Pull(pull) => format!("Pull request #{}, {}", pull.number, pull.title),
        Mark::Job { title: Some(title), .. } => format!("Job: {title}"),
        Mark::Job { .. } => "Job".into(),
    }
}

fn avatar(look: &Look, person: &Person) -> Div {
    let (ground, ink) = if person.me { (look.color(token::STATUS_COMPLETED_SUCCESS).opacity(0.16), look.color(token::STATUS_COMPLETED_SUCCESS)) } else { (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT)) };
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_BADGE))
        .rounded(look.length(token::H_BADGE) / 2.)
        .bg(ground)
        .text_color(ink)
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .child(SharedString::from(person.initial.clone()))
}

impl ProjectPageView {
    /// The three-way control in the head: which layout the one answer is drawn in.
    pub(super) fn render_who_views(&self, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let mut track = segment_track(look, "who-views");
        for view in WhoView::ALL {
            let said = match view {
                WhoView::Grid => "Features down the side, people across",
                WhoView::ByFeature => "Each feature with the people on it",
                WhoView::ByPerson => "Each person with their features",
            };
            let one = segment(look, SharedString::from(format!("who-view-{}", view.title())), view.title().into(), if view == self.view { Pick::Set } else { Pick::No }, false)
                .tooltip(tip(said))
                .on_click(cx.listener(move |this, _, _, cx| this.set_view(view, cx)));
            track = track.child(labelled(one, Role::RadioButton, view.title()));
        }
        labelled(track, Role::RadioGroup, "Layout").into_any_element()
    }

    /// A chip: a pull request's number, open or draft, or the owner's Job. A press opens what it names.
    fn who_chip(&self, look: &Look, walk: &mut Walk, mark: &Mark, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let at = walk.take(&Stop::of(mark));
        let (words, said) = chip_words(mark);
        let (ground, ink) = match mark {
            Mark::Job { .. } => (look.color(token::STATUS_COMPLETED_SUCCESS).opacity(0.16), look.color(token::STATUS_COMPLETED_SUCCESS)),
            Mark::Pull(pull) if pull.draft => (look.color(token::BG_HOVER), look.color(token::FG_MUTED)),
            Mark::Pull(_) => (look.color(token::STATUS_RUNNING).opacity(0.16), look.color(token::STATUS_RUNNING)),
        };
        let under_keys = focused && self.at == Some(at);
        let ring = ring_colour(look);
        let chip = div()
            .id(SharedString::from(format!("who-stop-{at}")))
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_BADGE))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .bg(ground)
            .text_color(ink)
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .whitespace_nowrap()
            .cursor_pointer()
            .map(|chip| if under_keys { chip.shadow(selected_shadow(look, Some(ring))) } else { chip.hover(|style| style.opacity(0.8)) })
            .when_some(said, |chip, said| chip.tooltip(tip(said)))
            .on_click(cx.listener(move |this, _, _, cx| this.pick(at, cx)))
            .child(SharedString::from(words));
        labelled(chip, Role::Button, chip_name(mark)).into_any_element()
    }

    /// The chips of one cell, together.
    fn who_chips(&self, look: &Look, walk: &mut Walk, marks: &[Mark], focused: bool, cx: &mut gpui::Context<Self>) -> Div {
        let mut chips = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_1));
        for mark in marks {
            chips = chips.child(self.who_chip(look, walk, mark, focused, cx));
        }
        chips
    }

    /// A feature's name: a press opens its page. Where two or more people are on it, it is in the awaiting-review colour.
    fn who_feature(&self, look: &Look, walk: &mut Walk, slug: &str, title: &str, shared: Option<&str>, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let at = walk.take(&Stop::Feature(slug.to_string()));
        let under_keys = focused && self.at == Some(at);
        let ring = ring_colour(look);
        let ink = if shared.is_some() { look.color(token::STATUS_AWAITING_REVIEW) } else { look.color(token::FG_DEFAULT) };
        let name = div()
            .id(SharedString::from(format!("who-stop-{at}")))
            .min_w_0()
            .rounded(look.length(token::RADIUS_SM))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .cursor_pointer()
            .map(|name| if under_keys { name.shadow(selected_shadow(look, Some(ring))) } else { name.hover(|style| style.underline()) })
            .when_some(shared.map(str::to_string), |name, people| name.tooltip(tip(people)))
            .on_click(cx.listener(move |this, _, _, cx| this.pick(at, cx)))
            .child(SharedString::from(title.to_string()));
        labelled(name, Role::Button, title.to_string()).into_any_element()
    }

    // ---- Grid --------------------------------------------------------------------------------

    fn render_grid(&self, look: &Look, model: &WhoModel, walk: &mut Walk, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let heads = model.people.iter().fold(div().w_full().flex().gap(look.length(token::SPACE_3)).pb(look.length(token::SPACE_2)).child(div().flex_none().w(name_width(look))), |row, person| {
            row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(person.name.clone())),
            )
        });
        let mut body = inner(look).child(labelled(heads.id("who-grid-heads"), Role::Row, "People"));
        for feature in &model.features {
            let shared = model.overlap_said(feature);
            let mut row = div().w_full().flex().items_start().gap(look.length(token::SPACE_3)).pt(look.length(token::SPACE_2)).border_t(edge).border_color(look.color(token::BORDER_SUBTLE));
            row = row.child(div().flex_none().w(name_width(look)).flex().child(self.who_feature(look, walk, &feature.slug, &feature.title, shared.as_deref(), focused, cx)));
            for person in 0..model.people.len() {
                let marks: &[Mark] = feature.cells.iter().find(|cell: &&Cell| cell.person == person).map_or(&[], |cell| cell.marks.as_slice());
                row = row.child(div().flex_1().min_w_0().child(self.who_chips(look, walk, marks, focused, cx)));
            }
            body = body.child(labelled(row.id(SharedString::from(format!("who-grid-{}", feature.slug))), Role::Row, feature.title.clone()));
        }
        body = body.child(self.render_who_key(look, model));
        labelled(shell(look, "who-grid", false, false).child(body), Role::Region, "Who's on what").into_any_element()
    }

    /// What the chips and the colour of a name mean.
    fn render_who_key(&self, look: &Look, model: &WhoModel) -> Div {
        let sample = |ground: gpui::Hsla, ink: gpui::Hsla, words: &'static str| {
            div().flex_none().flex().items_center().h(look.length(token::H_BADGE)).px(look.length(token::SPACE_2)).rounded(look.length(token::RADIUS_SM)).bg(ground).text_color(ink).font_family(mono(look)).child(words)
        };
        let part = |chip: Div, said: &'static str| div().flex().items_center().gap(look.length(token::SPACE_1) + px(1.)).child(chip).child(said);
        let owner = model.people.iter().any(|person| person.me);
        let mut key = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_4))
            .gap_y(look.length(token::SPACE_1))
            .pt(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE));
        if owner {
            key = key.child(part(sample(look.color(token::STATUS_COMPLETED_SUCCESS).opacity(0.16), look.color(token::STATUS_COMPLETED_SUCCESS), "Job"), "yours"));
        }
        key = key.child(part(sample(look.color(token::STATUS_RUNNING).opacity(0.16), look.color(token::STATUS_RUNNING), "#0"), "open"));
        key = key.child(part(sample(look.color(token::BG_HOVER), look.color(token::FG_MUTED), "#0"), "draft"));
        key.child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::STATUS_AWAITING_REVIEW)).child("more than one person"))
    }

    // ---- By feature --------------------------------------------------------------------------

    fn render_by_feature(&self, look: &Look, model: &WhoModel, walk: &mut Walk, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let mut body = inner(look).gap(look.length(token::SPACE_1));
        for (place, feature) in model.features.iter().enumerate() {
            let shared = model.overlap_said(feature);
            let mut head = div().w_full().flex().items_center().justify_between().gap(look.length(token::SPACE_3)).pb(look.length(token::SPACE_1));
            head = head.child(self.who_feature(look, walk, &feature.slug, &feature.title, shared.as_deref(), focused, cx));
            if let Some(people) = shared.clone() {
                head = head.child(
                    div()
                        .flex_none()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                        .child(SharedString::from(people)),
                );
            }
            let mut block = div().w_full().flex().flex_col().py(look.length(token::SPACE_2)).when(place > 0, |block| block.border_t(edge).border_color(look.color(token::BORDER_SUBTLE))).child(head);
            for cell in &feature.cells {
                let person = &model.people[cell.person];
                for mark in &cell.marks {
                    block = block.child(self.who_line(look, walk, person, mark, focused, cx));
                }
            }
            body = body.child(labelled(block.id(SharedString::from(format!("who-feature-{}", feature.slug))), Role::Group, feature.title.clone()));
        }
        labelled(shell(look, "who-by-feature", false, false).child(body), Role::Region, "Who's on what, by feature").into_any_element()
    }

    /// A person's line on a feature: the initial, what the pull request or Job is, and its chip.
    fn who_line(&self, look: &Look, walk: &mut Walk, person: &Person, mark: &Mark, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let title = match mark {
            Mark::Pull(pull) => Some(pull.title.clone()),
            Mark::Job { title, .. } => title.clone(),
        };
        let line = div()
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1) + px(1.))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .child(labelled(avatar(look, person).id(SharedString::from(format!("who-avatar-{}-{}", person.login, walk.next))), Role::Image, person.name.clone()))
            .child(div().flex_1().min_w_0().truncate().children(title.map(SharedString::from)))
            .child(self.who_chip(look, walk, mark, focused, cx));
        line.into_any_element()
    }

    // ---- By person ---------------------------------------------------------------------------

    fn render_by_person(&self, look: &Look, model: &WhoModel, walk: &mut Walk, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let mut grid = div().w_full().flex().flex_col().gap(look.length(token::SPACE_3));
        for pair in model.cards.chunks(2) {
            let mut line = div().w_full().flex().items_start().gap(look.length(token::SPACE_3));
            for card in pair {
                let person = &model.people[card.person];
                let edge = if card.overlap { look.color(token::STATUS_AWAITING_REVIEW) } else { look.color(token::BORDER_SUBTLE) };
                let mut one = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(edge)
                    .bg(look.color(token::BG_SUNKEN))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .child(avatar(look, person))
                            .child(SharedString::from(person.name.clone())),
                    );
                for held in &card.held {
                    let shared = model.features.iter().find(|feature| feature.slug == held.slug).and_then(|feature| model.overlap_said(feature));
                    one = one.child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap(look.length(token::SPACE_2))
                            .child(self.who_feature(look, walk, &held.slug, &held.title, shared.as_deref(), focused, cx))
                            .child(self.who_chips(look, walk, &held.marks, focused, cx).justify_end()),
                    );
                }
                line = line.child(labelled(one.id(SharedString::from(format!("who-person-{}", person.login))), Role::Group, person.name.clone()));
            }
            // A short last line keeps the cards' width, so the two columns stay the same.
            for _ in pair.len()..2 {
                line = line.child(div().flex_1().min_w_0());
            }
            grid = grid.child(line);
        }
        labelled(grid.id("who-by-person"), Role::Region, "Who's on what, by person").into_any_element()
    }

    // ---- the page ----------------------------------------------------------------------------

    pub(super) fn render_who(&self, look: &Look, focused: bool, _window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let model = match &self.who {
            Load::Reading => return self.render_loading(look),
            Load::Failed(why) => return not_read(look, why),
            Load::Read(read) => who_model::who(read, &self.beside.jobs),
        };
        if model.features.is_empty() {
            return said_line(look, "who-empty", who_model::EMPTY);
        }
        let mut walk = Walk { stops: model.stops(self.view), next: 0 };
        let font = sans(look);
        let shown = match self.view {
            WhoView::Grid => self.render_grid(look, &model, &mut walk, focused, cx),
            WhoView::ByFeature => self.render_by_feature(look, &model, &mut walk, focused, cx),
            WhoView::ByPerson => self.render_by_person(look, &model, &mut walk, focused, cx),
        };
        debug_assert_eq!(walk.next, walk.stops.len(), "the layout draws every stop the model lists");
        div().w_full().font_family(font).child(shown).into_any_element()
    }
}
