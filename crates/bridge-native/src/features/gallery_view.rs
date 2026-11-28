//! The gallery drawn: each area a card under a header band in its hue, its features as tiles in rows, and a feature opened as
//! one panel under a band of its area's hue, its pictures in cards and Also this? under them. Every value is a token.

use std::sync::Arc;

use chrono::Utc;
use gpui::{div, img, prelude::*, px, AnyElement, Div, Focusable as _, FontWeight, Image, IntoElement, ObjectFit, Pixels, Render, Role, SharedString, Window};
use gpui_component::input::Input;
use ipc::{FeaturePicture, PictureBy};

use super::dossier::{button, hue_among, hue_square, pill, pill_area, selected_shadow, tint};
use super::gallery::{columns, tile_width, Gallery, Kind, NOT_THIS, YES};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::reviews::card_inner_radius;
use crate::settings::view::{bone, ghost_with, ring_colour, tip};
use crate::titlebar::parts::{mono, sans};

/// A picture's height for its width: the shape of a laptop's screen, sixteen by ten.
fn screen_height(wide: Pixels) -> Pixels {
    wide * 10. / 16.
}

/// A card's width on a feature's own page: seven of the largest space.
fn card_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 7.
}

/// Where a picture was taken, as he would read it: the page's address without its scheme, or the native screen's name.
fn source(one: &FeaturePicture) -> String {
    match (&one.url, &one.screen) {
        (Some(url), _) => url.trim_start_matches("https://").trim_start_matches("http://").to_string(),
        (None, Some(screen)) => screen.clone(),
        (None, None) => String::new(),
    }
}

/// The well a picture sits in, sunken under its tile or card: the picture where there is one, cut to fill the well on a tile and
/// whole on a card, and a skeleton while it is read.
fn well(look: &Look, image: Option<Arc<Image>>, tall: Pixels, fit: ObjectFit) -> Div {
    let ground = look.color(token::BG_SUNKEN);
    let radius = look.length(token::RADIUS_MD);
    let loading = {
        let look = look.clone();
        move || bone(&look, px(0.), px(0.)).size_full().into_any_element()
    };
    div().w_full().h(tall).flex_none().rounded(radius).bg(ground).overflow_hidden().children(image.map(|image| img(image).size_full().rounded(radius).object_fit(fit).with_loading(loading)))
}

/// A feature with no picture: the well with a small window drawn in it, a bar across its top and lines under, and no words.
fn no_picture(look: &Look, title: &str, tall: Pixels) -> AnyElement {
    let line = |wide: f32| bone(look, px(0.), look.length(token::SPACE_1) + px(2.)).w(gpui::relative(wide));
    let window = div()
        .w(gpui::relative(0.46))
        .h(gpui::relative(0.56))
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .overflow_hidden()
        .child(div().flex_none().h(look.length(token::SPACE_3)).bg(look.color(token::BG_HOVER)))
        .child(div().flex_1().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_3)).child(line(0.8)).child(line(0.6)).child(line(0.7)));
    let shape = div().id(SharedString::from(format!("gallery-none-{title}"))).w_full().h(tall).flex_none().flex().items_center().justify_center().rounded(look.length(token::RADIUS_MD)).bg(look.color(token::BG_SUNKEN)).child(window);
    labelled(shape, Role::Image, format!("No picture of {title}")).into_any_element()
}

impl Gallery {
    // ---- the gallery --------------------------------------------------------------------------

    fn render_tile(&self, area: usize, at: usize, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Some(row) = self.areas.get(area).and_then(|one| one.features.get(at)) else { return div().into_any_element() };
        let wide = tile_width(look);
        let tall = screen_height(wide - look.length(token::SPACE_2) * 2.);
        let picture = match row.cover.as_ref().and_then(|id| self.images.get(id)) {
            Some(image) => well(look, Some(image.clone()), tall, ObjectFit::Cover).into_any_element(),
            None => no_picture(look, &row.title, tall),
        };
        let on_keys = focused && self.cursor == (area, at);
        let tile = div()
            .id(SharedString::from(format!("gallery-tile-{}", row.slug)))
            .flex_none()
            .w(wide)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .when(on_keys, |tile| tile.shadow(selected_shadow(look, Some(ring_colour(look)))))
            .on_click(cx.listener(move |this, _, window, cx| this.open((area, at), window, cx)))
            .child(picture)
            .child(
                div()
                    .px(look.length(token::SPACE_1))
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(row.title.clone())),
            );
        labelled(tile, Role::Button, row.title.clone()).into_any_element()
    }

    /// An area: a card with a header band across its top in the area's hue, and its tiles in rows of `cols`.
    fn render_area(&self, area: usize, cols: usize, focused: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Some(one) = self.areas.get(area) else { return div().into_any_element() };
        let hue = hue_among(self.areas.iter().map(|area| area.slug.as_str()), &one.slug);
        let band = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded_t(card_inner_radius(look))
            .bg(tint(look, look.color(token::BG_RAISED), hue, 0.18))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(hue).opacity(0.35))
            .child(hue_square(look, hue, look.length(token::SPACE_3)))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("gallery-area-title-{}", one.slug)))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(one.title.clone())),
                Role::Heading,
                one.title.clone(),
            ));
        let mut rows = div().id(SharedString::from(format!("gallery-tiles-{}", one.slug))).flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4));
        for (r, chunk) in one.features.chunks(cols).enumerate() {
            let mut line = div().flex().gap(look.length(token::SPACE_4));
            for c in 0..chunk.len() {
                line = line.child(self.render_tile(area, r * cols + c, focused, look, cx));
            }
            rows = rows.child(line);
        }
        div()
            .id(SharedString::from(format!("gallery-area-{}", one.slug)))
            .flex_none()
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_CARD))
            .child(band)
            .child(labelled(rows, Role::List, one.title.clone()))
            .into_any_element()
    }

    fn render_gallery(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let focused = self.focus.is_focused(window);
        let cols = columns(look, window.viewport_size().width);
        let mut sections = div().id("gallery-areas").track_scroll(&self.scroll).overflow_y_scroll().size_full().flex().flex_col().gap(look.length(token::GAP_SECTION)).p(look.length(token::SPACE_6));
        for area in 0..self.areas.len() {
            sections = sections.child(self.render_area(area, cols, focused, look, cx));
        }
        labelled(sections, Role::Region, "Features").into_any_element()
    }

    // ---- a feature's pictures -----------------------------------------------------------------

    /// The band across the top of the opened feature: Back, then the title and its area, on a tint of the area's hue.
    fn render_band(&self, slug: &str, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Some(row) = self.row(slug) else { return div().into_any_element() };
        let hue = hue_among(self.areas.iter().map(|area| area.slug.as_str()), &row.area);
        let area = self.areas.iter().find(|area| area.slug == row.area).map(|area| area.title.clone()).unwrap_or_default();
        let back = labelled(ghost_with(look, "gallery-back", Some("arrow-left"), "Back", false).tooltip(tip("Back to the gallery (Esc)")).on_click(cx.listener(|this, _, window, cx| this.back(window, cx))), Role::Button, "Back");
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .rounded_t(card_inner_radius(look))
            .bg(tint(look, look.color(token::BG_RAISED), hue, 0.18))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(hue).opacity(0.35))
            .child(back)
            .child(labelled(
                div()
                    .id("gallery-feature-title")
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XL))
                    .line_height(look.length(token::LEADING_XL))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(row.title.clone())),
                Role::Heading,
                row.title.clone(),
            ))
            .child(labelled(pill_area(look, hue, area.clone()).id("gallery-feature-area"), Role::Label, format!("Area: {area}")))
            .into_any_element()
    }

    /// Who took a picture and when, in small type.
    fn facts(one: &FeaturePicture, look: &Look) -> Div {
        let who = match (one.by, &one.job) {
            (PictureBy::Librarian, _) => "Librarian".to_string(),
            (PictureBy::Job, Some(job)) => job.clone(),
            (PictureBy::Job, None) => "A Job".to_string(),
        };
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(div().min_w_0().truncate().child(SharedString::from(who)))
            .child(div().flex_none().whitespace_nowrap().child(SharedString::from(super::librarian::ago(one.taken_at.as_str(), Utc::now()))))
    }

    /// One picture as a card: the picture whole, where it was taken, who took it and when, and its buttons; or under it the box
    /// that asks why, where Not this feature was pressed.
    fn render_card(&self, place: usize, kind: Kind, one: &FeaturePicture, focused: bool, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let Some(opened) = self.opened.as_ref() else { return div().into_any_element() };
        let wide = card_width(look);
        let tall = screen_height(wide - look.length(token::SPACE_2) * 2.);
        let here = opened.at == place;
        let ring = ring_colour(look);
        let cover = kind == Kind::Standing && place == 0;
        let said = source(one);
        let mut acts = div().flex_none().flex().items_center().gap(look.length(token::SPACE_2));
        for (b, words) in kind.buttons().iter().enumerate() {
            let on = focused && here && opened.button == Some(b);
            let id = one.id.clone();
            let press = button(look, SharedString::from(format!("gallery-{}-{}", one.id, b)), words, *words == YES, false, false, None)
                .when(on, |button| button.border_color(ring).shadow(selected_shadow(look, Some(ring))))
                .on_click(cx.listener(move |this, _, window, cx| match kind.buttons().get(b).copied() {
                    Some(YES) => this.keep(&id, cx),
                    Some(NOT_THIS) => this.ask_why(id.clone(), window, cx),
                    _ => {}
                }));
            acts = acts.child(labelled(press, Role::Button, *words));
        }
        let why = opened.why.as_ref().filter(|why| why.picture == one.id);
        let foot = match why {
            Some(why) => {
                let typing = why.input.read(cx).focus_handle(cx).is_focused(window);
                let field = labelled(
                    div()
                        .id("gallery-why")
                        .flex_1()
                        .min_w_0()
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_2))
                        .flex()
                        .items_center()
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if typing { ring } else { look.color(token::BORDER_DEFAULT) })
                        .bg(look.color(token::BG_BASE))
                        .key_context(crate::keymap::TEXT_BOX)
                        .text_size(look.length(token::TEXT_XS))
                        .child(Input::new(&why.input).appearance(false)),
                    Role::TextInput,
                    "Why",
                );
                let id = one.id.clone();
                let remove = labelled(button(look, "gallery-why-remove", "Remove", true, false, false, None).tooltip(tip("Take it off this feature (Enter)")).on_click(cx.listener(move |this, _, window, cx| this.remove(&id, window, cx))), Role::Button, "Remove");
                let cancel = labelled(button(look, "gallery-why-cancel", "Cancel", false, false, false, None).tooltip(tip("Keep it (Esc)")).on_click(cx.listener(|this, _, window, cx| this.shut_why(window, cx))), Role::Button, "Cancel");
                div().h(look.length(token::H_CONTROL)).flex().items_center().gap(look.length(token::SPACE_2)).child(field).child(remove).child(cancel).into_any_element()
            }
            None => div().h(look.length(token::H_CONTROL)).flex().items_center().gap(look.length(token::SPACE_2)).child(Self::facts(one, look)).child(acts).into_any_element(),
        };
        // As tall as the Cover chip on every card, so the words under the pictures of a row line up.
        let head = div()
            .h(look.length(token::H_BADGE))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .children(cover.then(|| labelled(pill(look, token::ACCENT, "Cover").id(SharedString::from(format!("gallery-cover-{}", one.id))).tooltip(tip("The newest picture: the gallery shows the feature by it")), Role::Label, "Cover")))
            .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())));
        let card = div()
            .id(SharedString::from(format!("gallery-card-{}", one.id)))
            .flex_none()
            .w(wide)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_CARD))
            .when(focused && here && opened.button.is_none() && why.is_none(), |card| card.shadow(selected_shadow(look, Some(ring))))
            .child(well(look, self.images.get(&one.id).cloned(), tall, ObjectFit::Contain))
            .child(div().px(look.length(token::SPACE_1)).flex().flex_col().gap(look.length(token::SPACE_2)).child(head).child(foot));
        let name = if kind == Kind::Suggested { format!("Suggested: {said}") } else { said };
        labelled(card, Role::Article, name).into_any_element()
    }

    /// One part of the page: its heading, then its cards side by side, wrapping. Also this? wears the awaiting review hue, since
    /// every card under it waits on his Yes or Not this feature.
    fn render_part(&self, title: &'static str, kind: Kind, from: usize, list: &[FeaturePicture], focused: bool, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let asks = kind == Kind::Suggested;
        let heading = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("gallery-part-{title}")))
                    .when(asks, |head| head.px(look.length(token::SPACE_2)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::STATUS_AWAITING_REVIEW_BG)).text_color(look.color(token::STATUS_AWAITING_REVIEW)))
                    .when(!asks, |head| head.text_color(look.color(token::FG_DEFAULT)))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(title),
                Role::Heading,
                title,
            ));
        let mut cards = div().flex().flex_wrap().gap(look.length(token::SPACE_4));
        if list.is_empty() {
            let title = self.opened.as_ref().and_then(|opened| self.row(&opened.slug)).map(|row| row.title.clone()).unwrap_or_default();
            let wide = card_width(look);
            cards = cards.child(div().w(wide).p(look.length(token::SPACE_2)).child(no_picture(look, &title, screen_height(wide - look.length(token::SPACE_2) * 2.))));
        }
        for (i, one) in list.iter().enumerate() {
            cards = cards.child(self.render_card(from + i, kind, one, focused, look, window, cx));
        }
        labelled(div().id(SharedString::from(format!("gallery-{title}"))).flex_none().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading).child(cards), Role::Region, title).into_any_element()
    }

    fn render_opened(&self, slug: &str, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let focused = self.focus.is_focused(window) || self.opened.as_ref().is_some_and(|opened| opened.why.is_some());
        let standing = self.pictures.get(slug).cloned().unwrap_or_default();
        let suggested = self.suggested.get(slug).cloned().unwrap_or_default();
        let mut body = div().id("gallery-feature-body").track_scroll(&self.page_scroll).overflow_y_scroll().flex_1().min_h_0().flex().flex_col().gap(look.length(token::GAP_SECTION)).p(look.length(token::SPACE_6));
        body = body.child(self.render_part("Pictures", Kind::Standing, 0, &standing, focused, look, window, cx));
        if !suggested.is_empty() {
            body = body.child(self.render_part("Also this?", Kind::Suggested, standing.len(), &suggested, focused, look, window, cx));
        }
        let panel = div()
            .id("gallery-feature")
            .size_full()
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_CARD))
            .child(self.render_band(slug, look, cx))
            .child(body);
        div().size_full().p(look.length(token::SPACE_6)).child(labelled(panel, Role::Region, "Feature pictures")).into_any_element()
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let body = match self.opened.as_ref().map(|opened| opened.slug.clone()) {
            Some(slug) => self.render_opened(&slug, &look, window, cx),
            None => self.render_gallery(&look, window, cx),
        };
        div()
            .id("feature-gallery")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().id("feature-gallery-keys").track_focus(&self.focus).flex_1().min_h_0().child(body))
    }
}
