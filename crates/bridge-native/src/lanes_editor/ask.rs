//! What Publish asks before it writes: which of the Jobs running the workflow move to the new version. A publish moves
//! none by itself (protocol 23.97), so the ask lists each one, where it stands and where it would land, and the ones he
//! ticks are moved after the write. It is drawn inside the Changes card under the bar.

use gpui::{div, prelude::*, AnyElement, Context, FocusHandle, FontWeight, Role, SharedString, Window};

use ipc::{AffectedJob, MigrateResult};

use super::draft::Target;
use super::{LanesEditor, Press};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::change_band::filled;
use crate::settings::definition::steps_of;
use crate::settings::view::{ghost, mono, ringed, tip};
use crate::titlebar::parts::glyph;

/// What the list is headed.
pub(super) const HEADING: &str = "Running Jobs on this workflow";

/// What a Job that edited its own copy says, and its tick's tooltip: a move would lose the edits.
pub(super) const EDITED: &str = "Edited its own copy, so it can't move";

/// One Job in the ask.
pub(super) struct AskRow {
    pub job: AffectedJob,
    /// Where it stands and where a move would put it.
    pub said: String,
    pub picked: bool,
    /// Why its move failed, once the publish is made.
    pub failed: Option<String>,
    pub focus: FocusHandle,
}

impl AskRow {
    /// What the row calls the Job: its title, else its session.
    pub fn name(&self) -> String {
        self.job.title.clone().filter(|title| !title.is_empty()).unwrap_or_else(|| self.job.session_id.as_str().to_string())
    }

    /// Whether the tick can be changed: not for a Job with a copy of its own, and not once the publish is made.
    pub fn pickable(&self) -> bool {
        !self.job.edited
    }
}

/// The ask a Publish makes, while it waits for the answer and, where a move failed, after the write.
pub(super) struct Ask {
    pub rows: Vec<AskRow>,
    pub target: Target,
    /// The write is made: what is left is the moves that failed.
    pub made: bool,
    pub publish: FocusHandle,
    pub cancel: FocusHandle,
    pub done: FocusHandle,
}

impl Ask {
    pub fn new(jobs: Vec<AffectedJob>, target: Target, published: &str, drafted: &str, cx: &mut gpui::App) -> Self {
        let stop = |cx: &mut gpui::App| cx.focus_handle().tab_stop(true);
        let rows = jobs.into_iter().map(|job| AskRow { said: standing(&job, published, drafted), job, picked: false, failed: None, focus: stop(cx) }).collect();
        Ask { rows, target, made: false, publish: stop(cx), cancel: stop(cx), done: stop(cx) }
    }

    /// The moves Fleet answered: a Job that did not move says why under its row. Whether any did not.
    pub fn answered(&mut self, results: &[MigrateResult]) -> bool {
        self.made = true;
        for result in results.iter().filter(|result| !result.moved) {
            if let Some(row) = self.rows.iter_mut().find(|row| row.job.session_id == result.session_id) {
                row.failed = Some(result.message.clone().unwrap_or_else(|| "This Job did not move.".into()));
            }
        }
        self.rows.iter().any(|row| row.failed.is_some())
    }

    /// The stops, in the order they are drawn.
    pub fn stops(&self) -> Vec<FocusHandle> {
        if self.made {
            return vec![self.done.clone()];
        }
        self.rows.iter().filter(|row| row.pickable()).map(|row| row.focus.clone()).chain([self.publish.clone(), self.cancel.clone()]).collect()
    }
}

/// Where a Job stands and where it would land. The landing is the next step after its own, in the order of the text that
/// is published, whose id the draft still has; where none is left, the end.
pub(super) fn standing(job: &AffectedJob, published: &str, drafted: &str) -> String {
    if job.edited {
        return EDITED.into();
    }
    let Some(step) = job.step_id.as_deref() else { return "On no step".into() };
    let label = job.step_label.as_deref().unwrap_or(step);
    if job.survives {
        return format!("On {label}, stays on {label}");
    }
    let kept = steps_of(drafted);
    let landing = steps_of(published)
        .into_iter()
        .skip_while(|(id, _)| id != step)
        .skip(1)
        .find_map(|(id, _)| kept.iter().find(|(held, _)| *held == id).map(|(_, label)| label.clone()));
    match landing {
        Some(next) => format!("On {label}, moves to {next}"),
        None => format!("On {label}, moves to the end"),
    }
}

/// A tick box: empty or ticked.
fn tick(look: &Look, ticked: bool, off: bool) -> gpui::Div {
    let accent = look.color(token::ACCENT);
    div()
        .flex_none()
        .size(look.length(token::SPACE_4))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if ticked { accent } else { look.color(token::BORDER_STRONG) })
        .when(off, |tick| tick.opacity(0.5))
        .when(ticked, |tick| tick.bg(accent).child(glyph("check", look.length(token::SPACE_3), look.color(token::FG_INVERSE))))
}

impl LanesEditor {
    /// The list of Jobs under the bar: a tick, the Job's name, where it stands. None while no ask is open.
    pub(super) fn render_ask(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let ask = self.ask.as_ref()?;
        let heading = div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child(HEADING);
        let rows = ask.rows.iter().enumerate().map(|(n, row)| {
            let name = SharedString::from(row.name());
            let off = !row.pickable() || ask.made || self.busy;
            let line = div()
                .id(SharedString::from(format!("workflow-ask-job-{n}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_size(look.length(token::TEXT_SM))
                .text_color(if row.pickable() { look.color(token::FG_DEFAULT) } else { look.color(token::FG_SUBTLE) })
                .when(!off, |line| line.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .aria_toggled(if row.picked { gpui::accesskit::Toggled::True } else { gpui::accesskit::Toggled::False })
                .child(tick(look, row.picked, off))
                .child(div().flex_none().child(name.clone()))
                .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(row.said.clone())));
            let line = if row.pickable() { ringed(look, line, &row.focus, window).track_focus(&row.focus) } else { line };
            let line = line.when(row.job.edited, |line| line.tooltip(tip(EDITED))).when(!off, |line| line.on_click(cx.listener(move |this, _, window, cx| this.pressed(Press::Pick(n), window, cx))));
            let failed = row.failed.clone().map(|said| labelled(crate::settings::view::refusal(look, said.clone()).id(SharedString::from(format!("workflow-ask-failed-{n}"))).pl(look.length(token::SPACE_8)), Role::Alert, said));
            div().flex().flex_col().min_w_0().child(labelled(line, Role::CheckBox, name)).children(failed)
        });
        let acts = if ask.made {
            let done = ringed(look, filled(look, "workflow-ask-done".into(), "Done", false), &ask.done, window)
                .track_focus(&ask.done)
                .on_click(cx.listener(|this, _, window, cx| this.pressed(Press::AskDone, window, cx)));
            div().flex().items_center().justify_end().child(labelled(done, Role::Button, "Done"))
        } else {
            let publish = ringed(look, filled(look, "workflow-ask-publish".into(), "Publish", self.busy), &ask.publish, window)
                .track_focus(&ask.publish)
                .when(!self.busy, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::AskPublish, window, cx))));
            let cancel = ringed(look, ghost(look, "workflow-ask-cancel", "Cancel", self.busy), &ask.cancel, window)
                .track_focus(&ask.cancel)
                .when(!self.busy, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::AskCancel, window, cx))));
            div().flex().items_center().justify_end().gap(look.length(token::SPACE_2)).child(labelled(cancel, Role::Button, "Cancel")).child(labelled(publish, Role::Button, "Publish"))
        };
        let list = div().id("workflow-ask").flex().flex_col().gap(look.length(token::SPACE_1)).min_w_0().child(heading).children(rows).child(acts);
        Some(labelled(list, Role::Group, HEADING).into_any_element())
    }
}
