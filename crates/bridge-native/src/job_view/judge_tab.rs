//! The Judge of the whole Job (`JudgeTab.tsx`): the toolbelt's Judge opens no box, it opens a pane of
//! its own on the right of the Job, as Electron's arrival rule splits a Job's panes for a kind it has
//! none of. At the pane's foot, what judging is and a box that asks; above that, what was asked and how
//! the reading stands: its shape while it reads, why it failed, or its notes. A clean reading says so.
//!
//! The pane's × shuts it; nothing else does, as a tab in Electron is closed by its ×.

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, Entity, FontWeight, Role, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{AskJudge, JudgeArtifact, JudgeKind, JudgeReading, JudgeState, SessionId, SessionRecord};

use super::acts::{Answer, SHEET};
use super::judge::{how_of, press_of, Pinned, Press};
use super::rows::{glyph, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// Where the whole Job's notes and refusals are kept on the page's cards.
const KEY: &str = "job";

/// The pane, while it is open on a Job: its box and what Enter in it does.
pub(super) struct JudgePane {
    area: Entity<TextareaState>,
    /// What was asked in this frame, read until Fleet's reading of it arrives.
    asked: Option<String>,
    _enter: Subscription,
}

/// The reading of the whole Job, going or kept.
pub(super) fn whole_reading<'a>(record: &'a SessionRecord, job: &str) -> Option<&'a JudgeReading> {
    record.judge_readings.iter().find(|one| one.artifact.kind == JudgeKind::Job && one.artifact.id == job)
}

/// The pane, drawn by the Job view so its presses act on the Job: the deck puts it on the right.
pub struct JobJudge {
    view: Entity<JobView>,
    _watch: Subscription,
}

impl JobJudge {
    pub fn new(view: Entity<JobView>, cx: &mut Context<Self>) -> Self {
        let _watch = cx.observe(&view, |_, _, cx| cx.notify());
        Self { view, _watch }
    }
}

impl Render for JobJudge {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view.update(cx, |view, cx| view.render_judge_pane(window, cx))
    }
}

impl JobView {
    /// The open Job's id.
    pub fn shown_id(&self) -> Option<SharedString> {
        self.shown.clone()
    }

    /// Whether the open Job has its Judge pane open.
    pub fn judge_pane_open(&self) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).is_some_and(|page| page.judge.is_some())
    }

    fn whole_state(&self, id: &SharedString, cx: &gpui::App) -> (Press, Option<String>) {
        let Some(record) = self.record(id, cx) else { return (Press::Idle, None) };
        let reading = whole_reading(record, id);
        let Some(page) = self.pages.get(id) else { return (Press::Idle, None) };
        let judging = matches!(page.cards.answers.get(&format!("judge:{KEY}")), Some(Answer::Judging(held)) if *held == reading.map(|one| one.id.clone()));
        let waiting = |note: &ipc::JudgeNote| reading.is_none_or(|one| how_of(page, one, note).is_none());
        let failed = reading.and_then(|one| one.failed.clone());
        (press_of(reading, judging, waiting), failed)
    }

    /// The toolbelt's Judge: a balance that reads the last reading's state, as `JudgeAct` draws it.
    pub(super) fn judge_act(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (state, failed) = self.whole_state(id, cx);
        let said = match state {
            Press::Reading => "Judge reading".to_string(),
            Press::Unread => "Notes to read".to_string(),
            Press::Clean => "Nothing missed".to_string(),
            Press::Failed => failed.unwrap_or_else(|| "Send a judge to read it".to_string()),
            Press::Idle => "Send a judge to read it".to_string(),
        };
        let mark = match state {
            Press::Reading => glyph(look, "circle-dot", Size::Mark, look.color(token::STEP_RUNNING))
                .with_animation("whole-judge-reading", Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.4, 1.)), |mark, delta| mark.opacity(delta))
                .into_any_element(),
            Press::Clean => glyph(look, "circle-check", Size::Mark, look.color(token::STEP_ADVANCED)).into_any_element(),
            _ => glyph(look, "scale", Size::Mark, look.color(token::FG_SUBTLE)).into_any_element(),
        };
        labelled(
            div()
                .id("act-judge")
                .relative()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(super::head::keyed_tip("judge", said.into()))
                .child(mark)
                .children((state == Press::Unread).then(|| {
                    div()
                        .absolute()
                        .top(look.length(token::SPACE_1))
                        .right(look.length(token::SPACE_1))
                        .size(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::STATUS_AWAITING_REVIEW))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BG_BASE))
                }))
                .on_click(cx.listener(|this, _, window, cx| this.open_judge_pane(window, cx))),
            Role::Button,
            "Judge",
        )
        .into_any_element()
    }

    /// Bring the pane forward, or open it, and put the keys in its box.
    pub(super) fn open_judge_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let box_key = format!("judge:{KEY}:Judge");
        let held = self.pages.get(&job).and_then(|page| page.cards.boxes.get(&box_key).cloned());
        let area = held.unwrap_or_else(|| cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).submit_on_enter(true)));
        let enter = cx.subscribe_in(&area, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.ask_whole(window, cx);
            }
        });
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.boxes.insert(box_key, area.clone());
            let asked = page.judge.take().and_then(|pane| pane.asked);
            page.judge = Some(JudgePane { area: area.clone(), asked, _enter: enter });
        }
        area.update(cx, |area, cx| area.focus(window, cx));
        self.judge_read(&job, cx);
        cx.notify();
    }

    /// The pane's ×: it shuts, and the keys go back to the Job's box.
    fn close_judge_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get_mut(id)) {
            page.judge = None;
        }
        self.to_composer(window, cx);
        cx.notify();
    }

    /// Ask the Judge of the whole Job. The pane reads as reading in this frame; Fleet's reading replaces it.
    fn ask_whole(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.whole_state(&job, cx).0 == Press::Reading {
            return;
        }
        let Some(area) = self.pages.get(&job).and_then(|page| page.judge.as_ref()).map(|pane| pane.area.clone()) else { return };
        let prompt = area.read(cx).value().trim().to_string();
        area.update(cx, |area, cx| area.set_value("", window, cx));
        let title = self.record(&job, cx).map(|record| record.title.clone().unwrap_or_default()).filter(|title| !title.is_empty()).unwrap_or_else(|| "the Job".to_string());
        let held = self.record(&job, cx).and_then(|record| whole_reading(record, &job)).map(|one| one.id.clone());
        let mark = format!("judge:{KEY}");
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.answers.insert(mark.clone(), Answer::Judging(held));
            page.cards.said.remove(KEY);
            if let Some(pane) = page.judge.as_mut() {
                pane.asked = Some(prompt.clone());
            }
        }
        cx.notify();
        let body = ipc::encode(&AskJudge { session_id: SessionId::carried(job.to_string()), artifact: JudgeArtifact { kind: JudgeKind::Job, id: job.to_string(), title }, prompt })
            .map_err(|why| format!("{why:?}"));
        let back = job.clone();
        self.post_with(
            job,
            "/sessions/judge",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.cards.answers.remove(&mark);
                    page.cards.said.insert(KEY.to_string(), why);
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    /// The pane: its tab and ×, the reading, and at its foot what judging is and the box.
    pub(super) fn render_judge_pane(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let look = &look;
        let Some(id) = self.shown.clone() else { return div().into_any_element() };
        let Some(page) = self.pages.get(&id) else { return div().into_any_element() };
        let Some(pane) = page.judge.as_ref() else { return div().into_any_element() };
        let (state, _) = self.whole_state(&id, cx);
        let reading = self.record(&id, cx).and_then(|record| whole_reading(record, &id));
        // While the asking is this frame's, what was asked is what was just typed.
        let judging = matches!(page.cards.answers.get(&format!("judge:{KEY}")), Some(Answer::Judging(_)));
        let prompt = if judging { pane.asked.clone() } else { reading.map(|one| one.prompt.clone()) };
        let refused = page.cards.said.get(KEY).cloned();
        let gate = look.color(token::KIND_GATE);
        let close = labelled(
            div()
                .id("judge-pane-close")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.close_judge_pane(window, cx))),
            Role::Button,
            "Close Judge",
        );
        let tab = labelled(
            div()
                .id("judge-pane-tab")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .child(glyph(look, "scale", Size::Small, look.color(token::FG_DEFAULT)))
                .child("Judge")
                .child(close),
            Role::Tab,
            "Judge",
        );
        let asked = prompt.map(|prompt| {
            let (term, said) = if prompt.is_empty() { ("Asked", "What did we miss?".to_string()) } else { ("Verify that", prompt) };
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(term))
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said)))
        });
        let said_line = |icon: Option<&'static str>, words: String, ink: gpui::Hsla| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(ink)
                .children(icon.map(|icon| glyph(look, icon, Size::Mark, look.color(token::STEP_ADVANCED))))
                .child(SharedString::from(words))
                .into_any_element()
        };
        let stands: Option<AnyElement> = if state == Press::Reading {
            Some(labelled(
                div().id("judge-pending").flex().flex_col().gap(look.length(token::SPACE_2)).children([0.3, 0.9, 0.6].into_iter().enumerate().map(|(at, wide)| {
                    div()
                        .h(look.length(token::SPACE_3))
                        .w(gpui::relative(wide))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::FG_DEFAULT).opacity(0.07))
                        .with_animation(SharedString::from(format!("judge-pending-{at}")), Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)), |shape, at| shape.opacity(at))
                })),
                Role::Status,
                "Judge reading",
            )
            .into_any_element())
        } else {
            reading.map(|one| match one.state {
            JudgeState::Failed => said_line(None, one.failed.clone().unwrap_or_default(), look.color(token::STEP_FAILED)),
            JudgeState::Done if one.notes.is_empty() => said_line(Some("circle-check"), "Nothing missed".to_string(), look.color(token::FG_DEFAULT)),
            _ => Pinned::whole_job(SHEET, KEY, one)
                .and_then(|at| self.notes_list(&at, at.whole(), "Notes from the judge".to_string(), page, look, cx))
                .unwrap_or_else(|| div().into_any_element()),
            })
        };
        let reading_box = div()
            .id("judge-pane-reading")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .children(asked)
            .children(stands)
            .children(refused.map(|why| super::acts::refusal(look, KEY, why)));
        let what = |words: &'static str| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(words);
        let reading_now = state == Press::Reading;
        let send = labelled(
            div()
                .id("judge-pane-send")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gate)
                .bg(gate.opacity(0.22))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(gate)
                .when(reading_now, |send| send.opacity(0.4))
                .when(!reading_now, |send| send.cursor_pointer().hover(|style| style.bg(gate.opacity(0.34))))
                .child(glyph(look, "send", Size::Small, gate))
                .child("Judge")
                .on_click(cx.listener(|this, _, window, cx| this.ask_whole(window, cx))),
            Role::Button,
            "Ask the Judge",
        );
        let field = labelled(
            div()
                .id("judge-pane-box")
                .key_context(crate::keymap::TEXT_BOX)
                .flex_1()
                .min_w_0()
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(Textarea::new(&pane.area).appearance(false).aria_label("Verify that")),
            Role::TextInput,
            "Verify that",
        );
        let composer = div()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gate)
            .bg(look.color(token::BG_SUNKEN))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .bg(gate.opacity(look.percent(token::STATUS_WASH)))
                    .child(field)
                    .child(send),
            );
        let foot = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(what("A Judge reads this Job once on the strongest model, read-only. It sees what you asked for, the work and the repository, and never the agent's account of it. A note reaches the agent only if you send it."))
            .child(what("Say what to verify. Left empty, it looks for what was missed."))
            .child(composer);
        labelled(
            div()
                .id("judge-pane")
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                .bg(look.color(token::BG_RAISED))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_t(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_stretch()
                        .h(look.length(token::H_ROW_HEADER))
                        .bg(look.color(token::BG_SUNKEN))
                        .border_b(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .child(tab),
                )
                .child(div().flex_1().min_h_0().flex().flex_col().gap(look.length(token::SPACE_3)).p(look.length(token::SPACE_3)).child(reading_box).child(foot)),
            Role::Region,
            "Judge",
        )
        .into_any_element()
    }
}
