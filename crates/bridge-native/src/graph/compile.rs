//! A spec and its rows worked into what is drawn: the axes, the series in their colour order, and
//! each layer's marks in the data's own terms. Nothing here knows a pixel; `plot.rs` lays it out.

use std::collections::BTreeMap;

use ipc::graph::{Aggregate, Channel, Encoding, FieldType, Interpolate, MarkDef, MarkKind, Row, Setting, Spec, StackOffset};
use serde_json::Value;

use super::ticks;

/// What the x axis measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XKind {
    /// Milliseconds since the epoch, written in this Mac's clock.
    Time,
    Number,
    /// Named places in order; a mark's x is its place's index.
    Band,
}

#[derive(Clone, Debug)]
pub struct XAxis {
    pub kind: XKind,
    pub title: Option<String>,
    pub unit: Option<String>,
    pub low: f64,
    pub high: f64,
    pub bands: Vec<String>,
    pub shown: bool,
}

#[derive(Clone, Debug)]
pub struct YAxis {
    pub title: Option<String>,
    pub unit: Option<String>,
    pub percent: bool,
    pub low: f64,
    pub high: f64,
    pub ticks: Vec<f64>,
    pub shown: bool,
    pub grid: bool,
}

/// How a series is keyed in the legend: as the mark it draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Line,
    Fill,
    Dot,
}

/// One series: what it is called, its place in the colour order, and how it is keyed.
#[derive(Clone, Debug)]
pub struct Series {
    pub name: String,
    pub slot: usize,
    pub key: Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    Linear,
    Smooth,
    Step,
}

/// A layer's marks, in the data's terms: x as the axis measures it, y as a value.
#[derive(Clone, Debug)]
pub enum Layer {
    Line { series: usize, points: Vec<(f64, f64)>, curve: Curve, dash: Option<Vec<f32>>, dots: bool },
    /// Each point's top and the base it fills down to; a stacked area's base is the one below it.
    Area { series: usize, points: Vec<(f64, f64, f64)>, curve: Curve, stacked: bool },
    /// Each bar's place and its segments from the base up, as `(series, from, to)`.
    Bars { bars: Vec<(f64, Vec<(usize, f64, f64)>)> },
    Points { points: Vec<(f64, f64, usize)> },
    /// A line across the plot at a value of y, or down it at a value of x.
    Rule { across: bool, at: f64, dash: Option<Vec<f32>> },
    /// Words at a place; where x or y is not given, the plot's right edge or its top.
    Text { x: Option<f64>, y: Option<f64>, text: String, align_left: bool },
}

/// A donut or a pie: each series' share of the whole.
#[derive(Clone, Debug)]
pub struct Arcs {
    pub slices: Vec<(usize, f64)>,
    pub hole: Option<f32>,
    pub unit: Option<String>,
}

/// Each series' raw value at each x, for the readout: the value a stack's segment stands for, not its top.
#[derive(Clone, Debug, Default)]
pub struct Readings {
    pub by_series: BTreeMap<usize, Vec<(f64, f64)>>,
}

impl Readings {
    pub fn at(&self, series: usize, x: f64) -> Option<f64> {
        let points = self.by_series.get(&series)?;
        let at = points.binary_search_by(|(px, _)| px.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal)).ok()?;
        points.get(at).map(|(_, y)| *y)
    }

    /// Every x a series has a value at, in order, each once.
    pub fn xs(&self) -> Vec<f64> {
        let mut all: Vec<f64> = self.by_series.values().flat_map(|points| points.iter().map(|(x, _)| *x)).collect();
        all.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all.dedup();
        all
    }
}

#[derive(Clone, Debug)]
pub struct Chart {
    pub x: XAxis,
    pub y: YAxis,
    pub series: Vec<Series>,
    pub layers: Vec<Layer>,
    pub arcs: Option<Arcs>,
    pub readings: Readings,
    /// Dots of two measures: the readout names the nearest dot, and both axes carry their titles.
    pub scatter: bool,
}

impl Chart {
    /// A legend is drawn for two series or more; one series is named by the title.
    pub fn legend(&self) -> bool {
        self.series.len() >= 2
    }
}

/// Why a spec that read cannot be drawn on these rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotDrawn {
    NoRows(String),
    NoX,
    NoY,
}

impl std::fmt::Display for NotDrawn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotDrawn::NoRows(feed) => write!(f, "No rows yet from {feed}"),
            NotDrawn::NoX => write!(f, "No layer names an x"),
            NotDrawn::NoY => write!(f, "No layer names a y"),
        }
    }
}

/// One layer with what it inherits filled in.
struct Leaf<'a> {
    mark: MarkDef,
    encoding: Encoding,
    rows: &'a [Row],
}

/// The rows of a live feed, by its name.
pub trait Feeds {
    fn rows(&self, name: &str) -> Option<&[Row]>;
}

impl Feeds for BTreeMap<String, Vec<Row>> {
    fn rows(&self, name: &str) -> Option<&[Row]> {
        self.get(name).map(Vec::as_slice)
    }
}

fn leaves<'a>(spec: &'a Spec, rows: &'a [Row], encoding: &Encoding, feeds: &'a dyn Feeds, out: &mut Vec<Leaf<'a>>) -> Result<(), NotDrawn> {
    let rows: &'a [Row] = match &spec.data {
        Some(data) => match (&data.values, &data.name) {
            (Some(values), _) => values.as_slice(),
            (None, Some(name)) => feeds.rows(name).ok_or_else(|| NotDrawn::NoRows(name.clone()))?,
            (None, None) => rows,
        },
        None => rows,
    };
    let encoding = spec.encoding.as_ref().map_or_else(|| encoding.clone(), |own| own.over(encoding));
    if let Some(mark) = &spec.mark {
        out.push(Leaf { mark: mark.def(), encoding: encoding.clone(), rows });
    }
    for layer in &spec.layer {
        leaves(layer, rows, &encoding, feeds, out)?;
    }
    Ok(())
}

pub fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        Value::Bool(yes) => Some(if *yes { 1. } else { 0. }),
        _ => None,
    }
}

/// A time as milliseconds since the epoch: a number is one already, a string is an RFC 3339 time or
/// one in this Mac's clock.
pub fn time(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => chrono::DateTime::parse_from_rfc3339(text)
            .map(|at| at.timestamp_millis() as f64)
            .ok()
            .or_else(|| {
                let naive = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S").or_else(|_| chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")).ok()?;
                naive.and_local_timezone(chrono::Local).single().map(|at| at.timestamp_millis() as f64)
            }),
        _ => None,
    }
}

pub fn label(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(_) => number(value).map(ticks::show_number).unwrap_or_default(),
        Value::Bool(yes) => yes.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The type a channel reads its field as: the one it names, else what its first value looks like.
fn kind_of(channel: &Channel, rows: &[Row]) -> FieldType {
    if let Some(kind) = channel.kind {
        return kind;
    }
    if channel.aggregate.is_some() {
        return FieldType::Quantitative;
    }
    let first = channel.field.as_ref().and_then(|field| rows.iter().find_map(|row| row.get(field)));
    match first.or(channel.datum.as_ref()) {
        Some(Value::Number(_)) => FieldType::Quantitative,
        Some(value @ Value::String(_)) if time(value).is_some() => FieldType::Temporal,
        _ => FieldType::Nominal,
    }
}

fn curve_of(mark: &MarkDef) -> Curve {
    match mark.interpolate {
        None | Some(Interpolate::Linear) => Curve::Linear,
        Some(Interpolate::Step | Interpolate::StepAfter) => Curve::Step,
        Some(_) => Curve::Smooth,
    }
}

/// A channel's title as it reads, and the unit in its brackets.
fn titled(channel: Option<&Channel>) -> (Option<String>, Option<String>) {
    let Some(channel) = channel else { return (None, None) };
    let title = channel.axis.on().and_then(|axis| axis.title.clone()).or_else(|| channel.title.clone()).or_else(|| channel.field.clone());
    let unit = title.as_deref().and_then(ticks::unit_of);
    (title.map(|title| ticks::bare_title(&title).to_string()), unit)
}

/// The series a mark's rows are split into, and the slot each name holds, first come first.
struct Slots {
    names: Vec<String>,
    keys: Vec<Key>,
    dashes: Vec<Option<Vec<f32>>>,
}

impl Slots {
    fn of(&mut self, name: &str, key: Key, dash: Option<Vec<f32>>) -> usize {
        if let Some(at) = self.names.iter().position(|one| one == name) {
            // A series drawn as a fill and a line keys as its fill.
            if key == Key::Fill {
                self.keys[at] = Key::Fill;
            }
            return at;
        }
        self.names.push(name.to_string());
        self.keys.push(key);
        self.dashes.push(dash);
        self.names.len() - 1
    }
}

/// The x of a row, in the axis's terms; a band's place is found or added in order.
fn x_of(channel: &Channel, row: Option<&Row>, kind: XKind, bands: &mut Vec<String>) -> Option<f64> {
    let value = match (&channel.field, row) {
        (Some(field), Some(row)) => row.get(field)?.clone(),
        _ => channel.datum.clone()?,
    };
    match kind {
        XKind::Time => time(&value),
        XKind::Number => number(&value),
        XKind::Band => {
            let name = label(&value);
            Some(match bands.iter().position(|one| *one == name) {
                Some(at) => at as f64,
                None => {
                    bands.push(name);
                    (bands.len() - 1) as f64
                }
            })
        }
    }
}

fn y_of(channel: &Channel, row: Option<&Row>) -> Option<f64> {
    match (&channel.field, row) {
        (Some(field), Some(row)) => number(row.get(field)?),
        _ => channel.datum.as_ref().and_then(number),
    }
}

fn aggregate(how: Aggregate, values: &[f64]) -> f64 {
    match how {
        Aggregate::Sum => values.iter().sum(),
        Aggregate::Mean => values.iter().sum::<f64>() / values.len().max(1) as f64,
        Aggregate::Count => values.len() as f64,
        Aggregate::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        Aggregate::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

/// A layer's rows as `(series, x, y)`, folded by its y's aggregate where it has one.
fn values(leaf: &Leaf, x: Option<&Channel>, kind: XKind, bands: &mut Vec<String>, series_of: &mut dyn FnMut(Option<&Row>) -> usize) -> Vec<(usize, f64, f64)> {
    let Some(y) = leaf.encoding.y.as_ref() else { return Vec::new() };
    let Some(x) = x else { return Vec::new() };
    let mut raw: Vec<(usize, f64, f64)> = Vec::new();
    for row in leaf.rows {
        let (Some(at), value) = (x_of(x, Some(row), kind, bands), y_of(y, Some(row))) else { continue };
        let value = match (value, y.aggregate) {
            (Some(value), _) => value,
            (None, Some(Aggregate::Count)) => 1.,
            (None, _) => continue,
        };
        raw.push((series_of(Some(row)), at, value));
    }
    let Some(how) = y.aggregate else { return raw };
    let mut grouped: BTreeMap<(usize, i64), (f64, Vec<f64>)> = BTreeMap::new();
    for (series, at, value) in raw {
        grouped.entry((series, (at * 1000.) as i64)).or_insert_with(|| (at, Vec::new())).1.push(value);
    }
    grouped.into_iter().map(|((series, _), (at, values))| (series, at, aggregate(how, &values))).collect()
}

/// Whether a mark stacks: bars and areas split by colour do, unless the spec says not to.
fn stacking(leaf: &Leaf) -> Option<StackOffset> {
    let y = leaf.encoding.y.as_ref()?;
    let coloured = leaf.encoding.color.as_ref().is_some_and(|color| color.field.is_some());
    match (&y.stack, leaf.mark.kind) {
        (Setting::On(StackOffset::None) | Setting::Off, _) => None,
        (Setting::On(offset), MarkKind::Bar | MarkKind::Area) => Some(*offset),
        (Setting::Unset, MarkKind::Bar | MarkKind::Area) if coloured => Some(StackOffset::Zero),
        _ => None,
    }
}

/// Stack `(series, x, y)` by x, each series on the ones before it in slot order: `(series, x, from, to)`.
fn stacked(points: &[(usize, f64, f64)], offset: StackOffset) -> Vec<(usize, f64, f64, f64)> {
    let mut by_x: BTreeMap<i64, Vec<(usize, f64, f64)>> = BTreeMap::new();
    for (series, x, y) in points {
        by_x.entry((*x * 1000.) as i64).or_default().push((*series, *x, *y));
    }
    let mut out = Vec::new();
    for (_, mut column) in by_x {
        column.sort_by_key(|(series, _, _)| *series);
        let whole: f64 = column.iter().map(|(_, _, y)| *y).sum();
        let mut base = 0.;
        for (series, x, y) in column {
            let y = if offset == StackOffset::Normalize && whole > 0. { y / whole } else { y };
            out.push((series, x, base, base + y));
            base += y;
        }
    }
    out
}

/// Work a spec out on its rows. `rows` are the data a spec without `data` of its own reads.
pub fn compile(spec: &Spec, rows: &[Row], feeds: &dyn Feeds) -> Result<Chart, NotDrawn> {
    let mut all = Vec::new();
    leaves(spec, rows, &Encoding::default(), feeds, &mut all)?;
    let mut slots = Slots { names: Vec::new(), keys: Vec::new(), dashes: Vec::new() };
    // A colour's sort or its scale's domain says the series' order before any row does.
    for leaf in &all {
        let Some(color) = leaf.encoding.color.as_ref() else { continue };
        for value in color.sort.iter().flatten().chain(color.scale.as_ref().and_then(|scale| scale.domain.as_ref()).into_iter().flatten()) {
            let key = match leaf.mark.kind {
                MarkKind::Line => Key::Line,
                MarkKind::Point => Key::Dot,
                _ => Key::Fill,
            };
            slots.of(&label(value), key, None);
        }
    }

    // The donut has no axes: its slices are its series.
    if let Some(leaf) = all.iter().find(|leaf| leaf.mark.kind == MarkKind::Arc) {
        let theta = leaf.encoding.theta.as_ref().ok_or(NotDrawn::NoY)?;
        let mut slices: Vec<(usize, f64)> = Vec::new();
        for row in leaf.rows {
            let Some(value) = y_of(theta, Some(row)) else { continue };
            let name = leaf.encoding.color.as_ref().and_then(|color| color.field.as_ref()).and_then(|field| row.get(field)).map(label).unwrap_or_else(|| "Share".to_string());
            let series = slots.of(&name, Key::Dot, None);
            match slices.iter_mut().find(|(one, _)| *one == series) {
                Some((_, sum)) => *sum += value,
                None => slices.push((series, value)),
            }
        }
        let (_, unit) = titled(Some(theta));
        let series = series_of(&slots);
        let empty_x = XAxis { kind: XKind::Band, title: None, unit: None, low: 0., high: 1., bands: Vec::new(), shown: false };
        let empty_y = YAxis { title: None, unit: None, percent: false, low: 0., high: 1., ticks: Vec::new(), shown: false, grid: false };
        return Ok(Chart { x: empty_x, y: empty_y, series, layers: Vec::new(), arcs: Some(Arcs { slices, hole: leaf.mark.inner_radius, unit }), readings: Readings::default(), scatter: false });
    }

    let x_channel = all.iter().find_map(|leaf| leaf.encoding.x.as_ref().filter(|x| x.field.is_some()).map(|x| (x.clone(), leaf.rows))).ok_or(NotDrawn::NoX)?;
    let y_channel = all.iter().find_map(|leaf| leaf.encoding.y.as_ref().filter(|y| y.field.is_some() || y.aggregate.is_some()).cloned()).ok_or(NotDrawn::NoY)?;
    let x_kind = match kind_of(&x_channel.0, x_channel.1) {
        FieldType::Temporal => XKind::Time,
        FieldType::Quantitative => XKind::Number,
        FieldType::Ordinal | FieldType::Nominal => XKind::Band,
    };
    let mut bands: Vec<String> = x_channel.0.sort.iter().flatten().map(label).collect();
    let mut layers = Vec::new();
    let mut readings = Readings::default();
    let mut x_seen: Vec<f64> = Vec::new();
    let mut y_seen: Vec<f64> = Vec::new();
    let mut scatter = false;
    let mut bars: BTreeMap<i64, (f64, Vec<(usize, f64, f64)>)> = BTreeMap::new();

    for leaf in &all {
        let kind = leaf.mark.kind;
        let x = leaf.encoding.x.as_ref();
        let y = leaf.encoding.y.as_ref();
        match kind {
            MarkKind::Rule => {
                // A constant across the plot, or one rule per row.
                let mut push = |across: bool, at: f64| {
                    layers.push(Layer::Rule { across, at, dash: leaf.mark.stroke_dash.clone() });
                    if across { y_seen.push(at) } else { x_seen.push(at) }
                };
                match (x, y) {
                    (_, Some(y)) if y.datum.is_some() && x.is_none_or(|x| x.field.is_none() && x.datum.is_none()) => {
                        if let Some(at) = y_of(y, None) {
                            push(true, at);
                        }
                    }
                    (Some(x), _) if x.datum.is_some() => {
                        if let Some(at) = x_of(x, None, x_kind, &mut bands) {
                            push(false, at);
                        }
                    }
                    (Some(x), _) if x.field.is_some() => {
                        for row in leaf.rows {
                            if let Some(at) = x_of(x, Some(row), x_kind, &mut bands) {
                                push(false, at);
                            }
                        }
                    }
                    (_, Some(y)) if y.field.is_some() => {
                        for row in leaf.rows {
                            if let Some(at) = y_of(y, Some(row)) {
                                push(true, at);
                            }
                        }
                    }
                    _ => {}
                }
            }
            MarkKind::Text => {
                let words = |row: Option<&Row>| -> Option<String> {
                    let channel = leaf.encoding.text.as_ref();
                    match (channel.and_then(|text| text.field.as_ref()), row) {
                        (Some(field), Some(row)) => row.get(field).map(label),
                        _ => channel.and_then(|text| text.value.as_ref().or(text.datum.as_ref())).map(label).or_else(|| leaf.mark.text.clone()),
                    }
                };
                let constant = x.is_none_or(|x| x.field.is_none()) && y.is_none_or(|y| y.field.is_none());
                let align_left = leaf.mark.align.as_deref() != Some("right");
                if constant {
                    let at_x = x.and_then(|x| x_of(x, None, x_kind, &mut bands));
                    let at_y = y.and_then(|y| y_of(y, None));
                    if let Some(text) = words(None) {
                        layers.push(Layer::Text { x: at_x, y: at_y, text, align_left: at_x.is_some() && align_left });
                    }
                } else {
                    for row in leaf.rows {
                        let at_x = x.and_then(|x| x_of(x, Some(row), x_kind, &mut bands));
                        let at_y = y.and_then(|y| y_of(y, Some(row)));
                        if let Some(text) = words(Some(row)) {
                            layers.push(Layer::Text { x: at_x, y: at_y, text, align_left });
                        }
                    }
                }
            }
            MarkKind::Line | MarkKind::Area | MarkKind::Bar | MarkKind::Point => {
                let key = match kind {
                    MarkKind::Line => Key::Line,
                    MarkKind::Point => Key::Dot,
                    _ => Key::Fill,
                };
                let color_field = leaf.encoding.color.as_ref().and_then(|color| color.field.clone());
                let dash_field = leaf.encoding.stroke_dash.as_ref().and_then(|dash| dash.field.clone());
                let (own_title, _) = titled(y);
                let lone = own_title.unwrap_or_else(|| "Value".to_string());
                let mark_dash = leaf.mark.stroke_dash.clone();
                let mut series_of = |row: Option<&Row>| -> usize {
                    let named = color_field.as_ref().and_then(|field| row?.get(field)).map(label);
                    let dashed = dash_field.as_ref().and_then(|field| row?.get(field)).map(label);
                    // A dash by a field marks every series but the first of its values with a dash.
                    let dash = match (&dashed, &mark_dash) {
                        (_, Some(dash)) => Some(dash.clone()),
                        (Some(word), None) if !word.is_empty() && dashed_first(leaf, word) => None,
                        (Some(_), None) => Some(vec![4., 3.]),
                        (None, None) => None,
                    };
                    slots.of(named.as_deref().unwrap_or(&lone), key, dash)
                };
                let points = values(leaf, x, x_kind, &mut bands, &mut series_of);
                for (series, at, value) in &points {
                    readings.by_series.entry(*series).or_default().push((*at, *value));
                    x_seen.push(*at);
                    let _ = series;
                    y_seen.push(*value);
                }
                let stack = stacking(leaf);
                match (kind, stack) {
                    (MarkKind::Bar, offset) => {
                        let stacks = match offset {
                            Some(offset) => stacked(&points, offset),
                            None => points.iter().map(|(series, x, y)| (*series, *x, 0., *y)).collect(),
                        };
                        for (series, at, from, to) in stacks {
                            y_seen.push(to);
                            bars.entry((at * 1000.) as i64).or_insert_with(|| (at, Vec::new())).1.push((series, from, to));
                        }
                    }
                    (MarkKind::Area, Some(offset)) => {
                        let stacks = stacked(&points, offset);
                        let mut by_series: BTreeMap<usize, Vec<(f64, f64, f64)>> = BTreeMap::new();
                        for (series, at, from, to) in stacks {
                            y_seen.push(to);
                            by_series.entry(series).or_default().push((at, to, from));
                        }
                        // The top of the stack is painted first, so each fill lies on the one under it.
                        for (series, mut points) in by_series.into_iter().rev() {
                            points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                            layers.push(Layer::Area { series, points, curve: curve_of(&leaf.mark), stacked: true });
                        }
                    }
                    (MarkKind::Area, None) => {
                        for (series, mut points) in split(&points) {
                            points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                            layers.push(Layer::Area { series, points: points.into_iter().map(|(x, y)| (x, y, 0.)).collect(), curve: curve_of(&leaf.mark), stacked: false });
                        }
                    }
                    (MarkKind::Line, _) => {
                        for (series, mut points) in split(&points) {
                            points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                            let dash = slots.dashes.get(series).cloned().flatten();
                            layers.push(Layer::Line { series, points, curve: curve_of(&leaf.mark), dash, dots: leaf.mark.point == Some(true) });
                        }
                    }
                    _ => {
                        scatter = true;
                        layers.push(Layer::Points { points: points.iter().map(|(series, x, y)| (*x, *y, *series)).collect() });
                    }
                }
            }
            MarkKind::Arc => {}
        }
    }
    if !bars.is_empty() {
        // Bars go under every line and rule, so they lead the layers.
        layers.insert(0, Layer::Bars { bars: bars.into_values().collect() });
    }
    for points in readings.by_series.values_mut() {
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        points.dedup_by(|a, b| a.0 == b.0);
    }

    let (x_title, x_unit) = titled(Some(&x_channel.0));
    let (low, high) = match x_kind {
        XKind::Band => (0., bands.len().max(1) as f64),
        _ => {
            let low = x_seen.iter().copied().fold(f64::INFINITY, f64::min);
            let high = x_seen.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let zero = x_kind == XKind::Number && x_channel.0.scale.as_ref().and_then(|scale| scale.zero) != Some(false);
            let (low, high) = if !low.is_finite() { (0., 1.) } else if zero { (low.min(0.), high) } else { (low, high) };
            // A number axis ends on a tick, as its y does; time ends where its rows do.
            if x_kind == XKind::Number { (ticks::value_ticks(low, high, 5).0, ticks::value_ticks(low, high, 5).1) } else { (low, high) }
        }
    };
    let x = XAxis { kind: x_kind, title: x_title, unit: x_unit, low, high, bands, shown: !x_channel.0.axis.is_off() };

    let (y_title, y_unit) = titled(Some(&y_channel));
    let percent = y_channel.format.as_deref().is_some_and(|format| format.ends_with('%')) || y_channel.stack == Setting::On(StackOffset::Normalize);
    let zero = y_channel.scale.as_ref().and_then(|scale| scale.zero) != Some(false);
    let mut low = y_seen.iter().copied().fold(f64::INFINITY, f64::min);
    let mut high = y_seen.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !low.is_finite() {
        (low, high) = (0., 1.);
    }
    if let Some(domain) = y_channel.scale.as_ref().and_then(|scale| scale.domain.as_ref()) {
        if let [from, to] = domain.as_slice() {
            (low, high) = (number(from).unwrap_or(low), number(to).unwrap_or(high));
        }
    }
    if zero {
        low = low.min(0.);
    }
    let count = y_channel.axis.on().and_then(|axis| axis.tick_count).map_or(4, |count| count as usize);
    let (low, high, y_ticks) = ticks::value_ticks(low, high, count);
    let grid = y_channel.axis.on().and_then(|axis| axis.grid).unwrap_or(true);
    let y = YAxis { title: y_title, unit: y_unit, percent, low, high, ticks: y_ticks, shown: !y_channel.axis.is_off(), grid };

    Ok(Chart { x, y, series: series_of(&slots), layers, arcs: None, readings, scatter })
}

/// Whether `word` is the first value a layer's dash field takes, which is drawn solid.
fn dashed_first(leaf: &Leaf, word: &str) -> bool {
    let field = leaf.encoding.stroke_dash.as_ref().and_then(|dash| dash.field.as_ref());
    field.and_then(|field| leaf.rows.iter().find_map(|row| row.get(field))).map(label).as_deref() == Some(word)
}

fn split(points: &[(usize, f64, f64)]) -> BTreeMap<usize, Vec<(f64, f64)>> {
    let mut by: BTreeMap<usize, Vec<(f64, f64)>> = BTreeMap::new();
    for (series, x, y) in points {
        by.entry(*series).or_default().push((*x, *y));
    }
    by
}

fn series_of(slots: &Slots) -> Vec<Series> {
    slots.names.iter().enumerate().map(|(slot, name)| Series { name: name.clone(), slot, key: slots.keys[slot] }).collect()
}
