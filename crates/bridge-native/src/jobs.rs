//! The Jobs list: every live Job, grouped by project, in the order Fleet sends them.
//!
//! A first draft on today's wire. A Job's state mark is not drawn: the word and glyph for it are
//! worked out in Bridge's TypeScript today (`packages/screens/src/session-state.ts`), and Fleet
//! takes that over as a view. Copying it here would make a second vocabulary.

use gpui::{
    div, prelude::*, uniform_list, Context, Entity, FocusHandle, FontWeight, Role, SharedString,
    Subscription, UniformListScrollHandle, Window,
};
use ipc::{SessionRecord, SessionState};

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// One line of the list: a project's name over its Jobs, or a Job.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    Project(SharedString),
    Job { id: SharedString, title: SharedString },
}

/// The lines for `jobs`: ended Jobs leave the list, and each project's Jobs sit under its name,
/// projects in the order their first Job came.
fn lines_of(jobs: &[SessionRecord]) -> Vec<Line> {
    let mut projects: Vec<(String, Vec<&SessionRecord>)> = Vec::new();
    for job in jobs.iter().filter(|job| job.state == SessionState::Live) {
        let project = job.manifest_id.as_ref().map_or_else(|| job.cwd.clone(), |id| id.as_str().to_string());
        match projects.iter_mut().find(|(name, _)| *name == project) {
            Some((_, held)) => held.push(job),
            None => projects.push((project, vec![job])),
        }
    }
    projects
        .into_iter()
        .flat_map(|(project, jobs)| {
            std::iter::once(Line::Project(project.into())).chain(jobs.into_iter().map(|job| Line::Job {
                id: job.id.as_str().to_string().into(),
                title: job.title.clone().unwrap_or_else(|| job.id.as_str().to_string()).into(),
            }))
        })
        .collect()
}

pub struct JobsList {
    lines: Vec<Line>,
    /// The Job the keys are on, by id, so a list that changes under it keeps it.
    focused: Option<SharedString>,
    /// The Job opened with Enter or a press.
    opened: Option<SharedString>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    _watch: Option<Subscription>,
}

impl JobsList {
    /// The list on a live Fleet: it follows the Jobs the connection holds.
    pub fn live(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        let watch = cx.observe(fleet, |this, fleet, cx| {
            let lines = lines_of(&fleet.read(cx).held().jobs);
            if lines != this.lines {
                this.lines = lines;
                cx.notify();
            }
        });
        let lines = lines_of(&fleet.read(cx).held().jobs);
        Self::with(lines, Some(watch), cx)
    }

    /// The list on mock Jobs, which never change.
    pub fn mock(jobs: &[SessionRecord], cx: &mut Context<Self>) -> Self {
        Self::with(lines_of(jobs), None, cx)
    }

    fn with(lines: Vec<Line>, watch: Option<Subscription>, cx: &mut Context<Self>) -> Self {
        let focused = lines.iter().find_map(|line| match line {
            Line::Job { id, .. } => Some(id.clone()),
            Line::Project(_) => None,
        });
        JobsList { lines, focused, opened: None, focus: cx.focus_handle(), scroll: UniformListScrollHandle::new(), _watch: watch }
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    fn job_at(&self, at: usize) -> Option<&SharedString> {
        match self.lines.get(at)? {
            Line::Job { id, .. } => Some(id),
            Line::Project(_) => None,
        }
    }

    fn at_of(&self, id: &SharedString) -> Option<usize> {
        self.lines.iter().position(|line| matches!(line, Line::Job { id: held, .. } if held == id))
    }

    /// Move the keys one Job down (`by` 1) or up (`by` -1), past project names, stopping at the ends.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let from = self.focused.as_ref().and_then(|id| self.at_of(id));
        let mut at = from.map_or(if by > 0 { -1 } else { self.lines.len() as isize }, |at| at as isize);
        loop {
            at += by;
            if at < 0 || at >= self.lines.len() as isize {
                return;
            }
            if let Some(id) = self.job_at(at as usize).cloned() {
                self.focused = Some(id);
                self.scroll.scroll_to_item(at as usize, gpui::ScrollStrategy::Nearest);
                cx.notify();
                return;
            }
        }
    }

    fn on_act(&mut self, act: &Act, _window: &mut Window, cx: &mut Context<Self>) {
        if act.is("move_focus") {
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx);
        } else if act.is("open_focused") {
            if self.opened != self.focused {
                self.opened = self.focused.clone();
                cx.notify();
            }
        } else {
            cx.propagate();
        }
    }

    fn render_line(&self, at: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let row = look.length(token::H_ROW);
        match &self.lines[at] {
            Line::Project(name) => labelled(
                div()
                    .id(("project", at))
                    .w_full()
                    .h(row)
                    .px(look.length(token::SPACE_4))
                    .pt(look.length(token::SPACE_2))
                    .flex()
                    .items_end()
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(name.to_uppercase()),
                Role::Heading,
                name.clone(),
            )
            .into_any_element(),
            Line::Job { id, title } => {
                let focused = self.focused.as_ref() == Some(id);
                let opened = self.opened.as_ref() == Some(id);
                let bar = if focused { look.color(token::ACCENT) } else { gpui::transparent_black() };
                let ground = if opened {
                    look.color(token::SURFACE_ROW_SELECTED)
                } else if focused {
                    look.color(token::SURFACE_ROW_HOVER)
                } else {
                    gpui::transparent_black()
                };
                let pressed = id.clone();
                labelled(
                    div()
                        .id(("job", at))
                        .w_full()
                        .h(row)
                        .flex()
                        .items_center()
                        .border_l(look.length(token::FOCUS_RING_WIDTH))
                        .border_color(bar)
                        .bg(ground)
                        .hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
                        .pl(look.length(token::SPACE_4) - look.length(token::FOCUS_RING_WIDTH))
                        .pr(look.length(token::SPACE_4))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(if opened { token::FG_DEFAULT } else { token::TEXT_BODY }))
                        .font_weight(FontWeight(look.number(if opened { token::WEIGHT_MEDIUM } else { token::WEIGHT_BODY })))
                        .child(div().truncate().child(title.clone()))
                        .on_click(cx.listener(move |this, _event, window, cx| {
                            this.focused = Some(pressed.clone());
                            this.opened = Some(pressed.clone());
                            window.focus(&this.focus, cx);
                            cx.notify();
                        })),
                    Role::Row,
                    title.clone(),
                )
                .into_any_element()
            }
        }
    }
}

impl Render for JobsList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let count = self.lines.len();
        labelled(
            div()
                .id("jobs")
                .key_context("list")
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::on_act))
                .flex()
                .flex_col()
                .size_full()
                .bg(look.color(token::SURFACE_CANVAS))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .child(
                    uniform_list("jobs-lines", count, cx.processor(|this, range: std::ops::Range<usize>, _window, cx| {
                        range.map(|at| this.render_line(at, cx)).collect::<Vec<_>>()
                    }))
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .py(look.length(token::SPACE_2)),
                ),
            Role::List,
            "Jobs",
        )
    }
}

/// The Jobs screen as the window draws it: the list in a column of the sidebar's width, and the
/// rest of the window, where an opened Job will be drawn.
pub struct JobsScreen {
    pub list: Entity<JobsList>,
}

impl Render for JobsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        div()
            .size_full()
            .flex()
            .bg(look.color(token::SURFACE_CANVAS))
            .child(
                div()
                    .w(look.length(token::SIDEBAR_DEFAULT))
                    .h_full()
                    .flex_none()
                    .border_r(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(self.list.clone()),
            )
            .child(div().flex_1())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jobs() -> Vec<SessionRecord> {
        crate::mock::jobs("jobs-many").expect("the fixture decodes")
    }

    #[test]
    fn ended_jobs_leave_the_list_and_each_project_heads_its_own() {
        let lines = lines_of(&jobs());
        assert_eq!(lines[0], Line::Project("armada".into()));
        assert!(!lines.iter().any(|line| matches!(line, Line::Job { title, .. } if title.contains("ended"))));
        let projects: Vec<_> = lines.iter().filter_map(|line| match line { Line::Project(name) => Some(name.clone()), _ => None }).collect();
        assert_eq!(projects, vec![SharedString::from("armada"), "sample-web".into(), "sample-api".into()]);
    }
}
