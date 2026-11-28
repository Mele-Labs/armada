//! The graphs a page holds, drawn before the page reaches its web view, as its sketches are (`page_sketches.rs`).
//!
//! An `<armada-graph>` carries a Vega-Lite spec in its `<script type="application/json">`, its rows in `data.values`, or
//! names one of the Job's live graphs with `graph="<id>"`. Each is read with `ipc::graph::read_spec`, worked out by the
//! thread's own compiler and written as SVG by `graph::svg`, so it reads as the card does. A spec that does not read draws
//! a muted line saying why. A live graph is drawn again whenever the feeds' rows change: the page tab puts the new
//! drawing in place with a script in the kit's world (`live_script`), as it hands the kit Fleet's answers.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use gpui::App;
use ipc::graph::{read_spec, Spec, Title};

use super::page_sketches::{attribute, json_of, scan};
use crate::graph::compile::{self, Feeds};
use crate::graph::plot::Inks;
use crate::graph::{feeds, svg, Form};
use crate::look::{token, ActiveLook};
use crate::scene::export::xml;

/// How an `<armada-graph>` sits in the page: framed as a sketch is, its title above in the sketch's band, its legend
/// over the drawing, and each readout shown only while the pointer is over its column.
const STYLE: &str = "<style data-armada-graph>armada-graph{display:block;margin:var(--space-4) 0;border:var(--edge-active) solid var(--border-subtle);border-radius:var(--radius-lg);overflow:hidden;background:var(--bg-sunken)}armada-graph .graph-title{padding:var(--space-2) var(--space-3);font-size:var(--text-xs);color:var(--fg-muted)}armada-graph .graph-body{padding:var(--space-2) var(--space-4) var(--space-4)}armada-graph svg{display:block;inline-size:100%;block-size:auto;font-family:var(--font-sans)}armada-graph .graph-legend{display:flex;flex-wrap:wrap;align-items:center;gap:var(--space-1) var(--space-4);margin:0 0 var(--space-3);font-size:var(--text-xs);color:var(--fg-muted)}armada-graph .graph-key{display:inline-flex;align-items:center;gap:var(--space-2)}armada-graph .graph-key i{display:inline-block;flex:none}armada-graph .graph-key-line{inline-size:14px;block-size:2px;border-radius:1px}armada-graph .graph-key-fill{inline-size:10px;block-size:10px;border-radius:3px}armada-graph .graph-key-dot{inline-size:9px;block-size:9px;border-radius:50%}armada-graph .graph-donut{display:flex;flex-wrap:wrap;align-items:center;gap:var(--space-6)}armada-graph .graph-donut svg{inline-size:auto;flex:none}armada-graph .graph-shares{flex:1;min-inline-size:calc(var(--space-12) * 2 + var(--space-4));max-inline-size:calc(var(--space-12) * 4);display:flex;flex-direction:column;gap:var(--space-3)}armada-graph .graph-share{display:grid;grid-template-columns:1fr auto;align-items:center;font-size:var(--text-sm);color:var(--fg-muted)}armada-graph .graph-share b{font-size:var(--text-base);color:var(--fg-default)}armada-graph .graph-share small{grid-column:1 / -1;padding-inline-start:calc(var(--space-2) + 10px);font-size:var(--text-2xs);color:var(--fg-subtle)}armada-graph .graph-not{margin:0;font-size:var(--text-xs);color:var(--fg-muted)}armada-graph .graph-show{visibility:hidden}armada-graph .graph-at:hover .graph-show{visibility:visible}</style>";

/// What an `<armada-graph>` asks to be drawn.
#[derive(Debug, PartialEq)]
pub(super) enum Graph {
    /// A spec of its own, its rows inline: the text of its JSON script, where it has one.
    Inline(Option<String>),
    /// One of the Job's live graphs, by its id.
    Live(String),
}

/// `html` with each graph's content replaced by what `draw` makes of it: its place among the page's graphs, its title and
/// what it asks for. A graph `draw` answers `None` for is left as it stands. Pure, so it is read without a window.
pub(super) fn drawn_with(html: &str, mut draw: impl FnMut(usize, &str, Graph) -> Option<String>) -> String {
    let mut out = String::with_capacity(html.len());
    let mut at = 0;
    for (index, found) in scan(html, "armada-graph").into_iter().enumerate() {
        let title = attribute(found.tag, "title").unwrap_or_default();
        let graph = match attribute(found.tag, "graph").map(|id| id.trim().to_string()).filter(|id| !id.is_empty()) {
            Some(id) => Graph::Live(id),
            None => Graph::Inline(json_of(&found)),
        };
        let Some(drawing) = draw(index, title.trim(), graph) else { continue };
        out.push_str(&html[at..found.content]);
        out.push_str(&drawing);
        at = found.close;
    }
    out.push_str(&html[at..]);
    // The look goes in the page's head once, however often a live graph is drawn again.
    match out.to_ascii_lowercase().find("</head") {
        Some(head) if at > 0 && !out.contains("<style data-armada-graph>") => {
            out.insert_str(head, STYLE);
            out
        }
        _ => out,
    }
}

/// Whether the page holds a graph that reads the Job's live rows.
pub(super) fn holds_live(html: &str) -> bool {
    let mut live = false;
    drawn_with(html, |_, _, graph| {
        live |= matches!(graph, Graph::Live(_));
        None
    });
    live
}

/// The words of a graph that cannot be drawn, in place of it.
fn not_drawn(title: &str, why: &str) -> String {
    format!(r#"{}<div class="graph-body"><p class="graph-not" role="note">{}</p></div>"#, title_band(title), xml(why))
}

fn title_band(title: &str) -> String {
    if title.is_empty() { String::new() } else { format!(r#"<div class="graph-title">{}</div>"#, xml(title)) }
}

fn spec_title(spec: &Spec) -> Option<String> {
    spec.title.as_ref().map(|title| match title {
        Title::Text(text) => text.clone(),
        Title::Full { text, .. } => text.clone(),
    })
}

/// How large a page draws its graphs, in the page's points: a chart as wide as a page's column and as tall as the
/// thread's card, a scatter a little taller, and a donut as large as the card's.
#[derive(Clone, Copy)]
pub(super) struct Sizes {
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) scatter: f32,
    pub(super) donut: f32,
}

/// A spec worked out on its rows and written as the graph's content: its title, its legend and its drawing.
pub(super) fn figure(spec: &Spec, title: &str, feeds: &dyn Feeds, inks: &Inks, sizes: Sizes, key: &str) -> String {
    let chart = match compile::compile(spec, &[], feeds) {
        Ok(chart) => Rc::new(chart),
        Err(why) => return not_drawn(title, &why.to_string()),
    };
    let name = if title.is_empty() { "Chart".to_string() } else { format!("Chart: {title}") };
    let mut out = title_band(title);
    out.push_str(r#"<div class="graph-body">"#);
    if chart.arcs.is_some() {
        let drawing = svg::svg(chart.clone(), inks, sizes.donut, sizes.donut, &name, key);
        let _ = write!(out, r#"<div class="graph-donut">{drawing}{}</div>"#, svg::shares(&chart, inks).unwrap_or_default());
    } else {
        out.push_str(&svg::legend(&chart, inks).unwrap_or_default());
        let height = if chart.scatter { sizes.scatter } else { sizes.height };
        out.push_str(&svg::svg(chart, inks, sizes.width, height, &name, key));
    }
    out.push_str("</div>");
    out
}

/// What one graph draws: a spec of its own, or the Job's live graph of its id, or why it draws neither.
pub(super) fn drawing(at: usize, title: &str, graph: Graph, cards: &[Rc<crate::graph::Card>], feeds: &dyn Feeds, inks: &Inks, sizes: Sizes) -> String {
    let key = format!("armada-graph-{at}");
    match graph {
        Graph::Inline(None) => not_drawn(title, "The graph holds no spec: put Vega-Lite in a <script type=\"application/json\"> inside it"),
        Graph::Inline(Some(text)) => match read_spec(&text) {
            Ok(spec) => {
                let title = if title.is_empty() { spec_title(&spec).unwrap_or_default() } else { title.to_string() };
                figure(&spec, &title, feeds, inks, sizes, &key)
            }
            Err(why) => not_drawn(title, &why.to_string()),
        },
        Graph::Live(id) => match cards.iter().find(|card| card.id == id) {
            None => not_drawn(title, &format!("This Job has no graph {id}")),
            Some(card) => {
                let title = if title.is_empty() { card.title.as_str() } else { title };
                match &card.form {
                    Form::Chart { spec, .. } => figure(spec, title, feeds, inks, sizes, &key),
                    Form::Strip(_) => not_drawn(title, "A stat strip draws in the thread, not in a page"),
                }
            }
        },
    }
}

fn draw_page(html: &str, job: &str, live_only: bool, cx: &App) -> String {
    if !html.to_ascii_lowercase().contains("<armada-graph") {
        return html.to_string();
    }
    let look = cx.look();
    let mut inks = Inks::of(look);
    // A dot's ring is the frame's own ground, as the card's is its own.
    inks.surface = look.color(token::BG_SUNKEN);
    let sizes = Sizes {
        width: f32::from(look.length(token::SPACE_12)) * 14.,
        height: f32::from(look.length(token::SPACE_12)) * 4.,
        scatter: f32::from(look.length(token::SPACE_12)) * 4.5,
        donut: f32::from(look.length(token::SPACE_8)) * 4.,
    };
    let none: BTreeMap<String, Vec<ipc::graph::Row>> = BTreeMap::new();
    let book = feeds::book(cx);
    let rows: &dyn Feeds = match &book {
        Some(book) => book.read(cx),
        None => &none,
    };
    let cards = feeds::of_job(job, cx);
    drawn_with(html, |at, title, graph| {
        if live_only && !matches!(graph, Graph::Live(_)) {
            return None;
        }
        Some(drawing(at, title, graph, &cards, rows, &inks, sizes))
    })
}

/// The page with its graphs drawn, live ones from the rows Bridge holds for `job`.
pub(super) fn drawn(html: &str, job: &str, cx: &App) -> String {
    draw_page(html, job, false, cx)
}

/// A drawn page with its live graphs drawn again on the rows as they stand now.
pub(super) fn redrawn(html: &str, job: &str, cx: &App) -> String {
    draw_page(html, job, true, cx)
}

/// The script that puts each live graph of a drawn page in place in the page as it is shown, by its place among the
/// page's graphs. Run in the kit's world, which reaches the page's elements and is not held to its policy.
pub(super) fn live_script(html: &str) -> String {
    let mut sets = String::new();
    for (at, found) in scan(html, "armada-graph").into_iter().enumerate() {
        if attribute(found.tag, "graph").is_some_and(|id| !id.trim().is_empty()) {
            let content = serde_json::to_string(found.inner).unwrap_or_else(|_| "\"\"".into());
            let _ = write!(sets, "if(all[{at}])all[{at}].innerHTML={content};");
        }
    }
    format!("(()=>{{const all=document.querySelectorAll('armada-graph');{sets}}})();undefined")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{"title":"Builds","data":{"values":[{"day":"Mon","n":3},{"day":"Tue","n":5}]},"mark":"bar","encoding":{"x":{"field":"day","type":"nominal"},"y":{"field":"n","type":"quantitative"}}}"#;

    fn page(graphs: &str) -> String {
        format!("<html><head><meta http-equiv=\"x\"></head><body><h1>T</h1>{graphs}<p>After.</p></body></html>")
    }

    fn inks() -> Inks {
        let grey = |l: f32| gpui::hsla(0., 0., l, 1.);
        Inks {
            series: [0.6, 0.1, 0.3, 0.8, 0.45, 0.95].map(|h| gpui::hsla(h, 0.6, 0.5, 1.)),
            other: grey(0.5),
            tick: grey(0.6),
            label: grey(0.7),
            strong: grey(0.9),
            grid: grey(0.2),
            axis: grey(0.3),
            rule: grey(0.6),
            marker: grey(0.4),
            surface: grey(0.05),
            pill: grey(0.12),
            font: gpui::px(12.),
            big: gpui::px(20.),
        }
    }

    const SIZES: Sizes = Sizes { width: 600., height: 190., scatter: 210., donut: 128. };

    #[test]
    fn an_inline_graph_and_a_live_one_are_told_apart_and_the_page_around_them_is_kept() {
        let html = page(&format!(r#"<armada-graph title="Builds"><script type="application/json">{SPEC}</script></armada-graph><armada-graph graph="graph-p99"></armada-graph><armada-graphs>x</armada-graphs>"#));
        let mut seen = Vec::new();
        let out = drawn_with(&html, |at, title, graph| {
            seen.push((at, title.to_string(), graph));
            Some(format!("<svg>{at}</svg>"))
        });
        assert_eq!(seen, [(0, "Builds".to_string(), Graph::Inline(Some(SPEC.to_string()))), (1, String::new(), Graph::Live("graph-p99".into()))]);
        assert!(out.contains(r#"<armada-graph title="Builds"><svg>0</svg></armada-graph><armada-graph graph="graph-p99"><svg>1</svg></armada-graph><armada-graphs>x</armada-graphs><p>After.</p>"#), "{out}");
        assert!(!out.contains("application/json"), "the spec is gone from the element");
        assert!(out.find("<style data-armada-graph>") < out.find("</head>"), "its look is in the head");
        assert!(holds_live(&html) && !holds_live(&page(&format!(r#"<armada-graph><script type="application/json">{SPEC}</script></armada-graph>"#))));
    }

    #[test]
    fn a_page_drawn_again_takes_the_look_once_and_a_page_without_graphs_is_left_as_written() {
        let html = page(r#"<armada-graph graph="a"></armada-graph>"#);
        let once = drawn_with(&html, |_, _, _| Some("<svg/>".into()));
        let twice = drawn_with(&once, |_, _, _| Some("<svg/>".into()));
        assert_eq!(twice.matches("<style data-armada-graph>").count(), 1);
        assert_eq!(once, twice);
        let plain = page("<p>No graph.</p>");
        assert_eq!(drawn_with(&plain, |_, _, _| unreachable!()), plain);
    }

    #[test]
    fn an_inline_spec_draws_as_svg_with_its_own_title_and_one_that_does_not_read_says_why() {
        let none: BTreeMap<String, Vec<ipc::graph::Row>> = BTreeMap::new();
        let drawn = drawing(0, "", Graph::Inline(Some(SPEC.into())), &[], &none, &inks(), SIZES);
        assert!(drawn.starts_with(r#"<div class="graph-title">Builds</div><div class="graph-body"><svg"#), "{drawn}");
        assert!(drawn.contains(r#"aria-label="Chart: Builds""#) && drawn.contains(">Mon</text>"), "{drawn}");
        let refused = drawing(1, "Bad", Graph::Inline(Some(r#"{"mark":"sankey"}"#.into())), &[], &none, &inks(), SIZES);
        let why = read_spec(r#"{"mark":"sankey"}"#).expect_err("a sankey is not drawn").to_string();
        assert_eq!(refused, format!(r#"<div class="graph-title">Bad</div><div class="graph-body"><p class="graph-not" role="note">{}</p></div>"#, xml(&why)));
        assert!(drawing(2, "", Graph::Inline(Some("{".into())), &[], &none, &inks(), SIZES).contains("The graph is not JSON"));
        assert!(drawing(3, "", Graph::Inline(None), &[], &none, &inks(), SIZES).contains("The graph holds no spec"));
    }

    #[test]
    fn a_live_graph_draws_its_card_on_the_rows_held_and_names_what_it_cannot_draw() {
        let spec = read_spec(r#"{"data":{"name":"feed"},"mark":"line","encoding":{"x":{"field":"at","type":"temporal"},"y":{"field":"ms","type":"quantitative","title":"p99 (ms)"}}}"#).expect("reads");
        let card = |id: &str, form: Form| Rc::new(crate::graph::Card { id: id.into(), title: "p99 latency".into(), source: String::new(), live: true, form, after: String::new() });
        let cards = [card("graph-p99", Form::Chart { spec, headline: None }), card("graph-health", Form::Strip(Vec::new()))];
        let row = |at: &str, ms: f64| serde_json::json!({"at": at, "ms": ms}).as_object().cloned().unwrap_or_default();
        let mut rows = BTreeMap::new();
        rows.insert("feed".to_string(), vec![row("2026-11-10T14:00:00Z", 200.), row("2026-11-10T14:01:00Z", 260.)]);
        let drawn = drawing(0, "", Graph::Live("graph-p99".into()), &cards, &rows, &inks(), SIZES);
        assert!(drawn.contains(r#"<div class="graph-title">p99 latency</div>"#) && drawn.contains(">260 ms</text>"), "{drawn}");
        let empty: BTreeMap<String, Vec<ipc::graph::Row>> = BTreeMap::new();
        assert!(drawing(0, "", Graph::Live("graph-p99".into()), &cards, &empty, &inks(), SIZES).contains("No rows yet from feed"));
        assert!(drawing(0, "", Graph::Live("graph-gone".into()), &cards, &rows, &inks(), SIZES).contains("This Job has no graph graph-gone"));
        assert!(drawing(0, "", Graph::Live("graph-health".into()), &cards, &rows, &inks(), SIZES).contains("A stat strip draws in the thread"));
    }

    #[test]
    fn the_live_script_puts_only_the_live_graphs_in_place_by_their_place_on_the_page() {
        let html = page(r#"<armada-graph><svg>static</svg></armada-graph><armada-graph graph="a"><svg>"new"</svg></armada-graph>"#);
        let script = live_script(&html);
        assert_eq!(script, r#"(()=>{const all=document.querySelectorAll('armada-graph');if(all[1])all[1].innerHTML="<svg>\"new\"</svg>";})();undefined"#);
    }
}
