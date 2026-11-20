//! The progress strip over a Job's right column: the workflow, the step of its steps, the time on it and what it
//! waits on, with each step a cell under it. A step done shows a check and the time it took; the step under way is
//! in the accent. A Job on no workflow shows its own plan's groups as the cells.
//!
//! What it says is worked out from the record by [`strip_of`] and [`own_plan_strip`], which draw nothing, so what a
//! step reads can be tested without a window. [`JobView::column_strip`] draws it, under the band's x
//! (`column_bands.rs`).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};

use super::column_bands::{band_frame, Band};
use super::composer::Kept;
use super::rows::{glyph, tip, Size};
use super::side::Stands;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::operator::view::{press, pressed, Weight};

/// What one cell of the strip says of its step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PipState {
    Done,
    Now,
    Next,
    /// The person skipped it; the rail counts it passed.
    Skipped,
    /// The step under way has stood past its usual time with no move. Amber.
    Stalled,
    /// A step done that was passed without his Approve: work ahead of it was seen. Amber.
    Unapproved,
}

impl PipState {
    /// The step the Job is on, moving or not.
    pub(crate) fn is_now(self) -> bool {
        matches!(self, PipState::Now | PipState::Stalled)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pip {
    pub name: String,
    pub state: PipState,
    /// How long the step took, on a step done. Absent while a step has no closed visit.
    pub time: Option<String>,
}

/// The strip's words, as the header and the cells read them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Strip {
    /// The workflow's name, or `No workflow`.
    pub flow: String,
    /// `Step 2 of 3` on a workflow; `own plan` where the cells are the plan's groups. Empty where there is none to say.
    pub place: String,
    /// `40m on Walk` on a workflow; how long the Job has run where it follows none.
    pub time: Option<String>,
    /// Only the span of `time` on a workflow: `40m`.
    pub step_time: Option<String>,
    pub pips: Vec<Pip>,
    /// What the step waits on, as the rail words it, each its own line.
    pub waits: Vec<String>,
    /// The step has stood past its usual time with no move: why, and the acts that move it.
    pub stalled: Option<Stalled>,
}

/// A step that is not moving, as the strip's amber box says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stalled {
    /// Fleet's sentence: what the step waits on, and whose it is.
    pub cause: String,
    /// The step, by its index, and its words: what Ask the Job names, and what Skip passes.
    pub step: usize,
    pub name: String,
    /// The step Skip moves the Job to: the next one not done or skipped. None on the last step.
    pub next: Option<String>,
}

/// What Ask the Job puts in the box: a question about the step, after what he had typed.
pub(super) fn step_question(step: &str, held: &str) -> String {
    let line = format!("What is {step} waiting on?");
    if held.trim().is_empty() {
        line
    } else {
        format!("{held}\n\n{line}")
    }
}

/// A span of minutes as the strip says it: `<1m`, `40m`, `3h`, `1h 10m`.
pub(super) fn spoken(minutes: i64) -> String {
    match minutes.max(0) {
        0 => "<1m".to_string(),
        m @ 1..=59 => format!("{m}m"),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{}h {:02}m", m / 60, m % 60),
    }
}

fn minutes_between(from: &str, to: DateTime<Utc>) -> Option<i64> {
    let from = super::workflow::parse(from)?;
    Some((to - from.with_timezone(&Utc)).num_minutes())
}

/// The minutes the Job has spent in `step`: each visit to it lasts until the next visit began. The visit the Job is
/// still in is not counted here, since it has no end.
fn closed_minutes(visits: &[(usize, String)], step: usize) -> Option<i64> {
    let mut total = None;
    for (at, (visited, since)) in visits.iter().enumerate() {
        let Some((_, until)) = visits.get(at + 1) else { break };
        if *visited != step {
            continue;
        }
        let (Some(from), Some(until)) = (super::workflow::parse(since), super::workflow::parse(until)) else { continue };
        *total.get_or_insert(0) += (until - from).num_minutes();
    }
    total
}

/// The strip of a Job that follows a workflow: `steps` are its steps' words, `at` the one it is on and `visits` when it
/// came to each. A Job whose workflow Fleet does not hold has a name and no cells.
pub(super) fn strip_of(name: &str, steps: &[String], at: Option<usize>, visits: &[(usize, String)], skipped: &HashSet<usize>, waits: &[String], now: DateTime<Utc>) -> Strip {
    // Fleet leaves the step off once the last step's pull request merged: every step is done then.
    let finished = at.is_none() && !visits.is_empty();
    let pips = steps
        .iter()
        .enumerate()
        .map(|(step, word)| {
            let state = match at {
                _ if skipped.contains(&step) => PipState::Skipped,
                Some(at) if step < at => PipState::Done,
                Some(at) if step == at => PipState::Now,
                Some(_) => PipState::Next,
                None if finished => PipState::Done,
                None => PipState::Next,
            };
            let time = (state == PipState::Done).then(|| closed_minutes(visits, step).map(spoken)).flatten();
            Pip { name: word.clone(), state, time }
        })
        .collect();
    let place = at.filter(|_| !steps.is_empty()).map(|at| format!("Step {} of {}", at + 1, steps.len())).unwrap_or_default();
    let on_step = at.and_then(|at| steps.get(at).map(|word| (at, word))).and_then(|(at, word)| {
        let since = visits.iter().rev().find(|(step, _)| *step == at).map(|(_, since)| since)?;
        Some((spoken(minutes_between(since, now)?), word))
    });
    let time = on_step.as_ref().map(|(span, word)| format!("{span} on {word}"));
    Strip { flow: name.to_string(), place, time, step_time: on_step.map(|(span, _)| span), pips, waits: waits.to_vec(), stalled: None }
}

impl Strip {
    /// Fleet's reading of the step laid over the strip: a step done that work ahead passed is amber, and the step under way
    /// is amber with its cause where it stands past its usual time. A Job Fleet sent no reading of is left as it is.
    pub(super) fn read_by(mut self, wire: Option<&ipc::WorkflowStrip>) -> Strip {
        let Some(wire) = wire else { return self };
        for (at, pip) in self.pips.iter_mut().enumerate() {
            if pip.state == PipState::Done && wire.passed_unapproved.contains(&(at as u32)) {
                pip.state = PipState::Unapproved;
            }
        }
        let Some(stalled) = wire.stalled.as_ref() else { return self };
        let Some(at) = self.pips.iter().position(|pip| pip.state == PipState::Now) else { return self };
        let next = self.pips.iter().skip(at + 1).find(|pip| pip.state == PipState::Next).map(|pip| pip.name.clone());
        self.pips[at].state = PipState::Stalled;
        self.pips[at].time = self.step_time.clone();
        self.stalled = Some(Stalled { cause: stalled.cause.clone(), step: at, name: self.pips[at].name.clone(), next });
        self
    }
}

/// The strip of a Job on no workflow: its own plan's groups as the cells, the one at work in the accent, and how long it
/// has run.
pub(super) fn own_plan_strip(groups: &[(String, Stands)], started: Option<&str>, now: DateTime<Utc>) -> Strip {
    let pips: Vec<Pip> = groups
        .iter()
        .map(|(title, stands)| Pip {
            name: title.clone(),
            state: match stands {
                Stands::Done => PipState::Done,
                Stands::Working => PipState::Now,
                Stands::Coming => PipState::Next,
            },
            time: None,
        })
        .collect();
    let place = if pips.is_empty() { String::new() } else { "own plan".to_string() };
    Strip { flow: "No workflow".to_string(), place, time: started.and_then(|since| minutes_between(since, now)).map(spoken), step_time: None, pips, waits: Vec::new(), stalled: None }
}

/// What a Jobs list row says of the Job's steps: a bar for each, the step it is on and how long it has been on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Track {
    pub pips: Vec<PipState>,
    pub step: Option<String>,
    pub time: Option<String>,
    /// Why the step is not moving, in Fleet's words, where it stands past its usual time.
    pub stalled: Option<String>,
}

/// The track of a Job that follows a workflow Fleet holds, or whose own copy it carries. None where it follows none, or Fleet holds
/// no definition of the one it follows.
pub(crate) fn track_of(job: &ipc::SessionRecord, held: &[ipc::WorkflowSummary], now: DateTime<Utc>) -> Option<Track> {
    let id = job.workflow.as_deref()?;
    let def = job
        .workflow_copy
        .as_ref()
        .filter(|copy| copy.id.as_str() == id)
        .or_else(|| held.iter().find(|one| one.id.as_str() == id && job.manifest_id.as_ref().is_none_or(|manifest| *manifest == one.manifest_id)))
        .or_else(|| held.iter().find(|one| one.id.as_str() == id))?;
    let steps: Vec<String> = def.steps.iter().map(|step| step.label.clone()).collect();
    let at = job.workflow_step.map(|at| at as usize).filter(|at| *at < steps.len());
    let visits: Vec<(usize, String)> = job.workflow_visits.iter().map(|visit| (visit.step as usize, visit.since.as_str().to_string())).collect();
    let skipped: HashSet<usize> = job.skipped_steps.iter().map(|step| *step as usize).collect();
    let strip = strip_of(&def.name, &steps, at, &visits, &skipped, &[], now).read_by(job.workflow_strip.as_ref());
    Some(Track { pips: strip.pips.iter().map(|pip| pip.state).collect(), step: at.map(|at| steps[at].clone()), time: strip.step_time.clone(), stalled: strip.stalled.map(|stalled| stalled.cause) })
}

/// A Job's steps as small bars, one for each, in the state each is in: done in the muted ink, the step under way in the accent,
/// and the rest in the hairline's. Said once as a whole, so a reader of the screen hears the steps and not each bar.
pub(crate) fn pips_bar(look: &Look, id: impl Into<gpui::ElementId>, states: &[PipState], said: impl Into<SharedString>) -> AnyElement {
    let said: SharedString = said.into();
    labelled(
        div().id(id).flex_none().flex().items_center().gap(look.length(token::SPACE_1) / 2.).tooltip(tip(said.clone())).children(states.iter().map(|state| {
            let hue = match state {
                PipState::Done => look.color(token::FG_MUTED),
                PipState::Now => look.color(token::ACCENT),
                PipState::Stalled | PipState::Unapproved => look.color(token::STATUS_AWAITING_REVIEW),
                PipState::Skipped => look.color(token::FG_SUBTLE),
                PipState::Next => look.color(token::BORDER_DEFAULT),
            };
            div().flex_none().w(look.length(token::SPACE_3)).h(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).bg(hue)
        })),
        Role::Image,
        said,
    )
    .into_any_element()
}

impl JobView {
    /// The strip of the Job, from what Fleet's record says now.
    pub(super) fn strip_for(&self, id: &str, cx: &gpui::App) -> Option<Strip> {
        let job = self.record(id, cx)?;
        let work = self.work(job, cx);
        let now = Utc::now();
        match work.name.as_deref().filter(|_| !work.off) {
            Some(name) => Some(strip_of(name, &work.steps, work.at, &work.visits, &work.skipped, &job.workflow_waiting_on, now).read_by(job.workflow_strip.as_ref())),
            None => {
                let place = super::side::placed(job);
                let groups: Vec<(String, Stands)> = job.plan.iter().filter(|plan| plan.approved).flat_map(|plan| plan.groups.iter()).map(|group| (group.title.clone(), place(group))).collect();
                Some(own_plan_strip(&groups, Some(job.started_at.as_str()), now))
            }
        }
    }

    /// The strip drawn: a header of the workflow, its place and the time, a cell for each step, and what it waits on.
    pub(super) fn column_strip(&self, id: &SharedString, strip: &Strip, look: &Look, cx: &Context<Self>) -> AnyElement {
        let text = |size: crate::look::Token| div().text_size(look.length(size)).line_height(look.length(token::LEADING_XS));
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .child(text(token::TEXT_XS).flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(strip.flow.clone())))
            .when(!strip.place.is_empty(), |head| head.child(text(token::TEXT_XS).flex_none().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(strip.place.clone()))))
            .when_some(strip.time.clone(), |head, time| head.child(text(token::TEXT_XS).min_w_0().truncate().text_color(look.color(if strip.stalled.is_some() { token::STATUS_AWAITING_REVIEW } else { token::FG_MUTED })).child(SharedString::from(time))))
            .child(div().flex_1())
            .child(self.band_hide(id, Band::Strip, look, cx));
        let cells = (!strip.pips.is_empty()).then(|| {
            labelled(
                div().id(SharedString::from(format!("strip-steps:{id}"))).flex().gap(look.length(token::SPACE_1)).children(strip.pips.iter().enumerate().map(|(at, pip)| self.strip_cell(id, at, pip, look))),
                Role::List,
                "Steps",
            )
        });
        let waits = strip.waits.iter().enumerate().map(|(at, said)| {
            labelled(
                div()
                    .id(SharedString::from(format!("strip-waits:{id}:{at}")))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(div().id(SharedString::from(format!("strip-waits-mark:{id}:{at}"))).flex_none().tooltip(tip("Waits on")).child(glyph(look, "clock", Size::Small, look.color(token::FG_SUBTLE))))
                    .child(text(token::TEXT_XS).min_w_0().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said.clone()))),
                Role::Group,
                said.clone(),
            )
        });
        let stalled = strip.stalled.as_ref().map(|stalled| self.stalled_box(id, stalled, look, cx));
        labelled(
            band_frame(look, SharedString::from(format!("band-strip:{id}"))).child(head).children(cells).children(waits).children(stalled),
            Role::Group,
            "Progress strip",
        )
        .into_any_element()
    }

    /// Why the step is not moving, in an amber box, with the two acts that move it: Ask the Job puts a question about the step
    /// in the message box and sends nothing, and Skip passes the step to the next.
    fn stalled_box(&self, id: &SharedString, stalled: &Stalled, look: &Look, cx: &Context<Self>) -> AnyElement {
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let (name, step) = (stalled.name.clone(), stalled.step);
        let ask = labelled(
            pressed(press(format!("strip-stalled-ask:{id}"), "Ask the Job".into(), None, None, Weight::Plain, look), cx, move |this, window, cx| this.ask_about_step(&name, window, cx)),
            Role::Button,
            "Ask the Job",
        );
        let skip = stalled.next.as_ref().map(|next| {
            let words = format!("Skip to {next}");
            labelled(pressed(press(format!("strip-stalled-skip:{id}"), words.clone().into(), None, None, Weight::Plain, look), cx, move |this, window, cx| this.skip_step(step, true, window, cx)), Role::Button, words)
        });
        labelled(
            div()
                .id(SharedString::from(format!("strip-stalled:{id}")))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(amber)
                .bg(look.color(token::BG_RAISED).blend(amber.opacity(0.10)))
                .child(div().id(SharedString::from(format!("strip-stalled-mark:{id}"))).flex_none().tooltip(tip("Not moving")).child(glyph(look, "triangle-alert", Size::Small, amber)))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(stalled.cause.clone())))
                        .child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).child(ask).children(skip)),
                ),
            Role::Group,
            format!("{} is not moving: {}", stalled.name, stalled.cause),
        )
        .into_any_element()
    }

    /// Ask the Job: a question about the step goes in the message box after what he had typed, and the caret with it. Nothing is sent.
    pub(super) fn ask_about_step(&mut self, step: &str, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let held = self.kept(cx);
        let kept = Kept { text: step_question(step, &held.text), tags: held.tags };
        self.set_words(kept, window, cx);
        self.to_composer(window, cx);
    }

    /// One step's cell: its words, then a check and its time where it is done.
    fn strip_cell(&self, id: &SharedString, at: usize, pip: &Pip, look: &Look) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let accent = look.color(token::ACCENT);
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let (ink, weight, ground, border) = match pip.state {
            PipState::Now => (accent, token::WEIGHT_HEADING, look.color(token::BG_RAISED).blend(accent.opacity(0.14)), accent),
            PipState::Stalled => (amber, token::WEIGHT_HEADING, look.color(token::BG_RAISED).blend(amber.opacity(0.14)), amber),
            PipState::Unapproved => (amber, token::WEIGHT_HEADING, look.color(token::BG_RAISED).blend(amber.opacity(0.10)), amber),
            PipState::Done => (look.color(token::FG_MUTED), token::WEIGHT_MEDIUM, look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)),
            PipState::Skipped | PipState::Next => (look.color(token::FG_SUBTLE), token::WEIGHT_BODY, look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)),
        };
        let said = match pip.state {
            PipState::Done => "Done",
            PipState::Now => "Under way",
            PipState::Stalled => "Not moving",
            PipState::Unapproved => "Passed without your Approve",
            PipState::Next => "Not started",
            PipState::Skipped => "Skipped",
        };
        let mark = match pip.state {
            PipState::Done => Some(("check", token::STEP_ADVANCED)),
            PipState::Skipped => Some(("minus", token::FG_SUBTLE)),
            PipState::Stalled | PipState::Unapproved => Some(("triangle-alert", token::STATUS_AWAITING_REVIEW)),
            PipState::Now | PipState::Next => None,
        };
        // A step passed with no Approve says so where the time of the others stands.
        let time = if pip.state == PipState::Unapproved { Some("no Approve".to_string()) } else { pip.time.clone() };
        let name = match (&pip.time, pip.state) {
            (Some(time), state) if state != PipState::Unapproved => format!("{}, {said}, {time}", pip.name),
            _ => format!("{}, {said}", pip.name),
        };
        labelled(
            div()
                .id(SharedString::from(format!("strip-step:{id}:{at}")))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .items_center()
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .border(edge)
                .border_color(border)
                .bg(ground)
                .text_color(ink)
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(weight)))
                .tooltip(tip(said))
                .child(div().max_w_full().truncate().child(SharedString::from(pip.name.clone())))
                .when(time.is_some() || mark.is_some(), |cell| {
                    cell.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .text_color(if matches!(pip.state, PipState::Stalled | PipState::Unapproved) { amber } else { look.color(token::FG_SUBTLE) })
                            .text_size(look.length(token::TEXT_2XS))
                            .when_some(mark, |line, (icon, hue)| line.child(glyph(look, icon, Size::Small, look.color(hue))))
                            .when_some(time, |line, time| line.child(SharedString::from(time))),
                    )
                }),
            Role::ListItem,
            name,
        )
        .into_any_element()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use chrono::TimeZone;

    fn words(of: &[&str]) -> Vec<String> {
        of.iter().map(|word| word.to_string()).collect()
    }

    fn at(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 20, hour, minute, 0).single().expect("a time")
    }

    fn visit(step: usize, hour: u32, minute: u32) -> (usize, String) {
        (step, at(hour, minute).to_rfc3339())
    }

    /// The Walk step of a feature: Propose took 25 minutes, and the Job has been on Walk for 40.
    #[test]
    fn a_step_reads_its_place_its_time_and_what_it_waits_on() {
        let steps = words(&["Propose", "Walk", "Deliver"]);
        let visits = [visit(0, 10, 0), visit(1, 10, 25)];
        let waits = words(&["Waiting on walk: Settings overlay"]);
        let strip = strip_of("feature", &steps, Some(1), &visits, &HashSet::new(), &waits, at(11, 5));
        assert_eq!(strip.flow, "feature");
        assert_eq!(strip.place, "Step 2 of 3");
        assert_eq!(strip.time.as_deref(), Some("40m on Walk"));
        assert_eq!(strip.waits, waits);
        assert_eq!(
            strip.pips,
            vec![
                Pip { name: "Propose".into(), state: PipState::Done, time: Some("25m".into()) },
                Pip { name: "Walk".into(), state: PipState::Now, time: None },
                Pip { name: "Deliver".into(), state: PipState::Next, time: None },
            ]
        );
    }

    /// A step the Job came back to counts both visits, and one it skipped is passed with no time.
    #[test]
    fn a_step_visited_twice_adds_its_visits_and_a_skipped_one_has_no_time() {
        let steps = words(&["Propose", "Walk", "Deliver"]);
        let visits = [visit(0, 9, 0), visit(1, 9, 30), visit(0, 10, 0), visit(1, 10, 20), visit(2, 11, 0)];
        let skipped: HashSet<usize> = [1].into_iter().collect();
        let strip = strip_of("feature", &steps, Some(2), &visits, &skipped, &[], at(11, 10));
        assert_eq!(strip.pips[0].time.as_deref(), Some("50m"), "30m, then 20m");
        assert_eq!((strip.pips[1].state, strip.pips[1].time.clone()), (PipState::Skipped, None));
        assert_eq!(strip.time.as_deref(), Some("10m on Deliver"));
        assert!(strip.waits.is_empty(), "an empty slot stays empty");
    }

    /// Once the last step's pull request merged Fleet names no step: every cell is done and no time runs.
    #[test]
    fn a_finished_workflow_has_every_step_done_and_no_time_on_one() {
        let steps = words(&["Propose", "Deliver"]);
        let visits = [visit(0, 9, 0), visit(1, 9, 30)];
        let strip = strip_of("feature", &steps, None, &visits, &HashSet::new(), &[], at(12, 0));
        assert!(strip.pips.iter().all(|pip| pip.state == PipState::Done));
        assert_eq!((strip.place.as_str(), strip.time), ("", None));
    }

    /// A Job on no workflow says so, shows its plan's groups, and reads how long it has run.
    #[test]
    fn a_job_on_no_workflow_shows_its_plan_groups() {
        let groups = vec![("Inventory".to_string(), Stands::Done), ("Quiz".to_string(), Stands::Working), ("Sketch".to_string(), Stands::Coming)];
        let strip = own_plan_strip(&groups, Some(&at(10, 0).to_rfc3339()), at(11, 10));
        assert_eq!((strip.flow.as_str(), strip.place.as_str(), strip.time.as_deref()), ("No workflow", "own plan", Some("1h 10m")));
        assert_eq!(strip.pips.iter().map(|pip| pip.state).collect::<Vec<_>>(), [PipState::Done, PipState::Now, PipState::Next]);
        // With no plan there is nothing to place, so the place is left unsaid.
        assert_eq!(own_plan_strip(&[], None, at(11, 10)).place, "");
    }

    /// A row's track is the strip's own reading of the record: a bar for each step of the definition, the one it is on in front.
    #[test]
    fn the_mock_jobs_cover_each_state_the_strip_draws() {
        let held = crate::mock::workflows();
        let jobs = crate::mock::jobs("job-workflow").expect("the fixture decodes");
        let by = |id: &str| jobs.iter().find(|job| job.id.as_str() == id).expect("the Job is in the fixture");
        let at = |id: &str| track_of(by(id), &held, Utc::now()).expect("a track").step;
        assert_eq!(at("mock-16").as_deref(), Some("Walk"));
        assert_eq!(at("mock-17").as_deref(), Some("Build and deliver"));
        assert_eq!(at("mock-18").as_deref(), Some("Find the cause"));
        assert!(matches!(by("mock-18").mark.as_ref().map(|mark| &mark.state), Some(ipc::JobState::Stuck)));
        for id in ["mock-16", "mock-17", "mock-18", "mock-19"] {
            assert!(!by(id).workflow_visits.is_empty() && !by(id).workflow_waiting_on.is_empty(), "{id} shows a time and what it waits on");
        }
        let kids: Vec<&str> = by("mock-19").attachments.iter().filter(|one| one.kind == "started_to").map(|one| one.target.as_str()).collect();
        assert_eq!(kids.len(), 3);
        let steps: HashSet<Option<String>> = kids.iter().map(|id| at(id)).collect();
        assert_eq!(steps.len(), 3, "the three children are on different steps");
        assert!(by("mock-04").workflow.is_none(), "one Job follows no workflow");
    }

    #[test]
    fn a_row_reads_its_steps_from_the_workflow_fleet_holds() {
        let held = crate::mock::workflows();
        let jobs = crate::mock::jobs("job-workflow").expect("the fixture decodes");
        let job = jobs.iter().find(|job| job.workflow.is_some() && job.workflow_step.is_some()).expect("a Job on a step");
        let def = job.workflow_copy.as_ref().filter(|copy| Some(copy.id.as_str()) == job.workflow.as_deref()).or_else(|| held.iter().find(|one| Some(one.id.as_str()) == job.workflow.as_deref())).expect("its definition");
        let track = track_of(job, &held, Utc::now()).expect("a track");
        assert_eq!(track.pips.len(), def.steps.len());
        assert_eq!(track.pips.iter().filter(|state| state.is_now()).count(), 1);
        assert_eq!(track.step.as_deref(), job.workflow_step.and_then(|at| def.steps.get(at as usize)).map(|step| step.label.as_str()));
        let none = jobs.iter().find(|job| job.workflow.is_none());
        assert!(none.is_none_or(|job| track_of(job, &held, Utc::now()).is_none()), "no workflow, no track");
    }

    #[test]
    fn spans_read_in_the_shortest_words() {
        assert_eq!([0, 40, 60, 70, 180].map(spoken), ["<1m", "40m", "1h", "1h 10m", "3h"]);
    }

    fn wire(stalled: Option<&str>, passed: &[u32]) -> ipc::WorkflowStrip {
        ipc::WorkflowStrip { stalled: stalled.map(|cause| ipc::StepStalled { since: ipc::Instant::carried("2026-11-20T09:00:00Z"), cause: cause.to_string() }), passed_unapproved: passed.to_vec(), ..ipc::WorkflowStrip::default() }
    }

    /// The Bug step that waits on a plan for two hours: the step and its time turn amber, the cause is Fleet's, and Skip names the next step.
    #[test]
    fn a_step_past_its_usual_time_is_amber_with_its_cause_and_the_step_skip_moves_to() {
        let steps = words(&["Find the cause", "Fix", "Deliver"]);
        let visits = [visit(0, 10, 0)];
        let cause = "The step waits on a plan from the Job, and none is recorded.";
        let strip = strip_of("bug", &steps, Some(0), &visits, &HashSet::new(), &[], at(12, 0)).read_by(Some(&wire(Some(cause), &[])));
        assert_eq!(strip.pips[0], Pip { name: "Find the cause".into(), state: PipState::Stalled, time: Some("2h".into()) });
        assert_eq!(strip.time.as_deref(), Some("2h on Find the cause"));
        assert_eq!(strip.stalled, Some(Stalled { cause: cause.into(), step: 0, name: "Find the cause".into(), next: Some("Fix".into()) }));
        assert!(strip.pips[0].state.is_now(), "it is still the step under way");
    }

    #[test]
    fn a_step_inside_its_time_stays_as_it_was_and_the_last_step_has_nothing_to_skip_to() {
        let steps = words(&["Find the cause", "Fix"]);
        let visits = [visit(0, 10, 0), visit(1, 10, 30)];
        let plain = strip_of("bug", &steps, Some(1), &visits, &HashSet::new(), &[], at(12, 0));
        assert_eq!(plain.clone().read_by(Some(&wire(None, &[]))), plain, "no stall, nothing changes");
        assert_eq!(plain.clone().read_by(None), plain, "an older Fleet sends no reading");
        let last = plain.read_by(Some(&wire(Some("Waits on checks."), &[])));
        assert_eq!(last.stalled.map(|stalled| stalled.next), Some(None), "Skip is not offered on the last step");
    }

    /// A step the rail moved past because work ahead was seen: amber, and it says so where the others say their time.
    #[test]
    fn a_step_passed_without_his_approve_is_amber_and_a_step_not_done_is_left_alone() {
        let steps = words(&["Propose", "Walk", "Deliver"]);
        let visits = [visit(0, 10, 0), visit(1, 10, 20), visit(2, 10, 30)];
        let skipped: HashSet<usize> = [1].into_iter().collect();
        let strip = strip_of("feature", &steps, Some(2), &visits, &skipped, &[], at(11, 0)).read_by(Some(&wire(None, &[0, 1, 2])));
        assert_eq!(strip.pips.iter().map(|pip| pip.state).collect::<Vec<_>>(), [PipState::Unapproved, PipState::Skipped, PipState::Now], "only a step done is marked");
    }

    #[test]
    fn the_mock_jobs_show_a_stalled_step_and_a_step_passed_without_his_approve() {
        let held = crate::mock::workflows();
        let jobs = crate::mock::jobs("job-workflow").expect("the fixture decodes");
        let track = |id: &str| track_of(jobs.iter().find(|job| job.id.as_str() == id).expect("the Job"), &held, Utc::now()).expect("a track");
        let bug = track("mock-18");
        assert_eq!(bug.pips[0], PipState::Stalled);
        assert!(bug.stalled.is_some_and(|cause| cause.contains("plan")));
        assert_eq!(track("mock-17").pips[0], PipState::Unapproved);
        assert!(track("mock-17").stalled.is_none() && track("mock-16").stalled.is_none());
    }

    #[test]
    fn ask_the_job_words_a_question_about_the_step_after_what_he_had_typed() {
        assert_eq!(step_question("Find the cause", ""), "What is Find the cause waiting on?");
        assert_eq!(step_question("Fix", "  "), "What is Fix waiting on?");
        assert_eq!(step_question("Fix", "Check the cart"), "Check the cart\n\nWhat is Fix waiting on?");
    }

    pub(in crate::job_view) mod drawn {
        use super::super::super::parts_fold::{drawn, redraw};
        use super::super::super::JobView;
        use gpui::{prelude::*, Bounds, Focusable as _, Pixels, Role};

        const JOB: &str = "mock-18";

        /// The workflow walk's Jobs with `job` open.
        pub(in crate::job_view) fn opened<'a>(cx: &'a mut gpui::TestAppContext, job: &str) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
            cx.update(|cx| {
                gpui_component::init(cx);
                crate::look::init(cx);
            });
            let jobs = crate::mock::jobs("job-workflow").expect("the fixture decodes");
            let threads = crate::mock::threads("job-workflow").expect("the threads decode");
            let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
            let (view, cx) = cx.add_window_view({
                let list = list.clone();
                move |window, cx| JobView::new(list, Some(threads), window, cx)
            });
            cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(1200.)));
            list.update(cx, |list, cx| list.open(job.into(), cx));
            redraw(cx);
            fresh(cx);
            (view, cx)
        }

        pub(in crate::job_view) fn fresh(cx: &mut gpui::VisualTestContext) {
            redraw(cx);
            cx.update(|_, cx| cx.set_global(crate::labels::LabelBook::default()));
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }

        fn bounds_of(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Bounds<Pixels> {
            cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
                .unwrap_or_else(|| panic!("{name} was not drawn"))
        }

        fn press(cx: &mut gpui::VisualTestContext, name: &str) {
            let at = bounds_of(cx, Role::Button, name).center();
            cx.simulate_click(at, gpui::Modifiers::none());
            fresh(cx);
        }

        #[gpui::test]
        fn a_stalled_step_draws_its_cause_and_both_acts_and_ask_the_job_sends_nothing(cx: &mut gpui::TestAppContext) {
            let (view, cx) = opened(cx, JOB);
            assert!(drawn(cx, Role::Group, "Find the cause is not moving: The step waits on a plan from the Job, and none is recorded."));
            assert!(drawn(cx, Role::Button, "Skip to Fix"));
            press(cx, "Ask the Job");
            let focused = cx.update(|window, cx| view.read(cx).composer.input.read(cx).focus_handle(cx).is_focused(window));
            assert!(focused, "the caret is in the box");
            view.read_with(cx, |view, cx| assert_eq!(view.composer.input.read(cx).value().as_ref(), "What is Find the cause waiting on?", "the question is typed for him, and nothing is sent"));
        }

        #[gpui::test]
        fn skip_passes_the_stalled_step_with_the_act_the_spine_uses(cx: &mut gpui::TestAppContext) {
            let (view, cx) = opened(cx, JOB);
            press(cx, "Skip to Fix");
            view.read_with(cx, |view, _| assert_eq!(view.pages.get(JOB).and_then(|page| page.side.skips.get(&0)).copied(), Some((false, true)), "step 0 is skipped"));
        }
    }
}
