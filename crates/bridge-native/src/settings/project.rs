//! The values a project, or every project, sets: the picker at the top of the sidebar, and each page's grid of
//! every project beside the project picked. The approved sketch "Native Settings: which project you're editing"
//! (13 Nov 2026).
//!
//! **A project's save writes its whole block.** `POST /projects/project/:id/edit` replaces the project's values, so
//! each write reads the project fresh, changes one key, and sends the rest as it found them; ✕ sends the block
//! without that key. Every project's are Kit's, written whole by `POST /kit/values`, and read back off what
//! `GET /projects/values` says came from Kit. **A kind of review falls through on its own** (#3504): a project sets
//! only the kinds it names.

use gpui::{div, prelude::*, svg, AnyElement, App, Context, FocusHandle, FontWeight, Pixels, Role, SharedString, Window};
use ipc::{LookSet, PrMode, Projects, ResolvedValues, ReviewsSet, ValueLayer, ValuesSet};

use super::entries::Body;
use super::overlay::{Picking, SettingsOverlay};
use super::view::{bone, bounds_of, card, ringed, small, switch_track, tip};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A page of values, under the project picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    PullRequests,
    StartingJobs,
    Look,
}

impl Page {
    pub fn keys(self) -> &'static [Key] {
        match self {
            Page::PullRequests => &[Key::Delivery, Key::PrMode, Key::AutoMerge, Key::Plans, Key::PullRequests, Key::Walks, Key::ReviewModel],
            Page::StartingJobs => &[Key::AgentStarts, Key::Tracker],
            Page::Look => &[Key::Theme, Key::Layout, Key::Density],
        }
    }

    /// The words a search finds it by, in `entries::SECTIONS`.
    pub fn section(self) -> &'static str {
        match self {
            Page::PullRequests => "pull-requests",
            Page::StartingJobs => "starting-jobs",
            Page::Look => "look",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Page::PullRequests => "How a Job opens, reviews and lands its work",
            Page::StartingJobs => "How a Job starts, and the tracker its issues come from",
            Page::Look => "How Bridge is drawn",
        }
    }
}

/// The values a page holds that a project layer sets: a `ValuesSet` key's, or the project's share switch.
pub fn keys_of(body: Body) -> &'static [Key] {
    match body {
        Body::Values(page) => page.keys(),
        Body::Macs => &[Key::Shared],
        _ => &[],
    }
}

/// One value a project sets. `ValuesSet`'s keys, a kind of review each, and `shared_with_macs` on the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Delivery,
    PrMode,
    AutoMerge,
    Plans,
    PullRequests,
    Walks,
    ReviewModel,
    AgentStarts,
    Tracker,
    Theme,
    Layout,
    Density,
    Shared,
}

pub const KEYS: [Key; 13] = [Key::Delivery, Key::PrMode, Key::AutoMerge, Key::Plans, Key::PullRequests, Key::Walks, Key::ReviewModel, Key::AgentStarts, Key::Tracker, Key::Theme, Key::Layout, Key::Density, Key::Shared];

/// A cell's column: every project's, or the project picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Column {
    Every,
    Own,
}

/// What `delivery` holds where a project asks on its first Job.
const ASK: &str = "ask";

impl Key {
    pub fn title(self) -> &'static str {
        match self {
            Key::Delivery => "How work lands",
            Key::PrMode => "Opens pull requests as",
            Key::AutoMerge => "Merging",
            Key::Plans => "Plans",
            Key::PullRequests => "Pull requests",
            Key::Walks => "Walks",
            Key::ReviewModel => "Review model",
            Key::AgentStarts => "Jobs a Job or the Operator starts",
            Key::Tracker => "Tracker",
            Key::Theme => "Theme",
            Key::Layout => "Layout",
            Key::Density => "Spacing",
            Key::Shared => "Share with your other Macs",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Key::Delivery => "Your first answer in a project is kept",
            Key::PrMode => "Draft or ready for review",
            Key::AutoMerge => "When Fleet merges a pull request",
            Key::Plans => "How the Operator reviews a plan",
            Key::PullRequests => "How the Operator reviews a pull request",
            Key::Walks => "How the Operator reviews a walk",
            Key::ReviewModel => "The model an Operator review runs on",
            Key::AgentStarts => "Whether they start at once or wait on your start card",
            Key::Tracker => "This project only",
            Key::Theme => "The colours Bridge is drawn in",
            Key::Layout => "The layout mod Bridge arranges its panels by",
            Key::Density => "How much room Bridge leaves around things",
            Key::Shared => "Its Jobs can be seen and acted on from your other Macs",
        }
    }

    /// Whether every project can set it. The tracker and the share switch are a project's alone.
    pub fn every(self) -> bool {
        !matches!(self, Key::Tracker | Key::Shared)
    }

    /// The words it takes and how each reads, where they are fixed. The review model, the theme and the layout
    /// are read from what this Mac has (`SettingsOverlay::options`).
    fn fixed(self) -> Vec<(String, String)> {
        let pairs: &[(&str, &str)] = match self {
            Key::Delivery => &[(ASK, "Ask each Job")],
            Key::PrMode => &[("ready", "Ready for review"), ("draft", "Draft")],
            Key::AutoMerge => &[("never", "Never"), ("checks-pass", "When checks pass"), ("always", "Always")],
            Key::Plans | Key::PullRequests => &[("off", "Off"), ("review", "Review"), ("review_and_approve", "Review and approve")],
            Key::Walks => &[("off", "Off"), ("review", "Review")],
            Key::AgentStarts => &[("at_once", "Start at once"), ("ask_me", "Ask me")],
            Key::Tracker => &[("linear", "Linear"), ("beads", "Beads")],
            Key::Density => &[("compact", "Compact"), ("default", "Default"), ("roomy", "Roomy")],
            Key::ReviewModel | Key::Theme | Key::Layout | Key::Shared => &[],
        };
        let mut words: Vec<(String, String)> = pairs.iter().map(|(word, said)| (word.to_string(), said.to_string())).collect();
        if self == Key::Delivery {
            words.extend(ipc::DeliveryChoice::OFFERED.iter().map(|one| (crate::new_project::values::delivery_word(*one).to_string(), one.label().to_string())));
        }
        words
    }
}

/// A key's word as the layers come to it, and the layer it came from. A kind of review says its own layer.
pub fn resolved(values: &ResolvedValues, key: Key) -> (Option<String>, ValueLayer) {
    let one = |value: &ipc::ResolvedValue| (value.value.clone().filter(|word| !word.is_empty()), value.from);
    let reviews = values.reviews.as_ref();
    let kind = |word: fn(&ipc::ResolvedReviews) -> (&String, Option<ValueLayer>)| reviews.map_or((Some("off".into()), ValueLayer::Default), |set| {
        let (said, from) = word(set);
        (Some(said.clone()), from.unwrap_or(set.from))
    });
    match key {
        // A project no layer tells how work lands asks on its first Job.
        Key::Delivery => values.delivery.as_ref().map_or((Some(ASK.into()), ValueLayer::Default), |value| {
            let (word, from) = one(value);
            (word.or_else(|| Some(ASK.into())), from)
        }),
        Key::PrMode => one(&values.pr_mode),
        Key::AutoMerge => one(&values.auto_merge),
        Key::ReviewModel => one(&values.review_model),
        Key::AgentStarts => values.agent_starts.as_ref().map_or((Some("at_once".into()), ValueLayer::Default), |value| one(value)),
        Key::Plans => kind(|set| (&set.plans, set.plans_from)),
        Key::PullRequests => kind(|set| (&set.pull_requests, set.pull_requests_from)),
        Key::Walks => kind(|set| (&set.walks, set.walks_from)),
        Key::Tracker => values.tracker.as_ref().map_or((None, ValueLayer::Default), |one| (one.value.as_ref().and_then(|block| block.get("provider").cloned()), one.from)),
        Key::Theme => one(&values.theme),
        Key::Layout => one(&values.layout),
        Key::Density => one(&values.density),
        Key::Shared => (None, ValueLayer::Default),
    }
}

/// What a key the project sets overrides, from the layers below it (#3519): the word, absent where nothing below
/// sets one, and its layer. `None` where the project does not set it.
pub fn inherited(values: &ResolvedValues, key: Key) -> Option<(Option<String>, ValueLayer)> {
    let below = |word: Option<&String>, from: Option<ValueLayer>| {
        let from = from.unwrap_or_default();
        (word.cloned().filter(|_| from != ValueLayer::Default), from)
    };
    // A kind of review says its own (#3524), where that kind is the project's.
    let kind = |own: Option<ValueLayer>, word: Option<&String>, from: Option<ValueLayer>| (own == Some(ValueLayer::Project)).then(|| below(word, from));
    let reviews = values.reviews.as_ref();
    match key {
        Key::Plans => return reviews.and_then(|set| kind(set.plans_from, set.plans_inherited.as_ref(), set.plans_inherited_from)),
        Key::PullRequests => return reviews.and_then(|set| kind(set.pull_requests_from, set.pull_requests_inherited.as_ref(), set.pull_requests_inherited_from)),
        Key::Walks => return reviews.and_then(|set| kind(set.walks_from, set.walks_inherited.as_ref(), set.walks_inherited_from)),
        _ => {}
    }
    let plain = match key {
        Key::PrMode => Some(&values.pr_mode),
        Key::AutoMerge => Some(&values.auto_merge),
        Key::ReviewModel => Some(&values.review_model),
        Key::Theme => Some(&values.theme),
        Key::Layout => Some(&values.layout),
        Key::Density => Some(&values.density),
        Key::AgentStarts => values.agent_starts.as_ref(),
        Key::Delivery => values.delivery.as_ref(),
        Key::Plans | Key::PullRequests | Key::Walks | Key::Tracker | Key::Shared => None,
    }?;
    (plain.from == ValueLayer::Project).then(|| below(plain.inherited.as_ref(), plain.inherited_from))
}

/// Why every project's cell beside a key the project sets cannot be changed from here: the word it shows is a
/// repository's or this Mac's, under Kit, so changing Kit would change nothing the project shows.
pub fn locked(own: Option<&ResolvedValues>, key: Key) -> Option<&'static str> {
    match inherited(own?, key)?.1 {
        ValueLayer::Repository => Some("Set by the repository"),
        ValueLayer::Machine => Some("Set on this Mac"),
        _ => None,
    }
}

/// Whether the project's own block sets the key, as the layers say: a kind of review by its own layer.
pub fn set_here(values: &ResolvedValues, key: Key) -> bool {
    resolved(values, key).1 == ValueLayer::Project
}

/// A block with one key set to `word`, or taken out where `word` is `None`, every other key kept. A kind of review
/// is set alone, and a `reviews` or `look` left naming nothing is dropped.
pub fn with(own: Option<&ValuesSet>, key: Key, word: Option<&str>) -> ValuesSet {
    let mut set = own.cloned().unwrap_or_default();
    let word = word.filter(|word| !word.is_empty()).map(str::to_string);
    let mut reviews = set.reviews.clone().unwrap_or_default();
    let mut look = set.look.clone().unwrap_or_default();
    match key {
        Key::Delivery => set.delivery = word,
        Key::PrMode => set.pr_mode = word.as_deref().and_then(PrMode::from_wire),
        Key::AutoMerge => set.auto_merge = word,
        Key::ReviewModel => set.review_model = word,
        Key::AgentStarts => set.agent_starts = word,
        Key::Plans => reviews.plans = word,
        Key::PullRequests => reviews.pull_requests = word,
        Key::Walks => reviews.walks = word,
        Key::Theme => look.theme = word,
        Key::Layout => look.layout = word,
        Key::Density => look.density = word,
        Key::Tracker => {
            set.tracker = word.map(|provider| {
                let mut block = set.tracker.clone().unwrap_or_default();
                block.insert("provider".into(), provider);
                block
            });
        }
        Key::Shared => {}
    }
    if matches!(key, Key::Plans | Key::PullRequests | Key::Walks) {
        set.reviews = (reviews != ReviewsSet::default()).then_some(reviews);
    }
    if matches!(key, Key::Theme | Key::Layout | Key::Density) {
        set.look = (look != LookSet::default()).then_some(look);
    }
    set
}

/// What Kit sets, read back off every project's values: each key whose word came from Kit. Kit names no tracker.
pub fn kit_own(every: &ResolvedValues) -> ValuesSet {
    let mut set = ValuesSet::default();
    for key in KEYS.into_iter().filter(|key| key.every()) {
        let (word, from) = resolved(every, key);
        if from == ValueLayer::Kit {
            set = with(Some(&set), key, word.as_deref());
        }
    }
    set
}

/// How many values a project's own block sets: each key, each look key and each kind of review.
pub fn set_count(own: Option<&ValuesSet>) -> usize {
    let Some(own) = own else { return 0 };
    let look = own.look.as_ref().map_or(0, |look| [&look.theme, &look.layout, &look.density].into_iter().filter(|one| one.is_some()).count());
    let reviews = own.reviews.as_ref().map_or(0, |set| [&set.plans, &set.pull_requests, &set.walks].into_iter().filter(|one| one.is_some()).count());
    let plain = [own.pr_mode.is_some(), own.auto_merge.is_some(), own.review_model.is_some(), own.tracker.is_some(), own.agent_starts.is_some(), own.delivery.is_some()].into_iter().filter(|one| *one).count();
    look + reviews + plain
}

/// Whether the project is shared with his other Macs, as a list of projects says. `None` where it is not there, or
/// is shown as `unread`: such a project has no switch.
pub fn shared_of(projects: &Projects, id: &str) -> Option<bool> {
    projects.projects.iter().find(|one| one.id == id).filter(|one| one.unread.is_none()).map(|one| one.shared_with_macs)
}

/// The project picked, by id and name; `None` is Every project.
pub type Editing = Option<(String, String)>;

/// The keys a page's grid draws, with Every project picked or a project: one Kit cannot set is a project's only.
pub fn rows(keys: &[Key], project: bool) -> Vec<Key> {
    keys.iter().copied().filter(|key| project || key.every()).collect()
}

/// The cells of a grid in the order Tab reaches them, row by row: every project's, then the project's.
pub fn cells(keys: &[Key], project: bool) -> Vec<Vec<(Key, Column)>> {
    rows(keys, project)
        .into_iter()
        .map(|key| {
            let mut row = Vec::new();
            if key.every() {
                row.push((key, Column::Every));
            }
            if project {
                row.push((key, Column::Own));
            }
            row
        })
        .collect()
}

/// Where the keys go from a cell on an arrow: along its row, or to the nearest cell in the row above or below.
pub fn moved(grid: &[Vec<(Key, Column)>], from: (Key, Column), arrow: &str) -> Option<(Key, Column)> {
    let (row, at) = grid.iter().enumerate().find_map(|(row, cells)| cells.iter().position(|one| *one == from).map(|at| (row, at)))?;
    match arrow {
        "left" => at.checked_sub(1).map(|at| grid[row][at]),
        "right" => grid[row].get(at + 1).copied(),
        "up" | "down" => {
            let next = if arrow == "up" { row.checked_sub(1)? } else { row + 1 };
            let cells = grid.get(next)?;
            cells.iter().find(|one| one.1 == from.1).or_else(|| cells.last()).copied()
        }
        _ => None,
    }
}

impl SettingsOverlay {
    /// A project's name as Fleet last listed it, or its id before then.
    pub fn project_name(&self, id: &str) -> String {
        self.scope.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == id)).map_or_else(|| id.to_string(), |one| one.name.clone())
    }

    /// A key's words and how each reads: fixed ones, or what this Mac has for the review model, the theme and
    /// the layout. A word in force that is not among them is kept, as it reads.
    pub fn options(&self, key: Key, now: Option<&str>, cx: &App) -> Vec<(String, String)> {
        let mut words = match key {
            Key::ReviewModel => match &self.models.held {
                Some(Ok(choices)) if !choices.offered.is_empty() => choices.offered.iter().filter(|one| one.on).map(|one| (one.name.clone(), one.display.clone())).collect(),
                Some(Ok(choices)) => choices.models.iter().map(|one| (one.clone(), one.clone())).collect(),
                _ => Vec::new(),
            },
            Key::Theme => self.theme_choices(cx).into_iter().map(|(_, one)| (one.id, one.title)).collect(),
            Key::Layout => self.wire.mods(cx).into_iter().filter(|row| row.kind == Some(ipc::ModKind::Layout) && row.valid).map(|row| (row.name.clone(), row.name)).collect(),
            _ => key.fixed(),
        };
        if let Some(now) = now.filter(|now| !words.iter().any(|(word, _)| word == now)) {
            words.push((now.to_string(), now.to_string()));
        }
        words
    }

    /// How a word reads in a cell.
    fn said(&self, key: Key, word: Option<&str>, cx: &App) -> String {
        match (key, word) {
            (Key::Tracker, None) => "The forge's own".into(),
            (_, None) => String::new(),
            (_, Some(word)) => self.options(key, Some(word), cx).into_iter().find(|(one, _)| one == word).map_or_else(|| word.to_string(), |(_, said)| said),
        }
    }

    /// The keys the page shown draws in its grid, and the cells Tab reaches in it.
    pub fn grid_of(&self, cx: &App) -> Vec<Vec<(Key, Column)>> {
        let Some(shown) = self.shown(cx) else { return Vec::new() };
        let project = self.scope.project.is_some();
        // The share switch is a project's; with Every project picked, Macs draws only this Mac's.
        let keys = keys_of(shown.body);
        if keys.is_empty() || (shown.body == Body::Macs && !project) {
            return Vec::new();
        }
        cells(keys, project)
    }

    /// The grid's keys, in the order Tab reaches them, once what they show has been read.
    pub fn grid_stops(&self, cx: &App) -> Vec<FocusHandle> {
        if self.scope.every.is_none() || (self.scope.project.is_some() && self.scope.values.is_none()) {
            return Vec::new();
        }
        self.grid_of(cx).into_iter().flatten().filter_map(|cell| self.scope.cells.get(&cell).cloned()).collect()
    }

    /// The cell holding the keys, if one does.
    pub fn cell_focused(&self, window: &Window) -> Option<(Key, Column)> {
        self.scope.cells.iter().find(|(_, handle)| handle.is_focused(window)).map(|(cell, _)| *cell)
    }

    fn cell_name(&self, (key, column): (Key, Column)) -> String {
        match (column, &self.scope.project) {
            (Column::Own, Some((_, name))) => format!("{} for {name}", key.title()),
            _ => format!("{} for every project", key.title()),
        }
    }

    /// Open a cell's choices under it; picking one sets it in that column.
    pub fn open_cell(&mut self, cell: (Key, Column), from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.live || self.scope.saving.is_some() || cell.0 == Key::Shared {
            return;
        }
        if cell.1 == Column::Every && self.scope.project.is_some() && locked(self.scope.values.as_ref(), cell.0).is_some() {
            return;
        }
        let values = match cell.1 {
            Column::Every => self.scope.every.as_ref(),
            Column::Own => self.scope.values.as_ref(),
        };
        let Some(values) = values else { return };
        let (now, from) = resolved(values, cell.0);
        let own = cell.1 == Column::Every || from == ValueLayer::Project;
        let ticked = now.filter(|_| own);
        let options = self.options(cell.0, ticked.as_deref(), cx).into_iter().map(|(word, said)| (SharedString::from(said), ticked.as_deref() == Some(word.as_str()))).collect();
        let name = self.cell_name(cell);
        let at = bounds_of(window, cx, Role::ComboBox, &name).map(|bounds| bounds.bottom_left()).unwrap_or_default();
        self.open_choice(Picking::Cell(cell.0, cell.1), name.into(), options, at, from_keys, window, cx);
    }

    /// The `n`th option picked for a cell.
    pub fn chose_cell(&mut self, (key, column): (Key, Column), n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let values = match column {
            Column::Every => self.scope.every.as_ref(),
            Column::Own => self.scope.values.as_ref(),
        };
        let now = values.and_then(|values| resolved(values, key).0);
        let Some((word, _)) = self.options(key, now.as_deref(), cx).into_iter().nth(n) else { return };
        self.write_cell((key, column), Some(word), window, cx);
    }

    /// Clear a value the project sets, so it follows every project again.
    pub fn clear_cell(&mut self, key: Key, window: &mut Window, cx: &mut Context<Self>) {
        if self.scope.values.as_ref().is_some_and(|values| set_here(values, key)) {
            self.write_cell((key, Column::Own), None, window, cx);
        }
    }
}

// ---- drawn ----------------------------------------------------------------------------------------

/// The picker at the top of the sidebar: Editing, over the project picked, with its glyph and a chevron.
pub fn picker(this: &SettingsOverlay, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let name = this.scope.project.as_ref().map_or("Every project".to_string(), |(_, name)| name.clone());
    let glyph = if this.scope.project.is_some() { "layers" } else { "globe" };
    let focus = &this.scope.picker;
    let button = div()
        .id("settings-editing")
        .track_focus(focus)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
        .mx(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::ACCENT))
        .bg(look.color(token::BG_RAISED))
        .cursor_pointer()
        .child(svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(token::ACCENT)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Editing"))
                .child(
                    div()
                        .truncate()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(name.clone())),
                ),
        )
        .child(svg().flex_none().path(icon_path("chevron-down")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)))
        .tooltip(tip("⌘P"))
        .on_click(cx.listener(|this, _, window, cx| this.open_picker(false, window, cx)));
    labelled(ringed(look, button, focus, window), Role::ComboBox, format!("Editing {name}")).into_any_element()
}

/// The menu the picker opens: Every project, each project, New project, then a Delete for the project being edited, so
/// another is deleted by picking it first. Every project has none, and nor does a copy of another Mac's: that Mac keeps it.
pub fn picker_rows(picked: &Editing, projects: Option<&Projects>) -> Vec<crate::dropdown::Entry> {
    use crate::dropdown::{Entry, Row};
    let every = picked.is_none();
    let mut rows = vec![Entry::Row(Row::new("every", "Every project").icon(if every { "check" } else { "globe" }).detail("defaults")), Entry::Rule];
    let shown = projects.map(|all| all.projects.as_slice()).unwrap_or_default().iter().take(super::overlay::AT.len()).enumerate().collect::<Vec<_>>();
    for (n, one) in &shown {
        let on = picked.as_ref().is_some_and(|(id, _)| *id == one.id);
        let mut row = Row::new(super::overlay::AT[*n], one.name.clone()).icon(if on { "check" } else { "layers" });
        if on {
            row = row.detail(format!("{} set here", set_count(one.values.as_ref())));
        }
        if let Some(why) = &one.unread {
            row = row.off(why.clone());
        }
        rows.push(Entry::Row(row));
    }
    rows.push(Entry::Rule);
    rows.push(Entry::Row(Row::new("new-project", "New project").icon("plus").key("⌘⇧N")));
    let edited = shown.iter().find(|(_, one)| picked.as_ref().is_some_and(|(id, _)| *id == one.id) && one.mac.is_none());
    if let Some((_, one)) = edited {
        rows.push(Entry::Rule);
        rows.push(Entry::Act { id: "delete", label: format!("Delete {}", one.name).into(), icon: Some("trash"), danger: true });
    }
    rows
}

/// What the confirm says: the project, and the tasks that go with it. None says no more than the project.
pub fn delete_sentence(name: &str, tasks: usize) -> String {
    match tasks {
        0 => format!("Delete {name}?"),
        1 => format!("Delete {name}? Its 1 task goes with it."),
        n => format!("Delete {name}? Its {n} tasks go with it."),
    }
}

/// Whether a page this wide lays its grid out in columns; narrower, each setting's words sit over its cells.
fn wide(page: Pixels, columns: usize, look: &Look) -> bool {
    let cell = look.length(token::W_MENU) + look.length(token::SPACE_4);
    page >= look.length(token::W_GUIDE_LIST_MIN) + cell * columns as f32
}

/// The width a page's card holds inside, as the overlay is laid out now.
fn page_inside(look: &Look, window: &Window, cx: &App) -> Pixels {
    let room = crate::room::of(window, cx);
    room.size.width * 0.9 - look.length(token::GAP_PANEL) * 3. - look.length(token::W_SETTINGS_INDEX) - look.length(token::PAD_CARD) * 2.
}

fn header(look: &Look, glyph: Option<&str>, said: String, accent: bool) -> gpui::Div {
    let ink = if accent { look.color(token::ACCENT) } else { look.color(token::FG_MUTED) };
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .min_w_0()
        .pb(look.length(token::SPACE_2))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(ink)
        .children(glyph.map(|glyph| svg().flex_none().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(ink)))
        .child(div().truncate().child(SharedString::from(said)))
}

/// A page's grid of values, in a card under the page's name: every project beside the project picked, or every
/// project's alone. Bones stand in its cells until both are read.
pub fn grid(this: &SettingsOverlay, title: &str, description: Option<&str>, keys: &[Key], look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let project = this.scope.project.clone();
    let columns = if project.is_some() { 2 } else { 1 };
    let wide = wide(page_inside(look, window, cx), columns, look);
    let cell_w = look.length(token::W_MENU);
    let gap = look.length(token::SPACE_4);
    let place = |cell: gpui::Div| if wide { cell.flex_none().w(cell_w) } else { cell.flex_1().min_w_0() };
    let mut heads = div().flex().gap(gap);
    if wide {
        heads = heads.child(header(look, None, "Setting".into(), false).flex_1());
    }
    heads = heads.child(place(header(look, Some("globe"), "Every project".into(), false)));
    if let Some((_, name)) = &project {
        heads = heads.child(place(header(look, Some("layers"), name.clone(), true)));
    }
    let rows = rows(keys, project.is_some()).into_iter().map(|key| {
        let words = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(key.title()))
                    .children(learned(this, key, look)),
            )
            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(key.description()));
        // Wide, the words are the row's first column; narrow, they sit over its cells.
        let (mut cells, above) = if wide { (div().flex().items_center().gap(gap).child(words.flex_1()), None) } else { (div().flex().items_center().gap(gap), Some(words)) };
        cells = cells.child(place(div().child(cell(this, (key, Column::Every), look, window, cx))));
        if project.is_some() {
            cells = cells.child(place(div().child(cell(this, (key, Column::Own), look, window, cx))));
        }
        let row = div().id(SharedString::from(format!("values-row-{key:?}"))).flex().flex_col().gap(look.length(token::SPACE_2)).py(look.length(token::SPACE_3)).border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE));
        let row = row.children(above).child(cells);
        labelled(row, Role::Row, key.title()).into_any_element()
    });
    let refused = this.scope.refused.clone().map(|said| labelled(super::view::refusal(look, said.clone()).id("values-refused"), Role::Alert, said).into_any_element());
    let shared_refused = this.scope.share_refused.clone().map(|said| labelled(super::view::refusal(look, said.clone()).id("project-shared-refused"), Role::Alert, said).into_any_element());
    let table = labelled(div().id(SharedString::from(format!("values-grid-{title}"))).flex().flex_col().child(heads).children(rows), Role::Table, title.to_string());
    let mut body: Vec<AnyElement> = Vec::new();
    if let Some(description) = description {
        body.push(small(look, description, look.color(token::FG_MUTED)).into_any_element());
    }
    body.push(table.into_any_element());
    body.extend(refused);
    body.extend(shared_refused);
    labelled(card(look, SharedString::from(format!("values-card-{title}")), title, None, body), Role::Region, title.to_string()).into_any_element()
}

/// "Learned" beside how work lands, while the project's word is the one Fleet learned from his first answer.
fn learned(this: &SettingsOverlay, key: Key, look: &Look) -> Option<AnyElement> {
    let own = this.scope.values.as_ref().filter(|_| key == Key::Delivery && this.scope.project.is_some())?;
    own.delivery.as_ref().filter(|value| value.learned)?;
    let ink = look.color(token::STATUS_AWAITING_REVIEW);
    let chip = div()
        .id("value-learned-Delivery")
        .flex_none()
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .border(look.length(token::BORDER_WIDTH))
        .border_color(ink)
        .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .text_color(ink)
        .tooltip(tip(LEARNED_WHY))
        .child("Learned");
    Some(labelled(chip, Role::Image, format!("Learned: {LEARNED_WHY}")).into_any_element())
}

const LEARNED_WHY: &str = "Fleet learned this from your first answer";

/// A page of values: its grid under its name.
pub fn render(this: &SettingsOverlay, page: Page, label: &str, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> Vec<AnyElement> {
    vec![grid(this, label, Some(page.description()), page.keys(), look, window, cx)]
}

/// One cell: a control that opens its choices, an empty place where every project has none, the share switch, or
/// a bone while its values are read.
fn cell(this: &SettingsOverlay, (key, column): (Key, Column), look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let tall = look.length(token::H_CONTROL_LG);
    if column == Column::Every && !key.every() {
        return div().h(tall).into_any_element();
    }
    // A key the project sets: every project's cell says what it overrides.
    let picked = this.scope.project.is_some();
    let overridden = (column == Column::Every && picked).then(|| this.scope.values.as_ref().and_then(|own| inherited(own, key))).flatten();
    if key == Key::Shared {
        return shared(this, look, window, cx).unwrap_or_else(|| bone(look, look.length(token::SPACE_12), look.length(token::H_BADGE)).into_any_element());
    }
    let values = match column {
        Column::Every => this.scope.every.as_ref(),
        Column::Own => this.scope.values.as_ref(),
    };
    let Some(values) = values else { return bone(look, look.length(token::W_GUIDE_LIST_MIN), tall).into_any_element() };
    let Some(focus) = this.scope.cells.get(&(key, column)) else { return div().into_any_element() };
    // Nothing below the project sets it: the default's word, muted. A repository's or this Mac's word is shown and
    // cannot be changed here.
    let defaulted = matches!(overridden, Some((None, ValueLayer::Default)));
    let lock = if overridden.is_some() { locked(this.scope.values.as_ref(), key) } else { None };
    let (word, from) = match overridden {
        Some((None, ValueLayer::Default)) => (resolved(values, key).0, ValueLayer::Default),
        Some((word, from)) => (word, from),
        None => resolved(values, key),
    };
    let set = column == Column::Own && from == ValueLayer::Project;
    // A project cell that follows every project says so; the tracker, which no layer but a project's names, says
    // what it comes to.
    let same = column == Column::Own && !set;
    let said = if same && key.every() { "Same as every project".to_string() } else { this.said(key, word.as_deref(), cx) };
    let off = !this.live || this.scope.saving.is_some() || lock.is_some();
    let name = this.cell_name((key, column));
    let ink = if same || off || defaulted { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) };
    let control = div()
        .id(SharedString::from(format!("value-{key:?}-{column:?}")))
        .track_focus(focus)
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(tall)
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(ink)
        .map(|control| {
            if set {
                control.border_color(look.color(token::ACCENT)).bg(look.color(token::ACCENT_FAINT)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            } else if same {
                control.border_dashed().border_color(look.color(token::BORDER_DEFAULT))
            } else {
                control.border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_SUNKEN))
            }
        })
        .when(!off, |control| control.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.open_cell((key, column), false, window, cx))))
        .when(column == Column::Every && picked, |control| control.tooltip(tip(lock.unwrap_or(if defaulted { "Armada's default" } else { "Changes every project" }))))
        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(said)));
    let control = if set {
        let project = this.scope.project.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
        let clear = div()
            .id(SharedString::from(format!("value-clear-{key:?}")))
            .flex_none()
            .size(look.length(token::SPACE_4))
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_SM))
            .when(!off, |clear| clear.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .tooltip(tip(format!("Clears {project}'s value, so it follows every project again")))
            .child(svg().path(icon_path("x")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
            .when(!off, |clear| {
                clear.on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.clear_cell(key, window, cx);
                }))
            });
        control.child(labelled(clear, Role::Button, format!("Clear {} for {project}", key.title())))
    } else if lock.is_some() {
        control
    } else {
        control.child(svg().flex_none().path(icon_path("chevron-down")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)))
    };
    labelled(ringed(look, control, focus, window), Role::ComboBox, name).into_any_element()
}

/// "Share with your other Macs": the project's switch. None for a project Fleet has not listed yet, or shows as
/// `unread`.
fn shared(this: &SettingsOverlay, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> Option<AnyElement> {
    let (id, _) = this.scope.project.as_ref()?;
    let on = shared_of(this.scope.projects.as_ref()?, id)?;
    let off = !this.live || this.scope.saving.is_some();
    let focus = this.scope.cells.get(&(Key::Shared, Column::Own))?;
    let row = div()
        .id("project-shared")
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_LG))
        .px(look.length(token::SPACE_2))
        .mx(-look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .child(switch_track(look, on, off))
        .track_focus(focus)
        .when(!off, |row| row.cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.flip_shared(window, cx))));
    Some(labelled(ringed(look, row, focus, window), Role::Switch, Key::Shared.title()).into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{Project, ResolvedReviews, ResolvedValue};

    fn armada() -> ValuesSet {
        ValuesSet {
            delivery: Some("auto_land".into()),
            auto_merge: Some("always".into()),
            reviews: Some(ReviewsSet { plans: Some("review_and_approve".into()), ..ReviewsSet::default() }),
            look: Some(LookSet { density: Some("roomy".into()), ..LookSet::default() }),
            ..ValuesSet::default()
        }
    }

    #[test]
    fn every_value_a_project_sets_is_on_one_page_under_the_project() {
        // Every key `ValuesSet` has, named so a key added to it fails here until a page holds it. `workflows_off` is
        // switched a row at a time on the Workflows page, through `set_project_workflow`, and is on no values page.
        let ValuesSet { pr_mode: _, auto_merge: _, review_model: _, look: _, tracker: _, reviews: _, agent_starts: _, delivery: _, workflows_off: _ } = ValuesSet::default();
        let keys: Vec<Key> = [Body::Values(Page::PullRequests), Body::Values(Page::StartingJobs), Body::Values(Page::Look), Body::Macs].into_iter().flat_map(|body| keys_of(body).iter().copied()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "no key is on two pages");
        assert_eq!(sorted, KEYS.to_vec(), "every key is on a page");
        assert!(keys_of(Body::Models).is_empty() && keys_of(Body::Theme).is_empty() && keys_of(Body::Layout).is_empty());
    }

    #[test]
    fn a_set_cell_is_cleared_alone_and_the_rest_of_the_block_kept() {
        let cleared = with(Some(&armada()), Key::AutoMerge, None);
        assert_eq!(cleared, ValuesSet { auto_merge: None, ..armada() });
        let cleared = with(Some(&armada()), Key::Density, None);
        assert_eq!(cleared.look, None, "a look naming nothing is dropped");
        assert_eq!(cleared.delivery.as_deref(), Some("auto_land"));
        let set = with(Some(&armada()), Key::PrMode, Some("draft"));
        assert_eq!(set, ValuesSet { pr_mode: PrMode::from_wire("draft"), ..armada() });
    }

    #[test]
    fn a_kind_of_review_is_set_and_cleared_alone() {
        let set = with(Some(&armada()), Key::Walks, Some("review"));
        assert_eq!(set.reviews, Some(ReviewsSet { plans: Some("review_and_approve".into()), pull_requests: None, walks: Some("review".into()) }), "the other two still fall through");
        let cleared = with(Some(&set), Key::Plans, None);
        assert_eq!(cleared.reviews, Some(ReviewsSet { walks: Some("review".into()), ..ReviewsSet::default() }));
        let none = with(Some(&cleared), Key::Walks, None);
        assert_eq!(none.reviews, None, "a block naming no kind is not sent");
        assert_eq!(none.auto_merge.as_deref(), Some("always"));
    }

    fn values(plans: ValueLayer, pull_requests: ValueLayer) -> ResolvedValues {
        ResolvedValues {
            pr_mode: ResolvedValue { value: Some("draft".into()), from: ValueLayer::Kit, ..ResolvedValue::default() },
            auto_merge: ResolvedValue { value: Some("always".into()), from: ValueLayer::Project, ..ResolvedValue::default() },
            delivery: Some(ResolvedValue { value: Some("auto_land".into()), from: ValueLayer::Project, ..ResolvedValue::default() }),
            reviews: Some(ResolvedReviews {
                plans: "review_and_approve".into(),
                pull_requests: "review".into(),
                walks: "off".into(),
                from: ValueLayer::Project,
                plans_from: Some(plans),
                pull_requests_from: Some(pull_requests),
                walks_from: Some(ValueLayer::Default),
                plans_inherited: None,
                plans_inherited_from: None,
                pull_requests_inherited: None,
                pull_requests_inherited_from: None,
                walks_inherited: None,
                walks_inherited_from: None,
            }),
            ..ResolvedValues::default()
        }
    }

    #[test]
    fn a_cell_is_set_where_the_project_layer_said_it_and_same_where_it_falls_through() {
        let values = values(ValueLayer::Project, ValueLayer::Kit);
        assert!(set_here(&values, Key::Plans));
        assert!(!set_here(&values, Key::PullRequests), "each kind says its own layer, not the block's");
        assert!(!set_here(&values, Key::Walks));
        assert!(set_here(&values, Key::AutoMerge) && set_here(&values, Key::Delivery));
        assert!(!set_here(&values, Key::PrMode));
        assert_eq!(resolved(&ResolvedValues::default(), Key::Delivery), (Some(ASK.into()), ValueLayer::Default), "no layer says: each Job asks");
    }

    #[test]
    fn every_projects_block_is_what_came_from_kit() {
        let every = values(ValueLayer::Kit, ValueLayer::Default);
        let kit = kit_own(&every);
        assert_eq!(kit.pr_mode, PrMode::from_wire("draft"));
        assert_eq!(kit.reviews, Some(ReviewsSet { plans: Some("review_and_approve".into()), ..ReviewsSet::default() }));
        assert_eq!((kit.auto_merge, kit.delivery), (None, None), "a project's word is not Kit's");
    }

    #[test]
    fn every_project_draws_one_column_and_leaves_out_what_only_a_project_sets() {
        let one = cells(Page::StartingJobs.keys(), false);
        assert_eq!(one, vec![vec![(Key::AgentStarts, Column::Every)]], "Kit names no tracker");
        let two = cells(Page::StartingJobs.keys(), true);
        assert_eq!(two, vec![vec![(Key::AgentStarts, Column::Every), (Key::AgentStarts, Column::Own)], vec![(Key::Tracker, Column::Own)]], "the tracker's every project cell is empty");
        assert_eq!(cells(keys_of(Body::Macs), true), vec![vec![(Key::Shared, Column::Own)]]);
    }

    #[test]
    fn beside_a_key_the_project_sets_every_projects_cell_says_what_it_overrides_and_is_blank_where_nothing_does() {
        let mut own = values(ValueLayer::Project, ValueLayer::Kit);
        own.auto_merge.inherited = Some("checks-pass".into());
        own.auto_merge.inherited_from = Some(ValueLayer::Kit);
        own.delivery = Some(ResolvedValue { value: Some("auto_land".into()), from: ValueLayer::Project, learned: true, inherited: None, inherited_from: Some(ValueLayer::Default) });
        assert_eq!(inherited(&own, Key::AutoMerge), Some((Some("checks-pass".into()), ValueLayer::Kit)));
        assert_eq!(inherited(&own, Key::Delivery), Some((None, ValueLayer::Default)), "nothing below sets it: drawn unset");
        assert_eq!(inherited(&own, Key::PrMode), None, "a key the project does not set overrides nothing");
        if let Some(reviews) = own.reviews.as_mut() {
            reviews.plans_inherited_from = Some(ValueLayer::Default);
        }
        assert_eq!(inherited(&own, Key::Plans), Some((None, ValueLayer::Default)), "a kind nothing below sets reads off from the default: unset");
        assert_eq!(inherited(&own, Key::PullRequests), None, "a kind the project does not set");
        assert_eq!(locked(Some(&own), Key::AutoMerge), None, "Kit's word is changed from here");
        own.auto_merge.inherited_from = Some(ValueLayer::Machine);
        assert_eq!(locked(Some(&own), Key::AutoMerge), Some("Set on this Mac"));
        own.auto_merge.inherited_from = Some(ValueLayer::Repository);
        assert_eq!(locked(Some(&own), Key::AutoMerge), Some("Set by the repository"));
        let grid = cells(&[Key::AutoMerge, Key::Delivery, Key::Plans], true);
        assert_eq!(grid, vec![vec![(Key::AutoMerge, Column::Every), (Key::AutoMerge, Column::Own)], vec![(Key::Delivery, Column::Every), (Key::Delivery, Column::Own)], vec![(Key::Plans, Column::Every), (Key::Plans, Column::Own)]]);
    }


    #[test]
    fn the_arrows_move_along_a_row_and_to_the_same_column_above_and_below() {
        let grid = cells(Page::StartingJobs.keys(), true);
        assert_eq!(moved(&grid, (Key::AgentStarts, Column::Every), "right"), Some((Key::AgentStarts, Column::Own)));
        assert_eq!(moved(&grid, (Key::AgentStarts, Column::Every), "left"), None, "past the first column the sidebar takes the keys");
        assert_eq!(moved(&grid, (Key::AgentStarts, Column::Own), "down"), Some((Key::Tracker, Column::Own)));
        assert_eq!(moved(&grid, (Key::AgentStarts, Column::Every), "down"), Some((Key::Tracker, Column::Own)), "a row with no such column takes its nearest");
        assert_eq!(moved(&grid, (Key::Tracker, Column::Own), "down"), None);
    }

    #[test]
    fn the_picker_lists_every_project_each_project_new_project_and_a_delete_for_the_one_edited() {
        use crate::dropdown::Entry;
        let one = |id: &str, name: &str, values: Option<ValuesSet>, unread: Option<&str>| Project { id: id.into(), name: name.into(), values, unread: unread.map(String::from), ..Project::default() };
        let projects = Projects { projects: vec![one("armada", "Armada", Some(armada()), None), one("ledger", "Ledger", None, Some("the file is gone"))], loose: Vec::new(), left_out: Vec::new() };
        let rows = picker_rows(&Some(("armada".into(), "Armada".into())), Some(&projects));
        let said: Vec<(String, Option<String>, bool)> = rows
            .iter()
            .filter_map(|entry| match entry {
                Entry::Row(row) => Some((row.label.to_string(), row.detail.as_ref().map(|one| one.to_string()), row.off.is_some())),
                _ => None,
            })
            .collect();
        assert_eq!(
            said,
            [("Every project".into(), Some("defaults".into()), false), ("Armada".into(), Some("4 set here".into()), false), ("Ledger".into(), None, true), ("New project".into(), None, false)]
        );
        assert_eq!(rows.iter().filter(|entry| matches!(entry, Entry::Rule)).count(), 3);
        let deletes: Vec<&str> = rows.iter().filter_map(|entry| if let Entry::Act { label, danger: true, .. } = entry { Some(label.as_ref()) } else { None }).collect();
        assert_eq!(deletes, ["Delete Armada"], "only the project being edited");
        let ledger = picker_rows(&Some(("ledger".into(), "Ledger".into())), Some(&projects));
        assert!(ledger.iter().any(|entry| matches!(entry, Entry::Act { label, .. } if label.as_ref() == "Delete Ledger")), "an unread project can still be deleted");
    }

    #[test]
    fn a_project_shows_the_share_switch_as_listed_and_an_unread_one_has_none() {
        let one = |id: &str, shared: bool, unread: Option<&str>| Project { id: id.into(), shared_with_macs: shared, unread: unread.map(String::from), ..Project::default() };
        let projects = Projects { projects: vec![one("web", true, None), one("api", false, None), one("lost", true, Some("the file is gone"))], loose: Vec::new(), left_out: Vec::new() };
        assert_eq!(shared_of(&projects, "web"), Some(true));
        assert_eq!(shared_of(&projects, "api"), Some(false));
        assert_eq!(shared_of(&projects, "lost"), None, "a project shown as unread has no switch");
        assert_eq!(shared_of(&projects, "none"), None);
    }
}
