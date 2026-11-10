//! The plan card (`job-view.md`, B2): the plan the agent recorded, put to the person to approve, to
//! suggest on or to put away, and the record of it once approved.
//!
//! The plan's words are the record's (`SessionRecord::plan`); where it stands is the thread's
//! (`ThreadPlan`). The review sheet, with its marks editor, and the Judge's notes on a plan are other
//! screens; a suggestion made here goes as one `note` mark on the first group, as the sheet's own
//! Send back does.

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, FontWeight, KeyDownEvent, MouseButton, Role, SharedString};
use ipc::{
    ApprovePlan, DismissWaiting, PlanCardState, PlanChanges, PlanMark, PlanTaskState, SendPlanChanges, SessionId, SessionPlan, SessionPlanGroup,
    ThreadPlan,
};

use std::rc::Rc;

use super::acts::{button, refusal, Answer, Mode, Tone};
use super::dock::first_line;
use super::judge::{plan_reading, Pinned};
use super::keys::{Card, Run};
use super::rows::{eyebrow, glyph, mono, tip, Size};
use super::{JobView, Page};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};

fn encoded<T: serde::Serialize>(body: &T) -> Result<String, String> {
    ipc::encode(body).map_err(|why| format!("{why:?}"))
}

/// What a task's mark says, its glyph and its colour.
fn task_look(state: PlanTaskState) -> (&'static str, crate::look::Token, &'static str) {
    match state {
        PlanTaskState::Open => ("circle-dashed", token::FG_SUBTLE, "Open"),
        PlanTaskState::Working => ("circle-dot", token::STEP_RUNNING, "Working"),
        PlanTaskState::HandedIn => ("check", token::FG_MUTED, "Handed in"),
        PlanTaskState::Done => ("check", token::STEP_ADVANCED, "Done"),
        PlanTaskState::Failed => ("x", token::STEP_FAILED, "Failed"),
    }
}

/// One group of the plan against what the last version said (`PlanRevisionView`).
struct Revised<'a> {
    group: &'a SessionPlanGroup,
    added: Vec<&'a str>,
    renamed: Vec<(&'a str, &'a str)>,
    moved: Vec<&'a str>,
    gone: Vec<&'a str>,
}

impl Revised<'_> {
    fn changed(&self) -> bool {
        !(self.added.is_empty() && self.renamed.is_empty() && self.moved.is_empty() && self.gone.is_empty())
    }
}

fn revised<'a>(plan: &'a SessionPlan, changes: &'a PlanChanges) -> Vec<Revised<'a>> {
    plan.groups
        .iter()
        .map(|group| {
            let has = |id: &str| group.tasks.iter().any(|task| task.id == id);
            Revised {
                group,
                added: changes.added.iter().map(String::as_str).filter(|id| has(id)).collect(),
                renamed: changes.renamed.iter().filter(|one| has(&one.id)).map(|one| (one.id.as_str(), one.from.as_str())).collect(),
                moved: changes.moved.iter().map(|one| one.id.as_str()).filter(|id| has(id)).collect(),
                gone: changes.removed.iter().filter(|one| one.group == group.id).map(|one| one.text.as_str()).collect(),
            }
        })
        .collect()
}

impl JobView {
    /// The plan as a card: waiting, set aside or approved.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_card(&self, ix: usize, card: &ThreadPlan, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let _ = now;
        let key = card.id.clone();
        let job = self.shown.clone().unwrap_or_default();
        let record = self.record(&job, cx).and_then(|record| record.plan.as_ref()).filter(|plan| plan.version == card.version);
        let answer = page.cards.answers.get(&key);
        let reading = self.record(&job, cx).and_then(|record| plan_reading(record, card.version));
        let approved = card.state == PlanCardState::Approved || matches!(answer, Some(Answer::Approved | Answer::Going));
        let going = matches!(answer, Some(Answer::Going));
        let aside = !approved && (card.state == PlanCardState::SetAside || matches!(answer, Some(Answer::SetAside)));
        let folded = self.folded(&key, aside || (approved && record.is_none()));
        let target = self.is_target(&key);
        let mark_said = if going { "Sending" } else if approved { "Plan approved" } else { "Plan waiting for approval" };
        let (mark_icon, mark_ink) = if approved { ("badge-check", token::STEP_ADVANCED) } else { ("bell-ring", token::STEP_WAITING) };
        let mut mark = div().id(SharedString::from(format!("plan-mark:{key}"))).flex_none().tooltip(tip(mark_said));
        mark = if going {
            mark.child(
                glyph(look, "loader-circle", Size::Mark, look.color(token::FG_MUTED)).with_animation(
                    "plan-going",
                    Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                    |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                ),
            )
        } else {
            mark.child(glyph(look, mark_icon, Size::Mark, look.color(mark_ink)))
        };
        let band_key = key.clone();
        let word = if aside { "Plan set aside" } else { "Plan" };
        let band = labelled(
            div()
                .id(SharedString::from(format!("plan-band:{key}")))
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .cursor_pointer()
                .text_color(look.color(if aside { token::FG_SUBTLE } else { token::FG_MUTED }))
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .child(glyph(look, "list-checks", Size::Small, look.color(token::FG_MUTED)))
                .child(eyebrow(look, word))
                .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(format!("v{}", card.version)))
                .children((!aside).then_some(mark))
                .child(div().flex_1())
                .child(glyph(look, if folded { "chevron-right" } else { "chevron-down" }, Size::Small, look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.aim(&band_key);
                    this.fold_card(ix, &band_key, aside, cx)
                })),
            Role::Button,
            word,
        );
        let approve = {
            let (approve_key, version) = (key.clone(), card.version);
            move |this: &mut JobView, window: &mut gpui::Window, cx: &mut Context<JobView>| {
                let session_id = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                this.settle(ix, approve_key.clone(), Answer::Approved, "/sessions/plan/approve", encoded(&ApprovePlan { session_id, version }), window, cx);
            }
        };
        {
            let (suggest_key, aside_key) = (key.clone(), key.clone());
            let version = card.version;
            let title = record.map(|plan| first_line(&plan.summary).to_string()).filter(|line| !line.is_empty()).unwrap_or_else(|| format!("version {version}"));
            let approve_run = approve.clone();
            page.keys.set(
                &key,
                (!approved && !aside).then(|| Card {
                    at: ix,
                    kind: "Plan".into(),
                    title,
                    yes: Some(("Approve".into(), Rc::new(move |this, window, cx| approve_run(this, window, cx)))),
                    change: record.and_then(|plan| plan.groups.first()).map(|_| -> (String, Run) {
                        ("Suggest".into(), Rc::new(move |this, window, cx| this.open_box(ix, &suggest_key, Mode::Note, "", window, cx)))
                    }),
                    send_back: Some((
                        "Not yet".into(),
                        Rc::new(move |this, window, cx| {
                            let session_id = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                            let body = encoded(&DismissWaiting { session_id, item_id: format!("plan:{version}"), reason: None, misread: false });
                            this.settle(ix, aside_key.clone(), Answer::SetAside, "/sessions/waiting/dismiss", body, window, cx);
                        }),
                    )),
                    no: None,
                }),
            );
        }
        let head_approve = (folded && !approved && !aside).then(|| {
            let approve = approve.clone();
            labelled(
                button(look, SharedString::from(format!("plan-approve-head:{key}")), "Approve", Tone::Primary, false, target.then_some("\u{2325}\u{21b5}"), true)
                    .on_click(cx.listener(move |this, _, window, cx| approve(this, window, cx))),
                Role::Button,
                "Approve",
            )
        });
        let head = div().flex().items_center().justify_between().gap(look.length(token::SPACE_3)).child(band).children(head_approve);
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2)).child(head);
        // The Judge's notes are pinned to the plan while it waits and is open; its keys answer wherever focus is on the card.
        let live = !folded && !approved;
        let pinned = record.filter(|_| live).and_then(|plan| Pinned::new(ix, &key, reading, plan));
        if !folded {
            if let Some(plan) = record {
                column = column.child(self.plan_body(ix, &key, plan, approved, pinned.as_ref(), page, look, cx));
            }
            if let Some(why) = page.cards.said.get(&key) {
                column = column.child(refusal(look, &key, why.clone()));
            }
            if !approved {
                column = column.child(self.plan_answers(ix, &key, card.version, record, reading, aside, target, page, look, approve, cx));
            }
        }
        let aim = key.clone();
        let name = if approved { "Plan approved" } else if aside { "Plan set aside" } else { "Plan to approve" };
        let spot = page.keys.spot(&format!("plan-card:{key}"), cx);
        let (act_key, press_focus) = (key.clone(), spot.focus.clone());
        let version = card.version;
        // `u` opens the unchanged groups of a revision, or shuts them all.
        let unchanged: Vec<String> = match record.and_then(|plan| plan.changes.as_ref().map(|changes| (plan, changes))) {
            Some((plan, changes)) if live && plan.version >= 2 && !changes.is_empty() => {
                revised(plan, changes).into_iter().filter(|one| !one.changed()).map(|one| format!("{key}:unchanged:{}", one.group.id)).collect()
            }
            _ => Vec::new(),
        };
        labelled(
            div()
                .id(SharedString::from(format!("plan:{key}")))
                .track_focus(&spot.focus)
                .key_context("plan")
                .on_action(cx.listener(move |this, act: &Act, window, cx| this.plan_act(act, ix, &act_key, version, live, window, cx)))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let plain = !event.keystroke.modifiers.modified();
                    if event.keystroke.key != "u" || !plain || unchanged.is_empty() {
                        return;
                    }
                    let on_card = this.shown.as_ref().and_then(|job| this.pages.get(job)).is_some_and(|page| press_focus.is_focused(window) || page.keys.found_focused(window).is_some());
                    if !on_card {
                        return;
                    }
                    cx.stop_propagation();
                    this.toggle_unchanged(ix, &unchanged, cx);
                }))
                .mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1)))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if target && !approved { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(token::BG_RAISED))
                .when(aside && folded, |card| card.py(look.length(token::SPACE_1)))
                .on_mouse_down(MouseButton::Left, cx.listener({
                    let focus = spot.focus.clone();
                    move |this, _, window, cx| {
                        this.aim(&aim);
                        // A note or a box inside the card takes the press first; the card takes it when nothing in it did.
                        if !focus.contains_focused(window, cx) {
                            window.focus(&focus, cx);
                        }
                        cx.notify();
                    }
                }))
                .child(column),
            Role::Article,
            name,
        )
        .into_any_element()
    }

    /// What the plan says: what the person gets, and the groups behind it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_body(&self, ix: usize, key: &str, plan: &SessionPlan, approved: bool, pinned: Option<&Pinned>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let revising = !approved && plan.version >= 2 && plan.changes.as_ref().is_some_and(|changes| !changes.is_empty());
        let technical_key = format!("{key}:technical");
        let plain = !plan.summary.trim().is_empty();
        let technical = !plain || page.cards.opened.contains(&technical_key);
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_2));
        if let (Some(changes), false) = (&plan.changes, revising) {
            if !changes.is_empty() && approved {
                body = body.child(self.plan_changes_list(plan, changes, look));
            }
        }
        if plain {
            body = body
                .child(
                    div()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(plan.summary.clone())),
                )
                .children(pinned.and_then(|at| self.notes_list(at, at.whole(), "Findings on the plan".to_string(), page, look, cx)))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("gives:{key}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .pl(look.length(token::SPACE_4))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .children(plan.groups.iter().map(|group| self.gives_row(group.gives.clone().unwrap_or_else(|| group.title.clone()), group, pinned, page, look, cx))),
                    Role::List,
                    "What each group gives",
                ))
                .child({
                    let flip = technical_key.clone();
                    let said = if technical { "Show less" } else { "Details" };
                    div().flex().child(labelled(
                        button(look, SharedString::from(format!("details:{key}")), said, Tone::Ghost, false, None, true)
                            .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &flip, cx))),
                        Role::Button,
                        said,
                    ))
                });
        }
        if technical {
            let inner = if revising {
                self.plan_revision(ix, key, plan, plan.changes.as_ref().expect("revising holds changes"), page, look, cx)
            } else {
                self.plan_groups(plan, approved, plain, pinned, page, look, cx)
            };
            // With no plain view the notes about the plan as a whole stand above the groups.
            body = match pinned.filter(|_| !plain).and_then(|at| self.notes_list(at, at.whole(), "Findings on the plan".to_string(), page, look, cx)) {
                Some(whole) => body.child(div().flex().flex_col().gap(look.length(token::SPACE_2)).child(whole).child(inner)),
                None => body.child(inner),
            };
        }
        body.into_any_element()
    }

    fn plan_changes_list(&self, plan: &SessionPlan, changes: &PlanChanges, look: &Look) -> AnyElement {
        let text_of = |id: &str| -> String {
            plan.groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.title.clone())
                .or_else(|| plan.groups.iter().flat_map(|group| &group.tasks).find(|task| task.id == id).map(|task| task.text.clone()))
                .unwrap_or_else(|| id.to_string())
        };
        let line = |icon: &'static str, ink: crate::look::Token, said: &'static str, struck: Option<String>, text: Option<String>| {
            div()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .child(div().id(SharedString::from(format!("change:{said}:{}", text.clone().or(struck.clone()).unwrap_or_default()))).child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .text_color(look.color(if said == "Removed" { token::FG_MUTED } else { token::FG_DEFAULT }))
                        .children(struck.map(|from| div().text_color(look.color(token::FG_MUTED)).line_through().child(from)))
                        .children(text.map(|text| div().child(text))),
                )
        };
        let mut lines: Vec<gpui::Div> = Vec::new();
        for id in &changes.added {
            lines.push(line("plus", token::STEP_ADVANCED, "Added", None, Some(text_of(id))));
        }
        for one in &changes.renamed {
            lines.push(line("pencil", token::STEP_RUNNING, "Renamed", Some(one.from.clone()), Some(text_of(&one.id))));
        }
        for one in &changes.removed {
            lines.push(line("trash-2", token::STEP_FAILED, "Removed", Some(one.text.clone()), None));
        }
        labelled(
            div()
                .id("changed-since")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .children(lines),
            Role::List,
            "Changed since the last plan",
        )
        .into_any_element()
    }

    /// The groups of the plan with their tasks' marks and agents (B2.3, 3c).
    #[allow(clippy::too_many_arguments)]
    fn plan_groups(&self, plan: &SessionPlan, approved: bool, plain: bool, pinned: Option<&Pinned>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let mut runs: Vec<(String, Vec<&SessionPlanGroup>)> = Vec::new();
        for group in &plan.groups {
            let label = group.lands_in.label.clone();
            match runs.iter_mut().find(|(one, _)| *one == label) {
                Some((_, groups)) => groups.push(group),
                None => runs.push((label, vec![group])),
            }
        }
        let bare = runs.len() == 1 && runs[0].1.iter().all(|group| !group.own_job) && runs[0].1.iter().all(|group| group.lands_in.pull_request.is_none());
        let list = |groups: &Vec<&SessionPlanGroup>| {
            div().flex().flex_col().gap(look.length(token::SPACE_2)).children(groups.iter().map(|group| self.plan_group(group, approved, plain, pinned, page, look, cx)))
        };
        let inner: AnyElement = if bare {
            list(&runs[0].1).into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .children(runs.iter().map(|(label, groups)| {
                    let pr = groups.iter().find_map(|group| group.lands_in.pull_request.as_ref());
                    let said = match pr {
                        Some(pr) => format!("Lands in #{} {}", pr.number, pr.title),
                        None => format!("Lands in {label}"),
                    };
                    labelled(
                        div()
                            .id(SharedString::from(format!("landing:{label}")))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(look.length(token::SPACE_2))
                                    .text_size(look.length(token::TEXT_XS))
                                    .text_color(look.color(token::FG_MUTED))
                                    .child(glyph(look, "git-pull-request", Size::Small, look.color(token::FG_MUTED)))
                                    .child(SharedString::from(said.clone())),
                            )
                            .child(div().ml(look.length(token::SPACE_3)).pl(look.length(token::SPACE_3)).border_l(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(list(groups))),
                        Role::Region,
                        said,
                    )
                }))
                .into_any_element()
        };
        labelled(div().id("plan-groups").child(inner), Role::List, "Plan").into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn plan_group(&self, group: &SessionPlanGroup, approved: bool, plain: bool, pinned: Option<&Pinned>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let marks = group.tasks.iter().map(|task| {
            let (icon, ink, said) = task_look(task.state);
            div().id(SharedString::from(format!("task:{}", task.id))).child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said))
        });
        let row = div()
            .id(SharedString::from(format!("group:{}", group.id)))
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_SM))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(group.title.clone())),
            )
            .when(plain, |row| {
                row.child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).children(group.paths.iter().map(|path| div().child(path.clone()))))
            })
            .children((!approved).then(|| div().flex().flex_wrap().justify_end().max_w(look.length(token::SPACE_12) * 4.).gap(look.length(token::SPACE_1)).children(marks)))
            .child(
                div()
                    .id(SharedString::from(format!("agent:{}", group.id)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip("Subagent type"))
                    .child(glyph(look, "bot", Size::Small, look.color(token::FG_MUTED)))
                    .child(div().font_family(mono(look)).child(SharedString::from(group.agent.clone()))),
            );
        // The Judge's notes on the group open under its line; the plain view draws them under what the group gives.
        match pinned.filter(|_| !plain).and_then(|at| self.group_findings(at, group, page, look, cx)) {
            None => row.into_any_element(),
            Some(notes) => div().flex().flex_col().child(row).child(notes).into_any_element(),
        }
    }

    /// The plan read against the version before it (B2.5): what changed in the open, the rest folded.
    #[allow(clippy::too_many_arguments)]
    fn plan_revision(&self, ix: usize, key: &str, plan: &SessionPlan, changes: &PlanChanges, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let mark = |icon: &'static str, ink: crate::look::Token, said: &'static str| {
            div().id(SharedString::from(format!("rev:{said}:{icon}"))).flex_none().child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said))
        };
        let groups = revised(plan, changes).into_iter().map(|one| {
            let group = one.group;
            if one.changed() {
                let tasks = group.tasks.iter().map(|task| {
                    let (icon, ink, said) = task_look(task.state);
                    let added = one.added.contains(&task.id.as_str());
                    let renamed = one.renamed.iter().find(|(id, _)| *id == task.id).map(|(_, from)| *from);
                    let moved = one.moved.contains(&task.id.as_str());
                    let carries = added || renamed.is_some() || moved;
                    div()
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_2))
                        .text_color(look.color(if carries { token::FG_DEFAULT } else { token::FG_MUTED }))
                        .child(div().w(look.length(token::SPACE_3)).children(if added {
                            Some(mark("plus", token::STEP_ADVANCED, "Added"))
                        } else if renamed.is_some() {
                            Some(mark("pencil", token::STEP_RUNNING, "Renamed"))
                        } else if moved {
                            Some(mark("move-right", token::STEP_RUNNING, "Moved"))
                        } else {
                            None
                        }))
                        .child(div().id(SharedString::from(format!("task:{}", task.id))).child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .children(renamed.map(|from| div().text_color(look.color(token::FG_MUTED)).line_through().child(from.to_string())))
                                .child(SharedString::from(task.text.clone())),
                        )
                });
                let gone = one.gone.iter().map(|text| {
                    div()
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_2))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().w(look.length(token::SPACE_3)).child(mark("trash-2", token::STEP_FAILED, "Removed")))
                        .child(div().line_through().child(text.to_string()))
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(group.title.clone())))
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).pl(look.length(token::SPACE_3)).children(tasks).children(gone))
                    .into_any_element()
            } else {
                let fold_key = format!("{key}:unchanged:{}", group.id);
                let open = page.cards.opened.contains(&fold_key);
                let flip = fold_key.clone();
                let tasks = open.then(|| {
                    div().flex().flex_col().gap(look.length(token::SPACE_1)).pl(look.length(token::SPACE_4)).children(group.tasks.iter().map(|task| {
                        let (icon, ink, said) = task_look(task.state);
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_color(look.color(token::FG_MUTED))
                            .child(div().id(SharedString::from(format!("task:{}", task.id))).child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said)))
                            .child(SharedString::from(task.text.clone()))
                    }))
                });
                div()
                    .flex()
                    .flex_col()
                    .child(labelled(
                        div()
                            .id(SharedString::from(format!("unchanged:{fold_key}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_MD))
                            .cursor_pointer()
                            .text_color(look.color(token::FG_MUTED))
                            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                            .tooltip(tip(if open { "Hide tasks" } else { "Show tasks" }))
                            .child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_MUTED)))
                            .child(SharedString::from(group.title.clone()))
                            .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &flip, cx))),
                        Role::Button,
                        group.title.clone(),
                    ))
                    .children(tasks)
                    .into_any_element()
            }
        });
        labelled(
            div().id("plan-revision").flex().flex_col().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).children(groups),
            Role::List,
            "Plan",
        )
        .into_any_element()
    }

    /// Approve, Suggest and Not yet (B2.7), with the box a suggestion is typed in, and the Judge's press at the end.
    #[allow(clippy::too_many_arguments)]
    fn plan_answers(
        &self,
        ix: usize,
        key: &str,
        version: u32,
        record: Option<&SessionPlan>,
        reading: Option<&ipc::JudgeReading>,
        aside: bool,
        target: bool,
        page: &Page,
        look: &Look,
        approve: impl Fn(&mut JobView, &mut gpui::Window, &mut Context<JobView>) + Clone + 'static,
        cx: &Context<Self>,
    ) -> AnyElement {
        let noting = page.cards.modes.get(key) == Some(&Mode::Note);
        let note_blank = self.typed_in(key, Mode::Note, cx).trim().is_empty();
        let chip = |text: &'static str| target.then_some(text);
        let first_group = record.and_then(|plan| plan.groups.first()).map(|group| group.id.clone());
        let row = div().flex().items_center().gap(look.length(token::SPACE_2));
        let row = if noting {
            let (send_key, cancel_key) = (key.to_string(), key.to_string());
            row.child(labelled(
                button(look, SharedString::from(format!("plan-send:{key}")), "Send changes", Tone::Primary, false, None, !note_blank && first_group.is_some()).when(
                    !note_blank && first_group.is_some(),
                    |b| {
                        b.on_click(cx.listener(move |this, _, window, cx| {
                            let text = this.typed_in(&send_key, Mode::Note, cx).trim().to_string();
                            let Some(group) = first_group.clone() else { return };
                            let session_id = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                            let mark = PlanMark::Note { id: format!("note-new-{ix}"), group, task: None, text };
                            let body = encoded(&SendPlanChanges { session_id, version, marks: vec![mark], approve: false });
                            let job = this.shown.clone().unwrap_or_default();
                            this.close_box(ix, &send_key, cx);
                            this.post(job, ix, send_key.clone(), "/sessions/plan/changes", body, window, cx);
                        }))
                    },
                ),
                Role::Button,
                "Send changes",
            ))
            .child(labelled(
                button(look, SharedString::from(format!("plan-cancel:{key}")), "Cancel", Tone::Ghost, false, None, true)
                    .on_click(cx.listener(move |this, _, _, cx| this.close_box(ix, &cancel_key, cx))),
                Role::Button,
                "Cancel",
            ))
        } else {
            let (suggest_key, aside_key) = (key.to_string(), key.to_string());
            row.child(labelled(
                button(look, SharedString::from(format!("plan-approve:{key}")), "Approve", Tone::Primary, false, chip("\u{2325}\u{21b5}"), true)
                    .on_click(cx.listener(move |this, _, window, cx| approve(this, window, cx))),
                Role::Button,
                "Approve",
            ))
            .when(!aside, |row| {
                row.child(labelled(
                    button(look, SharedString::from(format!("plan-suggest:{key}")), "Suggest", Tone::Secondary, false, chip("\u{2325}E"), true)
                        .on_click(cx.listener(move |this, _, window, cx| this.open_box(ix, &suggest_key, Mode::Note, "", window, cx))),
                    Role::Button,
                    "Suggest",
                ))
                .child(labelled(
                    button(look, SharedString::from(format!("plan-not-yet:{key}")), "Not yet", Tone::Secondary, false, chip("\u{2325}R"), true).on_click(cx.listener(
                        move |this, _, window, cx| {
                            let session_id = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                            let body = encoded(&DismissWaiting { session_id, item_id: format!("plan:{version}"), reason: None, misread: false });
                            this.settle(ix, aside_key.clone(), Answer::SetAside, "/sessions/waiting/dismiss", body, window, cx);
                        },
                    )),
                    Role::Button,
                    "Not yet",
                ))
            })
        };
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2));
        if noting {
            column = column.children(self.box_of(key, Mode::Note, "Note", look, cx));
        }
        let row = row.child(self.judge_press(ix, key, version, reading, page, look, cx));
        column
            .child(labelled(row.id(SharedString::from(format!("answers:{key}"))), Role::Toolbar, "Answers"))
            .into_any_element()
    }

    /// `u`: open every unchanged group of the revision, or, once all are open, shut them all.
    pub(super) fn toggle_unchanged(&mut self, ix: usize, folds: &[String], cx: &mut Context<Self>) {
        let all_open = folds.iter().all(|fold| self.open_in_card(fold));
        for fold in folds {
            if self.open_in_card(fold) == all_open {
                self.flip_in_card(ix, fold, cx);
            }
        }
    }
}
