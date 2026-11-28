//! The shared parts a scene is drawn from: row, card, chip, button, tabs, list, pane and text.
//!
//! Native Bridge had no shared parts: each screen carried its own element helpers. These take
//! their look from the most used variant of each, named on the part, and read every value through
//! [`crate::look`]. **No screen is moved onto them yet.** They stand beside the helpers until a
//! Job moves a screen at a time, and the prototype scene reader (`crate::prototype`) draws only
//! from here.
//!
//! Every part takes an element id and returns a `Stateful<Div>`, so the caller can press it, focus
//! it and give it a role and a name (`crate::labels::labelled`) without the part knowing.

#![allow(dead_code)]

pub mod button;
pub mod card;
pub mod chip;
pub mod list;
pub mod pane;
pub mod ring;
pub mod row;
pub mod tabs;
pub mod text;

use gpui::{Hsla, SharedString, Styled as _};

use crate::look::{token, Look};

/// What a colour says about its thing. The scene names one by word (`tone="success"`) or by any
/// colour token (`tone="status-running"`); a screen takes a variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Danger,
    Running,
    Waiting,
    Warning,
}

impl Tone {
    pub fn named(word: &str) -> Option<Tone> {
        Some(match word {
            "neutral" => Tone::Neutral,
            "accent" => Tone::Accent,
            "success" => Tone::Success,
            "danger" | "failed" => Tone::Danger,
            "running" => Tone::Running,
            "waiting" => Tone::Waiting,
            "warning" => Tone::Warning,
            _ => return None,
        })
    }

    pub fn ink(self, look: &Look) -> Hsla {
        look.color(match self {
            Tone::Neutral => token::FG_MUTED,
            Tone::Accent => token::ACCENT,
            Tone::Success => token::STATUS_COMPLETED_SUCCESS,
            Tone::Danger => token::STATUS_COMPLETED_FAILED,
            Tone::Running => token::STATUS_RUNNING,
            Tone::Waiting => token::STEP_WAITING,
            Tone::Warning => token::NOTICE_CAUTION,
        })
    }
}

/// The first family of the mono stack, as every mono line takes it.
pub(crate) fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// A registry glyph at `size`, in `ink`.
pub(crate) fn glyph(name: &str, size: gpui::Pixels, ink: Hsla) -> gpui::Svg {
    gpui::svg().path(crate::assets::icon_path(name)).flex_none().size(size).text_color(ink)
}
