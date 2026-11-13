//! The first-run screen: while Fleet says he has not answered how he works with Jobs (`first_run_answered` false), it is
//! drawn over everything and asks it as two rows of cards. ← and → choose within a row, Tab moves on to the next
//! row and then Start, Return presses Start. Start saves `take_in`, `answer_with` and `first_run_answered`, and the screen goes.
//! The cards are the Settings page's, drawn larger (`settings/ways.rs`); Settings keeps both choices.

use gpui::{anchored, deferred, div, prelude::*, AnyElement, AnyView, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, MouseButton, Role, SharedString, Subscription, Window};

use crate::fleet::FleetConnection;
use crate::labels::{cover, labelled};
use crate::look::{token, ActiveLook};
use crate::settings::overlay::SettingsOverlay;
use crate::settings::view::{ringed, small};
use crate::settings::ways::{self, Row, Sampler, Size};
use crate::settings::wire::Put;

pub struct FirstRunScreen {
    under: AnyView,
    settings: Entity<SettingsOverlay>,
    /// Holds the keys anywhere in the app or the screen, so a walk's keys go where a person's would.
    pub within: FocusHandle,
    pub take_row: FocusHandle,
    answer_row: FocusHandle,
    sample_stop: FocusHandle,
    start_stop: FocusHandle,
    take: &'static str,
    answer: &'static str,
    sampler: Entity<Sampler>,
    saving: bool,
    refused: Option<String>,
    /// What had the keys before the screen came up, given them back when it goes.
    back: Option<FocusHandle>,
    came_up: bool,
    _subs: Vec<Subscription>,
}

impl FirstRunScreen {
    /// Over the app on a live Fleet, or over the mock's in a walk. `settings` carries the wire either way.
    pub fn new(under: AnyView, settings: Entity<SettingsOverlay>, fleet: Option<&Entity<FleetConnection>>, cx: &mut Context<Self>) -> Self {
        let sampler = cx.new(|_| Sampler::new());
        let mut subs = vec![cx.observe(&sampler, |_, _, cx| cx.notify())];
        if let Some(fleet) = fleet {
            subs.push(cx.observe(fleet, |_, _, cx| cx.notify()));
        }
        FirstRunScreen {
            under,
            settings,
            within: cx.focus_handle(),
            take_row: cx.focus_handle().tab_stop(true),
            answer_row: cx.focus_handle().tab_stop(true),
            sample_stop: cx.focus_handle().tab_stop(true),
            start_stop: cx.focus_handle().tab_stop(true),
            take: "read",
            answer: "type",
            sampler,
            saving: false,
            refused: None,
            back: None,
            came_up: false,
            _subs: subs,
        }
    }

    fn owed(&self, cx: &gpui::App) -> bool {
        self.settings.read(cx).wire.first_run_owed(cx)
    }

    fn mock(&self, cx: &gpui::App) -> bool {
        matches!(self.settings.read(cx).wire, crate::settings::wire::Wire::Mock(_))
    }

    fn stops(&self) -> Vec<FocusHandle> {
        let mut stops = vec![self.take_row.clone()];
        if self.take == "listen" {
            stops.push(self.sample_stop.clone());
        }
        stops.push(self.answer_row.clone());
        stops.push(self.start_stop.clone());
        stops
    }

    /// Tab and Shift+Tab, kept inside the screen: past its last stop is its first.
    fn tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.stops();
        let at = stops.iter().position(|one| one.is_focused(window));
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => stops.len() - 1,
            (Some(at), false) => (at + 1) % stops.len(),
            (Some(at), true) => (at + stops.len() - 1) % stops.len(),
        };
        window.focus(&stops[next], cx);
        cx.notify();
    }

    fn pick(&mut self, row: Row, word: &'static str, cx: &mut Context<Self>) {
        match row {
            Row::Take => self.take = word,
            Row::Answer => self.answer = word,
        }
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        if keys.modifiers.platform || keys.modifiers.control || keys.modifiers.alt {
            return;
        }
        match keys.key.as_str() {
            "left" | "right" => {
                let by = if keys.key == "left" { -1 } else { 1 };
                let (row, held) = if self.take_row.is_focused(window) {
                    (Row::Take, self.take)
                } else if self.answer_row.is_focused(window) {
                    (Row::Answer, self.answer)
                } else {
                    return;
                };
                if let Some(to) = ways::moved(row, held, by) {
                    self.pick(row, to.word, cx);
                }
            }
            "tab" => self.tab(keys.modifiers.shift, window, cx),
            "enter" => self.start(window, cx),
            "space" if self.start_stop.is_focused(window) => self.start(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// Start: the two ways and then that he has answered, in that order, so a refusal part-way leaves him asked.
    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.saving = true;
        self.refused = None;
        let (take, answer) = (self.take, self.answer);
        cx.spawn_in(window, async move |this, cx| {
            let mut last = None;
            let mut why = None;
            for put in [("take_in", Put::Text(take.to_string())), ("answer_with", Put::Text(answer.to_string())), ("first_run_answered", Put::Switch(true))] {
                let Ok(sent) = this.update(cx, |this, cx| this.settings.update(cx, |settings, cx| settings.wire.save_put(put.0, put.1, cx))) else { return };
                match sent.await {
                    Ok(saved) => last = Some(saved),
                    Err(refused) => {
                        why = Some(refused.said);
                        break;
                    }
                }
            }
            let _ = this.update_in(cx, |this, window, cx| {
                this.saving = false;
                match (last, why) {
                    (_, Some(said)) => this.refused = Some(said),
                    (Some(saved), None) => {
                        this.settings.update(cx, |settings, cx| settings.wire.preferences_saved(saved, window, cx));
                        if let Some(back) = this.back.take() {
                            window.focus(&back, cx);
                        }
                        this.came_up = false;
                    }
                    (None, None) => {}
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn render_cover(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let rows = [Row::Take, Row::Answer].map(|row| {
            let (held, focus, id) = match row {
                Row::Take => (self.take, self.take_row.clone(), "first-take"),
                Row::Answer => (self.answer, self.answer_row.clone(), "first-answer"),
            };
            let ring = focus.is_focused(window);
            let cards = ways::ways_of(row)
                .iter()
                .map(|one| {
                    let word = one.word;
                    let chosen = word == held;
                    let card = ways::way_card(look, one, chosen, ring && chosen, Size::Large, SharedString::from(format!("first-way-{word}"))).on_click(cx.listener(move |this, _, window, cx| {
                        let focus = if row == Row::Take { &this.take_row } else { &this.answer_row };
                        window.focus(focus, cx);
                        this.pick(row, word, cx);
                    }));
                    labelled(card, Role::RadioButton, one.label).into_any_element()
                })
                .collect();
            ways::way_row(look, id, row, cards, &focus, Size::Large)
        });
        let [take_row, answer_row] = rows;
        let heading = "How do you like to work with Jobs?";
        let sample = (self.take == "listen").then(|| {
            let mock = self.mock(cx);
            let state = self.sampler.read(cx).state;
            let (button, beside) = ways::sample_press(look, state, false, Some(&self.sample_stop), window);
            let button = button
                .on_click(cx.listener(move |this, _, window, cx| {
                    window.focus(&this.sample_stop, cx);
                    this.sampler.update(cx, |sampler, cx| sampler.press("", mock, cx));
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.sampler.update(cx, |sampler, cx| sampler.press("", mock, cx));
                        cx.stop_propagation();
                    }
                }));
            div().flex().items_center().gap(look.length(token::SPACE_3)).child(labelled(button, Role::Button, "Hear a sample")).children(beside).into_any_element()
        });
        let section = |title: &'static str, row: AnyElement, under: Option<AnyElement>| {
            div().flex().flex_col().gap(look.length(token::SPACE_3)).child(small(look, title, look.color(token::FG_MUTED))).child(row).children(under).into_any_element()
        };
        let start = labelled(
            ringed(
                look,
                div()
                    .id("first-start")
                    .track_focus(&self.start_stop)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(look.length(token::H_CONTROL_LG))
                    .px(look.length(token::SPACE_6))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::ACCENT))
                    .bg(look.color(token::ACCENT))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::TEXT_ON_ACCENT))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))
                    .child("Start"),
                &self.start_stop,
                window,
            ),
            Role::Button,
            "Start",
        );
        let refused = self.refused.clone().map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("first-refused"), Role::Alert, said));
        let body = div()
            .id("first-run-page")
            .w_full()
            .max_w(look.length(token::SPACE_12) * 19.)
            .max_h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_6))
            .p(look.length(token::PAD_CARD))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(
                        div().id("first-heading").text_size(look.length(token::TEXT_XL)).line_height(look.length(token::LEADING_XL)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(heading),
                        Role::Heading,
                        heading,
                    ))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child("Settings keeps both. Change either one there.")),
            )
            .child(section(Row::Take.title(), take_row, sample))
            .child(section(Row::Answer.title(), answer_row, None))
            .child(div().flex().items_center().justify_end().gap(look.length(token::SPACE_3)).children(refused).child(start));
        deferred(
            anchored().position(room.origin).anchor(gpui::Anchor::TopLeft).child(cover(
                labelled(
                    div()
                        .id("first-run")
                        .occlude()
                        .w(room.size.width)
                        .h(room.size.height)
                        .flex()
                        .justify_center()
                        .items_center()
                        .bg(look.color(token::BG_BASE))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.on_key(event, window, cx)))
                        .child(body),
                    Role::Dialog,
                    heading,
                ),
            )),
        )
        .with_priority(3)
        .into_any_element()
    }
}

impl Render for FirstRunScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owed = self.owed(cx);
        if owed && !self.came_up {
            // The keys are taken once, as the screen comes up, and kept until Start gives them back.
            self.came_up = true;
            self.back = window.focused(cx);
            let take = self.take_row.clone();
            window.defer(cx, move |window, cx| window.focus(&take, cx));
        }
        let cover = owed.then(|| self.render_cover(window, cx));
        div().id("first-run-root").track_focus(&self.within).relative().size_full().child(self.under.clone()).children(cover)
    }
}
