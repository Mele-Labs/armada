//! The Jobs list: every live Job, grouped by project, in the order Fleet sends them.
//!
//! A first draft on today's wire. A Job's state mark is not drawn: the word and glyph for it are
//! worked out in Bridge's TypeScript today (`packages/screens/src/session-state.ts`), and Fleet
//! takes that over as a view. Copying it here would make a second vocabulary.

use gpui::{
    div, prelude::*, Context, Entity, FocusHandle, FontWeight, Role, SharedString,
    ScrollHandle, Subscription, Window,
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
/// projects in the order their first Job came. One project draws no name, as Electron's list
/// draws a head only when more than one repository is listed (`cockpit/deck/bands.ts`).
fn lines_of(jobs: &[SessionRecord]) -> Vec<Line> {
    let mut projects: Vec<(String, Vec<&SessionRecord>)> = Vec::new();
    for job in jobs.iter().filter(|job| job.state == SessionState::Live) {
        let project = job.manifest_id.as_ref().map_or_else(|| job.cwd.clone(), |id| id.as_str().to_string());
        match projects.iter_mut().find(|(name, _)| *name == project) {
            Some((_, held)) => held.push(job),
            None => projects.push((project, vec![job])),
        }
    }
    let headed = projects.len() > 1;
    projects
        .into_iter()
        .flat_map(|(project, jobs)| {
            headed.then(|| Line::Project(project.into())).into_iter().chain(jobs.into_iter().map(|job| Line::Job {
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
    scroll: ScrollHandle,
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
        JobsList { lines, focused, opened: None, focus: cx.focus_handle(), scroll: ScrollHandle::new(), _watch: watch }
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
                self.scroll.scroll_to_item(at as usize);
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

    /// One line as Electron's Cockpit list draws it (`cockpit/deck/deck.css`, `console-bridge.css`): a
    /// project's head in small mono capitals over a rule, and each Job as a strip with a lamp bar at
    /// its left and a status slot at its right. The lamp and the glyph are a Job's state, which Fleet
    /// does not send yet, so the lamp is idle and the slot is empty.
    fn render_line(&self, at: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = cx.look();
        let after_job = at > 0 && matches!(self.lines[at - 1], Line::Job { .. });
        match &self.lines[at] {
            Line::Project(name) => labelled(
                div()
                    .id(("project", at))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::SPACE_6))
                    .when(at > 0, |head| head.mt(look.length(token::SPACE_3)))
                    .mb(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                    // `--cb-code`: four fifths of the smallest text.
                    .text_size(look.length(token::TEXT_2XS) * 0.8)
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(name.to_uppercase()),
                Role::Heading,
                name.clone(),
            )
            .into_any_element(),
            Line::Job { id, title } => {
                let focused = self.focused.as_ref() == Some(id);
                let opened = self.opened.as_ref() == Some(id);
                let edge = if opened { token::ACCENT } else { token::BORDER_SUBTLE };
                let ground = if opened {
                    look.color(token::ACCENT_MUTED)
                } else if focused {
                    look.color(token::BG_HOVER)
                } else {
                    look.color(token::BG_SUNKEN)
                };
                let weight = if opened { token::WEIGHT_HEADING } else { token::WEIGHT_BODY };
                let pressed = id.clone();
                labelled(
                    div()
                        .id(("job", at))
                        .relative()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::SPACE_6) + look.length(token::SPACE_3))
                        .when(after_job, |row| row.mt(look.length(token::SPACE_1)))
                        .pl(look.length(token::SPACE_3))
                        .pr(look.length(token::SPACE_2))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(edge))
                        .bg(ground)
                        .when(!opened, |row| row.hover(|style| style.bg(look.color(token::BG_HOVER))))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left_0()
                                .w(look.length(token::SPACE_1))
                                .bg(look.color(token::BORDER_STRONG)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_DEFAULT))
                                .font_weight(FontWeight(look.number(weight)))
                                .child(title.clone()),
                        )
                        .child(div().flex_none().w(look.length(token::SPACE_4) * 2. + look.length(token::SPACE_1)))
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
        let lines: Vec<_> = (0..self.lines.len()).map(|at| self.render_line(at, cx)).collect();
        let look = cx.look();
        labelled(
            div()
                .id("jobs")
                .key_context("list")
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::on_act))
                .track_scroll(&self.scroll)
                .overflow_y_scroll()
                .size_full()
                .flex()
                .flex_col()
                .p(look.length(token::SPACE_2))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .children(lines),
            Role::List,
            "Jobs",
        )
    }
}

/// The Jobs screen in Electron's regions (`cockpit/deck/Deck.tsx`, `.deck-grid`): the list's panel at
/// the left, the open Job's room in the middle, and the right column's panel, under the title row.
pub struct JobsScreen {
    pub list: Entity<JobsList>,
}

/// A deck panel's frame (`.deck-panel`): the raised ground, a heavier top edge.
fn panel(look: &crate::look::Look) -> gpui::Div {
    div()
        .h_full()
        .flex_none()
        .bg(look.color(token::BG_RAISED))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
}

impl Render for JobsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let column = look.length(token::SPACE_12);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .child(crate::chrome::title_row(look))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(look.length(token::GAP_PANEL))
                    .p(look.length(token::SPACE_3))
                    .child(panel(look).w(column * 6.).child(self.list.clone()))
                    .child(div().flex_1())
                    .child(panel(look).w(column * 5.75)),
            )
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

    #[test]
    fn one_project_draws_its_jobs_with_no_head() {
        let armada: Vec<_> = jobs().into_iter().filter(|job| job.manifest_id.as_ref().is_some_and(|id| id.as_str() == "armada")).collect();
        let lines = lines_of(&armada);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| matches!(line, Line::Job { .. })));
    }
}
