//! The Features tab drawn from what `model.rs` made of Fleet's read: area heads with their features under
//! them in the left column, and the slot the feature's page fills on the right. Every value is a token.

use gpui::{div, list, prelude::*, px, relative, Div, FontWeight, IntoElement, Pixels, Render, Role, SharedString, Window};
use gpui_component::input::Input;

use super::model::{narrow_list_width, Line, Shown};
use super::{FeaturesScreen, Load};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::sans;

/// A filled chip: its words in a hue on a tint of that hue (`color-mix(... 14%, transparent)`).
fn chip(look: &Look, hue: Token, words: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(hue).opacity(0.14))
        .text_color(look.color(hue))
        .text_size(look.length(token::TEXT_2XS))
        .whitespace_nowrap()
        .child(div().min_w_0().truncate().child(words.into()))
}

/// What the list column needs at least: the two fixed columns, the gaps and edges of a row, and two token-steps of room
/// for a title, where a title is cut with an ellipsis.
pub(super) fn list_need(look: &Look) -> Pixels {
    let step = look.length(token::SPACE_12);
    step * 3. + step * 2.5 + look.length(token::SPACE_2) * 2. + look.length(token::SPACE_3) * 2. + look.length(token::BORDER_WIDTH) + step * 2.
}

/// The column who is on a feature now has: a chip naming the first Job, a mark that there are more, and every
/// Job in the tooltip. Fixed in width, so the chips line up down the list, and empty where nobody is on it.
fn on_column(look: &Look, shown: &Shown) -> gpui::AnyElement {
    let cell = div().flex_none().w(look.length(token::SPACE_12) * 3.).min_w_0().flex().items_center().gap(look.length(token::SPACE_1)).overflow_hidden();
    let Some(first) = shown.on.first() else { return cell.into_any_element() };
    let said = shown.on_said();
    let cell = cell
        .id(SharedString::from(format!("feature-on-{}", shown.row.slug)))
        .tooltip(tip(said.clone()))
        .child(chip(look, token::STATUS_RUNNING, first.clone()).flex_shrink(1.).min_w_0())
        .when(shown.on.len() > 1, |cell| cell.child(chip(look, token::STATUS_RUNNING, "…")));
    labelled(cell, Role::Image, format!("On it now: {}", shown.on.join(", "))).into_any_element()
}

/// The column the stale mark has, fixed and empty where nothing is stale.
fn stale_column(look: &Look, shown: &Shown) -> gpui::AnyElement {
    let cell = div().flex_none().w(look.length(token::SPACE_12) * 2.5).flex().items_center();
    let Some(stale) = &shown.stale else { return cell.into_any_element() };
    let cell = cell.id(SharedString::from(format!("feature-stale-{}", shown.row.slug))).tooltip(tip("A written section is older than the last merge that changed its files")).child(chip(look, token::STATUS_AWAITING_REVIEW, stale.clone()));
    labelled(cell, Role::Image, stale.clone()).into_any_element()
}

impl FeaturesScreen {
    fn render_bones(&self, look: &Look) -> Div {
        let mut bones = div().flex().flex_col();
        for area in 0..3 {
            bones = bones.child(div().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).bg(look.color(token::BG_SUNKEN)).child(bone(look, px(84.), look.length(token::LEADING_2XS))));
            for row in 0..3 {
                bones = bones.child(
                    div()
                        .id(SharedString::from(format!("features-bone-{area}-{row}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .border_t(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(div().flex_1().child(bone(look, px(120. + 36. * ((area + row) % 3) as f32), look.length(token::LEADING_XS))))
                        .child(bone(look, look.length(token::SPACE_12) * 2., look.length(token::H_BADGE))),
                );
            }
        }
        bones
    }

    fn render_line(&mut self, ix: usize, focused: bool, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let look = cx.look().clone();
        let look = &look;
        match self.listed.lines.get(ix).cloned() {
            Some(Line::Head { slug, title }) => labelled(
                div()
                    .id(SharedString::from(format!("features-area-{slug}")))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1) + px(2.))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(title.to_uppercase())),
                Role::Heading,
                title,
            )
            .into_any_element(),
            Some(Line::Feature(at)) => match self.listed.shown.get(at).cloned() {
                Some(shown) => self.render_feature(&shown, focused, look, cx),
                None => div().into_any_element(),
            },
            None => div().into_any_element(),
        }
    }

    /// One feature: its title, who is on it now and its stale mark. The one opened wears the accent and its edge,
    /// the one the keys are on a lighter ground with the focus colour's edge.
    fn render_feature(&self, shown: &Shown, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let slug = shown.row.slug.clone();
        let opened = self.opened.as_deref() == Some(slug.as_str());
        let under_keys = self.cursor.as_deref() == Some(slug.as_str());
        let edge = if opened {
            Some(look.color(token::ACCENT))
        } else if under_keys && focused {
            Some(ring_colour(look))
        } else {
            None
        };
        let press = slug.clone();
        let row = div()
            .id(SharedString::from(format!("feature-{slug}")))
            .relative()
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .map(|row| {
                if opened {
                    row.bg(look.color(token::ACCENT).opacity(0.22))
                } else if under_keys {
                    row.bg(look.color(token::BG_HOVER))
                } else {
                    row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)))
                }
            })
            .aria_selected(opened)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_at(press.clone(), window, cx);
                this.engage_page(cx);
                this.focus_list(window, cx);
            }))
            .children(edge.map(|colour| div().absolute().left_0().top_0().bottom_0().w(look.length(token::EDGE_ACTIVE)).bg(colour)))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(shown.row.title.clone())))
            .child(on_column(look, shown))
            .child(stale_column(look, shown));
        labelled(row, Role::ListBoxOption, shown.row.title.clone()).into_any_element()
    }

    fn render_list(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let filter = labelled(
            div().id("features-filter").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&self.filter)),
            Role::SearchInput,
            "Filter features",
        );
        let top = div()
            .flex_none()
            .flex()
            .items_center()
            .p(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(filter);
        let body = match &self.library {
            Load::Reading => labelled(self.render_bones(look).id("features-loading"), Role::Image, "Loading").into_any_element(),
            Load::Failed(why) => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_4))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                .child(SharedString::from(why.clone()))
                .into_any_element(),
            Load::Read(_) => {
                let focused = self.list_focus.is_focused(window);
                labelled(
                    div().id("features-list").size_full().child(list(self.list.clone(), cx.processor(move |this, ix: usize, _, cx| this.render_line(ix, focused, cx))).size_full()),
                    Role::ListBox,
                    "Features",
                )
                .into_any_element()
            }
        };
        div()
            .id("features-column")
            .track_focus(&self.list_focus)
            .flex_none()
            .map(|column| {
                // The page wants its rail, 240, and 480 beside it for the asks: fifteen token-steps. Where the screen is
                // narrower than a thousand the list gives up its floor for it, and keeps what its columns need.
                match narrow_list_width(window.viewport_size().width, look.length(token::W_DIALOG), list_need(look), look.length(token::SPACE_12) * 15.) {
                    Some(narrow) => column.w(narrow),
                    // Below this the fixed columns would leave the titles no room.
                    None => column.w(relative(0.36)).min_w(look.length(token::W_DIALOG)),
                }
            })
            .h_full()
            .flex()
            .flex_col()
            .border_r(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(top)
            .child(div().flex_1().min_h_0().child(body))
    }

    /// The right of the list. A feature's page is drawn here once one is open.
    fn render_page_slot(&self, look: &Look) -> impl IntoElement {
        labelled(div().id("feature-page").flex_1().min_w_0().h_full().bg(look.color(token::BG_BASE)).children(self.page.clone()), Role::Region, "Feature page")
    }
}

impl Render for FeaturesScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let list = self.render_list(&look, window, cx);
        let page = self.render_page_slot(&look);
        div()
            .id("features")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .relative()
            .size_full()
            .flex()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(list)
            .child(page)
    }
}
