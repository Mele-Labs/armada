//! Open, alone in its pane, as Armada Classic's `StoryColumn` draws it bare (`next`): the plan's
//! groups still to come, shut and read as titles until pressed; what has not landed, an issue still
//! open or a pull request not merged that no group still to finish works; and what Fleet is to do
//! for the Job. The route still to come stands under Now, as Ahead.
//!
//! With a workflow's steps or a gated plan's stretches as the zones, Open is those steps still to
//! come and the Open group after them (`workflow.rs`, `spine.rs`).

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};
use ipc::{FleetActState, SessionPlanGroup};

use super::now::{beat, lead, named, words};
use super::rows::tip;
use super::side::Stands;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

impl JobView {
    pub(super) fn side_open(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        self.open_body(id, false, look, cx).unwrap_or_else(|| div().into_any_element())
    }

    /// Open's rows, absent with none. On the Spine the route still to come stands here, after what
    /// has not landed (`next`).
    pub(super) fn open_body(&self, id: &SharedString, spine: bool, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.record(id, cx).cloned()?;
        // A plan drafted or marked stands whole in Now to edit; its groups are not drawn again here.
        let editing = job.plan.as_ref().is_some_and(|plan| !plan.approved || !self.plan_marks(id).is_empty());
        let groups: Vec<&SessionPlanGroup> = job.plan.iter().flat_map(|plan| plan.groups.iter()).collect();
        let place = super::side::placed(&job);
        let upcoming: Vec<&SessionPlanGroup> = if editing { Vec::new() } else { groups.iter().copied().filter(|group| place(group) == Stands::Coming).collect() };
        let held: Vec<&SessionPlanGroup> = groups.iter().copied().filter(|group| editing || place(group) != Stands::Done).collect();

        let ahead = if spine { self.ahead_list(&job, look, cx) } else { None };
        let rows: Vec<AnyElement> = self
            .flight_rows(&job, &upcoming, true, look, cx)
            .into_iter()
            .chain(self.not_landed(&job, &held, look, cx))
            .chain(ahead)
            .chain(self.fleet_coming(id, look, cx))
            .collect();
        (!rows.is_empty()).then(|| div().flex().flex_col().children(rows).into_any_element())
    }

    /// What Fleet is to do for the Job, as another worker's beats are (`comingList`).
    fn fleet_coming(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let coming: Vec<AnyElement> = self
            .thread(id, cx)
            .iter()
            .flat_map(|held| held.head.fleet_acts.iter())
            .filter(|act| act.state == FleetActState::Coming)
            .map(|act| {
                let key = format!("fleet:{}", act.id);
                let said = words(look, act.said.clone(), token::FG_MUTED, false, false, false).id(SharedString::from(format!("open-fleet-said:{}", act.id)));
                let said = match act.why.clone() {
                    Some(why) => said.tooltip(tip(why)).into_any_element(),
                    None => said.into_any_element(),
                };
                labelled(named(&key, beat(look, lead(look, &key, "server", token::FG_SUBTLE, "Fleet", false), said, None, None)), Role::Group, act.said.clone()).into_any_element()
            })
            .collect();
        (!coming.is_empty()).then(|| labelled(div().id("open-fleet").flex().flex_col().pt(look.length(token::SPACE_2)).children(coming), Role::List, "Fleet").into_any_element())
    }

    /// The work still open once a workflow's steps or a gated plan's stretches are the zones: what has
    /// not landed, then what Fleet is to do (`openMade`, `comingList`). The Open group draws it after
    /// the last step; hidden, the current step does.
    pub(super) fn open_work(&self, id: &SharedString, job: &ipc::SessionRecord, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let place = super::side::placed(job);
        let held: Vec<&SessionPlanGroup> = job.plan.iter().flat_map(|plan| plan.groups.iter()).filter(|group| place(group) != Stands::Done).collect();
        let rows: Vec<AnyElement> = self.not_landed(job, &held, look, cx).into_iter().chain(self.fleet_coming(id, look, cx)).collect();
        (!rows.is_empty()).then(|| div().flex().flex_col().children(rows).into_any_element())
    }
}
