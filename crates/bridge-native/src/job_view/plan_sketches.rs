//! The sketches a plan names (`SessionPlan::sketches`, `SessionPlanGroup::sketches`): each drawn inside the
//! plan as its card draws it (`sketch_card.rs`), the whole plan's under its summary and a group's under that
//! group. While the plan waits its Approve answers them too, so their own cards in the thread ask nothing (his
//! ruling of 13 Nov: one approval for one decision). Fleet already leaves them out of Needs you.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};
use ipc::{PlanSketch, SessionPlan, ThreadSketch};

use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

/// Every sketch the plan names, each once, with the group it is drawn under: none for the whole plan's.
/// A sketch named twice is drawn where it is named first, the whole plan before the groups.
fn placed(plan: &SessionPlan) -> Vec<(Option<&str>, &PlanSketch)> {
    let named = plan.sketches.iter().map(|one| (None, one)).chain(plan.groups.iter().flat_map(|group| group.sketches.iter().map(move |one| (Some(group.id.as_str()), one))));
    let mut out: Vec<(Option<&str>, &PlanSketch)> = Vec::new();
    for (at, one) in named {
        if !out.iter().any(|(_, seen)| seen.id == one.id) {
            out.push((at, one));
        }
    }
    out
}

/// The sketches drawn at `group`, or under the summary for `None`.
pub(super) fn drawn_at<'a>(plan: &'a SessionPlan, group: Option<&str>) -> Vec<&'a PlanSketch> {
    placed(plan).into_iter().filter(|(at, _)| *at == group).map(|(_, one)| one).collect()
}

/// Whether the sketch `id` (`sketch:<title>`) waits with the plan: the plan is neither approved nor set aside
/// and names it, so the plan's Approve answers it and its own card asks nothing.
pub(super) fn waits_with_plan(plan: Option<&SessionPlan>, id: &str) -> bool {
    plan.is_some_and(|plan| !plan.approved && plan.set_aside_at.is_none() && placed(plan).iter().any(|(_, one)| one.id == id))
}

impl JobView {
    /// Whether the sketch `id` of the Job shown waits with its plan.
    pub(super) fn sketch_waits_with_plan(&self, id: &str, cx: &gpui::App) -> bool {
        let plan = self.shown.as_ref().and_then(|job| self.record(job, cx)).and_then(|record| record.plan.as_ref());
        waits_with_plan(plan, id)
    }

    /// The plan's sketches drawn at `group` (`None` for the whole plan's), each as its card draws it at the
    /// column's width. Nothing where it names none, or where the thread holds no card for one.
    pub(super) fn plan_sketches(&self, key: &str, plan: &SessionPlan, group: Option<(&str, &str)>, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        // The Job whose page this is: the review sheet draws other Jobs' plans beside the one shown.
        let job = self.pages.iter().find(|(_, held)| std::ptr::eq(*held, page)).map(|(job, _)| job)?;
        let held = self.thread(job, cx)?;
        let cards: Vec<&ThreadSketch> = drawn_at(plan, group.map(|(id, _)| id)).into_iter().filter_map(|one| super::review::sketch_card(&held.head, &held.items, &one.id)).collect();
        if cards.is_empty() {
            return None;
        }
        let place = group.map_or("plan", |(id, _)| id);
        let name = group.map_or_else(|| "Sketches for the plan".to_string(), |(_, title)| format!("Sketches for {title}"));
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("plan-sketches:{key}:{place}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(cards.into_iter().map(|card| self.plan_sketch(&format!("{key}:{place}:{}", card.id), card, page, look, cx))),
                Role::Group,
                name,
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(approved: bool, set_aside: bool) -> SessionPlan {
        let wire = format!(
            r#"{{"version":2,"recorded_at":"2026-11-13T15:20:00Z","approved":{approved},{aside}"scope":[],
            "sketches":[{{"id":"sketch:Flow","title":"Flow"}}],
            "groups":[
              {{"id":"g1","agent":"Plan","title":"Propose","paths":[],"tasks":[]}},
              {{"id":"g2","agent":"gpui-engineer","title":"Rows","paths":[],"tasks":[],"sketches":[{{"id":"sketch:Rows","title":"Rows"}},{{"id":"sketch:Flow","title":"Flow"}}]}},
              {{"id":"g3","agent":"rust-engineer","title":"Serve","paths":[],"tasks":[],"sketches":[{{"id":"sketch:Rows","title":"Rows"}},{{"id":"sketch:Wire","title":"Wire"}}]}}
            ]}}"#,
            aside = if set_aside { r#""set_aside_at":"2026-11-13T15:25:00Z","# } else { "" },
        );
        ipc::decode::<SessionPlan>("plan", wire.as_bytes()).expect("a plan")
    }

    fn ids(sketches: Vec<&PlanSketch>) -> Vec<&str> {
        sketches.into_iter().map(|one| one.id.as_str()).collect()
    }

    #[test]
    fn a_plan_s_sketches_draw_under_its_summary_and_a_group_s_under_that_group() {
        let plan = plan(false, false);
        assert_eq!(ids(drawn_at(&plan, None)), ["sketch:Flow"]);
        assert_eq!(ids(drawn_at(&plan, Some("g1"))), Vec::<&str>::new());
        assert_eq!(ids(drawn_at(&plan, Some("g2"))), ["sketch:Rows"]);
        assert_eq!(ids(drawn_at(&plan, Some("g3"))), ["sketch:Wire"]);
    }

    #[test]
    fn a_sketch_named_twice_draws_once_where_it_is_named_first() {
        let plan = plan(false, false);
        let every: Vec<&str> = [None, Some("g1"), Some("g2"), Some("g3")].into_iter().flat_map(|at| ids(drawn_at(&plan, at))).collect();
        assert_eq!(every, ["sketch:Flow", "sketch:Rows", "sketch:Wire"]);
    }

    #[test]
    fn a_sketch_the_pending_plan_names_waits_with_the_plan() {
        let plan = plan(false, false);
        for id in ["sketch:Flow", "sketch:Rows", "sketch:Wire"] {
            assert!(waits_with_plan(Some(&plan), id), "{id}");
        }
        assert!(!waits_with_plan(Some(&plan), "sketch:Elsewhere"), "a sketch the plan does not name asks on its own");
        assert!(!waits_with_plan(None, "sketch:Flow"), "a Job with no plan holds no sketch");
    }

    #[test]
    fn an_approved_or_set_aside_plan_holds_no_sketch() {
        assert!(!waits_with_plan(Some(&plan(true, false)), "sketch:Flow"), "approved: its Approve answered them");
        assert!(!waits_with_plan(Some(&plan(false, true)), "sketch:Flow"), "set aside: each waits on him alone again");
    }
}
