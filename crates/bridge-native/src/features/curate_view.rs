//! What curating draws: a pending feature's row with its three buttons, the editor under it, an area head that is past
//! eight with its Split button, and the sheet that splits it. Every value is a token; `curate.rs` holds what they do.

use gpui::{div, prelude::*, AnyElement, Context, Div, Entity, FocusHandle, Focusable as _, FontWeight, Role, SharedString, Stateful, Window};
use gpui_component::input::{Input, InputState};
use ipc::FeatureRow;

use super::curate::{over, Kind};
use super::{FeaturesScreen, Load};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::{ring_colour, tip};
use crate::titlebar::parts::{glyph, kbd, sans};

/// The names the three buttons of a pending row go by.
pub fn confirm_name(title: &str) -> String {
    format!("Confirm {title}")
}

pub fn merge_name(title: &str) -> String {
    format!("Merge {title} into…")
}

pub fn rename_name(title: &str) -> String {
    format!("Rename {title}")
}

/// A small bordered chip that presses: `primary` is filled with the accent. It takes the keys through `focus`.
fn chip_button(look: &Look, id: impl Into<gpui::ElementId>, words: &str, primary: bool, off: bool, focus: &FocusHandle) -> Stateful<Div> {
    let (ground, hover, ink, edge) = if primary {
        (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT))
    } else {
        (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_DEFAULT))
    };
    let ring = ring_colour(look);
    let button = div()
        .id(id.into())
        .track_focus(focus)
        .tab_stop(!off)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .whitespace_nowrap()
        .focus_visible(move |style| style.border_color(ring))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
        .child(SharedString::from(words.to_string()));
    if off {
        button.opacity(0.5)
    } else {
        button.cursor_pointer().hover(move |style| style.bg(hover))
    }
}

/// A text box's frame: sunken, one hairline, the accent while it has the keys.
fn field(look: &Look, focused: bool) -> Div {
    div()
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if focused { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_SM))
}

fn caption(look: &Look, words: &'static str) -> Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child(words)
}

/// A caption over a text box, the box named for VoiceOver by its caption.
fn labelled_box(look: &Look, window: &Window, cx: &gpui::App, input: &Entity<InputState>, name: &'static str, id: &'static str) -> Div {
    let focused = input.read(cx).focus_handle(cx).is_focused(window);
    let text = div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(Input::new(input).appearance(false));
    div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption(look, name)).child(labelled(field(look, focused).id(id).child(text), Role::TextInput, name))
}

/// A tick box: empty, or the accent with a tick.
fn tick(look: &Look, on: bool) -> Div {
    let accent = look.color(token::ACCENT);
    div()
        .flex_none()
        .size(look.length(token::SPACE_4))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if on { accent } else { look.color(token::BORDER_STRONG) })
        .when(on, |tick| tick.bg(accent).child(glyph("check", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT))))
}

impl FeaturesScreen {
    /// A pending feature's line: its title over what the library says of it, and at the right Confirm, Merge into… and Rename.
    pub(super) fn pending_line(&self, row: &FeatureRow, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let Some(stops) = self.curate.stops.get(&row.slug).cloned() else { return div().into_any_element() };
        let button = |at: usize, id: &str, words: &str, name: String, said: &str, primary: bool, open: fn(&mut Self, String, &mut Window, &mut Context<Self>)| {
            let slug = row.slug.clone();
            let press = chip_button(look, SharedString::from(format!("{id}-{}", row.slug)), words, primary, false, &stops[at])
                .tooltip(tip(said.to_string()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    open(this, slug.clone(), window, cx);
                }));
            labelled(press, Role::Button, name)
        };
        let text = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(gpui::px(2.))
            .child(div().min_w_0().truncate().child(SharedString::from(row.title.clone())))
            .when(!row.summary.is_empty(), |text| {
                text.child(div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(row.summary.clone())))
            });
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(text)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .child(button(0, "feature-confirm", "Confirm", confirm_name(&row.title), "Keep it in the catalog, in an area, with a title and a line", true, Self::open_confirm))
                    .child(button(1, "feature-merge", "Merge into…", merge_name(&row.title), "Fold it into a feature the catalog keeps: its files and asks move there", false, Self::open_merge))
                    .child(button(2, "feature-rename", "Rename", rename_name(&row.title), "Change its slug and title: the old slug stays as an alias", false, Self::open_rename)),
            )
            .into_any_element()
    }

    /// The editor under the pending row of `slug`, where one is open.
    pub(super) fn form_under(&self, slug: &str, look: &Look, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let form = self.curate.form.as_ref().filter(|form| form.slug == slug)?;
        let (confirm, title) = (form.kind == Kind::Confirm, self.listed.find(slug).map(|at| self.listed.shown[at].row.title.clone()).unwrap_or_default());
        let ready = self.form_ready(cx);
        let (one, two) = if confirm { ("Title", "Line") } else { ("Slug", "Title") };
        let area = match &self.library {
            Load::Read(library) => library.areas.iter().find(|area| area.slug == form.area).map(|area| area.title.clone()).unwrap_or_default(),
            _ => String::new(),
        };
        let area_button = {
            let press = div()
                .id("feature-form-area")
                .track_focus(&form.area_focus)
                .tab_stop(true)
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .cursor_pointer()
                .focus_visible({
                    let ring = ring_colour(look);
                    move |style| style.border_color(ring)
                })
                .tooltip(tip("The area it is kept in"))
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.open_area_menu(window, cx);
                }))
                .child(div().min_w_0().truncate().child(SharedString::from(area)))
                .child(glyph("chevron-down", look.length(token::SPACE_4), look.color(token::FG_MUTED)));
            div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption(look, "Area")).child(labelled(press, Role::Button, "Area"))
        };
        let cancel = labelled(
            chip_button(look, "feature-form-cancel", "Cancel", false, false, &form.cancel).tooltip(tip("Close it and change nothing")).on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.close_form(window, cx);
            })),
            Role::Button,
            "Cancel",
        );
        let (go_words, go_said) = if confirm { ("Confirm", "Keep it in the catalog") } else { ("Save", "Change its slug and title") };
        let go = labelled(
            chip_button(look, "feature-form-go", go_words, true, !ready, &form.go).tooltip(tip(if ready { go_said } else { "Fill every box" })).on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.submit_form(window, cx);
            })),
            Role::Button,
            go_words,
        );
        let name: SharedString = format!("{} {title}", if confirm { "Confirm" } else { "Rename" }).into();
        let body = div()
            .id("feature-form")
            .track_focus(&form.frame)
            .w_full()
            .mt(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_SUNKEN))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(confirm, |body| body.child(area_button))
            .child(labelled_box(look, window, cx, &form.one, one, "feature-form-one"))
            .child(labelled_box(look, window, cx, &form.two, two, "feature-form-two"))
            .child(div().flex().items_center().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(go));
        Some(labelled(body, Role::Form, name).into_any_element())
    }

    /// What an area head wears past eight features: the count in the warning colour and Split….
    pub(super) fn split_beside(&self, area: &str, title: &str, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Load::Read(library) = &self.library else { return None };
        let count = over(library, area)?;
        let stop = self.curate.splits.get(area)?;
        let slug = area.to_string();
        let words = format!("{count} features");
        let warn = look.color(token::STATUS_AWAITING_REVIEW);
        let said = div()
            .id(SharedString::from(format!("features-over-{area}")))
            .flex_none()
            .text_size(look.length(token::TEXT_2XS))
            .text_color(warn)
            .tooltip(tip("An area reads best at eight features or fewer"))
            .child(SharedString::from(words.clone()));
        let split = chip_button(look, SharedString::from(format!("features-split-{area}")), "Split…", false, false, stop)
            .tooltip(tip("Move some of this area's features into a new area"))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.open_split(slug.clone(), window, cx);
            }));
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(said, Role::Image, words))
                .child(labelled(split, Role::Button, format!("Split {title}")))
                .into_any_element(),
        )
    }

    /// The sheet over the screen that splits an area.
    pub(super) fn render_sheet(&self, look: &Look, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let sheet = self.curate.sheet.as_ref()?;
        let ready = self.split_ready(cx);
        let why = self.split_why(cx);
        let view = window.viewport_size();
        let width = look.length(token::W_DIALOG).min(view.width - look.length(token::SPACE_6) * 2.);
        let top = (view.height * 0.06).max(look.length(token::H_TITLE_BAR));
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(SharedString::from(format!("Split {}", sheet.title))),
            )
            .child(kbd("Esc", look));
        let rows = sheet.features.iter().enumerate().map(|(at, (slug, title, on))| {
            let ring = ring_colour(look);
            let line = div()
                .id(SharedString::from(format!("features-split-row-{slug}")))
                .track_focus(&sheet.rows[at])
                .tab_stop(true)
                .w_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1) + gpui::px(2.))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_size(look.length(token::TEXT_SM))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focus_visible(move |style| style.border_color(ring))
                .aria_toggled(if *on { gpui::accesskit::Toggled::True } else { gpui::accesskit::Toggled::False })
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_split(at, cx)))
                .child(tick(look, *on))
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(title.clone())));
            labelled(line, Role::CheckBox, title.clone()).into_any_element()
        });
        let list = div().id("features-split-list").max_h(view.height * 0.4).overflow_y_scroll().flex().flex_col().children(rows);
        let cancel = labelled(
            chip_button(look, "features-split-cancel", "Cancel", false, false, &sheet.cancel).tooltip(tip("Close the sheet and change nothing")).on_click(cx.listener(|this, _, window, cx| this.close_sheet(window, cx))),
            Role::Button,
            "Cancel",
        );
        let go = labelled(
            chip_button(look, "features-split-go", "Split", true, !ready, &sheet.go).tooltip(tip(why)).on_click(cx.listener(|this, _, window, cx| this.submit_split(window, cx))),
            Role::Button,
            "Split",
        );
        let body = div()
            .p(look.length(token::SPACE_4))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(labelled_box(look, window, cx, &sheet.name, "New area", "features-split-name"))
            .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption(look, "Move to the new area")).child(labelled(list, Role::Group, "Features of the area")));
        let foot = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(cancel)
            .child(go);
        let frame = div()
            .id("features-split-frame")
            .track_focus(&sheet.frame)
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(width)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .child(head)
            .child(body)
            .child(foot);
        let scrim = div().id("features-split-scrim").absolute().inset_0().occlude().bg(look.color(token::SCRIM)).on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close_sheet(window, cx)));
        let column = div().absolute().top(top).left((view.width - width) / 2.).child(frame);
        let name: SharedString = format!("Split {}", sheet.title).into();
        Some(labelled(div().id("features-split").absolute().inset_0().child(scrim).child(column), Role::Dialog, name).into_any_element())
    }
}
