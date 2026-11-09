//! The pieces the walk bar is drawn from. Every value is a token; every control has a role and a
//! name.

use gpui::{div, prelude::*, svg, App, ClickEvent, FontWeight, Hsla, Role, SharedString, Window};

use crate::assets::icon_path;
use crate::labels::{labelled, Labelled};
use crate::look::{token, ActiveLook};

/// How a bar button reads: the one act that settles the walk, or the rest.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Primary,
    Quiet,
}

/// A button at the bar's control height. `keys` is the chord drawn after the label, as Electron's
/// bar draws ⌘↵ on Approve. Off, it draws muted and does nothing.
pub fn button(
    id: &'static str,
    label: impl Into<SharedString>,
    keys: Option<&'static str>,
    weight: Weight,
    on: bool,
    cx: &App,
    press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Labelled {
    let look = cx.look();
    let label = label.into();
    let (ground, hover, ink) = match weight {
        Weight::Primary => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT)),
        Weight::Quiet => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT)),
    };
    let edge: Hsla = match weight {
        Weight::Primary => ground,
        Weight::Quiet => look.color(token::BORDER_DEFAULT),
    };
    let mut body = div()
        .id(id)
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_size(look.length(token::TEXT_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .child(label.clone());
    if let Some(keys) = keys {
        body = body.child(
            div()
                .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                .text_size(look.length(token::TEXT_XS))
                .text_color(if weight == Weight::Primary { ink.opacity(0.75) } else { look.color(token::FG_MUTED) })
                .child(keys),
        );
    }
    let body = if on {
        body.cursor_pointer().hover(move |style| style.bg(hover)).on_click(press)
    } else {
        body.opacity(0.5)
    };
    labelled(body, Role::Button, label)
}

/// A square icon button at the small control height, as Electron's `Button iconOnly size="sm"`: ghost,
/// or `Primary` filled with the accent. `tip` is its tooltip and its name, with `keys` after it.
pub fn icon_button(
    id: &'static str,
    icon: &'static str,
    tip: &'static str,
    keys: Option<&'static str>,
    weight: Weight,
    on: bool,
    cx: &App,
    press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Labelled {
    let look = cx.look();
    let side = look.length(token::H_CONTROL_SM);
    let (ground, hover, ink) = match weight {
        Weight::Primary => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT)),
        Weight::Quiet => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_MUTED)),
    };
    let said: SharedString = match keys {
        Some(keys) => format!("{tip}  {keys}").into(),
        None => tip.into(),
    };
    let body = div()
        .id(id)
        .size(side)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_MD))
        .bg(ground)
        .child(svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(ink))
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx));
    let body = if on {
        body.cursor_pointer().hover(move |style| style.bg(hover)).on_click(press)
    } else {
        body.opacity(0.5)
    };
    labelled(body, Role::Button, tip)
}

/// A key chord drawn as Electron's `KbdChord`: mono, in a sunken box with a hairline.
pub fn chord(keys: &'static str, cx: &App) -> gpui::Div {
    let look = cx.look();
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_KBD))
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_SUNKEN))
        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
        .text_size(look.length(token::TEXT_2XS))
        .text_color(look.color(token::FG_MUTED))
        .child(keys)
}
