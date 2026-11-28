//! A compiled chart painted with gpui-component's plot primitives: the band and linear scales' sums,
//! the grid, the axis labels, lines, areas, bars, the pie's arcs, and the hover's crosshair, dots
//! and readout. One `Plot` paints every layer, since the library's charts each paint one series.

use std::rc::Rc;

use gpui::{
    div, point, prelude::*, px, quad, size, AnyElement, App, BorderStyle, Bounds, ContentMask, Corners, ElementId, FontWeight, Hsla, PathBuilder, PathStyle, Pixels, Point,
    SharedString, StrokeOptions, TextAlign, Window,
};
use gpui_component::plot::label::{measure_text_width, PlotLabel, Text};
use gpui_component::plot::scale::{Scale, ScaleBand, ScaleLinear};
use gpui_component::plot::shape::{Arc, Area, Bar, Line, Pie};
use gpui_component::plot::tooltip::{Dot, Tooltip};
use gpui_component::plot::{hover_progress, Grid, IntoPlot, PathCaches, Plot, TooltipState};
use lyon::tessellation::{LineCap, LineJoin};

use super::compile::{Chart, Curve, Key, Layer, XKind};
use super::ticks;
use crate::look::{token, Look};

/// The colours and sizes a chart is drawn in, read from the look once per frame.
#[derive(Clone)]
pub struct Inks {
    pub series: [Hsla; 6],
    /// A seventh series and past: never a seventh hue.
    pub other: Hsla,
    pub tick: Hsla,
    pub label: Hsla,
    pub strong: Hsla,
    pub grid: Hsla,
    pub axis: Hsla,
    pub rule: Hsla,
    pub marker: Hsla,
    pub surface: Hsla,
    pub pill: Hsla,
    pub font: Pixels,
    pub big: Pixels,
}

impl Inks {
    pub fn of(look: &Look) -> Inks {
        Inks {
            series: [token::CHART_1, token::CHART_2, token::CHART_3, token::CHART_4, token::CHART_5, token::CHART_6].map(|one| look.color(one)),
            other: look.color(token::FG_SUBTLE),
            tick: look.color(token::FG_SUBTLE),
            label: look.color(token::FG_MUTED),
            strong: look.color(token::FG_DEFAULT),
            grid: look.color(token::BORDER_SUBTLE),
            axis: look.color(token::BORDER_DEFAULT),
            rule: look.color(token::FG_MUTED),
            marker: look.color(token::BORDER_STRONG),
            surface: look.color(token::BG_RAISED),
            pill: look.color(token::BG_OVERLAY),
            font: look.length(token::TEXT_2XS),
            big: look.length(token::TEXT_XL),
        }
    }

    pub fn of_series(&self, slot: usize) -> Hsla {
        self.series.get(slot).copied().unwrap_or(self.other)
    }
}

/// The gap under the x labels, the plot's top, and its right edge, in points.
const BELOW: f32 = 22.;
const ABOVE: f32 = 8.;
const RIGHT: f32 = 8.;
/// The space between the y labels and the plot.
const GUTTER_GAP: f32 = 10.;
/// The thickest a bar is drawn, and the gap left between two stacked segments or two bars.
const BAR_MOST: f32 = 24.;
const GAP: f32 = 2.;
/// A dot's diameter, and the ring of surface around it.
const DOT: f32 = 8.;
/// The pointer is on a scatter's dot within this many points of it.
const REACH: f32 = 24.;

#[derive(IntoPlot)]
pub struct GraphPlot {
    id: ElementId,
    chart: Rc<Chart>,
    inks: Inks,
    /// The y labels' widest, measured as the plot is laid out.
    gutter: f32,
}

/// Where the plot's area stands inside its bounds, and the sums that place a value in it.
struct Frame {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    /// How far inside each end the first and last x stand: half a bar's step where there are bars.
    inset: f32,
}

impl GraphPlot {
    pub fn new(id: impl Into<ElementId>, chart: Rc<Chart>, inks: Inks) -> Self {
        GraphPlot { id: id.into(), chart, inks, gutter: 0. }
    }

    fn has_bars(&self) -> bool {
        self.chart.layers.iter().any(|layer| matches!(layer, Layer::Bars { .. }))
    }

    fn frame(&self, bounds: Bounds<Pixels>) -> Frame {
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let left = if self.chart.y.shown { self.gutter + GUTTER_GAP } else { 0. };
        let font = f32::from(self.inks.font);
        let below = if self.x_caption().is_some() { BELOW + font + 6. } else { BELOW };
        let bottom = height - if self.chart.x.shown { below } else { 2. };
        let above = if self.y_caption().is_some() { ABOVE + font + 8. } else { ABOVE };
        let right = width - RIGHT;
        let inset = match (self.has_bars(), self.chart.x.kind) {
            (true, XKind::Time | XKind::Number) => {
                let xs: Vec<f64> = self.bar_xs();
                let span = (self.chart.x.high - self.chart.x.low).max(1e-9);
                let step = xs.windows(2).map(|pair| pair[1] - pair[0]).fold(f64::INFINITY, f64::min);
                let count = xs.len().max(1) as f32;
                if step.is_finite() {
                    let across = (right - left).max(1.);
                    // Solve inset so the first and last bars' halves stand inside the plot.
                    let per = (step / span) as f32;
                    (across * per / (2. * (1. + per))).min(across / (2. * count))
                } else {
                    BAR_MOST
                }
            }
            _ => 0.,
        };
        Frame { left, top: above, right, bottom, inset }
    }

    fn bar_xs(&self) -> Vec<f64> {
        self.chart.layers.iter().filter_map(|layer| if let Layer::Bars { bars } = layer { Some(bars.iter().map(|(x, _)| *x).collect::<Vec<_>>()) } else { None }).flatten().collect()
    }

    fn x_scale(&self, frame: &Frame) -> ScaleLinear<f64> {
        let (low, high) = (self.chart.x.low, if self.chart.x.high > self.chart.x.low { self.chart.x.high } else { self.chart.x.low + 1. });
        ScaleLinear::new([low, high], [frame.left + frame.inset, frame.right - frame.inset])
    }

    fn bands(&self, frame: &Frame) -> ScaleBand<usize> {
        ScaleBand::new(0..self.chart.x.bands.len().max(1), [frame.left, frame.right]).padding_inner(0.3).padding_outer(0.15)
    }

    /// Where an x stands across the plot, as a point's middle.
    fn x_px(&self, frame: &Frame, x: f64) -> f32 {
        match self.chart.x.kind {
            XKind::Band => {
                let bands = self.bands(frame);
                bands.tick(&(x as usize)).map_or(frame.left, |at| at + bands.band_width() / 2.)
            }
            _ => self.x_scale(frame).tick(&x).unwrap_or(frame.left),
        }
    }

    fn y_scale(&self, frame: &Frame) -> ScaleLinear<f64> {
        ScaleLinear::new([self.chart.y.low, self.chart.y.high], [frame.bottom, frame.top])
    }

    fn y_px(&self, frame: &Frame, y: f64) -> f32 {
        self.y_scale(frame).tick(&y).unwrap_or(frame.bottom)
    }

    fn bar_width(&self, frame: &Frame) -> f32 {
        match self.chart.x.kind {
            XKind::Band => self.bands(frame).band_width().min(BAR_MOST),
            _ => {
                let xs = self.bar_xs();
                let step = xs.windows(2).map(|pair| self.x_px(frame, pair[1]) - self.x_px(frame, pair[0])).fold(f32::INFINITY, f32::min);
                if step.is_finite() { (step - GAP).clamp(2., BAR_MOST) } else { BAR_MOST }
            }
        }
    }

    fn y_label(&self, value: f64) -> String {
        let y = &self.chart.y;
        if y.percent {
            let shown = if y.high <= 1.0001 { value * 100. } else { value };
            return format!("{}%", ticks::grouped(shown));
        }
        ticks::axis_number(value, y.high.abs().max(y.low.abs()))
    }

    fn x_label(&self, x: f64) -> String {
        match self.chart.x.kind {
            XKind::Time => ticks::clock(x),
            XKind::Number => ticks::axis_number(x, self.chart.x.high.abs()),
            XKind::Band => self.chart.x.bands.get(x as usize).cloned().unwrap_or_default(),
        }
    }

    /// The words over the y labels: the unit where the title carries one, the whole title on a scatter,
    /// where both axes are measures and neither reads without its name.
    fn y_caption(&self) -> Option<String> {
        let y = &self.chart.y;
        if !y.shown {
            return None;
        }
        match (&y.title, &y.unit) {
            (Some(title), Some(unit)) if self.chart.scatter => Some(format!("{title} ({unit})")),
            (_, Some(unit)) => Some(unit.clone()),
            (Some(title), None) if self.chart.scatter || self.chart.legend() => Some(title.clone()),
            _ => None,
        }
    }

    /// The words under a number x axis, its title and unit. A time axis needs none.
    fn x_caption(&self) -> Option<String> {
        let x = &self.chart.x;
        if !x.shown || x.kind != XKind::Number {
            return None;
        }
        match (&x.title, &x.unit) {
            (Some(title), Some(unit)) => Some(format!("{title} ({unit})")),
            (Some(title), None) => Some(title.clone()),
            (None, unit) => unit.clone(),
        }
    }

    /// Whether a line or area passes through the box from `(x0, y0)` to `(x1, y1)`, in the plot's points.
    fn crosses(&self, frame: &Frame, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) -> bool {
        let inside = |x: f64, y: f64| {
            let (x, y) = (self.x_px(frame, x), self.y_px(frame, y));
            x >= x0 && x <= x1 && y >= y0 - 4. && y <= y1 + 4.
        };
        self.chart.layers.iter().any(|layer| match layer {
            Layer::Line { points, .. } => points.iter().any(|(x, y)| inside(*x, *y)),
            Layer::Area { points, .. } => points.iter().any(|(x, top, _)| inside(*x, *top)),
            Layer::Points { points } => points.iter().any(|(x, y, _)| inside(*x, *y)),
            _ => false,
        })
    }

    fn text(&self, words: String, at: Point<Pixels>, ink: Hsla, align: TextAlign) -> Text {
        Text::new(words, at, ink).font_size(self.inks.font).align(align)
    }

    fn paint_axes(&self, bounds: Bounds<Pixels>, frame: &Frame, window: &mut Window, cx: &mut App) {
        let chart = &self.chart;
        let font = f32::from(self.inks.font);
        let area = |frame: &Frame| Bounds::new(bounds.origin + point(px(frame.left), px(0.)), size(px(frame.right - frame.left), bounds.size.height));
        if chart.y.grid {
            let lines: Vec<Pixels> = chart.y.ticks.iter().map(|tick| self.y_px(frame, *tick)).filter(|at| (*at - frame.bottom).abs() > 0.5).map(px).collect();
            Grid::new().y(lines).stroke(self.inks.grid).paint(&area(frame), window);
        }
        // The baseline, where zero or the lowest tick stands.
        let base = self.y_px(frame, chart.y.low.max(0.).min(chart.y.high));
        window.paint_quad(gpui::fill(Bounds::new(bounds.origin + point(px(frame.left), px(base - 0.5)), size(px(frame.right - frame.left), px(1.))), self.inks.axis));
        let mut texts = Vec::new();
        if let Some(caption) = self.y_caption() {
            texts.push(self.text(caption, point(px(0.), px(0.)), self.inks.tick, TextAlign::Left));
        }
        if let Some(caption) = self.x_caption() {
            texts.push(self.text(caption, point(px(frame.right), px(frame.bottom + 7. + font + 6.)), self.inks.tick, TextAlign::Right));
        }
        if chart.y.shown {
            for tick in &chart.y.ticks {
                let at = self.y_px(frame, *tick);
                texts.push(self.text(self.y_label(*tick), point(px(frame.left - GUTTER_GAP), px(at - font / 2. - 1.)), self.inks.tick, TextAlign::Right));
            }
        }
        if chart.x.shown {
            let across = frame.right - frame.left;
            let y = frame.bottom + 7.;
            match chart.x.kind {
                XKind::Band => {
                    for (at, _) in chart.x.bands.iter().enumerate() {
                        texts.push(self.text(self.x_label(at as f64), point(px(self.x_px(frame, at as f64)), px(y)), self.inks.tick, TextAlign::Center));
                    }
                }
                kind => {
                    let count = ((across / 52.).floor() as usize).max(2);
                    let marks = match kind {
                        XKind::Time => ticks::time_ticks(chart.x.low, chart.x.high, count),
                        _ => ticks::value_ticks(chart.x.low, chart.x.high, count).2.into_iter().filter(|tick| *tick >= chart.x.low && *tick <= chart.x.high).collect(),
                    };
                    for tick in marks {
                        let words = self.x_label(tick);
                        let at = self.x_px(frame, tick);
                        let half = measure_text_width(&SharedString::from(words.clone()), self.inks.font, window) / 2.;
                        // A label is kept whole inside the plot's width rather than cut at its edge.
                        let at = at.clamp(frame.left + half, frame.right - half);
                        texts.push(self.text(words, point(px(at), px(y)), self.inks.tick, TextAlign::Center));
                    }
                }
            }
        }
        PlotLabel::new(texts).paint(&bounds, window, cx);
    }

    fn curve(curve: Curve) -> gpui_component::plot::Curve {
        match curve {
            Curve::Linear => gpui_component::plot::Curve::Linear,
            Curve::Smooth => gpui_component::plot::Curve::Natural,
            Curve::Step => gpui_component::plot::Curve::StepAfter,
        }
    }

    fn paint_marks(&self, bounds: Bounds<Pixels>, frame: &Frame, window: &mut Window, cx: &mut App) {
        let plot_area = Bounds::new(bounds.origin + point(px(frame.left - 2.), px(frame.top - 4.)), size(px(frame.right - frame.left + 4.), px(frame.bottom - frame.top + 6.)));
        let caches = PathCaches::for_paint("graph-marks", window, cx);
        let mut slot = 0;
        let mut texts = Vec::new();
        for layer in &self.chart.layers {
            match layer {
                Layer::Area { series, points, curve, stacked } => {
                    let ink = self.inks.of_series(self.chart.series[*series].slot);
                    let placed: Vec<(f32, f32, f32)> = points.iter().map(|(x, top, base)| (self.x_px(frame, *x), self.y_px(frame, *top), self.y_px(frame, *base))).collect();
                    window.with_content_mask(Some(ContentMask { bounds: plot_area }), |window| {
                        if *stacked {
                            // A stacked fill runs along its top and back along its base, in its own hue.
                            let mut fill = PathBuilder::fill();
                            if let Some((x, y, _)) = placed.first() {
                                fill.move_to(bounds.origin + point(px(*x), px(*y)));
                            }
                            for (x, y, _) in placed.iter().skip(1) {
                                fill.line_to(bounds.origin + point(px(*x), px(*y)));
                            }
                            for (x, _, base) in placed.iter().rev() {
                                fill.line_to(bounds.origin + point(px(*x), px(*base)));
                            }
                            fill.close();
                            if let Ok(path) = fill.build() {
                                window.paint_path(path, ink.opacity(0.32));
                            }
                        } else {
                            // A wash of the hue under the line, fading to nothing at the baseline.
                            let fade = gpui::linear_gradient(180., gpui::linear_color_stop(ink.opacity(0.30), 0.), gpui::linear_color_stop(ink.opacity(0.02), 1.));
                            let base = placed.first().map_or(frame.bottom, |(_, _, base)| *base);
                            let area = Area::new().data(placed.clone()).x(|(x, _, _)| Some(*x)).y0(base).y1(|(_, y, _)| Some(*y)).fill(fade).stroke(gpui::transparent_black()).curve(Self::curve(*curve));
                            caches.update(cx, |caches, _| {
                                let (fill, line) = caches.slot_pair(slot);
                                area.paint_cached(&bounds, fill, line, window);
                            });
                            slot += 1;
                        }
                        let line = Line::new().data(placed.clone()).x(|(x, _, _)| Some(*x)).y(|(_, y, _)| Some(*y)).stroke(ink).stroke_width(2.).curve(Self::curve(*curve));
                        caches.update(cx, |caches, _| line.paint_cached(&bounds, caches.slot(100 + slot), window));
                        slot += 1;
                    });
                }
                Layer::Bars { bars } => {
                    let width = self.bar_width(frame);
                    let radius = px((width / 2.).min(4.));
                    for (x, segments) in bars {
                        let left = self.x_px(frame, *x) - width / 2.;
                        let top = segments.iter().map(|(_, _, to)| *to).fold(f64::NEG_INFINITY, f64::max);
                        for (series, from, to) in segments {
                            let (base, end) = (self.y_px(frame, *from), self.y_px(frame, *to));
                            // A segment leaves the surface's gap above the one under it; the top one rounds its end.
                            let base = if *from > self.chart.y.low.max(0.) { base - GAP } else { base };
                            if base - end < 0.5 {
                                continue;
                            }
                            let corners = if *to >= top { Corners { top_left: radius, top_right: radius, bottom_left: px(0.), bottom_right: px(0.) } } else { Corners::all(px(0.)) };
                            let ink = self.inks.of_series(self.chart.series[*series].slot);
                            Bar::new().data([(left, base, end)]).cross(|(left, _, _)| Some(*left)).band_width(width).base(|(_, base, _)| *base).value(|(_, _, end)| Some(*end)).fill(move |_, _, _| ink).corner_radii(corners).paint(&bounds, window, cx);
                        }
                    }
                }
                Layer::Line { series, points, curve, dash, dots } => {
                    let ink = self.inks.of_series(self.chart.series[*series].slot);
                    let placed: Vec<(f32, f32)> = points.iter().map(|(x, y)| (self.x_px(frame, *x), self.y_px(frame, *y))).collect();
                    window.with_content_mask(Some(ContentMask { bounds: plot_area }), |window| match dash {
                        Some(dash) => paint_dashed(&placed, bounds.origin, ink, dash, window),
                        None => {
                            let line = Line::new().data(placed.clone()).x(|(x, _)| Some(*x)).y(|(_, y)| Some(*y)).stroke(ink).stroke_width(2.).curve(Self::curve(*curve));
                            caches.update(cx, |caches, _| line.paint_cached(&bounds, caches.slot(100 + slot), window));
                            slot += 1;
                        }
                    });
                    if *dots {
                        for (x, y) in &placed {
                            self.dot(bounds.origin + point(px(*x), px(*y)), ink, window);
                        }
                    }
                }
                Layer::Points { points } => {
                    for (x, y, series) in points {
                        let ink = self.inks.of_series(self.chart.series[*series].slot);
                        self.dot(bounds.origin + point(px(self.x_px(frame, *x)), px(self.y_px(frame, *y))), ink.opacity(0.9), window);
                    }
                }
                Layer::Rule { across, at, dash } => {
                    let dash = dash.clone().unwrap_or_default();
                    if *across {
                        let y = self.y_px(frame, *at);
                        dashes(bounds.origin + point(px(frame.left), px(y)), frame.right - frame.left, true, &dash, self.inks.rule, window);
                    } else {
                        let x = self.x_px(frame, *at);
                        dashes(bounds.origin + point(px(x), px(frame.top)), frame.bottom - frame.top, false, &dash, self.inks.marker, window);
                        // A marker down the plot carries a dot at its top, where its words begin.
                        window.paint_quad(gpui::fill(Bounds::centered_at(bounds.origin + point(px(x), px(frame.top)), size(px(5.), px(5.))), self.inks.marker).corner_radii(Corners::all(px(2.5))));
                    }
                }
                Layer::Text { x, y, text, align_left } => texts.push((*x, *y, text.clone(), *align_left)),
            }
        }
        // Words last, on a pill of the surface, so a line under them does not run through them.
        let font = f32::from(self.inks.font);
        let mut labels = Vec::new();
        for (x, y, words, align_left) in texts {
            let width = measure_text_width(&SharedString::from(words.clone()), self.inks.font, window);
            let (pad_x, pad_y) = (6., 2.);
            let (at_x, at_y) = match (x, y) {
                // A marker's words stand at the top, beside its line.
                (Some(x), None) => (self.x_px(frame, x) + 6., frame.top + 2.),
                // A threshold's words stand over its line at the right, or at the left where the data runs through them there.
                (None, Some(y)) => {
                    let top = self.y_px(frame, y) - font - pad_y * 2. - 3.;
                    let wide = width + pad_x * 2.;
                    let right = frame.right - wide - 2.;
                    let left = frame.left + 4.;
                    let at_x = if self.crosses(frame, (right, top), (frame.right, top + font + pad_y * 2.)) && !self.crosses(frame, (left, top), (left + wide, top + font + pad_y * 2.)) { left } else { right };
                    (at_x, top)
                }
                (Some(x), Some(y)) => {
                    let x = self.x_px(frame, x);
                    (if align_left { x + 6. } else { x - width - pad_x * 2. - 6. }, self.y_px(frame, y) - font / 2. - pad_y)
                }
                (None, None) => (frame.right - width - pad_x * 2., frame.top),
            };
            let pill = Bounds::new(bounds.origin + point(px(at_x), px(at_y)), size(px(width + pad_x * 2.), px(font + pad_y * 2. + 2.)));
            window.paint_quad(quad(pill, px(4.), self.inks.pill, px(1.), self.inks.axis, BorderStyle::default()));
            labels.push(Text::new(words, point(px(at_x + pad_x), px(at_y + pad_y + 1.)), self.inks.label).font_size(self.inks.font).font_weight(FontWeight::MEDIUM));
        }
        PlotLabel::new(labels).paint(&bounds, window, cx);
    }

    /// A dot in its hue with a ring of the surface, so it stays clear where it crosses a line.
    fn dot(&self, at: Point<Pixels>, ink: Hsla, window: &mut Window) {
        window.paint_quad(quad(Bounds::centered_at(at, size(px(DOT + 3.), px(DOT + 3.))), px((DOT + 3.) / 2.), ink, px(1.5), self.inks.surface, BorderStyle::default()));
    }

    fn paint_arcs(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let Some(arcs) = &self.chart.arcs else { return };
        let outer = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2. - 2.).max(1.);
        let inner = arcs.hole.map_or(outer * 0.66, |hole| hole.min(outer - 6.));
        let slices: Vec<(usize, f32)> = arcs.slices.iter().map(|(series, value)| (*series, *value as f32)).collect();
        let pie = Pie::new().value(|(_, value): &(usize, f32)| Some(*value)).pad_angle(0.025);
        let shape = Arc::new().inner_radius(inner).outer_radius(outer);
        let caches = PathCaches::for_paint("graph-arcs", window, cx);
        for (at, arc) in pie.arcs(&slices).iter().enumerate() {
            let ink = self.inks.of_series(self.chart.series[arc.data.0].slot);
            caches.update(cx, |caches, _| shape.paint_cached(arc, ink, &bounds, caches.slot(at), window));
        }
        // The whole in the hole, its unit under it.
        let whole: f64 = arcs.slices.iter().map(|(_, value)| *value).sum();
        let middle = point(bounds.size.width / 2., bounds.size.height / 2.);
        let big = f32::from(self.inks.big);
        let mut texts = vec![Text::new(ticks::axis_number(whole, whole), point(middle.x, middle.y - px(big * 0.72)), self.inks.strong).font_size(self.inks.big).align(TextAlign::Center)];
        if let Some(unit) = &arcs.unit {
            texts.push(Text::new(unit.clone(), point(middle.x, middle.y + px(big * 0.36)), self.inks.label).font_size(self.inks.font).align(TextAlign::Center));
        }
        PlotLabel::new(texts).paint(&bounds, window, cx);
    }

    /// The readout's rows at one x: each series with a value there, in its colour order.
    fn rows_at(&self, x: f64) -> Vec<(Hsla, Key, String, String)> {
        let chart = &self.chart;
        chart
            .series
            .iter()
            .enumerate()
            .filter_map(|(at, series)| {
                let value = chart.readings.at(at, x)?;
                let shown = if chart.y.percent && chart.y.high <= 1.0001 { format!("{}%", ticks::grouped(value * 100.)) } else { ticks::with_unit(value, chart.y.unit.as_deref()) };
                Some((self.inks.of_series(series.slot), series.key, shown, series.name.clone()))
            })
            .collect()
    }
}

/// A straight run of dashes, as 1-point quads, across or down from `from`.
fn dashes(from: Point<Pixels>, length: f32, across: bool, dash: &[f32], ink: Hsla, window: &mut Window) {
    let pattern: Vec<f32> = if dash.is_empty() { vec![length] } else { dash.iter().chain(dash.iter()).copied().take(if dash.len() % 2 == 1 { dash.len() * 2 } else { dash.len() }).collect() };
    let (mut at, mut ix) = (0., 0);
    while at < length {
        let run = pattern[ix % pattern.len()].max(0.5);
        if ix % 2 == 0 {
            let end = (at + run).min(length);
            let piece = if across {
                Bounds::new(from + point(px(at), px(-0.5)), size(px(end - at), px(1.)))
            } else {
                Bounds::new(from + point(px(-0.5), px(at)), size(px(1.), px(end - at)))
            };
            window.paint_quad(gpui::fill(piece, ink));
        }
        at += run;
        ix += 1;
    }
}

/// A line drawn with dashes, its points joined straight with round ends.
fn paint_dashed(points: &[(f32, f32)], origin: Point<Pixels>, ink: Hsla, dash: &[f32], window: &mut Window) {
    let round = StrokeOptions::default().with_line_width(2.).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
    let dashes: Vec<Pixels> = dash.iter().map(|one| px(*one)).collect();
    let mut path = PathBuilder::stroke(px(2.)).with_style(PathStyle::Stroke(round)).dash_array(&dashes);
    for (at, (x, y)) in points.iter().enumerate() {
        let here = origin + point(px(*x), px(*y));
        if at == 0 { path.move_to(here) } else { path.line_to(here) }
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, ink);
    }
}

impl Plot for GraphPlot {
    fn prepaint(&mut self, _bounds: Bounds<Pixels>, window: &mut Window, _cx: &mut App) -> Vec<AnyElement> {
        if self.chart.y.shown && self.chart.arcs.is_none() {
            self.gutter = self.chart.y.ticks.iter().map(|tick| measure_text_width(&SharedString::from(self.y_label(*tick)), self.inks.font, window)).fold(0., f32::max);
        }
        vec![]
    }

    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.chart.arcs.is_some() {
            self.paint_arcs(bounds, window, cx);
            return;
        }
        let frame = self.frame(bounds);
        self.paint_axes(bounds, &frame, window, cx);
        self.paint_marks(bounds, &frame, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn interactive(&self) -> bool {
        true
    }

    fn tooltip_state(&self, position: Point<Pixels>, bounds: Bounds<Pixels>, _cx: &App) -> Option<TooltipState> {
        let chart = &self.chart;
        let (x, y) = (f32::from(position.x), f32::from(position.y));
        if let Some(arcs) = &chart.arcs {
            let outer = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2. - 2.).max(1.);
            let inner = arcs.hole.map_or(outer * 0.66, |hole| hole.min(outer - 6.));
            let slices: Vec<(usize, f32)> = arcs.slices.iter().map(|(series, value)| (*series, *value as f32)).collect();
            let pie = Pie::new().value(|(_, value): &(usize, f32)| Some(*value)).pad_angle(0.025);
            let shape = Arc::new().inner_radius(inner).outer_radius(outer + 4.);
            let local = Bounds::new(Point::default(), bounds.size);
            let at = pie.arcs(&slices).iter().position(|arc| shape.contains(arc, point(x, y), &local))?;
            return Some(TooltipState::new(at, position, Vec::new()));
        }
        let frame = self.frame(bounds);
        if x < frame.left - 4. || x > frame.right + 4. || y > frame.bottom || y < frame.top - 4. {
            return None;
        }
        if chart.scatter {
            let points: Vec<(f64, f64, usize)> = chart.layers.iter().filter_map(|layer| if let Layer::Points { points } = layer { Some(points.clone()) } else { None }).flatten().collect();
            let (at, distance) = points
                .iter()
                .enumerate()
                .map(|(at, (px_, py_, _))| (at, (self.x_px(&frame, *px_) - x).hypot(self.y_px(&frame, *py_) - y)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;
            if distance > REACH {
                return None;
            }
            let (px_, py_, _) = points[at];
            let dot = point(px(self.x_px(&frame, px_)), px(self.y_px(&frame, py_)));
            return Some(TooltipState::new(at, dot, vec![dot]));
        }
        let xs = chart.readings.xs();
        let (at, nearest) = xs.iter().enumerate().min_by(|a, b| (self.x_px(&frame, *a.1) - x).abs().partial_cmp(&(self.x_px(&frame, *b.1) - x).abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        let cross = self.x_px(&frame, *nearest);
        // A dot on each line and area at the x, where its value is drawn.
        let mut dots = Vec::new();
        for layer in &chart.layers {
            match layer {
                Layer::Line { points, .. } => {
                    if let Some((_, value)) = points.iter().find(|(px_, _)| px_ == nearest) {
                        dots.push(point(px(cross), px(self.y_px(&frame, *value))));
                    }
                }
                Layer::Area { points, .. } => {
                    if let Some((_, top, _)) = points.iter().find(|(px_, _, _)| px_ == nearest) {
                        dots.push(point(px(cross), px(self.y_px(&frame, *top))));
                    }
                }
                _ => {}
            }
        }
        Some(TooltipState::new(at, point(px(cross), position.y), dots))
    }

    fn tooltip(&self, state: &TooltipState, cursor: Point<Pixels>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let chart = &self.chart;
        let progress = hover_progress(window, cx);
        let inks = &self.inks;
        let (title, rows): (String, Vec<(Hsla, Key, String, String)>) = if let Some(arcs) = &chart.arcs {
            let (series, value) = *arcs.slices.get(state.index)?;
            let whole: f64 = arcs.slices.iter().map(|(_, value)| *value).sum();
            let share = if whole > 0. { value / whole * 100. } else { 0. };
            let one = &chart.series[series];
            (one.name.clone(), vec![(inks.of_series(one.slot), Key::Fill, format!("{}%", ticks::grouped((share * 10.).round() / 10.)), ticks::with_unit(value, arcs.unit.as_deref()))])
        } else if chart.scatter {
            let points: Vec<(f64, f64, usize)> = chart.layers.iter().filter_map(|layer| if let Layer::Points { points } = layer { Some(points.clone()) } else { None }).flatten().collect();
            let (x, y, series) = *points.get(state.index)?;
            let one = &chart.series[series];
            let x_words = match chart.x.kind {
                XKind::Time => ticks::clock(x),
                _ => ticks::with_unit(x, chart.x.unit.as_deref()),
            };
            let named = |title: &Option<String>, fallback: &str| title.clone().unwrap_or_else(|| fallback.to_string());
            (
                one.name.clone(),
                vec![
                    (inks.of_series(one.slot), Key::Dot, ticks::with_unit(y, chart.y.unit.as_deref()), named(&chart.y.title, "y")),
                    (gpui::transparent_black(), Key::Dot, x_words, named(&chart.x.title, "x")),
                ],
            )
        } else {
            let x = *chart.readings.xs().get(state.index)?;
            let title = match chart.x.kind {
                XKind::Time => ticks::clock(x),
                XKind::Number => ticks::with_unit(x, chart.x.unit.as_deref()),
                XKind::Band => chart.x.bands.get(x as usize).cloned().unwrap_or_default(),
            };
            (title, self.rows_at(x))
        };
        let frame = self.frame(bounds);
        let strong = inks.strong;
        let readout = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(div().text_color(inks.label).child(SharedString::from(title)))
            .children(rows.into_iter().map(|(ink, key, value, name)| {
                // Values lead and names follow; each row keys its series with a short stroke of its hue.
                let mark = match key {
                    Key::Line => div().flex_none().w(px(12.)).h(px(2.)).rounded(px(1.)).bg(ink),
                    Key::Fill => div().flex_none().size(px(8.)).rounded(px(2.)).bg(ink),
                    Key::Dot => div().flex_none().size(px(8.)).rounded_full().bg(ink),
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().flex_none().w(px(12.)).flex().justify_center().child(mark))
                    .child(div().flex_none().text_color(strong).font_weight(FontWeight::SEMIBOLD).child(SharedString::from(value)))
                    .child(div().flex_none().text_color(inks.label).child(SharedString::from(name)))
            }));
        let dots: Vec<Dot> = state
            .dots
            .iter()
            .enumerate()
            .map(|(at, dot)| {
                let ink = self.dot_ink(at);
                Dot::new(*dot).size(px(DOT + 2.)).halo(px(20.)).stroke(inks.surface).fill(ink)
            })
            .collect();
        let tooltip = if chart.arcs.is_some() {
            // A donut is too small to hold the readout; it hugs the pointer instead.
            Tooltip::new(cursor, bounds.size).glide(false).gap(px(14.)).dots(dots).child(readout)
        } else {
            // The readout stands inside the plot's top edge, beside the crosshair on the side with more room, so it covers
            // neither the axis labels, the legend nor the dots it reads. A see-through surface lets the lines show under it.
            let cross = f32::from(state.cross_line.x);
            let beside = 10.;
            let away = div()
                .absolute()
                .top(px(frame.top + 4.))
                .map(|box_| if frame.right - cross >= cross - frame.left { box_.left(px(cross + beside)) } else { box_.right(bounds.size.width - px(cross - beside)) })
                .px(px(8.))
                .py(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(inks.axis)
                .bg(inks.pill.opacity(0.86))
                .shadow_md()
                .child(readout);
            Tooltip::new(cursor, bounds.size).glide(false).appearance(false).dots(dots).child(away)
        };
        let under = match (&chart.arcs, chart.scatter, self.has_bars()) {
            (Some(_), _, _) | (_, true, _) => None,
            // A bar's column lifts under the pointer; a line's x carries a hairline down the plot.
            (_, _, true) => {
                let width = self.bar_width(&frame) + 8.;
                Some(div().absolute().left(state.cross_line.x - px(width / 2.)).top(px(frame.top)).w(px(width)).h(px(frame.bottom - frame.top)).rounded(px(4.)).bg(inks.strong.opacity(0.07)).opacity(progress))
            }
            _ => Some(div().absolute().left(state.cross_line.x - px(0.5)).top(px(frame.top)).w(px(1.)).h(px(frame.bottom - frame.top)).bg(inks.label.opacity(0.6)).opacity(progress)),
        };
        Some(div().absolute().top_0().left_0().size_full().children(under).child(tooltip).into_any_element())
    }
}

impl GraphPlot {
    /// The hue of the `at`-th hover dot: the lines and areas in the order `tooltip_state` met them.
    fn dot_ink(&self, at: usize) -> Hsla {
        self.chart
            .layers
            .iter()
            .filter_map(|layer| match layer {
                Layer::Line { series, .. } | Layer::Area { series, .. } => Some(*series),
                _ => None,
            })
            .nth(at)
            .or_else(|| match self.chart.layers.iter().find(|layer| matches!(layer, Layer::Points { .. })) {
                Some(Layer::Points { points }) => points.first().map(|(_, _, series)| *series),
                _ => None,
            })
            .map_or(self.inks.other, |series| self.inks.of_series(self.chart.series[series].slot))
    }
}
