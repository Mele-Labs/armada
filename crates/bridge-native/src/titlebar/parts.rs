//! The small pieces the title bar's popovers share: a button, a glyph, a tooltip, a spinner, a
//! card. Every value is a token. The owner's 9 Nov tweak shows here once: controls a step taller
//! than Electron's and corners from `--radius-lg`, in the same tokens Foundation's Jobs list uses.

use gpui::{
    div, prelude::*, svg, Animation, AnimationExt as _, AnyView, App, ElementId, FontWeight, Hsla, Pixels, SharedString,
    Stateful, Div, Transformation, Window,
};

use crate::assets::icon_path;
use crate::look::{token, ActiveLook, Look};

pub fn sans(look: &Look) -> SharedString {
    look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default()
}

pub fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// A registry glyph at `size`, in `ink`.
pub fn glyph(name: &str, size: Pixels, ink: Hsla) -> gpui::Svg {
    svg().path(icon_path(name)).size(size).flex_none().text_color(ink)
}

/// The words a tooltip says.
pub fn tip(said: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let said: SharedString = said.into();
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
}

/// An open ring turning: a restart, a revert or a swap under way.
pub fn spinner(id: impl Into<ElementId>, size: Pixels, ink: Hsla, cx: &App) -> impl IntoElement {
    glyph("loader-circle", size, ink).with_animation(
        id,
        Animation::new(cx.look().duration(token::DURATION_PULSE)).repeat(),
        |mark, at| mark.with_transformation(Transformation::rotate(gpui::percentage(at))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Primary,
    Secondary,
    /// No ground: `Restart Fleet` on the notice's own ground.
    Ghost,
}

/// A button as Electron's `Button` size sm, a step taller. `pending` swaps the glyph for a turning
/// ring and takes the press; `disabled` dims it and does too.
pub struct Btn {
    pub id: ElementId,
    pub label: SharedString,
    pub icon: Option<&'static str>,
    pub weight: Weight,
    pub disabled: bool,
    pub pending: bool,
    /// Fills its row and centres its content, as the Fleet panel's buttons do.
    pub full: bool,
}

impl Btn {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, weight: Weight) -> Btn {
        Btn { id: id.into(), label: label.into(), icon: None, weight, disabled: false, pending: false, full: false }
    }

    pub fn icon(mut self, icon: &'static str) -> Btn {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Btn {
        self.disabled = disabled;
        self
    }

    pub fn pending(mut self, pending: bool) -> Btn {
        self.pending = pending;
        self
    }

    pub fn full(mut self) -> Btn {
        self.full = true;
        self
    }

    /// The element, with its press unset: the caller adds `on_click` and `labelled`. A button that
    /// is off or pending carries no pointer and its caller adds no press.
    pub fn draw(self, look: &Look, cx: &App) -> Stateful<Div> {
        let (ground, hover, ink, edge) = match self.weight {
            Weight::Primary => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT)),
            Weight::Secondary => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_DEFAULT)),
            Weight::Ghost => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), gpui::transparent_black()),
        };
        let size = look.length(token::SPACE_4);
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let live = !self.disabled && !self.pending;
        let mark: Option<gpui::AnyElement> = if self.pending {
            Some(spinner(SharedString::from(format!("{:?}-spin", self.id)), size, ink, cx).into_any_element())
        } else {
            self.icon.map(|name| glyph(name, size, ink).into_any_element())
        };
        let body = div()
            .id(self.id)
            .flex_none()
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .flex()
            .items_center()
            .when(self.full, |button| button.w_full().justify_center())
            .gap(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .focusable()
            .tab_stop(live)
            .focus_visible(move |style| style.border_color(ring))
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
            .children(mark)
            .child(self.label);
        if live {
            body.cursor_pointer().hover(move |style| style.bg(hover))
        } else {
            body.opacity(0.5)
        }
    }
}

/// An icon-only button at the bar's control height, ghost until hovered.
pub fn icon_button(id: impl Into<ElementId>, name: &str, ink: Hsla, look: &Look) -> Stateful<Div> {
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let hover = look.color(token::BG_HOVER);
    div()
        .id(id)
        .flex_none()
        .size(look.length(token::H_CONTROL_LG))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| style.border_color(ring))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
        .hover(move |style| style.bg(hover))
        .child(glyph(name, look.length(token::SPACE_4), ink))
}

/// The card every popover of the bar is: the overlay ground, a hairline, the deepest shadow, one radius.
pub fn card(look: &Look) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_OVERLAY))
        .shadow(look.shadows(token::SHADOW_OVERLAY))
        .text_color(look.color(token::FG_DEFAULT))
        .font_family(sans(look))
}

/// A line of small type.
pub fn small(look: &Look) -> Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS))
}

/// A keycap: a key and nothing else.
pub fn kbd(keys: impl Into<SharedString>, look: &Look) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_KBD))
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .text_color(look.color(token::FG_MUTED))
        .child(keys.into())
}

/// The row is a drag region, so a press on a control must not reach it: a control stops the press.
/// It does not take the keys on a press either: a control that goes away after it is pressed, as the
/// rings and Keep do, would leave them on nothing. The Tab key still reaches it.
pub fn no_drag(element: Stateful<Div>) -> Stateful<Div> {
    element.on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
        cx.stop_propagation();
        window.prevent_default();
    })
}
