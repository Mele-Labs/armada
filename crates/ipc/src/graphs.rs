//! A Job's live graphs: `show_graph`, `graph_points` and `withdraw_graph`, which an agent calls, and what
//! `list_graphs` and `graphs.changed` carry to Bridge. `docs/practices/protocol/a-job-shows-a-live-graph.md`.
//!
//! **The spec is [`crate::graph`]'s Vega-Lite subset**, read by [`crate::graph::read_spec`] at the call, so an
//! agent hears of a key Bridge would not draw. The rows a spec's `data.name` reads are a feed of the Job,
//! fed by the agent's `graph_points` or by a recipe Fleet runs, and Fleet keeps the newest
//! [`GRAPH_FEED_KEPT`] of each.
//!
//! **A headline's and a stat's change are worked out here and said on the card**, so Bridge draws
//! `+41% since deploy` and works nothing out. Fleet calls [`GraphChecked::card`] each time a feed moves.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::graph::{read_spec, FieldType, Row, Spec};
use crate::{Instant, MetricCall, SessionId};

/// The most rows one feed keeps, oldest dropped first.
pub const GRAPH_FEED_KEPT: usize = 2000;
/// The most rows one `graph_points` call adds.
pub const GRAPH_POINTS_AT_MOST: usize = 1000;
/// The most graphs one Job holds.
pub const GRAPHS_A_JOB: usize = 24;
/// How often a recipe may run at most, and at least.
pub const GRAPH_EVERY_SECS_LEAST: u32 = 10;
pub const GRAPH_EVERY_SECS_MOST: u32 = 86_400;

const MOST_TITLE_CHARS: usize = 120;
const MOST_ID_CHARS: usize = 64;
const MOST_FEED_CHARS: usize = 120;
const MOST_UNIT_CHARS: usize = 16;
const MOST_STATS: usize = 8;
/// The field a strip's stats, and a chart with no temporal `x`, read a row's time from.
pub const GRAPH_TIME_FIELD: &str = "at";

/// `show_graph`: a Job puts a live graph in its thread. **The same `id` again replaces it in place.**
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShowGraph {
    /// Lower case letters, digits and dashes, unique in the Job.
    pub id: String,
    pub title: String,
    /// Vega-Lite, as [`read_spec`] reads it. A chart has a spec and a strip has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<Value>,
    /// The latest value of a chart's feed, shown large over it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<HeadlineAsked>,
    /// A row of numbers, each with its sparkline, in place of a spec.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strip: Vec<StatAsked>,
    /// What feeds it while Fleet holds it. Absent, the agent feeds it with `graph_points`, or its spec's
    /// `data.values` is all it draws.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<GraphSource>,
    /// Which Job it is for. **Read only where the connection places no hosted Job**, as `ShowSketch`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// A headline as the agent asks for it: which field of the chart's feed, and what its change is against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlineAsked {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<GraphSince>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub better: Option<Better>,
}

/// One number of a strip, as the agent asks for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatAsked {
    pub label: String,
    pub feed: String,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// How the value is written in place of the field's latest, such as `12 of 12`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<GraphSince>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub better: Option<Better>,
}

/// What a change is measured against: the value the feed held at `at`, named by `label` (`deploy`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSince {
    pub label: String,
    pub at: Instant,
}

/// Which way a value moving is good news. Absent, a change is neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Better {
    Up,
    Down,
}

/// Where a graph's rows come from, beside the agent's own `graph_points`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSource {
    pub recipe: GraphRecipe,
}

/// A call Fleet makes every `every_secs` with no model, as a project metric's recipe is (`MetricRecipe`), and
/// a `jq` map from its answer to rows. **The map prints one JSON object of feed names to lists of rows**,
/// `{"p99": [{"at": "…", "ms": 212}]}`, or a list of rows where the graph reads one feed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRecipe {
    pub call: MetricCall,
    pub map: String,
    pub every_secs: u32,
}

/// `graph_points`: rows the agent worked out itself, added to one feed of its Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPoints {
    pub feed: String,
    pub rows: Vec<Row>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `graph_points` answers: how many of the rows were new, and the feed's count after.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPointsTaken {
    pub session_id: SessionId,
    pub feed: String,
    /// The rows added: a row the feed holds already is not added twice.
    pub new: usize,
    /// The rows the feed keeps now, at most [`GRAPH_FEED_KEPT`].
    pub kept: usize,
    /// Every row the feed was ever given, which [`GraphFeed::added`] carries.
    pub added: u64,
}

/// `withdraw_graph`: its recipe stops. **The card stays**, no longer live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawGraph {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// One graph as Bridge draws its card: the card's words and form, how it is fed, and where its reading stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphShown {
    pub id: String,
    pub title: String,
    pub form: GraphForm,
    pub fed_by: GraphFedBy,
    /// It still takes rows: not withdrawn, and its Job not ended. A live recipe is run every `every_secs`.
    pub live: bool,
    /// The latest `show_graph` of the id.
    pub shown_at: Instant,
    /// The recipe's last read that gave rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<Instant>,
    /// Why the recipe's last read gave none, in a sentence. The rows it held are kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broken: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphForm {
    /// A chart drawn from its spec, with an optional headline over it.
    Chart {
        spec: Spec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headline: Option<GraphHeadline>,
    },
    /// A row per number, each with its sparkline, value and change.
    Strip { stats: Vec<GraphStat> },
}

/// How a graph's rows arrive. Bridge words it under the title.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphFedBy {
    /// Fleet runs a recipe every `every_secs`.
    Recipe { every_secs: u32 },
    /// The agent sends rows with `graph_points`.
    Points,
}

/// The latest value of a chart's feed, and how it moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphHeadline {
    pub feed: String,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<GraphDelta>,
}

/// One number of a strip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphStat {
    pub label: String,
    pub feed: String,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<GraphDelta>,
}

/// How a figure moved, as Fleet words it: `said` `+41%` or `+2.1 pts`, `since` `since deploy`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphDelta {
    pub said: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    pub tone: GraphTone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphTone {
    Worse,
    Better,
    Steady,
}

/// One feed's rows. In `list_graphs` they are every row it keeps, oldest first; in `graphs.changed` the rows
/// added since the last event, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphFeed {
    pub name: String,
    pub rows: Vec<Row>,
    /// Every row the feed was ever given, these included. **A client holding `added - rows.len()` has every
    /// row before these**; any other count means it missed an event, and it reads `list_graphs` again.
    pub added: u64,
}

/// `list_graphs`: a Job's graphs in the order first shown, and every row their feeds keep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphList {
    pub session_id: SessionId,
    pub graphs: Vec<GraphShown>,
    pub feeds: Vec<GraphFeed>,
}

/// `graphs.changed`: one Job's graphs, whole, and the rows added to its feeds since the last event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphsChanged {
    pub session_id: SessionId,
    pub graphs: Vec<GraphShown>,
    /// Only the feeds that took rows, each with the rows it took.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<GraphFeed>,
}

/// A `show_graph` as Fleet keeps it: checked, its spec read.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphChecked {
    pub id: String,
    pub title: String,
    pub form: FormAsked,
    pub recipe: Option<GraphRecipe>,
    /// The feeds it reads, each once, in the order named.
    pub feeds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FormAsked {
    Chart { spec: Spec, headline: Option<(String, HeadlineAsked)> },
    Strip(Vec<StatAsked>),
}

/// How a feed name is written: letters, digits and `_ . / : -`, up to 120 of them.
pub fn feed_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MOST_FEED_CHARS
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | ':' | '-'))
}

fn id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= MOST_ID_CHARS && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The feeds a spec and its layers read by `data.name`, each once.
fn named_feeds(spec: &Spec, into: &mut Vec<String>) {
    if let Some(name) = spec.data.as_ref().and_then(|data| data.name.as_ref()) {
        if !into.contains(name) {
            into.push(name.clone());
        }
    }
    for layer in &spec.layer {
        named_feeds(layer, into);
    }
}

fn has_values(spec: &Spec) -> bool {
    spec.data.as_ref().is_some_and(|data| data.values.is_some()) || spec.layer.iter().any(has_values)
}

/// The field a chart reads a row's time from: its first temporal `x`, else [`GRAPH_TIME_FIELD`].
pub fn time_field(spec: &Spec) -> String {
    fn found(spec: &Spec) -> Option<String> {
        let x = spec.encoding.as_ref().and_then(|encoding| encoding.x.as_ref());
        if let Some(x) = x.filter(|x| x.kind == Some(FieldType::Temporal)) {
            if let Some(field) = &x.field {
                return Some(field.clone());
            }
        }
        spec.layer.iter().find_map(found)
    }
    found(spec).unwrap_or_else(|| GRAPH_TIME_FIELD.to_string())
}

fn short(text: &Option<String>, what: &str, most: usize, problems: &mut Vec<String>) {
    if text.as_ref().is_some_and(|text| text.chars().count() > most) {
        problems.push(format!("`{what}` is longer than {most} characters"));
    }
}

fn since_ok(since: &Option<GraphSince>, at: &str, problems: &mut Vec<String>) {
    let Some(since) = since else { return };
    if since.label.trim().is_empty() {
        problems.push(format!("{at}`since.label` is blank: name what the change is measured against, such as `deploy`"));
    }
    if millis_of_text(since.at.as_str()).is_none() {
        problems.push(format!("{at}`since.at` is not an RFC 3339 time, such as `2026-11-27T14:05:00Z`"));
    }
}

impl ShowGraph {
    /// The graph, or every problem with it in one sentence that names what is allowed.
    pub fn checked(&self) -> Result<GraphChecked, String> {
        let mut problems = Vec::new();
        let id = self.id.trim().to_string();
        if !id_ok(&id) {
            problems.push(format!("`id` is lower case letters, digits and dashes, at most {MOST_ID_CHARS}"));
        }
        let title = self.title.trim().to_string();
        if title.is_empty() || title.chars().count() > MOST_TITLE_CHARS {
            problems.push(format!("`title` is between 1 and {MOST_TITLE_CHARS} characters"));
        }
        let mut feeds = Vec::new();
        let form = match (&self.spec, self.strip.is_empty()) {
            (Some(_), false) => {
                problems.push(String::from("a graph is a `spec` or a `strip`, not both"));
                None
            }
            (None, true) => {
                problems.push(String::from("a graph needs a `spec`, Vega-Lite, or a `strip` of stats"));
                None
            }
            (Some(spec), true) => match read_spec(&spec.to_string()) {
                Err(why) => {
                    problems.push(format!("`spec` does not draw: {why}"));
                    None
                }
                Ok(spec) => {
                    named_feeds(&spec, &mut feeds);
                    if feeds.is_empty() && !has_values(&spec) {
                        problems.push(String::from("`spec` reads no data: name a feed in `data.name`, or give rows in `data.values`"));
                    }
                    let headline = match (&self.headline, feeds.first()) {
                        (None, _) => None,
                        (Some(_), None) => {
                            problems.push(String::from("a `headline` reads a feed, and `spec` names none in `data.name`"));
                            None
                        }
                        (Some(headline), Some(feed)) => {
                            if headline.field.trim().is_empty() {
                                problems.push(String::from("`headline.field` is blank"));
                            }
                            short(&headline.unit, "headline.unit", MOST_UNIT_CHARS, &mut problems);
                            since_ok(&headline.since, "", &mut problems);
                            Some((feed.clone(), headline.clone()))
                        }
                    };
                    Some(FormAsked::Chart { spec, headline })
                }
            },
            (None, false) => {
                if self.headline.is_some() {
                    problems.push(String::from("a `headline` is a chart's; a strip's stats each say their own"));
                }
                if self.strip.len() > MOST_STATS {
                    problems.push(format!("a `strip` holds at most {MOST_STATS} stats"));
                }
                for (ix, stat) in self.strip.iter().enumerate() {
                    let at = format!("strip[{ix}] ");
                    if stat.label.trim().is_empty() || stat.field.trim().is_empty() {
                        problems.push(format!("{at}needs a `label` and a `field`"));
                    }
                    if !feed_name_ok(&stat.feed) {
                        problems.push(format!("{at}`feed` is letters, digits and `_ . / : -`, at most {MOST_FEED_CHARS}"));
                    } else if !feeds.contains(&stat.feed) {
                        feeds.push(stat.feed.clone());
                    }
                    short(&stat.unit, "unit", MOST_UNIT_CHARS, &mut problems);
                    short(&stat.shown, "shown", 40, &mut problems);
                    since_ok(&stat.since, &at, &mut problems);
                }
                Some(FormAsked::Strip(self.strip.clone()))
            }
        };
        for feed in &feeds {
            if !feed_name_ok(feed) {
                problems.push(format!("the feed `{feed}` is not a name: letters, digits and `_ . / : -`, at most {MOST_FEED_CHARS}"));
            }
        }
        let recipe = self.source.as_ref().map(|source| source.recipe.clone());
        if let Some(recipe) = &recipe {
            if !(GRAPH_EVERY_SECS_LEAST..=GRAPH_EVERY_SECS_MOST).contains(&recipe.every_secs) {
                problems.push(format!("`source.recipe.every_secs` is between {GRAPH_EVERY_SECS_LEAST} and {GRAPH_EVERY_SECS_MOST}"));
            }
            if recipe.map.trim().is_empty() {
                problems.push(String::from("`source.recipe.map` is blank: a `jq` filter printing {\"<feed>\": [rows]}"));
            }
            if feeds.is_empty() {
                problems.push(String::from("a recipe feeds a feed, and this graph reads none: name one in the spec's `data.name`"));
            }
        }
        match (problems.is_empty(), form) {
            (true, Some(form)) => Ok(GraphChecked { id, title, form, recipe, feeds }),
            _ => Err(format!(
                "the graph was not shown: {}. `show_graph`'s description names the Vega-Lite it draws",
                problems.join("; ")
            )),
        }
    }
}

impl GraphPoints {
    /// The rows, or why none are taken.
    pub fn checked(&self) -> Result<(), String> {
        if !feed_name_ok(&self.feed) {
            return Err(format!("`feed` is letters, digits and `_ . / : -`, at most {MOST_FEED_CHARS}"));
        }
        if self.rows.is_empty() {
            return Err(String::from("`rows` is empty: give at least one row, an object of fields"));
        }
        if self.rows.len() > GRAPH_POINTS_AT_MOST {
            return Err(format!("`rows` holds at most {GRAPH_POINTS_AT_MOST} rows a call"));
        }
        Ok(())
    }
}

/// What a recipe's map printed, as rows by feed. `feeds` are the graph's: a list of rows goes to its one feed.
pub fn recipe_rows(printed: &[u8], feeds: &[String]) -> Result<Vec<(String, Vec<Row>)>, String> {
    let value: Value = crate::decode("a graph recipe's rows", printed).map_err(|why| format!("the `map` did not print JSON: {}", why.why))?;
    let by_feed: Vec<(String, Value)> = match value {
        Value::Object(map) => map.into_iter().collect(),
        Value::Array(rows) => match feeds {
            [one] => vec![(one.clone(), Value::Array(rows))],
            _ => return Err(String::from("the `map` printed a list of rows, and the graph reads several feeds: print {\"<feed>\": [rows]}")),
        },
        _ => return Err(String::from("the `map` printed neither {\"<feed>\": [rows]} nor a list of rows")),
    };
    let mut read = Vec::with_capacity(by_feed.len());
    for (feed, rows) in by_feed {
        if !feed_name_ok(&feed) {
            return Err(format!("the `map` printed `{feed}`, which is not a feed name"));
        }
        let Value::Array(rows) = rows else {
            return Err(format!("the `map` printed `{feed}` as something other than a list of rows"));
        };
        let mut kept = Vec::with_capacity(rows.len());
        for row in rows {
            match row {
                Value::Object(row) => kept.push(row),
                _ => return Err(format!("the `map` printed a row of `{feed}` that is not an object")),
            }
        }
        // A recipe that answers a long window gives only what a feed can keep.
        let over = kept.len().saturating_sub(GRAPH_FEED_KEPT);
        kept.drain(..over);
        read.push((feed, kept));
    }
    Ok(read)
}

/// Milliseconds since the epoch of an RFC 3339 time, its offset applied.
fn millis_of_text(text: &str) -> Option<i64> {
    let local = core_model::Timestamp::from_rfc3339(text.to_string()).epoch_millis()?;
    let zone = text.get(19..)?.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone {
        "Z" | "z" | "" => 0,
        signed if signed.len() == 6 && (signed.starts_with('+') || signed.starts_with('-')) => {
            let hours: i64 = signed.get(1..3)?.parse().ok()?;
            let minutes: i64 = signed.get(4..6)?.parse().ok()?;
            let off = (hours * 60 + minutes) * 60_000;
            if signed.starts_with('+') { off } else { -off }
        }
        _ => return None,
    };
    Some(local - offset)
}

/// A row's time: an RFC 3339 string, or a number of milliseconds as Vega-Lite reads one.
fn millis_of(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => millis_of_text(text),
        Value::Number(number) => number.as_f64().filter(|ms| ms.is_finite()).map(|ms| ms as i64),
        _ => None,
    }
}

fn number_of(row: &Row, field: &str) -> Option<f64> {
    row.get(field).and_then(Value::as_f64).filter(|value| value.is_finite())
}

/// A number written short: whole where it is whole or large, else to one place.
fn written(value: f64) -> String {
    let rounded = if value.abs() >= 10. { value.round() } else { (value * 10.).round() / 10. };
    if rounded.fract() == 0. {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded:.1}")
    }
}

/// How `field` of `rows` moved: from the value at `since.at`, else the first row, to the latest. **Points**
/// where the unit is `%`, a share whose percentage change would mislead, else a percentage. `None` where
/// there is no number to start or end from, or the start is 0 and a percentage would divide by it.
pub fn delta_of(rows: &[Row], time: &str, field: &str, unit: Option<&str>, since: Option<&GraphSince>, better: Option<Better>) -> Option<GraphDelta> {
    let latest = rows.iter().rev().find_map(|row| number_of(row, field))?;
    let start = match since {
        None => rows.iter().find_map(|row| number_of(row, field))?,
        Some(since) => {
            let at = millis_of_text(since.at.as_str())?;
            rows.iter()
                .filter_map(|row| Some((millis_of(row.get(time)?)?, number_of(row, field)?)))
                .filter(|(when, _)| *when <= at)
                .max_by_key(|(when, _)| *when)?
                .1
        }
    };
    let moved = latest - start;
    let said = match unit {
        Some("%") => match written(moved).as_str() {
            "0" => String::from("0 pts"),
            figure if moved > 0. => format!("+{figure} pts"),
            figure => format!("{figure} pts"),
        },
        _ if start == 0. => return None,
        _ => {
            let share = moved / start.abs() * 100.;
            match written(share).as_str() {
                "0" => String::from("0%"),
                figure if share > 0. => format!("+{figure}%"),
                figure => format!("{figure}%"),
            }
        }
    };
    let tone = match (better, moved.partial_cmp(&0.)) {
        (_, None | Some(std::cmp::Ordering::Equal)) | (None, _) => GraphTone::Steady,
        (Some(Better::Up), Some(std::cmp::Ordering::Greater)) | (Some(Better::Down), Some(std::cmp::Ordering::Less)) => GraphTone::Better,
        _ => GraphTone::Worse,
    };
    let tone = if said.starts_with("0") { GraphTone::Steady } else { tone };
    Some(GraphDelta { said, since: since.map(|since| format!("since {}", since.label.trim())), tone })
}

/// Where Fleet's reading of one graph stands, beside what was shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphReading {
    pub live: bool,
    pub shown_at: Option<Instant>,
    pub read_at: Option<Instant>,
    pub broken: Option<String>,
}

impl GraphChecked {
    /// The card Bridge draws, its changes worked out on `feeds`, the rows each of the Job's feeds keeps.
    pub fn card(&self, feeds: &BTreeMap<String, Vec<Row>>, reading: &GraphReading) -> GraphShown {
        let none: Vec<Row> = Vec::new();
        let rows = |feed: &str| feeds.get(feed).unwrap_or(&none);
        let form = match &self.form {
            FormAsked::Chart { spec, headline } => GraphForm::Chart {
                spec: spec.clone(),
                headline: headline.as_ref().map(|(feed, asked)| GraphHeadline {
                    feed: feed.clone(),
                    field: asked.field.clone(),
                    unit: asked.unit.clone(),
                    delta: delta_of(rows(feed), &time_field(spec), &asked.field, asked.unit.as_deref(), asked.since.as_ref(), asked.better),
                }),
            },
            FormAsked::Strip(stats) => GraphForm::Strip {
                stats: stats
                    .iter()
                    .map(|stat| GraphStat {
                        label: stat.label.clone(),
                        feed: stat.feed.clone(),
                        field: stat.field.clone(),
                        unit: stat.unit.clone(),
                        shown: stat.shown.clone(),
                        delta: delta_of(rows(&stat.feed), GRAPH_TIME_FIELD, &stat.field, stat.unit.as_deref(), stat.since.as_ref(), stat.better),
                    })
                    .collect(),
            },
        };
        GraphShown {
            id: self.id.clone(),
            title: self.title.clone(),
            form,
            fed_by: match &self.recipe {
                Some(recipe) => GraphFedBy::Recipe { every_secs: recipe.every_secs },
                None => GraphFedBy::Points,
            },
            live: reading.live,
            shown_at: reading.shown_at.clone().unwrap_or_else(|| Instant::carried("")),
            read_at: reading.read_at.clone(),
            broken: reading.broken.clone(),
        }
    }
}
