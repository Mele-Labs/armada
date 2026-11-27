//! The live rows each named feed holds, and the graph cards a Job's thread carries. One entity holds
//! the rows: a card observes it and draws again in place when a feed's rows change, and nothing else
//! does. On mock data the walk's scenario fills it (`mock.rs`); Fleet's feed is not on the wire yet.

use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{App, Context, Entity, Global};
use ipc::graph::Row;

use super::Card;

#[derive(Default)]
pub struct FeedBook {
    feeds: BTreeMap<String, Vec<Row>>,
    /// How many rows each feed keeps: as many as it opened with.
    keep: BTreeMap<String, usize>,
}

impl FeedBook {
    pub fn new(feeds: BTreeMap<String, Vec<Row>>) -> FeedBook {
        let keep = feeds.iter().map(|(name, rows)| (name.clone(), rows.len())).collect();
        FeedBook { feeds, keep }
    }

    pub fn rows(&self, name: &str) -> Option<&[Row]> {
        self.feeds.get(name).map(Vec::as_slice)
    }

    /// Put a batch of rows on the end of their feeds, each keeping as many as it opened with, and draw once.
    pub fn push(&mut self, batch: Vec<(String, Row)>, cx: &mut Context<Self>) {
        if batch.is_empty() {
            return;
        }
        for (name, row) in batch {
            let keep = self.keep.get(&name).copied().unwrap_or(usize::MAX);
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

/// The graphs of the Job a scenario shows, by Job, and the rows their feeds hold.
pub struct Graphs {
    pub book: Entity<FeedBook>,
    pub by_job: BTreeMap<String, Vec<Rc<Card>>>,
    /// What feeds the book while it is held.
    pub _tick: gpui::Task<()>,
}

impl Global for Graphs {}

/// The graph cards of `job` that stand after the thread item `after`, in order.
pub fn after(job: &str, after: &str, cx: &App) -> Vec<Rc<Card>> {
    cx.try_global::<Graphs>().and_then(|graphs| graphs.by_job.get(job)).map(|cards| cards.iter().filter(|card| card.after == after).cloned().collect()).unwrap_or_default()
}

/// Every graph card of `job`, in thread order.
pub fn of_job(job: &str, cx: &App) -> Vec<Rc<Card>> {
    cx.try_global::<Graphs>().and_then(|graphs| graphs.by_job.get(job)).cloned().unwrap_or_default()
}

pub fn book(cx: &App) -> Option<Entity<FeedBook>> {
    cx.try_global::<Graphs>().map(|graphs| graphs.book.clone())
}
