//! A compiled chart written as SVG, for a page that cannot hold gpui: an `<armada-graph>` in a document
//! (`job_view/page_graphs.rs`). It is laid out by `plot.rs`'s own frame, scales, ticks and labels, in the same inks
//! written as hex, so the page's graph and the thread's card read alike. A page runs no script, so the readout at each x
//! is drawn hidden and shown on hover by the page's stylesheet (`.graph-at:hover`).

use std::fmt::Write as _;
use std::rc::Rc;

use gpui::{point, px, size, Bounds, Hsla, Pixels};
use gpui_component::plot::shape::Pie;

use super::compile::{Chart, Curve, Key, Layer, XKind};
use super::plot::{Frame, GraphPlot, Inks, DOT, GAP, GUTTER_GAP, REACH};
use super::ticks;
use crate::scene::export::{css, xml};

/// How wide `text` stands at `font` points, near enough to lay out by: a page has no text system to ask.
pub fn width_of(text: &str, font: f32) -> f32 {
    text.chars()
        .map(|c| match c {
            '0'..='9' => 0.56,
            ' ' | '.' | ',' | ':' | '\'' | 'i' | 'l' | 'j' | 't' | 'f' | 'I' => 0.3,
            'm' | 'w' | 'M' | 'W' => 0.84,
            c if c.is_uppercase() => 0.64,
            _ => 0.53,
        })
        .sum::<f32>()
        * font
}

fn n(value: f32) -> String {
    let rounded = (value * 10.).round() / 10.;
    if rounded == 0. { "0".into() } else { format!("{rounded}") }
}

/// A colour's hue as hex, its opacity written apart (`alpha`).
fn ink(color: Hsla) -> String {
    css(Hsla { a: 1., ..color })
}

/// The opacity a fill or stroke takes, written beside its hex.
fn alpha(color: Hsla) -> String {
    n(color.a)
}

/// The path through `points` as the plot's curve joins them: straight, a Catmull-Rom curve as gpui-component's
/// `Natural` draws it, or a step after each point. `from` says whether it starts with a move or a line.
fn through(points: &[(f32, f32)], curve: Curve, from: char) -> String {
    let mut d = String::new();
    let Some((x0, y0)) = points.first() else { return d };
    let _ = write!(d, "{from}{},{}", n(*x0), n(*y0));
    let count = points.len();
    match curve {
        Curve::Linear => {
            for (x, y) in &points[1..] {
                let _ = write!(d, "L{},{}", n(*x), n(*y));
            }
        }
        Curve::Smooth => {
            for at in 0..count.saturating_sub(1) {
                let p0 = points[at.saturating_sub(1)];
                let (p1, p2) = (points[at], points[at + 1]);
                let p3 = points[(at + 2).min(count - 1)];
                let c1 = (p1.0 + (p2.0 - p0.0) / 6., p1.1 + (p2.1 - p0.1) / 6.);
                let c2 = (p2.0 - (p3.0 - p1.0) / 6., p2.1 - (p3.1 - p1.1) / 6.);
                let _ = write!(d, "C{},{} {},{} {},{}", n(c1.0), n(c1.1), n(c2.0), n(c2.1), n(p2.0), n(p2.1));
            }
        }
        Curve::Step => {
            for (at, pair) in points.windows(2).enumerate() {
                let _ = write!(d, "L{},{}", n(pair[1].0), n(pair[0].1));
                if at + 2 < count {
                    let _ = write!(d, "L{},{}", n(pair[1].0), n(pair[1].1));
                }
            }
        }
    }
    d
}

/// Words whose line box stands with its top at `(x, top)`, anchored at its start, middle or end.
fn text(out: &mut String, words: &str, (x, top): (f32, f32), font: f32, color: Hsla, anchor: &str, weight: Option<u16>) {
    let weight = weight.map(|weight| format!(r#" font-weight="{weight}""#)).unwrap_or_default();
    // gpui sets a label's line box at its top, one font tall; `central` stands the words in the middle of it as gpui does.
    let _ = write!(
        out,
        r#"<text x="{}" y="{}" font-size="{}" fill="{}" text-anchor="{anchor}" dominant-baseline="central"{weight}>{}</text>"#,
        n(x),
        n(top + font / 2.),
        n(font),
        ink(color),
        xml(words)
    );
}

fn dot(out: &mut String, (x, y): (f32, f32), fill: Hsla, ring: Hsla) {
    let _ = write!(out, r#"<circle cx="{}" cy="{}" r="{}" fill="{}" fill-opacity="{}" stroke="{}" stroke-width="1.5"/>"#, n(x), n(y), n((DOT + 3.) / 2. - 0.75), ink(fill), alpha(fill), ink(ring));
}

/// A chart as SVG `width` by `height` points, named `name` for VoiceOver. `key` keeps its ids apart from another
/// chart's on the same page.
pub fn svg(chart: Rc<Chart>, inks: &Inks, width: f32, height: f32, name: &str, key: &str) -> String {
    let font = f32::from(inks.font);
    let bare = GraphPlot::new("graph-svg", chart.clone(), inks.clone());
    let gutter = if chart.y.shown && chart.arcs.is_none() { chart.y.ticks.iter().map(|tick| width_of(&bare.y_label(*tick), font)).fold(0., f32::max) } else { 0. };
    let plot = bare.with_gutter(gutter);
    let bounds: Bounds<Pixels> = Bounds::new(point(px(0.), px(0.)), size(px(width), px(height)));
    let mut out = String::new();
    let _ = write!(out, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="{}">"#, xml(name), w = n(width), h = n(height));
    if chart.arcs.is_some() {
        arcs(&mut out, &chart, inks, width, height);
    } else {
        let frame = plot.frame(bounds);
        axes(&mut out, &plot, &frame, inks);
        marks(&mut out, &plot, &frame, inks, key);
        hovers(&mut out, &plot, &frame, inks, width);
    }
    out.push_str("</svg>");
    out
}

fn axes(out: &mut String, plot: &GraphPlot, frame: &Frame, inks: &Inks) {
    let chart = &plot.chart;
    let font = f32::from(inks.font);
    if chart.y.grid {
        for tick in &chart.y.ticks {
            let at = plot.y_px(frame, *tick);
            if (at - frame.bottom).abs() > 0.5 {
                let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="1" fill="{}"/>"#, n(frame.left), n(at - 0.5), n(frame.right - frame.left), ink(inks.grid));
            }
        }
    }
    let base = plot.y_px(frame, chart.y.low.max(0.).min(chart.y.high));
    let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="1" fill="{}"/>"#, n(frame.left), n(base - 0.5), n(frame.right - frame.left), ink(inks.axis));
    if let Some(caption) = plot.y_caption() {
        text(out, &caption, (0., 0.), font, inks.tick, "start", None);
    }
    if let Some(caption) = plot.x_caption() {
        text(out, &caption, (frame.right, frame.bottom + 7. + font + 6.), font, inks.tick, "end", None);
    }
    if chart.y.shown {
        for tick in &chart.y.ticks {
            let at = plot.y_px(frame, *tick);
            text(out, &plot.y_label(*tick), (frame.left - GUTTER_GAP, at - font / 2. - 1.), font, inks.tick, "end", None);
        }
    }
    if !chart.x.shown {
        return;
    }
    let y = frame.bottom + 7.;
    match chart.x.kind {
        XKind::Band => {
            for (at, _) in chart.x.bands.iter().enumerate() {
                text(out, &plot.x_label(at as f64), (plot.x_px(frame, at as f64), y), font, inks.tick, "middle", None);
            }
        }
        kind => {
            let count = (((frame.right - frame.left) / 52.).floor() as usize).max(2);
            let marks = match kind {
                XKind::Time => ticks::time_ticks(chart.x.low, chart.x.high, count),
                _ => ticks::value_ticks(chart.x.low, chart.x.high, count).2.into_iter().filter(|tick| *tick >= chart.x.low && *tick <= chart.x.high).collect(),
            };
            for tick in marks {
                let words = plot.x_label(tick);
                let half = width_of(&words, font) / 2.;
                let at = plot.x_px(frame, tick).clamp(frame.left + half, frame.right - half);
                text(out, &words, (at, y), font, inks.tick, "middle", None);
            }
        }
    }
}

fn marks(out: &mut String, plot: &GraphPlot, frame: &Frame, inks: &Inks, key: &str) {
    let chart = &plot.chart;
    let font = f32::from(inks.font);
    let _ = write!(
        out,
        r#"<defs><clipPath id="{key}-clip"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath></defs>"#,
        n(frame.left - 2.),
        n(frame.top - 4.),
        n(frame.right - frame.left + 4.),
        n(frame.bottom - frame.top + 6.)
    );
    let mut words = Vec::new();
    for (at, layer) in chart.layers.iter().enumerate() {
        match layer {
            Layer::Area { series, points, curve, stacked } => {
                let hue = inks.of_series(chart.series[*series].slot);
                let placed: Vec<(f32, f32, f32)> = points.iter().map(|(x, top, base)| (plot.x_px(frame, *x), plot.y_px(frame, *top), plot.y_px(frame, *base))).collect();
                let tops: Vec<(f32, f32)> = placed.iter().map(|(x, y, _)| (*x, *y)).collect();
                let _ = write!(out, r#"<g clip-path="url(#{key}-clip)">"#);
                if *stacked {
                    let bases: Vec<(f32, f32)> = placed.iter().rev().map(|(x, _, base)| (*x, *base)).collect();
                    let d = format!("{}{}Z", through(&tops, Curve::Linear, 'M'), through(&bases, Curve::Linear, 'L'));
                    let _ = write!(out, r#"<path d="{d}" fill="{}" fill-opacity="0.32"/>"#, ink(hue));
                } else if let (Some(first), Some(last)) = (placed.first(), placed.last()) {
                    // A wash of the hue under the line, fading to nothing at the baseline.
                    let base = first.2;
                    let _ = write!(
                        out,
                        r#"<defs><linearGradient id="{key}-fade-{at}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{c}" stop-opacity="0.3"/><stop offset="1" stop-color="{c}" stop-opacity="0.02"/></linearGradient></defs>"#,
                        c = ink(hue)
                    );
                    let d = format!("M{},{}{}L{},{}Z", n(first.0), n(base), through(&tops, *curve, 'L'), n(last.0), n(base));
                    let _ = write!(out, r#"<path d="{d}" fill="url(#{key}-fade-{at})"/>"#);
                }
                let _ = write!(out, r#"<path d="{}" fill="none" stroke="{}" stroke-width="2"/></g>"#, through(&tops, *curve, 'M'), ink(hue));
            }
            Layer::Bars { bars } => {
                let width = plot.bar_width(frame);
                let radius = (width / 2.).min(4.);
                for (x, segments) in bars {
                    let left = plot.x_px(frame, *x) - width / 2.;
                    let top = segments.iter().map(|(_, _, to)| *to).fold(f64::NEG_INFINITY, f64::max);
                    for (series, from, to) in segments {
                        let (base, end) = (plot.y_px(frame, *from), plot.y_px(frame, *to));
                        let base = if *from > chart.y.low.max(0.) { base - GAP } else { base };
                        if base - end < 0.5 {
                            continue;
                        }
                        let hue = ink(inks.of_series(chart.series[*series].slot));
                        let r = if *to >= top { radius.min(base - end) } else { 0. };
                        let (l, rt) = (left, left + width);
                        let d = format!(
                            "M{},{}L{},{}Q{},{} {},{}L{},{}Q{},{} {},{}L{},{}Z",
                            n(l),
                            n(base),
                            n(l),
                            n(end + r),
                            n(l),
                            n(end),
                            n(l + r),
                            n(end),
                            n(rt - r),
                            n(end),
                            n(rt),
                            n(end),
                            n(rt),
                            n(end + r),
                            n(rt),
                            n(base)
                        );
                        let _ = write!(out, r#"<path d="{d}" fill="{hue}"/>"#);
                    }
                }
            }
            Layer::Line { series, points, curve, dash, dots } => {
                let hue = inks.of_series(chart.series[*series].slot);
                let placed: Vec<(f32, f32)> = points.iter().map(|(x, y)| (plot.x_px(frame, *x), plot.y_px(frame, *y))).collect();
                let line = match dash {
                    // A dashed line is joined straight with round ends, as `paint_dashed` draws it.
                    Some(dash) => format!(
                        r#"<path d="{}" fill="none" stroke="{}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="{}"/>"#,
                        through(&placed, Curve::Linear, 'M'),
                        ink(hue),
                        dash.iter().map(|one| n(*one)).collect::<Vec<_>>().join(" ")
                    ),
                    None => format!(r#"<path d="{}" fill="none" stroke="{}" stroke-width="2"/>"#, through(&placed, *curve, 'M'), ink(hue)),
                };
                let _ = write!(out, r#"<g clip-path="url(#{key}-clip)">{line}</g>"#);
                if *dots {
                    for at in &placed {
                        dot(out, *at, hue, inks.surface);
                    }
                }
            }
            Layer::Points { points } => {
                for (x, y, series) in points {
                    let hue = inks.of_series(chart.series[*series].slot);
                    dot(out, (plot.x_px(frame, *x), plot.y_px(frame, *y)), hue.opacity(0.9), inks.surface);
                }
            }
            Layer::Rule { across, at, dash } => {
                let dashes = dash.as_ref().filter(|dash| !dash.is_empty()).map(|dash| format!(r#" stroke-dasharray="{}""#, dash.iter().map(|one| n(*one)).collect::<Vec<_>>().join(" "))).unwrap_or_default();
                if *across {
                    let y = plot.y_px(frame, *at);
                    let _ = write!(out, r#"<line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="{}" stroke-width="1"{dashes}/>"#, n(frame.left), n(frame.right), ink(inks.rule), y = n(y));
                } else {
                    let x = plot.x_px(frame, *at);
                    let _ = write!(out, r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" stroke="{}" stroke-width="1"{dashes}/>"#, n(frame.top), n(frame.bottom), ink(inks.marker), x = n(x));
                    // A marker down the plot carries a dot at its top, where its words begin.
                    let _ = write!(out, r#"<circle cx="{}" cy="{}" r="2.5" fill="{}"/>"#, n(x), n(frame.top), ink(inks.marker));
                }
            }
            Layer::Text { x, y, text, align_left } => words.push((*x, *y, text.clone(), *align_left)),
        }
    }
    // Words last, on a pill of the surface, placed as `paint_marks` places them.
    for (x, y, said, align_left) in words {
        let width = width_of(&said, font);
        let (pad_x, pad_y) = (6., 2.);
        let (at_x, at_y) = match (x, y) {
            (Some(x), None) => (plot.x_px(frame, x) + 6., frame.top + 2.),
            (None, Some(y)) => {
                let top = plot.y_px(frame, y) - font - pad_y * 2. - 3.;
                let wide = width + pad_x * 2.;
                let right = frame.right - wide - 2.;
                let left = frame.left + 4.;
                let at_x = if plot.crosses(frame, (right, top), (frame.right, top + font + pad_y * 2.)) && !plot.crosses(frame, (left, top), (left + wide, top + font + pad_y * 2.)) { left } else { right };
                (at_x, top)
            }
            (Some(x), Some(y)) => {
                let x = plot.x_px(frame, x);
                (if align_left { x + 6. } else { x - width - pad_x * 2. - 6. }, plot.y_px(frame, y) - font / 2. - pad_y)
            }
            (None, None) => (frame.right - width - pad_x * 2., frame.top),
        };
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{}" stroke="{}" stroke-width="1"/>"#,
            n(at_x + 0.5),
            n(at_y + 0.5),
            n(width + pad_x * 2. - 1.),
            n(font + pad_y * 2. + 1.),
            ink(inks.pill),
            ink(inks.axis)
        );
        text(out, &said, (at_x + pad_x, at_y + pad_y + 1.), font, inks.label, "start", Some(500));
    }
}

/// One readout: its title and a row per series, standing inside the plot's top beside `cross` on the side with more room.
fn readout(out: &mut String, title: &str, rows: &[(Hsla, Key, String, String)], cross: f32, frame: &Frame, inks: &Inks) {
    let font = f32::from(inks.font);
    let line = font + 5.;
    let (pad_x, pad_y, mark, gap) = (8., 6., 12., 6.);
    let widest = rows.iter().map(|(_, _, value, name)| mark + gap + width_of(value, font) * 1.04 + gap + width_of(name, font)).fold(width_of(title, font), f32::max);
    let (wide, tall) = (widest + pad_x * 2., pad_y * 2. + line * (rows.len() + 1) as f32 + 2. * rows.len() as f32);
    let left = if frame.right - cross >= cross - frame.left { cross + 10. } else { cross - 10. - wide };
    let top = frame.top + 4.;
    let _ = write!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="{}" fill-opacity="0.86" stroke="{}" stroke-width="1"/>"#,
        n(left),
        n(top),
        n(wide),
        n(tall),
        ink(inks.pill),
        ink(inks.axis)
    );
    let mut y = top + pad_y + (line - font) / 2.;
    text(out, title, (left + pad_x, y), font, inks.label, "start", None);
    for (hue, key, value, name) in rows {
        y += line + 2.;
        let middle = y + font / 2.;
        let x = left + pad_x;
        let _ = match key {
            Key::Line => write!(out, r#"<rect x="{}" y="{}" width="12" height="2" rx="1" fill="{}"/>"#, n(x), n(middle - 1.), ink(*hue)),
            Key::Fill => write!(out, r#"<rect x="{}" y="{}" width="8" height="8" rx="2" fill="{}"/>"#, n(x + 2.), n(middle - 4.), ink(*hue)),
            Key::Dot if hue.a > 0. => write!(out, r#"<circle cx="{}" cy="{}" r="4" fill="{}"/>"#, n(x + 6.), n(middle), ink(*hue)),
            Key::Dot => Ok(()),
        };
        let at = x + mark + gap;
        text(out, value, (at, y), font, inks.strong, "start", Some(600));
        text(out, name, (at + width_of(value, font) * 1.04 + gap, y), font, inks.label, "start", None);
    }
}

/// The readout at each x, each hidden until the pointer is over its column; a scatter's at each dot.
fn hovers(out: &mut String, plot: &GraphPlot, frame: &Frame, inks: &Inks, width: f32) {
    let chart = &plot.chart;
    let tall = frame.bottom - frame.top;
    if chart.scatter {
        let points: Vec<(f64, f64, usize)> = chart.layers.iter().filter_map(|layer| if let Layer::Points { points } = layer { Some(points.clone()) } else { None }).flatten().collect();
        for (x, y, series) in points {
            let at = (plot.x_px(frame, x), plot.y_px(frame, y));
            let one = &chart.series[series];
            let hue = inks.of_series(one.slot);
            let x_words = match chart.x.kind {
                XKind::Time => ticks::clock(x),
                _ => ticks::with_unit(x, chart.x.unit.as_deref()),
            };
            let named = |title: &Option<String>, fallback: &str| title.clone().unwrap_or_else(|| fallback.to_string());
            let rows = [(hue, Key::Dot, ticks::with_unit(y, chart.y.unit.as_deref()), named(&chart.y.title, "y")), (gpui::transparent_black(), Key::Dot, x_words, named(&chart.x.title, "x"))];
            let _ = write!(out, r#"<g class="graph-at"><circle cx="{}" cy="{}" r="{}" fill="none" pointer-events="all"/><g class="graph-show">"#, n(at.0), n(at.1), n(REACH / 2.));
            let _ = write!(out, r#"<circle cx="{}" cy="{}" r="10" fill="{}" fill-opacity="0.2"/><circle cx="{}" cy="{}" r="4.5" fill="{}" stroke="{}" stroke-width="1"/>"#, n(at.0), n(at.1), ink(hue), n(at.0), n(at.1), ink(hue), ink(inks.surface));
            readout(out, &one.name, &rows, at.0, frame, inks);
            out.push_str("</g></g>");
        }
        return;
    }
    let xs = chart.readings.xs();
    let placed: Vec<f32> = xs.iter().map(|x| plot.x_px(frame, *x)).collect();
    let bars = plot.has_bars();
    for (at, x) in xs.iter().enumerate() {
        let cross = placed[at];
        let from = if at == 0 { 0. } else { (placed[at - 1] + cross) / 2. };
        let to = if at + 1 == placed.len() { width } else { (cross + placed[at + 1]) / 2. };
        let _ = write!(out, r#"<g class="graph-at"><rect x="{}" y="{}" width="{}" height="{}" fill="none" pointer-events="all"/><g class="graph-show">"#, n(from), n(frame.top), n((to - from).max(1.)), n(tall));
        if bars {
            let wide = plot.bar_width(frame) + 8.;
            let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{}" fill-opacity="0.07"/>"#, n(cross - wide / 2.), n(frame.top), n(wide), n(tall), ink(inks.strong));
        } else {
            let _ = write!(out, r#"<rect x="{}" y="{}" width="1" height="{}" fill="{}" fill-opacity="0.6"/>"#, n(cross - 0.5), n(frame.top), n(tall), ink(inks.label));
        }
        // A dot on each line and area at the x, where its value is drawn.
        for layer in &chart.layers {
            let found = match layer {
                Layer::Line { series, points, .. } => points.iter().find(|(px_, _)| px_ == x).map(|(_, y)| (*series, *y)),
                Layer::Area { series, points, .. } => points.iter().find(|(px_, _, _)| px_ == x).map(|(_, top, _)| (*series, *top)),
                _ => None,
            };
            if let Some((series, y)) = found {
                let hue = inks.of_series(chart.series[series].slot);
                let y = plot.y_px(frame, y);
                let _ = write!(out, r#"<circle cx="{}" cy="{}" r="10" fill="{}" fill-opacity="0.2"/><circle cx="{}" cy="{}" r="4.5" fill="{}" stroke="{}" stroke-width="1"/>"#, n(cross), n(y), ink(hue), n(cross), n(y), ink(hue), ink(inks.surface));
            }
        }
        let title = match chart.x.kind {
            XKind::Time => ticks::clock(*x),
            XKind::Number => ticks::with_unit(*x, chart.x.unit.as_deref()),
            XKind::Band => chart.x.bands.get(*x as usize).cloned().unwrap_or_default(),
        };
        readout(out, &title, &plot.rows_at(*x), cross, frame, inks);
        out.push_str("</g></g>");
    }
}

/// A donut: each slice with the gap gpui-component's arc leaves, and the whole in the hole with its unit under it.
fn arcs(out: &mut String, chart: &Chart, inks: &Inks, width: f32, height: f32) {
    let Some(arcs) = &chart.arcs else { return };
    let outer = (width.min(height) / 2. - 2.).max(1.);
    let inner = arcs.hole.map_or(outer * 0.66, |hole| hole.min(outer - 6.));
    let (cx, cy) = (width / 2., height / 2.);
    let slices: Vec<(usize, f32)> = arcs.slices.iter().map(|(series, value)| (*series, *value as f32)).collect();
    let pie = Pie::new().value(|(_, value): &(usize, f32)| Some(*value)).pad_angle(0.025);
    let quarter = std::f32::consts::FRAC_PI_2;
    for arc in pie.arcs(&slices) {
        let (start, end) = (arc.start_angle - quarter, arc.end_angle - quarter);
        let span = end - start;
        let pad = if span >= std::f32::consts::PI { 0. } else { arc.pad_angle };
        let (outer_pad, inner_pad) = (pad, (outer * pad / inner).min(span * 0.8));
        let (a0, a1) = (start + outer_pad / 2., end - outer_pad / 2.);
        let (b0, b1) = (start + inner_pad / 2., end - inner_pad / 2.);
        let large = if a1 - a0 > std::f32::consts::PI { 1 } else { 0 };
        let at = |r: f32, angle: f32| (cx + r * angle.cos(), cy + r * angle.sin());
        let (p0, p1, q1, q0) = (at(outer, a0), at(outer, a1), at(inner, b1), at(inner, b0));
        let hue = ink(inks.of_series(chart.series[arc.data.0].slot));
        let _ = write!(
            out,
            r#"<path d="M{},{}A{o},{o} 0 {large} 1 {},{}L{},{}A{i},{i} 0 {large} 0 {},{}Z" fill="{hue}"/>"#,
            n(p0.0),
            n(p0.1),
            n(p1.0),
            n(p1.1),
            n(q1.0),
            n(q1.1),
            n(q0.0),
            n(q0.1),
            o = n(outer),
            i = n(inner)
        );
    }
    let whole: f64 = arcs.slices.iter().map(|(_, value)| *value).sum();
    let big = f32::from(inks.big);
    text(out, &ticks::axis_number(whole, whole), (cx, cy - big * 0.72), big, inks.strong, "middle", None);
    if let Some(unit) = &arcs.unit {
        text(out, unit, (cx, cy + big * 0.36), f32::from(inks.font), inks.label, "middle", None);
    }
}

/// The legend over a chart of two series or more, as the card draws it: each series' key and name. A donut's names its
/// shares beside it instead (`shares`).
pub fn legend(chart: &Chart, inks: &Inks) -> Option<String> {
    if !chart.legend() || chart.arcs.is_some() {
        return None;
    }
    let mut out = String::from(r#"<div class="graph-legend">"#);
    for series in &chart.series {
        let _ = write!(out, r#"<span class="graph-key"><i class="graph-key-{}" style="background:{}"></i>{}</span>"#, key_word(series.key), ink(inks.of_series(series.slot)), xml(&series.name));
    }
    out.push_str("</div>");
    Some(out)
}

/// A donut's slices as rows beside it: each name with its share, and its own figure under it.
pub fn shares(chart: &Chart, inks: &Inks) -> Option<String> {
    let arcs = chart.arcs.as_ref()?;
    let whole: f64 = arcs.slices.iter().map(|(_, value)| *value).sum();
    let mut out = String::from(r#"<div class="graph-shares">"#);
    for (series, value) in &arcs.slices {
        let one = &chart.series[*series];
        let share = if whole > 0. { value / whole * 100. } else { 0. };
        let _ = write!(
            out,
            r#"<div class="graph-share"><span class="graph-key"><i class="graph-key-fill" style="background:{}"></i>{}</span><b>{}%</b><small>{}</small></div>"#,
            ink(inks.of_series(one.slot)),
            xml(&one.name),
            ticks::grouped((share * 10.).round() / 10.),
            xml(&ticks::with_unit(*value, arcs.unit.as_deref()))
        );
    }
    out.push_str("</div>");
    Some(out)
}

fn key_word(key: Key) -> &'static str {
    match key {
        Key::Line => "line",
        Key::Fill => "fill",
        Key::Dot => "dot",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::graph::compile::compile;

    fn inks() -> Inks {
        let hue = |h: f32| gpui::hsla(h, 0.6, 0.5, 1.);
        Inks {
            series: [hue(0.6), hue(0.1), hue(0.3), hue(0.8), hue(0.45), hue(0.95)],
            other: hue(0.),
            tick: hue(0.2),
            label: hue(0.25),
            strong: gpui::hsla(0., 0., 0.9, 1.),
            grid: gpui::hsla(0., 0., 0.2, 1.),
            axis: gpui::hsla(0., 0., 0.3, 1.),
            rule: gpui::hsla(0., 0., 0.6, 1.),
            marker: gpui::hsla(0., 0., 0.4, 1.),
            surface: gpui::hsla(0., 0., 0.08, 1.),
            pill: gpui::hsla(0., 0., 0.12, 1.),
            font: px(12.),
            big: px(20.),
        }
    }

    fn chart(spec: &str) -> Rc<Chart> {
        let spec = ipc::graph::read_spec(spec).expect("the spec reads");
        Rc::new(compile(&spec, &[], &BTreeMap::new()).expect("the spec draws"))
    }

    const LINES: &str = r#"{"data":{"values":[{"at":"2026-11-10T14:00:00Z","p50":40,"p99":200},{"at":"2026-11-10T14:01:00Z","p50":44,"p99":230},{"at":"2026-11-10T14:02:00Z","p50":42,"p99":260}]},
        "layer":[
        {"mark":"line","encoding":{"x":{"field":"at","type":"temporal"},"y":{"field":"p50","type":"quantitative","title":"p50 (ms)"}}},
        {"mark":"line","encoding":{"x":{"field":"at","type":"temporal"},"y":{"field":"p99","type":"quantitative","title":"p99 (ms)"}}},
        {"mark":{"type":"rule","strokeDash":[4,4]},"encoding":{"y":{"datum":250}}}]}"#;

    #[test]
    fn a_line_chart_is_written_in_the_series_hues_with_its_axis_labels_and_a_readout_at_each_x() {
        let inks = inks();
        let chart = chart(LINES);
        let svg = svg(chart.clone(), &inks, 600., 200., "Chart: Latency", "g0");
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200""#) && svg.ends_with("</svg>"), "{svg}");
        assert!(svg.contains(r#"role="img" aria-label="Chart: Latency""#), "it is named for VoiceOver");
        for slot in 0..2 {
            assert!(svg.contains(&format!(r#"stroke="{}" stroke-width="2""#, css(inks.series[slot]))), "series {slot} is stroked in its hue");
        }
        assert!(svg.contains(r#"stroke-dasharray="4 4""#), "the threshold is dashed");
        for tick in &chart.y.ticks {
            assert!(svg.contains(&format!(">{}</text>", ticks::axis_number(*tick, chart.y.high))), "the y label {tick} is written");
        }
        assert!(svg.contains(">ms</text>"), "the unit stands over the y labels");
        assert_eq!(svg.matches(r#"<g class="graph-at">"#).count(), 3, "a readout for each minute");
        assert!(svg.contains(&format!(">{}</text>", ticks::clock(chart.readings.xs()[0]))), "each readout is titled with its time");
        assert!(svg.contains(">p99</text>") && svg.contains(">260 ms</text>"), "the readout names each series with its value");
    }

    #[test]
    fn the_svg_stands_each_mark_where_the_plot_paints_it() {
        let inks = inks();
        let chart = chart(LINES);
        let plot = GraphPlot::new("t", chart.clone(), inks.clone()).with_gutter(chart.y.ticks.iter().map(|tick| width_of(&GraphPlot::new("w", chart.clone(), inks.clone()).y_label(*tick), 12.)).fold(0., f32::max));
        let frame = plot.frame(Bounds::new(point(px(0.), px(0.)), size(px(600.), px(200.))));
        let svg = svg(chart.clone(), &inks, 600., 200., "Chart", "g0");
        let threshold = plot.y_px(&frame, 250.);
        assert!(svg.contains(&format!(r#"y1="{t}" x2="{}" y2="{t}""#, n(frame.right), t = n(threshold))), "the rule stands at 250 on the plot's own scale");
        let first = (plot.x_px(&frame, chart.readings.xs()[0]), plot.y_px(&frame, 40.));
        assert!(svg.contains(&format!("M{},{}", n(first.0), n(first.1))), "the first line starts at the plot's first point");
    }

    #[test]
    fn stacked_bars_round_only_the_top_segment_and_a_legend_names_each_series() {
        let inks = inks();
        let chart = chart(
            r#"{"data":{"values":[{"d":"Mon","code":"500","n":2},{"d":"Mon","code":"502","n":5},{"d":"Tue","code":"500","n":1},{"d":"Tue","code":"502","n":3}]},
            "mark":"bar","encoding":{"x":{"field":"d","type":"nominal"},"y":{"field":"n","type":"quantitative"},"color":{"field":"code","type":"nominal"}}}"#,
        );
        let svg = svg(chart.clone(), &inks, 400., 200., "Chart", "g1");
        assert_eq!(svg.matches(&format!(r#"Z" fill="{}"/>"#, css(inks.series[0]))).count(), 2, "a bottom segment for each day");
        assert!(svg.contains(">Mon</text>") && svg.contains(">Tue</text>"), "the bands are labelled");
        let legend = legend(&chart, &inks).expect("two series carry a legend");
        assert!(legend.contains(">500</span>") && legend.contains(">502</span>") && legend.contains(&css(inks.series[1])), "{legend}");
    }

    #[test]
    fn a_donut_writes_its_slices_and_whole_and_its_shares_beside_it() {
        let inks = inks();
        let chart = chart(r#"{"data":{"values":[{"v":"Stable","rps":900},{"v":"Canary","rps":100}]},"mark":{"type":"arc","innerRadius":40},"encoding":{"theta":{"field":"rps","type":"quantitative","title":"Requests (req/s)"},"color":{"field":"v","type":"nominal"}}}"#);
        let svg = svg(chart.clone(), &inks, 160., 160., "Chart: Split", "g2");
        assert_eq!(svg.matches("<path d=\"M").count(), 2, "a slice for each version");
        assert!(svg.contains(">1,000</text>") && svg.contains(">req/s</text>"), "{svg}");
        assert!(legend(&chart, &inks).is_none(), "a donut names its slices beside it");
        let shares = shares(&chart, &inks).expect("a donut has shares");
        assert!(shares.contains(">90%</b>") && shares.contains(">10%</b>"), "{shares}");
    }

    #[test]
    fn words_in_a_chart_are_escaped() {
        let chart = chart(r#"{"data":{"values":[{"x":"a<b","y":1},{"x":"c&d","y":2}]},"mark":"bar","encoding":{"x":{"field":"x","type":"nominal"},"y":{"field":"y","type":"quantitative"}}}"#);
        let svg = svg(chart, &inks(), 300., 150., "A \"quoted\" chart", "g3");
        assert!(svg.contains(">a&lt;b</text>") && svg.contains(">c&amp;d</text>") && svg.contains("A &quot;quoted&quot; chart"), "{svg}");
        assert!(!svg.contains("a<b"));
    }
}
