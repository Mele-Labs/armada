//! The Judge on a plan (`job-view.md`, B2.8): a press in the plan card's answers row opens a small
//! box that asks what to verify; the reading's notes are pinned to the plan's summary, its groups and
//! its tasks, each with Send to Job and Dismiss, and the keys that run down them are the registry's
//! (`plan_judge`, `finding_move`, `judge_note_send`, `judge_note_dismiss`).
//!
//! **Only a press starts a reading, and a note reaches the agent only if the person sends it.** A
//! note settled here reads as settled in this frame; a refusal takes it back and says Fleet's reason
//! on the card. The reading itself rides on the Job's record, so nothing here holds a copy of it.

use std::collections::HashSet;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, Anchor, AnchoredPositionMode, Animation, AnimationExt, AnyElement, Context, Entity, FontWeight, KeyDownEvent,
    MouseButton, MouseDownEvent, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{
    AskJudge, DismissJudgeNote, JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeReading, JudgeState, MarkJudgeRead, SendJudgeNote, SessionId, SessionPlan,
    SessionPlanGroup, SessionRecord,
};

use super::acts::{button, Answer, Tone};
use super::keys::Found;
use super::rows::{glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::generated::actions::ACTIONS;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// The key the registry gives an act, as its tooltip says it.
fn shortcut_of(id: &str) -> &'static str {
    ACTIONS.iter().find(|row| row.id == id).map_or("", |row| row.shortcut)
}

/// The reading of the plan at `version`, going or kept.
pub(super) fn plan_reading(record: &SessionRecord, version: u32) -> Option<&JudgeReading> {
    let id = version.to_string();
    record.judge_readings.iter().find(|one| one.artifact.kind == JudgeKind::Plan && one.artifact.id == id)
}

/// What the Judge's press shows, from the last reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Idle,
    Reading,
    /// Done, with notes not yet opened and some still waiting.
    Unread,
    /// Done, with nothing to say.
    Clean,
    Failed,
}

fn press_of(reading: Option<&JudgeReading>, judging: bool, waiting: impl Fn(&JudgeNote) -> bool) -> Press {
    if judging {
        return Press::Reading;
    }
    match reading {
        None => Press::Idle,
        Some(one) => match one.state {
            JudgeState::Reading => Press::Reading,
            JudgeState::Failed => Press::Failed,
            JudgeState::Done if one.notes.is_empty() => Press::Clean,
            JudgeState::Done if !one.read && one.notes.iter().any(waiting) => Press::Unread,
            JudgeState::Done => Press::Idle,
        },
    }
}

/// The box the Judge's press opened, over the card.
pub(super) struct Open {
    job: SharedString,
    key: String,
    area: Entity<TextareaState>,
    _enter: Subscription,
}

/// Where the notes of a plan stand: only a reading that finished has any.
pub(super) struct Pinned<'a> {
    ix: usize,
    key: &'a str,
    reading: &'a JudgeReading,
    known: HashSet<&'a str>,
}

impl<'a> Pinned<'a> {
    pub fn new(ix: usize, key: &'a str, reading: Option<&'a JudgeReading>, plan: &'a SessionPlan) -> Option<Self> {
        let reading = reading.filter(|one| one.state == JudgeState::Done)?;
        let known = plan.groups.iter().flat_map(|group| std::iter::once(group.id.as_str()).chain(group.tasks.iter().map(|task| task.id.as_str()))).collect();
        Some(Self { ix, key, reading, known })
    }

    /// Notes about the plan as a whole, or about a part the plan no longer holds.
    pub fn whole(&self) -> Vec<&'a JudgeNote> {
        self.reading.notes.iter().filter(|note| note.about.as_deref().is_none_or(|about| !self.known.contains(about))).collect()
    }

    /// Notes about one group or task.
    pub fn at(&self, id: &str) -> Vec<&'a JudgeNote> {
        self.reading.notes.iter().filter(|note| note.about.as_deref() == Some(id)).collect()
    }
}

/// How the person settled a note: Fleet's record says, else what was pressed here.
fn how_of(page: &Page, reading: &JudgeReading, note: &JudgeNote) -> Option<JudgeNoteHow> {
    note.settled.as_ref().map(|settled| settled.how).or_else(|| page.cards.judged.get(&format!("{}/{}", reading.id, note.id)).copied())
}

impl JobView {
    // ---- The press and its box -------------------------------------------------------------

    /// The Judge's press for the plan card, and the box it opens under it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn judge_press(&self, ix: usize, key: &str, version: u32, reading: Option<&JudgeReading>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        let judging = matches!(page.cards.answers.get(&format!("judge:{key}")), Some(Answer::Judging(held)) if *held == reading.map(|one| one.id.clone()));
        let waiting = |note: &JudgeNote| reading.is_none_or(|one| how_of(page, one, note).is_none());
        let state = press_of(reading, judging, waiting);
        let said = match state {
            Press::Reading => "Judge reading".to_string(),
            Press::Unread => "Notes to read".to_string(),
            Press::Clean => "Nothing missed".to_string(),
            Press::Failed => reading.and_then(|one| one.failed.clone()).unwrap_or_else(|| "Send a judge to read it".to_string()),
            Press::Idle => "Send a judge to read it".to_string(),
        };
        let spot = page.keys.spot(&format!("judge-press:{key}"), cx);
        let edges = spot.bounds.clone();
        let open = self.judge.as_ref().is_some_and(|open| open.job == job && open.key == key);
        let noted = reading.is_some_and(|one| one.state == JudgeState::Done && !one.notes.is_empty());
        let mark = match state {
            Press::Reading => glyph(look, "circle-dot", Size::Mark, look.color(token::STEP_RUNNING))
                .with_animation("judge-reading", Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.4, 1.)), |mark, delta| mark.opacity(delta))
                .into_any_element(),
            Press::Clean => glyph(look, "circle-check", Size::Mark, look.color(token::STEP_ADVANCED)).into_any_element(),
            _ => glyph(look, "scale", Size::Mark, look.color(token::FG_MUTED)).into_any_element(),
        };
        let (toggle_key, enter_key) = (key.to_string(), key.to_string());
        let press = labelled(
            div()
                .id(SharedString::from(format!("judge:{key}")))
                .track_focus(&spot.focus)
                .relative()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                .when(open, |press| press.bg(look.color(token::BG_HOVER)))
                .tooltip(tip(format!("{said} ({})", shortcut_of("plan_judge"))))
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
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        this.toggle_judge(ix, &enter_key, version, window, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.toggle_judge(ix, &toggle_key, version, window, cx)))
                .child(canvas(move |bounds, _, _| edges.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
            Role::Button,
            "Judge plan",
        );
        let pop = open.then(|| self.judge_pop(ix, key, version, noted, &spot.bounds, look, cx));
        div().relative().flex_none().child(press).children(pop).into_any_element()
    }

    /// The box under the press: what to verify, and Judge.
    #[allow(clippy::too_many_arguments)]
    fn judge_pop(&self, ix: usize, key: &str, version: u32, noted: bool, anchor: &std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(open) = self.judge.as_ref() else { return div().into_any_element() };
        let (close_key, notes_key, out_key) = (key.to_string(), key.to_string(), key.to_string());
        let anchor = anchor.clone();
        let field = labelled(
            div()
                .id(SharedString::from(format!("judge-box:{key}")))
                .key_context(crate::keymap::TEXT_BOX)
                .w_full()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(Textarea::new(&open.area).appearance(false).aria_label("Verify that")),
            Role::TextInput,
            "Verify that",
        );
        let panel = labelled(
            div()
                .id(SharedString::from(format!("judge-pop:{key}")))
                .occlude()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .w(look.length(token::SIDEBAR_DEFAULT) * 1.6)
                .max_w_full()
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    // A key made in the box is not heard by the card or the thread under it.
                    cx.stop_propagation();
                    if event.keystroke.key == "escape" {
                        this.close_judge(ix, &close_key, true, window, cx);
                    }
                }))
                .on_mouse_down_out(cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    // A press on the act itself is its own toggle, not an outside press.
                    if !anchor.get().contains(&event.position) {
                        this.close_judge(ix, &out_key, false, window, cx);
                    }
                }))
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Verify that"))
                .child(field)
                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Empty: what did we miss?"))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(look.length(token::SPACE_2))
                        .children(noted.then(|| {
                            labelled(
                                button(look, SharedString::from(format!("judge-notes:{key}")), "Read the notes", Tone::Ghost, false, None, true).on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.close_judge(ix, &notes_key, false, window, cx);
                                        this.show_notes(window, cx);
                                    },
                                )),
                                Role::Button,
                                "Read the notes",
                            )
                        }))
                        .child(labelled(
                            button(look, SharedString::from(format!("judge-run:{key}")), "Judge", Tone::Primary, false, None, true).on_click(cx.listener(move |this, _, window, cx| this.run_judge(version, window, cx))),
                            Role::Button,
                            "Judge",
                        )),
                ),
            Role::Dialog,
            "Judge",
        );
        deferred(
            anchored()
                .anchor(Anchor::TopRight)
                .position_mode(AnchoredPositionMode::Local)
                .position(point(look.length(token::H_CONTROL), look.length(token::H_CONTROL) + look.length(token::SPACE_1)))
                .snap_to_window_with_margin(look.length(token::SPACE_2))
                .child(panel),
        )
        .with_priority(1)
        .into_any_element()
    }

    fn toggle_judge(&mut self, ix: usize, key: &str, version: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.judge.as_ref().is_some_and(|open| open.job == job && open.key == key) {
            self.close_judge(ix, key, true, window, cx);
        } else {
            self.open_judge(ix, key, version, window, cx);
        }
    }

    /// Open the Judge's box on the plan, with what was typed in it before, and put the caret there.
    pub(super) fn open_judge(&mut self, ix: usize, key: &str, version: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let box_key = format!("judge:{key}:Judge");
        let held = self.pages.get(&job).and_then(|page| page.cards.boxes.get(&box_key).cloned());
        let area = held.unwrap_or_else(|| {
            cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8).submit_on_enter(true).placeholder("existing sign-ins survive the switch"))
        });
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.boxes.insert(box_key, area.clone());
            page.cards.target = Some(key.to_string());
        }
        let enter = cx.subscribe_in(&area, window, move |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.run_judge(version, window, cx);
            }
        });
        self.judge = Some(Open { job, key: key.to_string(), area: area.clone(), _enter: enter });
        area.update(cx, |area, cx| area.focus(window, cx));
        self.card_changed(ix, cx);
    }

    /// Shut the box. What was typed stays for the next opening; a press that ran the Judge clears it.
    pub(super) fn close_judge(&mut self, ix: usize, key: &str, refocus: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.judge.as_ref().is_some_and(|open| open.key == key) {
            self.judge = None;
        }
        if refocus {
            let spot = self.shown.as_ref().and_then(|job| self.pages.get(job)).and_then(|page| page.keys.spot_of(&format!("judge-press:{key}")));
            if let Some(spot) = spot {
                window.focus(&spot.focus, cx);
            }
        }
        self.card_changed(ix, cx);
    }

    /// Ask the Judge. The press reads as reading in this frame; Fleet's reading replaces it.
    fn run_judge(&mut self, version: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.judge.take() else { return };
        let Open { job, key, area, .. } = open;
        let prompt = area.read(cx).value().trim().to_string();
        let held = self.record(&job, cx).and_then(|record| plan_reading(record, version)).map(|one| one.id.clone());
        let ix = self.plan_ix(&job, cx);
        let mark = format!("judge:{key}");
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.boxes.remove(&format!("judge:{key}:Judge"));
            page.cards.answers.insert(mark.clone(), Answer::Judging(held));
            page.cards.said.remove(&key);
            if let Some(spot) = page.keys.spot_of(&format!("judge-press:{key}")) {
                window.focus(&spot.focus, cx);
            }
        }
        if let Some(ix) = ix {
            self.card_changed(ix, cx);
        }
        let body = encoded(&AskJudge {
            session_id: SessionId::carried(job.to_string()),
            artifact: JudgeArtifact { kind: JudgeKind::Plan, id: version.to_string(), title: format!("Plan v{version}") },
            prompt,
        });
        let back = job.clone();
        self.post_with(
            job,
            "/sessions/judge",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.cards.answers.remove(&mark);
                    page.cards.said.insert(key, why);
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    /// The list place of the plan that waits: after the thread's items, where the head's plan stands.
    fn plan_ix(&self, job: &SharedString, cx: &gpui::App) -> Option<usize> {
        let page = self.pages.get(job)?;
        let held = self.thread(job, cx)?;
        let at = page.extras.iter().position(|extra| matches!(extra, super::Extra::Plan(_)))?;
        Some(held.items.len() + at)
    }

    /// Open the plan at its notes: the card comes into view and the first note waiting takes focus.
    pub(super) fn show_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(ix) = self.plan_ix(&job, cx) else { return };
        if let Some(page) = self.pages.get(&job) {
            page.list.scroll_to_reveal_item(page.lead + ix);
        }
        cx.notify();
        let view = cx.weak_entity();
        window.on_next_frame(move |window, cx| {
            view.update(cx, |this, cx| {
                let first = this.shown.as_ref().and_then(|job| this.pages.get(job)).and_then(|page| page.keys.found_all().into_iter().next().and_then(|one| page.keys.spot_of(&one.key)));
                if let Some(spot) = first {
                    window.focus(&spot.focus, cx);
                    cx.notify();
                }
            })
            .ok();
        });
    }

    // ---- The keys --------------------------------------------------------------------------

    /// A registry act made on the plan card. `live` says the card shows the Judge: waiting and open.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_act(&mut self, act: &Act, ix: usize, key: &str, version: u32, live: bool, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("plan_judge") {
            if live {
                self.open_judge(ix, key, version, window, cx);
            } else {
                cx.propagate();
            }
            return;
        }
        let Some(page) = self.shown.as_ref().and_then(|job| self.pages.get(job)) else { return cx.propagate() };
        let Some(at) = page.keys.found_focused(window) else { return cx.propagate() };
        let all = page.keys.found_all();
        if act.is("finding_move") {
            // Down, up, down, up: the registry's four slots.
            let down = act.slot % 2 == 0;
            let to = if down { (at + 1).min(all.len() - 1) } else { at.saturating_sub(1) };
            if let Some(spot) = page.keys.spot_of(&all[to].key) {
                window.focus(&spot.focus, cx);
            }
        } else if act.is("judge_note_send") {
            self.judge_settle(ix, key, &all[at], JudgeNoteHow::Sent, window, cx);
        } else if act.is("judge_note_dismiss") {
            self.judge_settle(ix, key, &all[at], JudgeNoteHow::Dismissed, window, cx);
        } else {
            cx.propagate();
        }
    }

    /// Send a note to the Job or put it away. It reads as settled in this frame and focus passes to the next
    /// note waiting; a refusal takes it back.
    pub(super) fn judge_settle(&mut self, ix: usize, plan_key: &str, found: &Found, how: JudgeNoteHow, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let route = match how {
            JudgeNoteHow::Sent => "/sessions/judge/send",
            JudgeNoteHow::Dismissed => "/sessions/judge/dismiss",
        };
        let session_id = SessionId::carried(job.to_string());
        let body = match how {
            JudgeNoteHow::Sent => encoded(&SendJudgeNote { session_id, reading: found.reading.clone(), note: found.note.clone() }),
            JudgeNoteHow::Dismissed => encoded(&DismissJudgeNote { session_id, reading: found.reading.clone(), note: found.note.clone() }),
        };
        let mark = format!("{}/{}", found.reading, found.note);
        if let Some(page) = self.pages.get_mut(&job) {
            let all = page.keys.found_all();
            let at = all.iter().position(|one| one == found);
            let next = at.and_then(|at| all.get(at + 1).or_else(|| at.checked_sub(1).and_then(|before| all.get(before)))).and_then(|one| page.keys.spot_of(&one.key));
            page.cards.judged.insert(mark.clone(), how);
            page.cards.said.remove(plan_key);
            match next {
                Some(spot) => window.focus(&spot.focus, cx),
                None => {
                    if let Some(spot) = page.keys.spot_of(&format!("plan-card:{plan_key}")) {
                        window.focus(&spot.focus, cx);
                    }
                }
            }
        }
        self.card_changed(ix, cx);
        let (back, plan_key) = (job.clone(), plan_key.to_string());
        self.post_with(
            job,
            route,
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.cards.judged.remove(&mark);
                    page.cards.said.insert(plan_key, why);
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    /// The reading's notes are read once the Job's page is open and has them.
    pub(super) fn judge_read(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        let Some(record) = self.record(job, cx) else { return };
        let Some(plan) = record.plan.as_ref() else { return };
        let Some(reading) = plan_reading(record, plan.version) else { return };
        if reading.state != JudgeState::Done || reading.read || reading.notes.is_empty() {
            return;
        }
        let marker = format!("read:{}", reading.id);
        let body = encoded(&MarkJudgeRead { session_id: SessionId::carried(job.to_string()), reading: reading.id.clone() });
        let Some(page) = self.pages.get_mut(job) else { return };
        if !page.cards.opened.insert(marker) {
            return;
        }
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else { return };
        cx.background_executor()
            .spawn(async move {
                let body = body?;
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, "/sessions/judge/read", body)).map(|_| ())
            })
            .detach();
    }

    // ---- The notes -------------------------------------------------------------------------

    /// A list of notes under one part of the plan: each a claim with its acts, or, acted on, one muted line.
    pub(super) fn notes_list(&self, at: &Pinned, notes: Vec<&JudgeNote>, label: String, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if notes.is_empty() {
            return None;
        }
        let rows = notes.into_iter().map(|note| match how_of(page, at.reading, note) {
            None => self.finding_row(at, note, page, look, cx),
            Some(how) => self.settled_row(at, note, how, look),
        });
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("notes:{}:{}:{label}", at.key, at.reading.id)))
                    .flex()
                    .flex_col()
                    .self_stretch()
                    .min_w_0()
                    .gap(look.length(token::SPACE_2))
                    .children(rows),
                Role::List,
                label,
            )
            .into_any_element(),
        )
    }

    fn finding_row(&self, at: &Pinned, note: &JudgeNote, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let found = Found { key: format!("finding:{}:{}", at.reading.id, note.id), reading: at.reading.id.clone(), note: note.id.clone() };
        let spot = page.keys.spot(&found.key, cx);
        page.keys.found(found.clone());
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let (ix, plan_key) = (at.ix, at.key.to_string());
        let claim_row = |label: &'static str, value: AnyElement| {
            div()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_3))
                .child(div().flex_none().w(look.length(token::SPACE_12)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(label))
                .child(div().flex_1().min_w_0().child(value))
        };
        let act = |id: &'static str, said: &'static str, tone: Tone, hint: &'static str, how: JudgeNoteHow, shortcut: &'static str| {
            let (found, plan_key) = (found.clone(), plan_key.clone());
            labelled(
                button(look, SharedString::from(format!("{id}:{}", found.key)), said, tone, false, None, true)
                    .tooltip(tip(format!("{hint} ({shortcut})")))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.judge_settle(ix, &plan_key, &found, how, window, cx);
                    })),
                Role::Button,
                said,
            )
        };
        let name = format!("Finding: {}", note.claim);
        labelled(
            div()
                .id(SharedString::from(found.key.clone()))
                .track_focus(&spot.focus)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .min_w_0()
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(hue.opacity(0.45))
                .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                .focus_visible(|style| style.border_color(hue))
                .in_focus(|style| style.border_color(hue))
                .on_mouse_down(MouseButton::Left, {
                    let focus = spot.focus.clone();
                    move |_, window, cx| {
                        window.focus(&focus, cx);
                    }
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .min_w_0()
                        .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_MUTED)).child("You should know"))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_2))
                                .min_w_0()
                                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(hue).child(SharedString::from(note.claim.clone())))
                                .children(note.r#where.as_ref().map(|place| {
                                    claim_row(
                                        "Where",
                                        labelled(
                                            div()
                                                .id(SharedString::from(format!("cites:{}", found.key)))
                                                .text_size(look.length(token::TEXT_XS))
                                                .line_height(look.length(token::LEADING_SM))
                                                .font_family(mono(look))
                                                .text_color(look.color(token::FG_DEFAULT))
                                                .child(SharedString::from(place.clone())),
                                            Role::List,
                                            "Cites",
                                        )
                                        .into_any_element(),
                                    )
                                })),
                        ),
                )
                .child(
                    labelled(
                        div()
                            .id(SharedString::from(format!("finding-acts:{}", found.key)))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .pt(look.length(token::SPACE_3))
                            .border_t(look.length(token::BORDER_WIDTH))
                            .border_color(hue.opacity(0.3))
                            .child(act("finding-send", "Send to Job", Tone::Secondary, "Goes to the Job as your message", JudgeNoteHow::Sent, shortcut_of("judge_note_send")))
                            .child(act("finding-dismiss", "Dismiss", Tone::Ghost, "Leaves it unsent", JudgeNoteHow::Dismissed, shortcut_of("judge_note_dismiss"))),
                        Role::Group,
                        "Answers",
                    ),
                ),
            Role::Group,
            name,
        )
        .into_any_element()
    }

    /// A note acted on, as one muted line: a mark with its tooltip, then the claim.
    fn settled_row(&self, at: &Pinned, note: &JudgeNote, how: JudgeNoteHow, look: &Look) -> AnyElement {
        let (icon, said) = match how {
            JudgeNoteHow::Sent => ("arrow-right", "Sent"),
            JudgeNoteHow::Dismissed => ("ban", "Dismissed"),
        };
        let id = format!("settled:{}:{}", at.reading.id, note.id);
        labelled(
            div()
                .id(SharedString::from(id.clone()))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_SUBTLE))
                .child(
                    div()
                        .id(SharedString::from(format!("{id}:mark")))
                        .flex_none()
                        .w(look.length(token::SPACE_4))
                        .tooltip(tip(said))
                        .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))),
                )
                .child(div().min_w_0().truncate().child(SharedString::from(note.claim.clone()))),
            Role::Group,
            format!("Finding {}: {}", said.to_lowercase(), note.claim),
        )
        .into_any_element()
    }

    /// What a group holds: notes about the group itself, then each task a note names, in the plan's order, by its words.
    pub(super) fn group_findings(&self, at: &Pinned, group: &SessionPlanGroup, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let own = at.at(&group.id);
        let tasks: Vec<_> = group.tasks.iter().map(|task| (task, at.at(&task.id))).filter(|(_, found)| !found.is_empty()).collect();
        if own.is_empty() && tasks.is_empty() {
            return None;
        }
        let own = self.notes_list(at, own, format!("Findings on {}", group.title), page, look, cx);
        let tasks: Vec<AnyElement> = tasks
            .into_iter()
            .map(|(task, found)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .min_w_0()
                            .child(glyph(look, "circle-dashed", Size::Small, look.color(token::FG_SUBTLE)))
                            .child(div().min_w_0().child(SharedString::from(task.text.clone()))),
                    )
                    .children(self.notes_list(at, found, format!("Findings on {}", task.text), page, look, cx))
                    .into_any_element()
            })
            .collect();
        Some(
            div()
                .id(SharedString::from(format!("anchored:{}:{}", at.key, group.id)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .mt(look.length(token::SPACE_1))
                .mb(look.length(token::SPACE_2))
                .children(own)
                .children(tasks)
                .into_any_element(),
        )
    }

    /// One line of the plain view, what a group gives, and the notes it holds. A group with none is its line alone;
    /// one with some has a mark at the line's end, in a column of its own, that shuts them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn gives_row(&self, text: String, group: &SessionPlanGroup, at: Option<&Pinned>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(at) = at else { return div().child(SharedString::from(text)).into_any_element() };
        let held: Vec<&JudgeNote> = std::iter::once(at.at(&group.id)).chain(group.tasks.iter().map(|task| at.at(&task.id))).flatten().collect();
        if held.is_empty() {
            return div().child(SharedString::from(text)).into_any_element();
        }
        let live = held.iter().any(|note| how_of(page, at.reading, note).is_none());
        let shut_key = format!("{}:gives-shut:{}", at.key, group.id);
        let shut = page.cards.opened.contains(&shut_key);
        let ix = at.ix;
        let mark = live.then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("gives-notes:{}", group.id)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip("Notes from the judge"))
                    .child(glyph(look, "scan-search", Size::Mark, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &shut_key, cx))),
                Role::Button,
                format!("Findings on {text}"),
            )
        });
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_1().min_w_0().child(SharedString::from(text.clone())))
                    .child(div().flex_none().w(look.length(token::H_CONTROL_SM)).children(mark)),
            )
            .children((!shut || !live).then(|| self.group_findings(at, group, page, look, cx)).flatten())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(wire: &str) -> JudgeReading {
        ipc::decode::<JudgeReading>("a reading", wire.as_bytes()).expect("a reading")
    }

    const DONE: &str = r#"{"id":"r1","artifact":{"kind":"plan","id":"2","title":"Plan v2"},"prompt":"","state":"done","asked_at":"2026-11-09T10:00:00Z","read":false,
        "notes":[{"id":"n1","claim":"No rollback"},{"id":"n2","about":"g1","claim":"Slow"},{"id":"n3","about":"gone","claim":"Stale"}]}"#;

    #[test]
    fn an_unread_reading_with_notes_waiting_shows_the_dot() {
        let one = reading(DONE);
        assert_eq!(press_of(Some(&one), false, |_| true), Press::Unread);
        assert_eq!(press_of(Some(&one), false, |_| false), Press::Idle);
    }

    #[test]
    fn a_reading_with_no_notes_is_a_clean_check_and_a_press_in_flight_is_reading() {
        let clean = reading(&DONE.replace(
            r#""notes":[{"id":"n1","claim":"No rollback"},{"id":"n2","about":"g1","claim":"Slow"},{"id":"n3","about":"gone","claim":"Stale"}]"#,
            r#""notes":[]"#,
        ));
        assert_eq!(press_of(Some(&clean), false, |_| true), Press::Clean);
        assert_eq!(press_of(Some(&clean), true, |_| true), Press::Reading);
        assert_eq!(press_of(None, false, |_| true), Press::Idle);
    }

    #[test]
    fn a_failed_reading_is_never_a_result() {
        let failed = reading(&DONE.replace(r#""state":"done""#, r#""state":"failed","failed":"The call timed out""#));
        assert_eq!(press_of(Some(&failed), false, |_| true), Press::Failed);
    }

    fn plan(groups: &str) -> SessionPlan {
        let wire = format!(r#"{{"version":2,"summary":"","recorded_at":"2026-11-09T10:00:00Z","approved":false,"scope":[],"groups":[{groups}]}}"#);
        ipc::decode::<SessionPlan>("a plan", wire.as_bytes()).expect("a plan")
    }

    #[test]
    fn a_note_about_a_part_the_plan_no_longer_holds_is_about_the_plan() {
        let one = reading(DONE);
        let plan = plan(r#"{"id":"g1","agent":"a","title":"G","paths":[],"tasks":[{"id":"t1","text":"T","state":"open"}]}"#);
        let pinned = Pinned::new(0, "plan:v2", Some(&one), &plan).expect("a finished reading");
        let whole: Vec<&str> = pinned.whole().iter().map(|note| note.id.as_str()).collect();
        assert_eq!(whole, ["n1", "n3"]);
        assert_eq!(pinned.at("g1").len(), 1);
        assert!(pinned.at("t1").is_empty());
    }

    #[test]
    fn a_reading_still_going_pins_nothing() {
        let going = reading(&DONE.replace(r#""state":"done""#, r#""state":"reading""#));
        assert!(Pinned::new(0, "plan:v2", Some(&going), &plan("")).is_none());
    }
}
