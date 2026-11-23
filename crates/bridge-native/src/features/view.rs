//! The Features tab drawn from what `model.rs` made of Fleet's read: in a panel on the left the field that finds a feature, and each
//! area as a head with its own hue and its features indented under it, and in a panel beside it the slot the feature's page fills,
//! as Reviews and Jobs lay out a list and its content.
//! The look is the owner's Dossier take. Every value is a token.

use gpui::{div, list, prelude::*, px, Div, FontWeight, IntoElement, Pixels, Render, Role, SharedString, Window};
use gpui_component::input::Input;

use super::dossier::{hue_square, selected_shadow};
use super::model::{Line, Shown};
use super::{FeaturesScreen, Load};
use crate::reviews::panel;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::settings::view::{bone, ghost_with, ring_colour, tip};
use crate::titlebar::parts::kbd;
use crate::titlebar::parts::sans;

/// The width of the list column (`.rail`): six steps of the largest space.
fn rail_width(look: &Look) -> Pixels {
    look.length(token::SPACE_12) * 6.
}

/// What the page's panel leaves of the window's width: the screen's padding either side, the page panel's own edges, and where the
/// list is beside it that panel and the gap between.
impl FeaturesScreen {
    pub(super) fn inset(&self, cx: &gpui::App) -> Pixels {
        let look = cx.look();
        let frame = look.length(token::SPACE_3) * 2. + look.length(token::BORDER_WIDTH) * 2.;
        if self.stacked {
            frame
        } else {
            frame + rail_width(look) + look.length(token::GAP_PANEL)
        }
    }
}

/// The mark a Job is on a feature: the running dot with its soft ring, and every Job in the tooltip. Absent where nobody is on it.
fn on_mark(look: &Look, shown: &Shown) -> Option<gpui::AnyElement> {
    shown.on.first()?;
    let cell = div().id(SharedString::from(format!("feature-on-{}", shown.row.slug))).flex_none().p(look.length(token::SPACE_1) - px(1.)).tooltip(tip(shown.on_said())).child(super::dossier::dot(look, token::STATUS_RUNNING, true));
    Some(labelled(cell, Role::Image, format!("On it now: {}", shown.on.join(", "))).into_any_element())
}

/// The word a stale section leaves on its feature (`.stale`), in the awaiting review hue.
fn stale_mark(look: &Look, shown: &Shown) -> Option<gpui::AnyElement> {
    let stale = shown.stale.clone()?;
    let cell = div()
        .id(SharedString::from(format!("feature-stale-{}", shown.row.slug)))
        .flex_none()
        .text_size(look.length(token::TEXT_2XS))
        .text_color(look.color(token::STATUS_AWAITING_REVIEW))
        .tooltip(tip("A written section is older than the last merge that changed its files"))
        .child("stale");
    Some(labelled(cell, Role::Image, stale).into_any_element())
}

impl FeaturesScreen {
    fn render_bones(&self, look: &Look) -> Div {
        let mut bones = div().flex().flex_col().px(look.length(token::SPACE_2));
        for area in 0..3 {
            bones = bones.child(div().flex().items_center().gap(look.length(token::SPACE_2)).px(look.length(token::SPACE_2)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_1)).child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3))).child(bone(look, px(84. + 20. * area as f32), look.length(token::LEADING_2XS))));
            for row in 0..3 {
                bones = bones.child(
                    div()
                        .id(SharedString::from(format!("features-bone-{area}-{row}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .pl(look.length(token::SPACE_6) + px(2.))
                        .pr(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1) + px(2.))
                        .child(div().flex_1().child(bone(look, px(120. + 36. * ((area + row) % 3) as f32), look.length(token::LEADING_XS) - px(4.)))),
                );
            }
        }
        bones
    }

    /// The hue of the area `slug`: its place among the areas the library keeps, and the Operator's for the pending group.
    pub(super) fn hue_of(&self, slug: &str) -> Token {
        match &self.library {
            Load::Read(library) => super::dossier::hue_among(library.areas.iter().map(|area| area.slug.as_str()), slug),
            _ => super::dossier::hue_among([], slug),
        }
    }

    fn render_line(&mut self, ix: usize, focused: bool, window: &Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let look = cx.look().clone();
        let look = &look;
        match self.listed.lines.get(ix).cloned() {
            Some(Line::Head { slug, title }) => {
                let beside = self.split_beside(&slug, &title, look, cx);
                let shown_title = if slug == super::curate::PENDING_AREA { super::page_model::sentence(&title) } else { title.clone() };
                div()
                    .id(SharedString::from(format!("features-area-{slug}")))
                    .w_full()
                    .px(look.length(token::SPACE_2))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_2))
                            .pt(look.length(token::SPACE_2) + px(2.))
                            .pb(look.length(token::SPACE_1))
                            .child(hue_square(look, self.hue_of(&slug), look.length(token::SPACE_2) + px(2.)))
                            .child(labelled(
                                div()
                                    .id(SharedString::from(format!("features-area-title-{slug}")))
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(look.length(token::TEXT_XS))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(SharedString::from(shown_title)),
                                Role::Heading,
                                title,
                            ))
                            .children(beside),
                    )
                    .into_any_element()
            }
            Some(Line::Feature(at)) => match self.listed.shown.get(at).cloned() {
                Some(shown) => self.render_feature(&shown, focused, look, window, cx),
                None => div().into_any_element(),
            },
            None => div().into_any_element(),
        }
    }

    /// One feature: its name, and at the right the dot that a Job is on it and the word that a section is stale. The one opened
    /// is a raised row with a hairline and a shadow, and the one the keys are on has the focus colour for its hairline. A pending
    /// feature has its three buttons under its name, and an editor under them where one is open.
    fn render_feature(&self, shown: &Shown, focused: bool, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let slug = shown.row.slug.clone();
        let opened = self.opened.as_deref() == Some(slug.as_str());
        let under_keys = self.on_project.is_none() && self.cursor.as_deref() == Some(slug.as_str());
        let press = slug.clone();
        let pending = super::curate::is_pending(&shown.row);
        let main = if pending {
            self.pending_line(&shown.row, look, cx)
        } else {
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(shown.row.title.clone())))
                .children(on_mark(look, shown))
                .children(stale_mark(look, shown))
                .into_any_element()
        };
        let row = div()
            .id(SharedString::from(format!("feature-{slug}")))
            .w_full()
            .flex()
            .flex_col()
            .pl(look.length(token::SPACE_6) + px(2.))
            .pr(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1) + px(2.))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .map(|row| {
                if opened {
                    row.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)).shadow(selected_shadow(look, (under_keys && focused).then(|| ring_colour(look))))
                } else if under_keys && focused {
                    row.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).shadow(selected_shadow(look, Some(ring_colour(look))))
                } else {
                    row.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)))
                }
            })
            .aria_selected(opened)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_at(press.clone(), window, cx);
                this.engage_page(cx);
                if this.stacked {
                    this.show_page(window, cx);
                } else {
                    this.focus_list(window, cx);
                }
            }))
            .child(main)
            .children(if pending { self.form_under(&slug, look, window, cx) } else { None });
        div().w_full().px(look.length(token::SPACE_2)).child(labelled(row, Role::ListBoxOption, shown.row.title.clone())).into_any_element()
    }

    /// The field that finds a feature, with the key that reaches it (`.find`).
    fn render_find(&self, look: &Look) -> impl IntoElement {
        let filter = labelled(
            div().id("features-filter").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_XS)).child(Input::new(&self.filter).appearance(false)),
            Role::SearchInput,
            "Filter features",
        );
        div()
            .id("features-find")
            .flex_none()
            .m(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_2) + px(2.))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_BASE))
            .child(filter)
            .child(kbd("/", look))
    }

    fn render_list(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
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
                    div().id("features-list").size_full().pb(look.length(token::SPACE_3)).child(list(self.list.clone(), cx.processor(move |this, ix: usize, window, cx| this.render_line(ix, focused, window, cx))).size_full()),
                    Role::ListBox,
                    "Features",
                )
                .into_any_element()
            }
        };
        panel(look)
            .id("features-column")
            .track_focus(&self.list_focus)
            .flex_none()
            .map(|column| if self.stacked { column.w_full() } else { column.w(rail_width(look)) })
            .h_full()
            .flex()
            .flex_col()
            .children(self.render_chips(look, cx))
            .child(self.render_find(look))
            .children(self.render_librarian(look, window, cx))
            .child(self.render_project_rows(look, self.list_focus.is_focused(window), cx))
            .child(div().flex_1().min_h_0().child(body))
    }

    /// The strip over a page drawn alone: Back, as the overlay's page has it.
    fn render_back(&self, look: &Look, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let back = labelled(ghost_with(look, "features-back", Some("arrow-left"), "Back", false).on_click(cx.listener(|this, _, window, cx| this.back(window, cx))), Role::Button, "Back");
        div()
            .id("features-back-strip")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(back)
    }

    /// The right of the list. A feature's page is drawn here once one is open.
    fn render_page_slot(&self, look: &Look, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        labelled(panel(look).id("feature-page").flex_1().min_w_0().h_full().flex().flex_col().children(self.stacked.then(|| self.render_back(look, cx))).child(div().flex_1().min_h_0().children(self.project_page.clone().map(gpui::IntoElement::into_any_element).or_else(|| self.page.clone().map(gpui::IntoElement::into_any_element)))), Role::Region, "Feature page")
    }
}

impl Render for FeaturesScreen {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        div()
            .id("features")
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .relative()
            .size_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            .p(look.length(token::SPACE_3))
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .map(|screen| match (self.stacked, self.showing) {
                // Narrow: one panel at a time, each the full width. The page has a Back row over it, as the overlay's page has.
                (true, true) => screen.child(self.render_page_slot(&look, cx)),
                (true, false) => screen.child(self.render_list(&look, window, cx)),
                (false, _) => screen.child(self.render_list(&look, window, cx)).child(self.render_page_slot(&look, cx)),
            })
            .children(self.render_sheet(&look, window, cx))
            .children(self.curate.menu.as_ref().map(|open| open.menu.clone()))
    }
}
