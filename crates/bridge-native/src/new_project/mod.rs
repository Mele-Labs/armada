//! The New project sheet, as the owner approved it on 12 Nov: a name, the repositories or folders it holds,
//! a repository cloned or made new, where its file is kept, and its settings, then Create project (⌘↩).
//!
//! The title bar's switcher asks for it (`Asked::NewProject`) and the dispatch screen draws it over the window.
//! Create clones and makes any repository first, then `POST /projects`, and tells the screen through [`Made`]:
//! the screen scopes the window to the project and starts a Job in it as ⌘T does. This file is its state and
//! keys; `view.rs` draws it, `values.rs` holds its settings and `wire.rs` what it reads and sends.

pub mod values;
mod view;
pub mod wire;

use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    point, Anchor, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Global, KeyDownEvent, SharedString,
    Subscription, Task, Window,
};
use gpui_component::input::{InputEvent, InputState};
use ipc::{ProjectMember, ProjectPlace, Projects, RepositoryAreas, RepositorySummary, RunCloud, RunPlace};

use crate::dropdown::{Dropdown, Entry, Picked};
use crate::keymap::Act;
use crate::look::{token, ActiveLook};
use values::Key;
use wire::Wire;

const KEEP_NAME: &str = "new-project:name";
const KEEP_CLONE: &str = "new-project:clone";
const KEEP_NEW: &str = "new-project:new-repository";
const KEEP_SYNCED: &str = "new-project:synced-folder";

/// Fleet made the project: the window goes to it, and a Job starts in `repository` on its empty thread.
#[derive(Clone, Debug)]
pub struct Made {
    pub project: String,
    /// The Manifest id of its first member, which the Job starts in. None where it holds none.
    pub repository: Option<String>,
    /// How its work lands, where its settings say: the line over the Job's box.
    pub lands: Option<SharedString>,
    pub served: Vec<RepositorySummary>,
    pub projects: Projects,
}

impl EventEmitter<Made> for NewProjectSheet {}

/// One repository the sheet lists, as Fleet serves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Served {
    pub id: String,
    pub name: SharedString,
    /// Its root, absolute, as Fleet names it.
    pub root: String,
    /// Whether it has a Manifest, which the folder search names it by; without one it is searched by its root.
    pub manifest: bool,
}

/// How long typing in a search is quiet before Fleet is asked: as the `@` mention's file lookup waits.
const LOOKUP: Duration = Duration::from_millis(150);

/// A folder search as it was asked: of which repository, for what words.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FolderAsk {
    id: String,
    query: String,
}

/// What Fleet answered to a search of his forge account, for the words typed in Clone a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ForgeFound {
    Listed(ipc::ForgeRepositories),
    Refused(SharedString),
}

/// What Fleet answered to a [`FolderAsk`] that the sheet shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Found {
    Folders(Vec<String>),
    Refused(SharedString),
}

/// What of a repository is in the project: the whole of it, or folders of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pick {
    pub whole: bool,
    pub folders: Vec<SharedString>,
}

/// One line of the repositories list, which the keys move along.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Repository(usize),
    Whole(usize),
    /// A repository of his forge account that the Clone search found, by its place in the answer.
    Forge(usize),
    /// A folder of the opened repository, by its path, with the letters the search found, and whether it is
    /// drawn only because it is picked.
    Folder { at: usize, path: SharedString, lit: Vec<usize>, kept: bool },
    Clone,
    New,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Mac,
    Synced,
    Cloud,
}

pub const PLACES: [(Place, &str, &str); 3] = [(Place::Mac, "This Mac", "hard-drive"), (Place::Synced, "Synced folder", "folder"), (Place::Cloud, "Cloud", "globe")];

/// Which menu hangs off the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Menu {
    Setting(Key),
    CloneParent,
    NewParent,
}

struct Popup {
    menu: Menu,
    dropdown: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

/// The keys' places in the sheet, each a stop Tab reaches in this order.
pub struct Handles {
    pub frame: FocusHandle,
    pub list: FocusHandle,
    pub clone_parent: FocusHandle,
    pub new_parent: FocusHandle,
    pub place: FocusHandle,
    pub defaults: FocusHandle,
    pub custom: FocusHandle,
    pub settings: HashMap<Key, FocusHandle>,
    pub share: FocusHandle,
    pub cancel: FocusHandle,
    pub create: FocusHandle,
}

/// The text boxes, each keeping what is typed under its own key.
pub struct Boxes {
    pub name: Entity<InputState>,
    pub find: Entity<InputState>,
    pub folders: Entity<InputState>,
    pub clone: Entity<InputState>,
    pub new: Entity<InputState>,
    pub synced: Entity<InputState>,
}

pub struct NewProjectSheet {
    wire: Wire,
    open: bool,
    pub boxes: Boxes,
    pub at: Handles,
    pub served: Option<Vec<Served>>,
    areas: Option<RepositoryAreas>,
    pub opened: Option<usize>,
    pub picks: HashMap<String, Pick>,
    pub clone_open: bool,
    pub new_open: bool,
    pub clone_parent: Option<String>,
    pub new_parent: Option<String>,
    pub place: Place,
    pub custom: bool,
    /// Tracker › Custom: the name field opened under the row, and the Job it starts.
    pub tracker: Entity<crate::custom_tracker::CustomTracker>,
    /// What every project uses, once Fleet has said.
    pub inherited: Option<HashMap<Key, String>>,
    pub picked: HashMap<Key, String>,
    pub models: Vec<String>,
    pub share: bool,
    /// The line of the repositories list the keys are on.
    pub active: usize,
    pub refused: Option<SharedString>,
    pub sending: bool,
    popup: Option<Popup>,
    keys_were: Option<FocusHandle>,
    intercept: Option<Subscription>,
    reads: Vec<Task<()>>,
    act: Option<Task<()>>,
    /// Fleet's last answer to a folder search, and the request waiting on typing to go quiet or on Fleet.
    found: Option<(FolderAsk, Found)>,
    lookup: Option<Task<()>>,
    /// Fleet's last answer to a search of his forge account, with the words it answered, and the request waiting.
    forged: Option<(String, ForgeFound)>,
    forging: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl NewProjectSheet {
    pub fn new(wire: Wire, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let boxes = Boxes {
            name: field("", window, cx),
            find: field("Find a repository", window, cx),
            folders: field("", window, cx),
            clone: field("", window, cx),
            new: field("my-app", window, cx),
            synced: field("", window, cx),
        };
        let mut subs = Vec::new();
        for (state, keep) in [(&boxes.name, KEEP_NAME), (&boxes.clone, KEEP_CLONE), (&boxes.new, KEEP_NEW), (&boxes.synced, KEEP_SYNCED)] {
            subs.push(cx.subscribe_in(state, window, move |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    crate::drafts::keep(keep, state.read(cx).value().as_ref(), cx);
                    this.refused = None;
                    cx.notify();
                }
            }));
        }
        subs.push(cx.subscribe_in(&boxes.find, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.active = 0;
                cx.notify();
            }
        }));
        subs.push(cx.subscribe_in(&boxes.clone, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.want_forge(cx);
            }
        }));
        subs.push(cx.subscribe_in(&boxes.folders, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.active = 0;
                this.want_folders(cx);
                cx.notify();
            }
        }));
        // ⌘↩ in a text box, which takes the key before the sheet's action can.
        for state in [&boxes.name, &boxes.find, &boxes.folders, &boxes.clone, &boxes.new, &boxes.synced] {
            subs.push(cx.subscribe_in(state, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: true, .. }) {
                    this.create(window, cx);
                }
            }));
        }
        let tracker = cx.new(|cx| crate::custom_tracker::CustomTracker::new("np", "new-project:custom-tracker", window, cx));
        subs.push(cx.subscribe_in(&tracker, window, |this, _, asked: &crate::custom_tracker::Asked, window, cx| this.tracker_asked(asked, window, cx)));
        let settings = values::GROUPS.iter().flat_map(|group| group.keys.iter().map(|key| (*key, cx.focus_handle().tab_stop(true)))).collect();
        let at = Handles {
            frame: cx.focus_handle(),
            list: cx.focus_handle().tab_stop(true),
            clone_parent: cx.focus_handle().tab_stop(true),
            new_parent: cx.focus_handle().tab_stop(true),
            place: cx.focus_handle().tab_stop(true),
            defaults: cx.focus_handle().tab_stop(true),
            custom: cx.focus_handle().tab_stop(true),
            settings,
            share: cx.focus_handle().tab_stop(true),
            cancel: cx.focus_handle().tab_stop(true),
            create: cx.focus_handle().tab_stop(true),
        };
        NewProjectSheet {
            wire,
            open: false,
            boxes,
            at,
            served: None,
            areas: None,
            opened: None,
            picks: HashMap::new(),
            clone_open: false,
            new_open: false,
            clone_parent: None,
            new_parent: None,
            place: Place::Mac,
            custom: false,
            tracker,
            inherited: None,
            picked: HashMap::new(),
            models: Vec::new(),
            share: false,
            active: 0,
            refused: None,
            sending: false,
            popup: None,
            keys_were: None,
            intercept: None,
            reads: Vec::new(),
            act: None,
            found: None,
            lookup: None,
            forged: None,
            forging: None,
            _subs: subs,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    // -- opening and closing ----------------------------------------------------------------------------

    /// Open on what was typed last time, the keys in Name; what Fleet serves is read again.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.keys_were = window.focused(cx);
        self.open = true;
        let me = cx.entity().downgrade();
        self.intercept = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.open && this.popup.is_none() && this.at.frame.contains_focused(window, cx) {
                    let down = KeyDownEvent { keystroke: event.keystroke.clone(), is_held: false, prefer_character_input: false };
                    this.intercepted(&down, window, cx);
                }
            });
        }));
        for (state, keep) in [(&self.boxes.name, KEEP_NAME), (&self.boxes.clone, KEEP_CLONE), (&self.boxes.new, KEEP_NEW), (&self.boxes.synced, KEEP_SYNCED)] {
            let words = crate::drafts::kept(keep, cx).unwrap_or_default();
            state.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        self.clone_open |= !self.boxes.clone.read(cx).value().is_empty();
        self.new_open |= !self.boxes.new.read(cx).value().is_empty();
        self.read(cx);
        self.want_forge(cx);
        let name = self.boxes.name.clone();
        window.defer(cx, move |window, cx| name.update(cx, |name, cx| name.focus(window, cx)));
        cx.notify();
    }

    /// Esc, Cancel or a press on the scrim: the sheet goes and what was typed stays, the keys back where they were.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.intercept = None;
        self.popup = None;
        if let Some(keys) = self.keys_were.take() {
            window.focus(&keys, cx);
        }
        cx.notify();
    }

    /// The repositories, their folders, what every project uses and the models, each read off the main thread.
    fn read(&mut self, cx: &mut Context<Self>) {
        self.served = self.wire.repositories(cx).map(|all| all.iter().map(served_of).collect());
        let parent = self.parents().into_iter().next();
        self.clone_parent = self.clone_parent.take().or_else(|| parent.clone());
        self.new_parent = self.new_parent.take().or(parent);
        let areas = self.wire.areas(cx);
        let values = self.wire.values(cx);
        let models = self.wire.models(cx);
        self.reads = vec![
            cx.spawn(async move |this, cx| {
                let answer = areas.await;
                let _ = this.update(cx, |this, cx| {
                    match answer {
                        Ok(areas) => this.areas = Some(areas),
                        Err(why) => crate::log::Note::warn("the folders were not read").field("why", why).emit(),
                    }
                    cx.notify();
                });
            }),
            cx.spawn(async move |this, cx| {
                let answer = values.await;
                let _ = this.update(cx, |this, cx| {
                    match answer {
                        Ok(values) => this.inherited = Some(values::inherited(&values)),
                        Err(why) => crate::log::Note::warn("what every project uses was not read").field("why", why).emit(),
                    }
                    cx.notify();
                });
            }),
            cx.spawn(async move |this, cx| {
                let answer = models.await;
                let _ = this.update(cx, |this, cx| {
                    if let Ok(models) = answer {
                        this.models = models;
                        cx.notify();
                    }
                });
            }),
        ];
    }

    // -- the repositories -----------------------------------------------------------------------------------

    /// The lines of the list as they stand: each repository Find leaves, the opened one's Whole repository and
    /// folders, then Clone a repository and New empty repository.
    pub fn lines(&self, cx: &App) -> Vec<Line> {
        let query = self.boxes.find.read(cx).value().trim().to_lowercase();
        let mut lines = Vec::new();
        for (at, one) in self.served.iter().flatten().enumerate() {
            if !query.is_empty() && fuzzy(&query, &one.name.to_lowercase()).is_none() {
                continue;
            }
            lines.push(Line::Repository(at));
            if self.opened == Some(at) {
                lines.push(Line::Whole(at));
                lines.extend(self.folder_lines(at, cx));
            }
        }
        lines.push(Line::Clone);
        if self.clone_open {
            lines.extend((0..self.forge_rows(cx).len()).map(Line::Forge));
        }
        lines.push(Line::New);
        lines
    }

    /// The opened repository's folders: its top folders, or every folder the search finds, best first; then the
    /// folders picked that the list does not show.
    fn folder_lines(&self, at: usize, cx: &App) -> Vec<Line> {
        let Some(one) = self.served.as_ref().and_then(|all| all.get(at)) else { return Vec::new() };
        let query = self.folder_query(cx);
        let shown: Vec<(SharedString, Vec<usize>)> = if query.is_empty() {
            folders_of(self.areas.as_ref(), &one.id).into_iter().filter(|path| !path.contains('/')).map(|path| (path.into(), Vec::new())).collect()
        } else {
            match self.found.as_ref().filter(|(ask, _)| ask.id == one.id && ask.query == query).map(|(_, found)| found) {
                // Fleet's folders in Fleet's order, the lit letters worked out here.
                Some(Found::Folders(paths)) => paths.iter().map(|path| (path.clone().into(), fuzzy(&query, &path.to_lowercase()).map(|(_, lit)| lit).unwrap_or_default())).collect(),
                Some(Found::Refused(_)) => Vec::new(),
                // Not answered yet, or Fleet is out of reach: the folders already read.
                None => {
                    let mut found: Vec<(u32, String, Vec<usize>)> = folders_of(self.areas.as_ref(), &one.id)
                        .into_iter()
                        .filter_map(|path| fuzzy(&query, &path.to_lowercase()).map(|(score, lit)| (score, path, lit)))
                        .collect();
                    found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.len().cmp(&b.1.len())).then(a.1.cmp(&b.1)));
                    found.into_iter().map(|(_, path, lit)| (path.into(), lit)).collect()
                }
            }
        };
        let picked = self.picks.get(&one.id).map(|pick| pick.folders.clone()).unwrap_or_default();
        let mut lines: Vec<Line> = shown.iter().map(|(path, lit)| Line::Folder { at, path: path.clone(), lit: lit.clone(), kept: false }).collect();
        lines.extend(picked.into_iter().filter(|path| !shown.iter().any(|(one, _)| one == path)).map(|path| Line::Folder { at, path, lit: Vec::new(), kept: true }));
        lines
    }

    /// What was typed in the opened repository's folder search, as it is searched.
    fn folder_query(&self, cx: &App) -> String {
        self.boxes.folders.read(cx).value().trim().to_lowercase()
    }

    /// The search now asked of the opened repository, and where Fleet is to look: none while nothing is typed.
    fn folder_ask(&self, cx: &App) -> Option<(FolderAsk, wire::Place)> {
        let one = self.served.as_ref().and_then(|all| all.get(self.opened?))?;
        let query = self.folder_query(cx);
        if query.is_empty() {
            return None;
        }
        let place = if one.manifest { wire::Place::Manifest(one.id.clone()) } else { wire::Place::Root(one.root.clone()) };
        Some((FolderAsk { id: one.id.clone(), query }, place))
    }

    /// Run a search once typing has been quiet for `LOOKUP`: `asked` starts the request then, and `took` is given
    /// the answer with the words it was for. The task is dropped by the next key, so one keystroke in a run is
    /// never posted.
    fn after_quiet<A: 'static, R: 'static>(
        &self,
        ask: A,
        asked: impl FnOnce(&mut Self, &mut Context<Self>) -> Task<R> + 'static,
        took: impl FnOnce(&mut Self, A, R, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        let wait = if self.wire.is_mock() { None } else { Some(LOOKUP) };
        cx.spawn(async move |this, cx| {
            if let Some(wait) = wait {
                cx.background_executor().timer(wait).await;
            }
            let Ok(task) = this.update(cx, |this, cx| asked(this, cx)) else { return };
            let answer = task.await;
            let _ = this.update(cx, |this, cx| took(this, ask, answer, cx));
        })
    }

    /// Ask Fleet for the folders the search now names.
    fn want_folders(&mut self, cx: &mut Context<Self>) {
        self.lookup = None;
        let Some((ask, place)) = self.folder_ask(cx) else { return };
        if self.found.as_ref().is_some_and(|(held, _)| *held == ask) {
            return;
        }
        let query = ask.query.clone();
        self.lookup = Some(self.after_quiet(ask, move |this, cx| this.wire.search_folders(&place, &query, cx), |this, ask, answer, cx| this.folders_answered(ask, answer, cx), cx));
    }

    /// Fleet answered a folder search. An answer to words or a repository since changed is dropped, so a slow
    /// one never replaces a newer. Where Fleet could not be asked, the sheet keeps searching what it read.
    fn folders_answered(&mut self, ask: FolderAsk, answer: wire::Folders, cx: &mut Context<Self>) {
        if self.folder_ask(cx).map(|(now, _)| now).as_ref() != Some(&ask) {
            return;
        }
        match answer {
            wire::Folders::Found(paths) => self.found = Some((ask, Found::Folders(paths))),
            wire::Folders::Refused(why) => self.found = Some((ask, Found::Refused(why.into()))),
            wire::Folders::Down => self.found = None,
        }
        cx.notify();
    }

    /// Fleet's sentence where it refused the search now typed, to draw where the folders would be.
    pub fn folder_refusal(&self, cx: &App) -> Option<SharedString> {
        let (ask, _) = self.folder_ask(cx)?;
        match self.found.as_ref() {
            Some((held, Found::Refused(why))) if *held == ask => Some(why.clone()),
            _ => None,
        }
    }

    // -- the Clone search -------------------------------------------------------------------------------------

    /// What was typed in Clone a repository, as searched on his forge account: none while the box is shut or
    /// empty, or holds an address, which is a clone by URL and not a search.
    fn forge_query(&self, cx: &App) -> Option<String> {
        let typed = self.boxes.clone.read(cx).value().trim().to_string();
        (self.clone_open && !typed.is_empty() && !typed.contains("://") && !typed.contains('@')).then_some(typed)
    }

    /// Ask Fleet for the repositories the Clone box's words name.
    fn want_forge(&mut self, cx: &mut Context<Self>) {
        self.forging = None;
        let Some(query) = self.forge_query(cx) else { return };
        if self.forged.as_ref().is_some_and(|(held, _)| *held == query) {
            return;
        }
        let ask = query.clone();
        self.forging = Some(self.after_quiet(ask, move |this, cx| this.wire.search_forge(&query, cx), |this, ask, answer, cx| this.forge_answered(ask, answer, cx), cx));
    }

    /// Fleet answered a search of his forge account. As with the folders, an answer to words since changed is
    /// dropped; Fleet out of reach lists nothing and says nothing.
    fn forge_answered(&mut self, ask: String, answer: wire::Forge, cx: &mut Context<Self>) {
        if self.forge_query(cx).as_ref() != Some(&ask) {
            return;
        }
        match answer {
            wire::Forge::Found(listed) => self.forged = Some((ask, ForgeFound::Listed(listed))),
            wire::Forge::Refused(why) => self.forged = Some((ask, ForgeFound::Refused(why.into()))),
            wire::Forge::Down => self.forged = None,
        }
        cx.notify();
    }

    /// The repositories found for the words now typed.
    pub fn forge_rows(&self, cx: &App) -> &[ipc::ForgeRepository] {
        match (self.forge_query(cx), self.forged.as_ref()) {
            (Some(now), Some((ask, ForgeFound::Listed(listed)))) if now == *ask => &listed.repositories,
            _ => &[],
        }
    }

    /// Whether more matched the words typed than the rows show.
    pub fn forge_cut(&self, cx: &App) -> bool {
        matches!((self.forge_query(cx), self.forged.as_ref()), (Some(now), Some((ask, ForgeFound::Listed(listed)))) if now == *ask && listed.cut)
    }

    /// The forge's sentence where it refused the words now typed, to draw where the rows would be.
    pub fn forge_refusal(&self, cx: &App) -> Option<SharedString> {
        match (self.forge_query(cx), self.forged.as_ref()) {
            (Some(now), Some((ask, ForgeFound::Refused(why)))) if now == *ask => Some(why.clone()),
            _ => None,
        }
    }

    /// The place in the list of the repository Fleet already serves at `row`'s `local_path`, by its root.
    pub fn forge_local(&self, row: &ipc::ForgeRepository) -> Option<usize> {
        local_of(self.served.as_deref().unwrap_or_default(), row)
    }

    /// A press on a found repository: where Fleet serves it already, ticks it in the list above, as its own box
    /// does; where it does not, the clone box takes its address, for Create to clone.
    pub fn use_forge(&mut self, of: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.forge_rows(cx).get(of).cloned() else { return };
        match self.forge_local(&row) {
            Some(at) => self.tick_whole(at, cx),
            None => {
                crate::drafts::keep(KEEP_CLONE, &row.url, cx);
                self.boxes.clone.update(cx, |clone, cx| clone.set_value(row.url, window, cx));
                self.forging = None;
            }
        }
        cx.notify();
    }

    /// Whether the folders of the opened repository are still being read.
    pub fn folders_unread(&self) -> bool {
        self.areas.is_none()
    }

    pub fn pick_of(&self, at: usize) -> Pick {
        self.served.as_ref().and_then(|all| all.get(at)).and_then(|one| self.picks.get(&one.id)).cloned().unwrap_or_default()
    }

    /// The whole of a repository, ticked or not. Ticking it lets go of its folders.
    pub fn tick_whole(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(id) = self.served.as_ref().and_then(|all| all.get(at)).map(|one| one.id.clone()) else { return };
        let pick = self.picks.entry(id.clone()).or_default();
        if pick.whole || !pick.folders.is_empty() {
            self.picks.remove(&id);
        } else {
            pick.whole = true;
        }
        self.refused = None;
        cx.notify();
    }

    pub fn tick_folder(&mut self, at: usize, path: SharedString, cx: &mut Context<Self>) {
        let Some(id) = self.served.as_ref().and_then(|all| all.get(at)).map(|one| one.id.clone()) else { return };
        let pick = self.picks.entry(id.clone()).or_default();
        pick.whole = false;
        match pick.folders.iter().position(|one| *one == path) {
            Some(found) => {
                pick.folders.remove(found);
            }
            None => pick.folders.push(path),
        }
        if !pick.whole && pick.folders.is_empty() {
            self.picks.remove(&id);
        }
        self.refused = None;
        cx.notify();
    }

    /// Open a repository on its folders, or shut it. One is open at a time.
    pub fn toggle_open(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.opened = if self.opened == Some(at) { None } else { Some(at) };
        self.found = None;
        self.lookup = None;
        self.boxes.folders.update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    pub fn toggle_clone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clone_open = !self.clone_open;
        self.want_forge(cx);
        if self.clone_open {
            let field = self.boxes.clone.clone();
            window.defer(cx, move |window, cx| field.update(cx, |field, cx| field.focus(window, cx)));
        }
        cx.notify();
    }

    pub fn toggle_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_open = !self.new_open;
        if self.new_open {
            let field = self.boxes.new.clone();
            window.defer(cx, move |window, cx| field.update(cx, |field, cx| field.focus(window, cx)));
        }
        cx.notify();
    }

    /// The folders a repository can be cloned or made under: those the served repositories sit in.
    pub fn parents(&self) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        for one in self.served.iter().flatten() {
            if let Some((parent, _)) = one.root.trim_end_matches('/').rsplit_once('/') {
                if !parent.is_empty() && !all.iter().any(|known| known == parent) {
                    all.push(parent.to_string());
                }
            }
        }
        all
    }

    // -- keys ------------------------------------------------------------------------------------------------

    /// Every stop Tab reaches, in the sheet's order.
    fn stops(&self, cx: &App) -> Vec<FocusHandle> {
        let mut stops = vec![self.boxes.name.focus_handle(cx), self.boxes.find.focus_handle(cx), self.at.list.clone()];
        if self.opened.is_some() {
            stops.push(self.boxes.folders.focus_handle(cx));
        }
        if self.clone_open {
            stops.extend([self.boxes.clone.focus_handle(cx), self.at.clone_parent.clone()]);
        }
        if self.new_open {
            stops.extend([self.boxes.new.focus_handle(cx), self.at.new_parent.clone()]);
        }
        stops.push(self.at.place.clone());
        if self.place == Place::Synced {
            stops.push(self.boxes.synced.focus_handle(cx));
        }
        stops.extend([self.at.defaults.clone(), self.at.custom.clone()]);
        if self.custom {
            for group in &values::GROUPS {
                for key in group.keys {
                    stops.extend(self.at.settings.get(key).cloned());
                    // The Custom tracker's field and its press come right after the tracker's choice.
                    if *key == Key::Tracker {
                        stops.extend(self.tracker.read(cx).stops(cx));
                    }
                }
            }
            stops.push(self.at.share.clone());
        }
        stops.extend([self.at.cancel.clone(), self.at.create.clone()]);
        stops
    }

    /// What the sheet takes before a text box can: Esc, Tab, and ↓ out of a search into the list.
    fn intercepted(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        let plain = !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt;
        match key.key.as_str() {
            // Esc in the Custom tracker's field shuts the field, and not the sheet.
            "escape" if self.tracker.read(cx).asking() && self.tracker.read(cx).contains_focused(window, cx) => {
                self.tracker.update(cx, |tracker, cx| tracker.hide(cx));
                if let Some(handle) = self.at.settings.get(&Key::Tracker) {
                    window.focus(handle, cx);
                }
            }
            "escape" => self.close(window, cx),
            "tab" if plain => {
                let stops = self.stops(cx);
                let here = stops.iter().position(|stop| stop.contains_focused(window, cx));
                let len = stops.len();
                let next = match (here, key.modifiers.shift) {
                    (Some(at), false) => (at + 1) % len,
                    (Some(at), true) => (at + len - 1) % len,
                    (None, _) => 0,
                };
                window.focus(&stops[next], cx);
            }
            "down"
                if plain
                    && (self.boxes.find.focus_handle(cx).is_focused(window)
                        || self.boxes.folders.focus_handle(cx).is_focused(window)
                        || (self.clone_open && self.boxes.clone.focus_handle(cx).is_focused(window))) =>
            {
                // Out of a search, the keys land on the next line of the list: its first folder, or the first
                // repository the Clone search found. A refusal or a cut line is no line, so the keys pass it.
                let lines = self.lines(cx);
                if self.boxes.folders.focus_handle(cx).is_focused(window) {
                    // No folder found (Fleet refused, or nothing matches): the line after the opened repository's own.
                    self.active = lines
                        .iter()
                        .position(|line| matches!(line, Line::Folder { .. }))
                        .or_else(|| lines.iter().position(|line| matches!(line, Line::Whole(_))).map(|at| at + 1))
                        .unwrap_or(0);
                } else if self.boxes.clone.focus_handle(cx).is_focused(window) {
                    self.active = lines.iter().position(|line| matches!(line, Line::Forge(_))).or_else(|| lines.iter().position(|line| *line == Line::New)).unwrap_or(0);
                } else {
                    self.active = 0;
                }
                window.focus(&self.at.list, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The list's keys: ↑↓ move, Space ticks, Return ticks a folder or opens a repository, →← open and shut.
    pub fn list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform || key.modifiers.control || key.modifiers.alt {
            return;
        }
        let lines = self.lines(cx);
        let len = lines.len().max(1);
        self.active = self.active.min(len - 1);
        let line = lines.get(self.active).cloned();
        match (key.key.as_str(), line) {
            ("down", _) => self.active = (self.active + 1).min(len - 1),
            ("up", _) => self.active = self.active.saturating_sub(1),
            ("space", Some(Line::Repository(at) | Line::Whole(at))) => self.tick_whole(at, cx),
            ("space" | "enter", Some(Line::Folder { at, path, .. })) => self.tick_folder(at, path, cx),
            ("enter", Some(Line::Whole(at))) => self.tick_whole(at, cx),
            ("enter", Some(Line::Repository(at))) => self.toggle_open(at, window, cx),
            ("right", Some(Line::Repository(at))) if self.opened != Some(at) => self.toggle_open(at, window, cx),
            ("left", Some(Line::Repository(at))) if self.opened == Some(at) => self.toggle_open(at, window, cx),
            ("left", Some(Line::Whole(at) | Line::Folder { at, .. })) => {
                self.toggle_open(at, window, cx);
                self.active = self.lines(cx).iter().position(|line| *line == Line::Repository(at)).unwrap_or(0);
            }
            ("enter" | "space", Some(Line::Forge(of))) => self.use_forge(of, window, cx),
            ("enter" | "space", Some(Line::Clone)) => self.toggle_clone(window, cx),
            ("enter" | "space", Some(Line::New)) => self.toggle_new(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Where its file is kept: ←→ move the pick.
    pub fn place_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let at = PLACES.iter().position(|(one, ..)| *one == self.place).unwrap_or(0);
        let to = match event.keystroke.key.as_str() {
            "right" | "down" => (at + 1).min(PLACES.len() - 1),
            "left" | "up" => at.saturating_sub(1),
            _ => return,
        };
        self.place = PLACES[to].0;
        cx.stop_propagation();
        cx.notify();
    }

    /// The tiles: Return or Space picks the one with the keys, ←→ go to the other.
    pub fn tile_key(&mut self, custom: bool, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" | "space" => self.set_custom(custom, cx),
            "right" if !custom => window.focus(&self.at.custom, cx),
            "left" if custom => window.focus(&self.at.defaults, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub fn set_custom(&mut self, custom: bool, cx: &mut Context<Self>) {
        self.custom = custom;
        cx.notify();
    }

    pub fn on_act(&mut self, key: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match key.id {
            "send_message" => self.create(window, cx),
            // The sheet is a dialog: the keys that start or open something else do nothing while it is up.
            "new_job" | "new_terminal_job" | "dispatch_after" => {}
            _ => cx.propagate(),
        }
    }

    // -- menus ----------------------------------------------------------------------------------------------

    /// A setting's word, as the sheet shows it, and whether it is changed from what every project uses.
    pub fn shown(&self, key: Key) -> Option<(String, bool)> {
        let inherited = self.inherited.as_ref()?.get(&key).cloned().unwrap_or_default();
        let word = self.picked.get(&key).cloned().unwrap_or_else(|| inherited.clone());
        Some((word.clone(), word != inherited))
    }

    fn open_menu(&mut self, menu: Menu, under: &'static str, role: gpui::Role, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = bounds_of(window, cx, role, under) else { return };
        let entries = match menu {
            Menu::Setting(key) => {
                let now = self.shown(key).map(|(word, _)| word).unwrap_or_default();
                // The trackers this Mac has grow with its mods, and end on Custom.
                let words = if key == Key::Tracker { crate::custom_tracker::options(&self.wire.mods(cx)) } else { key.options(&self.models) };
                words
                    .into_iter()
                    .map(|(word, label)| Entry::Choice { id: crate::titlebar::id_of(format!("word:{word}")), label: label.into(), on: word == now })
                    .collect()
            }
            Menu::CloneParent | Menu::NewParent => {
                let now = if menu == Menu::CloneParent { &self.clone_parent } else { &self.new_parent };
                self.parents()
                    .into_iter()
                    .map(|parent| Entry::Choice { id: crate::titlebar::id_of(format!("word:{parent}")), label: tilde(&parent).into(), on: now.as_deref() == Some(parent.as_str()) })
                    .collect()
            }
        };
        let gap = cx.look().length(token::SPACE_1);
        let at = point(from.origin.x, from.origin.y + from.size.height + gap);
        let dropdown = cx.new(|cx| Dropdown::new(under, entries, at, Anchor::TopLeft, true, cx));
        let picked = cx.subscribe_in(&dropdown, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            let word = id.strip_prefix("word:").unwrap_or_default().to_string();
            this.popup = None;
            match menu {
                Menu::Setting(_) if word == crate::custom_tracker::CUSTOM => {
                    this.open_custom(window, cx);
                    return;
                }
                Menu::Setting(key) => {
                    this.picked.insert(key, word);
                }
                Menu::CloneParent => this.clone_parent = Some(word),
                Menu::NewParent => this.new_parent = Some(word),
            }
            this.keys_back(menu, window, cx);
        });
        let dismissed = cx.subscribe_in(&dropdown, window, move |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            this.keys_back(menu, window, cx);
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some(Popup { menu, dropdown, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    /// A menu shut: the keys go back to what opened it.
    fn keys_back(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let to = match menu {
            Menu::Setting(key) => self.at.settings.get(&key).cloned(),
            Menu::CloneParent => Some(self.at.clone_parent.clone()),
            Menu::NewParent => Some(self.at.new_parent.clone()),
        };
        if let Some(to) = to {
            window.focus(&to, cx);
        }
        cx.notify();
    }

    pub fn open_setting(&mut self, key: Key, window: &mut Window, cx: &mut Context<Self>) {
        if self.inherited.is_none() || key.options(&self.models).is_empty() {
            return;
        }
        self.open_menu(Menu::Setting(key), key.label(), gpui::Role::Button, window, cx);
    }

    /// The repository the Job starts in: the first one picked, else the first one Fleet serves, by Manifest id and name.
    fn tracker_target(&self) -> Option<(String, SharedString)> {
        let served = self.served.as_deref().unwrap_or_default();
        let first = self.members().into_iter().next().map(|member| member.manifest_id.as_str().to_string());
        first.and_then(|id| served.iter().find(|one| one.id == id)).or_else(|| served.first()).map(|one| (one.id.clone(), one.name.clone()))
    }

    /// Custom, picked among the trackers: the name field opens under the row, with the keys in it.
    fn open_custom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let repository = self.tracker_target().map(|(_, name)| name).unwrap_or_default();
        self.tracker.update(cx, |tracker, cx| tracker.show(repository, window, cx));
        cx.notify();
    }

    /// What the field asked: Make its mod starts a Job in the first repository, which the project is not made yet to
    /// hold, and Open the Job opens it over the sheet's place.
    fn tracker_asked(&mut self, asked: &crate::custom_tracker::Asked, window: &mut Window, cx: &mut Context<Self>) {
        use crate::custom_tracker::Asked;
        match asked {
            Asked::Make(name) => {
                let Some((manifest, repository)) = self.tracker_target() else {
                    self.tracker.update(cx, |tracker, cx| tracker.refused("Fleet serves no repository to start it in", cx));
                    return;
                };
                let typed = self.boxes.name.read(cx).value().trim().to_string();
                let project = if typed.is_empty() { "a new project".to_string() } else { typed };
                let sent = self.wire.start_job(&manifest, crate::custom_tracker::message(name, &project, &repository), cx);
                let tracker = self.tracker.clone();
                cx.spawn_in(window, async move |_, cx| {
                    let answer = sent.await;
                    let _ = cx.update(|window, cx| {
                        tracker.update(cx, |tracker, cx| match answer {
                            Ok(job) => tracker.started(job, window, cx),
                            Err(why) => tracker.refused(why, cx),
                        })
                    });
                })
                .detach();
            }
            Asked::OpenJob(id) => {
                self.close(window, cx);
                if let Some(open) = cx.try_global::<crate::job_view::OpenJobFrom>().cloned() {
                    let id = id.clone();
                    cx.defer(move |cx| (open.0)(id, None, cx));
                }
            }
        }
    }

    pub fn open_parent(&mut self, clone: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (menu, name) = if clone { (Menu::CloneParent, "Folder to clone into") } else { (Menu::NewParent, "Folder to make it in") };
        self.open_menu(menu, name, gpui::Role::Button, window, cx);
    }

    pub fn popup(&self) -> Option<Entity<Dropdown>> {
        self.popup.as_ref().map(|popup| popup.dropdown.clone())
    }

    pub fn menu_open_on(&self, key: Key) -> bool {
        self.popup.as_ref().is_some_and(|popup| popup.menu == Menu::Setting(key))
    }

    // -- Create ----------------------------------------------------------------------------------------------

    pub fn named(&self, cx: &App) -> bool {
        !self.boxes.name.read(cx).value().trim().is_empty()
    }

    /// Create project, from its button or ⌘↩: each repository to clone or make first, one at a time, then the project.
    pub fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sending || !self.named(cx) {
            return;
        }
        self.sending = true;
        self.refused = None;
        self.next(window, cx);
        cx.notify();
    }

    /// The next request of a Create. A repository cloned or made is served and picked whole, and its field
    /// clears, so a Create Fleet refuses after it does not ask for it again.
    fn next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.boxes.clone.read(cx).value().trim().to_string();
        let fresh = self.boxes.new.read(cx).value().trim().to_string();
        let (task, field) = if self.clone_open && !url.is_empty() {
            let Some(parent) = self.clone_parent.clone() else { return self.refuse("Pick a folder to clone into".into(), cx) };
            (self.wire.clone_repository(url, parent, cx), Some((self.boxes.clone.clone(), KEEP_CLONE)))
        } else if self.new_open && !fresh.is_empty() {
            let Some(parent) = self.new_parent.clone() else { return self.refuse("Pick a folder to make it in".into(), cx) };
            (self.wire.create_repository(fresh, parent, cx), Some((self.boxes.new.clone(), KEEP_NEW)))
        } else {
            return self.add(window, cx);
        };
        self.act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(made) => {
                    let one = served_of(&made);
                    this.picks.insert(one.id.clone(), Pick { whole: true, folders: Vec::new() });
                    this.served.get_or_insert_with(Vec::new).push(one);
                    if let Some((field, keep)) = field {
                        crate::drafts::clear(keep, cx);
                        field.update(cx, |field, cx| field.set_value("", window, cx));
                    }
                    this.next(window, cx);
                }
                Err(why) => this.refuse(why.into(), cx),
            });
        }));
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.boxes.name.read(cx).value().trim().to_string();
        let place = match self.place {
            Place::Mac => ProjectPlace::default(),
            Place::Cloud => ProjectPlace { place: RunPlace::Cloud, synced_folder: None, cloud: Some(RunCloud::Store {}) },
            Place::Synced => {
                let folder = self.boxes.synced.read(cx).value().trim().to_string();
                ProjectPlace { place: RunPlace::Synced, synced_folder: Some(untilde(&folder)), cloud: None }
            }
        };
        let members = self.members();
        let first = members.first().map(|member| member.manifest_id.as_str().to_string());
        let inherited = self.inherited.clone().unwrap_or_default();
        let project = create_body(name.clone(), members, (self.place != Place::Mac).then_some(place), self.custom, &self.picked, &inherited);
        let share = self.custom && self.share;
        let task = self.wire.add_project(project, cx);
        self.act = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let projects = match answer {
                Ok(projects) => projects,
                Err(why) => {
                    let _ = this.update(cx, |this, cx| this.refuse(why.into(), cx));
                    return;
                }
            };
            // Fleet puts a new project at the end of his list.
            let Some(id) = projects.projects.iter().rev().find(|one| one.name == name).map(|one| one.id.clone()) else { return };
            let projects = if share {
                let shared = this.update(cx, |this, cx| this.wire.share(&id, cx));
                match shared {
                    Ok(task) => task.await.unwrap_or(projects),
                    Err(_) => projects,
                }
            } else {
                projects
            };
            let _ = this.update_in(cx, |this, window, cx| this.made(id, name, first, projects, window, cx));
        }));
    }

    /// Each repository picked, whole or by its folders, in the order the list shows them.
    fn members(&self) -> Vec<ProjectMember> {
        let mut members = Vec::new();
        for one in self.served.iter().flatten() {
            let Some(pick) = self.picks.get(&one.id) else { continue };
            let id = ipc::ManifestId::carried(one.id.clone());
            if pick.whole {
                members.push(ProjectMember { manifest_id: id, folder: None });
            } else {
                members.extend(pick.folders.iter().map(|folder| ProjectMember { manifest_id: id.clone(), folder: Some(folder.to_string()) }));
            }
        }
        members
    }

    fn made(&mut self, project: String, name: String, repository: Option<String>, projects: Projects, window: &mut Window, cx: &mut Context<Self>) {
        // The project's own `delivery`, as Fleet kept it from the call that made it.
        let kept = projects.projects.iter().find(|one| one.id == project).and_then(|one| one.values.as_ref()).and_then(|values| values.delivery.as_deref());
        let lands = kept.and_then(values::delivery_said).map(|said| SharedString::from(format!("{said}, from {name}'s settings")));
        let served = self.wire.repositories(cx).unwrap_or_default();
        crate::drafts::clear(KEEP_NAME, cx);
        crate::drafts::clear(KEEP_SYNCED, cx);
        self.boxes.name.update(cx, |field, cx| field.set_value("", window, cx));
        self.sending = false;
        self.picks.clear();
        self.picked.clear();
        self.tracker.update(cx, |tracker, cx| tracker.reset(cx));
        self.opened = None;
        self.clone_open = false;
        self.new_open = false;
        // The keys go to the Job's box, not back to what opened the sheet.
        self.keys_were = None;
        self.close(window, cx);
        cx.emit(Made { project, repository, lands, served, projects });
    }

    fn refuse(&mut self, why: SharedString, cx: &mut Context<Self>) {
        self.sending = false;
        self.refused = Some(why);
        cx.notify();
    }
}

/// A served repository as the list names it: its Manifest's repository, else its folder's name.
fn served_of(one: &RepositorySummary) -> Served {
    let id = one.manifest.as_ref().map(|manifest| manifest.id.as_str().to_string()).or_else(|| one.implicit_id.as_ref().map(|id| id.as_str().to_string())).unwrap_or_default();
    let name = one.manifest.as_ref().map_or_else(|| one.root.trim_end_matches('/').rsplit('/').next().unwrap_or(&one.root).to_string(), |manifest| manifest.repository.clone());
    Served { id, name: name.into(), root: one.root.clone(), manifest: one.manifest.is_some() }
}

/// The place in the list of the repository Fleet serves at `row`'s `local_path`: the roots compared as Fleet names
/// them, without a closing `/`.
fn local_of(served: &[Served], row: &ipc::ForgeRepository) -> Option<usize> {
    let path = row.local_path.as_deref()?.trim_end_matches('/');
    served.iter().position(|one| one.root.trim_end_matches('/') == path)
}

/// A repository's folders as Fleet read its tree: each top folder, then the folders inside it.
fn folders_of(areas: Option<&RepositoryAreas>, id: &str) -> Vec<String> {
    let Some(one) = areas.and_then(|areas| areas.repositories.iter().find(|one| one.manifest_id.as_str() == id)) else { return Vec::new() };
    let mut all = Vec::new();
    for area in &one.areas {
        all.push(area.dir.clone());
        all.extend(area.kids.iter().map(|kid| format!("{}/{}", area.dir, kid.name)));
    }
    all
}

/// Whether `typed` is in `text` letter by letter, in order: how well, lower is better, and where each letter
/// was found, in bytes. A run of letters, and a letter at a word's start, score better.
pub fn fuzzy(typed: &str, text: &str) -> Option<(u32, Vec<usize>)> {
    let mut lit = Vec::new();
    let mut from = 0;
    let mut score = 0u32;
    for wanted in typed.chars().filter(|one| !one.is_whitespace()) {
        let (at, _) = text[from..].char_indices().find(|(_, one)| *one == wanted)?;
        let at = from + at;
        let start = at == 0 || text[..at].ends_with(['/', '-', '_', ' ', '.']);
        let gap = (at - from) as u32;
        score += if lit.is_empty() { gap.min(8) } else { gap * 2 };
        if start {
            score = score.saturating_sub(1);
        }
        lit.push(at);
        from = at + wanted.len_utf8();
    }
    Some((score, lit))
}

/// A path under the person's home, as `~/…`.
pub fn tilde(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned()).filter(|home| !home.is_empty()) {
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    // The walks' paths are under a home of their own.
    match path.strip_prefix("/Users/").and_then(|rest| rest.split_once('/')) {
        Some((_, rest)) => format!("~/{rest}"),
        None => path.to_string(),
    }
}

/// A path typed as `~/…`, made absolute for Fleet.
fn untilde(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => format!("{}{rest}", home.to_string_lossy()),
        _ => path.to_string(),
    }
}

/// Where a labelled element was drawn last frame, for a menu opened under it.
fn bounds_of(window: &Window, cx: &App, role: gpui::Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|drawn| drawn.bounds)
}

/// The line over a Job's box that says how its work lands, for Jobs a New project started: by Job id.
#[derive(Default)]
struct Lands(HashMap<String, SharedString>);

impl Global for Lands {}

pub fn lands_on(job: &str, line: SharedString, cx: &mut App) {
    cx.default_global::<Lands>().0.insert(job.to_string(), line);
}

pub fn lands(job: &str, cx: &App) -> Option<SharedString> {
    cx.try_global::<Lands>().and_then(|lands| lands.0.get(job)).cloned()
}

/// What `POST /projects` is sent on Create. Use defaults sends no values, so the project takes what every
/// project uses; Customize sends what was changed in its groups, in the same call.
fn create_body(name: String, members: Vec<ProjectMember>, place: Option<ProjectPlace>, custom: bool, picked: &HashMap<Key, String>, inherited: &HashMap<Key, String>) -> ipc::NewProject {
    ipc::NewProject {
        name,
        members: (!members.is_empty()).then_some(members),
        place,
        values: if custom { values::values_of(picked, inherited) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sends_the_customized_values_in_the_same_call_and_use_defaults_sends_none() {
        let inherited: HashMap<Key, String> = [(Key::AutoMerge, "checks-pass"), (Key::AgentStarts, "at_once"), (Key::Plans, "review"), (Key::PullRequests, "off"), (Key::Walks, "off"), (Key::Delivery, values::ASK)]
            .into_iter()
            .map(|(key, word)| (key, word.to_string()))
            .collect();
        let picked: HashMap<Key, String> = [(Key::Delivery, "auto_land"), (Key::AutoMerge, "always"), (Key::AgentStarts, "ask_me"), (Key::Walks, "review")]
            .into_iter()
            .map(|(key, word)| (key, word.to_string()))
            .collect();
        let member = ProjectMember { manifest_id: ipc::ManifestId::carried("sample-web".to_string()), folder: Some("web/checkout".into()) };
        let body = |custom| serde_json::to_value(create_body("Pocket".into(), vec![member.clone()], None, custom, &picked, &inherited)).expect("encodes");
        assert_eq!(body(false), serde_json::json!({"name": "Pocket", "members": [{"manifest_id": "sample-web", "folder": "web/checkout"}]}), "Use defaults sends no values");
        assert_eq!(
            body(true)["values"],
            serde_json::json!({"auto_merge": "always", "agent_starts": "ask_me", "delivery": "auto_land", "reviews": {"plans": "review", "pull_requests": "off", "walks": "review"}}),
            "Customize sends what changed, and how work lands goes in `delivery` with the rest"
        );
    }

    #[test]
    fn a_folder_is_found_by_its_letters_in_order_and_the_tighter_match_comes_first() {
        let (score, lit) = fuzzy("chk", "web/checkout").expect("found");
        assert_eq!(lit, [4, 5, 8]);
        assert!(fuzzy("chk", "web/cart").is_none());
        let (looser, _) = fuzzy("chk", "packages/hooks").expect("found");
        assert!(score < looser, "{score} against {looser}");
    }

    #[test]
    fn a_path_under_home_reads_with_a_tilde() {
        assert_eq!(tilde("/Users/user/Development/armada"), "~/Development/armada");
        assert_eq!(tilde("/srv/armada"), "/srv/armada");
    }

    #[test]
    fn folders_are_read_two_deep_from_the_tree() {
        let areas = crate::mock::repository_areas();
        let folders = folders_of(Some(&areas), "sample-web");
        assert!(folders.contains(&"web".to_string()) && folders.contains(&"services/pricing".to_string()));
        assert!(folders_of(Some(&areas), "sample-docs").is_empty());
    }

    fn sheet(cx: &mut gpui::TestAppContext) -> (Entity<NewProjectSheet>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (sheet, cx) = cx.add_window_view(|window, cx| NewProjectSheet::new(Wire::Mock(Box::new(wire::Mock::new())), window, cx));
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.served = sheet.wire.repositories(cx).map(|all| all.iter().map(served_of).collect());
            sheet.areas = Some(crate::mock::repository_areas());
            sheet.opened = sheet.served.as_ref().and_then(|all| all.iter().position(|one| one.id == "sample-web"));
            sheet.boxes.folders.update(cx, |folders, cx| folders.set_value("chk", window, cx));
        });
        (sheet, cx)
    }

    fn ask(query: &str) -> FolderAsk {
        FolderAsk { id: "sample-web".into(), query: query.into() }
    }

    fn folders(sheet: &NewProjectSheet, cx: &App) -> Vec<(String, Vec<usize>)> {
        sheet
            .lines(cx)
            .into_iter()
            .filter_map(|line| match line {
                Line::Folder { path, lit, .. } => Some((path.to_string(), lit)),
                _ => None,
            })
            .collect()
    }

    #[gpui::test]
    fn an_answer_for_words_since_changed_never_replaces_a_newer_one(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.folders_answered(ask("ch"), wire::Folders::Found(vec!["web/checkout".into()]), cx);
            assert_eq!(sheet.found, None, "an answer for words no longer typed is dropped");
            sheet.folders_answered(ask("chk"), wire::Folders::Found(vec!["web/checkout".into()]), cx);
            sheet.boxes.folders.update(cx, |folders, cx| folders.set_value("chko", window, cx));
            sheet.folders_answered(ask("chko"), wire::Folders::Found(vec!["services/checkout-api".into()]), cx);
            // The slow answer to the words before comes last.
            sheet.folders_answered(ask("chk"), wire::Folders::Found(vec!["web/checkout".into()]), cx);
            assert_eq!(sheet.found, Some((ask("chko"), Found::Folders(vec!["services/checkout-api".into()]))));
            sheet.opened = None;
            sheet.folders_answered(ask("chko"), wire::Folders::Found(vec!["web/checkout".into()]), cx);
            assert_eq!(sheet.found, Some((ask("chko"), Found::Folders(vec!["services/checkout-api".into()]))), "nor one for a repository since shut");
        });
    }

    #[gpui::test]
    fn fleets_folders_stand_in_fleets_order_with_the_letters_lit_on_each_path(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update(cx, |sheet, cx| {
            let found = vec!["services/checkout-api".to_string(), "web/checkout/components".to_string(), "docs".to_string()];
            sheet.folders_answered(ask("chk"), wire::Folders::Found(found), cx);
            let shown = folders(sheet, cx);
            let paths: Vec<&str> = shown.iter().map(|(path, _)| path.as_str()).collect();
            assert_eq!(paths, ["services/checkout-api", "web/checkout/components", "docs"], "Fleet's order, not the sheet's own ranking");
            assert_eq!(shown[1].1, [4, 5, 8], "the letters of the typed words, in order");
            assert!(shown[2].1.is_empty(), "a path the letters are not in carries none");
        });
    }

    #[gpui::test]
    fn a_refusal_stands_where_the_folders_would_and_fleet_out_of_reach_leaves_the_local_search(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update(cx, |sheet, cx| {
            sheet.folders_answered(ask("chk"), wire::Folders::Refused("The folder is not there".into()), cx);
            assert_eq!(sheet.folder_refusal(cx).as_deref(), Some("The folder is not there"));
            assert!(folders(sheet, cx).is_empty());
            sheet.folders_answered(ask("chk"), wire::Folders::Down, cx);
            assert_eq!(sheet.folder_refusal(cx), None);
            let local: Vec<String> = folders(sheet, cx).into_iter().map(|(path, _)| path).collect();
            assert!(local.contains(&"web/checkout".to_string()) && local.contains(&"services/checkout-api".to_string()), "the folders already read, searched here");
            assert!(local.iter().all(|path| path.matches('/').count() <= 1), "and none deeper than the two it read");
        });
    }

    fn forged<'a>(cx: &'a mut gpui::TestAppContext, typed: &'static str) -> (Entity<NewProjectSheet>, &'a mut gpui::VisualTestContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.clone_open = true;
            sheet.boxes.clone.update(cx, |clone, cx| clone.set_value(typed, window, cx));
        });
        (sheet, cx)
    }

    fn repository(owner: &str, name: &str, local: Option<&str>) -> ipc::ForgeRepository {
        ipc::ForgeRepository { owner: owner.into(), name: name.into(), url: format!("https://github.com/{owner}/{name}"), local_path: local.map(str::to_string), ..ipc::ForgeRepository::default() }
    }

    #[gpui::test]
    fn a_forge_answer_for_words_since_changed_never_replaces_a_newer_one(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = forged(cx, "ac");
        let listed = |name: &str| wire::Forge::Found(ipc::ForgeRepositories { repositories: vec![repository("acme", name, None)], cut: false });
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.forge_answered("a".into(), listed("old"), cx);
            assert_eq!(sheet.forged, None, "an answer for words no longer typed is dropped");
            sheet.forge_answered("ac".into(), listed("first"), cx);
            sheet.boxes.clone.update(cx, |clone, cx| clone.set_value("acm", window, cx));
            sheet.forge_answered("acm".into(), listed("newer"), cx);
            // The slow answer to the words before comes last.
            sheet.forge_answered("ac".into(), listed("first"), cx);
            assert_eq!(sheet.forge_rows(cx).iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["newer"]);
        });
    }

    #[gpui::test]
    fn a_repository_is_already_here_where_its_local_path_is_a_served_root(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = forged(cx, "ac");
        sheet.update(cx, |sheet, _| {
            let here = repository("acme", "sample-api", Some("/Users/user/Development/sample-api/"));
            let at = sheet.forge_local(&here).expect("served");
            assert_eq!(sheet.served.as_ref().expect("read")[at].id, "sample-api", "the root names the manifest the pick is made under");
            assert_eq!(sheet.forge_local(&repository("acme", "billing", None)), None, "no local path: not here");
            assert_eq!(sheet.forge_local(&repository("acme", "gone", Some("/Users/user/Development/gone"))), None, "a path no served root is: not here");
        });
    }

    #[gpui::test]
    fn pressing_a_repository_already_here_ticks_it_above_and_one_not_here_fills_the_clone_box(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = forged(cx, "ac");
        sheet.update_in(cx, |sheet, window, cx| {
            let found = ipc::ForgeRepositories { repositories: vec![repository("acme", "sample-api", Some("/Users/user/Development/sample-api")), repository("acme", "billing", None)], cut: false };
            sheet.forge_answered("ac".into(), wire::Forge::Found(found), cx);
            sheet.use_forge(0, window, cx);
            assert_eq!(sheet.picks.get("sample-api").map(|pick| pick.whole), Some(true));
            assert_eq!(sheet.boxes.clone.read(cx).value().as_ref(), "ac", "nothing is cloned");
            sheet.use_forge(1, window, cx);
            assert_eq!(sheet.boxes.clone.read(cx).value().as_ref(), "https://github.com/acme/billing");
            assert!(sheet.forge_rows(cx).is_empty(), "an address is a clone, not a search");
        });
    }

    #[gpui::test]
    fn a_cut_answer_says_more_match_and_a_422_stands_in_the_list_with_the_forges_sentence(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = forged(cx, "ac");
        sheet.update(cx, |sheet, cx| {
            sheet.forge_answered("ac".into(), wire::Forge::Found(ipc::ForgeRepositories { repositories: vec![repository("acme", "billing", None)], cut: true }), cx);
            assert!(sheet.forge_cut(cx));
            sheet.forge_answered("ac".into(), wire::Forge::Found(ipc::ForgeRepositories { repositories: vec![repository("acme", "billing", None)], cut: false }), cx);
            assert!(!sheet.forge_cut(cx));
            sheet.forge_answered("ac".into(), wire::Forge::Refused("GitHub has no token for this machine.".into()), cx);
            assert_eq!(sheet.forge_refusal(cx).as_deref(), Some("GitHub has no token for this machine."));
            assert!(sheet.forge_rows(cx).is_empty());
            sheet.forge_answered("ac".into(), wire::Forge::Down, cx);
            assert_eq!(sheet.forge_refusal(cx), None, "out of reach says nothing");
        });
    }

    fn down(sheet: &mut NewProjectSheet, window: &mut Window, cx: &mut Context<NewProjectSheet>) {
        let keystroke = gpui::Keystroke::parse("down").expect("a key");
        sheet.intercepted(&KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }, window, cx);
    }

    #[gpui::test]
    fn down_from_the_clone_box_passes_a_refusal_and_lands_on_new_empty_repository(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = forged(cx, "ac");
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.forge_answered("ac".into(), wire::Forge::Refused("GitHub has no token for this machine.".into()), cx);
            window.focus(&sheet.boxes.clone.focus_handle(cx), cx);
            down(sheet, window, cx);
            assert_eq!(sheet.lines(cx).get(sheet.active), Some(&Line::New), "the refusal is no line to land on");
            assert!(sheet.at.list.is_focused(window));
        });
        sheet.update_in(cx, |sheet, window, cx| {
            let found = ipc::ForgeRepositories { repositories: vec![repository("acme", "billing", None)], cut: true };
            sheet.forge_answered("ac".into(), wire::Forge::Found(found), cx);
            window.focus(&sheet.boxes.clone.focus_handle(cx), cx);
            down(sheet, window, cx);
            assert_eq!(sheet.lines(cx).get(sheet.active), Some(&Line::Forge(0)), "a found repository is a line");
            let keystroke = gpui::Keystroke::parse("down").expect("a key");
            sheet.list_key(&KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }, window, cx);
            assert_eq!(sheet.lines(cx).get(sheet.active), Some(&Line::New), "and the cut foot is passed the same way");
        });
    }

    #[gpui::test]
    fn down_from_the_folder_box_passes_a_refusal_to_the_line_after_the_opened_repository(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.folders_answered(ask("chk"), wire::Folders::Refused("/Users/user/Development/sample-web is not a folder on this machine".into()), cx);
            window.focus(&sheet.boxes.folders.focus_handle(cx), cx);
            down(sheet, window, cx);
            let lines = sheet.lines(cx);
            let whole = lines.iter().position(|line| matches!(line, Line::Whole(_))).expect("opened");
            assert_eq!(sheet.active, whole + 1, "the refusal is no line to land on");
        });
    }
}
