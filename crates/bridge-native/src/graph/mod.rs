//! Graphs a Job puts in its thread, drawn natively from a Vega-Lite subset (`ipc::graph`).
//!
//! `compile` works a spec out on its rows, `plot` paints the result with gpui-component's plot
//! primitives, `svg` writes the same plot for a document's page, and `feeds` holds the live rows a spec's `data.name` reads. The card that frames a
//! graph in the thread and the Graphs tab are the Job view's (`job_view/graph_card.rs`).

pub mod compile;
pub mod feeds;
pub mod mock;
pub mod plot;
pub mod spark;
pub mod svg;
pub mod ticks;

use ipc::graph::Spec;

/// Whether a value going up is good news, which colours its change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Worse,
    Better,
    Steady,
}

/// How a figure moved, as Fleet words it: `+41%`, and what it is measured against, `since deploy`.
#[derive(Clone, Debug)]
pub struct Delta {
    pub said: String,
    pub since: Option<String>,
    pub tone: Tone,
}

/// The latest value of a feed's field, shown large over the chart.
#[derive(Clone, Debug)]
pub struct Headline {
    pub feed: String,
    pub field: String,
    pub unit: Option<String>,
    pub delta: Option<Delta>,
}

/// One number of a stat strip: its words, the feed and field its sparkline and value read, and how
/// the value is written where it is not the field's latest (`12 of 12`).
#[derive(Clone, Debug)]
pub struct Stat {
    pub label: String,
    pub feed: String,
    pub field: String,
    pub unit: Option<String>,
    pub shown: Option<String>,
    pub delta: Option<Delta>,
}

#[derive(Clone, Debug)]
pub enum Form {
    /// A chart drawn from its spec, with an optional headline over it.
    Chart { spec: Spec, headline: Option<Headline> },
    /// A row per number, each with its sparkline, value and change.
    Strip(Vec<Stat>),
}

/// A graph card as the thread holds it: what it is called, where Fleet reads it from, and what it draws.
#[derive(Clone, Debug)]
pub struct Card {
    pub id: String,
    pub title: String,
    /// The words under the title: `Fleet reads every 15s`.
    pub source: String,
    /// Fleet is still reading its feeds.
    pub live: bool,
    pub form: Form,
    /// The thread item it stands after.
    pub after: String,
}
