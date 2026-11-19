//! The project's pages drawn from what `project_model.rs` made of Fleet's read, in the cards the feature page uses: the Overview
//! (the README's sentence, the code map with a key to its hues, the busiest features and the glossary) and Since you looked
//! (a paragraph, then a row for each feature that moved). Every value is a token.

use gpui::{div, prelude::*, px, AnyElement, FontWeight, HighlightStyle, IntoElement, Pixels, Render, Role, SharedString, StyledText, Window};

use super::dossier::{button, hue_square, pill, selected_shadow, tint};
use super::page_view::{head, inner, shell};
use super::project_model::{self, Glossary, Moved, OverviewModel, ProjectPage, Tile};
use super::project_page::ProjectPageView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::reviews::panel_inner_radius;
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{mono, sans};

/// The sides of the page the cards stand in, as the feature page's.
fn side(look: &Look) -> Pixels {
    look.length(token::SPACE_6) + look.length(token::SPACE_1)
}

/// The width of the chip that names why a feature is on the page: the same for every row, so the titles line up.
fn tag_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) + look.length(token::SPACE_6)
}

/// The width of the column the pull request numbers stand in.
fn pulls_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 2. + look.length(token::SPACE_4)
}

/// The least width a tile of the code map has.
fn tile_least(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 3.
}

/// A card that has not read yet: its title and what Fleet said.
pub(super) fn not_read(look: &Look, why: &str) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .p(look.length(token::SPACE_4))
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_MUTED))
        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
        .child(labelled(div().id("project-not-read-why").child(SharedString::from(why.to_string())), Role::Label, why.to_string()));
    shell(look, "project-not-read", false, false).child(body).into_any_element()
}

/// One quiet line in a card: why it holds nothing and what fills it.
pub(super) fn said_line(look: &Look, id: &'static str, said: impl Into<SharedString>) -> AnyElement {
    let said: SharedString = said.into();
    labelled(
        div()
            .id(id)
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(said.clone()),
        Role::Label,
        said,
    )
    .into_any_element()
}

fn card(look: &Look, key: &'static str, title: &'static str) -> gpui::Stateful<gpui::Div> {
    shell(look, SharedString::from(format!("project-card-{key}")), false, false).child(head(look, key, title, false, true))
}

fn tile(look: &Look, tile: &Tile) -> AnyElement {
    let band = tile.hue.map_or_else(|| look.color(token::BORDER_DEFAULT), |hue| look.color(hue));
    let body = div()
        .id(SharedString::from(format!("overview-room-{}", tile.path)))
        .flex_1()
        .min_w_0()
        .flex()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .overflow_hidden()
        .tooltip(tip(tile.path.clone()))
        .child(div().flex_none().w(look.length(token::SPACE_1)).bg(band))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1) + px(1.))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_MUTED))
                .child(SharedString::from(tile.module.clone())),
        );
    labelled(body, Role::ListItem, tile.said.clone()).into_any_element()
}

/// The lede: the README's sentence, with where it came from quiet after it. None draws no lede.
fn lede(look: &Look, sentence: &str, file: &str) -> AnyElement {
    let said = format!("{sentence} From {file}");
    let quiet = HighlightStyle { color: Some(look.color(token::FG_SUBTLE)), ..Default::default() };
    let text = StyledText::new(SharedString::from(said.clone())).with_highlights([(sentence.len() + 1..said.len(), quiet)]);
    labelled(div().id("overview-lede").text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(text), Role::Label, said).into_any_element()
}

impl ProjectPageView {
    /// The strip over the page: its title, the time he last looked under it where it is Since you looked, and Mark read at the right.
    fn render_top(&self, look: &Look, focused: bool, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let outer = if self.beside.flush { panel_inner_radius(look) } else { px(0.) };
        let round = (outer - look.length(token::EDGE_ACTIVE)).max(px(0.));
        let title: SharedString = match self.which {
            ProjectPage::Overview if !self.beside.name.is_empty() => self.beside.name.clone().into(),
            page => page.title().into(),
        };
        let heading = labelled(
            div().id("project-page-title").text_size(look.length(token::TEXT_LG)).line_height(look.length(token::LEADING_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()),
            Role::Heading,
            title,
        );
        let when = (self.which == ProjectPage::Since).then(|| {
            let said = SharedString::from(project_model::when_said(self.since_at));
            labelled(div().id("project-page-looked").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(said.clone()), Role::Label, said)
        });
        let sub = (self.which == ProjectPage::Who).then(|| {
            let said = SharedString::from(project_model::WHO_SUB);
            labelled(div().id("project-page-sub").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(said.clone()), Role::Label, said)
        });
        let views = (self.which == ProjectPage::Who && matches!(&self.who, super::page::Load::Read(read) if !read.features.is_empty())).then(|| self.render_who_views(look, cx));
        let mark = (self.which == ProjectPage::Since).then(|| {
            let ringed = focused && self.at == Some(0);
            let ring = ring_colour(look);
            let pill = button(look, "project-page-mark-read", "Mark read", false, true, false, None)
                .tooltip(tip("Set the time you last looked to now"))
                .when(ringed, move |pill| pill.border_color(ring))
                .on_click(cx.listener(|this, _, _, cx| this.mark_read(cx)));
            labelled(pill, Role::Button, "Mark read")
        });
        div()
            .id("project-page-top")
            .flex_none()
            .flex()
            .items_start()
            .justify_between()
            .gap(look.length(token::SPACE_6))
            .px(side(look))
            .pt(look.length(token::SPACE_6) - px(2.))
            .pb(look.length(token::SPACE_4) + px(2.))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .rounded_t(round)
            .bg(tint(look, look.color(token::BG_RAISED), token::ACCENT, 0.08))
            .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).min_w_0().child(heading).children(when).children(sub))
            .children(mark)
            .children(views)
    }

    pub(super) fn render_loading(&self, look: &Look) -> AnyElement {
        let mut cards = div().w_full().flex().flex_col().gap(look.length(token::SPACE_4));
        for at in 0..2 {
            let mut one = shell(look, SharedString::from(format!("project-card-bone-{at}")), false, false).child(
                div().flex_none().px(look.length(token::SPACE_3) + px(2.)).py(look.length(token::SPACE_3)).border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(bone(look, px(96.), look.length(token::LEADING_XS) - px(6.))),
            );
            for row in 0..3 {
                one = one.child(
                    div()
                        .id(SharedString::from(format!("project-bone-{at}-{row}")))
                        .px(look.length(token::SPACE_3) + px(2.))
                        .py(look.length(token::SPACE_2) + px(2.))
                        .child(bone(look, px(160. + 40. * ((at + row) % 3) as f32), look.length(token::LEADING_XS) - px(4.))),
                );
            }
            cards = cards.child(one);
        }
        labelled(cards.id("project-page-loading"), Role::Image, "Loading").into_any_element()
    }

    // ---- Overview ----------------------------------------------------------------------------

    fn render_map(&self, look: &Look, model: &OverviewModel, width: Pixels) -> AnyElement {
        let pad = look.length(token::SPACE_3) + px(2.);
        let gap = look.length(token::SPACE_2);
        let columns = (((width - pad * 2. + gap) / (tile_least(look) + gap)).floor() as usize).clamp(2, 6);
        let mut grid = div().flex().flex_col().gap(gap);
        for line in model.tiles.chunks(columns) {
            let mut row = div().flex().gap(gap);
            for one in line {
                row = row.child(tile(look, one));
            }
            // A short last line keeps the tiles' width, so the columns line up down the map.
            for _ in line.len()..columns {
                row = row.child(div().flex_1().min_w_0());
            }
            grid = grid.child(row);
        }
        let mut key = div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_4)).gap_y(look.length(token::SPACE_1));
        for area in &model.key {
            key = key.child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(hue_square(look, area.hue, look.length(token::SPACE_2)))
                    .child(SharedString::from(area.title.clone())),
            );
        }
        let body = inner(look).child(grid).when(!model.key.is_empty(), |body| body.child(key));
        labelled(card(look, "map", "Code map").child(body), Role::Region, "Code map").into_any_element()
    }

    fn render_busiest(&self, look: &Look, model: &OverviewModel, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let mut body = div().flex().flex_col().py(look.length(token::SPACE_1)).px(look.length(token::SPACE_2));
        if model.busiest.is_empty() {
            body = body.child(said_line(look, "overview-busiest-empty", project_model::BUSIEST_EMPTY));
        }
        for (at, one) in model.busiest.iter().enumerate() {
            let under_keys = focused && self.at == Some(at);
            let ring = ring_colour(look);
            let row = div()
                .id(SharedString::from(format!("overview-busy-{}", one.slug)))
                .w_full()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .cursor_pointer()
                .map(|row| {
                    if under_keys {
                        row.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).shadow(selected_shadow(look, Some(ring)))
                    } else {
                        row.text_color(look.color(token::FG_DEFAULT)).hover(|style| style.bg(look.color(token::BG_HOVER)))
                    }
                })
                .on_click(cx.listener(move |this, _, _, cx| this.pick(at, cx)))
                .child(div().flex_1().min_w_0().child(SharedString::from(one.title.clone())))
                .child(
                    div()
                        .flex_none()
                        .w(pulls_width(look))
                        .flex()
                        .justify_end()
                        .text_right()
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(SharedString::from(one.pulls.clone())),
                );
            body = body.child(labelled(row, Role::Button, one.title.clone()));
        }
        labelled(card(look, "busiest", "Busiest features").child(body), Role::Region, "Busiest features").into_any_element()
    }

    fn render_glossary(&self, look: &Look, model: &OverviewModel) -> AnyElement {
        let body = match &model.glossary {
            Glossary::Empty(said) => said_line(look, "overview-glossary-empty", *said),
            Glossary::Terms(terms) => {
                let mut list = inner(look);
                for (term, meaning) in terms {
                    list = list.child(
                        div()
                            .flex()
                            .flex_col()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(term.clone())))
                            .child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(meaning.clone()))),
                    );
                }
                list.into_any_element()
            }
        };
        labelled(card(look, "glossary", "Glossary").child(body), Role::Region, "Glossary").into_any_element()
    }

    fn render_overview(&self, look: &Look, focused: bool, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let model = match &self.overview {
            super::page::Load::Reading => return self.render_loading(look),
            super::page::Load::Failed(why) => return not_read(look, why),
            super::page::Load::Read(read) => project_model::overview(read, &self.beside.areas),
        };
        let width = (window.viewport_size().width - self.beside.inset - side(look) * 2.).max(px(0.));
        let half = |card: AnyElement| div().flex_1().min_w_0().child(card);
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(model.lede.as_ref().map(|(sentence, file)| lede(look, sentence, file)))
            .child(self.render_map(look, &model, width))
            .child(div().w_full().flex().items_start().gap(look.length(token::SPACE_4)).child(half(self.render_busiest(look, &model, focused, cx))).child(half(self.render_glossary(look, &model))))
            .into_any_element()
    }

    // ---- Since you looked --------------------------------------------------------------------

    fn render_moved(&self, look: &Look, at: usize, one: &Moved, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let (words, hue) = project_model::tag_mark(one.tag);
        // The keys stop on Mark read first, so a row's place is one on.
        let place = at + 1;
        let under_keys = focused && self.at == Some(place);
        let ring = ring_colour(look);
        let chip = pill(look, hue, words).w(tag_width(look)).justify_center();
        let mut text = div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1) - px(1.)).child(
            div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.title.clone())),
        );
        if !one.what.is_empty() {
            text = text.child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(one.what.clone())));
        }
        if !one.pulls.is_empty() {
            text = text.child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(one.pulls.clone())));
        }
        let row = div()
            .id(SharedString::from(format!("since-row-{}", one.slug)))
            .w_full()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .cursor_pointer()
            .map(|row| {
                if under_keys {
                    row.bg(look.color(token::BG_HOVER)).shadow(selected_shadow(look, Some(ring)))
                } else {
                    row.hover(|style| style.bg(look.color(token::BG_HOVER)))
                }
            })
            .on_click(cx.listener(move |this, _, _, cx| this.pick(place, cx)))
            .child(div().flex_none().w(tag_width(look)).child(chip))
            .child(text);
        let name = format!("{words}: {}", one.title);
        labelled(row, Role::Button, name).into_any_element()
    }

    fn render_since(&self, look: &Look, focused: bool, cx: &mut gpui::Context<Self>) -> AnyElement {
        let model = match &self.since {
            super::page::Load::Reading => return self.render_loading(look),
            super::page::Load::Failed(why) => return not_read(look, why),
            super::page::Load::Read(read) => project_model::since(read),
        };
        let paragraph = model.paragraph.as_ref().map(|text| {
            let ground = tint(look, look.color(token::BG_SUNKEN), token::ACCENT, 0.10);
            labelled(
                div()
                    .id("since-paragraph")
                    .w_full()
                    .px(look.length(token::SPACE_3) + px(2.))
                    .py(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(ground)
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(text.clone())),
                Role::Label,
                text.clone(),
            )
        });
        let rows = if model.rows.is_empty() {
            shell(look, "since-rows", false, false).child(said_line(look, "since-nothing", project_model::nothing_since(self.since_at))).into_any_element()
        } else {
            let mut list = div().flex().flex_col().p(look.length(token::SPACE_2));
            for (at, one) in model.rows.iter().enumerate() {
                list = list.child(self.render_moved(look, at, one, focused, cx));
            }
            labelled(shell(look, "since-rows", false, false).child(list), Role::Region, "Features that moved").into_any_element()
        };
        div().w_full().flex().flex_col().gap(look.length(token::SPACE_4)).children(paragraph).child(rows).into_any_element()
    }
}

impl Render for ProjectPageView {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let focused = self.body.is_focused(window);
        let top = self.render_top(&look, focused, cx);
        let content = match self.which {
            ProjectPage::Overview => self.render_overview(&look, focused, window, cx),
            ProjectPage::Since => self.render_since(&look, focused, cx),
            ProjectPage::Who => self.render_who(&look, focused, window, cx),
        };
        div()
            .id("project-page-view")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
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
                    .id("project-page-body")
                    .track_focus(&self.body)
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
