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
use super::rows::{glyph, tip, Size};
use super::side::Stands;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What one cell of the strip says of its step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PipState {
    Done,
    Now,
    Next,
    /// The person skipped it; the rail counts it passed.
    Skipped,
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
    pub pips: Vec<Pip>,
    /// What the step waits on, as the rail words it, each its own line.
    pub waits: Vec<String>,
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
    let time = at
        .and_then(|at| steps.get(at).map(|word| (at, word)))
        .and_then(|(at, word)| {
            let since = visits.iter().rev().find(|(step, _)| *step == at).map(|(_, since)| since)?;
            Some(format!("{} on {word}", spoken(minutes_between(since, now)?)))
        });
    Strip { flow: name.to_string(), place, time, pips, waits: waits.to_vec() }
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
    Strip { flow: "No workflow".to_string(), place, time: started.and_then(|since| minutes_between(since, now)).map(spoken), pips, waits: Vec::new() }
}

impl JobView {
    /// The strip of the Job, from what Fleet's record says now.
    pub(super) fn strip_for(&self, id: &str, cx: &gpui::App) -> Option<Strip> {
        let job = self.record(id, cx)?;
        let work = self.work(job, cx);
        let now = Utc::now();
        match work.name.as_deref().filter(|_| !work.off) {
            Some(name) => Some(strip_of(name, &work.steps, work.at, &work.visits, &work.skipped, &job.workflow_waiting_on, now)),
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
            .when_some(strip.time.clone(), |head, time| head.child(text(token::TEXT_XS).min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(time))))
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
        labelled(
            band_frame(look, SharedString::from(format!("band-strip:{id}"))).child(head).children(cells).children(waits),
            Role::Group,
            "Progress strip",
        )
        .into_any_element()
    }

    /// One step's cell: its words, then a check and its time where it is done.
    fn strip_cell(&self, id: &SharedString, at: usize, pip: &Pip, look: &Look) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let accent = look.color(token::ACCENT);
        let (ink, weight, ground, border) = match pip.state {
            PipState::Now => (accent, token::WEIGHT_HEADING, look.color(token::BG_RAISED).blend(accent.opacity(0.14)), accent),
            PipState::Done => (look.color(token::FG_MUTED), token::WEIGHT_MEDIUM, look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)),
            PipState::Skipped | PipState::Next => (look.color(token::FG_SUBTLE), token::WEIGHT_BODY, look.color(token::BG_RAISED), look.color(token::BORDER_SUBTLE)),
        };
        let said = match pip.state {
            PipState::Done => "Done",
            PipState::Now => "Under way",
            PipState::Next => "Not started",
            PipState::Skipped => "Skipped",
        };
        let mark = match pip.state {
            PipState::Done => Some(("check", token::STEP_ADVANCED)),
            PipState::Skipped => Some(("minus", token::FG_SUBTLE)),
            PipState::Now | PipState::Next => None,
        };
        let name = match &pip.time {
            Some(time) => format!("{}, {said}, {time}", pip.name),
            None => format!("{}, {said}", pip.name),
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
                .when(pip.time.is_some() || mark.is_some(), |cell| {
                    cell.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .text_color(look.color(token::FG_SUBTLE))
                            .text_size(look.length(token::TEXT_2XS))
                            .when_some(mark, |line, (icon, hue)| line.child(glyph(look, icon, Size::Small, look.color(hue))))
                            .when_some(pip.time.clone(), |line, time| line.child(SharedString::from(time))),
                    )
                }),
            Role::ListItem,
            name,
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn spans_read_in_the_shortest_words() {
        assert_eq!([0, 40, 60, 70, 180].map(spoken), ["<1m", "40m", "1h", "1h 10m", "3h"]);
    }
}
