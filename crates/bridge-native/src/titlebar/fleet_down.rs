//! The Fleet dot, the popover it opens (the build switcher, the figures, the sentence, Doctor, Copy
//! debug info), and the toast of a Fleet alive and not answering (`FleetPanel.tsx`, `FleetNotice.tsx`).
//! Fleet stopped, restarting or on another build is the centred panel's (`link_layer.rs`), and the
//! dot takes its colour while it stands.
//!
//! **These states stay in the app**: Fleet is not there to send them, so they are read from the
//! runtime file's presence (`face::connection_of`).

use gpui::{
    anchored, deferred, div, point, prelude::*, px, Anchor, AnimationExt as _, AnyElement, Context, FontWeight, Hsla, Role, SharedString, Window,
};

use super::face::{Dot, Kind, Outcome, Usage};
use super::parts::{card, glyph, mono, no_drag, small, tip, Btn, Weight};
use crate::toasts::FleetNotice;
use super::{Asked, Popup, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look};

fn dot_tone(dot: Dot, look: &Look) -> Hsla {
    match dot {
        Dot::Running => look.color(token::STATUS_COMPLETED_SUCCESS),
        Dot::NotRunning => look.color(token::STATUS_ESCALATED),
        Dot::Unreachable => look.color(token::STATUS_AWAITING_REVIEW),
        Dot::Starting | Dot::Unknown => look.color(token::FG_MUTED),
    }
}

fn outcome_tone(outcome: Outcome, look: &Look) -> (Hsla, &'static str) {
    match outcome {
        Outcome::Pass => (look.color(token::STATUS_COMPLETED_SUCCESS), "Pass"),
        Outcome::Warn => (look.color(token::STATUS_AWAITING_REVIEW), "Warn"),
        Outcome::Fail => (look.color(token::STATUS_ESCALATED), "Fail"),
        Outcome::Reading => (look.color(token::FG_MUTED), "Reading"),
    }
}

impl TitleBar {
    /// The Fleet dot: a 6px circle in the state's hue, or Armada's mark pulsing while it starts.
    pub(super) fn dot(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let statement = self.face.fleet.connection.statement();
        let said: SharedString = format!("Fleet — {}", statement.label).into();
        let tone = self.panel_ink(look).unwrap_or_else(|| dot_tone(statement.dot, look));
        let ring = Self::focus_ring(look);
        let at = std::rc::Rc::clone(&self.at.fleet);
        let mark: AnyElement = if statement.dot == Dot::Starting {
            div()
                .flex_none()
                .child(gpui::svg().path(crate::assets::MARK).size(look.length(token::SPACE_4)).text_color(tone))
                .with_animation(
                    "fleet-starting",
                    gpui::Animation::new(look.duration(token::DURATION_PULSE) * 2).repeat().with_easing(gpui::pulsating_between(0.35, 1.)),
                    |mark, opacity| mark.opacity(opacity),
                )
                .into_any_element()
        } else {
            div().flex_none().size(look.length(token::DOT)).rounded_full().bg(tone).into_any_element()
        };
        labelled(
            no_drag(div()
                .id("fleet")
                .relative()
                .flex_none()
                .size(look.length(token::H_CONTROL_LG))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if self.popup == Some(Popup::Fleet) { look.color(token::ACCENT) } else { gpui::transparent_black() })
                .when(self.popup == Some(Popup::Fleet), |button| button.bg(look.color(token::ACCENT_MUTED)))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(gpui::canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(mark)
                .tooltip(tip(said.clone()))
                .on_click(cx.listener(|this, _, window, cx| this.fleet_pressed(window, cx)))),
            Role::Button,
            said,
        )
    }

    /// The open popover under its trigger, right edges aligned.
    pub(super) fn popover(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        // Over the Fleet panel's dim while it stands, as the bar above it stays live.
        let over = if self.link.panel.is_some() { super::link_layer::LAYER + 1 } else { 0 };
        let (from, body) = match self.popup? {
            Popup::Usage { .. } => {
                let Usage::Limits(limits) = &self.face.usage else { return None };
                (self.at.meter.get()?, self.usage_card(limits, look, cx))
            }
            Popup::Fleet => (self.at.fleet.get()?, self.fleet_panel(look, cx)),
            // The switcher hangs from its trigger's left edge, where the bar starts.
            Popup::Projects => {
                let from = self.at.repository.get()?;
                let at = point(from.origin.x, from.origin.y + from.size.height + look.length(token::SPACE_1));
                let body = self.switcher_panel(look, cx);
                return Some(deferred(anchored().position(at).anchor(Anchor::TopLeft).snap_to_window_with_margin(look.length(token::SPACE_1)).child(body)).with_priority(over).into_any_element());
            }
        };
        let margin = look.length(token::SPACE_1);
        let at = point(from.origin.x + from.size.width, from.origin.y + from.size.height + margin);
        // A panel taller than the room under its trigger scrolls there rather than sliding up over the bar.
        let room = (window.viewport_size().height - at.y - margin).max(px(0.));
        let body = div().id("popover-room").max_h(room).overflow_y_scroll().child(body);
        Some(deferred(anchored().position(at).anchor(Anchor::TopRight).snap_to_window_with_margin(margin).child(body)).with_priority(over).into_any_element())
    }

    /// The Fleet panel: the head, the build switcher, the figures, the sentence, Doctor.
    fn fleet_panel(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let fleet = &self.face.fleet;
        let statement = fleet.connection.statement();
        let figures = fleet.connection.figures();
        let sentence = match fleet.connection {
            super::Connection::Connecting { .. } | super::Connection::Starting { .. } | super::Connection::Connected { .. } => None,
            _ => statement.detail.clone(),
        };
        let doctor = fleet.doctor.as_ref().map(|doctor| {
            let (tone, word) = outcome_tone(doctor.outcome, look);
            labelled(
                div()
                    .id("open-doctor")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(div().flex_none().size(look.length(token::DOT)).rounded_full().bg(tone))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Doctor"))
                    .child(div().text_color(tone).child(word))
                    .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_SUBTLE)).child(doctor.modules.clone()))
                    .child(glyph("chevron-right", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::Doctor))),
                Role::Button,
                "Open Doctor",
            )
        });
        // Something is wrong while Fleet does not answer: what it is, ready to paste.
        let copy = (!fleet.connection.is_live()).then(|| {
            labelled(
                div()
                    .id("copy-debug-info")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph("copy", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child("Copy debug info"))
                    .on_click(cx.listener(|this, _, _, cx| this.copy_debug_info(cx))),
                Role::Button,
                "Copy debug info",
            )
        });
        labelled(
            div()
                .id("fleet-panel")
                .track_focus(&self.popup_focus)
                .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.escape(window, cx);
                        cx.stop_propagation();
                    }
                }))
                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                    if this.menu.is_none() && this.dialog.is_none() {
                        this.shut(window, cx);
                    }
                }))
                .occlude()
                .child(
                    card(look)
                        .w(look.length(token::W_MENU) + look.length(token::SPACE_4) * 2.)
                        .gap(look.length(token::SPACE_3))
                        .p(look.length(token::SPACE_4))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(small(look).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child("Fleet"))
                                .child(div().flex_none().size(look.length(token::DOT)).rounded_full().bg(self.panel_ink(look).unwrap_or_else(|| dot_tone(statement.dot, look)))),
                        )
                        .children(self.face.build.as_ref().map(|held| self.build_section(held, look, cx)))
                        .children((!figures.is_empty()).then(|| {
                            div().flex().flex_col().gap(look.length(token::SPACE_1)).children(figures.into_iter().map(|(term, value)| {
                                small(look)
                                    .flex()
                                    .justify_between()
                                    .gap(look.length(token::SPACE_3))
                                    .child(div().text_color(look.color(token::FG_SUBTLE)).child(term))
                                    .child(div().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(value))
                            }))
                        }))
                        .children(sentence.map(|said| {
                            small(look).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).whitespace_normal().child(said)
                        }))
                        .children(doctor)
                        .children(copy),
                ),
            Role::Region,
            "Fleet",
        )
        .into_any_element()
    }

    /// Events Fleet dropped, as `Standing.tsx` says it: until the person notes how many, one alert above the screen.
    pub(super) fn dropped_alert(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let dropped = self.face.dropped;
        if dropped.missed <= dropped.noted {
            return None;
        }
        let title = "Events were dropped before Bridge saw them";
        let missed = dropped.missed;
        Some(
            labelled(
                div()
                    .id("events-dropped")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::STATUS_ESCALATED))
                    .bg(look.color(token::STATUS_ESCALATED_BG))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::STATUS_ESCALATED)).child(title))
                            .child(
                                div()
                                    .text_size(look.length(token::TEXT_XS))
                                    .line_height(look.length(token::LEADING_XS))
                                    .text_color(look.color(token::FG_MUTED))
                                    .child(format!("{missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.")),
                            ),
                    )
                    .child(labelled(
                        Btn::new("events-noted", "Noted", Weight::Ghost).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| {
                            this.face.dropped.noted = missed;
                            cx.emit(Asked::EventsNoted(missed));
                            cx.notify();
                        })),
                        Role::Button,
                        "Noted",
                    )),
                Role::Alert,
                title,
            )
            .into_any_element(),
        )
    }

    /// Fleet alive and not answering, as a toast: the one Fleet state the centred panel leaves out.
    pub(super) fn fleet_notices(&self) -> Vec<FleetNotice> {
        let fleet = &self.face.fleet;
        let swapping = self.face.build.as_ref().is_some_and(|held| held.report.swapping);
        if !matches!(fleet.connection, super::Connection::Unreachable { .. }) || !fleet.notice(swapping) {
            return Vec::new();
        }
        let statement = fleet.connection.statement();
        vec![FleetNotice {
            id: "fleet-notice",
            headline: statement.headline.clone().into(),
            next: statement.next.unwrap_or_default().into(),
            code: statement.code,
            fault: statement.kind == Kind::Fault,
        }]
    }
}
