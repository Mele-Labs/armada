//! Settings › This Mac › Judges: the judge types a project sees, and the ones he keeps in Kit. The list stands on the left, the
//! picked type on the right: an Armada type read only with the two ways to copy it into his Kit, one of his own to edit,
//! reset or delete. A type is saved in Kit on this Mac, for all projects or for one; the repository's copy is a file in its own
//! `.armada/judges/`, and committing one there is a Job Fleet starts, which this page asks for.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{canvas, div, prelude::*, svg, AnyElement, App, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, Keystroke, Pixels, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use ipc::{DeleteJudgeType, JudgeType, ReviewToolScope, ReviewToolSource, SaveJudgeType};

use super::overlay::{Picking, SettingsOverlay};
use super::view::{bone, mono, ringed, small, tip};
use super::wire::Refused;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What the right side shows: one of the types listed, or the form of a new one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Type(String),
    New,
}

/// What a Save of the page's fields means, worked out once so the press and ⌘S agree.
struct Writing {
    name: String,
    prompt: String,
    project: Option<String>,
    /// The Kit copy the fields were filled from, where the save replaces it.
    from: Option<JudgeType>,
}

pub struct JudgesPage {
    /// The project the list is read for, by id and name. It opens on the one Settings edits, else the first listed.
    pub project: Option<(String, String)>,
    /// What the last read of the project's types came to. `None` while it is read.
    read: Option<Result<Vec<JudgeType>, String>>,
    /// The project the list was read for, so a frame does not read it again.
    read_for: Option<String>,
    pub pick: Option<Pick>,
    /// Applies to, on a type of his own: all projects, else the project picked.
    pub all_projects: bool,
    /// The pick and project the fields were last filled for.
    filled: Option<String>,
    pub name: Entity<InputState>,
    pub prompt: Entity<TextareaState>,
    pub saving: bool,
    pub refused: Option<String>,
    stops: BTreeMap<String, FocusHandle>,
    /// Where the project picker was drawn, for the list that opens under it.
    picker_at: Rc<Cell<Bounds<Pixels>>>,
    _read: Option<Task<()>>,
    _write: Option<Task<()>>,
    _typing: Subscription,
}

impl JudgesPage {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsOverlay>) -> JudgesPage {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let prompt = cx.new(|cx| TextareaState::new(window, cx).auto_grow(6, 24));
        // What is typed in the prompt is kept under the type it is for until it is saved or reset to what was saved.
        let typing = cx.subscribe_in(&prompt, window, |this, state, event: &InputEvent, _, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let Some(key) = this.judges.filled.as_ref().map(|filled| format!("judges:prompt:{filled}")) else { return };
            let text = state.read(cx).value().to_string();
            if text.is_empty() || this.saved_prompt().is_some_and(|saved| saved == text) {
                crate::drafts::clear(&key, cx);
            } else {
                crate::drafts::keep(&key, &text, cx);
            }
            cx.notify();
        });
        JudgesPage {
            project: None,
            read: None,
            read_for: None,
            pick: None,
            all_projects: true,
            filled: None,
            name,
            prompt,
            saving: false,
            refused: None,
            stops: BTreeMap::new(),
            picker_at: Rc::default(),
            _read: None,
            _write: None,
            _typing: typing,
        }
    }

    /// Read again the next time the page is drawn, as when it was left and entered again.
    pub fn forget(&mut self) {
        self.read_for = None;
    }
}

/// A type's id from the name it is saved under, as Fleet makes it: lower case, the words joined by a dash.
fn slug(name: &str) -> String {
    name.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-")
}

/// The headings the list is grouped under, each with the types of its place, in Fleet's order.
fn grouped<'a>(types: &'a [JudgeType], project: &str) -> Vec<(String, Vec<&'a JudgeType>)> {
    let place = |source: ReviewToolSource| types.iter().filter(|one| one.source == source).collect::<Vec<_>>();
    [("Armada's".to_string(), place(ReviewToolSource::Armada)), ("Yours".to_string(), place(ReviewToolSource::Kit)), (format!("{project}'s repository"), place(ReviewToolSource::Repository))]
        .into_iter()
        .filter(|(_, rows)| !rows.is_empty())
        .collect()
}

impl SettingsOverlay {
    /// The prompt of the type picked as last saved.
    fn saved_prompt(&self) -> Option<String> {
        let Some(Ok(types)) = &self.judges.read else { return None };
        match &self.judges.pick {
            Some(Pick::Type(id)) => types.iter().find(|one| one.id == *id).map(|one| one.prompt.clone()),
            _ => Some(String::new()),
        }
    }

    /// The type picked: the one chosen, else the first listed. `None` on a new one.
    fn judge_picked(&self) -> Option<&JudgeType> {
        let Some(Ok(types)) = &self.judges.read else { return None };
        match &self.judges.pick {
            Some(Pick::New) => None,
            Some(Pick::Type(id)) => types.iter().find(|one| one.id == *id).or_else(|| types.first()),
            None => types.first(),
        }
    }

    fn judge_is_new(&self) -> bool {
        self.judges.pick == Some(Pick::New)
    }

    /// The repositories the project holds, by Manifest id.
    fn judge_manifests(&self) -> Vec<String> {
        let Some((id, _)) = &self.judges.project else { return Vec::new() };
        let Some(project) = self.scope.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == *id)) else { return Vec::new() };
        let mut manifests: Vec<String> = Vec::new();
        for member in &project.members {
            let manifest = member.manifest_id.as_str().to_string();
            if !manifests.contains(&manifest) {
                manifests.push(manifest);
            }
        }
        manifests
    }

    /// Read the types the project sees, unless they are read for it already: one call for each repository it holds, the first
    /// definition of an id kept. The project is the one Settings edits until another is picked here.
    pub(super) fn read_judges(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(all) = self.scope.projects.as_ref() else { return };
        if self.judges.project.as_ref().is_none_or(|(id, _)| !all.projects.iter().any(|one| one.id == *id)) {
            let first = self.scope.project.clone().or_else(|| all.projects.first().map(|one| (one.id.clone(), one.name.clone())));
            self.judges.project = first;
        }
        let Some((id, _)) = self.judges.project.clone() else { return };
        if self.judges.read_for.as_ref() == Some(&id) {
            return;
        }
        self.judges.read_for = Some(id.clone());
        self.judges.read = None;
        let manifests = self.judge_manifests();
        let reads: Vec<_> = manifests.iter().map(|manifest| self.wire.judge_types(manifest, Some(&id), cx)).collect();
        self.judges._read = Some(cx.spawn_in(window, async move |this, cx| {
            let mut types: Vec<JudgeType> = Vec::new();
            let mut refused: Option<String> = None;
            let mut read_any = reads.is_empty();
            for read in reads {
                match read.await {
                    Ok(found) => {
                        read_any = true;
                        for one in found {
                            if !types.iter().any(|held| held.id == one.id) {
                                types.push(one);
                            }
                        }
                    }
                    Err(why) => refused = refused.or(Some(why.said)),
                }
            }
            let _ = this.update_in(cx, |this, window, cx| {
                // An answer for a project no longer picked is let go.
                if this.judges.read_for.as_ref() != Some(&id) {
                    return;
                }
                this.judges.read = Some(match refused {
                    Some(why) if !read_any => Err(why),
                    _ => Ok(types),
                });
                this.judges.filled = None;
                this.judges_fill(window, cx);
                cx.notify();
            });
        }));
    }

    /// Put the type picked into the fields: its name and prompt, or what was kept of a draft of the prompt.
    pub(super) fn judges_fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Ok(_)) = &self.judges.read else { return };
        let Some((project, _)) = self.judges.project.clone() else { return };
        let new = self.judge_is_new();
        let which = self.judge_picked().map(|one| one.id.clone());
        let want = format!("{project}:{}", if new { "new".to_string() } else { which.clone().unwrap_or_default() });
        if self.judges.filled.as_ref() == Some(&want) {
            return;
        }
        let (name, prompt, all) = match self.judge_picked() {
            Some(one) => (one.name.clone(), one.prompt.clone(), one.project.is_none()),
            None => (String::new(), String::new(), true),
        };
        let prompt = crate::drafts::kept(&format!("judges:prompt:{want}"), cx).unwrap_or(prompt);
        self.judges.name.update(cx, |field, cx| field.set_value(name, window, cx));
        self.judges.prompt.update(cx, |field, cx| field.set_value(prompt, window, cx));
        self.judges.all_projects = all;
        self.judges.refused = None;
        self.judges.filled = Some(want);
    }

    /// A key for each control the page draws, made when first wanted.
    pub(super) fn judges_keys(&mut self, cx: &mut Context<Self>) {
        for name in ["new", "picker", "applies:all", "applies:one", "save", "reset", "commit", "edit:one", "edit:all"] {
            self.judges.stops.entry(name.to_string()).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
        if let Some(Ok(types)) = &self.judges.read {
            let ids: Vec<String> = types.iter().map(|one| format!("row:{}", one.id)).collect();
            for id in ids {
                self.judges.stops.entry(id).or_insert_with(|| cx.focus_handle().tab_stop(true));
            }
        }
    }

    fn judge_stop(&self, name: &str) -> Option<FocusHandle> {
        self.judges.stops.get(name).cloned()
    }

    /// The keys in the order Tab reaches them: New judge, the project, the row picked, then the detail's controls.
    pub(super) fn judges_stops(&self, cx: &App) -> Vec<FocusHandle> {
        let mut stops: Vec<FocusHandle> = ["new", "picker"].iter().filter_map(|name| self.judge_stop(name)).collect();
        let new = self.judge_is_new();
        if let Some(row) = self.judge_picked().and_then(|one| self.judge_stop(&format!("row:{}", one.id))) {
            stops.push(row);
        }
        let own = new || self.judge_picked().is_some_and(|one| one.source == ReviewToolSource::Kit);
        if own {
            stops.extend(["applies:all", "applies:one"].iter().filter_map(|name| self.judge_stop(name)));
            stops.push(self.judges.name.read(cx).focus_handle(cx));
            stops.push(Focusable::focus_handle(self.judges.prompt.read(cx), cx));
            stops.extend(["save"].iter().filter_map(|name| self.judge_stop(name)));
            if !new {
                stops.extend(["reset", "commit"].iter().filter_map(|name| self.judge_stop(name)));
            }
        } else if self.judge_picked().is_some() {
            stops.extend(["edit:one", "edit:all"].iter().filter_map(|name| self.judge_stop(name)));
        }
        stops
    }

    // ---- acts --------------------------------------------------------------------------------

    fn judge_pick(&mut self, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        self.judges.pick = Some(pick.clone());
        self.judges.refused = None;
        self.judges_fill(window, cx);
        if let Pick::Type(id) = pick {
            if let Some(row) = self.judge_stop(&format!("row:{id}")) {
                window.focus(&row, cx);
            }
        }
        cx.notify();
    }

    fn judge_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.judges.pick = Some(Pick::New);
        self.judges.filled = None;
        self.judges_fill(window, cx);
        let name = self.judges.name.read(cx).focus_handle(cx);
        window.focus(&name, cx);
        cx.notify();
    }

    fn judge_project_list(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(all) = &self.scope.projects else { return };
        let now = self.judges.project.as_ref().map(|(id, _)| id.clone());
        let options: Vec<(SharedString, bool)> = all.projects.iter().map(|one| (SharedString::from(one.name.clone()), Some(&one.id) == now.as_ref())).collect();
        let bounds = self.judges.picker_at.get();
        self.open_choice(Picking::JudgeProject, "Project".into(), options, bounds.bottom_left(), from_keys, window, cx);
    }

    /// A project chosen in the picker's list, by its place there.
    pub(super) fn chose_judge_project(&mut self, n: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.scope.projects.as_ref().and_then(|all| all.projects.get(n)) else { return };
        let to = (one.id.clone(), one.name.clone());
        if self.judges.project.as_ref() == Some(&to) {
            return;
        }
        self.judges.project = Some(to);
        self.judges.read_for = None;
        self.judges.read = None;
        self.judges.pick = None;
        self.judges.filled = None;
        cx.notify();
    }

    /// What a Save would write, or why it cannot.
    fn judge_writing(&self, cx: &App) -> Result<Writing, String> {
        let from = self.judge_picked().filter(|one| one.source == ReviewToolSource::Kit).cloned();
        let name = if self.judge_is_new() { self.judges.name.read(cx).value().trim().to_string() } else { from.as_ref().map(|one| one.name.clone()).unwrap_or_default() };
        if slug(&name).is_empty() {
            return Err("A judge needs a name".to_string());
        }
        let prompt = self.judges.prompt.read(cx).value().trim().to_string();
        if prompt.is_empty() {
            return Err("A judge needs something to look for".to_string());
        }
        let project = if self.judges.all_projects { None } else { self.judges.project.as_ref().map(|(id, _)| id.clone()) };
        Ok(Writing { name, prompt, project, from })
    }

    /// Save the fields in Kit, for all projects or the project. A type moved between the two leaves its old place.
    pub(super) fn judge_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.judges.saving || !self.live {
            return;
        }
        let write = match self.judge_writing(cx) {
            Ok(write) => write,
            Err(why) => {
                self.judges.refused = Some(why);
                cx.notify();
                return;
            }
        };
        let Some(manifest) = self.judge_manifests().into_iter().next() else { return };
        let id = slug(&write.name);
        let overwrite = write.from.as_ref().is_some_and(|from| from.project == write.project);
        let moved = write.from.as_ref().filter(|from| from.project != write.project).map(|from| from.project.clone());
        let saved = self.wire.save_judge_type(&manifest, SaveJudgeType { scope: ReviewToolScope::Kit, project: write.project.clone(), name: write.name, prompt: write.prompt, overwrite }, cx);
        let gone = moved.map(|project| self.wire.delete_judge_type(&manifest, DeleteJudgeType { scope: ReviewToolScope::Kit, project, id: id.clone() }, cx));
        self.judge_write(saved, gone, Some(id), window, cx);
    }

    /// Copy the picked type into Kit under its own name, for all projects or for the project, and edit the copy.
    fn judge_copy(&mut self, all: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.judges.saving || !self.live {
            return;
        }
        let (Some(one), Some((project, _)), Some(manifest)) = (self.judge_picked().cloned(), self.judges.project.clone(), self.judge_manifests().into_iter().next()) else { return };
        let save = SaveJudgeType { scope: ReviewToolScope::Kit, project: (!all).then_some(project), name: one.name.clone(), prompt: one.prompt.clone(), overwrite: false };
        let saved = self.wire.save_judge_type(&manifest, save, cx);
        self.judge_write(saved, None, Some(one.id), window, cx);
    }

    /// Delete Kit's copy of the picked type: where it replaced one of Armada's, Armada's shows again.
    fn judge_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.judges.saving || !self.live {
            return;
        }
        let (Some(one), Some(manifest)) = (self.judge_picked().filter(|one| one.source == ReviewToolSource::Kit).cloned(), self.judge_manifests().into_iter().next()) else { return };
        let gone = self.wire.delete_judge_type(&manifest, DeleteJudgeType { scope: ReviewToolScope::Kit, project: one.project.clone(), id: one.id.clone() }, cx);
        let draft = format!("judges:prompt:{}:{}", self.judges.project.as_ref().map_or("", |(id, _)| id.as_str()), one.id);
        crate::drafts::clear(&draft, cx);
        self.judge_write(gone, None, None, window, cx);
    }

    /// Wait for a save or a delete, then read the list again and pick what was written.
    fn judge_write(&mut self, first: Task<Result<(), Refused>>, then: Option<Task<Result<(), Refused>>>, pick: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.judges.saving = true;
        self.judges.refused = None;
        self.judges._write = Some(cx.spawn_in(window, async move |this, cx| {
            let mut answer = first.await;
            if answer.is_ok() {
                if let Some(then) = then {
                    answer = then.await;
                }
            }
            let _ = this.update_in(cx, |this, window, cx| {
                this.judges.saving = false;
                match answer {
                    Err(why) => this.judges.refused = Some(why.said),
                    Ok(()) => {
                        if let Some(draft) = this.judges.filled.as_ref().map(|filled| format!("judges:prompt:{filled}")) {
                            crate::drafts::clear(&draft, cx);
                        }
                        this.judges.pick = pick.map(Pick::Type);
                        this.judges.read_for = None;
                        this.judges.filled = None;
                        this.read_judges(window, cx);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Commit the picked type to the project's repository: Fleet starts a Job that adds the file and opens a pull request.
    fn judge_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.judges.saving || !self.live {
            return;
        }
        let Some(one) = self.judge_picked().filter(|one| one.source == ReviewToolSource::Kit).cloned() else { return };
        let Some(root) = self.judge_manifests().into_iter().next().and_then(|manifest| self.wire.repository_root(&manifest, cx)) else {
            self.judges.refused = Some("The repository is not one Fleet serves".to_string());
            return;
        };
        let project = self.judges.project.as_ref().map_or_else(String::new, |(_, name)| name.clone());
        let task = self.wire.commit_judge_type(&root, &one.id, one.project.as_deref(), cx);
        self.judges.saving = true;
        self.judges.refused = None;
        self.judges._write = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.judges.saving = false;
                match answer {
                    Err(why) => this.judges.refused = Some(why.said),
                    Ok(_) => {
                        let said = format!("Committing {} to {project}'s repository in a Job", one.name);
                        cx.defer(move |cx| crate::toasts::tell(said, cx));
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// A control pressed, by its key's name.
    fn judges_press(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "new" => self.judge_new(window, cx),
            "picker" => self.judge_project_list(true, window, cx),
            "applies:all" => self.judges.all_projects = true,
            "applies:one" => self.judges.all_projects = false,
            "save" => self.judge_save(window, cx),
            "reset" => self.judge_reset(window, cx),
            "commit" => self.judge_commit(window, cx),
            "edit:one" => self.judge_copy(false, window, cx),
            "edit:all" => self.judge_copy(true, window, cx),
            _ => {}
        }
        cx.notify();
    }

    /// The page's own keys: ⌘S saves, ↑ and ↓ move between rows, Enter and Space press the control that has the keys.
    pub(super) fn judges_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let held = &key.modifiers;
        if held.platform && !held.control && !held.alt && !held.shift && key.key == "s" {
            if self.judge_is_new() || self.judge_picked().is_some_and(|one| one.source == ReviewToolSource::Kit) {
                self.judge_save(window, cx);
                return true;
            }
            return false;
        }
        if held.platform || held.control || held.alt {
            return false;
        }
        let word = key.key.as_str();
        let Some(Ok(types)) = &self.judges.read else { return false };
        let listed: Vec<(String, FocusHandle)> = grouped(types, "").into_iter().flat_map(|(_, rows)| rows).filter_map(|one| Some((one.id.clone(), self.judge_stop(&format!("row:{}", one.id))?))).collect();
        if let Some(at) = listed.iter().position(|(_, handle)| handle.is_focused(window)) {
            if matches!(word, "up" | "down") {
                let to = if word == "up" { at.saturating_sub(1) } else { (at + 1).min(listed.len() - 1) };
                if to != at {
                    crate::ui_sounds::moved();
                }
                let id = listed[to].0.clone();
                self.judge_pick(Pick::Type(id), window, cx);
                return true;
            }
            return false;
        }
        if matches!(word, "enter" | "space") {
            let pressed = self.judges.stops.iter().find(|(name, handle)| !name.starts_with("row:") && handle.is_focused(window)).map(|(name, _)| name.clone());
            if let Some(name) = pressed {
                self.judges_press(&name, window, cx);
                return true;
            }
        }
        false
    }

    // ---- drawn -------------------------------------------------------------------------------

    /// The place chip of a type: where it is kept, in the hue of the place.
    fn judge_place(&self, one: &JudgeType, look: &Look) -> (String, Hsla) {
        match (one.source, &one.project) {
            (ReviewToolSource::Armada, _) => ("Armada".to_string(), look.color(token::FG_MUTED)),
            (ReviewToolSource::Kit, None) => ("All projects".to_string(), look.color(token::STATUS_RUNNING)),
            (ReviewToolSource::Kit, Some(project)) => (self.project_name(project), look.color(token::STATUS_PILOTED)),
            (ReviewToolSource::Repository, _) => ("Committed".to_string(), look.color(token::STATUS_COMPLETED_SUCCESS)),
        }
    }

    pub(super) fn render_judges(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let list = super::models::panel(look, "judges-list").flex_none().w(look.length(token::W_GUIDE_LIST)).p(look.length(token::SPACE_3)).gap(look.length(token::SPACE_4)).child(self.judges_list_head(look, window, cx)).child(self.judges_rows(look, window, cx));
        let detail = super::models::panel(look, "judges-detail").flex_1().child(self.judges_detail(look, window, cx));
        div()
            .id("settings-page")
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            .child(labelled(list, Role::Region, "Judges"))
            .child(labelled(detail, Role::Region, "Judge"))
            .into_any_element()
    }

    /// The title with New judge, and under it the project the list is read for.
    fn judges_list_head(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let off = !self.live;
        let new_stop = self.judge_stop("new");
        let new = div()
            .id("judges-new")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.45))
            .bg(hue.opacity(0.14))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(hue)
            .when(off, |press| press.opacity(0.5))
            .when(!off, |press| press.cursor_pointer().hover(|style| style.bg(hue.opacity(0.22))).on_click(cx.listener(|this, _, window, cx| this.judge_new(window, cx))))
            .child(svg().flex_none().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(hue))
            .child("New judge");
        let new = match &new_stop {
            Some(stop) => ringed(look, new.track_focus(stop), stop, window),
            None => new,
        };
        let title = div().id("judges-title").min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Judges");
        let name = self.judges.project.as_ref().map(|(_, name)| name.clone());
        let at = self.judges.picker_at.clone();
        let picker = name.map(|name| {
            let said = format!("For {name}");
            let picker = div()
                .id("judges-project")
                .relative()
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .when(!off, |press| press.cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.judge_project_list(false, window, cx))))
                .child(div().min_w_0().truncate().child(SharedString::from(said.clone())))
                .child(svg().flex_none().path(icon_path("chevron-down")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                .child(canvas(move |bounds, _, _| at.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full());
            let picker = match self.judge_stop("picker") {
                Some(stop) => ringed(look, picker.track_focus(&stop), &stop, window),
                None => picker,
            };
            labelled(picker, Role::Button, said)
        });
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().flex().items_center().justify_between().gap(look.length(token::SPACE_2)).child(labelled(title, Role::Heading, "Judges")).child(labelled(new, Role::Button, "New judge")))
            .children(picker)
            .into_any_element()
    }

    /// The rows, grouped under the places they are kept, or bones while the list is read.
    fn judges_rows(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let list = div().id("judges-groups").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(look.length(token::SPACE_4));
        let Some(read) = &self.judges.read else {
            let bones = [0, 1, 2, 3].map(|n| {
                div()
                    .id(SharedString::from(format!("judges-bone-{n}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .min_h(look.length(token::H_CONTROL_LG))
                    .px(look.length(token::SPACE_3))
                    .child(bone(look, look.length(token::SPACE_8), look.length(token::SPACE_8)))
                    .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(bone(look, gpui::px(96. + n as f32 * 12.), look.length(token::LEADING_XS))).child(bone(look, gpui::px(160.), look.length(token::LEADING_XS))))
            });
            return labelled(list.children(bones), Role::Image, "Loading").into_any_element();
        };
        let types = match read {
            Ok(types) => types,
            Err(why) => return labelled(list.child(super::view::refusal(look, format!("The judges could not be read. {why}")).id("judges-unread")), Role::Alert, why.clone()).into_any_element(),
        };
        let project = self.judges.project.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
        let picked = self.judge_picked().map(|one| one.id.clone());
        let groups = grouped(types, &project).into_iter().map(|(heading, rows)| {
            let caption = div()
                .flex()
                .items_center()
                .min_h(look.length(token::H_CONTROL_SM))
                .pl(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(heading.clone()));
            let rows = rows.into_iter().map(|one| self.judge_row(one, picked.as_ref() == Some(&one.id) && !self.judge_is_new(), look, window, cx));
            labelled(div().id(SharedString::from(format!("judges-group-{heading}"))).flex().flex_col().gap(look.length(token::SPACE_1)).child(caption).children(rows), Role::Group, heading)
        });
        list.children(groups).into_any_element()
    }

    fn judge_row(&self, one: &JudgeType, current: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let (place, ink) = self.judge_place(one, look);
        let id = one.id.clone();
        let tile = div().flex_none().size(look.length(token::SPACE_8)).flex().items_center().justify_center().rounded(look.length(token::RADIUS_MD)).bg(hue.opacity(0.28)).child(svg().path(icon_path("scale")).size(look.length(token::SPACE_4)).text_color(hue));
        let says = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.name.clone())))
            .child(div().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(crate::judge_types::first_sentence(&one.prompt))));
        let row = div()
            .id(SharedString::from(format!("judge-{}", one.id)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .cursor_pointer()
            .map(|row| if current { row.bg(look.color(token::SURFACE_ROW_SELECTED)) } else { row.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.1))) })
            .on_click(cx.listener(move |this, _, window, cx| this.judge_pick(Pick::Type(id.clone()), window, cx)))
            .child(tile)
            .child(says)
            .child(chip(look, &place, ink));
        let row = match self.judge_stop(&format!("row:{}", one.id)) {
            Some(stop) => ringed(look, row.track_focus(&stop), &stop, window),
            None => row,
        };
        labelled(row, Role::Button, format!("{}, {place}", one.name)).into_any_element()
    }

    /// The right side: the header band, then what the type is and the acts on it.
    fn judges_detail(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let Some(Ok(_)) = &self.judges.read else { return div().into_any_element() };
        let new = self.judge_is_new();
        let picked = self.judge_picked();
        if !new && picked.is_none() {
            return div().into_any_element();
        }
        let project = self.judges.project.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
        let (title, place) = match picked {
            Some(one) => (one.name.clone(), Some(self.judge_place(one, look))),
            None => ("New judge".to_string(), None),
        };
        let band = div()
            .id("judges-band")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(hue.opacity(0.14))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.3))
            .child(div().flex_none().size(look.length(token::SPACE_8)).flex().items_center().justify_center().rounded(look.length(token::RADIUS_MD)).bg(hue.opacity(0.28)).child(svg().path(icon_path("scale")).size(look.length(token::SPACE_4)).text_color(hue)))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child(SharedString::from(title.clone())))
            .children(place.map(|(said, ink)| chip(look, &said, ink)));
        let own = new || picked.is_some_and(|one| one.source == ReviewToolSource::Kit);
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_4)).p(look.length(token::SPACE_4));
        if own {
            let kept = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(small(look, "Kept in your Kit, on this Mac. Only you see it.", look.color(token::FG_MUTED))).children(
                picked.filter(|one| one.replaces == Some(ReviewToolSource::Armada)).map(|one| small(look, format!("In {project} it is used in place of Armada's {}.", one.name), look.color(token::FG_MUTED))),
            );
            body = body.child(self.judges_applies(&project, look, window, cx)).child(kept).child(self.judges_name(new, look)).child(self.judges_prompt(look));
        } else if let Some(one) = picked {
            body = body.child(self.judges_read_only(one, look));
        }
        let foot = if own { Some(self.judges_foot(new, picked, &project, look, window, cx)) } else { picked.map(|_| self.judges_copies(look, window, cx)) };
        let refused = self.judges.refused.clone().map(|why| labelled(super::view::refusal(look, why.clone()).id("judges-refused"), Role::Alert, why));
        div().id("judges-detail-body").flex().flex_col().child(band).child(body.children(refused).children(foot)).into_any_element()
    }

    /// Applies to: all projects, or the project alone.
    fn judges_applies(&self, project: &str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let segment = |name: &'static str, said: String, on: bool| {
            let seg = div()
                .id(SharedString::from(format!("judges-{name}")))
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { hue } else { look.color(token::BORDER_DEFAULT) })
                .text_size(look.length(token::TEXT_SM))
                .when(on, |seg| seg.bg(hue.opacity(0.14)).text_color(hue).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                .when(!on, |seg| seg.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER))))
                .cursor_pointer()
                .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                .on_click(cx.listener(move |this, _, window, cx| this.judges_press(name, window, cx)))
                .child(SharedString::from(said.clone()));
            let seg = match self.judge_stop(name) {
                Some(stop) => ringed(look, seg.track_focus(&stop), &stop, window),
                None => seg,
            };
            labelled(seg, Role::RadioButton, said).into_any_element()
        };
        let all = self.judges.all_projects;
        let row = div().id("judges-applies").flex().gap(look.length(token::SPACE_2)).child(segment("applies:all", "All projects".to_string(), all)).child(segment("applies:one", format!("{project} only"), !all));
        field(look, "Applies to", labelled(row, Role::RadioGroup, "Applies to").into_any_element())
    }

    fn judges_name(&self, new: bool, look: &Look) -> AnyElement {
        let input = div().id("judges-name").key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&self.judges.name).disabled(!new));
        field(look, "Name", labelled(input, Role::TextInput, "Name").into_any_element())
    }

    fn judges_prompt(&self, look: &Look) -> AnyElement {
        let boxed = div()
            .id("judges-prompt")
            .key_context(crate::keymap::TEXT_BOX)
            .w_full()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(Textarea::new(&self.judges.prompt).appearance(false).aria_label("What it looks for"));
        field(look, "What it looks for", labelled(boxed, Role::TextInput, "What it looks for").into_any_element())
    }

    /// An Armada or repository type's prompt, read only.
    fn judges_read_only(&self, one: &JudgeType, look: &Look) -> AnyElement {
        let said = match one.source {
            ReviewToolSource::Repository => "Committed to the repository. Edit for the project or for all projects to keep a copy in your Kit.",
            _ => "Carried by Armada. Edit for the project or for all projects to keep a copy in your Kit.",
        };
        let prompt = div()
            .id("judges-prompt-read")
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_SUNKEN))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(one.prompt.clone()));
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .child(small(look, said, look.color(token::FG_MUTED)))
            .child(field(look, "What it looks for", labelled(prompt, Role::Label, one.prompt.clone()).into_any_element()))
            .into_any_element()
    }

    /// Edit for this project, Edit for all projects: each saves a copy in Kit.
    fn judges_copies(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let off = !self.live || self.judges.saving;
        let button = |name: &'static str, said: &'static str| {
            let press = super::workflow_detail::sunken_button(look, SharedString::from(format!("judges-{name}")), said, off);
            let press = match self.judge_stop(name) {
                Some(stop) => ringed(look, press.track_focus(&stop), &stop, window),
                None => press,
            };
            labelled(press.when(!off, |press| press.on_click(cx.listener(move |this, _, window, cx| this.judges_press(name, window, cx)))), Role::Button, said).into_any_element()
        };
        div().flex().gap(look.length(token::SPACE_2)).child(button("edit:one", "Edit for this project")).child(button("edit:all", "Edit for all projects")).into_any_element()
    }

    /// Save, Reset to Armada's or Delete, and Commit to the project, with the line that says what they do.
    fn judges_foot(&self, new: bool, picked: Option<&JudgeType>, project: &str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let off = !self.live || self.judges.saving;
        let replaces = picked.and_then(|one| one.replaces) == Some(ReviewToolSource::Armada);
        let save = {
            let said = if self.judges.saving { "Saving\u{2026}" } else { "Save" };
            let press = div()
                .id("judges-save")
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::ACCENT))
                .bg(look.color(token::ACCENT))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .when(off, |press| press.opacity(0.5))
                .when(!off, |press| press.cursor_pointer().hover(|style| style.bg(look.color(token::ACCENT_HOVER))).on_click(cx.listener(|this, _, window, cx| this.judge_save(window, cx))))
                .tooltip(tip("\u{2318}S"))
                .child(said);
            let press = match self.judge_stop("save") {
                Some(stop) => ringed(look, press.track_focus(&stop), &stop, window),
                None => press,
            };
            labelled(press, Role::Button, "Save").into_any_element()
        };
        let ghost = |name: &'static str, glyph: &'static str, said: String| {
            let press = super::view::ghost_with(look, SharedString::from(format!("judges-{name}")), Some(glyph), &said, off);
            let press = match self.judge_stop(name) {
                Some(stop) => ringed(look, press.track_focus(&stop), &stop, window),
                None => press,
            };
            labelled(press.when(!off, |press| press.on_click(cx.listener(move |this, _, window, cx| this.judges_press(name, window, cx)))), Role::Button, said).into_any_element()
        };
        let mut row = div().flex().items_center().gap(look.length(token::SPACE_2)).child(save);
        let mut line = String::new();
        if !new {
            let (glyph, said) = if replaces { ("rotate-ccw", "Reset to Armada's".to_string()) } else { ("trash-2", "Delete".to_string()) };
            row = row.child(ghost("reset", glyph, said));
            line = match picked {
                Some(one) if replaces => format!("Reset deletes your copy; {project} uses Armada's {} again. ", one.name),
                _ => "Delete removes your copy. ".to_string(),
            };
            line.push_str(&format!("Commit starts a Job that adds this file to {project}'s repository."));
            row = row.child(div().flex_1()).child(ghost("commit", "git-pull-request", format!("Commit to {project}\u{2026}")));
        }
        div().flex().flex_col().gap(look.length(token::SPACE_2)).child(row).children((!line.is_empty()).then(|| small(look, line, look.color(token::FG_MUTED)))).into_any_element()
    }
}

/// A place chip: a small label in the hue of the place, on its own fill and outline.
fn chip(look: &Look, said: &str, ink: Hsla) -> AnyElement {
    div()
        .flex_none()
        .max_w(look.length(token::SIDEBAR_DEFAULT))
        .truncate()
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(ink.opacity(0.45))
        .bg(ink.opacity(0.14))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(ink)
        .child(SharedString::from(said.to_string()))
        .into_any_element()
}

/// A labelled field: its name small and muted above the control.
fn field(look: &Look, name: &str, control: AnyElement) -> AnyElement {
    div().flex().flex_col().gap(look.length(token::SPACE_1)).child(small(look, name.to_string(), look.color(token::FG_MUTED))).child(control).into_any_element()
}
