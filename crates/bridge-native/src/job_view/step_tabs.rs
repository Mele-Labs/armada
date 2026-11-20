//! The tabs a workflow step lists are the column's own: Fleet's `core_model::STEP_TABS` holds every id of `side::Zone`,
//! and `now` beside them, which the zone list lacks. A zone added here without an id there is a tab
//! a workflow file could not name.
//!
//! **A step's tabs open when the Job's step moves** ([`JobView::follow_steps`]): each the Job holds in no pane goes in beside his
//! own, and the step's front tab comes to the front of its pane unless his pointer or his keys are in that pane. A step names
//! Child Jobs `jobs`.

use std::borrow::Cow;
use std::collections::HashSet;

use gpui::{Context, Focusable as _, SharedString, Window};

use super::pane_tree::{self as tree, Arrangement, Out};
use super::panes_place::{pane_for, put_behind};
use super::side::Zone;
use super::view_tabs::ViewPane;
use super::JobView;

/// The kind of tab a step's tab id opens here, or none where the column draws no such tab.
pub(super) fn kind_of(id: &str) -> Option<&'static str> {
    match id {
        "jobs" => Some("children"),
        "now" => Some("now"),
        id => Zone::ALL.into_iter().map(Zone::id).find(|zone| *zone == id),
    }
}

/// The kinds of tab a step opens, the front one first, each once.
pub(super) fn kinds_of(tabs: &[String]) -> Vec<&'static str> {
    let mut kinds: Vec<&'static str> = Vec::new();
    for kind in tabs.iter().filter_map(|id| kind_of(id)) {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

/// The panes with the step's tabs opened beside his own. A kind no pane holds goes behind the tab its pane shows, Now in the
/// column's foot and the rest in its top, as `open_kind` places them; one a pane holds stays where he keeps it. The front kind's tab
/// comes to the front of its pane, unless `held` says he has the pointer or the keys in that pane.
pub(super) fn with_step_tabs<'a>(was: &'a Arrangement, kinds: &[&str], held: &dyn Fn(&str) -> bool) -> Out<'a> {
    let mut next = Cow::Borrowed(was);
    for kind in kinds {
        if tree::has_kind(&next, kind) || super::tabs::laid_off(kind) {
            continue;
        }
        let place = if *kind == "now" { ViewPane::Bottom } else { ViewPane::Top };
        let target = pane_for(&next, place).map(|pane| (pane, None));
        next = Cow::Owned(put_behind(&next, kind, target));
    }
    let front = kinds.first().and_then(|kind| tree::holder_of(&next, kind).and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == *kind).map(|tab| (pane.id.clone(), tab.id.clone()))));
    if let Some((_, tab)) = front.filter(|(pane, _)| !held(pane)) {
        let shown = tree::activate(&next, &tab).into_owned();
        if shown != *next {
            next = Cow::Owned(shown);
        }
    }
    next
}

impl JobView {
    /// The panes of the Job where his pointer is, or where he has the keys: on one of its tabs, on its +, or in the message box of
    /// the thread's pane. The pointer counts for the Job on screen only, the one whose panes were drawn.
    fn panes_held(&self, job: &str, window: &Window, cx: &gpui::App) -> HashSet<String> {
        let Some(arrangement) = self.arrangement_of(job) else { return HashSet::new() };
        let pointer = self.shown.as_deref().is_some_and(|shown| shown == job).then(|| window.mouse_position());
        let drawn = self.arranged.drawn_at.borrow();
        let typing = self.composer.input.read(cx).focus_handle(cx).contains_focused(window, cx);
        let thread = tree::holder_of(arrangement, "thread").map(|pane| pane.id.clone());
        tree::panes_of(&arrangement.root)
            .into_iter()
            .filter(|pane| {
                let under = pointer.is_some_and(|at| ["strip", "body"].into_iter().any(|part| drawn.get(&format!("{part}:{}", pane.id)).is_some_and(|bounds| bounds.contains(&at))));
                let keys = pane.tabs.iter().map(|tab| format!("{job}/{}", tab.id)).chain([format!("{job}/add:{}", pane.id)]).filter_map(|key| self.arranged.handles.get(&key)).any(|handle| handle.contains_focused(window, cx));
                under || keys || (typing && thread.as_deref() == Some(pane.id.as_str()))
            })
            .map(|pane| pane.id.clone())
            .collect()
    }

    /// A Job's step moved since it was last seen here: its tabs open beside his own. A Job seen for the first time opens nothing, and
    /// a move that comes before the store has answered for the Job's panes waits for the next record rather than replace what he kept.
    pub(super) fn follow_steps(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<SharedString> = self.pages.keys().cloned().collect();
        for id in ids {
            let Some(job) = self.record(&id, cx) else { continue };
            let step = job.workflow_step;
            let tabs = job.workflow_strip.as_ref().map(|strip| strip.tabs.clone()).unwrap_or_default();
            let seen = self.pages.get(&id).and_then(|page| page.side.stepped);
            if seen == Some(step) {
                continue;
            }
            if seen.is_some() && step.is_some() && !tabs.is_empty() {
                if !self.panes_answered(&id) {
                    continue;
                }
                let kinds = kinds_of(&tabs);
                let held = self.panes_held(&id, window, cx);
                self.rearrange_in(&id, None, true, |arrangement| with_step_tabs(arrangement, &kinds, &|pane| held.contains(pane)), cx);
            }
            if let Some(page) = self.pages.get_mut(&id) {
                page.side.stepped = Some(step);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::column_strip::tests::drawn::{fresh, opened};
    use super::super::panes_place::front_of;
    use super::super::side::Zone;
    use super::*;

    #[test]
    fn every_zone_id_is_a_tab_a_workflow_step_may_list() {
        for zone in Zone::ALL {
            // A step names Child Jobs `jobs`, so the zone's own id is no word of a workflow file.
            let named = if zone == Zone::Children { "jobs" } else { zone.id() };
            assert!(core_model::is_step_tab(named), "`{}` is a zone a step cannot name: add it to core_model::STEP_TABS", zone.id());
        }
    }

    fn words(of: &[&str]) -> Vec<String> {
        of.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn a_step_names_child_jobs_jobs_and_every_id_it_may_list_opens_a_tab() {
        assert_eq!(kind_of("jobs"), Some("children"));
        assert_eq!(kind_of("now"), Some("now"));
        assert_eq!(kind_of("plan"), Some("plan"));
        assert_eq!(kind_of("walks"), Some("walks"));
        assert_eq!(kinds_of(&words(&["walks", "artifacts", "now", "artifacts"])), ["walks", "artifacts", "now"], "front first, each once");
        for id in core_model::STEP_TABS {
            assert!(kind_of(id).is_some(), "`{id}` is a tab a step lists and the column draws");
        }
    }

    fn kinds(arrangement: &Arrangement) -> Vec<String> {
        tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter().map(|tab| tab.kind.clone())).collect()
    }

    fn front(arrangement: &Arrangement, kind: &str) -> Option<String> {
        tree::holder_of(arrangement, kind).and_then(front_of).map(|tab| tab.kind.clone())
    }

    #[test]
    fn a_step_opens_the_tabs_no_pane_holds_beside_his_own_and_brings_the_front_one_forward() {
        let was = tree::default_arrangement();
        let tabs = kinds_of(&words(&["plan", "qa", "now"]));
        let next = with_step_tabs(&was, &tabs, &|_| false).into_owned();
        assert_eq!(kinds(&next), ["thread", "story", "now", "plan", "qa"], "behind the tab the column showed, Now not doubled");
        assert_eq!(front(&next, "plan").as_deref(), Some("plan"), "the front one comes to the front");
        // His own tab is kept where he put it, and a step that lists a tab he has open adds none.
        let own = tree::add_tab(&was, &tree::holder_of(&was, "now").expect("the column").id, "files", tree::Extras::default()).into_owned();
        let again = with_step_tabs(&own, &kinds_of(&words(&["pulls", "files", "now"])), &|_| false).into_owned();
        assert_eq!(kinds(&again).iter().filter(|kind| *kind == "files").count(), 1);
        assert!(kinds(&again).contains(&"pulls".to_string()));
        assert_eq!(with_step_tabs(&again, &kinds_of(&words(&["pulls", "files", "now"])), &|_| false).into_owned(), again, "a step already open changes nothing");
    }

    #[test]
    fn the_front_tab_stays_where_it_was_while_his_pointer_or_keys_are_in_that_pane() {
        let was = tree::default_arrangement();
        let tabs = kinds_of(&words(&["plan", "now"]));
        let next = with_step_tabs(&was, &tabs, &|_| true).into_owned();
        assert!(kinds(&next).contains(&"plan".to_string()), "the tab still opens");
        assert_eq!(front(&next, "plan").as_deref(), Some("now"), "what he was looking at stays in front");
    }

    #[test]
    fn an_orchestrating_jobs_step_that_opens_jobs_puts_child_jobs_in_front() {
        let was = tree::default_arrangement();
        let next = with_step_tabs(&was, &kinds_of(&words(&["jobs", "pulls", "now"])), &|_| false).into_owned();
        assert_eq!(front(&next, "children").as_deref(), Some("children"));
    }

    fn move_step(cx: &mut gpui::VisualTestContext, view: &gpui::Entity<JobView>, to: u32, tabs: &[&str]) {
        let list = view.read_with(cx, |view, _| view.list.clone());
        let tabs = words(tabs);
        list.update(cx, |list, cx| {
            let job = list.record_mut("mock-16").expect("the Job");
            job.workflow_step = Some(to);
            job.workflow_strip = Some(ipc::WorkflowStrip { tabs, ..ipc::WorkflowStrip::default() });
            cx.notify();
        });
        cx.run_until_parked();
        fresh(cx);
    }

    #[gpui::test]
    fn a_job_that_moves_to_the_next_step_opens_its_tabs_beside_the_ones_he_opened(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, "mock-16");
        cx.run_until_parked();
        view.update_in(cx, |view, _, cx| view.open_kind("files", cx));
        fresh(cx);
        move_step(cx, &view, 2, &["pulls", "plan", "now"]);
        view.read_with(cx, |view, _| {
            let arrangement = view.arrangement();
            let held = kinds(&arrangement);
            for kind in ["files", "now", "pulls", "plan"] {
                assert!(held.contains(&kind.to_string()), "{kind} is open: {held:?}");
            }
            assert_eq!(front(&arrangement, "pulls").as_deref(), Some("pulls"), "the step's front tab is in front");
        });
    }

    #[gpui::test]
    fn the_front_tab_does_not_move_under_his_pointer(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, "mock-16");
        cx.run_until_parked();
        let at = view.read_with(cx, |view, _| {
            let pane = pane_for(&view.arrangement(), ViewPane::Top).expect("the column");
            view.arranged.drawn_at.borrow().get(&format!("body:{pane}")).map(|bounds| bounds.center()).expect("the column was drawn")
        });
        cx.simulate_mouse_move(at, None, gpui::Modifiers::none());
        fresh(cx);
        move_step(cx, &view, 2, &["pulls", "plan", "now"]);
        view.read_with(cx, |view, _| {
            let arrangement = view.arrangement();
            assert!(kinds(&arrangement).contains(&"pulls".to_string()), "the tabs open");
            assert_eq!(front(&arrangement, "pulls").as_deref(), Some("now"), "what he had in front stays");
        });
    }

    #[gpui::test]
    fn a_job_seen_for_the_first_time_opens_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx, "mock-16");
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert_eq!(kinds(&view.arrangement()), ["thread", "story", "now"], "the step it came on screen at opened nothing"));
    }
}
