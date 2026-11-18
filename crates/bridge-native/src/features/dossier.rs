//! The pieces the Features screen's look is made of, shared by its list and its page: an area's hue, a tint of a hue over a
//! ground, the small chip, the dot a state is, and the buttons. Every value is a token; the sketch's own CSS is named beside each.

use gpui::{div, point, prelude::*, px, BoxShadow, Div, FocusHandle, FontWeight, Hsla, SharedString, Stateful};

use crate::look::{token, Look, Token};
use crate::settings::view::ring_colour;

/// The hues an area takes, by its place in the library. The sketch palette, which is the status hues borrowed and so clears
/// contrast on its own tint (`--sketch-*`); purple is the owner's and the Operator's, so it is left out.
const AREAS: [Token; 6] = [token::SKETCH_BLUE, token::SKETCH_TEAL, token::SKETCH_GREEN, token::SKETCH_ORANGE, token::SKETCH_AMBER, token::SKETCH_GREY];

/// The hue of the area at `place` in the library's order.
pub fn area_hue(place: usize) -> Token {
    AREAS[place % AREAS.len()]
}

/// A hue for an area where only its name is known, the same each time.
pub fn hue_of_name(name: &str) -> Token {
    area_hue(name.bytes().fold(0usize, |sum, byte| sum.wrapping_mul(31).wrapping_add(byte as usize)))
}

/// `color-mix(in srgb, hue N%, ground)`: the hue at `share` over the ground.
pub fn tint(look: &Look, ground: Hsla, hue: Token, share: f32) -> Hsla {
    ground.blend(look.color(hue).opacity(share))
}

/// The square an area has before its name (`.hue`).
pub fn hue_square(look: &Look, hue: Token, side: gpui::Pixels) -> Div {
    div().flex_none().size(side).rounded(look.length(token::RADIUS_SM)).bg(look.color(hue))
}

/// The dot a state is (`.dot`): seven wide, and where something runs now, a soft ring round it (`.d-run`).
pub fn dot(look: &Look, hue: Token, running: bool) -> Div {
    let side = look.length(token::SPACE_2) - px(1.);
    div().flex_none().size(side).rounded(side / 2.).bg(look.color(hue)).when(running, |dot| {
        dot.shadow(vec![BoxShadow { color: look.color(hue).opacity(0.22), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::SPACE_1) - px(1.), inset: false }])
    })
}

/// A pill filled with a hue at 16% (`.chip`), its words in the hue.
pub fn pill(look: &Look, hue: Token, words: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::H_BADGE) / 2.)
        .bg(look.color(hue).opacity(0.16))
        .text_color(look.color(hue))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .whitespace_nowrap()
        .child(words.into())
}

/// The quiet pill (`.c-area`): the hover ground, the muted words.
pub fn pill_quiet(look: &Look, words: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) + px(1.))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::H_BADGE) / 2.)
        .bg(look.color(token::BG_HOVER))
        .text_color(look.color(token::FG_MUTED))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .whitespace_nowrap()
        .child(words.into())
}

/// The quiet pill of an area (`.c-area` with its `.hue`): the area's square, then its name.
pub fn pill_area(look: &Look, hue: Token, words: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) + px(1.))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::H_BADGE) / 2.)
        .bg(look.color(token::BG_HOVER))
        .text_color(look.color(token::FG_MUTED))
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .whitespace_nowrap()
        .child(hue_square(look, hue, look.length(token::SPACE_2)))
        .child(words.into())
}

/// A button (`.btn`): bordered on the raised ground, or `primary` filled with the accent. `big` is the page's own size, the
/// small one a row's. It takes the keys through `focus` where it has a handle.
pub fn button(look: &Look, id: impl Into<gpui::ElementId>, words: &str, primary: bool, big: bool, off: bool, focus: Option<&FocusHandle>) -> Stateful<Div> {
    let (ground, hover, ink, edge) = if primary {
        (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT))
    } else {
        (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_DEFAULT))
    };
    let ring = ring_colour(look);
    let button = div()
        .id(id.into())
        .when_some(focus, |button, focus| button.track_focus(focus).tab_stop(!off))
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(if big { token::H_CONTROL } else { token::H_CONTROL_SM }))
        .px(look.length(if big { token::SPACE_3 } else { token::SPACE_2 }))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_size(look.length(if big { token::TEXT_XS } else { token::TEXT_2XS }))
        .line_height(look.length(if big { token::LEADING_XS } else { token::LEADING_2XS }))
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

/// The shadow of the row that is selected in the list (`0 1px 2px rgb(0 0 0 / .45), 0 0 0 1px border-default`), its ring in
/// `ring` where the keys are on it.
pub fn selected_shadow(look: &Look, ring: Option<Hsla>) -> Vec<BoxShadow> {
    vec![
        BoxShadow { color: gpui::black().opacity(0.45), offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
        BoxShadow { color: ring.unwrap_or_else(|| look.color(token::BORDER_DEFAULT)), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: false },
    ]
}
