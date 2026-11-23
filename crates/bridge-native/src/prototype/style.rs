//! The attributes any part may carry, read against the theme.
//!
//! `pad` and `gap` take a space step (`3` is `--space-3`); `radius` a name (`sm`, `md`, `lg`,
//! `full`) or points; `fill`, `edge` and `tone` a colour token as `tokens.json` names it
//! (`bg-raised`, `border-subtle`) or a tone word (`success`); `w` and `h` points, a percentage or
//! `full`. A value the theme has no token for is recorded in the trouble list and drawn in the
//! marker colour, never as something that looks right.

use gpui::{Hsla, Pixels};

use super::markup::Node;
use crate::look::Look;
use crate::parts::Tone;

/// What went wrong reading a scene, each once, in the order it was found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trouble(Vec<String>);

impl Trouble {
    pub fn push(&mut self, line: String) {
        if !self.0.contains(&line) {
            self.0.push(line);
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.0
    }

    pub fn into_lines(self) -> Vec<String> {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Points(Pixels),
    Fraction(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
}

/// The box attributes of one part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// Vertical and horizontal padding.
    pub pad: Option<(Pixels, Pixels)>,
    pub gap: Option<Pixels>,
    pub radius: Option<Pixels>,
    pub fill: Option<Hsla>,
    pub edge: Option<Hsla>,
    /// `Some(true)` is a row, `Some(false)` a column.
    pub row: Option<bool>,
    pub align: Option<Align>,
    pub justify: Option<Justify>,
    pub grow: bool,
    pub w: Option<Size>,
    pub h: Option<Size>,
}

/// A colour by tone word or token name; the marker colour, and a line of trouble, where it is neither.
pub fn colour(look: &Look, node: &Node, attr: &str, word: &str, trouble: &mut Trouble) -> Hsla {
    if let Some(tone) = Tone::named(word) {
        return tone.ink(look);
    }
    match look.color_named(word) {
        Ok(colour) => colour,
        Err(_) => {
            trouble.push(format!("<{}> {attr}=\"{word}\" is no colour token", node.tag));
            look.unknown_marker()
        }
    }
}

fn space(look: &Look, node: &Node, attr: &str, word: &str, trouble: &mut Trouble) -> Pixels {
    if word == "0" {
        return gpui::px(0.);
    }
    match look.length_named(&format!("space-{word}")) {
        Ok(step) => step,
        Err(_) => {
            trouble.push(format!("<{}> {attr}=\"{word}\" is no space step", node.tag));
            gpui::px(0.)
        }
    }
}

fn size(look: &Look, node: &Node, attr: &str, word: &str, trouble: &mut Trouble) -> Option<Size> {
    let _ = look;
    if word == "full" {
        return Some(Size::Fraction(1.));
    }
    if let Some(percent) = word.strip_suffix('%').and_then(|number| number.parse::<f32>().ok()) {
        return Some(Size::Fraction(percent / 100.));
    }
    match word.parse::<f32>() {
        Ok(points) => Some(Size::Points(gpui::px(points))),
        Err(_) => {
            trouble.push(format!("<{}> {attr}=\"{word}\" is not points, a percentage or full", node.tag));
            None
        }
    }
}

pub fn read(look: &Look, node: &Node, trouble: &mut Trouble) -> Style {
    let mut style = Style::default();
    if let Some(word) = node.attr("pad") {
        let steps: Vec<Pixels> = word.split_whitespace().map(|one| space(look, node, "pad", one, trouble)).collect();
        style.pad = match steps.as_slice() {
            [all] => Some((*all, *all)),
            [y, x] => Some((*y, *x)),
            _ => {
                trouble.push(format!("<{}> pad=\"{word}\" takes one step or two", node.tag));
                None
            }
        };
    }
    if let Some(word) = node.attr("gap") {
        style.gap = Some(space(look, node, "gap", word, trouble));
    }
    if let Some(word) = node.attr("radius") {
        style.radius = match word {
            "full" => Some(gpui::px(9999.)),
            "sm" | "md" | "lg" => look.length_named(&format!("radius-{word}")).ok(),
            points => match points.parse::<f32>() {
                Ok(points) => Some(gpui::px(points)),
                Err(_) => {
                    trouble.push(format!("<{}> radius=\"{word}\" is not sm, md, lg, full or points", node.tag));
                    None
                }
            },
        };
    }
    style.fill = node.attr("fill").map(|word| colour(look, node, "fill", word, trouble));
    style.edge = node.attr("edge").map(|word| colour(look, node, "edge", word, trouble));
    if let Some(word) = node.attr("dir") {
        match word {
            "row" => style.row = Some(true),
            "col" | "column" => style.row = Some(false),
            _ => trouble.push(format!("<{}> dir=\"{word}\" is not row or col", node.tag)),
        }
    }
    if let Some(word) = node.attr("align") {
        style.align = match word {
            "start" => Some(Align::Start),
            "center" => Some(Align::Center),
            "end" => Some(Align::End),
            "stretch" => Some(Align::Stretch),
            _ => {
                trouble.push(format!("<{}> align=\"{word}\" is not start, center, end or stretch", node.tag));
                None
            }
        };
    }
    if let Some(word) = node.attr("justify") {
        style.justify = match word {
            "start" => Some(Justify::Start),
            "center" => Some(Justify::Center),
            "end" => Some(Justify::End),
            "between" => Some(Justify::Between),
            _ => {
                trouble.push(format!("<{}> justify=\"{word}\" is not start, center, end or between", node.tag));
                None
            }
        };
    }
    style.grow = node.flag("grow");
    style.w = node.attr("w").and_then(|word| size(look, node, "w", word, trouble));
    style.h = node.attr("h").and_then(|word| size(look, node, "h", word, trouble));
    style
}
