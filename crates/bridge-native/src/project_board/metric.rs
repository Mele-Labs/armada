//! What a metric block draws, worked out from one reading Fleet made. A metric block is a recipe's rows: the first number large,
//! the rows as items for List, Table and Cards (`view.rs` draws those once for every block), a series for Graph and a picture
//! where the tool gave one. Nothing here touches the screen or the wire.

use chrono::{DateTime, Local, Utc};
use ipc::time_format::TimeFormat;
use ipc::{Instant, MetricAnswer, MetricPoint, MetricReading, MetricRow, MetricValue};

use super::layout::View;
use super::model::{Item, Press};
use crate::look::token;

/// The views a reading is offered in: the four every metric has, and Picture only where the answer carries one.
pub fn offered(reading: &MetricReading) -> Vec<View> {
    let mut views = vec![View::List, View::Table, View::Cards, View::Graph];
    if reading.answer.as_ref().is_some_and(|answer| answer.picture.is_some()) {
        views.push(View::Picture);
    }
    views
}

/// The view a block is drawn in: the one it keeps, or the first that is offered where the kept one is not (a Picture saved
/// before the tool stopped giving one). The saved view is not touched.
pub fn drawn_in(kept: View, reading: &MetricReading) -> View {
    let views = offered(reading);
    if views.contains(&kept) { kept } else { views[0] }
}

/// A value at a time.
pub type Point = (DateTime<Utc>, f64);

/// What Graph draws, and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub enum Plot {
    /// The answer's own rows, which carry the time the tool says each stood. At least two of them make a series.
    Rows(Vec<Point>),
    /// The values Fleet kept from its readings while the dashboard was open.
    Kept(Vec<Point>),
}

impl Plot {
    pub fn points(&self) -> &[Point] {
        match self {
            Plot::Rows(points) | Plot::Kept(points) => points,
        }
    }
}

fn instant(at: &Instant) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok().map(|at| at.with_timezone(&Utc))
}

fn number(row: &MetricRow) -> Option<f64> {
    match row.value {
        MetricValue::Number(value) => Some(value),
        MetricValue::Text(_) => None,
    }
}

/// Rows that carry a time and a number are a series, drawn in time; an answer without one draws what Fleet kept.
pub fn plot(reading: &MetricReading) -> Plot {
    let mut timed: Vec<Point> = reading.answer.iter().flat_map(|answer| &answer.rows).filter_map(|row| Some((instant(row.at.as_ref()?)?, number(row)?))).collect();
    if timed.len() >= 2 {
        timed.sort_by_key(|(at, _)| *at);
        return Plot::Rows(timed);
    }
    let kept = reading.kept.iter().filter_map(|MetricPoint { at, value }| Some((instant(at)?, *value))).collect();
    Plot::Kept(kept)
}

/// A number as it is read: whole where it is whole, else two places at most.
pub fn show_number(value: f64) -> String {
    if value.fract() == 0. && value.abs() < 1e15 {
        return format!("{value:.0}");
    }
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A row's value with its unit.
pub fn show_row(row: &MetricRow) -> String {
    let value = match &row.value {
        MetricValue::Number(value) => show_number(*value),
        MetricValue::Text(text) => text.clone(),
    };
    match row.unit.as_deref().filter(|unit| !unit.is_empty()) {
        Some(unit) => format!("{value} {unit}"),
        None => value,
    }
}

/// The first number of an answer, which the block shows large.
pub struct Headline {
    pub value: String,
    pub unit: Option<String>,
    pub label: String,
}

pub fn headline(answer: &MetricAnswer) -> Option<Headline> {
    let row = answer.rows.iter().find(|row| number(row).is_some())?;
    let value = number(row).map(show_number)?;
    Some(Headline { value, unit: row.unit.clone().filter(|unit| !unit.is_empty()), label: row.label.clone() })
}

/// The rows of an answer as the items List, Table and Cards draw.
pub fn items(id: &str, answer: &MetricAnswer) -> Vec<Item> {
    answer
        .rows
        .iter()
        .enumerate()
        .map(|(at, row)| {
            let item = Item::new(format!("metric-{id}-{at}"), "circle-dot", token::SKETCH_BLUE, "Reading", row.label.clone()).meta(show_row(row));
            match &row.link {
                Some(link) => item.press(Press::Url(link.clone())),
                None => item,
            }
        })
        .collect()
}

/// The clock in the format he set, with no seconds: `14:02`.
pub fn axis(at: DateTime<Utc>) -> String {
    crate::settings::time::time_of(at.with_timezone(&Local), TimeFormat { seconds: false, ..crate::settings::time::held() })
}

/// The time a reading stood, as the clock reads it: `14:02`.
pub fn clock(at: &Instant) -> Option<String> {
    instant(at).map(axis)
}

/// The same time in full, for a tooltip: its day where it is not today, and the seconds where he reads them.
pub fn full(at: &Instant) -> Option<String> {
    crate::settings::time::stamp_at(at.as_str())
}

/// `as of 14:02`, and the full time for its tooltip.
pub fn as_of(reading: &MetricReading) -> Option<(String, String)> {
    let at = reading.read_at.as_ref()?;
    Some((format!("as of {}", clock(at)?), full(at)?))
}

/// `Last good reading 14:02`, where the broken reading still holds an answer, and the full time for its tooltip.
pub fn last_good(reading: &MetricReading) -> Option<(String, String)> {
    reading.answer.as_ref()?;
    let at = reading.read_at.as_ref()?;
    Some((format!("Last good reading {}", clock(at)?), full(at)?))
}
