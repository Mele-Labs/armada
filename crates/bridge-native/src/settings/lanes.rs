//! A workflow's lanes on Settings › Workflows (`WorkspaceEditor`): the workflow as it runs at the level the picked row
//! sets, read from Fleet (`get_workflow_lanes`), under the line that says what an edit there changes. **A repository's
//! own copy is that repository's level; Armada's and Yours are the default's**, as Classic has had it since the tabs
//! went. What differs below a level and where Workspaces tie are Fleet's marks, which the canvas draws.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{div, prelude::*, px, AnyElement, App, Context, KeyDownEvent, Role, SharedString};
use ipc::JobLanesView;

use super::overlay::SettingsOverlay;
use super::view::{bone, small};
use super::workflows::{Listed, Place};
use crate::labels::labelled;
use crate::lanes_canvas::{lanes_canvas, LanesHost, LanesState};
use crate::look::{token, Look};

/// The default's level.
const DEFAULT: &str = "default";


pub struct WorkflowLanes {
    read: BTreeMap<SharedString, Result<JobLanesView, String>>,
    asking: BTreeSet<SharedString>,
    pub canvas: LanesState,
}

impl WorkflowLanes {
    pub fn new(cx: &mut App) -> Self {
        Self { read: BTreeMap::new(), asking: BTreeSet::new(), canvas: LanesState::new(cx) }
    }

    /// Settings opened again: read afresh.
    pub fn reopen(&mut self) {
        self.read.clear();
    }
}

/// The level the picked row is read at, and the repository it is read for.
pub fn scope_of(one: &Listed) -> (String, String) {
    let scope = if one.place == Place::Repository { one.manifest.clone() } else { DEFAULT.to_string() };
    (scope, one.runs_in.clone().unwrap_or_else(|| one.manifest.clone()))
}

/// What an edit on the picked copy changes (`SAYS`).
pub fn says(scope: &str, repository: &str) -> String {
    if scope == DEFAULT { "Edits here change every Workspace and repository that keeps the default".into() } else { format!("Edits here change {repository} only") }
}

/// One read for each workflow, level and repository, which the canvas keeps its view under too.
fn key_of(one: &Listed) -> SharedString {
    let (scope, manifest) = scope_of(one);
    format!("{}|{scope}|{manifest}", one.id).into()
}

impl SettingsOverlay {
    /// Ask Fleet for the picked workflow's lanes, once for each level it is picked at.
    pub(super) fn read_lanes(&mut self, cx: &mut Context<Self>) {
        let Some(one) = self.shown_workflows(cx).1 else { return };
        let key = key_of(&one);
        if self.lanes.read.contains_key(&key) || self.lanes.asking.contains(&key) {
            return;
        }
        self.lanes.asking.insert(key.clone());
        let (scope, manifest) = scope_of(&one);
        let sent = self.wire.workflow_lanes(&one.id, &scope, &manifest, cx);
        cx.spawn(async move |this, cx| {
            let answer = sent.await;
            let _ = this.update(cx, |this, cx| {
                this.lanes.asking.remove(&key);
                this.lanes.read.insert(key, answer.map_err(|why| why.said));
                cx.notify();
            });
        })
        .detach();
    }

    /// The line saying what an edit changes, then the canvas: placeholder shapes while it is read, and what Fleet
    /// said where it refused the level.
    pub(super) fn render_lanes(&self, one: &Listed, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = key_of(one);
        let (scope, manifest) = scope_of(one);
        let repository = self.wire.repositories(cx).unwrap_or_default().into_iter().find(|(id, _, _)| *id == scope).map_or(manifest, |(_, name, _)| name);
        let said = says(&scope, &repository);
        let line = labelled(small(look, said.clone(), look.color(token::FG_MUTED)).id("workflow-lanes-says").px(look.length(token::SPACE_3)), Role::Label, said);
        let body = match self.lanes.read.get(&key) {
            None => bone(look, px(0.), look.length(token::SPACE_12) * 11.).w_full().into_any_element(),
            Some(Err(said)) => labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("workflow-lanes-refused"), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(lanes)) => {
                // Four lanes of the least height fill it; a lane more makes it taller by that much.
                let tall = (look.length(token::SPACE_12) * 11.).max(px(lanes.lanes.len() as f32 * crate::lanes_canvas::LANE_MIN) + look.length(token::BORDER_WIDTH) * 2.);
                // An Esc the lanes did not take, with nothing picked, shuts Settings as it does on every page.
                let shut = cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.close(window, cx);
                        cx.stop_propagation();
                    }
                });
                div().flex_none().h(tall).min_w_0().flex().flex_col().on_key_down(shut).child(lanes_canvas(self, &key, lanes, look, cx)).into_any_element()
            }
        };
        labelled(div().id("workflow-lanes").flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().child(line).child(body), Role::Region, "Workflow").into_any_element()
    }
}

impl LanesHost for SettingsOverlay {
    fn lanes_state(&self) -> &LanesState {
        &self.lanes.canvas
    }

    fn lanes_of<'a>(&'a self, id: &SharedString, _cx: &'a App) -> Option<&'a JobLanesView> {
        self.lanes.read.get(id).and_then(|read| read.as_ref().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(place: Place, manifest: &str, runs_in: Option<&str>) -> Listed {
        Listed { id: "feature".into(), place, manifest: manifest.into(), runs_in: runs_in.map(String::from), for_requests: None }
    }

    #[test]
    fn a_repository_own_copy_is_read_at_its_level_and_every_other_row_at_the_default() {
        assert_eq!(scope_of(&row(Place::Repository, "api", None)), ("api".to_string(), "api".to_string()));
        assert_eq!(scope_of(&row(Place::Armada, "armada", Some("api"))), ("default".to_string(), "api".to_string()));
        assert_eq!(scope_of(&row(Place::Yours, "armada", None)), ("default".to_string(), "armada".to_string()));
        assert_eq!(says("api", "api"), "Edits here change api only");
        assert_eq!(says("default", "api"), "Edits here change every Workspace and repository that keeps the default");
    }
}
