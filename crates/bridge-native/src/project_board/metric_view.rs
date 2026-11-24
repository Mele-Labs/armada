//! A metric block drawn: its state glyph, the first number large, the rows in the view it keeps, and where it cannot read, the
//! field for this Mac's token or the words of what broke. What is drawn is worked out in `metric.rs`; List, Table and Cards are
//! the board's own (`view.rs`).

use gpui::{canvas, div, point, prelude::*, px, size, AnyElement, Bounds, Context, Corners, FontWeight, PathBuilder, Role, SharedString};
use gpui_component::input::Input;
use ipc::{MetricAnswer, MetricCredential, MetricReading, MetricState};

use super::board::Board;
use super::layout::{BlockKind, View};
use super::metric::{self, Plot};
use super::model::{Press, Shown};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::ring_colour;
use crate::titlebar::parts::{glyph, mono, tip, Btn, Weight};

impl Board {
    /// The state of a metric as a registry glyph in its hue, with the words of the state in its tooltip.
    pub(super) fn metric_state(&self, reading: &MetricReading, look: &Look) -> AnyElement {
        let (mark, hue, said) = match &reading.state {
            MetricState::Read => ("circle-check", token::STATUS_COMPLETED_SUCCESS, "Read"),
            MetricState::NoToken { .. } => ("key-round", token::DEGRADED, "No token on this Mac"),
            MetricState::Broken { .. } => ("circle-x", token::STATUS_COMPLETED_FAILED, "Could not read"),
        };
        labelled(
            div().id(SharedString::from(format!("metric-{}-state", reading.id))).flex_none().flex().items_center().tooltip(tip(said)).child(glyph(mark, look.length(token::SPACE_4), look.color(hue))),
            Role::Image,
            said,
        )
        .into_any_element()
    }

    /// What a metric block holds under its header. A reading that cannot read says why first, and the last answer it has stands
    /// dimmed beneath.
    pub(super) fn metric_body(&self, reading: &MetricReading, view: View, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let gap = look.length(token::SPACE_2);
        let live = matches!(reading.state, MetricState::Read);
        let state = match &reading.state {
            MetricState::Read => None,
            MetricState::NoToken { credential } => Some(self.token_prompt(reading, credential, look, cx)),
            MetricState::Broken { why } => Some(self.broken_note(reading, why, look, cx)),
        };
        let answer = reading.answer.as_ref().map(|answer| {
            let shown = div().flex().flex_col().gap(gap).children(self.metric_headline(reading, answer, live, look)).child(self.metric_rows(reading, answer, view, look, cx));
            if live { shown.into_any_element() } else { shown.opacity(0.5).into_any_element() }
        });
        self.body_scroll(SharedString::from(format!("metric-{}-body", reading.id))).flex_1().min_h_0().p(gap).flex().flex_col().gap(gap).children(state).children(answer).into_any_element()
    }

    /// The first number large, its unit and label, and when it was read.
    fn metric_headline(&self, reading: &MetricReading, answer: &MetricAnswer, live: bool, look: &Look) -> Option<AnyElement> {
        let head = metric::headline(answer)?;
        let as_of = live.then(|| metric::as_of(reading)).flatten();
        let said = match &head.unit {
            Some(unit) => format!("{}, {} {unit}", head.label, head.value),
            None => format!("{}, {}", head.label, head.value),
        };
        let row = div()
            .id(SharedString::from(format!("metric-{}-headline", reading.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_none().text_size(look.length(token::TEXT_2XL)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(head.value)))
            .children(head.unit.map(|unit| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(unit))))
            .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(head.label)))
            .child(div().flex_1())
            .children(as_of.map(|(short, full)| {
                div().id(SharedString::from(format!("metric-{}-as-of", reading.id))).flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip(full)).child(SharedString::from(short))
            }));
        Some(labelled(row, Role::Group, said).into_any_element())
    }

    fn metric_rows(&self, reading: &MetricReading, answer: &MetricAnswer, view: View, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        match view {
            View::Graph => self.metric_graph(reading, look),
            View::Picture => self.metric_picture(reading, answer, look, cx),
            _ => {
                let shown = Shown { items: metric::items(&reading.id, answer), tiles: Vec::new() };
                self.rows(BlockKind::metric(&reading.id), view, &shown, look, cx)
            }
        }
    }

    /// The series as a line in time, with the clock at each end. Rows that carry a time are the series; otherwise it is what
    /// Fleet kept from its readings. Under two points there is nothing to draw, and nothing is.
    fn metric_graph(&self, reading: &MetricReading, look: &Look) -> AnyElement {
        let plot = metric::plot(reading);
        let points: Vec<(f32, f32)> = plot.points().iter().map(|(at, value)| (at.timestamp_millis() as f32, *value as f32)).collect();
        let (Some(first), Some(last)) = (plot.points().first(), plot.points().last()) else { return div().into_any_element() };
        if points.len() < 2 {
            return div().into_any_element();
        }
        let (line, base) = (look.color(token::SKETCH_BLUE), look.color(token::BORDER_DEFAULT));
        let (from, to) = (points[0].0, points[points.len() - 1].0);
        let low = points.iter().map(|(_, value)| *value).fold(f32::INFINITY, f32::min);
        let high = points.iter().map(|(_, value)| *value).fold(f32::NEG_INFINITY, f32::max);
        let drawing = canvas(
            |_, _, _| {},
            move |bounds: Bounds<gpui::Pixels>, _, window, _| {
                let pad = px(4.);
                let (width, height) = (bounds.size.width - pad * 2.0, bounds.size.height - pad * 2.0);
                let at = |time: f32, value: f32| {
                    let across = if to > from { (time - from) / (to - from) } else { 0. };
                    let up = if high > low { (value - low) / (high - low) } else { 0.5 };
                    point(bounds.origin.x + pad + width * across, bounds.origin.y + pad + height * (1. - up))
                };
                let mut floor = PathBuilder::stroke(px(1.));
                floor.move_to(point(bounds.origin.x, bounds.origin.y + bounds.size.height));
                floor.line_to(point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height));
                if let Ok(path) = floor.build() {
                    window.paint_path(path, base);
                }
                let mut stroke = PathBuilder::stroke(px(1.5));
                for (step, (time, value)) in points.iter().enumerate() {
                    let here = at(*time, *value);
                    if step == 0 { stroke.move_to(here) } else { stroke.line_to(here) }
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, line);
                }
                if let Some((time, value)) = points.last() {
                    window.paint_quad(gpui::fill(Bounds::centered_at(at(*time, *value), size(px(6.), px(6.))), line).corner_radii(Corners::all(px(3.))));
                }
            },
        )
        .w_full()
        .h(look.length(token::H_ROW_UNIT) * 2.0);
        let ends = div()
            .flex()
            .justify_between()
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(SharedString::from(metric::axis(first.0)))
            .child(SharedString::from(metric::axis(last.0)));
        let source = match plot {
            Plot::Rows(_) => "Graph of the rows",
            Plot::Kept(_) => "Graph of the readings kept",
        };
        labelled(div().id(SharedString::from(format!("metric-{}-graph", reading.id))).flex().flex_col().gap(look.length(token::SPACE_1)).tooltip(tip(source)).child(drawing).child(ends), Role::Image, format!("Graph of {}", reading.title)).into_any_element()
    }

    /// The picture the tool rendered, as a card with the address's host and an open glyph; a press opens it in the browser.
    /// Inline drawing waits on an image client, which Bridge does not have yet.
    fn metric_picture(&self, reading: &MetricReading, answer: &MetricAnswer, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let Some(picture) = answer.picture.clone() else { return div().into_any_element() };
        let ring = ring_colour(look);
        let host = metric::host(&picture);
        let said = format!("Open the picture of {} from {host}", reading.title);
        let card = div()
            .id(SharedString::from(format!("metric-{}-picture", reading.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_ROW_ACTION))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .text_size(look.length(token::TEXT_XS))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip(format!("Open {picture} in the browser")))
            .on_click(cx.listener(move |this, _, _, cx| this.press(Press::Url(picture.clone()), cx)))
            .child(glyph("image", look.length(token::SPACE_4), look.color(token::SKETCH_BLUE)))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(host)))
            .child(glyph("external-link", look.length(token::SPACE_4), look.color(token::FG_MUTED)));
        labelled(card, Role::Button, said).into_any_element()
    }

    /// What the credential is and where it is made, and the field to paste it in. The field is masked and kept nowhere.
    fn token_prompt(&self, reading: &MetricReading, credential: &MetricCredential, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let id = reading.id.clone();
        let saving = self.saving_tokens.contains_key(&id);
        let typed = self.token_typed(&id, cx);
        let how = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .child(glyph("key-round", look.length(token::SPACE_4), look.color(token::DEGRADED)))
            .child(div().flex_1().min_w_0().child(SharedString::from(credential.how.clone())));
        let make = credential.url.clone().map(|url| {
            let said = format!("Open {url}");
            labelled(
                Btn::new(SharedString::from(format!("metric-{id}-make")), "Make one", Weight::Ghost).icon("external-link").draw(look, cx).tooltip(tip(said.clone())).on_click(cx.listener(move |this, _, _, cx| this.press(Press::Url(url.clone()), cx))),
                Role::Link,
                format!("Make a token for {}", reading.title),
            )
        });
        let field = self.token_fields.get(&id).map(|field| {
            labelled(
                div()
                    .id(SharedString::from(format!("metric-{id}-token")))
                    .flex_1()
                    .min_w_0()
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .child(div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_XS)).child(Input::new(&field.input).appearance(false))),
                Role::TextInput,
                format!("Token for {}", reading.title),
            )
        });
        let save = (field.is_some() || saving).then(|| {
            let held = id.clone();
            let button = Btn::new(SharedString::from(format!("metric-{id}-save")), "Save", Weight::Primary).disabled(!typed).pending(saving).draw(look, cx).tooltip(tip("Keep the token in this Mac's keychain"));
            let button = if typed && !saving { button.on_click(cx.listener(move |this, _, window, cx| this.save_token(&held, window, cx))) } else { button };
            labelled(button, Role::Button, format!("Save the token for {}", reading.title))
        });
        div().flex().flex_col().gap(look.length(token::SPACE_2)).child(how).children(make.map(|make| div().flex().child(make))).child(div().flex().items_center().gap(look.length(token::SPACE_2)).children(field).children(save)).into_any_element()
    }

    /// Why a recipe did not read, in the font of a log, when its last good reading was, and the way to have it fixed.
    fn broken_note(&self, reading: &MetricReading, why: &str, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let id = reading.id.clone();
        let good = metric::last_good(reading);
        let fix = labelled(
            Btn::new(SharedString::from(format!("metric-{id}-fix")), "Fix with the librarian", Weight::Secondary)
                .icon("sparkles")
                .draw(look, cx)
                .tooltip(tip("Puts the ask in the librarian's box"))
                .on_click(cx.listener(move |this, _, _, cx| this.fix_metric(&id, cx))),
            Role::Button,
            format!("Fix {} with the librarian", reading.title),
        );
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(
                div()
                    .id(SharedString::from(format!("metric-{}-why", reading.id)))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(glyph("circle-x", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(why.to_string()))),
            )
            .children(good.map(|(said, full)| {
                div().id(SharedString::from(format!("metric-{}-last-good", reading.id))).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).tooltip(tip(full)).child(SharedString::from(said))
            }))
            .child(div().flex().child(fix))
            .into_any_element()
    }
}

impl super::ProjectScreen {
    /// A broken metric's "Fix with the librarian": the box holds `Fix <title>` and has the keys.
    /// Becomes `self.ask_box.update(cx, |ask, cx| ask.prefill(&format!("Fix {title}"), &format!("metric:{id}"), window, cx))` once AskBox lands.
    pub(super) fn fix_with_librarian(&mut self, title: &str, _id: &str, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let words = format!("Fix {title}");
        self.ask.update(cx, |state, cx| state.set_value(words.clone(), window, cx));
        crate::drafts::keep(super::KEEP_ASK, &words, cx);
        self.ask.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }
}
