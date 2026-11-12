//! The Job's right column, as Electron's `StoryColumn` draws it in its two default panes: Open, Done
//! and Deferred over Now. Each tab has its × and each strip its +; the top pane's + also opens Plan,
//! Changes, Ledger, Files, Subagents, Checks (`panes.rs`) and Judge (`judge_tab.rs`) until the pane
//! system can place them anywhere.
//!
//! Now is `now.rs`.

use std::collections::HashSet;

use gpui::{div, prelude::*, AnyElement, Context, Entity, FontWeight, Role, SharedString, Subscription, Window};
use ipc::{PlanTaskState, SessionPlanGroup};

use super::rows::{glyph, Size};
use super::tabs::Pane;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What Ask for an update sends (`ASK_FOR_UPDATE`, `cockpit/deck/acts.ts`).
pub(super) const ASK_FOR_UPDATE: &str = "Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.";

/// The top pane's tab. Plan and the kinds after it are not in Electron's Default layout, so each
/// stays shut on a Job until + opens it.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Zone {
    #[default]
    Open,
    Done,
    Deferred,
    Plan,
    Changes,
    Pulls,
    Issues,
    Jobs,
    Ledger,
    Files,
    Subagents,
    Checks,
    System,
    Bays,
    Judge,
}

impl Zone {
    pub(super) const ALL: [Zone; 15] = [
        Zone::Open,
        Zone::Done,
        Zone::Deferred,
        Zone::Plan,
        Zone::Changes,
        Zone::Pulls,
        Zone::Issues,
        Zone::Jobs,
        Zone::Ledger,
        Zone::Files,
        Zone::Subagents,
        Zone::Checks,
        Zone::System,
        Zone::Bays,
        Zone::Judge,
    ];

    pub(super) fn id(self) -> &'static str {
        match self {
            Zone::Open => "open",
            Zone::Done => "done",
            Zone::Deferred => "deferred",
            Zone::Plan => "plan",
            Zone::Changes => "changes",
            Zone::Pulls => "pulls",
            Zone::Issues => "issues",
            Zone::Jobs => "jobs",
            Zone::Ledger => "ledger",
            Zone::Files => "files",
            Zone::Subagents => "subagents",
            Zone::Checks => "checks",
            Zone::System => "system",
            Zone::Bays => "bays",
            Zone::Judge => "judge",
        }
    }

    /// Shut until + opens it.
    pub(super) fn opened_by_hand(self) -> bool {
        !matches!(self, Zone::Open | Zone::Done | Zone::Deferred)
    }

    fn look(self) -> (&'static str, &'static str) {
        match self {
            Zone::Open => ("circle", "Open"),
            Zone::Done => ("circle-check", "Done"),
            Zone::Deferred => ("circle-dashed", "Deferred"),
            Zone::Plan => ("list-checks", "Plan"),
            Zone::Changes => ("diff", "Changes"),
            Zone::Pulls => ("git-pull-request", "Pull Requests"),
            Zone::Issues => ("ticket", "Issues"),
            Zone::Jobs => ("link", "Jobs"),
            Zone::Ledger => ("files", "Ledger"),
            Zone::Files => ("folder-tree", "Files"),
            Zone::Subagents => ("zap", "Subagents"),
            Zone::Checks => ("shield-check", "Checks"),
            Zone::System => ("cpu", "System"),
            Zone::Bays => ("layout-grid", "Bays"),
            Zone::Judge => ("scale", "Judge"),
        }
    }
}

/// The column's state on a Job's page: which groups are unfolded.
#[derive(Default)]
pub(super) struct SideState {
    unfolded: HashSet<String>,
    /// Deferred rows settled from here, kept away until Fleet's record says so or refuses.
    pub(super) settled: HashSet<String>,
}

/// Where a group of an approved plan stands.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stands {
    Coming,
    Working,
    Done,
}

pub(super) fn stands(group: &SessionPlanGroup) -> Stands {
    if group.tasks.iter().any(|task| task.state == PlanTaskState::Working) {
        Stands::Working
    } else if !group.tasks.is_empty() && group.tasks.iter().all(|task| matches!(task.state, PlanTaskState::Done | PlanTaskState::HandedIn)) {
        Stands::Done
    } else {
        Stands::Coming
    }
}

/// The column, drawn by the Job view so its presses act on the Job: the deck puts it on the right.
pub struct JobSide {
    view: Entity<JobView>,
    _watch: Subscription,
}

impl JobSide {
    pub fn new(view: Entity<JobView>, cx: &mut Context<Self>) -> Self {
        let _watch = cx.observe(&view, |_, _, cx| cx.notify());
        Self { view, _watch }
    }
}

impl Render for JobSide {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view.update(cx, |view, cx| view.render_side(window, cx))
    }
}

fn pane(look: &Look) -> gpui::Div {
    div()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(look.color(token::BG_RAISED))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
}

fn strip(look: &Look) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_stretch()
        .h(look.length(token::H_ROW_HEADER))
        .bg(look.color(token::BG_SUNKEN))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
}

fn tab(look: &Look, id: &'static str, icon: &'static str, said: &'static str, front: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .pl(look.length(token::SPACE_3))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
        .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
        .when(front, |tab| tab.bg(look.color(token::BG_RAISED)))
        .when(!front, |tab| tab.cursor_pointer().hover(|style| style.text_color(look.color(token::FG_DEFAULT))))
        .child(glyph(look, icon, Size::Small, look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED })))
        .child(said)
}

impl JobView {
    pub(super) fn render_side(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let look = &look;
        let zone = self.zone_shown();
        let tabs: Vec<_> = Zone::ALL.into_iter().filter(|one| !self.tab_closed(one.id())).map(|one| {
            let (icon, said) = one.look();
            let id = match one {
                Zone::Open => "side-open",
                Zone::Done => "side-done",
                Zone::Deferred => "side-deferred",
                Zone::Plan => "side-plan",
                Zone::Changes => "side-changes",
                Zone::Pulls => "side-pulls",
                Zone::Issues => "side-issues",
                Zone::Jobs => "side-jobs",
                Zone::Ledger => "side-ledger",
                Zone::Files => "side-files",
                Zone::Subagents => "side-subagents",
                Zone::Checks => "side-checks",
                Zone::System => "side-system",
                Zone::Bays => "side-bays",
                Zone::Judge => "side-judge",
            };
            let front = Some(one) == zone;
            let group = SharedString::from(id);
            labelled(
                tab(look, id, icon, said, front)
                    .group(group.clone())
                    .child(self.close_button(one.id(), said, group, front, look, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.zone = one;
                        this.want_pane_reads(window, cx);
                        cx.notify();
                    })),
                Role::Tab,
                format!("Job column: {said}"),
            )
        }).collect();
        // Changes and Files draw their own strip over a body that scrolls, as `.job-pane__pick` does.
        let own_frame = matches!(zone, Some(Zone::Changes | Zone::Files | Zone::Judge));
        let top_body = self.shown.clone().zip(zone).and_then(|(id, zone)| match zone {
            Zone::Open => Some(self.side_open(&id, look, cx)),
            Zone::Done => Some(self.side_done(&id, look, cx)),
            Zone::Deferred => Some(self.side_deferred(&id, look, cx)),
            Zone::Plan => self.pane_plan(&id, look, cx),
            Zone::Changes => self.pane_changes(&id, look, cx),
            Zone::Pulls => self.pane_pulls(&id, look, cx),
            Zone::Issues => self.pane_issues(&id, look, cx),
            Zone::Jobs => self.pane_jobs(&id, look, cx),
            Zone::Ledger => self.pane_ledger(&id, look, cx),
            Zone::Files => Some(self.pane_files(&id, look, cx)),
            Zone::Subagents => self.pane_subagents(&id, look, cx),
            Zone::Checks => Some(self.pane_checks(&id, look, cx)),
            Zone::System => self.pane_system(&id, look, cx),
            Zone::Bays => self.pane_bays(&id, look, cx),
            Zone::Judge => self.pane_judge(&id, look, cx),
        });
        let now_open = !self.tab_closed("now");
        let now_body = self.shown.clone().filter(|_| now_open).map(|id| self.side_now(&id, look, cx));
        let body = |id: &'static str, inner: Option<AnyElement>| {
            div()
                .id(id)
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_4))
                .children(inner)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::GAP_PANEL))
            .child(
                labelled(
                    pane(look)
                        .id("side-top")
                        .flex_grow(2.)
                        .flex_shrink(1.)
                        .flex_basis(gpui::px(0.))
                        .child(strip(look).child(div().id("side-top-tabs").flex().items_stretch().min_w_0().overflow_x_scroll().track_scroll(&self.panes.top_tabs).children(tabs)).child(self.add_button(Pane::Top, look, cx)))
                        .child(if own_frame {
                            div().id("side-top-body").flex_1().min_h_0().flex().flex_col().children(top_body).into_any_element()
                        } else {
                            body("side-top-body", top_body).into_any_element()
                        }),
                    Role::Region,
                    "Open, Done and Deferred",
                ),
            )
            .child(labelled(
                pane(look)
                    .id("side-now")
                    .flex_grow(3.)
                    .flex_shrink(1.)
                    .flex_basis(gpui::px(0.))
                    .child(
                        strip(look)
                            .when(now_open, |strip| {
                                strip.child(labelled(
                                    tab(look, "side-now-tab", "activity", "Now", true).group("side-now-tab").child(self.close_button("now", "Now", "side-now-tab".into(), true, look, cx)),
                                    Role::Tab,
                                    "Job column: Now",
                                ))
                            })
                            .child(self.add_button(Pane::Now, look, cx)),
                    )
                    .child(body("side-now-body", now_body)),
                Role::Region,
                "Job column: Now",
            ))
            .into_any_element()
    }

    pub(super) fn side_unfolded(&self, key: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id.as_str())).is_some_and(|page| page.side.unfolded.contains(key))
    }

    /// Unfold or fold a line of the column, by its key.
    pub(super) fn side_fold(&mut self, key: &str) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            if !page.side.unfolded.remove(key) {
                page.side.unfolded.insert(key.to_string());
            }
        }
    }
}
