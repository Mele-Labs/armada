//! A Job's live graph: shown, replaced in place and withdrawn, fed by the agent's rows and by a recipe on its
//! timer, published as `graphs.changed` with only the rows that were new, and stopped with its Job.

use std::sync::Arc;
use std::time::Duration;

use api::{Graphs, HostedSessions, Next, Refusal, Subscription};
use ipc::graph::Row;
use ipc::{CloseSession, Event, GraphForm, GraphPoints, GraphsChanged, SessionId, ShowGraph, WithdrawGraph};

use super::planted::Held;
use super::session_host::{rig_fitted, Rig};

fn rig_held() -> (Rig, Arc<Held>) {
    let clock = Arc::new(Held::started());
    let held = Arc::clone(&clock);
    (rig_fitted(move |fittings| fittings.clock = held), clock)
}

fn show(body: &str) -> ShowGraph {
    ipc::decode_body("a graph to show", body.as_bytes()).expect("a body")
}

fn row(text: &str) -> Row {
    ipc::decode("a row", text.as_bytes()).expect("an object")
}

/// A line of p99 latency, with a headline. `recipe` is the JSON of a `source.recipe`.
fn p99(job: &SessionId, title: &str, recipe: Option<String>) -> ShowGraph {
    let source = recipe.map(|recipe| format!(r#","source":{{"recipe":{recipe}}}"#)).unwrap_or_default();
    show(&format!(
        r#"{{"id":"p99","title":"{title}","session_id":"{}","spec":{{"data":{{"name":"latency"}},"mark":"line","encoding":{{"x":{{"field":"at","type":"temporal"}},"y":{{"field":"ms","type":"quantitative"}}}}}},"headline":{{"field":"ms","unit":"ms","better":"down"}}{source}}}"#,
        job.as_str()
    ))
}

/// A recipe that prints the file `name` in the rig's folder, whatever it holds, every 15 seconds.
fn cat(rig: &Rig, name: &str) -> String {
    let path = std::path::Path::new(&rig.root).join(name);
    format!(r#"{{"call":{{"transport":"cli","argv":["/bin/cat","{}"]}},"map":".","every_secs":15}}"#, path.display())
}

fn printed(rig: &Rig, name: &str, text: &str) {
    std::fs::write(std::path::Path::new(&rig.root).join(name), text).unwrap();
}

async fn feed_rows(rig: &Rig, job: &SessionId, feed: &str) -> usize {
    let list = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap();
    list.feeds.iter().find(|one| one.name == feed).map_or(0, |one| one.rows.len())
}

/// Every `graphs.changed` of `job` published until the stream has been quiet a moment.
async fn changed(watching: &mut Subscription, job: &SessionId) -> Vec<GraphsChanged> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        if let Event::GraphsChanged(graphs) = delivered.event {
            if graphs.session_id == *job {
                seen.push(graphs);
            }
        }
    }
    seen
}

#[tokio::test]
async fn a_recipe_runs_at_its_interval_and_a_broken_read_keeps_the_rows() {
    let (rig, clock) = rig_held();
    let job = rig.start().await;
    printed(&rig, "rows.json", r#"{"latency":[{"at":"2026-11-27T14:00:00Z","ms":200}]}"#);
    rig.fleet.show_graph(None, p99(&job, "p99 latency", Some(cat(&rig, "rows.json")))).await.expect("shown");
    let mut watching = rig.fleet.events().subscribe();

    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 1, "a new recipe runs at the next pass");
    let sent = changed(&mut watching, &job).await;
    assert_eq!(sent.len(), 1, "one event a tick: {sent:?}");
    assert_eq!((sent[0].feeds[0].rows.len(), sent[0].feeds[0].added), (1, 1));

    printed(&rig, "rows.json", r#"{"latency":[{"at":"2026-11-27T14:00:00Z","ms":200},{"at":"2026-11-27T14:00:15Z","ms":250}]}"#);
    clock.on(10);
    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 1, "not due before its 15 seconds");
    assert!(changed(&mut watching, &job).await.is_empty());

    clock.on(6);
    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 2, "the window read again adds only its new row");
    let sent = changed(&mut watching, &job).await;
    assert_eq!(sent[0].feeds[0].rows, vec![row(r#"{"at":"2026-11-27T14:00:15Z","ms":250}"#)]);
    assert_eq!(sent[0].feeds[0].added, 2);
    let GraphForm::Chart { headline: Some(headline), .. } = &sent[0].graphs[0].form else { panic!("a headline") };
    assert_eq!(headline.delta.as_ref().map(|delta| (delta.said.as_str(), delta.tone)), Some(("+25%", ipc::GraphTone::Worse)));

    printed(&rig, "rows.json", "not json");
    clock.on(16);
    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 2, "a broken read keeps the rows");
    let card = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.remove(0);
    assert!(card.live && card.read_at.is_some());
    assert!(card.broken.as_deref().is_some_and(|why| !why.is_empty()), "why is kept: {card:?}");
}

#[tokio::test]
async fn a_graph_shown_again_stands_in_place_and_a_withdrawn_one_stays_unlive() {
    let rig = rig_fitted(|_| {});
    let job = rig.start().await;
    rig.fleet.show_graph(None, p99(&job, "p99 latency", None)).await.expect("shown");
    let errors = show(&format!(r#"{{"id":"errors","title":"Errors","session_id":"{}","strip":[{{"label":"Error rate","feed":"health","field":"pct","unit":"%"}}]}}"#, job.as_str()));
    rig.fleet.show_graph(None, errors).await.expect("shown");
    let again = rig.fleet.show_graph(None, p99(&job, "p99, after the deploy", None)).await.expect("shown again");
    assert_eq!(again.title, "p99, after the deploy");
    let list = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap();
    assert_eq!(list.graphs.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["p99", "errors"], "replaced where it stood");

    let withdrawn = rig.fleet.withdraw_graph(None, WithdrawGraph { id: "p99".into(), session_id: Some(job.clone()) }).await.expect("withdrawn");
    assert!(!withdrawn.live);
    assert_eq!(rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.len(), 2, "the card stays");

    let Err(Refusal::Unacceptable(refused)) = rig.fleet.withdraw_graph(None, WithdrawGraph { id: "nope".into(), session_id: Some(job.clone()) }).await else {
        panic!("refused")
    };
    assert_eq!(refused.code, "fleet.graph_not_shown");
    assert!(refused.message.contains("p99, errors"), "{}", refused.message);

    let mut bad = p99(&job, "p99", None);
    bad.spec = show(r#"{"id":"a","title":"A","spec":{"mark":"boxplot"}}"#).spec;
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.show_graph(None, bad).await else { panic!("refused") };
    assert_eq!(refused.code, "fleet.graph_refused");
    assert!(refused.message.contains("line, area, bar, point, rule, text, arc"), "names what is drawn: {}", refused.message);

    let shell = p99(&job, "p99", Some(String::from(r#"{"call":{"transport":"http","method":"DELETE","url":"https://example.com"},"map":".","every_secs":15}"#)));
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.show_graph(None, shell).await else { panic!("refused") };
    assert!(refused.message.contains("`GET` or a `POST`"), "the call is held to a metric's rules: {}", refused.message);
}

#[tokio::test]
async fn graph_points_send_only_the_rows_that_were_new() {
    let rig = rig_fitted(|_| {});
    let job = rig.start().await;
    rig.fleet.show_graph(None, p99(&job, "p99 latency", None)).await.expect("shown");
    let mut watching = rig.fleet.events().subscribe();
    changed(&mut watching, &job).await;

    let text = |n: i64| format!(r#"{{"at":{},"ms":{}}}"#, n * 60_000, 200 + n);
    let rows = |from: i64, to: i64| -> Vec<Row> { (from..to).map(|n| row(&text(n))).collect() };
    let points = |from: i64, to: i64| -> GraphPoints {
        let rows: Vec<String> = (from..to).map(text).collect();
        let body = format!(r#"{{"feed":"latency","rows":[{}],"session_id":"{}"}}"#, rows.join(","), job.as_str());
        ipc::decode_body("rows for a feed", body.as_bytes()).expect("a body")
    };
    let first = rig.fleet.graph_points(None, points(0, 3)).await.expect("taken");
    assert_eq!((first.new, first.kept, first.added), (3, 3, 3));
    let second = rig.fleet.graph_points(None, points(2, 5)).await.expect("taken");
    assert_eq!((second.new, second.kept, second.added), (2, 5, 5));
    let sent = changed(&mut watching, &job).await;
    assert_eq!(sent.len(), 2, "one event a call: {sent:?}");
    assert_eq!(sent[1].feeds[0].rows, rows(3, 5));
    assert_eq!(sent[1].feeds[0].added, 5, "a client holding three rows has every one before these");

    let repeat = rig.fleet.graph_points(None, points(3, 5)).await.expect("taken");
    assert_eq!(repeat.new, 0);
    assert!(changed(&mut watching, &job).await.is_empty(), "no new rows, no event");
}

#[tokio::test]
async fn a_feed_keeps_its_newest_two_thousand_rows() {
    let rig = rig_fitted(|_| {});
    let job = rig.start().await;
    for batch in 0..3 {
        let rows: Vec<String> = (batch * 1000..(batch + 1) * 1000).map(|n| format!(r#"{{"n":{n}}}"#)).collect();
        let body = format!(r#"{{"feed":"many","rows":[{}],"session_id":"{}"}}"#, rows.join(","), job.as_str());
        let points: GraphPoints = ipc::decode_body("rows for a feed", body.as_bytes()).expect("a body");
        rig.fleet.graph_points(None, points).await.expect("taken");
    }
    let list = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap();
    let feed = &list.feeds[0];
    assert_eq!((feed.rows.len(), feed.added), (ipc::GRAPH_FEED_KEPT, 3000));
    assert_eq!(feed.rows[0], row(r#"{"n":1000}"#), "the oldest went first");
}

#[tokio::test]
async fn a_jobs_end_stops_its_recipes() {
    let (rig, clock) = rig_held();
    let job = rig.start().await;
    printed(&rig, "rows.json", r#"{"latency":[{"at":"2026-11-27T14:00:00Z","ms":200}]}"#);
    rig.fleet.show_graph(None, p99(&job, "p99 latency", Some(cat(&rig, "rows.json")))).await.expect("shown");
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("closed");

    clock.on(16);
    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 0, "nothing ran for an ended Job");
    let card = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.remove(0);
    assert!(!card.live);
}

#[tokio::test]
async fn a_recipe_signs_in_with_its_credentials_token_and_without_one_the_card_names_it() {
    let clock = Arc::new(Held::started());
    let keychain = Arc::new(super::requesting::Held::default());
    let (held, tokens) = (Arc::clone(&clock), Arc::clone(&keychain));
    let rig = rig_fitted(move |fittings| {
        fittings.clock = held;
        fittings.metric_tokens = tokens;
    });
    let job = rig.start().await;
    let root = std::path::Path::new(&rig.root);
    printed(&rig, "rows.json", r#"{"latency":[{"at":"2026-11-27T14:00:00Z","ms":200}]}"#);
    // The rows print only where the token reached the call; a failing call says the token, which must not be kept.
    let script = format!(
        r#"if [ -f {fail} ]; then echo "refused $TOKEN" >&2; exit 1; fi; test "$TOKEN" = t0ken-0f-gr4fana && cat {rows}"#,
        fail = root.join("fail").display(),
        rows = root.join("rows.json").display()
    );
    let recipe = format!(
        r#"{{"call":{{"transport":"cli","argv":["/bin/sh","-c",{}],"env":[{{"name":"TOKEN","value":"{{credential}}"}}]}},"map":".","every_secs":15,"credential":{{"name":"grafana","how":"A Grafana service account token with Viewer","url":"https://grafana.example/org/serviceaccounts"}}}}"#,
        ipc::encode(&script).unwrap()
    );
    rig.fleet.show_graph(None, p99(&job, "p99 latency", Some(recipe))).await.expect("a credential is held to a metric's rules");

    rig.fleet.graphs_read_when_due().await;
    let card = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.remove(0);
    let credential = card.no_token.as_ref().expect("the card names the missing token");
    assert_eq!((credential.name.as_str(), credential.url.as_deref()), ("grafana", Some("https://grafana.example/org/serviceaccounts")));
    assert_eq!(card.broken.as_deref(), Some("No grafana token on this Mac."));
    assert_eq!(feed_rows(&rig, &job, "latency").await, 0, "nothing ran without it");

    keychain.values.lock().unwrap().insert("//grafana".into(), "t0ken-0f-gr4fana".into());
    clock.on(16);
    rig.fleet.graphs_read_when_due().await;
    assert_eq!(feed_rows(&rig, &job, "latency").await, 1, "the token filled the call");
    let card = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.remove(0);
    assert_eq!((card.no_token, card.broken), (None, None));

    printed(&rig, "fail", "");
    clock.on(16);
    rig.fleet.graphs_read_when_due().await;
    let card = rig.fleet.list_graphs(None, Some(job.clone())).await.unwrap().graphs.remove(0);
    let why = card.broken.expect("the failed call is said");
    assert!(why.contains("refused") && !why.contains("t0ken-0f-gr4fana"), "scrubbed of the token: {why}");
}
