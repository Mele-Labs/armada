//! The plan's own sheet over the app (Classic's `PlanReview`): Suggest on the plan card, a press on
//! the card or its sketch, and Enter or Space on the card open it. It draws the plan edited in place,
//! or the diff against the version before with Edit plan, and the sketch drawn with it. The foot holds
//! Edit plan, Send changes while there are marks, and Approve, which shuts the sheet. Esc or Close
//! shuts it, and another Job on screen shuts it too.

use gpui::{anchored, deferred, div, prelude::*, Anchor, AnyElement, Context, FocusHandle, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{ApprovePlan, SendPlanChanges, SessionId};

use super::acts::{button, button_frame, Answer, Tone, SHEET};
use super::cards::glass;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// The sheet while it is up: the Job it is over, and where the keys go back to.
pub(super) struct Open {
    job: SharedString,
    back_to: Option<FocusHandle>,
}

/// The sheet's focus stops, in Tab's order: the close, then the foot's presses.
pub(super) struct Focuses {
    pub close: FocusHandle,
    switch: FocusHandle,
    send: FocusHandle,
    approve: FocusHandle,
}

impl Focuses {
    pub(super) fn new(cx: &mut gpui::App) -> Self {
        Focuses { close: cx.focus_handle(), switch: cx.focus_handle(), send: cx.focus_handle(), approve: cx.focus_handle() }
    }
}

impl JobView {
    pub(super) fn plan_sheet_open(&self) -> bool {
        self.plan_sheet.as_ref().is_some_and(|open| self.shown.as_ref() == Some(&open.job))
    }

    pub(super) fn open_plan_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((job, _)) = self.plan_of_shown(cx) else { return };
        // A sheet opens fresh, as Classic mounts it: on the diff, if the plan has one and nothing is marked.
        if let Some(page) = self.pages.get_mut(&job) {
            page.plan_edit.editing = false;
        }
        let back_to = window.focused(cx);
        self.plan_sheet = Some(Open { job, back_to });
        window.focus(&self.plan_sheet_focus.close, cx);
        cx.notify();
    }

    pub(super) fn close_plan_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.plan_sheet.take() else { return };
        match open.back_to {
            Some(handle) => window.focus(&handle, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    /// The thread's acts while the sheet is up: the cards behind take none of them.
    pub(super) fn plan_sheet_act(&self, act: &crate::keymap::Act) -> bool {
        self.plan_sheet_open()
            && (act.is("thread_yes") || act.is("thread_change") || act.is("thread_send_back") || act.is("thread_no") || act.is("needs_you") || act.is("needs_you_fold") || act.is("send_message"))
    }

    /// Approve, with what he marked if he marked anything; the plan card reads the answer, and the sheet shuts.
    fn plan_sheet_approve(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((job, plan)) = self.plan_of_shown(cx) else { return };
        let Some(key) = self.thread(&job, cx).and_then(|held| held.head.plan.as_ref().map(|card| card.id.clone())) else { return };
        let marks = self.take_marks(&job);
        let session_id = SessionId::carried(job.to_string());
        let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
        let (route, body) = if marks.is_empty() {
            ("/sessions/plan/approve", encoded(ipc::encode(&ApprovePlan { session_id, version: plan.version })))
        } else {
            ("/sessions/plan/changes", encoded(ipc::encode(&SendPlanChanges { session_id, version: plan.version, marks, approve: true })))
        };
        self.close_plan_sheet(window, cx);
        self.settle(SHEET, key, Answer::Approved, route, body, window, cx);
    }

    /// Send changes: the marks go to the agent, and the sheet stays up on the plan.
    fn plan_sheet_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((job, plan)) = self.plan_of_shown(cx) else { return };
        let marks = self.take_marks(&job);
        if marks.is_empty() {
            return;
        }
        let session_id = SessionId::carried(job.to_string());
        let body = ipc::encode(&SendPlanChanges { session_id, version: plan.version, marks, approve: false }).map_err(|why| format!("{why:?}"));
        self.post_with(job, "/sessions/plan/changes", body, |_, _, _| {}, window, cx);
        window.focus(&self.plan_sheet_focus.close, cx);
        cx.notify();
    }

    fn plan_sheet_switch(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.plan_edit.editing = !page.plan_edit.editing;
        }
        cx.notify();
    }

    /// Esc and Tab, caught before anything inside the sheet hears them.
    fn plan_sheet_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.plan_field_open() {
                    self.field_done(window, cx);
                } else {
                    self.close_plan_sheet(window, cx);
                }
            }
            "tab" if !self.plan_field_open() => {
                let stops = self.plan_sheet_stops(cx);
                let at = stops.iter().position(|stop| stop.is_focused(window));
                let next = match (at, event.keystroke.modifiers.shift) {
                    (Some(at), false) => (at + 1) % stops.len(),
                    (Some(at), true) => (at + stops.len() - 1) % stops.len(),
                    (None, false) => 0,
                    (None, true) => stops.len() - 1,
                };
                window.focus(&stops[next], cx);
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    /// The presses Tab goes round, as they are drawn now.
    fn plan_sheet_stops(&self, cx: &gpui::App) -> Vec<FocusHandle> {
        let focus = &self.plan_sheet_focus;
        let mut stops = vec![focus.close.clone()];
        let Some((id, plan)) = self.plan_of_shown(cx) else { return stops };
        let marked = !self.plan_marks(&id).is_empty();
        if self.plan_sheet_revision(&plan) && !plan.approved && !marked {
            stops.push(focus.switch.clone());
        }
        if marked {
            stops.push(focus.send.clone());
        }
        if !plan.approved {
            stops.push(focus.approve.clone());
        }
        stops
    }

    /// Whether the plan was recorded again with changes, so it reads first as the diff.
    fn plan_sheet_revision(&self, plan: &ipc::SessionPlan) -> bool {
        plan.version >= 2 && plan.changes.as_ref().is_some_and(|changes| !changes.is_empty())
    }

    pub(super) fn render_plan_sheet(&self, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.plan_sheet_open() {
            return None;
        }
        let (id, plan) = self.plan_of_shown(cx)?;
        let page = self.pages.get(&id)?;
        let held = self.thread(&id, cx)?;
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let focus = &self.plan_sheet_focus;
        let marked = !page.plan_edit.marks.is_empty();
        let editing = page.plan_edit.editing;
        let revision = self.plan_sheet_revision(&plan);

        let pressed = |run: fn(&mut JobView, &mut Window, &mut Context<JobView>)| {
            cx.listener(move |this: &mut JobView, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                    run(this, window, cx);
                    cx.stop_propagation();
                }
            })
        };
        let ring = |b: gpui::Stateful<gpui::Div>, handle: &FocusHandle, ink: &'static str| b.track_focus(handle).focus_visible(move |style| style.border_color(look.color(ink)));

        let mark = plan.approved.then(|| {
            labelled(div().id("plan-sheet-approved").flex_none().tooltip(tip("Plan approved")).child(glyph(look, "badge-check", Size::Mark, look.color(token::STEP_ADVANCED))), Role::Image, "Plan approved")
        });
        let close = labelled(
            div()
                .id("plan-sheet-close")
                .track_focus(&focus.close)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close \u{b7} Esc"))
                .child(glyph(look, "x", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.close_plan_sheet(window, cx)))
                .on_key_down(pressed(Self::close_plan_sheet)),
            Role::Button,
            "Close",
        );
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .h(look.length(token::H_ROW_UNIT) + look.length(token::SPACE_6))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(look.length(token::LEADING_LG))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child("Plan"),
            )
            .children(mark)
            .child(close);

        // A plan recorded again reads first as the diff the thread's card draws.
        let card = held.head.plan.as_ref().filter(|card| card.version == plan.version);
        let diff = card.zip(plan.changes.as_ref()).filter(|_| revision).map(|(card, changes)| self.plan_revision(SHEET, &card.id, &plan, changes, page, look, cx));
        let sketch = plan.sketch.as_ref().map(|sketch| {
            let scene = super::review::sketch_card(&held.head, &held.items, &sketch.id).and_then(|card| match super::cards::shown_scene(card, None) {
                (key, Some(scene)) => Some(crate::scene::view(key.into(), scene, crate::scene::Fit::Preview, crate::scene::Clock::default(), crate::scene::Lens::default()).into_any_element()),
                _ => None,
            });
            labelled(
                div()
                    .id("plan-sheet-sketch")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_SM))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(glyph(look, "pencil-ruler", Size::Small, look.color(token::FG_MUTED)))
                            .child(SharedString::from(sketch.title.clone())),
                    )
                    .child(div().rounded(look.length(token::RADIUS_MD)).overflow_hidden().child(glass(look, false, scene))),
                Role::Figure,
                format!("Sketch {}", sketch.title),
            )
        });
        let body = div()
            .id("plan-sheet-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::PAD_CARD) + look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(self.plan_editor(&id, &plan, diff, false, true, look, cx))
            .children(sketch);

        let foot = (marked || !plan.approved).then(|| {
            let switch = (revision && !plan.approved && !marked).then(|| {
                let said = if editing { "Show changes" } else { "Edit plan" };
                labelled(
                    ring(button(look, "plan-sheet-switch", said, Tone::Ghost, false, None, true), &focus.switch, token::ACCENT)
                        .on_click(cx.listener(|this, _, _, cx| this.plan_sheet_switch(cx)))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                                this.plan_sheet_switch(cx);
                                cx.stop_propagation();
                            }
                        })),
                    Role::Button,
                    said,
                )
            });
            let send = marked.then(|| {
                labelled(
                    ring(button_frame(look, "plan-sheet-send", Tone::Secondary, false, true), &focus.send, token::ACCENT)
                        .child(glyph(look, "send", Size::Small, look.color(token::FG_DEFAULT)))
                        .child("Send changes")
                        .on_click(cx.listener(|this, _, window, cx| this.plan_sheet_send(window, cx)))
                        .on_key_down(pressed(Self::plan_sheet_send)),
                    Role::Button,
                    "Send changes",
                )
            });
            let approve = (!plan.approved).then(|| {
                labelled(
                    ring(button(look, "plan-sheet-approve", "Approve", Tone::Primary, false, None, true), &focus.approve, token::TEXT_ON_ACCENT)
                        .tooltip(tip(if marked { "Approve with these changes" } else { "Approve" }))
                        .on_click(cx.listener(|this, _, window, cx| this.plan_sheet_approve(window, cx)))
                        .on_key_down(pressed(Self::plan_sheet_approve)),
                    Role::Button,
                    "Approve",
                )
            });
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_6))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(switch)
                .children(send)
                .children(approve)
        });

        let margin = look.length(token::SPACE_4);
        let wide = (viewport.width * 0.76).max(look.length(token::W_DIALOG_WIDE)).min(viewport.width - margin * 2.);
        let sheet = labelled(
            div()
                .id("plan-sheet")
                .capture_key_down(cx.listener(Self::plan_sheet_key))
                .occlude()
                .w(wide)
                .h(viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.)
                .mt(look.length(token::H_TITLE_BAR) + margin)
                .mr(margin)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(head)
                .child(body)
                .children(foot),
            Role::Dialog,
            "Plan",
        );
        Some(
            deferred(
                anchored().position(room.origin).anchor(Anchor::TopLeft).child(
                    // A press on the ground behind takes nothing: the close and Esc are the two exits.
                    div().id("plan-sheet-scrim").occlude().w(viewport.width).h(viewport.height).flex().justify_end().bg(look.color(token::SCRIM)).child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }
}
