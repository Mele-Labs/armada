//! Curating the library from the list: a pending feature is confirmed, merged into another or renamed, and an area
//! past eight features is split. Each is one or more calls Fleet's Operator-only acts now open to Bridge
//! (`change_feature`, `upsert_features`); what Fleet kept is read again after, so the list is what Fleet holds.
//!
//! This file is the state, the decisions and the sends. `curate_view.rs` draws them.

use std::collections::HashMap;

use gpui::{point, px, Anchor, AppContext as _, Context, DismissEvent, Entity, FocusHandle, Focusable as _, Pixels, Point, Role, Subscription, Task, Window};
use gpui_component::input::{InputEvent, InputState};
use ipc::{AreaSeed, ChangeFeature, FeatureLibrary, FeaturePage, FeatureRow, FeatureSeed, FeatureState, UpsertFeatures};

use super::{Load, Source};
use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::settings::wire::Refused;
use crate::titlebar::id_of;

/// The area Fleet lists pending features under until the owner places one.
pub const PENDING_AREA: &str = "pending";

/// An area with more features than this offers Split.
pub const SPLIT_OVER: usize = 8;

/// A feature a Job named that the catalog does not hold.
pub fn is_pending(row: &FeatureRow) -> bool {
    row.state == FeatureState::Pending
}

/// The areas a feature can be placed in: every area the library keeps, in its order, but the pending group.
pub fn areas_kept(library: &FeatureLibrary) -> Vec<(String, String)> {
    library.areas.iter().filter(|area| area.slug != PENDING_AREA).map(|area| (area.slug.clone(), area.title.clone())).collect()
}

/// The features of an area that are not pending, in their order.
pub fn kept_in<'a>(library: &'a FeatureLibrary, area: &str) -> Vec<&'a FeatureRow> {
    library.areas.iter().filter(|one| one.slug == area).flat_map(|one| &one.features).filter(|row| !is_pending(row)).collect()
}

/// How many features an area holds where it holds more than eight.
pub fn over(library: &FeatureLibrary, area: &str) -> Option<usize> {
    Some(kept_in(library, area).len()).filter(|count| *count > SPLIT_OVER)
}

/// The features a pending one can be merged into, each with the title of its area: every one the catalog keeps.
pub fn merge_targets(library: &FeatureLibrary) -> Vec<(&FeatureRow, &str)> {
    library.areas.iter().filter(|area| area.slug != PENDING_AREA).flat_map(|area| area.features.iter().filter(|row| !is_pending(row)).map(|row| (row, area.title.as_str()))).collect()
}

/// A slug a trailer can carry, made from a name: lower-case letters and digits, words joined by one dash.
pub fn slug_of(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// One call to Fleet.
#[derive(Clone, Debug, PartialEq)]
pub enum Write {
    Upsert(UpsertFeatures),
    Change(ChangeFeature),
}

/// Confirm: the feature kept as confirmed in `area` under the title and line he settled on. A feature already kept has its
/// prefixes replaced whole by an upsert, so what its page holds goes back as it is.
pub fn confirm_write(project: &str, row: &FeatureRow, area: &str, title: &str, line: &str, page: Option<&FeaturePage>) -> Write {
    Write::Upsert(UpsertFeatures {
        project: project.to_string(),
        areas: Vec::new(),
        features: vec![FeatureSeed {
            slug: row.slug.clone(),
            area: area.to_string(),
            title: title.trim().to_string(),
            summary: line.trim().to_string(),
            state: Some(FeatureState::Confirmed),
            paths: page.map(|page| page.paths.clone()).unwrap_or_default(),
            entry_points: page.map(|page| page.entry_points.clone()).unwrap_or_default(),
            manifest_id: page.and_then(|page| page.manifest_id.clone()),
        }],
    })
}

/// Merge: the feature folded into the one of slug `into`.
pub fn merge_write(project: &str, slug: &str, into: &str) -> Write {
    Write::Change(ChangeFeature { project: project.to_string(), feature: slug.to_string(), into: Some(into.to_string()), slug: None, title: None, area: None , why: None })
}

/// Rename: what changed of the slug and the title, or none where neither did, which Fleet refuses.
pub fn rename_write(project: &str, row: &FeatureRow, slug: &str, title: &str) -> Option<Write> {
    let (slug, title) = (slug.trim(), title.trim());
    let slug = (slug != row.slug && !slug.is_empty()).then(|| slug.to_string());
    let title = (title != row.title && !title.is_empty()).then(|| title.to_string());
    (slug.is_some() || title.is_some()).then(|| Write::Change(ChangeFeature { project: project.to_string(), feature: row.slug.clone(), into: None, slug, title, area: None , why: None }))
}

/// Split: the area kept beside the one it is split from, then each ticked feature moved into it. The call names every area
/// the library keeps, in its order with the new one after the one it is split from, since Fleet orders an area by its place
/// in the call and a call naming only the new one would put it first.
pub fn split_writes(project: &str, areas: &[(String, String)], from: &str, name: &str, ticked: &[String]) -> Vec<Write> {
    let new = slug_of(name);
    let mut seeds: Vec<AreaSeed> = Vec::new();
    for (slug, title) in areas {
        seeds.push(AreaSeed { slug: slug.clone(), title: title.clone() });
        if slug == from {
            seeds.push(AreaSeed { slug: new.clone(), title: name.trim().to_string() });
        }
    }
    let mut writes = vec![Write::Upsert(UpsertFeatures { project: project.to_string(), areas: seeds, features: Vec::new() })];
    writes.extend(ticked.iter().map(|feature| Write::Change(ChangeFeature { project: project.to_string(), feature: feature.clone(), into: None, slug: None, title: None, area: Some(new.clone()) , why: None })));
    writes
}

/// Which of the two forms an inline editor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Confirm,
    Rename,
}

/// The editor open under a pending row: two boxes, and for Confirm the area.
pub(super) struct Form {
    pub slug: String,
    pub kind: Kind,
    /// Confirm: the area's slug.
    pub area: String,
    /// Confirm: the title. Rename: the slug.
    pub one: Entity<InputState>,
    /// Confirm: the line. Rename: the title.
    pub two: Entity<InputState>,
    pub frame: FocusHandle,
    pub area_focus: FocusHandle,
    pub cancel: FocusHandle,
    pub go: FocusHandle,
    _subs: Vec<Subscription>,
}

impl Form {
    /// The places Tab goes, in order.
    fn stops(&self, cx: &gpui::App) -> Vec<FocusHandle> {
        let mut stops = vec![self.one.focus_handle(cx), self.two.focus_handle(cx)];
        if self.kind == Kind::Confirm {
            stops.push(self.area_focus.clone());
        }
        stops.extend([self.cancel.clone(), self.go.clone()]);
        stops
    }
}

/// The sheet that splits an area.
pub(super) struct Sheet {
    pub area: String,
    pub title: String,
    pub name: Entity<InputState>,
    /// The area's features, each with whether it is ticked.
    pub features: Vec<(String, String, bool)>,
    pub rows: Vec<FocusHandle>,
    pub frame: FocusHandle,
    pub cancel: FocusHandle,
    pub go: FocusHandle,
    _subs: Vec<Subscription>,
}

impl Sheet {
    fn stops(&self, cx: &gpui::App) -> Vec<FocusHandle> {
        let mut stops = vec![self.name.focus_handle(cx)];
        stops.extend(self.rows.iter().cloned());
        stops.extend([self.cancel.clone(), self.go.clone()]);
        stops
    }

    /// The slugs ticked.
    pub fn ticked(&self) -> Vec<String> {
        self.features.iter().filter(|(_, _, on)| *on).map(|(slug, _, _)| slug.clone()).collect()
    }
}

/// A picker open under one of the buttons.
pub(super) struct OpenMenu {
    pub menu: Entity<Dropdown>,
    _subs: Vec<Subscription>,
}

/// What the list holds for curating.
#[derive(Default)]
pub(super) struct Curate {
    /// Each pending feature's three buttons, Confirm, Merge into and Rename, by slug.
    pub stops: HashMap<String, [FocusHandle; 3]>,
    /// Each over-full area's Split button, by slug.
    pub splits: HashMap<String, FocusHandle>,
    pub form: Option<Form>,
    pub sheet: Option<Sheet>,
    pub menu: Option<OpenMenu>,
    sends: Vec<Task<()>>,
    /// The calls made on mock data, which has no Fleet to send them to, for a test to read.
    pub sent: Vec<Write>,
}

/// The next stop after the one that has the keys, or the previous, wrapping.
fn cycle(stops: &[FocusHandle], back: bool, window: &mut Window, cx: &mut gpui::App) {
    if stops.is_empty() {
        return;
    }
    let now = stops.iter().position(|stop| stop.is_focused(window));
    let next = match (now, back) {
        (None, false) => 0,
        (None, true) => stops.len() - 1,
        (Some(at), false) => (at + 1) % stops.len(),
        (Some(at), true) => (at + stops.len() - 1) % stops.len(),
    };
    window.focus(&stops[next], cx);
}

impl super::FeaturesScreen {
    /// The project the library on screen belongs to.
    fn project_id(&self) -> Option<String> {
        match &self.library {
            Load::Read(library) => Some(library.project.clone()),
            _ => None,
        }
    }

    fn row_of(&self, slug: &str) -> Option<FeatureRow> {
        match &self.library {
            Load::Read(library) => library.areas.iter().flat_map(|area| &area.features).find(|row| row.slug == slug).cloned(),
            _ => None,
        }
    }

    fn library_held(&self) -> Option<&FeatureLibrary> {
        match &self.library {
            Load::Read(library) => Some(library),
            _ => None,
        }
    }

    /// A button's handles for each pending feature and each area that offers Split, kept as the library moves.
    pub(super) fn keep_stops(&mut self, cx: &mut Context<Self>) {
        let Load::Read(library) = &self.library else { return };
        let pending: Vec<String> = library.areas.iter().flat_map(|area| &area.features).filter(|row| is_pending(row)).map(|row| row.slug.clone()).collect();
        let over_full: Vec<String> = library.areas.iter().filter(|area| over(library, &area.slug).is_some()).map(|area| area.slug.clone()).collect();
        self.curate.stops.retain(|slug, _| pending.contains(slug));
        for slug in pending {
            self.curate.stops.entry(slug).or_insert_with(|| [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()]);
        }
        self.curate.splits.retain(|slug, _| over_full.contains(slug));
        for slug in over_full {
            self.curate.splits.entry(slug).or_insert_with(|| cx.focus_handle());
        }
    }

    /// The buttons on the row the keys are on.
    fn row_stops(&self) -> Vec<FocusHandle> {
        let Some(slug) = self.cursor.as_deref() else { return Vec::new() };
        let Some(row) = self.row_of(slug) else { return Vec::new() };
        if is_pending(&row) {
            return self.curate.stops.get(slug).map(|stops| stops.to_vec()).unwrap_or_default();
        }
        self.curate.splits.get(&row.area).map(|stop| vec![stop.clone()]).unwrap_or_default()
    }

    /// Whether a button of a row, an editor or the sheet has the keys: the bare keys and Enter are theirs, not the list's.
    pub(super) fn curating(&self, window: &Window, cx: &gpui::App) -> bool {
        self.curate.sheet.is_some()
            || self.curate.menu.is_some()
            || self.curate.form.as_ref().is_some_and(|form| form.frame.contains_focused(window, cx))
            || self.curate.stops.values().any(|stops| stops.iter().any(|stop| stop.is_focused(window)))
            || self.curate.splits.values().any(|stop| stop.is_focused(window))
    }

    /// Whether a text box of the editor or the sheet has the keys.
    pub(super) fn typing_in_curate(&self, window: &Window, cx: &gpui::App) -> bool {
        self.curate.form.as_ref().is_some_and(|form| form.frame.contains_focused(window, cx)) || self.curate.sheet.as_ref().is_some_and(|sheet| sheet.frame.contains_focused(window, cx))
    }

    /// Tab and Shift+Tab where the keys are in the sheet or an editor, or on a row that has buttons of its own. True where it
    /// took the key; past the last button of a row Tab goes on to the page, as it does from a row with none.
    pub(super) fn tab_curate(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(sheet) = &self.curate.sheet {
            let stops = sheet.stops(cx);
            cycle(&stops, back, window, cx);
            return true;
        }
        if let Some(form) = self.curate.form.as_ref().filter(|form| form.frame.contains_focused(window, cx)) {
            let stops = form.stops(cx);
            cycle(&stops, back, window, cx);
            return true;
        }
        if self.tab_librarian(back, window, cx) {
            return true;
        }
        let stops = self.row_stops();
        match stops.iter().position(|stop| stop.is_focused(window)) {
            None if !back && !stops.is_empty() && self.list_focus.is_focused(window) => window.focus(&stops[0], cx),
            Some(0) if back => self.focus_list(window, cx),
            Some(at) if back => window.focus(&stops[at - 1], cx),
            Some(at) if at + 1 < stops.len() => window.focus(&stops[at + 1], cx),
            _ => return false,
        }
        true
    }

    /// Enter on a button of a row: the press the keys are on.
    pub(super) fn press_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(slug) = self.cursor.clone() else { return false };
        if let Some(at) = self.curate.stops.get(&slug).and_then(|stops| stops.iter().position(|stop| stop.is_focused(window))) {
            match at {
                0 => self.open_confirm(slug, window, cx),
                1 => self.open_merge(slug, window, cx),
                _ => self.open_rename(slug, window, cx),
            }
            return true;
        }
        if let Some(area) = self.row_of(&slug).map(|row| row.area).filter(|area| self.curate.splits.get(area).is_some_and(|stop| stop.is_focused(window))) {
            self.open_split(area, window, cx);
            return true;
        }
        false
    }

    /// Esc: the picker, the sheet or the editor that is open shuts, and the keys go back to the button that opened it. The
    /// picker shuts itself.
    pub(super) fn escape_curate(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.curate.menu.is_some() {
            return false;
        }
        if self.curate.sheet.is_some() {
            self.close_sheet(window, cx);
            return true;
        }
        if self.curate.form.is_some() {
            self.close_form(window, cx);
            return true;
        }
        self.escape_librarian(window, cx)
    }

    // ---- the editors --------------------------------------------------------------------------

    fn input(&self, value: &str, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |state, cx| state.set_value(value.to_string(), window, cx));
        input
    }

    fn open_form(&mut self, slug: String, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.row_of(&slug) else { return };
        // Confirm opens on the first area the project keeps.
        let first = self.library_held().and_then(|library| areas_kept(library).into_iter().next()).map(|(slug, _)| slug).unwrap_or_default();
        let area = if kind == Kind::Confirm { first } else { String::new() };
        self.shut_form(cx);
        let (one, two) = match kind {
            Kind::Confirm => (self.input(&row.title, window, cx), self.input(&row.summary, window, cx)),
            Kind::Rename => (self.input(&row.slug, window, cx), self.input(&row.title, window, cx)),
        };
        let mut subs = Vec::new();
        for input in [&one, &two] {
            subs.push(cx.subscribe_in(input, window, |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.submit_form(window, cx),
                InputEvent::Change => cx.notify(),
                _ => {}
            }));
        }
        let first = one.clone();
        self.curate.form = Some(Form { slug: slug.clone(), kind, area, one, two, frame: cx.focus_handle(), area_focus: cx.focus_handle(), cancel: cx.focus_handle(), go: cx.focus_handle(), _subs: subs });
        window.defer(cx, move |window, cx| first.update(cx, |input, cx| input.focus(window, cx)));
        self.remeasure(&slug);
        cx.notify();
    }

    /// Confirm: the row opens its editor with the first area, the title and the line it has.
    pub(super) fn open_confirm(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_form(slug, Kind::Confirm, window, cx);
    }

    /// Rename: the row opens its editor with the slug and the title it has.
    pub(super) fn open_rename(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_form(slug, Kind::Rename, window, cx);
    }

    fn shut_form(&mut self, cx: &mut Context<Self>) {
        if let Some(form) = self.curate.form.take() {
            self.remeasure(&form.slug);
        }
        cx.notify();
    }

    /// Cancel or Esc: nothing is sent, and the keys go back to the button that opened it.
    pub(super) fn close_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.curate.form.take() else { return };
        let back = self.curate.stops.get(&form.slug).map(|stops| match form.kind {
            Kind::Confirm => stops[0].clone(),
            Kind::Rename => stops[2].clone(),
        });
        self.remeasure(&form.slug);
        match back {
            Some(back) => window.focus(&back, cx),
            None => self.focus_list(window, cx),
        }
        cx.notify();
    }

    /// The list measured the row again, since an editor under it changed its height.
    fn remeasure(&mut self, slug: &str) {
        if let Some(line) = self.listed.find(slug).and_then(|at| self.listed.line_of(at)) {
            self.list.remeasure_items(line..line + 1);
        }
    }

    /// What the editor holds is enough to send.
    pub(super) fn form_ready(&self, cx: &gpui::App) -> bool {
        let Some(form) = &self.curate.form else { return false };
        let (one, two) = (form.one.read(cx).value().trim().to_string(), form.two.read(cx).value().trim().to_string());
        match form.kind {
            Kind::Confirm => !one.is_empty() && !two.is_empty() && !form.area.is_empty(),
            Kind::Rename => {
                let Some(row) = self.row_of(&form.slug) else { return false };
                !one.is_empty() && !two.is_empty() && (one != row.slug || two != row.title)
            }
        }
    }

    /// Enter or the editor's button.
    pub(super) fn submit_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.form_ready(cx) {
            return;
        }
        let Some(form) = self.curate.form.as_ref() else { return };
        let (slug, kind, area) = (form.slug.clone(), form.kind, form.area.clone());
        let (one, two) = (form.one.read(cx).value().to_string(), form.two.read(cx).value().to_string());
        let (Some(project), Some(row)) = (self.project_id(), self.row_of(&slug)) else { return };
        self.close_form(window, cx);
        match kind {
            Kind::Rename => {
                let Some(write) = rename_write(&project, &row, &one, &two) else { return };
                let follows = Some(one.trim().to_string()).filter(|slug| !slug.is_empty());
                self.send(vec![write], follows, cx);
            }
            Kind::Confirm => {
                // The upsert replaces the feature's prefixes whole, so the page is read first and they go back as they were.
                let page = self.page_for(&project, &slug, cx);
                self.curate.sends.retain(|task| !task.is_ready());
                let task = cx.spawn(async move |this, cx| match page.await {
                    Ok(page) => {
                        let write = confirm_write(&project, &row, &area, &one, &two, page.as_ref());
                        let _ = this.update(cx, |this, cx| this.send(vec![write], Some(slug), cx));
                    }
                    Err(why) => {
                        let _ = this.update(cx, |_, cx| told(why.said, cx));
                    }
                });
                self.curate.sends.push(task);
            }
        }
    }

    /// The page a confirm keeps the prefixes of. None on mock data, which holds none of its own.
    fn page_for(&self, project: &str, slug: &str, cx: &gpui::App) -> Task<Result<Option<FeaturePage>, Refused>> {
        let Source::Live(fleet) = &self.source else { return Task::ready(Ok(None)) };
        let Some(port) = fleet.read(cx).held().port() else { return Task::ready(Err(Refused::said("Fleet is not connected"))) };
        let read = super::wire::page(port, project, slug, cx);
        cx.background_spawn(async move { read.await.map(Some) })
    }

    // ---- the pickers --------------------------------------------------------------------------

    /// Where a picker's corner sits: under the button of this name, at its right edge.
    fn under(&self, name: &str, window: &Window, cx: &gpui::App) -> Point<Pixels> {
        crate::settings::view::bounds_of(window, cx, Role::Button, name).map_or_else(|| point(window.viewport_size().width / 2., px(120.)), |bounds| point(bounds.right(), bounds.bottom()))
    }

    /// A picker open: its pick is `picked`, and the keys go to `back` when it shuts.
    fn open_menu(&mut self, menu: Entity<Dropdown>, back: FocusHandle, picked: impl Fn(&mut Self, &str, &mut Window, &mut Context<Self>) + 'static, window: &mut Window, cx: &mut Context<Self>) {
        let pick = cx.subscribe_in(&menu, window, move |this, _, event: &Picked, window, cx| {
            let Picked::Entry { id, .. } = event;
            picked(this, id, window, cx);
        });
        let shut = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.curate.menu = None;
            window.focus(&back, cx);
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.curate.menu = Some(OpenMenu { menu, _subs: vec![pick, shut] });
        cx.notify();
    }

    /// Merge into: the catalog's features, in a picker that finds one by what is typed.
    pub(super) fn open_merge(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(library), Some(row), Some(stops)) = (self.library_held(), self.row_of(&slug), self.curate.stops.get(&slug)) else { return };
        let mut entries = vec![Entry::Search { label: "Find a feature", to_rule: false }];
        entries.extend(merge_targets(library).into_iter().map(|(target, area)| Entry::Row(Row::new(id_of(format!("merge:{}", target.slug)), target.title.clone()).under(area.to_string()))));
        let back = stops[1].clone();
        let at = self.under(&super::curate_view::merge_name(&row.title), window, cx);
        let menu = cx.new(|cx| Dropdown::new("Merge into", entries, at, Anchor::TopRight, true, cx));
        self.open_menu(
            menu,
            back,
            move |this, id, window, cx| {
                if let (Some(into), Some(project)) = (id.strip_prefix("merge:"), this.project_id()) {
                    window.focus(&this.list_focus.clone(), cx);
                    let write = merge_write(&project, &slug, into);
                    this.send(vec![write], Some(into.to_string()), cx);
                }
            },
            window,
            cx,
        );
    }

    /// The area picker of the Confirm editor.
    pub(super) fn open_area_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(library), Some(form)) = (self.library_held(), self.curate.form.as_ref()) else { return };
        let entries: Vec<Entry> = areas_kept(library).into_iter().map(|(slug, title)| Entry::Choice { id: id_of(format!("area:{slug}")), label: title.into(), on: slug == form.area }).collect();
        let back = form.area_focus.clone();
        let at = self.under("Area", window, cx);
        let menu = cx.new(|cx| Dropdown::new("Area", entries, at, Anchor::TopRight, true, cx));
        self.open_menu(
            menu,
            back,
            |this, id, _, cx| {
                if let (Some(area), Some(form)) = (id.strip_prefix("area:"), this.curate.form.as_mut()) {
                    form.area = area.to_string();
                    cx.notify();
                }
            },
            window,
            cx,
        );
    }

    // ---- the split sheet ----------------------------------------------------------------------

    /// Split: the sheet over the list, with the area's features, none ticked.
    pub(super) fn open_split(&mut self, area: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(library) = self.library_held() else { return };
        let title = library.areas.iter().find(|one| one.slug == area).map(|one| one.title.clone()).unwrap_or_default();
        let features: Vec<(String, String, bool)> = kept_in(library, &area).into_iter().map(|row| (row.slug.clone(), row.title.clone(), false)).collect();
        let name = cx.new(|cx| InputState::new(window, cx));
        let subs = vec![cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.submit_split(window, cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        })];
        let rows = features.iter().map(|_| cx.focus_handle()).collect();
        let first = name.clone();
        self.curate.sheet = Some(Sheet { area, title, name, features, rows, frame: cx.focus_handle(), cancel: cx.focus_handle(), go: cx.focus_handle(), _subs: subs });
        window.defer(cx, move |window, cx| first.update(cx, |input, cx| input.focus(window, cx)));
        cx.notify();
    }

    pub(super) fn toggle_split(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(one) = self.curate.sheet.as_mut().and_then(|sheet| sheet.features.get_mut(at)) {
            one.2 = !one.2;
            cx.notify();
        }
    }

    /// Up and Down in the checklist: the next box or the previous.
    pub(super) fn step_split(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(sheet) = &self.curate.sheet else { return false };
        let Some(at) = sheet.rows.iter().position(|row| row.is_focused(window)) else { return false };
        let to = (at as isize + by).clamp(0, sheet.rows.len() as isize - 1) as usize;
        if to != at {
            crate::ui_sounds::moved();
        }
        window.focus(&sheet.rows[to].clone(), cx);
        true
    }

    /// The new area's name is said, it is a slug no area has, and a feature is ticked.
    pub(super) fn split_ready(&self, cx: &gpui::App) -> bool {
        let (Some(sheet), Some(library)) = (&self.curate.sheet, self.library_held()) else { return false };
        let slug = slug_of(sheet.name.read(cx).value().as_ref());
        !slug.is_empty() && !library.areas.iter().any(|area| area.slug == slug) && sheet.features.iter().any(|(_, _, on)| *on)
    }

    /// What stops the sheet from splitting, for the button's tooltip.
    pub(super) fn split_why(&self, cx: &gpui::App) -> &'static str {
        let (Some(sheet), Some(library)) = (&self.curate.sheet, self.library_held()) else { return "" };
        let slug = slug_of(sheet.name.read(cx).value().as_ref());
        if slug.is_empty() {
            "Name the new area"
        } else if library.areas.iter().any(|area| area.slug == slug) {
            "An area has that name"
        } else if !sheet.features.iter().any(|(_, _, on)| *on) {
            "Tick at least one feature"
        } else {
            "Make the area and move the ticked features into it"
        }
    }

    /// Esc or Cancel: nothing is sent and the keys go back to Split.
    pub(super) fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sheet) = self.curate.sheet.take() else { return };
        match self.curate.splits.get(&sheet.area) {
            Some(back) => window.focus(&back.clone(), cx),
            None => self.focus_list(window, cx),
        }
        cx.notify();
    }

    pub(super) fn submit_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.split_ready(cx) {
            return;
        }
        let (Some(sheet), Some(project), Some(library)) = (&self.curate.sheet, self.project_id(), self.library_held()) else { return };
        let writes = split_writes(&project, &areas_kept(library), &sheet.area, sheet.name.read(cx).value().as_ref(), &sheet.ticked());
        self.close_sheet(window, cx);
        self.send(writes, None, cx);
    }

    // ---- the sends ----------------------------------------------------------------------------

    /// The calls made one after another, stopping at the first Fleet refuses, which is said in a toast in Fleet's words. Fleet
    /// is read again after, whatever came of them, and the row the keys follow is `follows`. On mock data there is no Fleet:
    /// the calls are kept for a test to read.
    pub(super) fn send(&mut self, writes: Vec<Write>, follows: Option<String>, cx: &mut Context<Self>) {
        let fleet = match &self.source {
            Source::Live(fleet) => fleet.clone(),
            Source::Mock => {
                self.curate.sent.extend(writes);
                cx.notify();
                return;
            }
        };
        let Some(port) = fleet.read(cx).held().port() else {
            told("Fleet is not connected".into(), cx);
            return;
        };
        self.curate.sends.retain(|task| !task.is_ready());
        let task = cx.spawn(async move |this, cx| {
            let mut kept = true;
            for write in writes {
                let sent = this.update(cx, |_, cx| match &write {
                    Write::Upsert(body) => {
                        let sent = super::wire::upsert(port, body, cx);
                        cx.background_spawn(async move { sent.await.map(|_| ()) })
                    }
                    Write::Change(body) => {
                        let sent = super::wire::change(port, body, cx);
                        cx.background_spawn(async move { sent.await.map(|_| ()) })
                    }
                });
                let Ok(sent) = sent else { return };
                if let Err(why) = sent.await {
                    let _ = this.update(cx, |_, cx| told(why.said, cx));
                    kept = false;
                    break;
                }
            }
            let _ = this.update(cx, |this, cx| {
                if kept {
                    if let Some(follows) = follows {
                        this.close_page(cx);
                        this.cursor = Some(follows);
                    }
                    // The button that was pressed may be gone once Fleet is read again: the keys wait on the list.
                    let (me, handle) = (cx.entity(), this.window);
                    cx.defer(move |cx| {
                        let _ = handle.update(cx, |_, window, cx| me.update(cx, |this, cx| this.focus_list(window, cx)));
                    });
                }
                this.read(cx);
            });
        });
        self.curate.sends.push(task);
    }
}

/// Fleet's own words, in a toast.
fn told(said: String, cx: &mut gpui::App) {
    cx.defer(move |cx| crate::toasts::tell(said, cx));
}
