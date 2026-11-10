//! The rows that are one line or one bar: slots, commands, started and waiting Jobs, a change of a
//! value (`job-view.md`, A7). `records.rs` holds the rows that carry a body.

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{LinkedEvent, SessionRow, WaiterEvent};

use super::menus::teach_entries;
use super::rows::{eyebrow, glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

impl JobView {
    /// A row Fleet serves as it came (`ThreadItem::Row`), by its kind. `None` is a row no screen draws.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn row(
        &self,
        ix: usize,
        row: &SessionRow,
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        _window: &mut gpui::Window,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        match row {
            SessionRow::Message { .. } => self.message(row, page, look, now, cx),
            SessionRow::Ran { .. }
            | SessionRow::Requested { .. }
            | SessionRow::Saved { .. }
            | SessionRow::Retro { .. }
            | SessionRow::Sent { .. }
            | SessionRow::PlanChanges { .. }
            | SessionRow::Disputed { .. }
            | SessionRow::Draft { .. }
            | SessionRow::WindowAsk { .. }
            | SessionRow::Ask { .. } => Some(self.body_row(ix, row, page, look, now, cx)),
            // Never drawn in the thread: the plan's card is its own item, and the others have no screen.
            SessionRow::Plan { .. } | SessionRow::Delivery { .. } | SessionRow::BrowserReminder { .. } => None,
            _ => self.line(ix, row, page, look, now, cx),
        }
    }

    fn line(&self, ix: usize, row: &SessionRow, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        Some(match row {
            SessionRow::Tool { id, text, .. } => div()
                .id(SharedString::from(format!("command:{id}")))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(text.clone())
                .into_any_element(),
            SessionRow::Command { id, at, text } => {
                let spot = self.spot(page, &format!("command:{id}"), cx);
                let command = div()
                    .id(SharedString::from(format!("command:{id}")))
                    .track_focus(&spot.focus)
                    .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(text.clone());
                let command = self.menu_on(command, &spot, "Command", spot.focus.clone(), Rc::new(teach_entries), self.teach_pick("command", text.clone(), Some(at.clone())), cx);
                labelled(command, Role::ListItem, text.clone()).into_any_element()
            }
            SessionRow::Compaction { id, text, .. } => {
                let head = vec![
                    glyph(look, "layers", Size::Small, look.color(token::FG_MUTED)).into_any_element(),
                    div().child("Conversation compacted").into_any_element(),
                ];
                let body = self.prose(format!("compaction:{id}"), text, look);
                self.fold(ix, format!("compaction:{id}"), "Conversation compacted".into(), head, Some(body), page, look, cx)
            }
            SessionRow::Lease { id, at, slot, branch } | SessionRow::Release { id, at, slot, branch } => {
                let leased = matches!(row, SessionRow::Lease { .. });
                let held = look.color(token::SLOT_HELD);
                let said = format!("{} slot {slot}", if leased { "Leased" } else { "Released" });
                labelled(
                    div()
                        .id(SharedString::from(format!("lease:{id}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .w_full()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .min_w_0()
                                .px(look.length(token::SPACE_3))
                                .py(look.length(token::SPACE_1))
                                .rounded(look.length(token::RADIUS_LG))
                                .bg(held.opacity(0.1))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_SM))
                                .text_color(look.color(token::FG_MUTED))
                                .child(glyph(look, "key-round", Size::Large, held))
                                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(held).child(said))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(look.length(token::SPACE_1))
                                        .truncate()
                                        .text_color(look.color(token::FG_SUBTLE))
                                        .child(glyph(look, "git-branch", Size::Small, look.color(token::FG_SUBTLE)))
                                        .child(branch.clone()),
                                ),
                        )
                        .child(self.stamp(at, look, now, false, cx)),
                    Role::Region,
                    if leased { "Slot leased" } else { "Slot released" },
                )
                .into_any_element()
            }
            SessionRow::Window { id, title, .. } => self.window_row(id, title, look),
            SessionRow::Started { id, at, session, fork } => {
                let ink = look.color(token::FG_INVERSE);
                let bar = Self::padded(look, div())
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .w_full()
                    .bg(look.color(token::FG_DEFAULT))
                    .text_color(ink)
                    .child(glyph(look, if *fork { "git-fork" } else { "corner-up-right" }, Size::Large, ink))
                    .child(eyebrow(look, if *fork { "Forked" } else { "Started" }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_SM))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .child(session.title.clone()),
                    )
                    .child(self.stamp(at, look, now, true, cx))
                    .child(glyph(look, "chevron-right", Size::Large, ink));
                self.opens(session.id.as_str(), format!("started:{id}"), session.title.clone(), cx, bar)
            }
            SessionRow::Linked { id, at, session, event, replaces } => {
                if replaces.is_some() {
                    return None;
                }
                let (said, icon, ink) = match event {
                    LinkedEvent::Needs => ("Needs you", "eye", token::STATUS_AWAITING_REVIEW),
                    LinkedEvent::Done => ("Done", "power", token::STEP_ADVANCED),
                    LinkedEvent::Merged => ("Merged", "git-merge", token::STEP_ADVANCED),
                    LinkedEvent::Started => ("Started", "circle-dot", token::FG_MUTED),
                };
                let edge = if *event == LinkedEvent::Needs { token::STATUS_AWAITING_REVIEW } else { token::BORDER_SUBTLE };
                let bar = Self::padded(look, div())
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .w_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(edge))
                    .bg(look.color(token::BG_SUNKEN))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(
                        div()
                            .id(SharedString::from(format!("linked-mark:{id}")))
                            .child(glyph(look, "link", Size::Small, look.color(token::FG_MUTED)))
                            .tooltip(tip("Linked Job")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .child(session.title.clone()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("linked-event:{id}")))
                            .child(glyph(look, icon, Size::Small, look.color(ink)))
                            .tooltip(tip(said)),
                    )
                    .child(self.stamp(at, look, now, true, cx))
                    .child(glyph(look, "chevron-right", Size::Small, look.color(token::FG_MUTED)));
                self.opens(session.id.as_str(), format!("linked:{id}"), format!("{}, {said}", session.title), cx, bar)
            }
            SessionRow::Waiter { id, at, session, event, side, says, from, to, by } => {
                let waiting = *side == ipc::WaiterSide::Waiting;
                let said = match (event, waiting) {
                    (WaiterEvent::Waits, false) => "Waits on you",
                    (WaiterEvent::Waits, true) => "Waits on",
                    (WaiterEvent::Changed, _) if matches!(by, Some(ipc::WaitBy::You)) => "You changed it",
                    (WaiterEvent::Changed, _) => "Condition changed",
                    (WaiterEvent::Started, _) => "Started",
                };
                let ink = look.color(token::FG_INVERSE);
                let bar = Self::padded(look, div())
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .w_full()
                    .bg(look.color(token::FG_DEFAULT))
                    .text_color(ink)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .child(glyph(look, "link", Size::Small, ink))
                            .child(eyebrow(look, said))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(look.length(token::TEXT_SM))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                                    .child(session.title.clone()),
                            )
                            .child(self.stamp(at, look, now, true, cx))
                            .child(glyph(look, "chevron-right", Size::Large, ink)),
                    )
                    .children(says.clone().map(|says| {
                        div()
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .child(says.lines().next().unwrap_or("").to_string())
                    }))
                    .children(from.clone().zip(to.clone()).map(|(from, to)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_XS))
                            .child(from)
                            .child(glyph(look, "arrow-right", Size::Small, ink))
                            .child(to)
                    }));
                self.opens(session.id.as_str(), format!("waiter:{id}"), format!("{}, {}", session.title, said.to_lowercase()), cx, bar)
            }
            SessionRow::Check { id, name, state, target, .. } => self.plain(
                format!("check:{id}"),
                format!("Check {name}"),
                look,
                vec![
                    self.check_mark(*state, look),
                    div().child(name.clone()).into_any_element(),
                    div().text_color(look.color(token::FG_SUBTLE)).child(target.clone().unwrap_or_default()).into_any_element(),
                ],
            ),
            _ => return None,
        })
    }

    /// The person changed a value of the Job: a bar in the `you` colour (A7.16).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn change(
        &self,
        id: &str,
        at: &ipc::Instant,
        what: ipc::ChangedValue,
        from: &str,
        to: &str,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let word = match what {
            ipc::ChangedValue::Workflow => "Workflow",
            ipc::ChangedValue::Model => "Model",
            ipc::ChangedValue::Effort => "Effort",
        };
        let ink = look.color(token::FG_INVERSE);
        labelled(
            Self::padded(look, div())
                .id(SharedString::from(format!("change:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .bg(look.color(token::YOU))
                .text_color(ink)
                .child(eyebrow(look, word))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .child(from.to_string())
                        .child(glyph(look, "arrow-right", Size::Small, ink))
                        .child(to.to_string()),
                )
                .child(self.stamp(at, look, now, true, cx)),
            Role::Region,
            format!("{word} changed"),
        )
        .into_any_element()
    }

    /// The Job's agent never started (A7.2): a dashed red line, and its log when there is one.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn not_started(
        &self,
        ix: usize,
        id: &str,
        at: &ipc::Instant,
        cause: &str,
        log: &[String],
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let red = look.color(token::STATUS_COMPLETED_FAILED);
        let head = vec![
            glyph(look, "x", Size::Small, red).into_any_element(),
            div().flex_1().min_w_0().truncate().child(format!("Did not start: {cause}")).into_any_element(),
            self.stamp(at, look, now, true, cx),
        ];
        let body = (!log.is_empty()).then(|| {
            div()
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(token::FG_DEFAULT))
                .child(log.join("\n"))
                .into_any_element()
        });
        let frame = div()
            .py(look.length(token::SPACE_1))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_dashed()
            .border_color(look.color(token::BORDER_DEFAULT))
            .font_family(mono(look))
            .text_color(red)
            .child(self.fold(ix, format!("start-failed:{id}"), "Failed to start".into(), head, body, page, look, cx));
        labelled(div().id(SharedString::from(format!("not-started:{id}"))).child(frame), Role::Region, "Failed to start")
            .into_any_element()
    }
}
