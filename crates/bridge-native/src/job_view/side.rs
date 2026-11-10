//! The Job's right column, as Electron's `StoryColumn` draws it in its two default panes: Open, Done
//! and Deferred over Now. The panes' Add tab and their close are not drawn, since the other pane kinds
//! (Requests, Checks, Subagents and the rest) are not built yet.
//!
//! Now's owed rows read Fleet's `waiting_for` alone, one row per item, where Electron mixes in the
//! thread's walk rows and the asked question and so keeps dismissed walks and draws a question twice.
//! A finished task of the agent's own list is not listed in Now (his word, 10 Nov).

use std::collections::HashSet;

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, Role, SharedString, Subscription, Window};
use ipc::{ApprovePlan, ConfirmLater, LaterHow, PlanTaskState, SendPlanChanges, SessionId, SessionPlanGroup, SessionTodoState, SettleLater, WaitingItem, WaitingSource};

use super::acts::{button, Tone};
use super::head::icon_button;
use super::plan::task_look;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What Ask for an update sends (`ASK_FOR_UPDATE`, `cockpit/deck/acts.ts`).
const ASK_FOR_UPDATE: &str = "Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.";

/// The top pane's tab.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Zone {
    #[default]
    Open,
    Done,
    Deferred,
}

impl Zone {
    const ALL: [Zone; 3] = [Zone::Open, Zone::Done, Zone::Deferred];

    fn look(self) -> (&'static str, &'static str) {
        match self {
            Zone::Open => ("circle", "Open"),
            Zone::Done => ("circle-check", "Done"),
            Zone::Deferred => ("circle-dashed", "Deferred"),
        }
    }
}

/// The column's state on a Job's page: which groups are unfolded.
#[derive(Default)]
pub(super) struct SideState {
    unfolded: HashSet<String>,
}

/// Where a group of an approved plan stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stands {
    Coming,
    Working,
    Done,
}

fn stands(group: &SessionPlanGroup) -> Stands {
    if group.tasks.iter().any(|task| task.state == PlanTaskState::Working) {
        Stands::Working
    } else if !group.tasks.is_empty() && group.tasks.iter().all(|task| matches!(task.state, PlanTaskState::Done | PlanTaskState::HandedIn)) {
        Stands::Done
    } else {
        Stands::Coming
    }
}

/// The column, drawn by the Job view so its presses act on the Job: the deck puts it on the right.
pub struct JobSide {
    view: Entity<JobView>,
    _watch: Subscription,
}

impl JobSide {
    pub fn new(view: Entity<JobView>, cx: &mut Context<Self>) -> Self {
        let _watch = cx.observe(&view, |_, _, cx| cx.notify());
        Self { view, _watch }
    }
}

impl Render for JobSide {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view.update(cx, |view, cx| view.render_side(window, cx))
    }
}

fn pane(look: &Look) -> gpui::Div {
    div()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(look.color(token::BG_RAISED))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
}

fn strip(look: &Look) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_stretch()
        .h(look.length(token::H_ROW_HEADER))
        .bg(look.color(token::BG_SUNKEN))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
}

fn tab(look: &Look, id: &'static str, icon: &'static str, said: &'static str, front: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
        .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
        .when(front, |tab| tab.bg(look.color(token::BG_RAISED)))
        .when(!front, |tab| tab.cursor_pointer().hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
        .child(glyph(look, icon, Size::Small, look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED })))
        .child(said)
}

/// One line of a zone, as `.sv-beat` draws it: a glyph, the words, and what stands after them.
fn beat(look: &Look, icon: &'static str, ink: crate::look::Token, words: impl Into<SharedString>) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_2))
        .min_h(look.length(token::H_ROW))
        .py(look.length(token::SPACE_1))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph(look, icon, Size::Small, look.color(ink))))
        .child(div().flex_1().min_w_0().child(words.into()))
}

impl JobView {
    pub(super) fn render_side(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let look = &look;
        let zone = self.zone;
        let tabs = Zone::ALL.into_iter().map(|one| {
            let (icon, said) = one.look();
            let id = match one {
                Zone::Open => "side-open",
                Zone::Done => "side-done",
                Zone::Deferred => "side-deferred",
            };
            labelled(
                tab(look, id, icon, said, one == zone).on_click(cx.listener(move |this, _, _, cx| {
                    this.zone = one;
                    cx.notify();
                })),
                Role::Tab,
                format!("Job column: {said}"),
            )
        });
        let top_body = self.shown.clone().map(|id| match zone {
            Zone::Open => self.side_groups(&id, Stands::Coming, look, cx),
            Zone::Done => self.side_groups(&id, Stands::Done, look, cx),
            Zone::Deferred => self.side_deferred(&id, look, cx),
        });
        let ask = self.shown.clone().filter(|id| !self.closed(id, cx)).map(|_| {
            labelled(
                icon_button(look, "side-ask-update", "message-circle-more", "Ask for an update".into(), false, false)
                    .on_click(cx.listener(|this, _, window, cx| this.send_line(ASK_FOR_UPDATE.to_string(), window, cx))),
                Role::Button,
                "Ask for an update",
            )
        });
        let now_body = self.shown.clone().map(|id| self.side_now(&id, look, cx));
        let body = |id: &'static str, inner: Option<AnyElement>| {
            div()
                .id(id)
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_4))
                .children(inner)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::GAP_PANEL))
            .child(
                labelled(
                    pane(look).id("side-top").flex_grow(2.).flex_shrink(1.).flex_basis(gpui::px(0.)).child(strip(look).children(tabs)).child(body("side-top-body", top_body)),
                    Role::Region,
                    "Open, Done and Deferred",
                ),
            )
            .child(labelled(
                pane(look)
                    .id("side-now")
                    .flex_grow(3.)
                    .flex_shrink(1.)
                    .flex_basis(gpui::px(0.))
                    .child(
                        strip(look)
                            .child(labelled(tab(look, "side-now-tab", "activity", "Now", true), Role::Tab, "Job column: Now"))
                            .child(div().flex_1())
                            .child(div().flex().items_center().pr(look.length(token::SPACE_1)).children(ask)),
                    )
                    .child(body("side-now-body", now_body)),
                Role::Region,
                "Job column: Now",
            ))
            .into_any_element()
    }

    /// The groups of an approved plan that stand as `want`, each a line that unfolds to its tasks.
    fn side_groups(&self, id: &SharedString, want: Stands, look: &Look, cx: &Context<Self>) -> AnyElement {
        let plan = self.record(id, cx).and_then(|job| job.plan.clone()).filter(|plan| plan.approved);
        let unfolded = self.pages.get(id.as_str()).map(|page| page.side.unfolded.clone()).unwrap_or_default();
        let groups = plan.iter().flat_map(|plan| plan.groups.iter()).filter(|group| stands(group) == want).map(|group| {
            let open = unfolded.contains(&group.id);
            let key = group.id.clone();
            let (icon, ink) = match want {
                Stands::Done => ("check", token::STEP_ADVANCED),
                Stands::Working => ("circle-dot", token::STEP_RUNNING),
                Stands::Coming => ("circle-dashed", token::FG_SUBTLE),
            };
            let head = labelled(
                div()
                    .id(SharedString::from(format!("side-group:{}", group.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_ROW))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .rounded(look.length(token::RADIUS_MD))
                    .child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_SUBTLE)))
                    .child(glyph(look, icon, Size::Small, look.color(ink)))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(group.title.clone())))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(page) = this.shown.clone().and_then(|id| this.pages.get_mut(&id)) {
                            if !page.side.unfolded.remove(&key) {
                                page.side.unfolded.insert(key.clone());
                            }
                        }
                        cx.notify();
                    })),
                Role::Button,
                format!("Plan group: {}", group.title),
            );
            let tasks = open.then(|| {
                div().pl(look.length(token::SPACE_4)).children(group.tasks.iter().map(|task| {
                    let (icon, ink, _) = task_look(task.state);
                    beat(look, icon, ink, task.text.clone())
                }))
            });
            div().child(head).children(tasks)
        });
        div().flex().flex_col().children(groups).into_any_element()
    }

    /// The follow-ups the agent kept and he has not confirmed, each with Remind, Deferred and Not deferred.
    fn side_deferred(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(job) = self.record(id, cx) else { return div().into_any_element() };
        let rows = job.later.iter().filter(|later| !later.confirmed).map(|later| {
            let (remind, confirm, drop) = (later.sentence.clone(), later.id.clone(), later.id.clone());
            let act = |name: String, icon: &'static str, said: &'static str, on: Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>| {
                labelled(
                    div()
                        .id(SharedString::from(name))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::H_CONTROL_SM))
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .tooltip(tip(said))
                        .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| on(this, window, cx))),
                    Role::Button,
                    said,
                )
            };
            div()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .child(div().flex_1().min_w_0().pt(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(later.sentence.clone())))
                .child(act(format!("remind:{}", later.id), "bell-ring", "Remind", Box::new(move |this, window, cx| {
                    this.send_line(format!("You said: \"{remind}\" Do it, file it, or drop it with a reason."), window, cx);
                })))
                .child(act(format!("confirm:{}", later.id), "thumbs-up", "Deferred", Box::new(move |this, window, cx| this.later_confirm(confirm.clone(), window, cx))))
                .child(act(format!("drop:{}", later.id), "thumbs-down", "Not deferred", Box::new(move |this, window, cx| this.later_drop(drop.clone(), window, cx))))
        });
        div().flex().flex_col().children(rows).into_any_element()
    }

    pub(super) fn side_unfolded(&self, key: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id.as_str())).is_some_and(|page| page.side.unfolded.contains(key))
    }

    /// Unfold or fold a line of the column, by its key.
    pub(super) fn side_fold(&mut self, key: &str) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            if !page.side.unfolded.remove(key) {
                page.side.unfolded.insert(key.to_string());
            }
        }
    }

    fn later_confirm(&mut self, later: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let body = ipc::encode(&ConfirmLater { id: later, session_id: Some(SessionId::carried(job.to_string())) }).map_err(|why| format!("{why:?}"));
        self.post_with(job, "/sessions/later/confirm", body, |_, _, _| {}, window, cx);
    }

    fn later_drop(&mut self, later: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let settle = SettleLater { id: later, how: LaterHow::Dropped, issue: None, reason: None, session_id: Some(SessionId::carried(job.to_string())), misread: true };
        let body = ipc::encode(&settle).map_err(|why| format!("{why:?}"));
        self.post_with(job, "/sessions/settle_later", body, |_, _, _| {}, window, cx);
    }

    /// What waits on him, what the agent is on, the groups at work and its open tasks, then a plan he
    /// has not approved, edited in place.
    fn side_now(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(job) = self.record(id, cx).cloned() else { return div().into_any_element() };
        let owed = job.waiting_for.iter().filter(|item| item.source != WaitingSource::Done).map(|item| self.owed_row(item, look, cx)).collect::<Vec<_>>();
        let agent = self.standing_mark(id, cx).map(|(icon, said, hue, _)| beat(look, icon, hue, said));
        let working = self.side_groups(id, Stands::Working, look, cx);
        let todos = job.activity.iter().flat_map(|activity| activity.todos.iter()).filter(|todo| todo.state != SessionTodoState::Done).map(|todo| match todo.state {
            SessionTodoState::Doing => beat(look, "circle-dot", token::STEP_RUNNING, todo.text.clone()),
            _ => beat(look, "circle-dashed", token::FG_SUBTLE, todo.text.clone()),
        });
        let marks = self.plan_marks(id);
        let drafting = job.plan.clone().filter(|plan| !plan.approved || !marks.is_empty()).filter(|_| !self.review_open()).map(|plan| {
            let send = labelled(
                button(look, "side-plan-send", "Send changes", Tone::Secondary, false, None, !marks.is_empty())
                    .when(!marks.is_empty(), |send| send.on_click(cx.listener(|this, _, window, cx| this.side_plan(false, window, cx)))),
                Role::Button,
                "Send the plan's changes",
            );
            let approve = labelled(
                button(look, "side-plan-approve", "Approve", Tone::Primary, false, None, true)
                    .tooltip(tip(if marks.is_empty() { "Approve the plan" } else { "Approve with these changes" }))
                    .on_click(cx.listener(|this, _, window, cx| this.side_plan(true, window, cx))),
                Role::Button,
                "Approve the plan",
            );
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_3))
                .child(self.plan_editor(id, &plan, None, true, look, cx))
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(send).child(approve))
        });
        div().flex().flex_col().children(owed).children(agent).child(working).children(todos).children(drafting).into_any_element()
    }

    /// One thing waiting on him: a press opens it where it is answered.
    fn owed_row(&self, item: &WaitingItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.id.clone();
        let source = item.source;
        labelled(
            div()
                .id(SharedString::from(format!("owed:{}", item.id)))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_ROW))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(div().id(SharedString::from(format!("owed-mark:{}", item.id))).flex_none().pt(look.length(token::SPACE_1) / 2.).tooltip(tip("Waiting on you")).child(glyph(look, "megaphone", Size::Small, look.color(token::STATUS_AWAITING_REVIEW))))
                .child(div().flex_1().min_w_0().line_clamp(2).child(SharedString::from(item.text.clone())))
                .on_click(cx.listener(move |this, _, window, cx| this.open_owed(&key, source, window, cx))),
            Role::Button,
            format!("Waiting on you: {}", item.text),
        )
        .into_any_element()
    }

    fn open_owed(&mut self, item: &str, source: WaitingSource, window: &mut Window, cx: &mut Context<Self>) {
        match source {
            WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft => {
                self.open_review(window, cx);
                self.review_at(item, window, cx);
            }
            WaitingSource::FirstMessage | WaitingSource::Done => {}
            _ => {
                // A question's item is `ask:<call>`, which the panel holds as `question:<call>`.
                let key = item.strip_prefix("ask:").map_or_else(|| format!("item:{item}"), |call| format!("question:{call}"));
                self.set_needs(true, Some(key), window, cx);
            }
        }
    }

    /// Approve or Send changes from Now, with what he marked.
    fn side_plan(&mut self, approve: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((job, plan)) = self.plan_of_shown(cx) else { return };
        let marks = self.take_marks(&job);
        let session_id = SessionId::carried(job.to_string());
        let (route, body) = if approve && marks.is_empty() {
            ("/sessions/plan/approve", ipc::encode(&ApprovePlan { session_id, version: plan.version }))
        } else {
            ("/sessions/plan/changes", ipc::encode(&SendPlanChanges { session_id, version: plan.version, marks, approve }))
        };
        self.post_with(job, route, body.map_err(|why| format!("{why:?}")), |_, _, _| {}, window, cx);
        cx.notify();
    }
}
