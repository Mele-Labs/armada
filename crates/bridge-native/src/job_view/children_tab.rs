//! Child Jobs, for a Job that orchestrates: each Job it started as a row with its title, its steps as bars and what it
//! waits on. A press opens the child. A Job that started none draws nothing.

use chrono::Utc;
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::SessionRecord;

use super::column_strip::{pips_bar, strip_of};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a child waits on, in the words Fleet gave: what waits on the person first, else what its step waits for.
pub(super) fn waits_of(on_him: &[String], step: &[String]) -> Option<String> {
    on_him.first().or_else(|| step.first()).cloned()
}

impl JobView {
    /// The Jobs this one started, in the order it started them.
    pub(super) fn children_of<'a>(&self, id: &str, cx: &'a gpui::App) -> Vec<&'a SessionRecord> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let all = self.list.read(cx).records();
        job.attachments.iter().filter(|one| one.kind == "started_to").filter_map(|one| all.iter().find(|other| other.id.as_str() == one.target)).collect()
    }

    pub(super) fn pane_children(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let kids = self.children_of(id, cx);
        if kids.is_empty() {
            return None;
        }
        let rows: Vec<AnyElement> = kids.into_iter().map(|kid| self.child_row(kid, look, cx)).collect();
        Some(labelled(div().id(SharedString::from(format!("pane-children:{id}"))).flex().flex_col().children(rows), Role::List, "Child Jobs").into_any_element())
    }

    fn child_row(&self, kid: &SessionRecord, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = kid.id.as_str().to_string();
        let title = crate::jobs::titled(kid.title.as_deref());
        let work = self.work(kid, cx);
        let strip = work.name.as_deref().filter(|_| !work.off).map(|name| strip_of(name, &work.steps, work.at, &work.visits, &work.skipped, &[], Utc::now()).read_by(kid.workflow_strip.as_ref()));
        // A child whose step stands past its usual time with no move turns amber, with Fleet's cause to read.
        let stalled = kid.workflow_strip.as_ref().and_then(|wire| wire.stalled.as_ref()).map(|stalled| stalled.cause.clone());
        let on_him: Vec<String> = kid.waiting_for.iter().map(|item| item.text.clone()).collect();
        let waits = waits_of(&on_him, &kid.workflow_waiting_on);
        let step = strip.as_ref().and_then(|strip| strip.pips.iter().find(|pip| pip.state.is_now())).map(|pip| pip.name.clone());
        let name = [Some(title.clone()), step.clone(), stalled.clone(), waits.clone()].into_iter().flatten().collect::<Vec<_>>().join(", ");
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let open = key.clone();
        labelled(
            div()
                .id(SharedString::from(format!("child-job:{key}")))
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_ROW))
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tab_index(0)
                .when(stalled.is_some(), |row| row.mb(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(amber).bg(look.color(token::BG_RAISED).blend(amber.opacity(0.10))))
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .child(div().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title)))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_color(look.color(token::FG_MUTED))
                        .children(strip.as_ref().map(|strip| {
                            let states: Vec<_> = strip.pips.iter().map(|pip| pip.state).collect();
                            let said = step.clone().unwrap_or_else(|| strip.flow.clone());
                            let said = match &stalled {
                                Some(cause) => format!("{said}: {cause}"),
                                None => said,
                            };
                            pips_bar(look, SharedString::from(format!("child-pips:{key}")), &states, said)
                        }))
                        .children(waits.map(|waits| div().max_w(look.length(token::SPACE_12) * 3.).truncate().child(SharedString::from(waits)))),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.open_job(open.clone(), cx))),
            Role::Button,
            name,
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_child_waits_on_what_asks_him_before_what_its_step_waits_for() {
        let words = |of: &[&str]| of.iter().map(|word| word.to_string()).collect::<Vec<_>>();
        assert_eq!(waits_of(&words(&["Approve the walk"]), &words(&["Waiting on PR #3951: Merged"])).as_deref(), Some("Approve the walk"));
        assert_eq!(waits_of(&[], &words(&["Waiting on PR #3951: Merged"])).as_deref(), Some("Waiting on PR #3951: Merged"));
        assert_eq!(waits_of(&[], &[]), None, "an empty slot stays empty");
    }

    /// The Job that started three, one of which has stood past its usual time: its row says why, as the child's own words.
    #[gpui::test]
    fn a_child_whose_step_is_not_moving_names_the_cause_and_the_others_do_not(cx: &mut gpui::TestAppContext) {
        use super::super::column_strip::tests::drawn::{fresh, opened};
        use super::super::parts_fold::drawn_like;
        let (view, cx) = opened(cx, "mock-19");
        view.update_in(cx, |view, _, cx| view.open_kind("children", cx));
        fresh(cx);
        let cause = "The step waits on the checks of its pull request, and Fleet has seen none run.";
        let rows = drawn_like(cx, gpui::Role::Button, "Deliver the child tab");
        assert_eq!(rows.len(), 1, "the child is a row of the tab");
        assert!(rows[0].contains(cause), "the row reads Fleet's cause: {}", rows[0]);
        let others = drawn_like(cx, gpui::Role::Button, "Draw the child rows");
        assert!(others.iter().all(|row| !row.contains("Fleet has seen")), "a child inside its time says nothing of it");
    }
}
