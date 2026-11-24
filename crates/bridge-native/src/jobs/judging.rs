//! The Judge on the Jobs list: one act on a row that asks for a reading of the Job, and the row's mark of it, which is the
//! moving mark while it reads, a dot where notes wait and a check where it found nothing. A press on the dot or the check
//! opens the Job with its Judge tab in front.

use gpui::{div, prelude::*, svg, Animation, AnimationExt, AnyElement, Context, Role, SharedString};
use ipc::{AskJudge, JudgeArtifact, JudgeKind, JudgeState, SessionId};

use super::{send, JobsList};
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
    fn said(self) -> &'static str {
        match self {
            Mark::Reading => "Judge reading",
            Mark::Notes => "Notes to read",
            Mark::Clean => "Nothing missed",
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
        let mark = mark_of(self.record(id)?)?;
        let size = look.length(token::SPACE_3);
        let glyph = |name: &'static str, ink: crate::look::Token| svg().path(icon_path(name)).flex_none().size(size).text_color(look.color(ink));
        let said = format!("{}, {title}", mark.said());
        let shell = div().id(SharedString::from(format!("judge-mark-{id}"))).flex_none().flex().items_center().justify_center().size(look.length(token::H_CONTROL_SM)).rounded(look.length(token::RADIUS_SM));
        let tooltip = {
            let said = SharedString::from(mark.said());
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

    /// Ask the Judge to read the Job, as the Job head's Judge does. The row's mark follows Fleet's reading.
    pub(super) fn judge_job(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(job) = self.record(id) else { return };
        let title = titled_or_job(job);
        let artifact = JudgeArtifact { kind: JudgeKind::Job, id: id.to_string(), title };
        let body = ipc::encode(&AskJudge { session_id: SessionId::carried(id.to_string()), artifact, prompt: String::new() }).map_err(|why| format!("{why:?}"));
        send(self.port(cx), "/sessions/judge", body, cx);
    }
}

fn titled_or_job(job: &ipc::SessionRecord) -> String {
    job.title.clone().filter(|title| !title.is_empty()).unwrap_or_else(|| "the Job".to_string())
}
