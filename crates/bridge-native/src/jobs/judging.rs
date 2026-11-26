//! The Judge on the Jobs list: one act on a row that asks for a reading of the Job, and the row's mark of it, which is the
//! moving mark while it reads, a dot where notes wait and a check where it found nothing. A press on the dot or the check
//! opens the Job with its Judge tab in front.

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, Context, Role, SharedString, Window};
use ipc::{JudgeKind, JudgeState};

use super::JobsList;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a row says of the last reading of its Job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mark {
    Reading,
    /// Done, notes not yet opened, some still waiting.
    Notes,
    Clean,
}

impl Mark {
    /// What the row's mark says, naming the judge type that read where one did.
    fn said(self, judged_as: Option<&str>) -> String {
        match (self, judged_as) {
            (Mark::Reading, _) => "Judge reading".to_string(),
            (Mark::Notes, Some(name)) => format!("Judged as {name}: notes waiting"),
            (Mark::Notes, None) => "Notes waiting".to_string(),
            (Mark::Clean, Some(name)) => format!("Judged as {name}: nothing missed"),
            (Mark::Clean, None) => "Nothing missed".to_string(),
        }
    }
}

/// The mark the last reading of a Job leaves on its row, if it leaves one.
pub(super) fn mark_of(job: &ipc::SessionRecord) -> Option<Mark> {
    let reading = job.judge_readings.iter().find(|one| one.artifact.kind == JudgeKind::Job && one.artifact.id == job.id.as_str())?;
    match reading.state {
        JudgeState::Reading => Some(Mark::Reading),
        JudgeState::Failed => None,
        JudgeState::Done if reading.notes.is_empty() => Some(Mark::Clean),
        JudgeState::Done if !reading.read && reading.notes.iter().any(|note| note.settled.is_none()) => Some(Mark::Notes),
        JudgeState::Done => None,
    }
}

impl JobsList {
    /// The mark at the row's end where its Job has a reading to show. A reading going is only a mark; a dot or a check is
    /// a press that opens the Job at its Judge tab.
    pub(super) fn judge_mark(&self, id: &SharedString, title: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let record = self.record(id)?;
        let mark = mark_of(record)?;
        let judged_as = record.judge_readings.iter().find(|one| one.artifact.kind == JudgeKind::Job && one.artifact.id == record.id.as_str()).map(crate::judge_types::name_of);
        let size = look.length(token::SPACE_3);
        let glyph = |name: &'static str, ink: crate::look::Token| svg().path(icon_path(name)).flex_none().size(size).text_color(look.color(ink));
        let said = format!("{}, {title}", mark.said(judged_as.as_deref()));
        let shell = div().id(SharedString::from(format!("judge-mark-{id}"))).flex_none().flex().items_center().justify_center().size(look.length(token::H_CONTROL_SM)).rounded(look.length(token::RADIUS_SM));
        let tooltip = {
            let said = SharedString::from(mark.said(judged_as.as_deref()));
            move |window: &mut gpui::Window, cx: &mut gpui::App| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
        };
        let shell = match mark {
            Mark::Reading => shell.child(
                glyph("circle-dot", token::STEP_RUNNING).with_animation(
                    SharedString::from(format!("judging-{id}")),
                    Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.4, 1.)),
                    |mark, delta| mark.opacity(delta),
                ),
            ),
            Mark::Notes | Mark::Clean => {
                let job = id.clone();
                let dot = if mark == Mark::Notes {
                    div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::STATUS_AWAITING_REVIEW)).into_any_element()
                } else {
                    glyph("circle-check", token::STEP_ADVANCED).into_any_element()
                };
                shell
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(dot)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        crate::job_view::ask_judge_tab(&job, window, cx);
                        this.open_job(&job, cx);
                    }))
            }
        };
        Some(labelled(shell.tooltip(tooltip), if mark == Mark::Reading { Role::Status } else { Role::Button }, said).into_any_element())
    }

    /// Judge on a row: the Job opens with the list of what it holds, and the one chosen gets the box of judge types. The
    /// row's mark follows Fleet's reading.
    pub(super) fn judge_job(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.record(id).is_none() {
            return;
        }
        crate::job_view::ask_judge_choice(id, window, cx);
        self.open_job(id, cx);
    }
}
