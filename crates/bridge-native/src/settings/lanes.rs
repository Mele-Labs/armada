//! A workflow's lanes on Settings › Workflows (`WorkspaceEditor`): the workflow as it runs at the level the picked row
//! sets, read from Fleet (`get_workflow_lanes`), under the line that says what an edit there changes. **A repository's
//! own copy is that repository's level; Armada's and Yours are the default's**, as Classic has had it since the tabs
//! went. What differs below a level and where Workspaces tie are Fleet's marks, which the canvas draws.
//!
//! The canvas, the draft laid over it and Publish are the `lanes_editor`'s: one for each row, kept here while Settings
//! is open, and this page answers its Fleet reads and saves through its own wire.

use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, App, Context, Entity, FocusHandle, Role, Subscription, Task, WeakEntity, Window};
use ipc::SaveWorkflow;

use super::overlay::SettingsOverlay;
use super::view::{bone, ghost_with, ringed, small};
use super::workflow_detail::wire_of;
use super::workflows::{Listed, Place};
use crate::labels::labelled;
use crate::lanes_editor::{EditorEvent, EditorWire, LanesEditor, Refused, Subject, Target};
use crate::look::{token, Look};

/// The default's level.
const DEFAULT: &str = "default";

pub struct WorkflowLanes {
    /// The editor of each row, by its key: the draft stays with the row while Settings is open.
    editors: BTreeMap<String, Entity<LanesEditor>>,
    _subs: BTreeMap<String, Subscription>,
    /// The stop of "Add lifecycle", on the line that says what an edit changes.
    pub add: FocusHandle,
}

impl WorkflowLanes {
    pub fn new(cx: &mut App) -> Self {
        Self { editors: BTreeMap::new(), _subs: BTreeMap::new(), add: cx.focus_handle().tab_stop(true) }
    }

    /// Settings opened again: every row read afresh, except one that holds a draft.
    pub fn reopen(&mut self, cx: &App) {
        self.editors.retain(|_, editor| editor.read(cx).has_edits());
        let kept: Vec<String> = self.editors.keys().cloned().collect();
        self._subs.retain(|key, _| kept.contains(key));
    }
}

/// The level the picked row is read at, and the repository it is read for.
pub fn scope_of(one: &Listed) -> (String, String) {
    let scope = if one.place == Place::Repository { one.manifest.clone() } else { DEFAULT.to_string() };
    (scope, one.runs_in.clone().unwrap_or_else(|| one.manifest.clone()))
}

/// What an edit on the picked copy changes (`SAYS`). None for the default: the draft's question answers where it
/// applies.
pub fn says(scope: &str, repository: &str) -> Option<String> {
    (scope != DEFAULT).then(|| format!("Edits here change {repository} only"))
}

/// Armada's own workflow, and what runs in a repository that keeps no copy: an edit on it asks where it applies.
pub fn shipped(one: &Listed) -> bool {
    one.place == Place::Armada || one.runs_in.is_some()
}

/// The workflow a row draws, as the editor is told it, with the project it is offered: a project's own copy is
/// that project's, any other row is offered the project picked.
fn subject_of(one: &Listed, project: Option<(String, String)>) -> Subject {
    let (level, manifest) = scope_of(one);
    Subject { workflow: one.id.clone(), level, repository: manifest.clone(), manifest, held_under: one.manifest.clone(), source: wire_of(one.place), shipped: shipped(one), project }
}

/// The editor's reads and saves, answered by Settings' own wire.
struct SettingsWire(WeakEntity<SettingsOverlay>);

impl SettingsWire {
    fn with<T: 'static>(&self, cx: &mut App, make: impl FnOnce(&mut SettingsOverlay, &mut Context<SettingsOverlay>) -> Task<Result<T, Refused>>) -> Task<Result<T, Refused>> {
        self.0.update(cx, make).unwrap_or_else(|_| Task::ready(Err(Refused::said("Settings closed"))))
    }
}

impl EditorWire for SettingsWire {
    fn lanes(&self, at: &Subject, cx: &mut App) -> Task<Result<ipc::JobLanesView, Refused>> {
        let at = at.clone();
        self.with(cx, move |this, cx| this.wire.workflow_lanes(&at.workflow, &at.level, &at.manifest, cx))
    }

    fn definition(&self, at: &Subject, cx: &mut App) -> Task<Result<ipc::WorkflowDefinition, Refused>> {
        let at = at.clone();
        self.with(cx, move |this, cx| this.wire.workflow_definition(&at.workflow, at.source, &at.held_under, cx))
    }

    fn save_workflow(&self, manifest: &str, save: SaveWorkflow, cx: &mut App) -> Task<Result<ipc::WorkflowSaved, Refused>> {
        let manifest = manifest.to_string();
        self.with(cx, move |this, cx| this.wire.save_workflow(&manifest, save, cx))
    }

    fn lifecycles(&self, manifest: &str, cx: &mut App) -> Task<Result<ipc::RepositoryLifecycles, Refused>> {
        let manifest = manifest.to_string();
        self.with(cx, move |this, cx| this.wire.lifecycles(&manifest, cx))
    }

    fn save_piece(&self, save: ipc::SaveLifecyclePiece, cx: &mut App) -> Task<Result<ipc::LifecyclePieceSaved, Refused>> {
        self.with(cx, move |this, cx| this.wire.save_lifecycle_piece(save, cx))
    }

    fn delete_piece(&self, delete: ipc::DeleteLifecyclePiece, cx: &mut App) -> Task<Result<ipc::LifecyclePieceDeleted, Refused>> {
        self.with(cx, move |this, cx| this.wire.delete_lifecycle_piece(delete, cx))
    }
}

impl SettingsOverlay {
    /// The picked row's editor, made once for each row.
    pub(super) fn read_lanes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.shown_workflows(cx).1 else { return };
        let key = one.key();
        let project = self.offered_project(&one);
        if let Some(editor) = self.lanes.editors.get(&key) {
            editor.update(cx, |editor, cx| editor.offer_project(project, cx));
            return;
        }
        let wire: Rc<dyn EditorWire> = Rc::new(SettingsWire(cx.entity().downgrade()));
        let subject = subject_of(&one, project);
        let editor = cx.new(|cx| LanesEditor::new(subject, wire, window, cx));
        editor.update(cx, |editor, _| editor.scrolls_in(self.workflows.detail.scroll.clone()));
        let row = one.clone();
        let sub = cx.subscribe_in(&editor, window, move |this, _, event: &EditorEvent, window, cx| this.editor_said(&row, event, window, cx));
        self.lanes.editors.insert(key.clone(), editor);
        self.lanes._subs.insert(key, sub);
    }

    /// The project a row's editor is offered: the row's own where it is a project's copy, else the one picked.
    fn offered_project(&self, one: &Listed) -> Option<(String, String)> {
        match (one.place, &one.project) {
            (Place::Project, Some(id)) => Some((id.clone(), self.project_name(id))),
            _ => self.scope.project.clone(),
        }
    }

    /// What an editor told the page: shut Settings on an Esc it did not take, and after a Publish read what was
    /// written again, picking the copy it made.
    fn editor_said(&mut self, one: &Listed, event: &EditorEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            EditorEvent::Escaped => self.close(window, cx),
            EditorEvent::Published { target } => {
                let copy = shipped(one).then(|| match target {
                    Target::Repository(manifest) => Listed { id: one.id.clone(), place: Place::Repository, manifest: manifest.clone(), runs_in: None, for_requests: one.for_requests.clone(), project: None },
                    Target::Project(id) => Listed { id: one.id.clone(), place: Place::Project, manifest: one.manifest.clone(), runs_in: None, for_requests: one.for_requests.clone(), project: Some(id.clone()) },
                    Target::EveryProject => Listed { id: one.id.clone(), place: Place::Yours, manifest: one.manifest.clone(), runs_in: None, for_requests: one.for_requests.clone(), project: None },
                });
                // The rows that read the same workflow read it again, and the page its steps and its list.
                let key = one.key();
                for (_, editor) in self.lanes.editors.iter().filter(|(row, _)| **row != key) {
                    editor.update(cx, |editor, cx| editor.reload(cx));
                }
                // The bar the keys were on is gone: they go to the canvas where the row stays, else to the pages.
                match (&copy, self.lanes.editors.get(&key)) {
                    (None, Some(editor)) => editor.update(cx, |editor, cx| editor.focus_canvas(window, cx)),
                    _ => window.focus(&self.side, cx),
                }
                self.read_steps(one.clone(), window, cx);
                self.read_list(copy.map(|copy| copy.key()), None, window, cx);
            }
        }
    }

    /// The picked row's editor, which an agent's change lands in.
    pub(super) fn editor_of(&self, one: &Listed) -> Option<Entity<LanesEditor>> {
        self.lanes.editors.get(&one.key()).cloned()
    }

    /// The editor's stops: its bar and history where the draft has changes, then the canvas.
    pub(super) fn lanes_stops(&self, one: &Listed, cx: &App) -> Vec<FocusHandle> {
        self.lanes.editors.get(&one.key()).map(|editor| editor.read(cx).stops()).unwrap_or_default()
    }

    /// Whether the canvas of the picked row holds the keys.
    pub(super) fn canvas_keyed(&self, window: &Window, cx: &App) -> bool {
        self.lanes.editors.values().any(|editor| editor.read(cx).canvas_focused(window))
    }

    /// Whether the keys are in a field the canvas opened in place.
    pub(super) fn lanes_typing(&self, window: &Window, cx: &App) -> bool {
        self.lanes.editors.values().any(|editor| editor.read(cx).typing(window, cx))
    }

    /// The keys the editor takes ahead of the page's: ⌘Z, Enter and Space on its bar, Esc on its history; Tab, Enter
    /// and Esc in the fields it opened on the canvas. Never from another text field.
    pub(super) fn lanes_key(&mut self, key: &gpui::Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let (_, Some(one)) = self.shown_workflows(cx) else { return false };
        let Some(editor) = self.editor_of(&one) else { return false };
        if editor.read(cx).typing(window, cx) {
            return editor.update(cx, |editor, cx| editor.field_key(key, window, cx));
        }
        if self.in_text(window, cx) {
            return false;
        }
        editor.update(cx, |editor, cx| editor.key(key, window, cx))
    }

    /// "Add lifecycle" saves a New lifecycle piece and picks it; built with the lifecycle inspector.
    pub(super) fn add_lifecycle(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// The line saying what an edit changes and the button that adds a lifecycle, then what stands over the canvas
    /// (the box that asks an agent), then the editor: its draft bar over the canvas, placeholder shapes while the
    /// lanes are read, and what Fleet said where it refused the level.
    pub(super) fn render_lanes(&self, one: &Listed, above: Option<AnyElement>, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (scope, manifest) = scope_of(one);
        let repository = self.wire.repositories(cx).unwrap_or_default().into_iter().find(|(id, _, _)| *id == scope).map_or(manifest, |(_, name, _)| name);
        let said = match (one.place, &one.project) {
            (Place::Project, Some(id)) => Some(format!("Edits here change {} only", self.project_name(id))),
            _ => says(&scope, &repository),
        };
        let add = ringed(look, ghost_with(look, "workflow-add-lifecycle", Some("plus"), "Add lifecycle", false), &self.lanes.add, window)
            .track_focus(&self.lanes.add)
            .on_click(cx.listener(|this, _, window, cx| this.add_lifecycle(window, cx)));
        let line = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3))
            .gap_y(look.length(token::SPACE_1))
            .min_w_0()
            .child(match said {
                Some(said) => labelled(small(look, said.clone(), look.color(token::FG_MUTED)).id("workflow-lanes-says").flex_auto().min_w_0().px(look.length(token::SPACE_3)), Role::Label, said).into_any_element(),
                None => div().flex_auto().into_any_element(),
            })
            .child(labelled(add, Role::Button, "Add lifecycle"));
        let body = match self.lanes.editors.get(&one.key()) {
            None => bone(look, px(0.), look.length(token::SPACE_12) * 11.).w_full().into_any_element(),
            Some(editor) => editor.clone().into_any_element(),
        };
        labelled(div().id("workflow-lanes").flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().child(line).children(above).child(body), Role::Region, "Workflow").into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(place: Place, manifest: &str, runs_in: Option<&str>) -> Listed {
        Listed { id: "feature".into(), place, manifest: manifest.into(), runs_in: runs_in.map(String::from), for_requests: None, project: None }
    }

    #[test]
    fn a_repository_own_copy_is_read_at_its_level_and_every_other_row_at_the_default() {
        assert_eq!(scope_of(&row(Place::Repository, "api", None)), ("api".to_string(), "api".to_string()));
        assert_eq!(scope_of(&row(Place::Armada, "armada", Some("api"))), ("default".to_string(), "api".to_string()));
        assert_eq!(scope_of(&row(Place::Yours, "armada", None)), ("default".to_string(), "armada".to_string()));
        assert_eq!(says("api", "api").as_deref(), Some("Edits here change api only"));
        assert_eq!(says("default", "api"), None);
    }

    #[test]
    fn an_edit_asks_where_it_applies_on_armadas_own_and_on_what_runs_in_a_repository_that_keeps_no_copy() {
        assert!(shipped(&row(Place::Armada, "armada", None)));
        assert!(shipped(&row(Place::Yours, "armada", Some("api"))));
        assert!(!shipped(&row(Place::Yours, "armada", None)));
        assert!(!shipped(&row(Place::Repository, "api", None)));
        let own = subject_of(&row(Place::Repository, "api", None), None);
        assert_eq!(own.own_target(), Some(Target::Repository("api".into())));
        assert_eq!(subject_of(&row(Place::Yours, "armada", None), None).own_target(), Some(Target::EveryProject));
        let project = Some(("armada".to_string(), "Armada".to_string()));
        assert_eq!(subject_of(&Listed { project: Some("armada".into()), ..row(Place::Project, "armada", None) }, project).own_target(), Some(Target::Project("armada".into())));
        let kept = subject_of(&row(Place::Armada, "armada", Some("api")), None);
        assert_eq!((kept.own_target(), kept.repository.as_str()), (None, "api"));
    }
}
