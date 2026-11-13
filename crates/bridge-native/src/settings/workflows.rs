//! Settings → Workflows: every workflow Fleet runs, grouped by where it is kept, with the switch for Jobs at the
//! top and one on each row. A port of the list column of Classic's Workflow manager
//! (`packages/components/src/compositions/WorkflowManager/WorkflowManager.tsx`, `workflow-manager-wire.ts`,
//! `workflows-off.ts`), read on All: Fleet's `GET /workflows` as the connection holds it, the switches saved into
//! `features.workflows` and `workflows.off`.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{canvas, div, prelude::*, px, svg, AnyElement, App, Context, Entity, FocusHandle, Focusable, FontWeight, Keystroke, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{SettingValue, WorkflowSummary};

use super::overlay::SettingsOverlay;
use super::view::{bone, mono, ringed, small, switch_track, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

pub const FOR_JOBS: &str = "features.workflows";
pub const OFF: &str = "workflows.off";

/// Where a definition is kept, as the page names Fleet's `armada`, `repository`, `kit` and `project`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Armada,
    Repository,
    Yours,
    /// One project's copy; the row's `project` says whose.
    Project,
}

fn place_of(wire: &str) -> Option<Place> {
    match wire {
        "armada" => Some(Place::Armada),
        "repository" => Some(Place::Repository),
        "kit" => Some(Place::Yours),
        "project" => Some(Place::Project),
        _ => None,
    }
}

/// One row of the list (`Listed`).
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub id: String,
    pub place: Place,
    pub manifest: String,
    /// The repository a row of Armada's or Yours runs in, where it keeps no copy of its own.
    pub runs_in: Option<String>,
    pub for_requests: Option<String>,
    /// The project whose copy this is, where `place` is `Project`.
    pub project: Option<String>,
}

impl Listed {
    /// One key per row (`keyOf`).
    pub fn key(&self) -> String {
        let place = match self.place {
            Place::Armada => "armada",
            Place::Repository => "repository",
            Place::Yours => "yours",
            Place::Project => "project",
        };
        let at = match (&self.place, &self.runs_in) {
            (Place::Repository, _) => format!("@{}", self.manifest),
            (Place::Project, None) => format!("/{}", self.project.as_deref().unwrap_or_default()),
            (Place::Project, Some(runs_in)) => format!("/{}>{runs_in}", self.project.as_deref().unwrap_or_default()),
            (_, Some(runs_in)) => format!(">{runs_in}"),
            _ => String::new(),
        };
        format!("{place}{at}:{}", self.id)
    }

    fn in_repository(&self) -> bool {
        self.place == Place::Repository || self.runs_in.is_some()
    }
}

/// The rows from what Fleet runs on All: each workflow once, a row for each definition a more specific place
/// replaced, then what runs in each repository it keeps no copy of; each repository's rows where it is listed, in
/// the order Fleet lists them under it (`listedOf`, `runningIn`, `runOrderOf`).
pub fn listed_of(workflows: &[WorkflowSummary], served: &[String]) -> Vec<Listed> {
    let mut out: Vec<Listed> = Vec::new();
    let add = |one: Listed, out: &mut Vec<Listed>| {
        if !out.iter().any(|was| was.key() == one.key()) {
            out.push(one);
        }
    };
    for one in workflows {
        let Some(place) = place_of(&one.source) else { continue };
        let manifest = one.manifest_id.as_str().to_string();
        add(Listed { id: one.id.as_str().to_string(), place, manifest: manifest.clone(), runs_in: None, for_requests: one.for_requests.clone(), project: one.project.clone() }, &mut out);
        for under in &one.overrides {
            if let Some(place) = place_of(&under.source) {
                add(Listed { id: one.id.as_str().to_string(), place, manifest: manifest.clone(), runs_in: None, for_requests: None, project: under.project.clone() }, &mut out);
            }
        }
    }
    let keeps: Vec<String> = out.iter().filter(|one| one.place == Place::Repository).map(|one| format!("{}/{}", one.manifest, one.id)).collect();
    for manifest in served {
        for one in workflows {
            let Some(place) = place_of(&one.source) else { continue };
            if one.manifest_id.as_str() != manifest || place == Place::Repository || keeps.contains(&format!("{manifest}/{}", one.id.as_str())) {
                continue;
            }
            add(Listed { id: one.id.as_str().to_string(), place, manifest: manifest.clone(), runs_in: Some(manifest.clone()), for_requests: one.for_requests.clone(), project: one.project.clone() }, &mut out);
        }
    }
    let ran = |one: &Listed| {
        if !one.in_repository() {
            return 0;
        }
        workflows.iter().position(|each| each.manifest_id.as_str() == one.manifest && each.id.as_str() == one.id).unwrap_or(usize::MAX)
    };
    let at = |one: &Listed| if one.in_repository() { served.iter().position(|each| *each == one.manifest).map_or(isize::MAX, |at| at as isize) } else { -1 };
    // A stable sort: rows that tie stay in the order they were found.
    out.sort_by_key(|one| (at(one), ran(one)));
    // A replaced definition is listed without its line; until its own definition is read, it says what the id is for.
    let lines: Vec<(String, String)> = out.iter().filter_map(|one| one.for_requests.clone().map(|line| (one.id.clone(), line))).collect();
    for one in out.iter_mut().filter(|one| one.for_requests.is_none()) {
        one.for_requests = lines.iter().find(|(id, _)| *id == one.id).map(|(_, line)| line.clone());
    }
    out
}

/// A group of the list: its caption, its rows, and where its switches switch (`None` is the machine).
pub struct Group {
    pub label: String,
    pub rows: Vec<Listed>,
    pub at: Option<(String, String)>,
}

/// Shipped with Armada, each repository's, the picked project's own copies, then Yours, each with what the search
/// keeps. Another project's copies are drawn only where they run.
pub fn groups_of(rows: &[Listed], query: &str, name_of: &dyn Fn(&str) -> String, project: Option<&(String, String)>) -> Vec<Group> {
    let wanted = query.trim().to_lowercase();
    let found: Vec<&Listed> = rows.iter().filter(|one| one.id.to_lowercase().contains(&wanted)).collect();
    let mut groups = vec![Group { label: "Shipped with Armada".into(), rows: found.iter().filter(|one| one.place == Place::Armada && one.runs_in.is_none()).map(|one| (*one).clone()).collect(), at: None }];
    let mut manifests: Vec<&str> = Vec::new();
    for one in found.iter().filter(|one| one.in_repository()) {
        if !manifests.contains(&one.manifest.as_str()) {
            manifests.push(&one.manifest);
        }
    }
    for manifest in manifests {
        let name = name_of(manifest);
        groups.push(Group {
            label: format!("{name} repository"),
            rows: found.iter().filter(|one| one.in_repository() && one.manifest == manifest).map(|one| (*one).clone()).collect(),
            at: Some((manifest.to_string(), name)),
        });
    }
    if let Some((id, name)) = project {
        let own = found.iter().filter(|one| one.place == Place::Project && one.runs_in.is_none() && one.project.as_deref() == Some(id.as_str()));
        groups.push(Group { label: format!("{name} project"), rows: own.map(|one| (*one).clone()).collect(), at: None });
    }
    groups.push(Group { label: "Yours".into(), rows: found.iter().filter(|one| one.place == Place::Yours && one.runs_in.is_none()).map(|one| (*one).clone()).collect(), at: None });
    groups.retain(|group| !group.rows.is_empty());
    groups
}

/// In a repository's group, the mark of a row that is not shipped with Armada: the repository's own copy, or Yours
/// (`customOf`). Its glyph and what its tooltip says.
fn custom_of(one: &Listed, rows: &[Listed], name: &str, project_name: &dyn Fn(&str) -> String) -> Option<(&'static str, String)> {
    let replaces = if rows.iter().any(|other| other.id == one.id && other.place == Place::Armada) { ", replaces Armada's" } else { "" };
    match one.place {
        Place::Repository => Some(("folder-git-2", format!("{name}'s own copy{replaces}"))),
        Place::Yours => Some(("briefcase", format!("Yours{replaces}"))),
        Place::Project => Some(("layers", format!("{} project's copy{replaces}", project_name(one.project.as_deref().unwrap_or_default())))),
        Place::Armada => None,
    }
}

/// Outside a repository's group, what a copy replaces (`replacesOf`).
fn replaces_of(one: &Listed, rows: &[Listed]) -> Option<&'static str> {
    let has = |place: Place| rows.iter().any(|other| other.id == one.id && other.place == place);
    match one.place {
        Place::Repository if has(Place::Armada) => Some("This repository's copy replaces Armada's"),
        Place::Yours if has(Place::Repository) => Some("Yours replaces this repository's"),
        Place::Yours if has(Place::Armada) => Some("Yours replaces Armada's"),
        Place::Project if has(Place::Yours) => Some("This project's copy replaces Yours"),
        Place::Project if has(Place::Repository) => Some("This project's copy replaces this repository's"),
        Place::Project if has(Place::Armada) => Some("This project's copy replaces Armada's"),
        _ => None,
    }
}

/// The workflows switched off, as `workflows.off` holds them: on the machine, and in each repository by its
/// Manifest id. Anything missing or the wrong shape reads as nothing off (`workflowsOffOf`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Off {
    pub machine: Vec<String>,
    pub repositories: BTreeMap<String, Vec<String>>,
}

fn ids_of(value: Option<&SettingValue>) -> Vec<String> {
    match value {
        Some(SettingValue::List(items)) => items.iter().filter_map(|item| if let SettingValue::Text(text) = item { Some(text.clone()) } else { None }).collect(),
        _ => Vec::new(),
    }
}

pub fn off_of(value: Option<&SettingValue>) -> Off {
    let Some(SettingValue::Map(held)) = value else { return Off::default() };
    let repositories = match held.get("repositories") {
        Some(SettingValue::Map(map)) => map.iter().map(|(manifest, ids)| (manifest.clone(), ids_of(Some(ids)))).collect(),
        _ => BTreeMap::new(),
    };
    Off { machine: ids_of(held.get("machine")), repositories }
}

/// The setting with one workflow switched, on the machine where no repository is named (`switched`).
pub fn switched(off: &Off, id: &str, on: bool, manifest: Option<&str>) -> SettingValue {
    let moved = |ids: &[String]| {
        let mut next: Vec<String> = ids.iter().filter(|one| *one != id).cloned().collect();
        if !on {
            next.push(id.to_string());
        }
        next
    };
    let list = |ids: Vec<String>| SettingValue::List(ids.into_iter().map(SettingValue::Text).collect());
    let mut machine = off.machine.clone();
    let mut repositories = off.repositories.clone();
    match manifest {
        None => machine = moved(&off.machine),
        Some(manifest) => {
            let next = moved(off.repositories.get(manifest).map(Vec::as_slice).unwrap_or_default());
            if next.is_empty() {
                repositories.remove(manifest);
            } else {
                repositories.insert(manifest.to_string(), next);
            }
        }
    }
    SettingValue::Map(
        [("machine".to_string(), list(machine)), ("repositories".to_string(), SettingValue::Map(repositories.into_iter().map(|(manifest, ids)| (manifest, list(ids))).collect()))]
            .into_iter()
            .collect(),
    )
}

/// What a row's switch draws: on or off where it switches, or the machine's off in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowSwitch {
    Switch { on: bool, label: String },
    OffOnMachine,
}

pub fn row_switch(off: &Off, id: &str, at: Option<&(String, String)>) -> RowSwitch {
    let off_on_machine = off.machine.iter().any(|one| one == id);
    match at {
        Some(_) if off_on_machine => RowSwitch::OffOnMachine,
        None => RowSwitch::Switch { on: !off_on_machine, label: format!("{} on this machine", if off_on_machine { "Off" } else { "On" }) },
        Some((manifest, name)) => {
            let on = !off.repositories.get(manifest).is_some_and(|ids| ids.iter().any(|one| one == id));
            RowSwitch::Switch { on, label: format!("{} in {name}", if on { "On" } else { "Off" }) }
        }
    }
}

/// The page's own state: the row picked, the search, and the keys of each control.
pub struct WorkflowsPage {
    pub picked: Option<String>,
    pub filter: Entity<InputState>,
    pub for_jobs: FocusHandle,
    /// Each row's name and switch, by the row's key.
    rows: BTreeMap<String, [FocusHandle; 2]>,
    /// The workflow picked, drawn beside the list.
    pub detail: super::workflow_detail::Detail,
    /// The page's width as last drawn, and whether that left the detail too narrow to sit beside the list, so
    /// the two take the same place in turn.
    wide: Rc<Cell<gpui::Pixels>>,
    narrow: Cell<bool>,
    /// Where the detail stands in the list's place: picked from the list, until Esc or the back row.
    pub opened: bool,
    pub back: FocusHandle,
    _filter: Subscription,
}

impl WorkflowsPage {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsOverlay>) -> WorkflowsPage {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Find workflow"));
        let sub = cx.subscribe_in(&filter, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        WorkflowsPage { picked: None, filter, for_jobs: cx.focus_handle().tab_stop(true), rows: BTreeMap::new(), detail: Default::default(), wide: Rc::default(), narrow: Cell::new(false), opened: false, back: cx.focus_handle().tab_stop(true), _filter: sub }
    }

    pub fn filter_focused(&self, window: &Window, cx: &App) -> bool {
        self.filter.read(cx).focus_handle(cx).is_focused(window)
    }

    /// Whether the detail is drawn in the list's place now.
    pub fn in_place(&self) -> bool {
        self.narrow.get() && self.opened
    }
}

impl SettingsOverlay {
    /// The rows as drawn now, or `None` before Fleet has listed what it runs.
    pub(super) fn workflow_rows(&self, cx: &App) -> Option<Vec<Listed>> {
        let served: Vec<String> = self.wire.repositories(cx).unwrap_or_default().into_iter().map(|(id, _, _)| id).collect();
        self.workflows.detail.listed(self.wire.workflows(cx)).map(|workflows| listed_of(&workflows, &served))
    }

    fn workflow_groups(&self, cx: &App) -> Option<(Vec<Listed>, Vec<Group>)> {
        self.workflow_groups_for(&self.workflows.filter.read(cx).value(), cx)
    }

    fn workflow_groups_for(&self, query: &str, cx: &App) -> Option<(Vec<Listed>, Vec<Group>)> {
        let rows = self.workflow_rows(cx)?;
        let names = self.wire.repositories(cx).unwrap_or_default();
        let name_of = |manifest: &str| names.iter().find(|(id, _, _)| id == manifest).map_or_else(|| manifest.to_string(), |(_, name, _)| name.clone());
        let groups = groups_of(&rows, query, &name_of, self.scope.project.as_ref());
        Some((rows, groups))
    }

    /// Every row's switch whatever is searched, for ⌘K: its key, what it switches, and whether it is on.
    pub(super) fn workflow_switches(&self, cx: &App) -> Vec<(String, String, bool)> {
        let Some((_, groups)) = self.workflow_groups_for("", cx) else { return Vec::new() };
        let off = self.workflows_off();
        groups
            .into_iter()
            .flat_map(|group| {
                let off = off.clone();
                group.rows.into_iter().filter_map(move |one| match row_switch(&off, &one.id, group.at.as_ref()) {
                    RowSwitch::Switch { on, .. } => {
                        let said = group.at.as_ref().map_or_else(|| format!("{} on this machine", one.id), |(_, name)| format!("{} in {name}", one.id));
                        Some((one.key(), said, on))
                    }
                    RowSwitch::OffOnMachine => None,
                })
            })
            .collect()
    }

    /// A row's switch taken from ⌘K, by its key.
    pub fn apply_workflow(&mut self, key: &str, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, groups)) = self.workflow_groups_for("", cx) else { return };
        let Some((one, at)) = groups.into_iter().find_map(|group| group.rows.into_iter().find(|one| one.key() == key).map(|one| (one, group.at))) else { return };
        self.switch_workflow(&one.id, on, at.as_ref().map(|(manifest, _)| manifest.as_str()), window, cx);
    }

    fn workflows_off(&self) -> Off {
        off_of(self.setting(OFF).map(|one| &one.value))
    }

    /// Keys for each row's name and switch, made the first time it is listed.
    pub(super) fn workflow_keys(&mut self, cx: &mut Context<Self>) {
        let Some(rows) = self.workflow_rows(cx) else { return };
        for one in rows {
            self.workflows.rows.entry(one.key()).or_insert_with(|| [cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)]);
        }
    }

    /// The rows the search keeps, in the order drawn, each with where its switch switches; and the one picked.
    pub(super) fn shown_workflows(&self, cx: &App) -> (Vec<(Listed, Option<(String, String)>)>, Option<Listed>) {
        let Some((_, groups)) = self.workflow_groups(cx) else { return (Vec::new(), None) };
        let shown: Vec<(Listed, Option<(String, String)>)> = groups.into_iter().flat_map(|group| {
            let at = group.at.clone();
            group.rows.into_iter().map(move |one| (one, at.clone()))
        }).collect();
        let picked = shown.iter().find(|(one, _)| Some(one.key()) == self.workflows.picked).or_else(|| shown.first()).map(|(one, _)| one.clone());
        (shown, picked)
    }

    /// The page's stops: the switch for Jobs, the search, the row picked, then each row's switch; then the detail's.
    pub(super) fn workflow_stops(&self, window: &Window, cx: &App) -> Vec<FocusHandle> {
        if self.workflows.in_place() {
            let mut stops = vec![self.workflows.back.clone()];
            stops.extend(self.detail_stops_for(window, cx));
            return stops;
        }
        let mut stops = vec![self.workflows.for_jobs.clone(), self.workflows.filter.read(cx).focus_handle(cx)];
        let (shown, picked) = self.shown_workflows(cx);
        let off = self.workflows_off();
        for (one, at) in &shown {
            let Some([row, switch]) = self.workflows.rows.get(&one.key()) else { continue };
            if picked.as_ref().is_some_and(|picked| picked.key() == one.key()) {
                stops.push(row.clone());
            }
            if matches!(row_switch(&off, &one.id, at.as_ref()), RowSwitch::Switch { .. }) {
                stops.push(switch.clone());
            }
        }
        if !self.workflows.narrow.get() {
            stops.extend(self.detail_stops_for(window, cx));
        }
        stops
    }

    pub fn switch_for_jobs(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.save(FOR_JOBS.into(), Some(SettingValue::Bool(on)), window, cx);
    }

    /// One workflow switched on the machine, or in the repository named.
    pub fn switch_workflow(&mut self, id: &str, on: bool, manifest: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let value = switched(&self.workflows_off(), id, on, manifest);
        self.save(OFF.into(), Some(value), window, cx);
    }

    pub(super) fn pick_workflow(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.workflows.picked = Some(key.to_string());
        // The keys go to the row, or to the back row where the detail stands in the list's place: never to an act
        // the pick took away.
        if self.workflows.in_place() {
            window.focus(&self.workflows.back, cx);
        } else if let Some([row, _]) = self.workflows.rows.get(key) {
            window.focus(row, cx);
        }
        cx.notify();
    }

    /// A row taken: drawn beside the list, or in its place where the page is narrow, the keys on its first stop.
    fn open_workflow(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.pick_workflow(key, window, cx);
        if self.workflows.narrow.get() {
            self.workflows.opened = true;
            window.focus(&self.workflows.back, cx);
            cx.notify();
        }
    }

    /// Back to the list from the detail drawn in its place, the same row picked and the keys on it.
    pub(super) fn back_to_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workflows.opened = false;
        if let Some(key) = self.workflows.picked.clone().or_else(|| self.shown_workflows(cx).1.map(|one| one.key())) {
            self.pick_workflow(&key, window, cx);
        }
        cx.notify();
    }

    /// The page's own keys: Esc clears the search, F reaches it, ↑ and ↓ move between rows, Enter and Space switch.
    pub(super) fn workflows_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        let word = key.key.as_str();
        if self.workflows.in_place() && word == "[" && mods.platform && !mods.control && !mods.alt {
            self.back_to_list(window, cx);
            return true;
        }
        if self.lanes_key(key, window, cx) {
            return true;
        }
        if self.band_key(key, window, cx) {
            return true;
        }
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        if self.detail_key(word, window, cx) {
            return true;
        }
        if self.workflows.in_place() {
            match word {
                "escape" => {
                    self.back_to_list(window, cx);
                    return true;
                }
                "enter" | "space" if self.workflows.back.is_focused(window) => {
                    self.back_to_list(window, cx);
                    return true;
                }
                // The search is the list's: F goes back to it first.
                "f" if !mods.shift && !self.in_text(window, cx) => {
                    self.back_to_list(window, cx);
                    self.workflows.filter.update(cx, |filter, cx| filter.focus(window, cx));
                    return true;
                }
                _ => {}
            }
        }
        if self.workflows.filter_focused(window, cx) {
            if word == "escape" && !self.workflows.filter.read(cx).value().is_empty() {
                self.workflows.filter.update(cx, |filter, cx| filter.set_value("", window, cx));
                cx.notify();
                return true;
            }
            return false;
        }
        if word == "f" && !mods.shift && !self.in_text(window, cx) {
            self.workflows.filter.update(cx, |filter, cx| filter.focus(window, cx));
            return true;
        }
        if self.workflows.for_jobs.is_focused(window) && matches!(word, "enter" | "space") {
            let on = self.setting(FOR_JOBS).is_none_or(|one| one.value != SettingValue::Bool(false));
            self.switch_for_jobs(!on, window, cx);
            return true;
        }
        let (shown, _) = self.shown_workflows(cx);
        let focused = shown.iter().position(|(one, _)| self.workflows.rows.get(&one.key()).is_some_and(|[row, _]| row.is_focused(window)));
        if let Some(at) = focused {
            match word {
                "up" | "down" => {
                    let next = if word == "up" { at.saturating_sub(1) } else { (at + 1).min(shown.len() - 1) };
                    let key = shown[next].0.key();
                    self.pick_workflow(&key, window, cx);
                    return true;
                }
                "enter" | "space" => {
                    let key = shown[at].0.key();
                    self.open_workflow(&key, window, cx);
                    return true;
                }
                _ => return false,
            }
        }
        let on_switch = shown.iter().find(|(one, _)| self.workflows.rows.get(&one.key()).is_some_and(|[_, switch]| switch.is_focused(window))).cloned();
        if let Some((one, at)) = on_switch {
            if matches!(word, "enter" | "space") {
                if let RowSwitch::Switch { on, .. } = row_switch(&self.workflows_off(), &one.id, at.as_ref()) {
                    self.switch_workflow(&one.id, !on, at.as_ref().map(|(manifest, _)| manifest.as_str()), window, cx);
                }
                return true;
            }
        }
        false
    }

    // ---- drawn --------------------------------------------------------------------------------

    pub(super) fn render_workflows(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let off_sending = !self.live || self.saving.is_some();
        let for_jobs = self.setting(FOR_JOBS).is_none_or(|one| one.value != SettingValue::Bool(false));
        let jobs_switch = ringed(
            look,
            div()
                .id("workflows-for-jobs")
                .track_focus(&self.workflows.for_jobs)
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child("Workflows for Jobs"))
                .child(switch_track(look, for_jobs, off_sending))
                .when(!off_sending, |row| row.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.switch_for_jobs(!for_jobs, window, cx)))),
            &self.workflows.for_jobs,
            window,
        );
        let key_hint = div()
            .flex_none()
            .px(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED))
            .child("F");
        let filter = labelled(
            div().id("workflows-filter").flex_none().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&self.workflows.filter).suffix(key_hint)),
            Role::SearchInput,
            "Search workflows",
        );
        let kept = div().px(look.length(token::SPACE_2)).font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("Kept in");
        let mut groups = div().id("workflows-groups").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(look.length(token::SPACE_4));
        let (_, picked) = self.shown_workflows(cx);
        match self.workflow_groups(cx) {
            None => {
                let mut rows = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(div().px(look.length(token::SPACE_2)).py(look.length(token::SPACE_2)).child(bone(look, px(64.), look.length(token::LEADING_XS))));
                for (n, wide) in [96., 128., 80., 112., 144.].into_iter().enumerate() {
                    rows = rows.child(div().id(SharedString::from(format!("workflows-bone-{n}"))).flex().items_center().justify_between().min_h(look.length(token::H_CONTROL)).px(look.length(token::SPACE_3)).child(bone(look, px(wide), look.length(token::LEADING_XS))).child(bone(look, px(40.), look.length(token::LEADING_XS))));
                }
                groups = groups.child(labelled(rows.id("workflows-pending"), Role::Image, "Loading"));
            }
            Some((rows, listed)) => {
                let off = self.workflows_off();
                for group in listed {
                    let caption = div()
                        .flex()
                        .items_center()
                        .min_h(look.length(token::H_CONTROL_SM))
                        .pl(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(group.label.clone()));
                    let mut block = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption);
                    for one in &group.rows {
                        let current = picked.as_ref().is_some_and(|picked| picked.key() == one.key());
                        block = block.child(self.render_workflow_row(one, &rows, group.at.as_ref(), current, &off, off_sending, look, window, cx));
                    }
                    groups = groups.child(labelled(block.id(SharedString::from(format!("workflows-group-{}", group.label))), Role::Group, group.label.clone()));
                }
            }
        }
        // Where the detail beside the list would be narrower than a dialog, the two take the list's place in turn.
        let wide = self.workflows.wide.get();
        let narrow = wide > px(0.) && wide - look.length(token::W_GUIDE_LIST) - look.length(token::GAP_PANEL) < look.length(token::W_DIALOG);
        self.workflows.narrow.set(narrow);
        let in_place = narrow && self.workflows.opened;
        let measure = {
            let wide = self.workflows.wide.clone();
            canvas(
                move |bounds, window, _| {
                    if wide.get() != bounds.size.width {
                        wide.set(bounds.size.width);
                        window.refresh();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .left_0()
            .top_0()
            .size_full()
        };
        let list = super::models::panel(look, "workflows-list")
            .map(|list| if narrow { list.flex_1() } else { list.flex_none().w(look.length(token::W_GUIDE_LIST)) })
            .p(look.length(token::SPACE_3))
            .gap(look.length(token::SPACE_4))
            .child(labelled(jobs_switch, Role::Switch, "Workflows for Jobs"))
            .child(filter)
            .child(kept)
            .child(groups);
        let back = in_place.then(|| {
            let back = ringed(
                look,
                div()
                    .id("workflows-back")
                    .track_focus(&self.workflows.back)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.1)))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(svg().path(icon_path("chevron-left")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                    .child("Workflows")
                    .on_click(cx.listener(|this, _, window, cx| this.back_to_list(window, cx))),
                &self.workflows.back,
                window,
            );
            div().px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3)).child(labelled(back, Role::Button, "Back to Workflows"))
        });
        let detail = super::models::panel(look, "workflows-detail").track_scroll(&self.workflows.detail.scroll).flex_1().children(back).children(picked.map(|one| self.render_workflow_detail(&one, look, window, cx)));
        div()
            .id("settings-page")
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            .child(measure)
            .when(!in_place, |page| page.child(labelled(list, Role::Region, "Workflows")))
            .when(!narrow || in_place, |page| page.child(labelled(detail, Role::Region, "Workflow")))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_workflow_row(&self, one: &Listed, rows: &[Listed], at: Option<&(String, String)>, current: bool, off: &Off, sending: bool, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = one.key();
        let Some([row_key, switch_key]) = self.workflows.rows.get(&key).cloned() else { return div().into_any_element() };
        let project_name = |id: &str| self.project_name(id);
        let custom = at.and_then(|(_, name)| custom_of(one, rows, name, &project_name));
        let replaces = if at.is_none() { replaces_of(one, rows) } else { None };
        let ink = if custom.is_some() { look.color(token::ACCENT) } else if current { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
        let press = key.clone();
        let entry = div()
            .flex_1()
            .min_w_0()
            .px(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_XS))
            .text_color(ink)
            .truncate()
            .child(SharedString::from(one.id.clone()));
        let mark: Option<(&'static str, String)> = custom.clone().or_else(|| replaces.map(|said| ("layers", said.to_string())));
        let mark_cell = div().flex_none().w(look.length(token::SPACE_6)).flex().items_center().justify_center().children(mark.map(|(glyph, said)| {
            labelled(
                div().id(SharedString::from(format!("workflow-mark-{key}"))).tooltip(tip(said.clone())).child(svg().path(icon_path(glyph)).size(px(14.)).text_color(look.color(token::FG_SUBTLE))),
                Role::Image,
                said,
            )
        }));
        let switch_cell = match row_switch(off, &one.id, at) {
            RowSwitch::OffOnMachine => div().child(small(look, "Off on this machine", look.color(token::FG_MUTED))).into_any_element(),
            RowSwitch::Switch { on, label } => {
                let id = one.id.clone();
                let manifest = at.map(|(manifest, _)| manifest.clone());
                let switch = ringed(
                    look,
                    div()
                        .id(SharedString::from(format!("workflow-switch-{key}")))
                        .track_focus(&switch_key)
                        .rounded_full()
                        .tooltip(tip(label.clone()))
                        .child(switch_track(look, on, sending))
                        .when(!sending, |switch| {
                            switch.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| {
                                // The row under it picks; the switch only switches.
                                cx.stop_propagation();
                                this.switch_workflow(&id, !on, manifest.as_deref(), window, cx);
                            }))
                        }),
                    &switch_key,
                    window,
                );
                labelled(switch, Role::Switch, format!("{}: {label}", one.id)).into_any_element()
            }
        };
        // The row is one press, one fill and one ring, its switch included, as Classic's picked row is.
        let row = ringed(
            look,
            div()
                .id(SharedString::from(format!("workflow-{key}")))
                .track_focus(&row_key)
                .flex()
                .items_center()
                .min_h(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .map(|row| if current { row.bg(look.color(token::SURFACE_ROW_SELECTED)) } else { row.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.1))) })
                .on_click(cx.listener(move |this, _, window, cx| this.open_workflow(&press, window, cx)))
                .child(entry)
                .child(mark_cell)
                .child(div().flex_none().min_w(look.length(token::SPACE_12)).pr(look.length(token::SPACE_3)).flex().justify_end().child(switch_cell)),
            &row_key,
            window,
        );
        labelled(row, Role::Button, one.id.clone()).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Listed> {
        listed_of(&super::super::mock::settings_workflows(), &["armada".into(), "sample-web".into(), "sample-api".into()])
    }

    #[test]
    fn each_place_is_a_group_and_each_repository_holds_what_runs_there() {
        let rows = rows();
        let groups = groups_of(&rows, "", &|manifest| manifest.to_string(), None);
        let said: Vec<(String, Vec<String>)> = groups.iter().map(|group| (group.label.clone(), group.rows.iter().map(|one| one.id.clone()).collect())).collect();
        let said: Vec<(&str, Vec<&str>)> = said.iter().map(|(label, ids)| (label.as_str(), ids.iter().map(String::as_str).collect())).collect();
        assert_eq!(
            said,
            [
                ("Shipped with Armada", vec!["feature", "bug", "design_plan", "prototype"]),
                ("armada repository", vec!["feature", "bug", "design_plan", "prototype", "docs_only"]),
                ("sample-web repository", vec!["bug"]),
                ("Yours", vec!["docs_only"]),
            ]
        );
    }

    #[test]
    fn with_a_project_picked_its_own_copies_are_a_group_above_yours() {
        let rows = rows();
        let armada = ("armada".to_string(), "Armada".to_string());
        let groups = groups_of(&rows, "", &|manifest| manifest.to_string(), Some(&armada));
        let labels: Vec<&str> = groups.iter().map(|group| group.label.as_str()).collect();
        assert_eq!(labels, ["Shipped with Armada", "armada repository", "sample-web repository", "Armada project", "Yours"]);
        let own = &groups[3].rows;
        assert_eq!((own.len(), own[0].id.as_str(), own[0].project.as_deref()), (1, "design_plan", Some("armada")));
        assert_eq!(replaces_of(&own[0], &rows), Some("This project's copy replaces Armada's"));
        let storefront = ("storefront".to_string(), "Storefront".to_string());
        assert!(!groups_of(&rows, "", &|manifest| manifest.to_string(), Some(&storefront)).iter().any(|group| group.label == "Storefront project"), "a project with no copies has no group");
    }

    #[test]
    fn the_search_keeps_the_ids_that_hold_it_and_drops_empty_groups() {
        let groups = groups_of(&rows(), "DOCS", &|manifest| manifest.to_string(), None);
        assert_eq!(groups.iter().map(|group| group.label.as_str()).collect::<Vec<_>>(), ["armada repository", "Yours"]);
    }

    #[test]
    fn a_repositorys_own_copy_and_yours_are_marked_in_its_group() {
        let rows = rows();
        let own = rows.iter().find(|one| one.place == Place::Repository && one.manifest == "armada").expect("armada's feature");
        assert_eq!(custom_of(own, &rows, "armada", &|id| id.to_string()), Some(("folder-git-2", "armada's own copy, replaces Armada's".to_string())));
        let yours = rows.iter().find(|one| one.place == Place::Yours && one.runs_in.is_some()).expect("Yours in armada");
        assert_eq!(custom_of(yours, &rows, "armada", &|id| id.to_string()).map(|(glyph, _)| glyph), Some("briefcase"));
    }

    #[test]
    fn a_switch_writes_the_machine_or_the_repository_and_drops_an_empty_one() {
        let off = Off::default();
        let machine = off_of(Some(&switched(&off, "bug", false, None)));
        assert_eq!(machine.machine, ["bug"]);
        let there = off_of(Some(&switched(&machine, "feature", false, Some("armada"))));
        assert_eq!(there.repositories.get("armada").map(Vec::as_slice), Some(&["feature".to_string()][..]));
        let back = off_of(Some(&switched(&there, "feature", true, Some("armada"))));
        assert!(back.repositories.is_empty());
        assert_eq!(row_switch(&machine, "bug", Some(&("armada".into(), "armada".into()))), RowSwitch::OffOnMachine);
        assert_eq!(row_switch(&there, "feature", Some(&("armada".into(), "armada".into()))), RowSwitch::Switch { on: false, label: "Off in armada".into() });
    }
}
