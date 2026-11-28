//! `show_graph`'s checks, the change a headline and a stat say, a recipe's printed rows, and `graphs.changed` on the wire.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::graph::Row;
use crate::{
    delta_of, recipe_rows, Better, Event, GraphFeed, GraphForm, GraphReading, GraphSince, GraphTone, GraphsChanged, Instant, SessionId, ShowGraph,
    ThreadGraph, ThreadItem,
};

fn show(body: Value) -> ShowGraph {
    crate::decode_body("a graph to show", body.to_string().as_bytes()).expect("a body")
}

fn row(value: Value) -> Row {
    value.as_object().cloned().expect("an object")
}

fn p99() -> Value {
    json!({
        "id": "p99",
        "title": "p99 latency",
        "spec": {"data": {"name": "checkout/latency"}, "layer": [
            {"mark": {"type": "area", "interpolate": "monotone"}, "encoding": {"x": {"field": "when", "type": "temporal"}, "y": {"field": "p99", "type": "quantitative"}}},
            {"mark": "rule", "encoding": {"y": {"datum": 250}}}
        ]},
        "headline": {"field": "p99", "unit": "ms", "since": {"label": "deploy", "at": "2026-11-27T14:05:00Z"}, "better": "down"},
        "source": {"recipe": {"call": {"transport": "cli", "argv": ["printf", "{}"]}, "map": ".", "every_secs": 15}}
    })
}

#[test]
fn a_chart_reads_its_feed_and_headline_and_a_transform_is_refused_by_name() {
    let checked = show(p99()).checked().expect("draws");
    assert_eq!(checked.feeds, vec!["checkout/latency".to_string()]);
    assert_eq!(checked.recipe.as_ref().map(|recipe| recipe.every_secs), Some(15));
    let mut body = p99();
    body["spec"]["transform"] = json!([{"filter": "datum.p99 > 1"}]);
    let refused = show(body).checked().expect_err("refused");
    assert!(refused.contains("`spec` does not draw") && refused.contains("transform is not drawn"), "{refused}");
    assert!(refused.contains("show_graph"), "a refusal points at the act's own help: {refused}");
}

#[test]
fn every_problem_with_a_graph_is_said_at_once() {
    let refused = show(json!({
        "id": "P 99",
        "title": "",
        "spec": {"mark": "line", "encoding": {"x": {"field": "at"}}},
        "strip": [{"label": "CPU", "feed": "health", "field": "cpu"}],
        "source": {"recipe": {"call": {"transport": "cli", "argv": ["true"]}, "map": "", "every_secs": 1}}
    }))
    .checked()
    .expect_err("refused");
    for said in ["`id` is lower case", "`title` is between", "a `spec` or a `strip`, not both", "every_secs` is between 10", "`source.recipe.map` is blank"] {
        assert!(refused.contains(said), "{said} missing from: {refused}");
    }
    let refused = show(json!({"id": "a", "title": "A", "spec": {"mark": "line"}, "headline": {"field": "x"}})).checked().expect_err("refused");
    assert!(refused.contains("reads no data") && refused.contains("names none in `data.name`"), "{refused}");
    let refused = show(json!({"id": "a", "title": "A", "strip": [{"label": "", "feed": "a b", "field": "x", "since": {"label": "deploy", "at": "lunch"}}]}))
        .checked()
        .expect_err("refused");
    assert!(refused.contains("strip[0] needs a `label`") && refused.contains("strip[0] `feed` is letters") && refused.contains("not an RFC 3339 time"), "{refused}");
    let points: crate::GraphPoints = crate::decode_body("rows for a feed", br#"{"feed":"p99","rows":[{"ms":1},2]}"#).expect("a body");
    assert!(points.checked().expect_err("refused").starts_with("rows[1] is not an object"));
}

#[test]
fn a_change_is_a_percentage_against_the_value_at_since_or_points_for_a_share() {
    let rows = vec![
        row(json!({"at": "2026-11-27T14:00:00Z", "p99": 200, "err": 0.4})),
        row(json!({"at": "2026-11-27T15:04:00+01:00", "p99": 205, "err": 0.5})),
        row(json!({"at": "2026-11-27T14:30:00Z", "p99": 289, "err": 2.5})),
    ];
    let since = GraphSince { label: "deploy".into(), at: Instant::carried("2026-11-27T14:05:00Z") };
    // The value at the deploy is the latest at or before it: 14:04 UTC, written in +01:00.
    let delta = delta_of(&rows, "at", "p99", Some("ms"), Some(&since), Some(Better::Down)).expect("a change");
    assert_eq!((delta.said.as_str(), delta.since.as_deref(), delta.tone), ("+41%", Some("since deploy"), GraphTone::Worse));
    let delta = delta_of(&rows, "at", "err", Some("%"), None, Some(Better::Down)).expect("a change");
    assert_eq!((delta.said.as_str(), delta.since, delta.tone), ("+2.1 pts", None, GraphTone::Worse));
    let delta = delta_of(&rows, "at", "p99", None, None, Some(Better::Up)).expect("a change");
    assert_eq!(delta.tone, GraphTone::Better);
    assert_eq!(delta_of(&rows, "at", "p99", None, None, None).expect("a change").tone, GraphTone::Steady);
    let early = GraphSince { label: "start".into(), at: Instant::carried("2026-11-27T13:00:00Z") };
    assert_eq!(delta_of(&rows, "at", "p99", None, Some(&early), None), None, "no row stood then");
}

#[test]
fn a_card_carries_its_headlines_change_worked_out_on_its_feed() {
    let checked = show(p99()).checked().expect("draws");
    let mut feeds = BTreeMap::new();
    feeds.insert(
        "checkout/latency".to_string(),
        vec![row(json!({"when": "2026-11-27T14:04:00Z", "p99": 200})), row(json!({"when": "2026-11-27T14:20:00Z", "p99": 150}))],
    );
    let card = checked.card(&feeds, &GraphReading { live: true, shown_at: Some(Instant::carried("2026-11-27T14:00:00.000Z")), ..GraphReading::default() });
    let GraphForm::Chart { headline: Some(headline), .. } = &card.form else { panic!("a chart with a headline: {card:?}") };
    let delta = headline.delta.as_ref().expect("a change");
    assert_eq!((delta.said.as_str(), delta.tone), ("-25%", GraphTone::Better), "read by the spec's temporal x, `when`");
}

#[test]
fn a_recipe_prints_rows_by_feed_or_a_list_for_its_one_feed() {
    let one = vec!["p99".to_string()];
    let read = recipe_rows(br#"{"p99":[{"at":"x","ms":1}],"errors":[]}"#, &one).expect("reads");
    assert_eq!(read.iter().map(|(feed, rows)| (feed.as_str(), rows.len())).collect::<Vec<_>>(), vec![("errors", 0), ("p99", 1)]);
    let read = recipe_rows(br#"[{"ms":1},{"ms":2}]"#, &one).expect("reads");
    assert_eq!(read[0].0, "p99");
    let two = vec!["a".to_string(), "b".to_string()];
    assert!(recipe_rows(br#"[{"ms":1}]"#, &two).expect_err("refused").contains("several feeds"));
    assert!(recipe_rows(br#"{"p99":[1]}"#, &one).expect_err("refused").contains("not an object"));
    assert!(recipe_rows(b"ms 1", &one).expect_err("refused").contains("did not print JSON"));
}

#[test]
fn graphs_changed_carries_the_cards_whole_and_the_rows_appended() {
    let checked = show(p99()).checked().expect("draws");
    let card = checked.card(&BTreeMap::new(), &GraphReading { live: true, shown_at: Some(Instant::carried("2026-11-27T14:00:00.000Z")), ..GraphReading::default() });
    let event = Event::GraphsChanged(GraphsChanged {
        session_id: SessionId::carried("s1"),
        graphs: vec![card],
        feeds: vec![GraphFeed { name: "checkout/latency".into(), rows: vec![row(json!({"when": "2026-11-27T14:20:00Z", "p99": 150}))], added: 61 }],
    });
    assert_eq!(event.kind(), "graphs.changed");
    let json = crate::encode(&event).expect("plain data");
    for said in [
        r#""kind":"graphs.changed","session_id":"s1","graphs":[{"id":"p99","title":"p99 latency","form":{"kind":"chart","spec":"#,
        r#""headline":{"feed":"checkout/latency","field":"p99","unit":"ms"}"#,
        r#""fed_by":{"kind":"recipe","every_secs":15},"live":true,"shown_at":"2026-11-27T14:00:00.000Z"}]"#,
        r#""feeds":[{"name":"checkout/latency","rows":[{"p99":150,"when":"2026-11-27T14:20:00Z"}],"added":61}]"#,
    ] {
        assert!(json.contains(said), "{said} missing from {json}");
    }
    let back: Event = crate::decode("an event", json.as_bytes()).expect("reads back");
    assert_eq!(back, event);
}

#[test]
fn a_show_graph_rows_text_names_its_graph_and_the_item_carries_it() {
    assert_eq!(crate::graph_call_of("mcp__armada-fleet__show_graph p99"), Some("p99".to_string()));
    assert_eq!(crate::graph_call_of("mcp__armada-fleet__show_sketch p99"), None);
    let item = ThreadItem::Graph(ThreadGraph { id: "t1".into(), at: Instant::carried("2026-11-27T14:00:00.000Z"), graph: "p99".into(), again: false });
    assert_eq!(crate::encode(&item).expect("plain data"), r#"{"kind":"graph","id":"t1","at":"2026-11-27T14:00:00.000Z","graph":"p99"}"#);
    assert_eq!(item.id(), "t1");
}
