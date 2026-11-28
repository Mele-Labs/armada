//! Graphs a Job puts in its thread, drawn natively from a Vega-Lite subset (`ipc::graph`).
//!
//! `compile` works a spec out on its rows, `plot` paints the result with gpui-component's plot
//! primitives, `svg` writes the same plot for a document's page, and `feeds` holds each Job's cards and the live rows a spec's `data.name`
//! reads, as Fleet's `list_graphs` and `graphs.changed` carry them. The card that frames a graph in the thread and the Graphs tab are the
//! Job view's (`job_view/graph_card.rs`).

pub mod compile;
pub mod feeds;
pub mod mock;
pub mod plot;
pub mod spark;
pub mod svg;
pub mod ticks;

use ipc::graph::Spec;
use ipc::{GraphDelta, GraphFedBy, GraphForm, GraphShown, GraphTone, MetricCredential};

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
    /// Why the recipe's last read gave no rows, said muted on the card. The rows it held still draw.
    pub broken: Option<String>,
    /// The credential the recipe names and this Mac's keychain does not hold.
    pub no_token: Option<MetricCredential>,
}

fn tone_of(tone: GraphTone) -> Tone {
    match tone {
        GraphTone::Worse => Tone::Worse,
        GraphTone::Better => Tone::Better,
        GraphTone::Steady => Tone::Steady,
    }
}

fn delta_of(delta: &GraphDelta) -> Delta {
    Delta { said: delta.said.clone(), since: delta.since.clone(), tone: tone_of(delta.tone) }
}

/// How often Fleet reads a recipe, in the words under a card's title: `every 15s`, `every minute`, `every 5 min`.
fn every(secs: u32) -> String {
    match secs {
        60 => "every minute".to_string(),
        3600 => "every hour".to_string(),
        86_400 => "every day".to_string(),
        secs if secs % 3600 == 0 => format!("every {} h", secs / 3600),
        secs if secs % 60 == 0 => format!("every {} min", secs / 60),
        secs => format!("every {secs}s"),
    }
}

/// The words under a card's title for how its rows arrive.
pub fn source_of(fed_by: GraphFedBy) -> String {
    match fed_by {
        GraphFedBy::Recipe { every_secs } => format!("Fleet reads {}", every(every_secs)),
        GraphFedBy::Points => "The agent sends its rows".to_string(),
    }
}

impl Card {
    /// The card of a graph as Fleet sends it, its words and changes as Fleet said them.
    pub fn of(shown: &GraphShown) -> Card {
        let form = match &shown.form {
            GraphForm::Chart { spec, headline } => Form::Chart {
                spec: spec.clone(),
                headline: headline.as_ref().map(|head| Headline { feed: head.feed.clone(), field: head.field.clone(), unit: head.unit.clone(), delta: head.delta.as_ref().map(delta_of) }),
            },
            GraphForm::Strip { stats } => Form::Strip(
                stats
                    .iter()
                    .map(|stat| Stat { label: stat.label.clone(), feed: stat.feed.clone(), field: stat.field.clone(), unit: stat.unit.clone(), shown: stat.shown.clone(), delta: stat.delta.as_ref().map(delta_of) })
                    .collect(),
            ),
        };
        // A missing token is said by its own note; the sentence Fleet keeps for it would say it twice.
        let broken = if shown.no_token.is_some() { None } else { shown.broken.clone() };
        Card { id: shown.id.clone(), title: shown.title.clone(), source: source_of(shown.fed_by), live: shown.live, form, broken, no_token: shown.no_token.clone() }
    }
}

#[cfg(test)]
mod tests {
    use ipc::{GraphFedBy, GraphForm, GraphShown, GraphStat, GraphTone, Instant, MetricCredential};

    use super::*;

    fn strip(fed_by: GraphFedBy) -> GraphShown {
        let delta = ipc::GraphDelta { said: "+2.1 pts".into(), since: Some("since deploy".into()), tone: GraphTone::Worse };
        let stat = GraphStat { label: "Error rate".into(), feed: "health".into(), field: "pct".into(), unit: Some("%".into()), shown: None, delta: Some(delta) };
        GraphShown { id: "health".into(), title: "checkout-api health".into(), form: GraphForm::Strip { stats: vec![stat] }, fed_by, live: false, shown_at: Instant::carried("2026-11-27T14:00:00Z"), read_at: None, broken: Some("The call timed out.".into()), no_token: None }
    }

    #[test]
    fn a_graph_fleet_sends_is_the_card_it_draws_its_words_as_fleet_said_them() {
        let card = Card::of(&strip(GraphFedBy::Recipe { every_secs: 15 }));
        assert_eq!((card.id.as_str(), card.title.as_str(), card.source.as_str(), card.live), ("health", "checkout-api health", "Fleet reads every 15s", false));
        assert_eq!(card.broken.as_deref(), Some("The call timed out."), "the reason is kept for the card to say");
        let Form::Strip(stats) = &card.form else { panic!("a strip") };
        let delta = stats[0].delta.as_ref().expect("its change");
        assert_eq!((stats[0].label.as_str(), stats[0].feed.as_str(), stats[0].unit.as_deref(), delta.said.as_str(), delta.since.as_deref(), delta.tone), ("Error rate", "health", Some("%"), "+2.1 pts", Some("since deploy"), Tone::Worse));

        let every = |secs| Card::of(&strip(GraphFedBy::Recipe { every_secs: secs })).source;
        assert_eq!([every(60), every(300), every(90), every(7200), every(86_400)], ["Fleet reads every minute", "Fleet reads every 5 min", "Fleet reads every 90s", "Fleet reads every 2 h", "Fleet reads every day"]);
        assert_eq!(Card::of(&strip(GraphFedBy::Points)).source, "The agent sends its rows");

        let mut tokenless = strip(GraphFedBy::Recipe { every_secs: 60 });
        tokenless.no_token = Some(MetricCredential { name: "grafana".into(), how: "A token".into(), url: None });
        let card = Card::of(&tokenless);
        assert_eq!((card.no_token.map(|one| one.name), card.broken), (Some("grafana".to_string()), None), "a missing token is its own note, not a broken line too");
    }
}
