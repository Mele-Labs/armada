//! What the right column's tabs are: Open, Done and Deferred, and the kinds + opens after them (Plan,
//! Changes, Ledger, Files, Subagents, Checks in `panes.rs`, Spine in `spine.rs`, Judge in `judge_tab.rs`),
//! with the column's state on a Job's page and the look of a strip's tab. Any pane may hold any of them
//! (`place_draw.rs`); Now is `now.rs`.

use std::collections::HashSet;

use gpui::{div, prelude::*, FontWeight, SharedString};
use ipc::{PlanTaskState, SessionPlanGroup};

use super::rows::{glyph, Size};
use super::JobView;
use crate::look::{token, Look};

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
    Workflow,
    System,
    Bays,
    Spine,
    Judge,
}

impl Zone {
    pub(super) const ALL: [Zone; 17] = [
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
        Zone::Workflow,
        Zone::System,
        Zone::Bays,
        Zone::Spine,
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
            Zone::Workflow => "workflow",
            Zone::System => "system",
            Zone::Bays => "bays",
            Zone::Spine => "spine",
            Zone::Judge => "judge",
        }
    }

    pub(super) fn look(self) -> (&'static str, &'static str) {
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
            Zone::Workflow => ("workflow", "Workflow"),
            Zone::System => ("cpu", "System"),
            Zone::Bays => ("layout-grid", "Bays"),
            Zone::Spine => ("waypoints", "Spine"),
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
    /// Watching draws the watches that ended too.
    pub(super) watches_ended: bool,
    /// Watches stopped or deleted from here, drawn so until Fleet's record says so or refuses.
    pub(super) watch_acted: std::collections::HashMap<String, super::watching::Acted>,
    /// The Spine's Deferred frame is open to its list.
    pub(super) later_open: bool,
    /// The Spine's Brief is open to the whole brief and how the Job started.
    pub(super) brief_open: bool,
    /// The workflow he picked from the head, with the one the record named then: it stands until
    /// the record names another, or Fleet refuses it.
    pub(super) flow_pick: Option<(Option<String>, Option<String>)>,
    /// He changed the workflow here, so the proposer's mark goes (`unpick`).
    pub(super) unpicked: bool,
    /// Steps skipped or unskipped here, each with whether the record had it skipped then.
    pub(super) skips: std::collections::HashMap<u32, (bool, bool)>,
    /// A step he jumped to, with the step the record was at then.
    pub(super) jumped: Option<(Option<u32>, u32)>,
    /// The stretches of a gated plan whose tasks are open, by stretch and state.
    pub(super) chunks: HashSet<String>,
    /// The Open group after the last step is folded.
    pub(super) open_group_shut: bool,
}

/// Where a group of an approved plan stands.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stands {
    Coming,
    Working,
    Done,
}

/// Where each group of an approved plan stands: where its stretch does, for a plan that ends on
/// gates, else by its tasks.
pub(super) fn placed(job: &ipc::SessionRecord) -> impl Fn(&SessionPlanGroup) -> Stands {
    let gates = job.plan.as_ref().filter(|plan| plan.approved && super::workflow::gated(&plan.groups)).map(|plan| super::workflow::gate_places(&plan.groups, plan.held.as_ref()));
    move |group| gates.as_ref().and_then(|gates| gates.get(&group.id).copied()).unwrap_or_else(|| stands(group))
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

pub(super) fn pane(look: &Look) -> gpui::Div {
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

pub(super) fn strip(look: &Look) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_stretch()
        .h(look.length(token::H_ROW_HEADER))
        .bg(look.color(token::BG_SUNKEN))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
}

pub(super) fn tab(look: &Look, id: impl Into<gpui::ElementId>, icon: &'static str, said: impl Into<SharedString>, front: bool) -> gpui::Stateful<gpui::Div> {
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
        .child(said.into())
}

impl JobView {
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
