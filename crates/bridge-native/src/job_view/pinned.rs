//! What the thread pins at its foot while the agent works: Stop (A1.10) and the card of the
//! subagents running now (A4.3).

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{SessionId, StopSession, ThreadItem, ToolSubagent, ToolSubagentState};

use super::acts::refusal;
use super::rows::{elapsed_words, glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

const STOP_KEY: &str = "stop";

impl JobView {
    /// The card of the subagents running, held in sight at the thread's foot while the rows scroll under it
    /// (`position: sticky; bottom: space-2`): where it stands in the thread it is drawn there, and while that
    /// place is below what is on screen it stands at the foot of the view instead.
    pub(super) fn render_subagents_pin(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let at = page.extras.iter().position(|extra| matches!(extra, super::Extra::Subagents))?;
        let item_ix = page.lead + page.items + at;
        let viewport = page.list.viewport_bounds();
        let seen = page.list.bounds_for_item(item_ix).is_some_and(|bounds| bounds.bottom() <= viewport.bottom());
        if seen {
            return None;
        }
        let held = self.thread(id, cx)?;
        let look = cx.look();
        let card = self.subagents_card(item_ix, &held.items, page, look, Local::now(), cx)?;
        Some(
            div()
                .w_full()
                .px(look.length(token::SPACE_6))
                .flex()
                .justify_center()
                .child(div().w_full().min_w_0().max_w(look.length(token::SPACE_12) * 20.).child(card))
                .into_any_element(),
        )
    }

    /// What stands at the foot of the view over the rows: the chip to what waits below, over the subagents card.
    pub(super) fn render_foot(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let (chip, pin) = (self.render_callchip(id, cx), self.render_subagents_pin(id, cx));
        if chip.is_none() && pin.is_none() {
            return None;
        }
        let look = cx.look();
        Some(div().absolute().bottom(look.length(token::SPACE_2)).left_0().right_0().flex().flex_col().gap(look.length(token::SPACE_2)).children(chip).children(pin).into_any_element())
    }

    /// Stop: ends what the agent is doing. The Job stays live and its next message resumes it.
    pub(super) fn stop_button(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            div()
                .id("stop")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Stop"))
                .child(glyph(look, "square", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    let Some(job) = this.shown.clone() else { return };
                    let body = ipc::encode(&StopSession { session_id: SessionId::carried(job.to_string()) }).map_err(|why| format!("{why:?}"));
                    this.post_with(
                        job.clone(),
                        "/sessions/stop",
                        body,
                        move |this, why, cx| {
                            if let Some(page) = this.pages.get_mut(&job) {
                                page.cards.said.insert(STOP_KEY.into(), why);
                            }
                            cx.notify();
                        },
                        window,
                        cx,
                    );
                })),
            Role::Button,
            "Stop",
        )
        .into_any_element()
    }

    /// Why Stop was refused, under the line it sits in.
    pub(super) fn stop_said(&self, page: &Page, look: &Look) -> Option<AnyElement> {
        page.cards.said.get(STOP_KEY).map(|why| refusal(look, STOP_KEY, why.clone()))
    }

    /// The subagents running now: a card at the foot with the count, and the list behind a press.
    pub(super) fn subagents_card(&self, ix: usize, items: &[ThreadItem], page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        let running: Vec<&ToolSubagent> = items
            .iter()
            .filter_map(|item| match item {
                ThreadItem::Subagent { subagent, .. } if subagent.state == ToolSubagentState::Running => Some(subagent),
                _ => None,
            })
            .collect();
        if running.is_empty() {
            return None;
        }
        let open = self.open_in_card("subagents");
        let count = if running.len() == 1 { "1 subagent running".to_string() } else { format!("{} subagents running", running.len()) };
        let toggle = labelled(
            div()
                .id("subagents-toggle")
                .flex_1()
                .min_w_0()
                .h(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .cursor_pointer()
                .rounded(look.length(token::RADIUS_MD))
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(div().id("subagents-mark").child(self.working_mark(look, look.color(token::STATUS_RUNNING))).tooltip(tip("Running")))
                .child(SharedString::from(count.clone()))
                .child(glyph(look, if open { "chevron-down" } else { "chevron-up" }, Size::Large, look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, "subagents", cx))),
            Role::Button,
            count,
        );
        let head = div().flex().items_center().gap(look.length(token::SPACE_1)).pr(look.length(token::SPACE_1)).child(toggle).child(self.stop_button(look, cx));
        let rows = open.then(|| {
            div()
                .flex()
                .flex_col()
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(running.iter().map(|subagent| {
                    let ran = elapsed_words(subagent.since.as_str(), &now.to_rfc3339());
                    let mark_said = match &subagent.agent_type {
                        Some(kind) => format!("Subagent running \u{b7} {kind}"),
                        None => "Subagent running".to_string(),
                    };
                    labelled(
                        div()
                            .id(SharedString::from(format!("subagent-live:{}", subagent.id)))
                            .flex()
                            .items_start()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_3))
                            .py(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .child(
                                div()
                                    .id(SharedString::from(format!("subagent-live-mark:{}", subagent.id)))
                                    .pt(look.length(token::SPACE_1))
                                    .child(glyph(look, "circle-dot", Size::Small, look.color(token::STEP_RUNNING)))
                                    .tooltip(tip(mark_said)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(look.length(token::SPACE_2))
                                            .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(subagent.task.clone())))
                                            .children(ran.map(|ran| {
                                                div().flex_none().text_size(look.length(token::TEXT_XS)).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(ran))
                                            })),
                                    )
                                    .children(subagent.latest.as_ref().map(|latest| {
                                        div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(latest.clone()))
                                    })),
                            ),
                        Role::Row,
                        format!("Subagent {}", subagent.task),
                    )
                }))
        });
        let _ = page;
        Some(
            labelled(
                div()
                    .id("subagents")
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(head)
                    .children(rows),
                Role::Region,
                "Subagents running",
            )
            .into_any_element(),
        )
    }
}
