//! The walk's graphs: a deploy watch of checkout-api, on rows made here rather than read. Each spec is
//! Vega-Lite as an agent would write it and goes through `ipc::graph::read_spec`, so a spec the reader
//! refuses fails the walk instead of drawing. Every 15 seconds each live feed takes its next minute.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, Local, TimeZone};
use gpui::{App, AppContext as _, Task};
use ipc::graph::Row;
use serde_json::{json, Value};

use super::feeds::{FeedBook, Graphs};
use super::{Card, Delta, Form, Headline, Stat, Tone};

/// The scenario the walk plays on, and its Job.
pub const SCENARIO: &str = "live-graphs";
pub const JOB: &str = "watch-checkout";

const LATENCY: &str = "checkout-api/latency";
const ERRORS: &str = "checkout-api/errors";
const HEALTH: &str = "checkout-api/health";
const SPLIT: &str = "checkout-api/split";
const REQUESTS: &str = "checkout-api/requests";

/// The minutes the feeds hold before the deploy and after it.
const BEFORE: i64 = 35;
const AFTER: i64 = 25;

/// The clock the walk reads: today at 13:30 here, the deploy at 14:05.
fn start() -> DateTime<Local> {
    let today = Local::now().date_naive();
    today.and_hms_opt(13, 30, 0).and_then(|at| Local.from_local_datetime(&at).single()).unwrap_or_else(Local::now)
}

fn deploy() -> DateTime<Local> {
    start() + chrono::Duration::minutes(BEFORE)
}

/// A small, fixed wobble in -1..1, the same on every run.
fn wobble(seed: u64, at: i64) -> f64 {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(at as u64).wrapping_mul(1442695040888963407);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    (x % 20001) as f64 / 10000. - 1.
}

fn row(pairs: Value) -> Row {
    pairs.as_object().cloned().unwrap_or_default()
}

fn round(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

/// How far into the regression minute `at` is: 0 before the deploy, rising to 1 over six minutes.
fn after_deploy(at: i64) -> f64 {
    ((at - BEFORE) as f64 / 6.).clamp(0., 1.)
}

fn latency_row(at: i64) -> Row {
    let when = start() + chrono::Duration::minutes(at);
    let worse = after_deploy(at);
    let p50 = 41. + 6. * worse + 2.5 * wobble(1, at);
    let p95 = 118. + 46. * worse + 9. * wobble(2, at);
    let p99 = 205. + 84. * worse + 14. * wobble(3, at);
    row(json!({"at": when.to_rfc3339(), "p50": round(p50, 1), "p95": round(p95, 1), "p99": round(p99, 0)}))
}

fn error_rows(at: i64) -> Vec<Row> {
    let when = start() + chrono::Duration::minutes(at);
    let worse = after_deploy(at);
    let count = |base: f64, rise: f64, seed: u64| (base + rise * worse + (base + rise * worse).sqrt() * wobble(seed, at)).max(0.).round();
    [("500", count(2., 2., 4)), ("502", count(0.6, 9., 5)), ("503", count(0.4, 2.5, 6))]
        .into_iter()
        .map(|(code, count)| row(json!({"at": when.to_rfc3339(), "code": code, "count": count})))
        .collect()
}

fn health_row(at: i64) -> Row {
    let when = start() + chrono::Duration::minutes(at);
    let worse = after_deploy(at);
    // The rollout takes three pods away for three minutes, then they are back.
    let pods = if (BEFORE..BEFORE + 3).contains(&at) { 9. + (at - BEFORE) as f64 } else { 12. };
    row(json!({
        "at": when.to_rfc3339(),
        "error_pct": round(0.35 + 2.05 * worse + 0.12 * wobble(7, at), 2),
        "pods_ready": pods,
        "cpu_pct": round(54. + 14. * worse + 3. * wobble(8, at), 0),
    }))
}

fn request_rows() -> Vec<Row> {
    (0..90)
        .map(|at| {
            let canary = at % 6 == 0;
            let kb = round(4. + 500. * ((wobble(9, at) + 1.) / 2.).powf(1.6), 0);
            let slope = if canary { 0.62 } else { 0.34 };
            let ms = round(38. + kb * slope + 24. * wobble(10, at), 0);
            row(json!({"kb": kb, "ms": ms.max(8.), "version": if canary { "Canary" } else { "Stable" }}))
        })
        .collect()
}

fn feeds() -> BTreeMap<String, Vec<Row>> {
    let mut feeds = BTreeMap::new();
    feeds.insert(LATENCY.to_string(), (0..BEFORE + AFTER).map(latency_row).collect());
    // The bars hold the last forty minutes, so each stays thick enough to read.
    feeds.insert(ERRORS.to_string(), (BEFORE + AFTER - 40..BEFORE + AFTER).flat_map(error_rows).collect());
    feeds.insert(HEALTH.to_string(), (0..BEFORE + AFTER).map(health_row).collect());
    feeds.insert(SPLIT.to_string(), vec![row(json!({"version": "Stable", "rps": 1656})), row(json!({"version": "Canary", "rps": 184}))]);
    feeds.insert(REQUESTS.to_string(), request_rows());
    feeds
}

fn spec(text: String) -> ipc::graph::Spec {
    ipc::graph::read_spec(&text).unwrap_or_else(|why| panic!("the walk's graph does not read: {why}"))
}

fn worse(said: &str, since: Option<&str>) -> Option<Delta> {
    Some(Delta { said: said.into(), since: since.map(Into::into), tone: Tone::Worse })
}

/// The cards, each after the agent's message it belongs to.
fn cards() -> Vec<Rc<Card>> {
    let deploy = deploy().to_rfc3339();
    let marker = format!(
        r#"{{"mark":{{"type":"rule"}},"encoding":{{"x":{{"datum":"{deploy}"}}}}}},{{"mark":{{"type":"text","align":"left"}},"encoding":{{"x":{{"datum":"{deploy}"}},"text":{{"value":"Deploy 4.12"}}}}}}"#
    );
    let card = |id: &str, title: &str, source: &str, after: &str, form: Form| Rc::new(Card { id: id.into(), title: title.into(), source: source.into(), live: true, form, after: after.into() });
    let p99 = spec(format!(
        r#"{{"$schema":"https://vega.github.io/schema/vega-lite/v5.json","title":"p99 latency","width":"container","data":{{"name":"{LATENCY}"}},"layer":[
            {{"mark":{{"type":"area","interpolate":"monotone"}},"encoding":{{"x":{{"field":"at","type":"temporal","title":"Time"}},"y":{{"field":"p99","type":"quantitative","title":"p99 (ms)"}}}}}},
            {{"mark":{{"type":"rule","strokeDash":[4,4]}},"encoding":{{"y":{{"datum":250}}}}}},
            {{"mark":"text","encoding":{{"y":{{"datum":250}},"text":{{"value":"SLO 250 ms"}}}}}},
            {marker}]}}"#
    ));
    let percentiles = spec(format!(
        r#"{{"title":"Latency percentiles","data":{{"name":"{LATENCY}"}},"encoding":{{"x":{{"field":"at","type":"temporal"}}}},"layer":[
            {{"mark":{{"type":"line","interpolate":"monotone"}},"encoding":{{"y":{{"field":"p50","type":"quantitative","title":"p50 (ms)"}}}}}},
            {{"mark":{{"type":"line","interpolate":"monotone"}},"encoding":{{"y":{{"field":"p95","type":"quantitative","title":"p95 (ms)"}}}}}},
            {{"mark":{{"type":"line","interpolate":"monotone"}},"encoding":{{"y":{{"field":"p99","type":"quantitative","title":"p99 (ms)"}}}}}},
            {marker}]}}"#
    ));
    let errors = spec(format!(
        r#"{{"title":"Errors per minute","data":{{"name":"{ERRORS}"}},"layer":[
            {{"mark":"bar","encoding":{{"x":{{"field":"at","type":"temporal"}},"y":{{"field":"count","type":"quantitative","title":"Errors"}},"color":{{"field":"code","type":"nominal","title":"Status","sort":["500","502","503"]}}}}}},
            {marker}]}}"#
    ));
    let split = spec(format!(
        r#"{{"title":"Traffic split","data":{{"name":"{SPLIT}"}},"mark":{{"type":"arc","innerRadius":52}},"encoding":{{"theta":{{"field":"rps","type":"quantitative","title":"Requests (req/s)"}},"color":{{"field":"version","type":"nominal","sort":["Stable","Canary"]}}}}}}"#
    ));
    let scatter = spec(format!(
        r#"{{"title":"Latency by payload size","data":{{"name":"{REQUESTS}"}},"mark":{{"type":"point","filled":true}},"encoding":{{"x":{{"field":"kb","type":"quantitative","title":"Payload (KB)"}},"y":{{"field":"ms","type":"quantitative","title":"Latency (ms)"}},"color":{{"field":"version","type":"nominal","sort":["Stable","Canary"]}},"tooltip":[{{"field":"kb"}},{{"field":"ms"}}]}}}}"#
    ));
    let stat = |label: &str, field: &str, unit: Option<&str>, shown: Option<&str>, delta: Option<Delta>| Stat { label: label.into(), feed: HEALTH.into(), field: field.into(), unit: unit.map(Into::into), shown: shown.map(Into::into), delta };
    vec![
        card("graph-p99", "p99 latency", "Fleet reads every 15s", "m-g2", Form::Chart { spec: p99, headline: Some(Headline { feed: LATENCY.into(), field: "p99".into(), unit: Some("ms".into()), delta: worse("+41%", Some("since deploy")) }) }),
        card("graph-percentiles", "Latency percentiles", "Fleet reads every 15s", "m-g2", Form::Chart { spec: percentiles, headline: None }),
        card("graph-errors", "Errors per minute", "Fleet reads every 15s", "m-g3", Form::Chart { spec: errors, headline: None }),
        card(
            "graph-health",
            "checkout-api health",
            "Fleet reads every 15s",
            "m-g3",
            Form::Strip(vec![
                stat("Error rate", "error_pct", Some("%"), None, worse("+2.1 pts", None)),
                stat("Pods ready", "pods_ready", None, Some("12 of 12"), Some(Delta { said: "All ready".into(), since: None, tone: Tone::Better })),
                stat("CPU", "cpu_pct", Some("%"), None, Some(Delta { said: "+14 pts".into(), since: None, tone: Tone::Steady })),
            ]),
        ),
        card("graph-split", "Traffic split", "Fleet reads every minute", "m-g4", Form::Chart { spec: split, headline: None }),
        card("graph-payload", "Latency by payload size", "Fleet reads every minute", "m-g4", Form::Chart { spec: scatter, headline: None }),
    ]
}

/// Put the walk's graphs in place for `scenario`, or take any away for another.
pub fn begin(scenario: &str, cx: &mut App) {
    if scenario != SCENARIO {
        cx.remove_global::<Graphs>();
        return;
    }
    let book = cx.new(|_| FeedBook::new(feeds()));
    let held = book.downgrade();
    let tick: Task<()> = cx.spawn(async move |cx| {
        // The minute each feed takes next.
        let mut at = BEFORE + AFTER;
        loop {
            cx.background_executor().timer(Duration::from_secs(15)).await;
            let mut batch = vec![(LATENCY.to_string(), latency_row(at)), (HEALTH.to_string(), health_row(at))];
            batch.extend(error_rows(at).into_iter().map(|one| (ERRORS.to_string(), one)));
            at += 1;
            let pushed = held.update(cx, |book, cx| book.push(batch, cx));
            if pushed.is_err() {
                break;
            }
        }
    });
    let mut by_job = BTreeMap::new();
    by_job.insert(JOB.to_string(), cards());
    cx.set_global(Graphs { book, by_job, _tick: tick });
}
