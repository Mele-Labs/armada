//! The pieces the walk bar is drawn from. Every value is a token; every control has a role and a
//! name.

use gpui::{div, prelude::*, App, ClickEvent, FontWeight, Hsla, Role, SharedString, Window};

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
