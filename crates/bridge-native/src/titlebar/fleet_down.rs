//! The Fleet dot, the popover it opens (the build switcher, the figures, the sentence, Doctor), and
//! the notice above the screen when Fleet is down or does not match (`FleetPanel.tsx`,
//! `FleetNotice.tsx`, `FailureNotice.tsx`).
//!
//! **These states stay in the app**: Fleet is not there to send them, so they are read from the
//! runtime file's presence (`face::connection_of`).

use gpui::{
    anchored, deferred, div, point, prelude::*, px, Anchor, AnimationExt as _, AnyElement, Context, FontWeight, Hsla, Role, SharedString, Window,
};

use super::face::{Dot, Kind, Outcome, Restarting, Usage};
use super::parts::{card, glyph, mono, no_drag, sans, small, tip, Btn, Weight};
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

const RESTART_TOOLTIP: &str = "Restart Fleet. Working Drones are adopted and cannot be redirected until they finish.";

impl TitleBar {
    /// The Fleet dot: a 6px circle in the state's hue, or Armada's mark pulsing while it starts.
    pub(super) fn dot(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let statement = self.face.fleet.connection.statement();
        let said: SharedString = format!("Fleet — {}", statement.label).into();
        let tone = dot_tone(statement.dot, look);
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
    pub(super) fn popover(&self, look: &Look, _window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let (from, body) = match self.popup? {
            Popup::Usage { .. } => {
                let Usage::Limits(limits) = &self.face.usage else { return None };
                (self.at.meter.get()?, self.usage_card(limits, look, cx))
            }
            Popup::Fleet => (self.at.fleet.get()?, self.fleet_panel(look, cx)),
        };
        let at = point(from.origin.x + from.size.width, from.origin.y + from.size.height + look.length(token::SPACE_1));
        Some(deferred(anchored().position(at).anchor(Anchor::TopRight).snap_to_window_with_margin(look.length(token::SPACE_1)).child(body)).with_priority(0).into_any_element())
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
                                .child(div().flex_none().size(look.length(token::DOT)).rounded_full().bg(dot_tone(statement.dot, look))),
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
                        .children(doctor),
                ),
            Role::Region,
            "Fleet",
        )
        .into_any_element()
    }

    /// The notice above the screen, and what Restart Fleet adds under it.
    pub(super) fn notice(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let fleet = &self.face.fleet;
        let swapping = self.face.build.as_ref().is_some_and(|held| held.report.swapping);
        if !fleet.notice(swapping) {
            return None;
        }
        let statement = fleet.connection.statement();
        let working = fleet.restarting == Restarting::Working;
        let apart = fleet.restarting == Restarting::Apart;
        let restart = fleet.connection.is_mismatch().then(|| {
            let busy = working || apart;
            let button = Btn::new("restart-fleet", if working { "Restarting Fleet" } else { "Restart Fleet" }, Weight::Ghost)
                .icon("rotate-cw")
                .pending(working)
                .disabled(busy)
                .draw(look, cx)
                .bg(look.color(token::BG_SUNKEN))
                .border_color(look.color(token::BORDER_DEFAULT))
                .tooltip(tip(RESTART_TOOLTIP));
            let button = if busy { button } else { button.on_click(cx.listener(|this, _, _, cx| this.restart_fleet_pressed(cx))) };
            labelled(button, Role::Button, if working { "Restarting Fleet" } else { "Restart Fleet" }).into_any_element()
        });
        let block = self.failure_block(
            look,
            "fleet-notice",
            statement.kind,
            statement.headline.clone().into(),
            statement.next.unwrap_or_default().into(),
            statement.code,
            statement.detail.clone(),
            restart.into_iter().collect(),
            None,
            cx,
        );
        let apart_alert = apart.then(|| {
            alert(
                look,
                "still-not-matching",
                "Still not matching",
                "Fleet restarted on the installed build, and it is not this Bridge's. Update Armada, then reopen Bridge.",
            )
        });
        let no_answer = (fleet.restarting == Restarting::NoAnswer).then(|| {
            let dismiss = labelled(
                Btn::new("dismiss-restart-failure", "Dismiss", Weight::Ghost)
                    .draw(look, cx)
                    .bg(look.color(token::BG_SUNKEN))
                    .on_click(cx.listener(|this, _, _, cx| this.answer(|face| face.fleet.restarting = Restarting::Idle, cx))),
                Role::Button,
                "Dismiss",
            )
            .into_any_element();
            self.failure_block(
                look,
                "restart-failure",
                Kind::Fault,
                "Fleet did not come back inside 30 seconds".into(),
                "Read the Fleet log, then restart it again.".into(),
                "bridge.fleet.restart_no_answer",
                Some("no new runtime file in 30s".into()),
                vec![dismiss],
                None,
                cx,
            )
        });
        Some(
            div()
                .id("fleet-band")
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .bg(look.color(token::BG_BASE))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(block)
                .children(apart_alert)
                .children(no_answer)
                .into_any_element(),
        )
    }

    /// A failure as the notice draws it: a leading edge in the class's hue, the headline and what to do,
    /// a code chip, the folded detail, and the acts.
    #[allow(clippy::too_many_arguments)]
    fn failure_block(
        &self,
        look: &Look,
        id: &'static str,
        kind: Kind,
        headline: SharedString,
        next: SharedString,
        code: &'static str,
        detail: Option<String>,
        acts: Vec<AnyElement>,
        _unused: Option<()>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let (edge, edge_width, headline_ink, chip_ground) = match kind {
            Kind::Fault => (look.color(token::ERROR), look.length(token::ERROR_EDGE), look.color(token::ERROR), look.color(token::ERROR)),
            Kind::Degraded => (look.color(token::DEGRADED), look.length(token::DEGRADED_EDGE), look.color(token::FG_DEFAULT), look.color(token::DEGRADED)),
        };
        let open = self.details_open;
        let copy = div()
            .flex()
            .child(labelled(
                Btn::new("copy-debug-info", "Copy debug info", Weight::Secondary)
                    .draw(look, cx)
                    .rounded_r_none()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::CopyDebugInfo))),
                Role::Button,
                "Copy debug info",
            ))
            .child(labelled(
                super::parts::icon_button("more-ways", "chevron-down", look.color(token::FG_MUTED), look)
                    .h(look.length(token::H_CONTROL))
                    .w(look.length(token::H_CONTROL))
                    .rounded_l_none()
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .tooltip(tip("More ways to act on this failure")),
                Role::Button,
                "More ways to act on this failure",
            ));
        labelled(
            div()
                .id(id)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .pl(look.length(token::SPACE_4))
                .border_l(edge_width)
                .border_color(edge)
                .font_family(sans(look))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_2))
                        .when(kind == Kind::Degraded, |head| {
                            head.child(div().flex_none().mt(look.length(token::SPACE_2)).size(look.length(token::DOT)).rounded_full().bg(look.color(token::DEGRADED_DOT)))
                        })
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(look.length(token::SPACE_1))
                                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(headline_ink).child(headline.clone()))
                                .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(next)),
                        ),
                )
                .child(
                    div().flex().child(
                        div()
                            .px(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_MD))
                            .bg(chip_ground)
                            .text_color(look.color(token::FG_INVERSE))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_XS))
                            .child(code),
                    ),
                )
                .children(detail.map(|detail| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(
                            div()
                                .id(SharedString::from(format!("details-{id}")))
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_1))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .cursor_pointer()
                                .focusable()
                                .tab_stop(true)
                                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                                .child(glyph(if open { "chevron-down" } else { "chevron-right" }, px(12.), look.color(token::FG_MUTED)))
                                .child("Details")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.details_open = !this.details_open;
                                    cx.notify();
                                })),
                            Role::Button,
                            "Details",
                        ))
                        .children(open.then(|| {
                            small(look).font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).whitespace_normal().child(detail)
                        }))
                }))
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).children(acts).child(copy)),
            if kind == Kind::Fault { Role::Alert } else { Role::Status },
            headline,
        )
        .into_any_element()
    }
}

/// An Alert in the escalated hue: a frame, a ground, a title and a body.
fn alert(look: &Look, id: &'static str, title: &'static str, body: &'static str) -> AnyElement {
    labelled(
        div()
            .id(id)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::STATUS_ESCALATED))
            .bg(look.color(token::STATUS_ESCALATED_BG))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::STATUS_ESCALATED)).child(title))
            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(body)),
        Role::Alert,
        title,
    )
    .into_any_element()
}
