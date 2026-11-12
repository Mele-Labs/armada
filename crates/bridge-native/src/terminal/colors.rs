//! The screen's colours, as Electron gives xterm.js its theme (`XtermScreen.tsx:51-70`): the well,
//! the words, the cursor and the selection from the tokens, the hues a run takes for red, green,
//! blue and cyan, and xterm.js's own for the rest. A program's own colours (OSC 4) win.

use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use gpui::{Hsla, Rgba};

use crate::look::{token, Look, Token};

/// xterm.js's default sixteen (`DEFAULT_ANSI_COLORS`, Tango), for the slots the tokens leave.
const XTERM: [u32; 16] = [
    0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753, 0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8,
    0x34e2e2, 0xeeeeec,
];

/// The token each of the sixteen takes in Electron, where one does.
const TOKENS: [Option<Token>; 16] = [
    Some(token::BG_RAISED),
    Some(token::RUN_FAILED),
    Some(token::RUN_PASSED),
    None,
    Some(token::ACCENT),
    None,
    Some(token::RUN_RUNNING),
    None,
    Some(token::FG_SUBTLE),
    Some(token::RUN_FAILED),
    Some(token::RUN_PASSED),
    None,
    Some(token::ACCENT_HOVER),
    None,
    Some(token::RUN_RUNNING),
    None,
];

fn hex(value: u32) -> Hsla {
    Rgba { r: ((value >> 16) & 0xff) as f32 / 255.0, g: ((value >> 8) & 0xff) as f32 / 255.0, b: (value & 0xff) as f32 / 255.0, a: 1.0 }.into()
}

fn rgb(value: Rgb) -> Hsla {
    hex(((value.r as u32) << 16) | ((value.g as u32) << 8) | value.b as u32)
}

/// The 256-colour cube and grey ramp above the sixteen.
fn indexed(index: u8) -> Hsla {
    if index >= 232 {
        let level = 8 + 10 * (index as u32 - 232);
        return hex((level << 16) | (level << 8) | level);
    }
    let at = index as u32 - 16;
    let step = |n: u32| if n == 0 { 0 } else { 55 + 40 * n };
    hex((step(at / 36) << 16) | (step((at / 6) % 6) << 8) | step(at % 6))
}

/// The colours a screen is drawn in, read once a frame from the look in force.
pub struct Palette {
    pub background: Hsla,
    pub foreground: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
    sixteen: [Hsla; 16],
}

impl Palette {
    pub fn of(look: &Look) -> Palette {
        let sixteen = std::array::from_fn(|at| TOKENS[at].map_or_else(|| hex(XTERM[at]), |token| look.color(token)));
        Palette {
            background: look.color(token::BG_SUNKEN),
            foreground: look.color(token::FG_DEFAULT),
            cursor: look.color(token::FG_DEFAULT),
            selection: look.color(token::ACCENT_MUTED),
            sixteen,
        }
    }

    /// A cell's colour: the program's own where it set one, else the theme's.
    pub fn resolve(&self, color: Color, set: &Colors) -> Hsla {
        match color {
            Color::Spec(value) => rgb(value),
            Color::Indexed(index) => match set[index as usize] {
                Some(value) => rgb(value),
                None if index < 16 => self.sixteen[index as usize],
                None => indexed(index),
            },
            Color::Named(name) => match set[name] {
                Some(value) => rgb(value),
                None => self.named(name),
            },
        }
    }

    fn named(&self, name: NamedColor) -> Hsla {
        match name {
            NamedColor::Foreground | NamedColor::BrightForeground => self.foreground,
            NamedColor::DimForeground => self.foreground.opacity(0.66),
            NamedColor::Background => self.background,
            NamedColor::Cursor => self.cursor,
            other => {
                let at = other as usize;
                if at < 16 {
                    self.sixteen[at]
                } else {
                    // The dim eight follow the cursor in alacritty's order: DimBlack is the first.
                    let dim = at.saturating_sub(NamedColor::DimBlack as usize).min(7);
                    self.sixteen[dim].opacity(0.66)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_and_ramp_are_xterms() {
        assert_eq!(indexed(16), hex(0x000000));
        assert_eq!(indexed(231), hex(0xffffff));
        assert_eq!(indexed(196), hex(0xff0000));
        assert_eq!(indexed(232), hex(0x080808));
        assert_eq!(indexed(255), hex(0xeeeeee));
    }
}
