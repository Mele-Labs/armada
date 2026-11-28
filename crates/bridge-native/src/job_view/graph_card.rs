//! A graph in a Job's thread: a card with a live dot, the title and where Fleet reads it from, the
//! latest value large where the graph has a headline, and the chart; or, as a stat strip, a row per
//! number with its sparkline. Each card is an entity of its own that watches its Job's book, so a new
//! row or a card Fleet sent again draws that card again in place and nothing else. The Graphs tab
//! holds the same cards larger.
//!
//! The thread draws a card where its `show_graph` call stands, its `graph` item; a later call of the
//! same id is marked `again` and draws nothing, since the card at the first already shows it. A card
//! whose recipe broke says why, muted, over the rows it kept, and one whose credential has no token
//! on this Mac says what to make and where, as a project metric's block does.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, Context, Entity, EntityId, FocusHandle, FontWeight, Global, Hsla, Role, SharedString, Subscription, WeakEntity, Window};
use ipc::{MetricCredential, ThreadGraph};

use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::graph::compile::{self, Chart, Key};
use crate::graph::feeds::{self, FeedBook};
use crate::graph::plot::{GraphPlot, Inks};
use crate::graph::spark::Spark;
use crate::graph::{ticks, Card, Delta, Form, Tone};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};

/// Where a card is drawn: in the thread, or larger in the Graphs tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Thread,
    Tab,
}

/// What a card draws, worked out when its rows change and read as it draws.
enum Drawn {
    Chart { chart: Rc<Chart>, latest: Option<f64> },
    Strip(Vec<StatDrawn>),
    Waiting(String),
}

struct StatDrawn {
    values: Vec<f64>,
    shown: String,
}

pub struct GraphCard {
    card: Rc<Card>,
    place: Place,
    drawn: Drawn,
    view: WeakEntity<JobView>,
    pub focus: FocusHandle,
    /// Watches its Job's book, which holds the card as Fleet last sent it and the rows its feeds hold.
    _watch: Option<Subscription>,
}

impl GraphCard {
    fn new(card: Rc<Card>, book: Option<Entity<FeedBook>>, place: Place, view: WeakEntity<JobView>, cx: &mut Context<Self>) -> GraphCard {
        let watch = book.as_ref().map(|book| {
            cx.observe(book, |this: &mut GraphCard, book, cx| {
                // Fleet sends a Job's cards whole each time, so its words, live dot and changes are read again too.
                if let Some(card) = book.read(cx).card(&this.card.id) {
                    this.card = card;
                }
                this.drawn = Self::work_out(&this.card, &book, cx);
                cx.notify();
            })
        });
        let drawn = match &book {
            Some(book) => Self::work_out(&card, book, cx),
            None => Drawn::Waiting("No rows yet".into()),
        };
        GraphCard { card, place, drawn, view, focus: cx.focus_handle().tab_stop(true), _watch: watch }
    }

    fn work_out(card: &Card, book: &Entity<FeedBook>, cx: &App) -> Drawn {
        let book = book.read(cx);
        match &card.form {
            Form::Chart { spec, headline } => match compile::compile(spec, &[], book) {
                Ok(chart) => {
                    let latest = headline.as_ref().and_then(|head| book.rows(&head.feed)?.iter().rev().find_map(|row| row.get(&head.field).and_then(compile::number)));
                    Drawn::Chart { chart: Rc::new(chart), latest }
                }
                Err(why) => Drawn::Waiting(why.to_string()),
            },
            Form::Strip(stats) => Drawn::Strip(
                stats
                    .iter()
                    .map(|stat| {
                        let values: Vec<f64> = book.rows(&stat.feed).unwrap_or(&[]).iter().filter_map(|row| row.get(&stat.field).and_then(compile::number)).collect();
                        let shown = stat.shown.clone().unwrap_or_else(|| values.last().map_or_else(String::new, |value| ticks::with_unit(*value, stat.unit.as_deref())));
                        StatDrawn { values, shown }
                    })
                    .collect(),
            ),
        }
    }

    fn open_in_tab(&self, window: &mut Window, cx: &mut App) {
        if let Some(view) = self.view.upgrade() {
            view.update(cx, |view, cx| view.open_graphs(window, cx));
        }
    }
}

fn tone_ink(tone: Tone) -> Token {
    match tone {
        Tone::Worse => token::STATUS_AWAITING_REVIEW,
        Tone::Better => token::STATUS_COMPLETED_SUCCESS,
        Tone::Steady => token::FG_MUTED,
    }
}

/// A change as a chip: its arrow and words in the tone's hue on a tint of it.
fn delta_chip(look: &Look, id: String, delta: &Delta) -> AnyElement {
    let ink = look.color(tone_ink(delta.tone));
    let said = match delta.tone {
        Tone::Worse => "Worse than before",
        Tone::Better => "As it should be",
        Tone::Steady => "Moved, neither better nor worse",
    };
    // The arrow follows the figure's sign; the hue says whether that is good news.
    let mark = match delta.said.chars().next() {
        Some('+') => Some("trending-up"),
        Some('-' | '\u{2212}') => Some("trending-down"),
        _ if delta.tone == Tone::Better => Some("check"),
        _ => None,
    };
    let words = match &delta.since {
        Some(since) => format!("{} {since}", delta.said),
        None => delta.said.clone(),
    };
    labelled(
        div()
            .id(SharedString::from(id))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::SPACE_6) - look.length(token::BORDER_WIDTH) * 2.)
            .px(look.length(token::SPACE_2))
            .rounded_full()
            .bg(ink.opacity(0.14))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .tooltip(tip(said))
            .children(mark.map(|mark| glyph(look, mark, Size::Small, ink)))
            .child(SharedString::from(words.clone())),
        Role::Status,
        words,
    )
    .into_any_element()
}

/// The key a legend and a readout give a series: a stroke for a line, a square for a fill, a dot for a point.
fn key_mark(key: Key, ink: Hsla) -> gpui::Div {
    match key {
        Key::Line => div().flex_none().w(px(14.)).h(px(2.)).rounded(px(1.)).bg(ink),
        Key::Fill => div().flex_none().size(px(10.)).rounded(px(3.)).bg(ink),
        Key::Dot => div().flex_none().size(px(9.)).rounded_full().bg(ink),
    }
}

impl GraphCard {
    fn header(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let id = &self.card.id;
        let (ink, said) = if self.card.live { (look.color(token::STATUS_RUNNING), "Live: Fleet is reading it") } else { (look.color(token::STATUS_NOT_STARTED), "Fleet stopped reading it") };
        let dot = labelled(
            div()
                .id(SharedString::from(format!("graph-live:{id}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::SPACE_4))
                .rounded_full()
                .bg(ink.opacity(0.18))
                .tooltip(tip(said))
                .child(div().size(look.length(token::SPACE_2)).rounded_full().bg(ink)),
            Role::Image,
            said,
        );
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .truncate()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(self.card.title.clone())),
            )
            .child(
                div()
                    .truncate()
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(SharedString::from(self.card.source.clone())),
            );
        let open = (self.place == Place::Thread).then(|| {
            let name = format!("Open {} in Graphs", self.card.title);
            labelled(
                div()
                    .id(SharedString::from(format!("graph-open:{id}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_color(look.color(token::FG_MUTED))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .tooltip(tip("Open in Graphs"))
                    .child(glyph(look, "maximize-2", Size::Small, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(|this, _, window, cx| this.open_in_tab(window, cx))),
                Role::Button,
                name,
            )
        });
        div().flex().items_center().gap(look.length(token::SPACE_3)).child(dot).child(words).children(open).into_any_element()
    }

    fn headline(&self, look: &Look) -> Option<AnyElement> {
        let (Form::Chart { headline: Some(head), .. }, Drawn::Chart { latest: Some(value), .. }) = (&self.card.form, &self.drawn) else { return None };
        let id = &self.card.id;
        let size = if self.place == Place::Tab { token::TEXT_2XL } else { token::TEXT_XL };
        let shown = ticks::grouped(*value);
        let said = match &head.unit {
            Some(unit) => format!("{}, {shown} {unit}", self.card.title),
            None => format!("{}, {shown}", self.card.title),
        };
        let row = div()
            .id(SharedString::from(format!("graph-headline:{id}")))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(look.length(token::SPACE_1))
                    .child(div().flex_none().text_size(look.length(size)).line_height(look.length(size) * 1.15).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(shown)))
                    .children(head.unit.clone().map(|unit| div().flex_none().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(unit)))),
            )
            .children(head.delta.as_ref().map(|delta| delta_chip(look, format!("graph-delta:{id}"), delta)));
        Some(labelled(row, Role::Group, said).into_any_element())
    }

    fn legend(&self, chart: &Chart, inks: &Inks, look: &Look) -> Option<AnyElement> {
        if !chart.legend() || chart.arcs.is_some() {
            return None;
        }
        let items = chart.series.iter().map(|series| {
            labelled(
                div()
                    .id(SharedString::from(format!("graph-key:{}:{}", self.card.id, series.name)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(key_mark(series.key, inks.of_series(series.slot)))
                    .child(SharedString::from(series.name.clone())),
                Role::Label,
                series.name.clone(),
            )
        });
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(look.length(token::SPACE_4))
                .gap_y(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .children(items)
                .into_any_element(),
        )
    }

    fn chart(&self, chart: &Rc<Chart>, inks: &Inks, look: &Look) -> AnyElement {
        let id = &self.card.id;
        let place = match self.place {
            Place::Thread => "thread",
            Place::Tab => "tab",
        };
        let plot = GraphPlot::new(SharedString::from(format!("graph-plot:{place}:{id}")), chart.clone(), inks.clone());
        let tall = match (self.place, chart.scatter) {
            (Place::Thread, false) => look.length(token::SPACE_12) * 4.,
            (Place::Thread, true) => look.length(token::SPACE_12) * 4.5,
            (Place::Tab, _) => look.length(token::SPACE_12) * 6.5,
        };
        let drawing = labelled(div().id(SharedString::from(format!("graph-chart:{place}:{id}"))).w_full().h(tall).child(plot), Role::Image, format!("Chart: {}", self.card.title));
        let Some(arcs) = &chart.arcs else { return drawing.into_any_element() };
        // A donut stands at the left and its legend at its right, each slice with its share.
        let whole: f64 = arcs.slices.iter().map(|(_, value)| *value).sum();
        let side = look.length(token::SPACE_8) * 4.;
        let rows = arcs.slices.iter().map(|(series, value)| {
            let one = &chart.series[*series];
            let share = if whole > 0. { value / whole * 100. } else { 0. };
            let share = format!("{}%", ticks::grouped((share * 10.).round() / 10.));
            let named = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(key_mark(Key::Fill, inks.of_series(one.slot)))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(one.name.clone())))
                .child(div().flex_none().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(share.clone())));
            // The slice's own figure stands under its name, in line with the name rather than the key.
            let figure = div()
                .pl(look.length(token::SPACE_2) + px(10.))
                .truncate()
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(SharedString::from(ticks::with_unit(*value, arcs.unit.as_deref())));
            labelled(
                div()
                    .id(SharedString::from(format!("graph-slice:{id}:{}", one.name)))
                    .flex()
                    .flex_col()
                    .child(named)
                    .child(figure),
                Role::Label,
                format!("{}, {share}", one.name),
            )
        });
        // Where the card is narrow, the legend goes under the donut rather than squeezing beside it.
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_6))
            .child(labelled(div().id(SharedString::from(format!("graph-chart:{place}:{id}"))).flex_none().size(side).child(GraphPlot::new(SharedString::from(format!("graph-plot:{place}:{id}")), chart.clone(), inks.clone())), Role::Image, format!("Chart: {}", self.card.title)))
            .child(div().flex_1().min_w(look.length(token::SPACE_12) * 2. + look.length(token::SPACE_4)).max_w(look.length(token::SPACE_12) * 4.).flex().flex_col().gap(look.length(token::SPACE_3)).children(rows))
            .into_any_element()
    }

    fn strip(&self, stats: &[StatDrawn], inks: &Inks, look: &Look) -> AnyElement {
        let Form::Strip(defs) = &self.card.form else { return div().into_any_element() };
        let id = &self.card.id;
        let rows = defs.iter().zip(stats).enumerate().map(|(at, (def, drawn))| {
            let spark = Spark::new(drawn.values.clone(), inks.of_series(0), inks.surface);
            let said = format!("{}, {}", def.label, drawn.shown);
            let figure = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1) / 2.)
                .child(div().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(def.label.clone())))
                .child(
                    // Where the row is narrow, the change goes under the value rather than over the sparkline.
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x(look.length(token::SPACE_2))
                        .gap_y(look.length(token::SPACE_1))
                        .child(
                            div()
                                .flex_none()
                                .text_size(look.length(token::TEXT_LG))
                                .line_height(look.length(token::LEADING_LG))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_DEFAULT))
                                .child(SharedString::from(drawn.shown.clone())),
                        )
                        .children(def.delta.as_ref().map(|delta| delta_chip(look, format!("graph-stat-delta:{id}:{at}"), delta))),
                );
            let line = div()
                .id(SharedString::from(format!("graph-stat:{id}:{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .child(figure)
                .child(div().flex_none().w(look.length(token::SPACE_12) * 1.5).h(look.length(token::SPACE_8) + look.length(token::SPACE_1)).child(spark));
            labelled(line, Role::Row, said)
        });
        div().flex().flex_col().children(rows).into_any_element()
    }
}

impl GraphCard {
    /// The credential the recipe needs and this Mac lacks: what to make, and a link to where it is made.
    fn token_note(&self, credential: &MetricCredential, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let id = &self.card.id;
        let missing = format!("No {} token on this Mac", credential.name);
        let make = credential.url.clone().map(|url| {
            labelled(
                div()
                    .id(SharedString::from(format!("graph-token-make:{id}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL))
                    // The link's words stand in line with the sentences above it; its hover ground reaches past them.
                    .px(look.length(token::SPACE_2))
                    .ml(-look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_color(look.color(token::FG_MUTED))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .tooltip(tip(format!("Open {url}")))
                    .child(glyph(look, "external-link", Size::Small, look.color(token::FG_MUTED)))
                    .child("Make one")
                    .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                Role::Link,
                format!("Make a {} token", credential.name),
            )
        });
        // What to make, then where: the link goes under the words, so a narrow card keeps the sentences whole.
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .items_start()
            .gap(look.length(token::SPACE_1) / 2.)
            .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(missing.clone())))
            .child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(credential.how.clone())))
            .children(make);
        let note = div()
            .id(SharedString::from(format!("graph-token:{id}")))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::DEGRADED).opacity(0.10))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph(look, "key-round", Size::Small, look.color(token::DEGRADED))))
            .child(words);
        labelled(note, Role::Status, format!("{missing}. {}", credential.how)).into_any_element()
    }

    /// Why the recipe's last read gave nothing, muted: the rows it kept still draw under it.
    fn broken_line(&self, why: &str, look: &Look) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("graph-broken:{}", self.card.id)))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph(look, "unplug", Size::Small, look.color(token::FG_MUTED))))
                .child(div().flex_1().min_w_0().child(SharedString::from(why.to_string()))),
            Role::Status,
            format!("Not read: {why}"),
        )
        .into_any_element()
    }
}

impl gpui::Render for GraphCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let look = &look;
        let inks = Inks::of(look);
        let focused = self.focus.is_focused(window);
        let body = match &self.drawn {
            Drawn::Chart { chart, .. } => {
                let chart = chart.clone();
                div().flex().flex_col().gap(look.length(token::SPACE_3)).children(self.headline(look)).children(self.legend(&chart, &inks, look)).child(self.chart(&chart, &inks, look)).into_any_element()
            }
            Drawn::Strip(stats) => self.strip(stats, &inks, look),
            Drawn::Waiting(why) => div().h(look.length(token::SPACE_12) * 2.).flex().items_center().justify_center().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(why.clone())).into_any_element(),
        };
        let header = self.header(look, cx);
        let note = match (&self.card.no_token, &self.card.broken) {
            (Some(credential), _) => Some(self.token_note(credential, look, cx)),
            (None, Some(why)) => Some(self.broken_line(why, look)),
            (None, None) => None,
        };
        // A card with no token and no rows yet is the note alone: its empty chart would only say the same.
        let body = (!(self.card.no_token.is_some() && matches!(self.drawn, Drawn::Waiting(_)))).then_some(body);
        let said = format!("Graph: {}", self.card.title);
        let card = div()
            .id(SharedString::from(format!("graph-card:{:?}:{}", self.place, self.card.id)))
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .min_w_0()
            .px(look.length(token::SPACE_4) + look.length(token::SPACE_1))
            .pt(look.length(token::SPACE_4))
            .pb(look.length(token::SPACE_4))
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if focused { token::ACCENT } else { token::BORDER_DEFAULT }))
            .shadow(look.shadows(token::SHADOW_CARD))
            .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                // Enter on a card in the thread opens it larger in the Graphs tab.
                if event.keystroke.key == "enter" && this.place == Place::Thread {
                    this.open_in_tab(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(header)
            .children(note)
            .children(body);
        labelled(card, Role::Article, said)
    }
}

/// Each Job view's graph cards, by Job, place and card.
#[derive(Default)]
struct Held {
    cards: HashMap<(EntityId, String, Place), HashMap<String, Entity<GraphCard>>>,
}

impl Global for Held {}

impl JobView {
    /// Ask Fleet for the Job's graphs once it is on screen; `graphs.changed` keeps them after.
    pub(super) fn want_graphs(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        if self.shown.as_ref() != Some(id) {
            return;
        }
        if let Some(fleet) = self.fleet.clone() {
            fleet.update(cx, |fleet, cx| fleet.read_graphs(id, cx));
        }
    }

    /// Make the cards of `job` for `place` that are not made yet, as the Job draws: a card is an entity,
    /// and the thread's items are drawn where none can be made.
    pub(super) fn sync_graphs(&self, job: &SharedString, place: Place, cx: &mut Context<Self>) {
        let cards = feeds::of_job(job, cx);
        let key = (cx.entity_id(), job.to_string(), place);
        if cards.is_empty() {
            if cx.try_global::<Held>().is_some_and(|held| held.cards.contains_key(&key)) {
                cx.default_global::<Held>().cards.remove(&key);
            }
            return;
        }
        let weak = cx.entity().downgrade();
        let book = feeds::book(job, cx);
        let mut held = cx.default_global::<Held>().cards.remove(&key).unwrap_or_default();
        held.retain(|id, _| cards.iter().any(|card| card.id == *id));
        for card in cards {
            if !held.contains_key(&card.id) {
                let id = card.id.clone();
                let made = cx.new(|cx| GraphCard::new(card, book.clone(), place, weak.clone(), cx));
                held.insert(id, made);
            }
        }
        cx.default_global::<Held>().cards.insert(key, held);
    }

    fn held_graph(&self, job: &str, place: Place, id: &str, cx: &Context<Self>) -> Option<Entity<GraphCard>> {
        cx.try_global::<Held>()?.cards.get(&(cx.entity_id(), job.to_string(), place))?.get(id).cloned()
    }

    /// A `show_graph` call's place in the thread: the card of the graph it names, nothing for a later call of the same
    /// id, and the call's line where Fleet holds no graph by that id, as for a call it refused.
    pub(super) fn graph_item(&self, item: &ThreadGraph, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if item.again {
            return None;
        }
        let job = self.shown.as_deref()?;
        if let Some(card) = self.held_graph(job, Place::Thread, &item.graph, cx) {
            return Some(div().pt(look.length(token::SPACE_1)).child(card).into_any_element());
        }
        // Until Fleet has answered, a graph it holds is not drawn as one it lacks.
        if !feeds::is_read(job, cx) {
            return None;
        }
        let said = format!("show_graph {}", item.graph);
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("graph-call:{}", item.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip("Fleet holds no graph by this id"))
                    .child(glyph(look, "chart-line", Size::Small, look.color(token::FG_SUBTLE)))
                    .child(div().font_family(mono(look)).child(SharedString::from(said.clone()))),
                Role::Label,
                said,
            )
            .into_any_element(),
        )
    }

    /// The Graphs tab's body: every graph of the Job, larger, one under the other.
    pub(super) fn graphs_body(&self, job: &SharedString, tab: &super::pane_tree::Tab, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.sync_graphs(job, Place::Tab, cx);
        let look = cx.look().clone();
        let cards: Vec<Entity<GraphCard>> = feeds::of_job(job, cx).iter().filter_map(|card| self.held_graph(job, Place::Tab, &card.id, cx)).collect();
        let list = div()
            .id(SharedString::from(format!("pane-body:{}", tab.id)))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(cards);
        Some(labelled(list, Role::Region, "Graphs").into_any_element())
    }

    /// Open the Graphs tab of the Job shown: the one it has comes to the front, and none makes one.
    pub(super) fn open_graphs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let open = self.arrangement_of(&job).and_then(|arrangement| {
            super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == KIND).map(|tab| tab.id.clone())
        });
        match open {
            Some(id) => {
                self.look_in(&job, move |arrangement| super::pane_tree::activate(arrangement, &id), cx);
            }
            None => self.add_tab(KIND, window, cx),
        }
        cx.notify();
    }
}

/// The tab's kind in a Job's panes.
pub(super) const KIND: &str = "graphs";

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use gpui::{prelude::*, Role};
    use ipc::{GraphFedBy, GraphFeed, GraphForm, GraphList, GraphShown, GraphsChanged, Instant, MetricCredential, SessionId};

    use super::super::column_strip::tests::drawn::fresh;
    use super::super::parts_fold::{drawn, drawn_like};
    use super::super::JobView;
    use crate::graph::feeds::{FeedBook, Graphs};

    const JOB: &str = "watch-checkout";
    const FEED: &str = "checkout-api/latency";

    fn shown(id: &str, title: &str) -> GraphShown {
        let spec = ipc::graph::read_spec(&format!(r#"{{"data":{{"name":"{FEED}"}},"mark":"line","encoding":{{"x":{{"field":"at","type":"temporal"}},"y":{{"field":"p99","type":"quantitative"}}}}}}"#)).expect("reads");
        GraphShown { id: id.into(), title: title.into(), form: GraphForm::Chart { spec, headline: None }, fed_by: GraphFedBy::Recipe { every_secs: 15 }, live: true, shown_at: Instant::carried("2026-11-10T22:01:00Z"), read_at: None, broken: None, no_token: None }
    }

    fn grafana() -> MetricCredential {
        MetricCredential { name: "grafana".into(), how: "A Grafana service account token".into(), url: Some("https://grafana.example/tokens".into()) }
    }

    fn graphs() -> Vec<GraphShown> {
        let mut tokenless = shown("graph-percentiles", "Latency percentiles");
        tokenless.no_token = Some(grafana());
        tokenless.broken = Some("No grafana token on this Mac.".into());
        vec![shown("graph-p99", "p99 latency"), tokenless]
    }

    /// The live graphs' Job open on its thread, with a book as Fleet's `list_graphs` would fill it: p99 and a graph whose
    /// token is missing. The thread's other `show_graph` calls name graphs Fleet does not hold.
    fn opened(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, gpui::Entity<FeedBook>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let rows = (0..5).map(|at| serde_json::json!({"at": format!("2026-11-10T22:0{at}:00Z"), "p99": 200 + at * 10}).as_object().cloned().unwrap_or_default()).collect();
        let book = cx.new(|_| {
            let mut book = FeedBook::unread();
            book.read_list(GraphList { session_id: SessionId::carried(JOB), graphs: graphs(), feeds: vec![GraphFeed { name: FEED.into(), rows, added: 5 }] });
            book
        });
        cx.update(|cx| {
            let mut by_job = BTreeMap::new();
            by_job.insert(JOB.to_string(), book.clone());
            cx.set_global(Graphs { by_job, _tick: None });
        });
        let jobs = crate::mock::jobs("live-graphs").expect("the fixture decodes");
        let threads = crate::mock::threads("live-graphs").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(4000.)));
        list.update(cx, |list, cx| list.open(JOB.into(), cx));
        fresh(cx);
        fresh(cx);
        (view, book, cx)
    }

    #[gpui::test]
    fn a_graph_stands_once_where_it_was_first_shown_and_fleet_sending_it_again_redraws_it_in_place(cx: &mut gpui::TestAppContext) {
        let (_view, book, cx) = opened(cx);
        assert_eq!(drawn_like(cx, Role::Article, "Graph: p99"), ["Graph: p99 latency"], "the later call of the same id draws no second card");
        let mut again = graphs();
        again[0].title = "p99 after the rollback".into();
        book.update(cx, |book, cx| {
            book.take_changed(GraphsChanged { session_id: SessionId::carried(JOB), graphs: again, feeds: Vec::new() });
            cx.notify();
        });
        fresh(cx);
        assert_eq!(drawn_like(cx, Role::Article, "Graph: p99"), ["Graph: p99 after the rollback"], "the card is replaced where it stands");
    }

    #[gpui::test]
    fn a_missing_token_is_said_on_its_card_and_a_call_fleet_holds_no_graph_for_is_its_line(cx: &mut gpui::TestAppContext) {
        let (_view, _book, cx) = opened(cx);
        assert!(drawn(cx, Role::Status, "No grafana token on this Mac. A Grafana service account token"), "{:?}", drawn_like(cx, Role::Status, "No"));
        assert!(drawn(cx, Role::Link, "Make a grafana token"));
        assert!(drawn_like(cx, Role::Status, "Not read").is_empty(), "the token's sentence is not said twice");
        assert!(drawn(cx, Role::Label, "show_graph graph-errors"), "{:?}", drawn_like(cx, Role::Label, "show_graph"));
    }
}
