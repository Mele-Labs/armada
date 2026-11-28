//! A graph a Job puts in its thread, described as a subset of Vega-Lite: the marks, channels and
//! types Bridge draws natively, read from the JSON an agent writes as Vega-Lite would read it.
//!
//! [`read_spec`] is the one way in. It refuses a key Bridge does not draw by name, with the words it
//! does take, so the agent can write the spec again. A key that only styles, such as `width`, a
//! `$schema` or a literal colour, is read and left: the chart colours and sizes are the tokens'.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

/// One record of a graph's data: field name to value, as Vega-Lite's `data.values` holds them.
pub type Row = Map<String, Value>;

/// A graph, or one layer of one. A layer takes the data and encoding of the spec around it where it
/// names none of its own, as Vega-Lite's do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<Encoding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer: Vec<Spec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Title {
    Text(String),
    Full {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
    },
}

impl Title {
    pub fn text(&self) -> &str {
        match self {
            Title::Text(text) | Title::Full { text, .. } => text,
        }
    }
}

/// Rows written into the spec, or the name of a live feed Fleet reads and keeps filling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Row>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkKind {
    #[default]
    Line,
    Area,
    Bar,
    Point,
    Rule,
    Text,
    Arc,
}

/// A mark by its word alone, `"line"`, or with what it is drawn with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mark {
    Kind(MarkKind),
    Full(MarkDef),
}

impl Mark {
    pub fn kind(&self) -> MarkKind {
        match self {
            Mark::Kind(kind) => *kind,
            Mark::Full(def) => def.kind,
        }
    }

    /// What the mark is drawn with; a bare word takes every default.
    pub fn def(&self) -> MarkDef {
        match self {
            Mark::Kind(kind) => MarkDef { kind: *kind, ..MarkDef::default() },
            Mark::Full(def) => def.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkDef {
    #[serde(rename = "type", default = "line_kind")]
    pub kind: MarkKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolate: Option<Interpolate>,
    /// Dash and gap lengths, in points, for a line or a rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash: Option<Vec<f32>>,
    /// A donut's hole, in points, for an arc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_radius: Option<f32>,
    /// A dot at each point of a line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dy: Option<f32>,
    /// A text mark's words, where no `text` channel names them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<bool>,
}

fn line_kind() -> MarkKind {
    MarkKind::Line
}

/// How a line or an area runs between its points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Interpolate {
    Linear,
    Monotone,
    Natural,
    Basis,
    Cardinal,
    Step,
    StepAfter,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encoding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_dash: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<Channel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<Tooltip>,
}

impl Encoding {
    /// This encoding with each channel it leaves out taken from `outer`, as a layer inherits.
    pub fn over(&self, outer: &Encoding) -> Encoding {
        let pick = |own: &Option<Channel>, outer: &Option<Channel>| own.clone().or_else(|| outer.clone());
        Encoding {
            x: pick(&self.x, &outer.x),
            y: pick(&self.y, &outer.y),
            color: pick(&self.color, &outer.color),
            theta: pick(&self.theta, &outer.theta),
            stroke_dash: pick(&self.stroke_dash, &outer.stroke_dash),
            text: pick(&self.text, &outer.text),
            tooltip: self.tooltip.clone().or_else(|| outer.tooltip.clone()),
        }
    }
}

/// The fields a readout names, one or several.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tooltip {
    One(Channel),
    Many(Vec<Channel>),
}

impl Tooltip {
    pub fn channels(&self) -> Vec<&Channel> {
        match self {
            Tooltip::One(one) => vec![one],
            Tooltip::Many(many) => many.iter().collect(),
        }
    }
}

/// One channel: a field of the rows, a constant `datum` in the data's own terms (a rule's 250 ms),
/// or a constant `value` in the drawing's (a text mark's words).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<FieldType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<Aggregate>,
    #[serde(default, skip_serializing_if = "Setting::is_unset")]
    pub stack: Setting<StackOffset>,
    #[serde(default, skip_serializing_if = "Setting::is_unset")]
    pub axis: Setting<Axis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    #[serde(default, skip_serializing_if = "Setting::is_unset")]
    pub legend: Setting<Legend>,
    /// The order of a band's or a colour's values, where it is not the order they first appear in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<Value>>,
    /// A d3 format, as Vega-Lite takes one. Bridge reads only whether it ends in `%`, which shows a share
    /// as a percentage; numbers are otherwise written as Bridge writes every number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Quantitative,
    Temporal,
    Ordinal,
    Nominal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    Sum,
    Mean,
    Count,
    Min,
    Max,
}

/// How a stacked channel stacks: from zero, or as shares of each stack's whole. `true` is `zero`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StackOffset {
    Zero,
    Normalize,
    /// `false`: drawn over one another, not stacked.
    None,
}

impl<'de> Deserialize<'de> for StackOffset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Bool(true) => Ok(StackOffset::Zero),
            Value::Bool(false) => Ok(StackOffset::None),
            Value::String(word) if word == "zero" => Ok(StackOffset::Zero),
            Value::String(word) if word == "normalize" => Ok(StackOffset::Normalize),
            other => Err(serde::de::Error::custom(format!("stack takes zero, normalize, true, false or null, not {other}"))),
        }
    }
}

/// A key Vega-Lite lets a spec set to `null` to turn a thing off: absent, `null`, or set.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Setting<T> {
    #[default]
    Unset,
    Off,
    On(T),
}

impl<T> Setting<T> {
    pub fn is_unset(&self) -> bool {
        matches!(self, Setting::Unset)
    }

    pub fn is_off(&self) -> bool {
        matches!(self, Setting::Off)
    }

    pub fn on(&self) -> Option<&T> {
        match self {
            Setting::On(one) => Some(one),
            _ => None,
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Setting<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(one) => Setting::On(one),
            None => Setting::Off,
        })
    }
}

impl<T: Serialize> Serialize for Setting<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Setting::On(one) => one.serialize(serializer),
            _ => serializer.serialize_none(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Axis {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_count: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scale {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nice: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Legend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Why a spec is not drawn: where in it, and what that place takes instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The words are not JSON.
    NotJson { why: String },
    /// A key or a word Bridge does not draw.
    NotDrawn { at: String, word: String, allowed: &'static [&'static str] },
    /// Something the place needs is not there.
    Missing { at: String, what: &'static str },
    /// The JSON is the right shape in its keys and not in a value.
    Unreadable { why: String },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NotJson { why } => write!(f, "The graph is not JSON: {why}"),
            Refusal::NotDrawn { at, word, allowed } => write!(f, "{at} takes {}; {word} is not drawn", allowed.join(", ")),
            Refusal::Missing { at, what } => write!(f, "{at} needs {what}"),
            Refusal::Unreadable { why } => write!(f, "The graph does not read: {why}"),
        }
    }
}

impl std::error::Error for Refusal {}

/// The keys a spec or a layer takes. `$schema`, sizes and `config` only style, and are left.
const SPEC_KEYS: &[&str] = &["title", "description", "data", "mark", "encoding", "layer"];
const LEFT_KEYS: &[&str] = &["$schema", "width", "height", "autosize", "config", "padding", "background", "name", "view", "usermeta"];
const DATA_KEYS: &[&str] = &["values", "name"];
const MARKS: &[&str] = &["line", "area", "bar", "point", "rule", "text", "arc"];
const MARK_KEYS: &[&str] = &["type", "interpolate", "strokeDash", "innerRadius", "point", "opacity", "filled", "align", "baseline", "dx", "dy", "text", "tooltip"];
const MARK_LEFT: &[&str] = &["color", "stroke", "fill", "strokeWidth", "size", "fontSize", "fontWeight", "font", "cornerRadius", "cornerRadiusEnd", "thickness", "clip"];
const CHANNELS: &[&str] = &["x", "y", "color", "theta", "tooltip", "strokeDash", "text"];
const CHANNEL_KEYS: &[&str] = &["field", "type", "datum", "value", "title", "aggregate", "stack", "axis", "scale", "legend", "sort", "format"];
const TYPES: &[&str] = &["quantitative", "temporal", "ordinal", "nominal"];
const AGGREGATES: &[&str] = &["sum", "mean", "count", "min", "max"];
const AXIS_KEYS: &[&str] = &["title", "format", "grid", "tickCount"];
const AXIS_LEFT: &[&str] = &["labelAngle", "labelColor", "tickColor", "domainColor", "gridColor", "orient"];
const SCALE_KEYS: &[&str] = &["domain", "zero", "nice"];
const SCALE_LEFT: &[&str] = &["range", "scheme", "type"];
const LEGEND_KEYS: &[&str] = &["title"];
const LEGEND_LEFT: &[&str] = &["orient", "direction"];
const INTERPOLATIONS: &[&str] = &["linear", "monotone", "natural", "basis", "cardinal", "step", "step-after"];

/// Read a graph an agent wrote. A refusal names the first place Bridge would not draw and what
/// that place takes.
pub fn read_spec(text: &str) -> Result<Spec, Refusal> {
    let value: Value = serde_json::from_str(text).map_err(|why| Refusal::NotJson { why: why.to_string() })?;
    let cleaned = check_spec(&value, "The graph", true)?;
    let spec: Spec = serde_json::from_value(cleaned).map_err(|why| Refusal::Unreadable { why: why.to_string() })?;
    if spec.mark.is_none() && spec.layer.is_empty() {
        return Err(Refusal::Missing { at: "The graph".into(), what: "a mark or a layer" });
    }
    Ok(spec)
}

fn object<'a>(value: &'a Value, at: &str, what: &'static str) -> Result<&'a Map<String, Value>, Refusal> {
    value.as_object().ok_or_else(|| Refusal::Missing { at: at.to_string(), what })
}

/// The keys of `map` that are drawn, the left ones dropped and any other refused.
fn keep(map: &Map<String, Value>, at: &str, allowed: &'static [&'static str], left: &[&str]) -> Result<Map<String, Value>, Refusal> {
    let mut kept = Map::new();
    for (key, value) in map {
        if allowed.contains(&key.as_str()) {
            kept.insert(key.clone(), value.clone());
        } else if !left.contains(&key.as_str()) {
            return Err(Refusal::NotDrawn { at: at.to_string(), word: key.clone(), allowed });
        }
    }
    Ok(kept)
}

fn word<'a>(value: &'a Value, at: &str, allowed: &'static [&'static str]) -> Result<&'a str, Refusal> {
    let text = value.as_str().ok_or_else(|| Refusal::Missing { at: at.to_string(), what: "a word" })?;
    if allowed.contains(&text) {
        Ok(text)
    } else {
        Err(Refusal::NotDrawn { at: at.to_string(), word: text.to_string(), allowed })
    }
}

fn check_spec(value: &Value, at: &str, top: bool) -> Result<Value, Refusal> {
    let map = object(value, at, "an object")?;
    let left: &[&str] = if top { LEFT_KEYS } else { &LEFT_KEYS[1..] };
    let mut kept = keep(map, at, SPEC_KEYS, left)?;
    if let Some(data) = kept.get("data") {
        let at = format!("{at}'s data");
        let data = keep(object(data, &at, "values or a name")?, &at, DATA_KEYS, &[])?;
        if !data.contains_key("values") && !data.contains_key("name") {
            return Err(Refusal::Missing { at, what: "values or a name" });
        }
        kept.insert("data".into(), Value::Object(data));
    }
    if let Some(mark) = kept.get("mark").cloned() {
        kept.insert("mark".into(), check_mark(&mark, &format!("{at}'s mark"))?);
    }
    if let Some(encoding) = kept.get("encoding").cloned() {
        kept.insert("encoding".into(), check_encoding(&encoding, &format!("{at}'s encoding"))?);
    }
    if let Some(layer) = kept.get("layer").cloned() {
        let layers = layer.as_array().ok_or_else(|| Refusal::Missing { at: format!("{at}'s layer"), what: "a list of layers" })?;
        let checked = layers.iter().enumerate().map(|(ix, one)| check_spec(one, &format!("Layer {}", ix + 1), false)).collect::<Result<Vec<_>, _>>()?;
        kept.insert("layer".into(), Value::Array(checked));
    }
    Ok(Value::Object(kept))
}

fn check_mark(value: &Value, at: &str) -> Result<Value, Refusal> {
    if value.is_string() {
        word(value, at, MARKS)?;
        return Ok(value.clone());
    }
    let kept = keep(object(value, at, "a mark word or an object")?, at, MARK_KEYS, MARK_LEFT)?;
    word(kept.get("type").ok_or_else(|| Refusal::Missing { at: at.to_string(), what: "a type" })?, at, MARKS)?;
    if let Some(how) = kept.get("interpolate") {
        word(how, &format!("{at}'s interpolate"), INTERPOLATIONS)?;
    }
    Ok(Value::Object(kept))
}

fn check_encoding(value: &Value, at: &str) -> Result<Value, Refusal> {
    let kept = keep(object(value, at, "an object")?, at, CHANNELS, &[])?;
    let mut checked = Map::new();
    for (name, channel) in kept {
        let at = format!("{at}'s {name}");
        let one = match (&name[..], channel) {
            ("tooltip", Value::Array(many)) => Value::Array(many.iter().map(|one| check_channel(one, &at)).collect::<Result<_, _>>()?),
            (_, one) => check_channel(&one, &at)?,
        };
        checked.insert(name, one);
    }
    Ok(Value::Object(checked))
}

fn check_channel(value: &Value, at: &str) -> Result<Value, Refusal> {
    let mut kept = keep(object(value, at, "a field, a datum or a value")?, at, CHANNEL_KEYS, &["bandPosition"])?;
    if let Some(kind) = kept.get("type") {
        word(kind, &format!("{at}'s type"), TYPES)?;
    }
    if let Some(aggregate) = kept.get("aggregate") {
        word(aggregate, &format!("{at}'s aggregate"), AGGREGATES)?;
    }
    if !kept.contains_key("field") && !kept.contains_key("datum") && !kept.contains_key("value") && kept.get("aggregate").and_then(Value::as_str) != Some("count") {
        return Err(Refusal::Missing { at: at.to_string(), what: "a field, a datum or a value" });
    }
    for (key, allowed, left) in [("axis", AXIS_KEYS, AXIS_LEFT), ("scale", SCALE_KEYS, SCALE_LEFT), ("legend", LEGEND_KEYS, LEGEND_LEFT)] {
        if let Some(inner) = kept.get(key).filter(|inner| !inner.is_null()).cloned() {
            let at = format!("{at}'s {key}");
            kept.insert(key.into(), Value::Object(keep(object(&inner, &at, "an object or null")?, &at, allowed, left)?));
        }
    }
    // A sort that is a word (`ascending`) keeps the rows' own order; only a list of values orders a domain.
    if kept.get("sort").is_some_and(|sort| !sort.is_array()) {
        kept.remove("sort");
    }
    Ok(Value::Object(kept))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layered_spec_reads_and_a_transform_is_refused_by_name() {
        let spec = read_spec(r#"{"$schema":"https://example.com/schema.json","width":400,"data":{"name":"p99"},"layer":[{"mark":{"type":"area","interpolate":"monotone","color":"red"},"encoding":{"x":{"field":"at","type":"temporal"},"y":{"field":"ms","type":"quantitative"}}},{"mark":"rule","encoding":{"y":{"datum":250}}}]}"#).expect("reads");
        assert_eq!(spec.layer.len(), 2);
        assert_eq!(spec.layer[0].mark.as_ref().map(Mark::kind), Some(MarkKind::Area));
        let refused = read_spec(r#"{"mark":"line","transform":[{"filter":"datum.ms > 1"}]}"#).expect_err("refused");
        assert!(matches!(refused, Refusal::NotDrawn { ref word, .. } if word == "transform"), "{refused}");
        let refused = read_spec(r#"{"mark":"boxplot"}"#).expect_err("refused");
        assert!(refused.to_string().contains("line, area, bar, point, rule, text, arc"), "{refused}");
    }

    #[test]
    fn a_null_axis_is_off_and_an_absent_one_unset() {
        let spec = read_spec(r#"{"mark":"bar","encoding":{"x":{"field":"a","type":"nominal","axis":null},"y":{"field":"b","type":"quantitative","stack":"normalize"}}}"#).expect("reads");
        let encoding = spec.encoding.expect("an encoding");
        assert!(encoding.x.expect("x").axis.is_off());
        let y = encoding.y.expect("y");
        assert!(y.axis.is_unset());
        assert_eq!(y.stack, Setting::On(StackOffset::Normalize));
    }
}
