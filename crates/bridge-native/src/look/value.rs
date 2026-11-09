//! What a token's value text means: colours, lengths, durations, shadows, easing and font stacks.
//!
//! Pure, with no gpui type, so every case is a test without a window. The forms are the ones
//! `packages/tokens` and the themes use: hex, `rgb(r g b / a)`, `color-mix(in srgb, ...)`,
//! `transparent`, `Npx`, `Nms`, `Ns`, a bare number, `N%`, `cubic-bezier(...)` and a comma list.
//! A value of another form is `None`, and the caller keeps its text.

use std::time::Duration;

/// An sRGB colour with straight (not premultiplied) alpha, each channel 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    fn from_bytes(r: u32, g: u32, b: u32, a: u32) -> Color {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: a as f32 / 255.0 }
    }
}

/// A colour in any form a token holds it in. `None` for text that is not one.
pub fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        return parse_hex(hex);
    }
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "transparent" => return Some(Color::TRANSPARENT),
        "white" => return Some(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
        "black" => return Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
        _ => {}
    }
    if let Some(args) = function_args(&lower, "rgb").or_else(|| function_args(&lower, "rgba")) {
        return parse_rgb(args);
    }
    if let Some(args) = function_args(&lower, "color-mix") {
        return parse_mix(args);
    }
    None
}

fn parse_hex(hex: &str) -> Option<Color> {
    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |at: usize| u32::from_str_radix(&hex[at..at + 2], 16).ok();
    let single = |at: usize| u32::from_str_radix(&hex[at..at + 1], 16).ok().map(|nibble| nibble * 17);
    match hex.len() {
        3 => Some(Color::from_bytes(single(0)?, single(1)?, single(2)?, 255)),
        4 => Some(Color::from_bytes(single(0)?, single(1)?, single(2)?, single(3)?)),
        6 => Some(Color::from_bytes(pair(0)?, pair(2)?, pair(4)?, 255)),
        8 => Some(Color::from_bytes(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
        _ => None,
    }
}

/// `name( ... )` with `text` being exactly that call: the text between the parentheses.
fn function_args<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(name)?.trim_start().strip_prefix('(')?;
    rest.trim_end().strip_suffix(')')
}

/// `r g b`, `r g b / a` and the older comma form. A channel is 0 to 255 or a percentage.
fn parse_rgb(args: &str) -> Option<Color> {
    let (colour, alpha) = match args.split_once('/') {
        Some((colour, alpha)) => (colour, Some(alpha.trim())),
        None => (args, None),
    };
    let mut channels: Vec<&str> = colour.split(|ch: char| ch == ',' || ch.is_whitespace()).filter(|part| !part.is_empty()).collect();
    let alpha = match (alpha, channels.len()) {
        (Some(alpha), 3) => alpha,
        (None, 4) => channels.pop()?,
        (None, 3) => "1",
        _ => return None,
    };
    let channel = |part: &str| -> Option<f32> {
        match part.strip_suffix('%') {
            Some(percent) => Some((percent.parse::<f32>().ok()? / 100.0).clamp(0.0, 1.0)),
            None => Some((part.parse::<f32>().ok()? / 255.0).clamp(0.0, 1.0)),
        }
    };
    let alpha = match alpha.strip_suffix('%') {
        Some(percent) => (percent.parse::<f32>().ok()? / 100.0).clamp(0.0, 1.0),
        None => alpha.parse::<f32>().ok()?.clamp(0.0, 1.0),
    };
    Some(Color { r: channel(channels[0])?, g: channel(channels[1])?, b: channel(channels[2])?, a: alpha })
}

/// `in srgb, <colour> [N%], <colour> [N%]`. Mixed premultiplied, as CSS Color 4 says; a share
/// left out is the other's complement, and shares that do not add to 100 are scaled to it, with the
/// result's alpha scaled by the total where it is under 100.
fn parse_mix(args: &str) -> Option<Color> {
    let parts = split_top(args, ',');
    let [space, first, second] = parts.as_slice() else { return None };
    if space.trim() != "in srgb" {
        return None;
    }
    let (one, share_one) = colour_and_share(first)?;
    let (two, share_two) = colour_and_share(second)?;
    let (share_one, share_two) = match (share_one, share_two) {
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => (a, 100.0 - a),
        (None, Some(b)) => (100.0 - b, b),
        (None, None) => (50.0, 50.0),
    };
    let total = share_one + share_two;
    if total <= 0.0 {
        return None;
    }
    let (w1, w2) = (share_one / total, share_two / total);
    let alpha = one.a * w1 + two.a * w2;
    let scale = (total / 100.0).min(1.0);
    if alpha <= 0.0 {
        return Some(Color::TRANSPARENT);
    }
    let mixed = |a: f32, b: f32| (a * one.a * w1 + b * two.a * w2) / alpha;
    Some(Color { r: mixed(one.r, two.r), g: mixed(one.g, two.g), b: mixed(one.b, two.b), a: alpha * scale })
}

/// `<colour>` or `<colour> N%` (the share may also lead).
fn colour_and_share(part: &str) -> Option<(Color, Option<f32>)> {
    let words = split_top(part.trim(), ' ');
    let words: Vec<&str> = words.iter().map(|word| word.trim()).filter(|word| !word.is_empty()).collect();
    let share_of = |word: &str| word.strip_suffix('%').and_then(|n| n.parse::<f32>().ok());
    match words.as_slice() {
        [colour] => Some((parse_color(colour)?, None)),
        [colour, share] => match share_of(share) {
            Some(share) => Some((parse_color(colour)?, Some(share))),
            None => Some((parse_color(share)?, Some(share_of(colour)?))),
        },
        _ => None,
    }
}

/// Split on `separator` where it is outside every pair of parentheses and quotes.
pub fn split_top(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    let mut quote: Option<char> = None;
    for (at, ch) in text.char_indices() {
        match (quote, ch) {
            (Some(open), ch) if ch == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, ch) if ch == separator && depth == 0 => {
                parts.push(&text[start..at]);
                start = at + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

/// `12px`, `-6px`, `0.5px` or a bare `0`.
pub fn parse_px(text: &str) -> Option<f32> {
    let text = text.trim();
    match text.strip_suffix("px") {
        Some(number) => number.trim().parse().ok(),
        None if text == "0" => Some(0.0),
        None => None,
    }
}

/// `160ms` or `0.2s`.
pub fn parse_duration(text: &str) -> Option<Duration> {
    let text = text.trim();
    let seconds = match text.strip_suffix("ms") {
        Some(millis) => millis.trim().parse::<f64>().ok()? / 1000.0,
        None => text.strip_suffix('s')?.trim().parse::<f64>().ok()?,
    };
    (seconds >= 0.0 && seconds.is_finite()).then(|| Duration::from_secs_f64(seconds))
}

/// A bare number such as a font weight or a z-index.
pub fn parse_number(text: &str) -> Option<f32> {
    text.trim().parse().ok()
}

/// `40%` as 0.4.
pub fn parse_percent(text: &str) -> Option<f32> {
    text.trim().strip_suffix('%')?.trim().parse::<f32>().ok().map(|percent| percent / 100.0)
}

/// `cubic-bezier(a, b, c, d)`.
pub fn parse_bezier(text: &str) -> Option<[f32; 4]> {
    let args = function_args(text.trim(), "cubic-bezier")?;
    let numbers: Vec<f32> = args.split(',').map(|part| part.trim().parse().ok()).collect::<Option<_>>()?;
    numbers.try_into().ok()
}

/// A font stack as its family names, quotes taken off: `"IBM Plex Sans", system-ui` is two.
pub fn parse_font_stack(text: &str) -> Vec<String> {
    split_top(text, ',')
        .into_iter()
        .map(|family| family.trim().trim_matches(|ch| ch == '"' || ch == '\'').to_string())
        .filter(|family| !family.is_empty())
        .collect()
}

/// `2px solid #4A9EDB`: the width and colour of a rule or a focus ring. Only `solid` is drawn.
pub fn parse_edge(text: &str) -> Option<(f32, Color)> {
    let words: Vec<&str> = split_top(text.trim(), ' ').into_iter().map(str::trim).filter(|word| !word.is_empty()).collect();
    match words.as_slice() {
        [width, "solid", color] => Some((parse_px(width)?, parse_color(color)?)),
        _ => None,
    }
}

/// One layer of a `box-shadow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub inset: bool,
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

/// A comma list of `[inset] x y [blur [spread]] colour`, the colour first or last. `None` if any
/// layer is not that shape, so a half-read shadow is never drawn.
pub fn parse_shadows(text: &str) -> Option<Vec<Shadow>> {
    split_top(text, ',').into_iter().map(|layer| parse_shadow(layer.trim())).collect()
}

fn parse_shadow(layer: &str) -> Option<Shadow> {
    let mut inset = false;
    let mut lengths = Vec::new();
    let mut color = None;
    for word in split_top(layer, ' ').into_iter().map(str::trim).filter(|word| !word.is_empty()) {
        if word == "inset" {
            inset = true;
        } else if let Some(length) = parse_px(word) {
            lengths.push(length);
        } else {
            color = Some(parse_color(word)?);
        }
    }
    let [x, y, rest @ ..] = lengths.as_slice() else { return None };
    Some(Shadow { inset, x: *x, y: *y, blur: rest.first().copied().unwrap_or(0.0), spread: rest.get(1).copied().unwrap_or(0.0), color: color? })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.002
    }

    fn assert_color(got: Option<Color>, r: f32, g: f32, b: f32, a: f32) {
        let got = got.expect("a colour");
        assert!(near(got.r, r) && near(got.g, g) && near(got.b, b) && near(got.a, a), "{got:?}");
    }

    #[test]
    fn hex_in_every_length() {
        assert_color(parse_color("#0F1419"), 15.0 / 255.0, 20.0 / 255.0, 25.0 / 255.0, 1.0);
        assert_color(parse_color("#fff"), 1.0, 1.0, 1.0, 1.0);
        assert_color(parse_color("#00000080"), 0.0, 0.0, 0.0, 128.0 / 255.0);
        assert!(parse_color("#12345").is_none());
        assert!(parse_color("#GGGGGG").is_none());
    }

    #[test]
    fn rgb_with_a_slash_alpha() {
        assert_color(parse_color("rgb(25 33 45 / 0.86)"), 25.0 / 255.0, 33.0 / 255.0, 45.0 / 255.0, 0.86);
        assert_color(parse_color("rgb(255 255 255 / 7%)"), 1.0, 1.0, 1.0, 0.07);
        assert_color(parse_color("rgba(0, 0, 0, 0.5)"), 0.0, 0.0, 0.0, 0.5);
    }

    #[test]
    fn color_mix_with_transparent_keeps_the_hue_and_scales_the_alpha() {
        // The shape of --accent-faint: 10% of the accent over nothing is the accent at 10%.
        assert_color(
            parse_color("color-mix(in srgb, #4A9EDB 10%, transparent)"),
            0x4A as f32 / 255.0,
            0x9E as f32 / 255.0,
            0xDB as f32 / 255.0,
            0.10,
        );
    }

    #[test]
    fn color_mix_of_two_opaque_colours_is_their_blend() {
        // --phase-live-edge: 45% of one, the rest the other.
        assert_color(parse_color("color-mix(in srgb, #000000 45%, #FFFFFF)"), 0.55, 0.55, 0.55, 1.0);
        assert_color(parse_color("color-mix(in srgb, #000000, #FFFFFF)"), 0.5, 0.5, 0.5, 1.0);
    }

    #[test]
    fn color_mix_premultiplies_so_a_clear_colour_pulls_nothing() {
        // Half of white over half of a clear red is white at half alpha, not pink.
        assert_color(parse_color("color-mix(in srgb, #FFFFFF 50%, rgb(255 0 0 / 0))"), 1.0, 1.0, 1.0, 0.5);
    }

    #[test]
    fn shares_under_a_hundred_scale_the_alpha() {
        assert_color(parse_color("color-mix(in srgb, #FFFFFF 20%, #FFFFFF 20%)"), 1.0, 1.0, 1.0, 0.4);
    }

    #[test]
    fn a_colour_space_other_than_srgb_is_not_guessed() {
        assert!(parse_color("color-mix(in oklch, #000 50%, #fff)").is_none());
    }

    #[test]
    fn lengths_durations_and_numbers() {
        assert_eq!(parse_px("28px"), Some(28.0));
        assert_eq!(parse_px("-6px"), Some(-6.0));
        assert_eq!(parse_px("0"), Some(0.0));
        assert_eq!(parse_px("2px solid red"), None);
        assert_eq!(parse_duration("160ms"), Some(Duration::from_millis(160)));
        assert_eq!(parse_duration("45s"), Some(Duration::from_secs(45)));
        assert_eq!(parse_number("600"), Some(600.0));
        assert_eq!(parse_percent("40%"), Some(0.4));
    }

    #[test]
    fn easing_and_font_stacks() {
        assert_eq!(parse_bezier("cubic-bezier(0.2, 0, 0, 1)"), Some([0.2, 0.0, 0.0, 1.0]));
        assert_eq!(parse_bezier("cubic-bezier(0.2, 0, 0)"), None);
        assert_eq!(
            parse_font_stack("\"IBM Plex Sans\", ui-sans-serif, system-ui, sans-serif"),
            vec!["IBM Plex Sans", "ui-sans-serif", "system-ui", "sans-serif"]
        );
    }

    #[test]
    fn a_shadow_with_a_colour_mix_inside_it_is_split_at_the_right_commas() {
        let text = "inset 0 1px 0 rgb(255 255 255 / 0.25), 0 2px 10px -3px color-mix(in srgb, #4A9EDB 45%, transparent)";
        let shadows = parse_shadows(text).expect("two layers");
        assert_eq!(shadows.len(), 2);
        assert!(shadows[0].inset);
        assert_eq!((shadows[1].y, shadows[1].blur, shadows[1].spread), (2.0, 10.0, -3.0));
        assert!(near(shadows[1].color.a, 0.45));
    }

    #[test]
    fn an_edge_is_a_width_and_a_colour() {
        let (width, color) = parse_edge("2px solid #4A9EDB").expect("an edge");
        assert_eq!(width, 2.0);
        assert!(near(color.b, 0xDB as f32 / 255.0));
        assert!(parse_edge("2px dashed #4A9EDB").is_none());
    }

    #[test]
    fn a_shadow_that_is_not_that_shape_is_none() {
        assert!(parse_shadows("0 8px").is_none());
    }
}
