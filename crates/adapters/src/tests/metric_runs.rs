//! A project's metric recipe run with no model: a cli call and an HTTP call through `jq` to rows, and a token that is
//! filled into a header and is in no refusal.

use adapter_traits::Secret;
use ipc::{MetricCall, MetricHeader, MetricRecipe, MetricValue};

use super::taught_tracker_http::{answer, serve};
use crate::run_metric;

const SECRET: &str = "tok-s3cret/+";

fn recipe(call: MetricCall, map: &str) -> MetricRecipe {
    MetricRecipe { id: "open".into(), title: "Open".into(), call, map: map.into(), every_secs: None, credential: None }
}

fn signed_in(call: MetricCall, map: &str) -> MetricRecipe {
    let credential = ipc::MetricCredential { name: "sentry".into(), how: "A token".into(), url: None };
    MetricRecipe { credential: Some(credential), ..recipe(call, map) }
}

fn echo(text: &str) -> MetricCall {
    MetricCall::Cli { argv: vec!["/bin/echo".into(), text.into()] }
}

const ROWS: &str = r#"{rows: [{label: "open", value: .data.n, unit: "issues"}]}"#;

#[test]
fn a_cli_recipe_reads_its_rows_through_the_map() {
    let read = run_metric(&recipe(echo(r#"{"data":{"n":7}}"#), ROWS), None).expect("it reads");
    assert_eq!(read.rows.len(), 1);
    assert_eq!(read.rows[0].label, "open");
    assert_eq!(read.rows[0].value, MetricValue::Number(7.0));
    assert_eq!(read.rows[0].unit.as_deref(), Some("issues"));
}

#[test]
fn a_map_that_does_not_print_an_answer_is_refused_with_its_reason() {
    let said = run_metric(&recipe(echo(r#"{"data":{"n":7}}"#), ".data.n"), None).expect_err("a number is not an answer");
    assert!(said.contains("did not print an answer of rows"), "{said}");
    let said = run_metric(&recipe(echo("{}"), ".data.n | error(\"no count\")"), None).expect_err("jq refused");
    assert!(said.contains("no count"), "{said}");
}

#[test]
fn a_token_goes_in_the_header_and_is_in_no_refusal() {
    let call = |url: &str| MetricCall::Http {
        method: "GET".into(),
        url: format!("{url}/issues"),
        headers: vec![MetricHeader { name: "Authorization".into(), value: "Bearer {credential}".into() }],
        body: None,
    };
    let token = Secret::new(SECRET.to_string());
    let (url, server) = serve(vec![answer("200 OK", r#"{"data":{"n":3}}"#), answer("401 Unauthorized", &format!(r#"{{"detail":"bad token {SECRET}"}}"#))]);
    let read = run_metric(&signed_in(call(&url), ROWS), Some(&token)).expect("it reads");
    assert_eq!(read.rows[0].value, MetricValue::Number(3.0));
    let said = run_metric(&signed_in(call(&url), ROWS), Some(&token)).expect_err("a 401 is said");
    assert!(said.contains("401") && said.contains("[credential]"), "{said}");
    assert!(!said.contains(SECRET), "the token is never said: {said}");
    let requests = server.join().unwrap();
    assert!(requests[0].contains(&format!("Bearer {SECRET}")), "the header carries it: {}", requests[0]);
}

#[test]
fn a_token_may_not_go_where_a_process_list_or_a_log_would_show_it() {
    let argv = signed_in(MetricCall::Cli { argv: vec!["/bin/echo".into(), "{credential}".into()] }, ".");
    let said = run_metric(&argv, Some(&Secret::new(SECRET.to_string()))).expect_err("refused before it runs");
    assert!(said.contains("never `argv`"), "{said}");
    let url = signed_in(MetricCall::Http { method: "GET".into(), url: "https://t.example/?k={credential}".into(), headers: vec![], body: None }, ".");
    assert!(run_metric(&url, Some(&Secret::new(SECRET.to_string()))).expect_err("refused").contains("never `argv`, `server` or `url`"));
    let unnamed = recipe(MetricCall::Http { method: "GET".into(), url: "https://t.example/".into(), headers: vec![MetricHeader { name: "A".into(), value: "{credential}".into() }], body: None }, ".");
    assert!(run_metric(&unnamed, None).expect_err("refused").contains("names no credential"));
    let plain = recipe(MetricCall::Http { method: "GET".into(), url: "http://t.example/".into(), headers: vec![], body: None }, ".");
    assert!(run_metric(&plain, None).expect_err("refused").contains("https"), "a remote host is over TLS");
}
