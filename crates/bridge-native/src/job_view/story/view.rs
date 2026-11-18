//! The Story tab drawn from what `model.rs` made of Fleet's read: asks as raised cards, each a head line that unfolds to
//! its rows. Every value is a token.

use gpui::{div, list, prelude::*, px, AnyElement, Div, FontWeight, Hsla, IntoElement, Render, Role, SharedString, Window};

use super::model::{Card, Row};
use super::{Load, Region, StoryTab};
use crate::features::asks::hue_token;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{glyph, sans};

/// The edge of the unfolded ask: the accent at 45% over the default edge.
fn opened_edge(look: &Look) -> Hsla {
    look.color(token::BORDER_DEFAULT).blend(look.color(token::ACCENT).opacity(0.45))
}

/// A small chip with a quiet fill, as the feature page's area chip is.
fn chip(look: &Look, words: &'static str) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::FG_DEFAULT).opacity(0.08))
        .text_size(look.length(token::TEXT_2XS))
        .text_color(look.color(token::FG_MUTED))
        .whitespace_nowrap()
        .child(words)
}

fn muted_line(look: &Look, words: &str) -> Div {
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(words.to_string()))
}

impl StoryTab {
    /// One row under an unfolded ask: a 16px glyph in its hue, the words and the lines under them. A Filed under row is a link.
    fn render_row(&self, ix: usize, row: &Row, picked: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let id = SharedString::from(format!("story-row-{ix}-{}", row.key));
        let ink = if row.feature.is_some() {
            look.color(token::ACCENT)
        } else if row.tinted {
            look.color(hue_token(row.hue))
        } else {
            look.color(token::FG_DEFAULT)
        };
        let mark = labelled(
            div()
                .id(SharedString::from(format!("{id}-mark")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::SPACE_4))
                .mt(px(2.))
                .tooltip(tip(row.said.clone()))
                .child(glyph(row.glyph, look.length(token::SPACE_4), look.color(hue_token(row.hue)))),
            Role::Image,
            row.said.clone(),
        );
        let mut text = div().flex_1().min_w_0().flex().flex_col().gap(px(2.)).child(div().text_color(ink).child(SharedString::from(row.text.clone())));
        if !row.meta.is_empty() {
            text = text.child(muted_line(look, &row.meta));
        }
        if !row.also.is_empty() {
            text = text.child(muted_line(look, &row.also));
        }
        let line = div()
            .id(id)
            .w_full()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
            .when(picked, |line| line.bg(look.color(token::ACCENT).opacity(0.12)).rounded(look.length(token::RADIUS_MD)))
            .child(mark)
            .child(text);
        match row.feature.clone() {
            Some(slug) => labelled(
                line.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| {
                    this.region = Region::Links;
                    window.focus(&this.focus, cx);
                    this.open_feature(slug.clone(), window, cx);
                })),
                Role::Link,
                row.text.clone(),
            )
            .into_any_element(),
            None => line.into_any_element(),
        }
    }

    fn render_head(&self, ix: usize, card: &Card, open: bool, on: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let said = format!("{} {}", card.number, card.title);
        let state = labelled(
            div()
                .id(SharedString::from(format!("story-state-{ix}")))
                .flex_none()
                .w(look.length(token::SPACE_12) * 2.)
                .flex()
                .items_center()
                .justify_end()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(hue_token(card.stand.hue)))
                .tooltip(tip(card.stand.word))
                .child(glyph(card.stand.glyph, look.length(token::SPACE_3) + px(2.), look.color(hue_token(card.stand.hue))))
                .child(card.stand.word),
            Role::Image,
            card.stand.word,
        );
        let head = div()
            .id(SharedString::from(format!("story-head-{ix}")))
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .mx(-look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { ring_colour(look) } else { gpui::transparent_black() })
            .when(on, |head| head.bg(look.color(token::ACCENT).opacity(0.12)))
            .when(!on, |head| head.hover(|style| style.bg(look.color(token::BG_HOVER))))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .cursor_pointer()
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.focus, cx);
                this.toggle(ix, cx);
            }))
            // The title gives way before the state, which keeps its column whatever the title's length.
            .child(div().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone())))
            .child(chip(look, card.kind))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(card.last.clone())))
            .child(state);
        labelled(head, Role::Button, said).into_any_element()
    }

    fn render_card(&self, ix: usize, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Some(card) = self.built.cards.get(ix) else { return div().into_any_element() };
        let open = self.unfolded.as_deref() == Some(card.ask_id.as_str());
        let focused = self.focus.is_focused(window);
        let on = focused && self.cursor == Some(ix) && self.region == Region::Asks;
        let mut links = 0;
        let rows: Vec<AnyElement> = if open {
            card.rows
                .iter()
                .enumerate()
                .map(|(at, row)| {
                    let picked = focused && self.region == Region::Links && row.feature.is_some() && {
                        links += 1;
                        links - 1 == self.link_at
                    };
                    self.render_row(at, row, picked, look, cx)
                })
                .collect()
        } else {
            Vec::new()
        };
        let head = self.render_head(ix, card, open, on, look, cx);
        let body = div()
            .id(SharedString::from(format!("story-ask-{ix}")))
            .w_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if open { opened_edge(look) } else { look.color(token::BORDER_SUBTLE) })
            .rounded(look.length(token::RADIUS_LG))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .pb(if open { look.length(token::SPACE_1) } else { look.length(token::SPACE_3) })
            .child(div().pb(if open { look.length(token::SPACE_2) } else { px(0.) }).child(head))
            .children(rows);
        labelled(body, Role::ListBoxOption, format!("{} {}", card.number, card.title)).into_any_element()
    }

    /// What he was shown before any ask: an unnumbered group under its own heading.
    fn render_before(&self, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let rows: Vec<AnyElement> = self.built.before.iter().enumerate().map(|(at, row)| self.render_row(at, row, false, look, cx)).collect();
        div()
            .id("story-before")
            .w_full()
            .flex()
            .flex_col()
            .child(labelled(
                div()
                    .id("story-before-heading")
                    .pb(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Before any ask"),
                Role::Heading,
                "Before any ask",
            ))
            .children(rows)
            .into_any_element()
    }

    fn render_item(&self, ix: usize, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let inner = if ix < self.built.cards.len() { self.render_card(ix, look, window, cx) } else { self.render_before(look, cx) };
        div().w_full().px(look.length(token::SPACE_6)).pt(if ix == 0 { look.length(token::SPACE_4) } else { px(0.) }).pb(look.length(token::SPACE_4)).child(inner).into_any_element()
    }

    /// Cards of bone shapes, while the story is read.
    fn render_bones(&self, look: &Look) -> AnyElement {
        let mut bones = div().w_full().flex().flex_col().gap(look.length(token::SPACE_4)).px(look.length(token::SPACE_6)).py(look.length(token::SPACE_4));
        for at in 0..3 {
            bones = bones.child(
                div()
                    .id(SharedString::from(format!("story-bone-{at}")))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .bg(look.color(token::BG_RAISED))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .rounded(look.length(token::RADIUS_LG))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .child(bone(look, px(150. + 36. * (at % 3) as f32), look.length(token::LEADING_SM) - px(4.)))
                    .child(bone(look, look.length(token::SPACE_12) + look.length(token::SPACE_4), look.length(token::H_BADGE) - px(4.)))
                    .child(div().flex_1())
                    .child(bone(look, look.length(token::SPACE_12), look.length(token::LEADING_XS) - px(4.))),
            );
        }
        labelled(bones.id("story-loading"), Role::Image, "Loading").into_any_element()
    }

    fn render_body(&self, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        match &self.story {
            Load::Reading => self.render_bones(look),
            Load::Failed(why) => div()
                .px(look.length(token::SPACE_6))
                .py(look.length(token::SPACE_4))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                .child(SharedString::from(why.clone()))
                .into_any_element(),
            Load::Read(_) if self.places() == 0 => div().into_any_element(),
            Load::Read(_) => labelled(
                div()
                    .id("story-asks")
                    .size_full()
                    .child(list(self.list.clone(), cx.processor(move |this, ix: usize, window, cx| {
                        let look = cx.look().clone();
                        this.render_item(ix, &look, window, cx)
                    })).size_full()),
                Role::ListBox,
                "Asks",
            )
            .into_any_element(),
        }
    }
}

impl Render for StoryTab {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let body = self.render_body(&look, cx);
        let overlay = self.overlay().cloned().map(|overlay| div().id("story-overlay").absolute().top_0().left_0().size_full().occlude().child(overlay));
        div()
            .id(if self.nested { "story-nested" } else { "story-tab" })
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().flex_1().min_h_0().child(body))
            .children(overlay)
    }
}
