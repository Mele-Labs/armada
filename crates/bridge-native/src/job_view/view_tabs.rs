//! Tabs another part of the app puts in a Job's panes, each drawing a view of its own: the terminal's
//! first. The source opens one with [`JobView::open_view_tab_in`] in the pane it names ([`ViewPane`]), or
//! with [`JobView::open_view_tab`] in the top pane, offers kinds in every pane's + with
//! [`offer_tab`], hears which Job comes on screen with [`on_job_shown`], and finds the Job view with
//! [`JobView::live`]. A tab's acts, its close veto and its closed hook run after the Job view's own
//! update, so they may update the Job view again; the veto alone runs at once and must not.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, AnyView, App, Context, DismissEvent, Entity, Focusable, KeyDownEvent, MouseButton, Role, SharedString, Subscription, WeakEntity, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::TagKind;

use super::composer::{Kept, KeptTag};
use super::JobView;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::labelled;
use crate::look::Look;

/// One tab drawn by its source's view.
#[derive(Clone)]
pub struct ViewTab {
    /// Unique among the Job's view tabs, as `terminal:<id>`.
    pub id: String,
    pub title: SharedString,
    /// A registry icon name.
    pub glyph: &'static str,
    pub view: AnyView,
    /// What a right press on the tab offers, in order.
    pub menu: Vec<TabAct>,
    /// Asked before a press on × closes it: false keeps it open. Runs at once.
    pub closing: Option<Rc<dyn Fn(&mut Window, &mut App) -> bool>>,
    /// Told once it has closed, by × or by [`JobView::close_view_tab`].
    pub closed: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    /// Told the new title when a rename started by [`JobView::rename_view_tab`] is kept with Enter.
    pub renamed: Option<Rc<dyn Fn(String, &mut Window, &mut App)>>,
}

/// One row of a view tab's right-press menu.
#[derive(Clone)]
pub struct TabAct {
    pub label: SharedString,
    pub danger: bool,
    pub act: Rc<dyn Fn(&mut Window, &mut App)>,
}

/// The pane a view tab stands in: the thread's, the top of the right column, or its foot, where Now is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ViewPane {
    Thread,
    Top,
    Bottom,
}

impl ViewPane {
    pub(super) fn of(pane: super::tabs::Pane) -> ViewPane {
        match pane {
            super::tabs::Pane::Thread => ViewPane::Thread,
            super::tabs::Pane::Top => ViewPane::Top,
            super::tabs::Pane::Now => ViewPane::Bottom,
        }
    }
}

/// A kind of tab every pane's + offers after its own, and what opens one on Job `job` in the pane whose +
/// was pressed.
#[derive(Clone)]
pub struct TabOffer {
    pub id: &'static str,
    pub label: SharedString,
    pub glyph: &'static str,
    pub open: Rc<dyn Fn(String, ViewPane, &mut Window, &mut App)>,
}

#[derive(Clone, Default)]
pub struct TabOffers(pub Vec<TabOffer>);

impl gpui::Global for TabOffers {}

/// Put a kind in every pane's +, in place of one with its id.
pub fn offer_tab(offer: TabOffer, cx: &mut App) {
    let offers = cx.default_global::<TabOffers>();
    offers.0.retain(|one| one.id != offer.id);
    offers.0.push(offer);
}

/// Told the Job that comes on screen, or none.
#[derive(Clone, Default)]
pub struct JobShown(pub Vec<Rc<dyn Fn(Option<String>, &mut Window, &mut App)>>);

impl gpui::Global for JobShown {}

pub fn on_job_shown(hook: impl Fn(Option<String>, &mut Window, &mut App) + 'static, cx: &mut App) {
    cx.default_global::<JobShown>().0.push(Rc::new(hook));
}

/// The Job view drawn now, set as it is made.
pub(super) struct LiveJobView(pub WeakEntity<JobView>);

impl gpui::Global for LiveJobView {}

/// The rows a tab menu can hold, since a menu's ids live as long as the app.
const ACT_IDS: [&str; 12] = ["view-act:0", "view-act:1", "view-act:2", "view-act:3", "view-act:4", "view-act:5", "view-act:6", "view-act:7", "view-act:8", "view-act:9", "view-act:10", "view-act:11"];

/// Each Job's view tabs in strip order with the pane each stands in, and the one in front of each pane where a
/// view tab is.
#[derive(Default)]
pub(super) struct ViewTabs {
    by_job: HashMap<SharedString, Vec<(ViewPane, ViewTab)>>,
    front: HashMap<(SharedString, ViewPane), String>,
    /// The tab whose title is a field now.
    renaming: Option<TabRename>,
}

struct TabRename {
    job: SharedString,
    id: String,
    input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

impl JobView {
    /// The Job view on screen, where there is one.
    pub fn live(cx: &App) -> Option<Entity<JobView>> {
        cx.try_global::<LiveJobView>().and_then(|live| live.0.upgrade())
    }

    /// Open `tab` on Job `job` in the top pane, or put it in place of the tab with its id. `front` brings it forward.
    pub fn open_view_tab(&mut self, job: &str, tab: ViewTab, front: bool, cx: &mut Context<Self>) {
        self.open_view_tab_in(job, ViewPane::Top, tab, front, cx);
    }

    /// Open `tab` on Job `job` in `pane`, or put it in place of the tab with its id, which moves to `pane`.
    /// `front` brings it forward in that pane.
    pub fn open_view_tab_in(&mut self, job: &str, pane: ViewPane, tab: ViewTab, front: bool, cx: &mut Context<Self>) {
        let key = SharedString::from(job.to_string());
        let tabs = self.view_tabs.by_job.entry(key.clone()).or_default();
        let id = tab.id.clone();
        match tabs.iter_mut().find(|one| one.1.id == id) {
            Some(held) => {
                if held.0 != pane && self.view_tabs.front.get(&(key.clone(), held.0)).is_some_and(|front| *front == id) {
                    self.view_tabs.front.remove(&(key.clone(), held.0));
                }
                *held = (pane, tab);
            }
            None => tabs.push((pane, tab)),
        }
        if front {
            self.view_tabs.front.insert((key.clone(), pane), id.clone());
            // In the thread's pane a view tab stands before the thread and the rows' tabs.
            if pane == ViewPane::Thread {
                self.rows_back(&key);
            }
            if pane == ViewPane::Top && self.shown.as_ref() == Some(&key) {
                self.scroll_to_view_tab(&id);
            }
        }
        cx.notify();
    }

    /// Close Job `job`'s tab `id` without asking its veto, and tell its source.
    pub fn close_view_tab(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tabs) = self.view_tabs.by_job.get_mut(job) else { return };
        let Some(at) = tabs.iter().position(|one| one.1.id == id) else { return };
        let (pane, gone) = tabs.remove(at);
        let key = (SharedString::from(job.to_string()), pane);
        if self.view_tabs.front.get(&key).is_some_and(|front| front == id) {
            // The tab before it in its pane comes forward, else the first left there, else the pane's own tab.
            let before = tabs[..at].iter().rev().find(|one| one.0 == pane);
            match before.or_else(|| tabs.iter().find(|one| one.0 == pane)) {
                Some(next) => {
                    let next = next.1.id.clone();
                    self.view_tabs.front.insert(key, next);
                }
                None => {
                    self.view_tabs.front.remove(&key);
                }
            }
        }
        if let Some(closed) = gone.closed {
            window.defer(cx, move |window, cx| closed(window, cx));
        }
        cx.notify();
    }

    /// Give Job `job`'s tab `id` a new title.
    pub fn retitle_view_tab(&mut self, job: &str, id: &str, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        if let Some((_, tab)) = self.view_tabs.by_job.get_mut(job).and_then(|tabs| tabs.iter_mut().find(|one| one.1.id == id)) {
            tab.title = title.into();
            cx.notify();
        }
    }

    /// The ids of Job `job`'s view tabs, in strip order, in every pane.
    pub fn view_tab_ids(&self, job: &str) -> Vec<String> {
        self.view_tabs.by_job.get(job).map(|tabs| tabs.iter().map(|one| one.1.id.clone()).collect()).unwrap_or_default()
    }

    /// The pane Job `job`'s view tab `id` stands in.
    pub fn view_tab_pane(&self, job: &str, id: &str) -> Option<ViewPane> {
        self.view_tabs.by_job.get(job)?.iter().find(|one| one.1.id == id).map(|one| one.0)
    }

    /// Send to agent: a `#` tag of `kind` for `id` goes at the end of Job `job`'s message box, in the
    /// box itself where the Job is on screen and in its kept draft where not.
    pub fn tag_in_draft(&mut self, job: &str, kind: TagKind, id: &str, title: &str, window: &mut Window, cx: &mut Context<Self>) {
        let chip = super::offer::chip_of(kind, id, title, None);
        if self.box_key().as_deref() == Some(job) {
            let input = self.composer.input.clone();
            input.update(cx, |input, cx| {
                let end = input.value().len();
                let lead = !input.value().is_empty() && !input.value().ends_with(char::is_whitespace);
                if lead {
                    input.set_selected_range(end..end, cx);
                    input.insert(" ", window, cx);
                }
                let end = input.value().len();
                if input.replace_range_with_token(end..end, chip, window, cx).is_ok() {
                    input.insert(" ", window, cx);
                }
            });
            self.typed(cx);
            return;
        }
        let mut kept: Kept = self.draft_of(job);
        if !kept.text.is_empty() && !kept.text.ends_with(char::is_whitespace) {
            kept.text.push(' ');
        }
        let start = kept.text.len();
        kept.text.push_str(chip.text());
        kept.tags.push(KeptTag { start, end: kept.text.len(), id: chip.id().to_string(), label: chip.label().to_string() });
        kept.text.push(' ');
        self.keep_draft(job, kept, cx);
    }

    /// The view tab in front of `pane` on the shown Job, where one is.
    pub(super) fn view_front(&self, pane: ViewPane) -> Option<&ViewTab> {
        let job = self.shown.as_ref()?;
        let id = self.view_tabs.front.get(&(job.clone(), pane))?;
        self.view_tabs.by_job.get(job)?.iter().find(|one| one.0 == pane && &one.1.id == id).map(|one| &one.1)
    }

    /// Whether the shown Job has a view tab in `pane`.
    pub(super) fn view_tabs_in(&self, pane: ViewPane) -> bool {
        self.shown.as_ref().and_then(|job| self.view_tabs.by_job.get(job)).is_some_and(|tabs| tabs.iter().any(|one| one.0 == pane))
    }

    /// Ask before an act, in the Job view's dialog: `title`, its lines, the act's button and Cancel. Enter takes
    /// the act, Esc gives it up. `on_yes` runs after the Job view's update.
    #[allow(clippy::too_many_arguments)]
    pub fn confirm(&mut self, title: &str, lines: &[&str], yes: &str, danger: bool, window: &mut Window, cx: &mut Context<Self>, on_yes: Rc<dyn Fn(&mut Window, &mut App)>) {
        let confirm = cx.new(|cx| super::head::Confirm::new(title, lines, ("", Vec::new()), yes, "Cancel", danger, cx));
        self.open_confirm(confirm, window, cx, move |_, window, cx| {
            let on_yes = on_yes.clone();
            window.defer(cx, move |window, cx| on_yes(window, cx));
        });
    }

    /// The tab's title becomes a field in place, as Classic's tab does: Enter keeps it and tells the tab's
    /// `renamed`, leaving keeps it too, Esc puts it back.
    pub fn rename_view_tab(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self.view_tabs.by_job.get(job).and_then(|tabs| tabs.iter().find(|one| one.1.id == id)).map(|one| one.1.title.clone()) else { return };
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(title.to_string(), window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let entered = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_view_rename(true, window, cx),
            _ => {}
        });
        self.view_tabs.renaming = Some(TabRename { job: SharedString::from(job.to_string()), id: id.to_string(), input, _subs: vec![entered] });
        cx.notify();
    }

    fn finish_view_rename(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rename) = self.view_tabs.renaming.take() else { return };
        let said = rename.input.read(cx).value().trim().to_string();
        window.focus(&self.focus.clone(), cx);
        let Some((_, tab)) = self.view_tabs.by_job.get_mut(&rename.job).and_then(|tabs| tabs.iter_mut().find(|one| one.1.id == rename.id)) else { return cx.notify() };
        if keep && !said.is_empty() && said != tab.title.as_ref() {
            tab.title = said.clone().into();
            if let Some(renamed) = tab.renamed.clone() {
                window.defer(cx, move |window, cx| renamed(said, window, cx));
            }
        }
        cx.notify();
    }

    /// A press on one of `pane`'s own tabs puts it in front of the view tabs there.
    pub(super) fn view_tab_back(&mut self, pane: ViewPane) {
        if let Some(job) = self.shown.clone() {
            self.view_tabs.front.remove(&(job, pane));
        }
    }

    fn scroll_to_view_tab(&self, id: &str) {
        let zones = super::side::Zone::ALL.into_iter().filter(|one| !self.tab_closed(one.id())).count();
        let at = self
            .shown
            .as_ref()
            .and_then(|job| self.view_tabs.by_job.get(job))
            .and_then(|tabs| tabs.iter().filter(|one| one.0 == ViewPane::Top).position(|one| one.1.id == id));
        if let Some(at) = at {
            self.panes.top_tabs.scroll_to_item(zones + at);
        }
    }

    /// The shown Job's view tabs in `pane`, for its strip.
    pub(super) fn view_tab_elements(&self, pane: ViewPane, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(job) = self.shown.clone() else { return Vec::new() };
        let front = self.view_front(pane).map(|one| one.id.clone());
        let Some(tabs) = self.view_tabs.by_job.get(&job) else { return Vec::new() };
        tabs.iter()
            .filter(|one| one.0 == pane)
            .map(|(_, one)| {
                let is_front = front.as_deref() == Some(one.id.as_str());
                let key = SharedString::from(format!("view-tab:{}", one.id));
                let (pick, close, menu) = (one.id.clone(), one.id.clone(), one.id.clone());
                let job = (job.clone(), job.clone());
                let field = self.view_tabs.renaming.as_ref().filter(|rename| rename.job == job.0 && rename.id == one.id).map(|rename| {
                    labelled(
                        div()
                            .id("view-tab-rename")
                            .key_context(crate::keymap::TEXT_BOX)
                            .min_w(look.length(crate::look::token::SPACE_12) * 3.)
                            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                if event.keystroke.key == "escape" {
                                    this.finish_view_rename(false, window, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(Input::new(&rename.input).appearance(false).cleanable(false)),
                        Role::TextInput,
                        format!("Rename {}", one.title),
                    )
                });
                let said = if field.is_some() { SharedString::default() } else { one.title.clone() };
                labelled(
                    super::side::tab(look, key.clone(), one.glyph, said, is_front)
                        .children(field)
                        .group(key.clone())
                        .child(
                            labelled(
                                super::tabs::frame(look, SharedString::from(format!("view-tab-close:{}", one.id)), "x")
                                    .mr(look.length(crate::look::token::SPACE_1))
                                    .when(!is_front, |close| close.opacity(0.).group_hover(key.clone(), |style| style.opacity(1.)))
                                    .tooltip(super::head::tip("Close".into()))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.press_close_view_tab(&job.0, &close, window, cx);
                                    })),
                                Role::Button,
                                format!("Close {}", one.title),
                            ),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.view_tabs.front.insert((job.1.clone(), pane), pick.clone());
                            if pane == ViewPane::Thread {
                                this.rows_back(&job.1);
                            }
                            window.focus(&this.focus.clone(), cx);
                            cx.notify();
                        }))
                        .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.open_view_tab_menu(&menu, event.position, window, cx);
                        })),
                    Role::Tab,
                    format!("Job column: {}", one.title),
                )
                .into_any_element()
            })
            .collect()
    }

    /// × on a view tab: its veto first, then the close.
    fn press_close_view_tab(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let veto = self.view_tabs.by_job.get(job).and_then(|tabs| tabs.iter().find(|one| one.1.id == id)).and_then(|one| one.1.closing.clone());
        if veto.is_some_and(|closing| !closing(window, cx)) {
            return;
        }
        self.close_view_tab(job, id, window, cx);
    }

    fn open_view_tab_menu(&mut self, id: &str, at: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(acts) = self.view_tabs.by_job.get(&job).and_then(|tabs| tabs.iter().find(|one| one.1.id == id)).map(|one| one.1.menu.clone()) else { return };
        if acts.is_empty() {
            return;
        }
        let entries = acts.iter().zip(ACT_IDS).map(|(act, id)| Entry::Item { id, label: act.label.clone(), icon: None, danger: act.danger, key: None, off: None }).collect();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Tab", entries, at, gpui::Anchor::TopLeft, false, cx).at_point(window, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.tabs_menu = None;
            if let Some(act) = ACT_IDS.iter().position(|one| one == id).and_then(|at| acts.get(at)).map(|one| one.act.clone()) {
                window.defer(cx, move |window, cx| act(window, cx));
            }
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let back = window.focused(cx);
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.tabs_menu = None;
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.tabs_menu = Some(super::tabs::AddMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    /// Tell the sources which Job came on screen.
    pub(super) fn tell_job_shown(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(hooks) = cx.try_global::<JobShown>().map(|hooks| hooks.0.clone()) else { return };
        let job = self.shown.as_ref().map(|id| id.to_string());
        window.defer(cx, move |window, cx| {
            for hook in &hooks {
                hook(job.clone(), window, cx);
            }
        });
    }
}
