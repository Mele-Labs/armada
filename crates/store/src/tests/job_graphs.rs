//! A Job's graph is replaced where it stands, its feeds keep their newest rows and add no row twice, and both survive a reopen.

use crate::tests::{open, TempDir};
use crate::KeptGraph;

const AT: &str = "2026-11-27T14:00:00.000Z";

fn graph(id: &str, every: Option<u32>) -> KeptGraph {
    KeptGraph {
        session_id: "s1".into(),
        id: id.into(),
        shown: format!(r#"{{"id":"{id}"}}"#),
        shown_at: AT.into(),
        every_seconds: every,
        live: true,
        tried_at: None,
        read_at: None,
        broken: None,
    }
}

fn rows(from: usize, to: usize) -> Vec<String> {
    (from..to).map(|n| format!(r#"{{"n":{n}}}"#)).collect()
}

#[test]
fn a_feed_keeps_its_newest_rows_and_adds_none_twice() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.add_graph_rows("s1", "p99", &rows(0, 6), 4).expect("kept");
    assert_eq!((first.new.len(), first.kept, first.added), (6, 4, 6));
    // A window read again gives only its new rows.
    let again = store.add_graph_rows("s1", "p99", &rows(4, 8), 4).expect("kept");
    assert_eq!(again.new, rows(6, 8));
    assert_eq!((again.kept, again.added), (4, 8));
    let feeds = store.graph_feeds("s1").expect("read");
    assert_eq!(feeds.len(), 1);
    assert_eq!((feeds[0].name.as_str(), feeds[0].rows.clone(), feeds[0].added), ("p99", rows(4, 8), 8));
    assert!(store.graph_feeds("s2").expect("read").is_empty(), "a feed is one Job's");
}

#[test]
fn a_graph_shown_again_stands_where_it_was_and_is_live_again() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.keep_graph(&graph("p99", Some(15))).expect("kept");
        store.keep_graph(&graph("errors", None)).expect("kept");
        store.graph_tried("s1", "p99", "2026-11-27T14:00:15.000Z", None).expect("kept");
        store.graph_tried("s1", "p99", "2026-11-27T14:00:30.000Z", Some("the call timed out")).expect("kept");
        assert!(store.withdraw_graph("s1", "p99").expect("kept"));
        assert!(!store.withdraw_graph("s1", "p99").expect("kept"), "withdrawn once");
        assert!(store.live_graph_recipes().expect("read").is_empty());
        let mut again = graph("p99", Some(30));
        again.shown = String::from(r#"{"id":"p99","again":true}"#);
        store.keep_graph(&again).expect("kept");
        store.add_graph_rows("s1", "p99", &rows(0, 2), 2000).expect("kept");
    }
    let store = open(&dir);
    let graphs = store.graphs_of("s1").expect("read");
    assert_eq!(graphs.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), vec!["p99", "errors"], "in the order first shown");
    let p99 = &graphs[0];
    assert!(p99.live && p99.shown.contains("again") && p99.every_seconds == Some(30));
    assert_eq!((p99.tried_at.as_deref(), p99.broken.as_deref()), (None, None), "a new recipe runs at the next pass");
    assert_eq!(p99.read_at.as_deref(), Some("2026-11-27T14:00:15.000Z"), "the last read that gave rows stays");
    assert_eq!(store.live_graph_recipes().expect("read").len(), 1);
    assert_eq!(store.graph_feeds("s1").expect("read")[0].rows.len(), 2);
}

#[test]
fn a_jobs_end_ends_every_graph_of_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_graph(&graph("p99", Some(15))).expect("kept");
    store.keep_graph(&KeptGraph { session_id: "s2".into(), ..graph("p99", Some(15)) }).expect("kept");
    assert!(store.end_graphs_of("s1").expect("kept"));
    assert!(!store.end_graphs_of("s1").expect("kept"));
    let live = store.live_graph_recipes().expect("read");
    assert_eq!(live.iter().map(|one| one.session_id.as_str()).collect::<Vec<_>>(), vec!["s2"]);
}
