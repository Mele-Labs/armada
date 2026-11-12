//! A Job's workflow in the Spine, as Armada Classic's `StoryColumn` draws it: the head with the
//! workflow's name and its picker (`WorkflowHead`), and a zone for each step with Skip, Undo and Jump
//! here (`StepSkip`, `StepJump`). A plan whose groups end on gates is the workflow instead
//! (`stretches.ts`): a zone for each stretch whose gate closed, headed by the fact that closed it
//! (`GateClosed`), and Now as the stretches still to finish (`PlanChunks`).
//!
//! A pick, a skip and a jump show at once and stand until Fleet's record says otherwise; a refusal
//! takes them back and is said in a toast. The Open group is Fleet's `bridge.openGroup` setting, as
//! Classic keeps it, so hiding it hides it in both apps.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, MouseButton, Role, SharedString, Window};
use ipc::{GateChecks, GateClose, GateHeld, JumpSessionStep, PlanGate, SaveSettings, SessionId, SessionPlanGroup, SessionRecord, SetSessionWorkflow, SettingValue, SkipSessionStep};

use super::done::{group_time, Finished};
use super::head::{below_left, drawn, tip};
use super::now::lamp_mark;
use super::rows::{glyph, Size};
use super::side::Stands;
use super::spine::{Area, Mark, Offer, Spec};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The workflow a Job follows and where it is, as Classic's `workflowOf` and `WorkflowHead` read it.
pub(super) struct Work {
    /// The workflow's name where the Job follows one: the one Fleet holds, else its id.
    pub(super) name: Option<String>,
    /// Its steps' words, in order. Empty where Fleet does not hold the workflow.
    pub(super) steps: Vec<String>,
    /// The step the Job is at. Fleet leaves it off after the last step's pull request merged.
    pub(super) at: Option<usize>,
    /// When the Job came to each step it visited.
    pub(super) visits: Vec<(usize, String)>,
    pub(super) skipped: HashSet<usize>,
    /// Workflows are off on this machine (`features.workflows`).
    pub(super) off: bool,
    /// The picker is offered: a hosted Job, with Fleet's workflows read.
    pub(super) picking: bool,
    /// The workflow the picker has ticked, by id.
    pub(super) chosen: Option<String>,
    /// The proposer picked it and nobody has changed it since.
    pub(super) proposed: bool,
    /// The Job carries a proposal.
    pub(super) proposal: bool,
    /// What the picker offers: the workflows of the Job's repository, and those of none.
    pub(super) choices: Vec<(String, String)>,
    /// What the held workflow ends in, as Fleet resolves it (`landsOf`).
    pub(super) lands: Option<ipc::Lands>,
}

impl Work {
    /// The zones are the workflow's steps (`stepped`).
    pub(super) fn stepped(&self) -> bool {
        !self.off && !self.steps.is_empty() && self.at.is_some()
    }

    /// The workflow names a step so: a step of the route by that name is that step, not a second row.
    pub(super) fn names(&self, label: &str) -> bool {
        self.steps.iter().any(|step| step.eq_ignore_ascii_case(label))
    }

    /// The head stands at the top of the Spine: on a workflow, or with a proposal and the picker
    /// offered (`StoryColumn` 538-543). A Job on none and with no proposal has no head.
    pub(super) fn head(&self) -> bool {
        !self.off && (self.name.is_some() || (self.proposal && self.picking))
    }

    pub(super) fn state(&self, step: usize) -> StretchState {
        match self.at {
            Some(at) if step < at => StretchState::Done,
            Some(at) if step == at => StretchState::Now,
            _ => StretchState::Next,
        }
    }
}

/// Where a step or a stretch stands (`stateOf`, `StretchState`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StretchState {
    Done,
    Now,
    Next,
}

/// A stretch's gate, as its last group carries it.
#[derive(Clone)]
pub(super) struct Gate {
    pub(super) kind: PlanGate,
    pub(super) closed: Option<GateClose>,
    pub(super) checks: Option<GateChecks>,
    /// The Job is held on this gate, waiting on him: since when.
    pub(super) held: Option<String>,
}

/// One run of groups up to a gate (`Stretch`).
pub(super) struct Stretch<'a> {
    pub(super) key: String,
    pub(super) name: String,
    pub(super) groups: Vec<&'a SessionPlanGroup>,
    pub(super) gate: Option<Gate>,
    pub(super) state: StretchState,
}

/// Whether the plan carries gates at all (`gated`).
pub(super) fn gated(groups: &[SessionPlanGroup]) -> bool {
    groups.iter().any(|group| group.gate != PlanGate::None)
}

/// The plan read as gates: each run of groups up to the next gate is one stretch, and the Job is in
/// the first stretch whose gate is open (`stretchesOf`).
pub(super) fn stretches_of<'a>(groups: &'a [SessionPlanGroup], held: Option<&GateHeld>) -> Vec<Stretch<'a>> {
    let mut runs: Vec<Vec<&SessionPlanGroup>> = Vec::new();
    let mut run = Vec::new();
    for group in groups {
        run.push(group);
        if group.gate != PlanGate::None {
            runs.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    let mut live = false;
    runs.into_iter()
        .map(|one| {
            let end = one[one.len() - 1];
            let gate = (end.gate != PlanGate::None).then(|| Gate {
                kind: end.gate,
                closed: end.gate_closed.clone(),
                checks: end.gate_checks.clone(),
                held: held.filter(|held| held.group == end.id && end.gate_closed.is_none()).map(|held| held.since.as_str().to_string()),
            });
            let closed = gate.as_ref().is_some_and(|gate| gate.closed.is_some());
            let state = if closed {
                StretchState::Done
            } else if live {
                StretchState::Next
            } else {
                StretchState::Now
            };
            if !closed {
                live = true;
            }
            Stretch {
                key: one.iter().map(|group| group.id.as_str()).collect::<Vec<_>>().join("+"),
                name: one.iter().map(|group| group.title.as_str()).collect::<Vec<_>>().join(" · "),
                groups: one,
                gate,
                state,
            }
        })
        .collect()
}

/// Where each group of a gated plan stands: where its stretch does, whatever its tasks say.
pub(super) fn gate_places(groups: &[SessionPlanGroup], held: Option<&GateHeld>) -> HashMap<String, Stands> {
    stretches_of(groups, held)
        .into_iter()
        .flat_map(|stretch| {
            let stands = match stretch.state {
                StretchState::Done => Stands::Done,
                StretchState::Now => Stands::Working,
                StretchState::Next => Stands::Coming,
            };
            stretch.groups.into_iter().map(move |group| (group.id.clone(), stands))
        })
        .collect()
}

/// What an open gate ends its stretch on, after "Ends when" (`GATE_ENDS`).
fn gate_ends(kind: PlanGate) -> &'static str {
    match kind {
        PlanGate::PlanApproved => "you approve the plan",
        PlanGate::WalkApproved => "you approve the walk",
        PlanGate::SketchApproved => "you approve the sketch",
        PlanGate::DocumentApproved => "you approve the document",
        PlanGate::YouApproved => "you approve",
        PlanGate::ChecksGreen => "Checks pass at the pushed commit",
        PlanGate::Merged => "it merges into main",
        PlanGate::JobsEnded => "every Job it started has merged or ended",
        PlanGate::None => "",
    }
}

/// A gate's glyph (`GATE_GLYPH`), from the one gate table; no gate takes the Jobs tab's link.
fn gate_glyph(kind: PlanGate) -> &'static str {
    super::workflow_card::gate_mark(kind).map_or("link", |mark| mark.glyph)
}

/// How a gate reads (`readGate`).
struct GateRead {
    said: String,
    failed_mark: bool,
    glyph: String,
    failed: Vec<String>,
    red: Vec<String>,
}

fn read_gate(gate: &Gate) -> GateRead {
    let checks = gate.closed.is_none().then_some(gate.checks.as_ref()).flatten();
    let said = match (&gate.closed, checks) {
        (Some(closed), _) => closed.fact.clone(),
        (None, None) => format!("Ends when {}", gate_ends(gate.kind)),
        (None, Some(checks)) if checks.state == "running" => format!("Checks running at {}", checks.commit),
        (None, Some(checks)) => format!("Checks failed at {}", checks.commit),
    };
    let glyph = if gate.closed.is_some() {
        "Gate, closed by Fleet".to_string()
    } else if gate.held.is_some() {
        "Gate, the Job is held on you".to_string()
    } else {
        format!("Ends when {}", gate_ends(gate.kind))
    };
    let red_checks = checks.filter(|checks| checks.state == "red");
    GateRead {
        said,
        failed_mark: red_checks.is_some(),
        glyph,
        failed: red_checks.map(|checks| checks.failed.clone()).unwrap_or_default(),
        red: gate.closed.as_ref().map(|closed| closed.already_red.clone()).or_else(|| checks.map(|checks| checks.already_red.clone())).unwrap_or_default(),
    }
}

/// A step whose name asks for a walk (`WALK_STEP`).
pub(super) fn walk_step(name: &str) -> bool {
    let name = name.to_lowercase();
    ["present", "walk", "show"].iter().any(|word| name.contains(word))
}

pub(super) fn parse(at: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(at).ok()
}

/// The words a stamp says for a time: the date and the minute, local.
fn stamp(at: &str) -> Option<String> {
    parse(at).map(|at| at.with_timezone(&chrono::Local).format("%-d %b %H:%M").to_string())
}

/// Index ids for a menu's dynamic rows: an entry id is `&'static str`.
const PICKS: [&str; 24] = ["w0", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "w8", "w9", "w10", "w11", "w12", "w13", "w14", "w15", "w16", "w17", "w18", "w19", "w20", "w21", "w22", "w23"];

impl JobView {
    /// The workflow the Job follows and where it is, with what he pressed that Fleet has not answered yet.
    pub(super) fn work(&self, job: &SessionRecord, cx: &gpui::App) -> Work {
        let (off, workflows) = match self.fleet.as_ref() {
            Some(fleet) => {
                let held = fleet.read(cx).held();
                (held.workflows_off, held.workflows.clone())
            }
            None => (false, Some(crate::mock::workflows())),
        };
        let side = self.pages.get(job.id.as_str()).map(|page| &page.side);
        // The workflow he picked stands until the record names another.
        let workflow = match side.and_then(|side| side.flow_pick.as_ref()) {
            Some((was, picked)) if *was == job.workflow => picked.clone(),
            _ => job.workflow.clone(),
        };
        let all = workflows.clone().unwrap_or_default();
        let held = workflow.as_ref().and_then(|id| {
            job.workflow_copy.clone().filter(|copy| copy.id.as_str() == id).or_else(|| {
                all.iter().find(|one| one.id.as_str() == id && job.manifest_id.as_ref().is_none_or(|manifest| *manifest == one.manifest_id)).or_else(|| all.iter().find(|one| one.id.as_str() == id)).cloned()
            })
        });
        let steps: Vec<String> = held.as_ref().map(|held| held.steps.iter().map(|step| step.label.clone()).collect()).unwrap_or_default();
        let at = match side.and_then(|side| side.jumped) {
            Some((was, to)) if was == job.workflow_step => Some(to),
            _ => job.workflow_step,
        }
        .map(|at| at as usize)
        .filter(|at| *at < steps.len());
        let mut skipped: HashSet<usize> = job.skipped_steps.iter().map(|step| *step as usize).collect();
        if let Some(side) = side {
            for (step, (was, now)) in &side.skips {
                let step = *step as usize;
                if skipped.contains(&step) == *was {
                    if *now {
                        skipped.insert(step);
                    } else {
                        skipped.remove(&step);
                    }
                }
            }
        }
        let choices: Vec<(String, String)> = all.iter().filter(|one| job.manifest_id.as_ref().is_none_or(|manifest| *manifest == one.manifest_id)).map(|one| (one.id.as_str().to_string(), one.name.clone())).collect();
        let unpicked = side.is_some_and(|side| side.unpicked);
        let name = held.as_ref().map(|held| held.name.clone()).or_else(|| workflow.clone());
        Work {
            name: if off { None } else { name },
            steps: if off { Vec::new() } else { steps },
            at: if off { None } else { at },
            visits: job.workflow_visits.iter().map(|visit| (visit.step as usize, visit.since.as_str().to_string())).collect(),
            skipped,
            off,
            picking: job.hosted.is_some() && workflows.is_some(),
            chosen: workflow,
            proposed: !off && !unpicked && job.proposal.as_ref().is_some_and(|proposal| proposal.picked.iter().any(|field| field.as_wire() == "workflow")),
            proposal: job.proposal.is_some(),
            choices,
            lands: if off { None } else { held.as_ref().map(|held| held.lands) },
        }
    }

    /// A plan whose groups end on gates is the workflow: approved, not being edited, with a gate.
    pub(super) fn by_gates(&self, id: &str, job: &SessionRecord) -> bool {
        job.plan.as_ref().is_some_and(|plan| plan.approved && self.plan_marks(id).is_empty() && gated(&plan.groups))
    }

    /// The head at the top of the Spine: the workflow's name with its picker where it can be changed,
    /// and the proposer's mark where he has not changed its pick (`WorkflowHead`).
    pub(super) fn workflow_head(&self, work: &Work, alone: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let name = work.chosen.as_ref().and_then(|chosen| work.choices.iter().find(|(id, _)| id == chosen)).map(|(_, name)| name.clone()).or_else(|| work.name.clone()).unwrap_or_else(|| "None".to_string());
        let title = div()
            .min_w_0()
            .truncate()
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(SharedString::from(name.clone()));
        let key = gpui::Keystroke::parse("alt-w").ok();
        let named: AnyElement = if work.picking {
            let open = self.pop_named("Workflow", cx);
            labelled(
                div()
                    .id(if alone { "workflow-pick-alone" } else { "workflow-pick" })
                    .track_focus(if alone { &self.spine.head_alone } else { &self.spine.head })
                    .flex()
                    .flex_shrink(1.)
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .min_w_0()
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .ml(-look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_color(look.color(token::FG_SUBTLE))
                    .cursor_pointer()
                    .when(open, |pick| pick.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .aria_expanded(open)
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new("Workflow").key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                    .child(title)
                    .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_SUBTLE)))
                    .on_click(cx.listener(move |this, _, window, cx| this.workflow_menu(alone, window, cx)))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                        if event.keystroke.key == "down" && !event.keystroke.modifiers.modified() {
                            this.workflow_menu(alone, window, cx);
                            cx.stop_propagation();
                        }
                    })),
                Role::Button,
                format!("Workflow {name}"),
            )
            .into_any_element()
        } else {
            div().id(if alone { "workflow-name-alone" } else { "workflow-name" }).min_w_0().flex().tooltip(tip("Workflow".into())).child(title).into_any_element()
        };
        let proposed = work.proposed.then(|| labelled(div().id(if alone { "workflow-proposed-alone" } else { "workflow-proposed" }).flex_none().tooltip(tip("Picked by the proposer".into())).child(glyph(look, "lightbulb", Size::Small, look.color(token::FG_SUBTLE))), Role::Image, "Picked by the proposer"));
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .min_h(look.length(token::H_CONTROL))
            .py(look.length(token::SPACE_3))
            .pl(look.length(token::SPACE_4) * 2.)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(named)
            .children(proposed)
            .into_any_element()
    }

    /// Whether the named menu is up.
    fn pop_named(&self, name: &str, cx: &gpui::App) -> bool {
        self.pop.as_ref().is_some_and(|pop| pop.menu.read(cx).name().as_ref() == name)
    }

    /// The picker under the head's name: None, then each workflow, the Job's ticked.
    pub(super) fn workflow_menu(&mut self, alone: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(job) = self.record(&id, cx).cloned() else { return };
        let work = self.work(&job, cx);
        if !work.picking {
            return;
        }
        let mut entries = vec![Entry::Choice { id: "none", label: "None".into(), on: work.chosen.is_none() }];
        let ids: Vec<String> = work.choices.iter().take(PICKS.len()).map(|(id, _)| id.clone()).collect();
        entries.extend(work.choices.iter().take(PICKS.len()).enumerate().map(|(at, (one, name))| Entry::Choice { id: PICKS[at], label: name.clone().into(), on: work.chosen.as_deref() == Some(one.as_str()) }));
        let at = drawn(window, cx, Role::Button, &format!("Workflow {}", work.chosen.as_ref().and_then(|chosen| work.choices.iter().find(|(id, _)| id == chosen)).map(|(_, name)| name.clone()).or(work.name.clone()).unwrap_or_else(|| "None".to_string())))
            .map_or_else(|| window.mouse_position(), below_left);
        let back = Some(if alone { self.spine.head_alone.clone() } else { self.spine.head.clone() });
        self.open_button_menu(
            "Workflow",
            entries,
            at,
            back,
            move |this, picked, window, cx| {
                let pick = PICKS.iter().position(|one| *one == picked).and_then(|at| ids.get(at).cloned());
                this.choose_workflow(pick, window, cx);
            },
            window,
            cx,
        );
    }

    /// A workflow picked from the head: from the next turn the Job follows it.
    fn choose_workflow(&mut self, pick: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(job) = self.record(&id, cx).cloned() else { return };
        let was = job.workflow.clone();
        if let Some(page) = self.pages.get_mut(&id) {
            page.side.flow_pick = Some((was, pick.clone()));
            page.side.unpicked = true;
        }
        cx.notify();
        let owner = id.clone();
        let body = ipc::encode(&SetSessionWorkflow { session_id: SessionId::carried(id.to_string()), workflow: pick }).map_err(|why| format!("{why:?}"));
        self.post_with(
            id,
            "/sessions/workflow",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&owner) {
                    page.side.flow_pick = None;
                }
                cx.notify();
                cx.defer(move |cx| crate::toasts::tell(why, cx));
            },
            window,
            cx,
        );
    }

    /// Skip a step, or undo its skip. Fleet answers the record, which moves the Job past a skipped step.
    pub(super) fn skip_step(&mut self, step: usize, skip: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(job) = self.record(&id, cx).cloned() else { return };
        let step = step as u32;
        let was = job.skipped_steps.contains(&step);
        if let Some(page) = self.pages.get_mut(&id) {
            page.side.skips.insert(step, (was, skip));
        }
        cx.notify();
        let owner = id.clone();
        let body = ipc::encode(&SkipSessionStep { session_id: SessionId::carried(id.to_string()), step, skipped: skip }).map_err(|why| format!("{why:?}"));
        self.post_with(
            id,
            "/sessions/step-skip",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&owner) {
                    page.side.skips.remove(&step);
                }
                cx.notify();
                cx.defer(move |cx| crate::toasts::tell(why, cx));
            },
            window,
            cx,
        );
    }

    /// Move the Job straight to a step to come; Fleet skips the steps between.
    pub(super) fn jump_step(&mut self, step: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(job) = self.record(&id, cx).cloned() else { return };
        let step = step as u32;
        if let Some(page) = self.pages.get_mut(&id) {
            page.side.jumped = Some((job.workflow_step, step));
        }
        cx.notify();
        let owner = id.clone();
        let body = ipc::encode(&JumpSessionStep { session_id: SessionId::carried(id.to_string()), step }).map_err(|why| format!("{why:?}"));
        self.post_with(
            id,
            "/sessions/step-jump",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&owner) {
                    page.side.jumped = None;
                }
                cx.notify();
                cx.defer(move |cx| crate::toasts::tell(why, cx));
            },
            window,
            cx,
        );
    }

    /// Whether the Open group is drawn: Fleet's `bridge.openGroup`, with a hide he pressed that Fleet
    /// has not answered yet.
    pub(super) fn open_group_shown(&self, cx: &gpui::App) -> bool {
        let held = self.fleet.as_ref().map_or(true, |fleet| fleet.read(cx).held().open_group);
        match self.open_group_set {
            Some((was, now)) if was == held => now,
            _ => held,
        }
    }

    /// Hide Open group: for every Job, until Settings, Bridge has it back.
    pub(super) fn hide_open_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let held = self.fleet.as_ref().map_or(true, |fleet| fleet.read(cx).held().open_group);
        self.open_group_set = Some((held, false));
        cx.notify();
        let mut changes = std::collections::BTreeMap::new();
        changes.insert("bridge.openGroup".to_string(), Some(SettingValue::Bool(false)));
        let body = ipc::encode(&SaveSettings { changes }).map_err(|why| format!("{why:?}"));
        self.post_with(
            id,
            "/settings/save",
            body,
            |this, why, cx| {
                this.open_group_set = None;
                cx.notify();
                cx.defer(move |cx| crate::toasts::tell(why, cx));
            },
            window,
            cx,
        );
        self.keys_to_box(window, cx);
    }

    /// The acts a step's band carries for skipping it: a bare Skip glyph, or on a skipped step a
    /// worded Undo, so the way back reads without a hover (`SkipStep`).
    pub(super) fn skip_act(&self, step: usize, skipped: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = if skipped { "Undo skip" } else { "Skip step" };
        let button = div()
            .id(SharedString::from(format!("step-skip:{step}")))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_CONTROL_SM))
            .min_w(look.length(token::H_CONTROL_SM))
            .rounded(look.length(token::RADIUS_MD))
            .text_color(look.color(token::FG_SUBTLE))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, window, cx| {
                this.skip_step(step, !skipped, window, cx);
                cx.stop_propagation();
            }));
        let button = if skipped {
            button.px(look.length(token::SPACE_2)).font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default()).child(glyph(look, "undo-2", Size::Small, look.color(token::FG_SUBTLE))).child("Undo")
        } else {
            button.tooltip(tip(said.into())).child(glyph(look, "skip-forward", Size::Small, look.color(token::FG_SUBTLE)))
        };
        labelled(button, Role::Button, said).into_any_element()
    }

    /// Jump here, on a step to come (`JumpHere`).
    pub(super) fn jump_act(&self, step: usize, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(format!("step-jump:{step}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Jump here".into()))
                .child(glyph(look, "corner-down-right", Size::Small, look.color(token::FG_SUBTLE)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.jump_step(step, window, cx);
                    cx.stop_propagation();
                })),
            Role::Button,
            "Jump here",
        )
        .into_any_element()
    }

    /// Skip's place, kept where it is not offered, so the icons line up down the Spine (`sv-panel__slot`).
    pub(super) fn act_slot(look: &Look) -> AnyElement {
        div().flex_none().w(look.length(token::H_CONTROL_SM)).into_any_element()
    }

    /// A walk a step asks for, not shown yet (`WalkToCome`).
    pub(super) fn walk_to_come(look: &Look) -> AnyElement {
        labelled(
            div()
                .id("walk-to-come")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_ROW))
                .px(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(div().id("walk-to-come-mark").flex_none().tooltip(tip("Walk, not shown yet".into())).child(glyph(look, "footprints", Size::Small, look.color(token::FG_SUBTLE))))
                .child("Walk"),
            Role::Group,
            "Walk, not shown yet",
        )
        .into_any_element()
    }

    /// The tests a Checks gate names: those that hold it, then those red on main too (`GateTests`).
    fn gate_tests(read: &GateRead, key: &str, look: &Look) -> Option<AnyElement> {
        if read.failed.is_empty() && read.red.is_empty() {
            return None;
        }
        let chip = |test: &String, failed: bool, at: usize| {
            let said: SharedString = if failed { "Failed here, and passes on main: holds the gate".into() } else { "Red on main too, so it does not hold the gate".into() };
            let ink = if failed { token::STEP_FAILED } else { token::FG_MUTED };
            div()
                .id(SharedString::from(format!("{key}-test:{}:{at}", if failed { "failed" } else { "red" })))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(ink))
                .font_family(super::rows::mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .max_w_full()
                .tooltip(tip(said))
                .child(glyph(look, if failed { "shield-x" } else { "shield-minus" }, Size::Small, look.color(ink)))
                .child(div().min_w_0().truncate().child(SharedString::from(test.clone())))
        };
        let chips = read.failed.iter().enumerate().map(|(at, test)| chip(test, true, at)).chain(read.red.iter().enumerate().map(|(at, test)| chip(test, false, at)));
        Some(labelled(div().id(SharedString::from(format!("{key}-tests"))).flex().flex_wrap().gap(look.length(token::SPACE_1)).pb(look.length(token::SPACE_1)).children(chips), Role::List, "Tests").into_any_element())
    }

    /// At the head of a finished stretch in Done: the gate that ended it, the fact Fleet saw close it,
    /// and when (`GateClosed`).
    pub(super) fn gate_closed(gate: Option<&Gate>, key: &str, look: &Look) -> Option<AnyElement> {
        let gate = gate.filter(|gate| gate.closed.is_some())?;
        let read = read_gate(gate);
        let at = gate.closed.as_ref().and_then(|closed| stamp(closed.at.as_str()));
        Some(
            div()
                .flex()
                .flex_col()
                .pb(look.length(token::SPACE_1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(labelled(div().id(SharedString::from(format!("{key}-gate"))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip(read.glyph.clone().into())).child(glyph(look, gate_glyph(gate.kind), Size::Small, look.color(token::STEP_ADVANCED))), Role::Image, read.glyph.clone()))
                        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(read.said.clone())))
                        .children(at.map(|at| div().flex_none().child(SharedString::from(at)))),
                )
                .children(JobView::gate_tests(&read, key, look).map(|tests| div().pl(look.length(token::SPACE_4) + look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).child(tests)))
                .into_any_element(),
        )
    }

    /// Now for a plan that ends on gates: one row a stretch still to finish, a line a group and the
    /// gate's glyph on the last; under the live one what is being done now and a Tasks press. Any
    /// other opens its tasks on a press (`PlanChunks`).
    pub(super) fn plan_chunks(&self, id: &SharedString, stretches: &[Stretch], doing: Option<AnyElement>, mut tasks_of: impl FnMut(&Stretch) -> Option<AnyElement>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let opened = self.pages.get(id).map(|page| page.side.chunks.clone()).unwrap_or_default();
        let mut doing = doing;
        let rows: Vec<AnyElement> = stretches
            .iter()
            .map(|stretch| {
                let live = stretch.state == StretchState::Now;
                let at = format!("{}:{:?}", stretch.key, stretch.state);
                let open = opened.contains(&at);
                let read = stretch.gate.as_ref().map(read_gate);
                let (state_icon, state_said, state_ink) = match stretch.state {
                    StretchState::Done => ("check", "Done", token::STEP_ADVANCED),
                    StretchState::Now => ("circle-dot", "Now", token::STEP_RUNNING),
                    StretchState::Next => ("circle-dashed", "Next", token::FG_SUBTLE),
                };
                let ink = match stretch.state {
                    StretchState::Now => token::FG_DEFAULT,
                    StretchState::Done => token::FG_MUTED,
                    StretchState::Next => token::FG_SUBTLE,
                };
                let held = stretch.gate.as_ref().is_some_and(|gate| gate.held.is_some());
                let failed = read.as_ref().is_some_and(|read| read.failed_mark);
                let gate_ink = if held {
                    token::STATUS_AWAITING_REVIEW
                } else if failed {
                    token::STEP_FAILED
                } else {
                    token::FG_SUBTLE
                };
                let lines = stretch.groups.len();
                let names = stretch.groups.iter().enumerate().map(|(at, group)| {
                    let gate = (at + 1 == lines).then(|| stretch.gate.as_ref()).flatten().map(|gate| {
                        let said = read.as_ref().map(|read| read.glyph.clone()).unwrap_or_default();
                        labelled(div().id(SharedString::from(format!("chunk-gate:{}", stretch.key))).flex_none().w(look.length(token::SPACE_6)).flex().justify_center().tooltip(tip(said.clone().into())).child(glyph(look, gate_glyph(gate.kind), Size::Small, look.color(gate_ink))), Role::Image, said)
                    });
                    div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().line_clamp(2).child(SharedString::from(group.title.clone()))).children(gate)
                });
                let said = read.as_ref().filter(|_| live || open).map(|read| {
                    let since = stretch.gate.as_ref().and_then(|gate| match (&gate.closed, &gate.held) {
                        (Some(closed), _) => stamp(closed.at.as_str()),
                        (None, Some(held)) => super::now::span_since(held, chrono::Local::now()).map(|span| format!("Held on you for {span}")),
                        _ => None,
                    });
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(if failed { token::STEP_FAILED } else if live { token::FG_MUTED } else { token::FG_SUBTLE }))
                        .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                        .child(div().flex_auto().min_w_0().truncate().child(SharedString::from(read.said.clone())))
                        .children(since.map(|since| div().flex_none().child(SharedString::from(since))))
                });
                let head = div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(ink))
                    .when(live, |row| row.font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .child(labelled(div().id(SharedString::from(format!("chunk-state:{}", stretch.key))).flex_none().w(look.length(token::SPACE_4)).h(look.length(token::LEADING_SM)).flex().items_center().justify_center().tooltip(tip(state_said.into())).child(glyph(look, state_icon, Size::Small, look.color(state_ink))), Role::Image, state_said))
                    .child(div().flex_1().min_w_0().flex().flex_col().children(names).children(said));
                let head = if live {
                    head.into_any_element()
                } else {
                    let key = at.clone();
                    labelled(
                        head.id(SharedString::from(format!("chunk-row:{}", stretch.key)))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .tab_index(0)
                            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                            .aria_expanded(open)
                            .on_click(cx.listener(move |this, _, _, cx| this.chunk_turn(&key, cx))),
                        Role::Button,
                        stretch.name.clone(),
                    )
                    .into_any_element()
                };
                let tests = read.as_ref().and_then(|read| JobView::gate_tests(read, &format!("chunk:{}", stretch.key), look)).map(|tests| div().pl(look.length(token::SPACE_1) + look.length(token::SPACE_4) + look.length(token::SPACE_2)).child(tests));
                let under = if live {
                    let key = at.clone();
                    let tasks = labelled(
                        div()
                            .id(SharedString::from(format!("chunk-tasks:{}", stretch.key)))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .pr(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_SM))
                            .text_size(look.length(token::TEXT_XS))
                            .text_color(look.color(token::FG_SUBTLE))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                            .tab_index(0)
                            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                            .aria_expanded(open)
                            .child(div().w(look.length(token::SPACE_4)).flex().justify_center().child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_SUBTLE))))
                            .child("Tasks")
                            .on_click(cx.listener(move |this, _, _, cx| this.chunk_turn(&key, cx))),
                        Role::Button,
                        "Tasks",
                    );
                    Some(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .pl(look.length(token::SPACE_1) + look.length(token::SPACE_4) + look.length(token::SPACE_2))
                            .pt(look.length(token::SPACE_1))
                            .pb(look.length(token::SPACE_3))
                            .children(doing.take().map(|doing| labelled(div().id("chunk-doing").flex().flex_col().gap(look.length(token::SPACE_1)).child(doing), Role::Region, "Doing now")))
                            .child(div().flex().child(tasks))
                            .children(open.then(|| tasks_of(stretch)).flatten()),
                    )
                } else {
                    open.then(|| tasks_of(stretch)).flatten().map(|tasks| div().flex().flex_col().pl(look.length(token::SPACE_1) + look.length(token::SPACE_4) + look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).pb(look.length(token::SPACE_3)).child(tasks))
                };
                div().flex().flex_col().child(head).children(tests).children(under).into_any_element()
            })
            .collect();
        labelled(div().id("plan-chunks").flex().flex_col().py(look.length(token::SPACE_2)).children(rows), Role::List, "Plan").into_any_element()
    }

    /// A stretch's tasks opened or folded. Kept by stretch and state, so a stretch that moves on folds again.
    fn chunk_turn(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if let Some(page) = self.pages.get_mut(&id) {
            if !page.side.chunks.remove(key) {
                page.side.chunks.insert(key.to_string());
            }
        }
        cx.notify();
    }

    /// The subagents at work in the live stretch: each one's task, and the group it works on. A press
    /// opens its thread (`Helpers`).
    pub(super) fn helpers(&self, job: &SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let groups: &[SessionPlanGroup] = job.plan.as_ref().map_or(&[], |plan| plan.groups.as_slice());
        let rows: Vec<AnyElement> = job
            .attachments
            .iter()
            .filter(|one| one.kind == "subagent" && one.state == ipc::AttachmentState::Standing)
            .map(|one| {
                let id = one.target.clone();
                let task = super::ledger::subagent_of(one).task;
                let on = one.detail.get("group").and_then(|group| groups.iter().find(|each| each.id == *group)).map(|group| group.title.clone());
                let open = id.clone();
                labelled(
                    div()
                        .id(SharedString::from(format!("helper:{id}")))
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .tab_index(0)
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .child(div().id(SharedString::from(format!("helper-mark:{id}"))).flex_none().w(look.length(token::SPACE_4)).flex().justify_center().tooltip(tip("Subagent".into())).child(glyph(look, "drone", Size::Small, look.color(token::STEP_RUNNING))))
                        .child(div().flex_1().min_w_0().flex().flex_col().child(SharedString::from(task.clone())).children(on.map(|on| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(on)))))
                        .on_click(cx.listener(move |this, _, window, cx| this.open_subagent_tab(&open, window, cx))),
                    Role::Button,
                    task,
                )
                .into_any_element()
            })
            .collect();
        (!rows.is_empty()).then(|| labelled(div().id("helpers").flex().flex_col().gap(look.length(token::SPACE_1)).children(rows), Role::List, "Subagents at work").into_any_element())
    }
}

impl JobView {
    /// The zones a workflow's steps or a gated plan's stretches are, with the Open group after them;
    /// none where the Job has neither. `alone` draws them for a tab of their own, where the Open group
    /// always stands and the current step never holds the open work.
    pub(super) fn flow_specs(&self, id: &SharedString, job: &SessionRecord, alone: bool, look: &Look, cx: &Context<Self>) -> Option<Vec<Spec>> {
        let work = self.work(job, cx);
        let gates = self.by_gates(id, job);
        if !gates && !work.stepped() {
            return None;
        }
        let shown = self.open_group_shown(cx);
        let can = job.hosted.is_some();
        let (lamp, now_body) = self.now_parts(id, true, look, cx).unwrap_or((None, None));
        let now_lamp = || {
            lamp.map(|lamp| {
                let (hue, said, pulse) = lamp.look();
                lamp_mark(look, "spine-now", hue, said, pulse)
            })
        };
        let open_work = || self.open_work(id, job, look, cx);
        let groups: &[SessionPlanGroup] = job.plan.as_ref().filter(|plan| plan.approved && self.plan_marks(id).is_empty()).map_or(&[], |plan| plan.groups.as_slice());
        let place = super::side::placed(job);
        let finished_groups: Vec<&SessionPlanGroup> = groups.iter().filter(|group| place(group) == Stands::Done).collect();
        let finished = self.finished_things(id, &finished_groups, cx);
        let column = |rows: Vec<AnyElement>| (!rows.is_empty()).then(|| div().flex().flex_col().children(rows).into_any_element());
        let mut specs = Vec::new();

        if gates {
            let plan = job.plan.as_ref()?;
            let stretches = stretches_of(&plan.groups, plan.held.as_ref());
            let closes: Vec<Option<String>> = stretches.iter().map(|one| one.gate.as_ref().and_then(|gate| gate.closed.as_ref()).map(|closed| closed.at.as_str().to_string())).collect();
            let last_closed = stretches.iter().rposition(|one| one.state == StretchState::Done).map_or(-1, |at| at as isize);
            let last_key = stretches.last().map(|one| one.key.clone());
            let mut made_in: HashMap<isize, Vec<Finished>> = HashMap::new();
            for one in finished.into_iter().filter(Finished::made) {
                made_in.entry(stretch_of_time(one.time(), &closes, last_closed)).or_default().push(one);
            }
            let (done, live): (Vec<(usize, Stretch)>, Vec<(usize, Stretch)>) = stretches.into_iter().enumerate().partition(|(_, one)| one.state == StretchState::Done);
            for (at, stretch) in &done {
                let key = format!("gate:{at}");
                let mut list: Vec<Finished> = stretch.groups.iter().map(|group| Finished::Group(group, group_time(group))).collect();
                list.extend(made_in.remove(&(*at as isize)).unwrap_or_default());
                let body = column(JobView::gate_closed(stretch.gate.as_ref(), &key, look).into_iter().chain(Some(self.under_done(job, list, &key, look, cx))).collect());
                specs.push(Spec::zone(key, Some(format!("done:{at}")), stretch.name.clone(), Area::Done, "step-done").mark(Mark::Done).body(body));
            }
            let live: Vec<Stretch> = live.into_iter().map(|(_, one)| one).collect();
            let chunks = self.plan_chunks(
                id,
                &live,
                now_body,
                |stretch| {
                    let rows: Vec<AnyElement> = if stretch.state == StretchState::Done {
                        vec![self.under_done(job, stretch.groups.iter().map(|group| Finished::Group(group, group_time(group))).collect(), &stretch.key, look, cx)]
                    } else {
                        self.flight_rows(job, &stretch.groups, stretch.state == StretchState::Next, false, look, cx).into_iter().collect()
                    };
                    let list = (stretch.state == StretchState::Now).then(|| self.now_tasks(job, look, false)).flatten();
                    let landing = (Some(&stretch.key) == last_key.as_ref()).then(|| self.landing(job, &work, look, cx)).flatten();
                    column(rows.into_iter().chain(list).chain(landing).collect())
                },
                look,
                cx,
            );
            let held_open = (!shown && !alone).then(open_work).flatten();
            specs.push(Spec::zone("plan-now".into(), Some("now".into()), "Now", Area::Now, "step-now").lamp(now_lamp()).acts(Some(self.now_acts(id, look, cx))).bare(alone).body(column(std::iter::once(chunks).chain(held_open).collect())));
        } else {
            let walked = !super::now::shown_walks(job, &self.thread_rows(id, cx)).is_empty();
            let at = work.at.unwrap_or_default();
            let last_done = at as isize - 1;
            let mut early = Vec::new();
            let mut done_in: HashMap<isize, Vec<Finished>> = HashMap::new();
            for one in finished {
                if last_done < 0 {
                    early.push(one);
                } else {
                    done_in.entry(step_of_time(one.time(), &work.visits, last_done)).or_default().push(one);
                }
            }
            let upcoming: Vec<&SessionPlanGroup> = groups.iter().filter(|group| place(group) == Stands::Coming).collect();
            let mut now_body = now_body;
            let steps = work.steps.len();
            for (step, name) in work.steps.iter().enumerate() {
                let state = work.state(step);
                let last = step + 1 == steps;
                let skipped = work.skipped.contains(&step) && !last;
                let offer = if last || !can { Offer::None } else { Offer::Skip { step, skipped } };
                let skip = (offer != Offer::None).then(|| self.skip_act(step, skipped, look, cx));
                let key = format!("step:{step}");
                let teach = match state {
                    StretchState::Done => "step-done",
                    StretchState::Now => "step-now",
                    StretchState::Next => "step-next",
                };
                let area = match state {
                    StretchState::Done => Area::Done,
                    StretchState::Now => Area::Now,
                    StretchState::Next => Area::Open,
                };
                if skipped && state != StretchState::Now {
                    specs.push(Spec::zone(key, None, name.clone(), area, teach).mark(Mark::Skipped).skipped(true).acts(skip).offer(offer));
                    continue;
                }
                let spec = match state {
                    StretchState::Done => {
                        let list = sorted(done_in.remove(&(step as isize)).unwrap_or_default());
                        let fold = (!(alone && list.is_empty())).then(|| format!("done:{step}"));
                        let body = (!list.is_empty()).then(|| self.under_done(job, list, &key, look, cx));
                        Spec::zone(key, fold, name.clone(), area, teach).mark(Mark::Done).body(body)
                    }
                    StretchState::Now => {
                        let early = (!early.is_empty()).then(|| self.under_done(job, sorted(std::mem::take(&mut early)), "step-early", look, cx));
                        let held_open = (!shown && !alone).then(open_work).flatten();
                        let landing = last.then(|| self.landing(job, &work, look, cx)).flatten();
                        let rows = early.into_iter().chain(now_body.take()).chain(self.flight_rows(job, &upcoming, true, false, look, cx)).chain(held_open).chain(landing).collect();
                        let acts = div().flex().items_center().gap(look.length(token::SPACE_1)).child(self.now_acts(id, look, cx)).children(skip).into_any_element();
                        Spec::zone(key, Some("now".into()), name.clone(), area, teach).lamp(now_lamp()).acts(Some(acts)).offer(offer).skipped(skipped).body(column(rows))
                    }
                    StretchState::Next => {
                        let acts = can.then(|| div().flex().items_center().gap(look.length(token::SPACE_1)).child(self.jump_act(step, look, cx)).child(skip.unwrap_or_else(|| JobView::act_slot(look))).into_any_element());
                        let walk = (walk_step(name) && !walked).then(|| JobView::walk_to_come(look));
                        let landing = last.then(|| self.landing(job, &work, look, cx)).flatten();
                        let fold = (!alone).then(|| "next".to_string());
                        Spec::zone(key, fold, name.clone(), area, teach).acts(acts).offer(offer).body(column(walk.into_iter().chain(landing).collect()))
                    }
                };
                specs.push(spec);
            }
        }

        // Open work in one place after the last step, so a folded step never hides it. A tab of its
        // own always draws it; the Spine only while the Open group is shown.
        if alone || shown {
            if let Some(body) = open_work() {
                let offer = if alone { Offer::None } else { Offer::HideOpen };
                specs.push(Spec::zone("open-group".into(), Some("open-group".into()), "Open", Area::Open, "step-next").bare(alone).offer(offer).body(Some(body)));
            }
        }
        Some(specs)
    }
}

/// What a step holds, in time order; a thing with no time keeps its place (`doneIn`'s sort).
fn sorted(mut list: Vec<Finished>) -> Vec<Finished> {
    for at in 1..list.len() {
        let mut to = at;
        while to > 0 && matches!((list[to - 1].time(), list[to].time()), (Some(before), Some(after)) if parse(before) > parse(after)) {
            list.swap(to - 1, to);
            to -= 1;
        }
    }
    list
}

/// Which finished step a time falls under: the latest visit that began at or before it, never past the
/// last finished step; with no time or no visits, the last finished step (`stepOfTime`).
pub(super) fn step_of_time(when: Option<&str>, visits: &[(usize, String)], last_done: isize) -> isize {
    let Some(time) = when.and_then(parse) else { return last_done };
    if visits.is_empty() {
        return last_done;
    }
    let hit = visits.iter().fold(-1isize, |found, (step, since)| if parse(since).is_some_and(|since| since <= time) { *step as isize } else { found });
    if hit < 0 {
        0
    } else {
        hit.min(last_done)
    }
}

/// Which finished stretch a time falls under: the first whose gate closed at or after it; with no
/// time, or one since the last close, the last finished stretch (`stretchOfTime`).
pub(super) fn stretch_of_time(when: Option<&str>, closes: &[Option<String>], last_closed: isize) -> isize {
    let Some(time) = when.and_then(parse) else { return last_closed };
    closes.iter().position(|at| at.as_deref().and_then(parse).is_some_and(|at| time <= at)).map_or(last_closed, |at| at as isize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: &str, gate: PlanGate, closed: bool) -> SessionPlanGroup {
        let mut group: SessionPlanGroup = ipc::decode("SessionPlanGroup", format!(r#"{{"id":"{id}","title":"{id}","tasks":[]}}"#).as_bytes()).expect("a group");
        group.gate = gate;
        if closed {
            group.gate_closed = Some(ipc::decode("GateClose", br#"{"at":"2026-11-10T10:00:00Z","fact":"Pull request #1 merged"}"#).expect("a close"));
        }
        group
    }

    #[test]
    fn a_stretch_runs_to_its_gate_and_the_first_open_one_is_now() {
        let groups = vec![group("a", PlanGate::None, false), group("b", PlanGate::Merged, true), group("c", PlanGate::ChecksGreen, false), group("d", PlanGate::None, false)];
        let stretches = stretches_of(&groups, None);
        let read: Vec<(&str, StretchState)> = stretches.iter().map(|one| (one.key.as_str(), one.state)).collect();
        assert_eq!(read, vec![("a+b", StretchState::Done), ("c", StretchState::Now), ("d", StretchState::Next)]);
        assert_eq!(stretches[0].name, "a · b");
    }

    #[test]
    fn a_time_falls_under_the_visit_that_began_before_it() {
        let visits = vec![(0, "2026-11-10T09:00:00Z".to_string()), (1, "2026-11-10T10:00:00Z".to_string()), (2, "2026-11-10T11:00:00Z".to_string())];
        assert_eq!(step_of_time(Some("2026-11-10T10:30:00Z"), &visits, 1), 1);
        assert_eq!(step_of_time(Some("2026-11-10T11:30:00Z"), &visits, 1), 1);
        assert_eq!(step_of_time(Some("2026-11-10T08:00:00Z"), &visits, 1), 0);
        assert_eq!(step_of_time(None, &visits, 1), 1);
    }

    #[test]
    fn a_step_that_asks_for_a_walk_is_named_so() {
        assert!(walk_step("Present the walk"));
        assert!(walk_step("Show it"));
        assert!(!walk_step("Build"));
    }
}
