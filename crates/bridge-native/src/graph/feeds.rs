//! Each Job's graph cards and the live rows their feeds hold, one book a Job so two Jobs' feeds never mix. A card
//! observes its Job's book and draws again in place when the book moves, and nothing else does.
//!
//! Fleet fills a book: `list_graphs` whole when the Job is shown, then each `graphs.changed` with the Job's cards whole
//! and only the rows added since the last event (`docs/practices/protocol/a-job-shows-a-live-graph.md`). A feed's
//! `added` says whether a row went by unseen, and then the book is read again rather than drawn with a hole in it. On
//! mock data the walk's scenario fills one instead (`mock.rs`).

use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{App, Context, Entity, Global, Task};
use ipc::graph::Row;
use ipc::{GraphList, GraphsChanged, GRAPH_FEED_KEPT};

use super::Card;

pub struct FeedBook {
    feeds: BTreeMap<String, Vec<Row>>,
    /// How many rows a feed keeps where `keep` names none.
    window: usize,
    /// How many rows each mock feed keeps: as many as it opened with.
    keep: BTreeMap<String, usize>,
    /// Every row Fleet ever gave each feed, as its last `GraphFeed::added` said.
    added: BTreeMap<String, u64>,
    cards: Vec<Rc<Card>>,
    /// Fleet's `list_graphs` has answered for it, so an event lands on rows it can count from.
    read: bool,
}

/// What a `graphs.changed` did to a book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    /// The cards and every feed's rows are as Fleet holds them.
    Whole,
    /// A feed's rows do not follow on from the ones held: `list_graphs` is read again.
    Missed,
    /// The book has not been read yet, and the read under way answers for the event.
    Unread,
}

impl FeedBook {
    /// A walk's book: its cards, and feeds that each keep as many rows as they open with.
    pub fn new(feeds: BTreeMap<String, Vec<Row>>, cards: Vec<Rc<Card>>) -> FeedBook {
        let keep = feeds.iter().map(|(name, rows)| (name.clone(), rows.len())).collect();
        FeedBook { feeds, window: usize::MAX, keep, added: BTreeMap::new(), cards, read: true }
    }

    /// A Job's book before Fleet has answered: no cards, and feeds that keep what Fleet keeps.
    pub fn unread() -> FeedBook {
        FeedBook { feeds: BTreeMap::new(), window: GRAPH_FEED_KEPT, keep: BTreeMap::new(), added: BTreeMap::new(), cards: Vec::new(), read: false }
    }

    pub fn rows(&self, name: &str) -> Option<&[Row]> {
        self.feeds.get(name).map(Vec::as_slice)
    }

    /// The Job's graphs, in the order first shown.
    pub fn cards(&self) -> &[Rc<Card>] {
        &self.cards
    }

    pub fn card(&self, id: &str) -> Option<Rc<Card>> {
        self.cards.iter().find(|card| card.id == id).cloned()
    }

    /// `list_graphs`: the cards and every feed's rows replace the ones held.
    pub fn read_list(&mut self, list: GraphList) {
        self.cards = list.graphs.iter().map(|shown| Rc::new(Card::of(shown))).collect();
        self.feeds.clear();
        self.added.clear();
        for feed in list.feeds {
            let mut rows = feed.rows;
            let over = rows.len().saturating_sub(self.window);
            rows.drain(..over);
            self.feeds.insert(feed.name.clone(), rows);
            self.added.insert(feed.name, feed.added);
        }
        self.read = true;
    }

    /// `graphs.changed`: the cards replace the ones held, and each feed's rows go on the end of it where they follow on
    /// from the rows held. A feed whose rows do not is left as it is, and the answer says to read the book again.
    pub fn take_changed(&mut self, changed: GraphsChanged) -> Taken {
        if !self.read {
            return Taken::Unread;
        }
        self.cards = changed.graphs.iter().map(|shown| Rc::new(Card::of(shown))).collect();
        let mut taken = Taken::Whole;
        for feed in changed.feeds {
            let held = self.added.get(&feed.name).copied().unwrap_or(0);
            if feed.added.checked_sub(feed.rows.len() as u64) != Some(held) {
                taken = Taken::Missed;
                continue;
            }
            self.added.insert(feed.name.clone(), feed.added);
            let keep = self.keep.get(&feed.name).copied().unwrap_or(self.window);
            let rows = self.feeds.entry(feed.name).or_default();
            rows.extend(feed.rows);
            let over = rows.len().saturating_sub(keep);
            rows.drain(..over);
        }
        taken
    }

    /// Put a batch of rows on the end of their feeds, each keeping as many as it opened with, and draw once.
    pub fn push(&mut self, batch: Vec<(String, Row)>, cx: &mut Context<Self>) {
        if batch.is_empty() {
            return;
        }
        for (name, row) in batch {
            let keep = self.keep.get(&name).copied().unwrap_or(self.window);
            let rows = self.feeds.entry(name).or_default();
            rows.push(row);
            let over = rows.len().saturating_sub(keep);
            rows.drain(..over);
        }
        cx.notify();
    }
}

impl super::compile::Feeds for FeedBook {
    fn rows(&self, name: &str) -> Option<&[Row]> {
        FeedBook::rows(self, name)
    }
}

/// Each Job's book, by Job id.
#[derive(Default)]
pub struct Graphs {
    pub by_job: BTreeMap<String, Entity<FeedBook>>,
    /// What feeds a walk's book while it is held.
    pub _tick: Option<Task<()>>,
}

impl Global for Graphs {}

/// Every graph card of `job`, in the order first shown.
pub fn of_job(job: &str, cx: &App) -> Vec<Rc<Card>> {
    book(job, cx).map(|book| book.read(cx).cards.clone()).unwrap_or_default()
}

/// The book of `job`, where one is held.
pub fn book(job: &str, cx: &App) -> Option<Entity<FeedBook>> {
    cx.try_global::<Graphs>().and_then(|graphs| graphs.by_job.get(job)).cloned()
}

/// Whether Fleet has answered for `job`'s graphs, so a graph the thread names and the book lacks is one Fleet holds none of.
pub fn is_read(job: &str, cx: &App) -> bool {
    book(job, cx).is_some_and(|book| book.read(cx).read)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use ipc::{Event, GraphFeed, GraphFedBy, GraphForm, GraphShown, Instant, SessionId};

    use super::*;

    fn row(n: u64) -> Row {
        serde_json::json!({"at": n * 60_000, "ms": n}).as_object().cloned().unwrap_or_default()
    }

    fn rows(from: u64, to: u64) -> Vec<Row> {
        (from..to).map(row).collect()
    }

    fn shown(title: &str) -> GraphShown {
        let spec = ipc::graph::read_spec(r#"{"data":{"name":"latency"},"mark":"line","encoding":{"x":{"field":"at","type":"temporal"},"y":{"field":"ms","type":"quantitative"}}}"#).expect("reads");
        GraphShown { id: "p99".into(), title: title.into(), form: GraphForm::Chart { spec, headline: None }, fed_by: GraphFedBy::Points, live: true, shown_at: Instant::carried("2026-11-27T14:00:00Z"), read_at: None, broken: None, no_token: None }
    }

    fn list(job: &str, from: u64, to: u64) -> GraphList {
        GraphList { session_id: SessionId::carried(job), graphs: vec![shown("p99 latency")], feeds: vec![GraphFeed { name: "latency".into(), rows: rows(from, to), added: to }] }
    }

    fn changed(job: &str, title: &str, from: u64, to: u64) -> GraphsChanged {
        GraphsChanged { session_id: SessionId::carried(job), graphs: vec![shown(title)], feeds: vec![GraphFeed { name: "latency".into(), rows: rows(from, to), added: to }] }
    }

    #[test]
    fn rows_fleet_adds_go_on_the_end_of_their_feed_which_keeps_what_fleet_keeps() {
        let kept = GRAPH_FEED_KEPT as u64;
        let mut book = FeedBook::unread();
        book.read_list(list("job", 0, kept));
        assert_eq!(book.take_changed(changed("job", "p99, after the deploy", kept, kept + 3)), Taken::Whole);
        let held = book.rows("latency").expect("the feed");
        assert_eq!(held.len(), GRAPH_FEED_KEPT, "the oldest went first");
        assert_eq!((held.first(), held.last()), (Some(&row(3)), Some(&row(kept + 2))));
        assert_eq!(book.card("p99").map(|card| card.title.clone()).as_deref(), Some("p99, after the deploy"), "the cards arrive whole");
    }

    #[test]
    fn rows_that_do_not_follow_on_from_the_ones_held_are_not_drawn_and_ask_for_a_read() {
        let mut book = FeedBook::unread();
        assert_eq!(book.take_changed(changed("job", "p99", 0, 2)), Taken::Unread, "the read under way answers for it");
        assert!(book.cards().is_empty());
        book.read_list(list("job", 0, 5));
        assert_eq!(book.take_changed(changed("job", "p99", 7, 9)), Taken::Missed, "rows 5 and 6 went by unseen");
        assert_eq!(book.rows("latency").map(<[Row]>::len), Some(5), "no feed with a hole in it");
        assert_eq!(book.take_changed(changed("job", "p99", 5, 9)), Taken::Whole);
    }

    /// A stand-in Fleet that answers `list_graphs` with every row it holds, counting the reads.
    fn stand_in_fleet(reads: Arc<AtomicUsize>, answer: GraphList) -> u16 {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("its address").port();
        let body = ipc::encode(&answer).expect("encodes");
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
                let mut first = String::new();
                if reader.read_line(&mut first).is_err() {
                    continue;
                }
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                    line.clear();
                }
                let ours = first.split_whitespace().nth(1).is_some_and(|path| path == "/sessions/graphs?session_id=job-a");
                if ours {
                    reads.fetch_add(1, Ordering::SeqCst);
                }
                let (status, body) = if ours { ("200 OK", body.clone()) } else { ("404 Not Found", "{}".to_string()) };
                let mut stream = stream;
                let _ = write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            }
        });
        port
    }

    fn settle(cx: &mut gpui::TestAppContext, until: impl Fn(&mut gpui::TestAppContext) -> bool) {
        for _ in 0..100 {
            cx.run_until_parked();
            if until(cx) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[gpui::test]
    fn a_job_shown_reads_its_graphs_and_rows_missed_read_them_again_in_its_own_book(cx: &mut gpui::TestAppContext) {
        let reads = Arc::new(AtomicUsize::new(0));
        let port = stand_in_fleet(reads.clone(), list("job-a", 0, 9));
        let fleet = gpui::AppContext::new(cx, |cx| crate::fleet::FleetConnection::idle(cx));
        fleet.update(cx, |fleet, cx| {
            fleet.connect_at(port, cx);
            fleet.read_graphs("job-a", cx);
            fleet.read_graphs("job-a", cx);
        });
        let held = |cx: &mut gpui::TestAppContext| cx.update(|cx| book("job-a", cx).and_then(|book| book.read(cx).rows("latency").map(<[Row]>::len)));
        settle(cx, |cx| held(cx) == Some(9));
        assert_eq!((held(cx), reads.load(Ordering::SeqCst)), (Some(9), 1), "read once when shown");

        // Another Job's rows land in its own book, and only once a view has asked for it.
        fleet.update(cx, |fleet, cx| fleet.fold_events(vec![Event::GraphsChanged(changed("job-b", "p99", 0, 3))], cx));
        assert!(cx.update(|cx| book("job-b", cx).is_none()));

        fleet.update(cx, |fleet, cx| fleet.fold_events(vec![Event::GraphsChanged(changed("job-a", "p99", 9, 10))], cx));
        assert_eq!(held(cx), Some(10), "a row that follows on is put on the end");
        fleet.update(cx, |fleet, cx| fleet.fold_events(vec![Event::GraphsChanged(changed("job-a", "p99", 12, 13))], cx));
        settle(cx, |_| reads.load(Ordering::SeqCst) == 2);
        settle(cx, |cx| held(cx) == Some(9));
        assert_eq!((reads.load(Ordering::SeqCst), held(cx)), (2, Some(9)), "rows went by unseen, so the book is read whole again");
    }
}
