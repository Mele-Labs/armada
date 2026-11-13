//! Tabs another part of the app puts in a Job's panes, each drawing a view of its own: the terminal's
//! first. The source opens one with [`JobView::open_view_tab_in`] in the pane it names ([`ViewPane`]), or
//! with [`JobView::open_view_tab`] in the top pane, offers kinds in every pane's + with
//! [`offer_tab`], hears which Job comes on screen with [`on_job_shown`], and finds the Job view with
//! [`JobView::live`]. A tab's acts, its close veto and its closed hook run after the Job view's own
//! update, so they may update the Job view again; the veto alone runs at once and must not.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{prelude::*, AnyView, App, Context, Entity, SharedString, Subscription, WeakEntity, Window};
use gpui_component::input::{InputEvent, InputState};
use ipc::TagKind;

use super::composer::{Kept, KeptTag};
use super::JobView;

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
pub(super) const ACT_IDS: [&str; 12] = ["view-act:0", "view-act:1", "view-act:2", "view-act:3", "view-act:4", "view-act:5", "view-act:6", "view-act:7", "view-act:8", "view-act:9", "view-act:10", "view-act:11"];

/// Each Job's view tabs, which hold what each draws; the panes hold where each stands and which is in front.
#[derive(Default)]
pub(super) struct ViewTabs {
    pub(super) by_job: HashMap<SharedString, Vec<ViewTab>>,
    /// The tab whose title is a field now.
    pub(super) renaming: Option<TabRename>,
}

pub(super) struct TabRename {
    pub(super) job: SharedString,
    pub(super) id: String,
    pub(super) input: Entity<InputState>,
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
    /// `front` brings it forward in that pane. `pane` names the pane that holds the thread, or the first or second of
    /// the column beside it; with none such, the tab opens as a split.
    pub fn open_view_tab_in(&mut self, job: &str, pane: ViewPane, tab: ViewTab, front: bool, cx: &mut Context<Self>) {
        let key = SharedString::from(job.to_string());
        let id = tab.id.clone();
        let tabs = self.view_tabs.by_job.entry(key.clone()).or_default();
        match tabs.iter_mut().find(|one| one.id == id) {
            Some(held) => *held = tab,
            None => tabs.push(tab),
        }
        self.ensure_placed(&key, cx);
        let terminal = super::place::terminal_key(&id).to_string();
        let held = self.arrangement_of(job).is_some_and(|arrangement| super::pane_tree::panes_of(&arrangement.root).iter().any(|one| one.tabs.iter().any(|tab| tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal.as_str()))));
        // A tab the + of a pane asked for goes to that pane; one that stands already stays where it is unless it is asked to move.
        let aimed = if held { None } else { self.take_aim(job) };
        let target = aimed.or_else(|| self.arrangement_of(job).and_then(|arrangement| super::place::pane_for(arrangement, pane)));
        self.look_in(&key, move |arrangement| std::borrow::Cow::Owned(super::place::put_terminal(arrangement, &terminal, target.as_deref(), front)), cx);
        cx.notify();
    }

    /// The id of the view tab a tree tab stands for.
    pub(super) fn view_id_of(&self, job: &str, tab: &super::pane_tree::Tab) -> Option<String> {
        let terminal = tab.terminal.as_deref()?;
        self.view_tabs.by_job.get(job)?.iter().find(|one| super::place::terminal_key(&one.id) == terminal).map(|one| one.id.clone())
    }

    /// Close Job `job`'s tab `id` without asking its veto, and tell its source.
    pub fn close_view_tab(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tabs) = self.view_tabs.by_job.get_mut(job) else { return };
        let Some(at) = tabs.iter().position(|one| one.id == id) else { return };
        let gone = tabs.remove(at);
        let key = SharedString::from(job.to_string());
        let terminal = super::place::terminal_key(id).to_string();
        let held = self.arrangement_of(job).and_then(|arrangement| {
            super::pane_tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal.as_str())).map(|tab| tab.id.clone())
        });
        if let Some(held) = held {
            self.rearrange_in(&key, None, true, move |arrangement| super::pane_tree::close_tab(arrangement, &held), cx);
        }
        if let Some(closed) = gone.closed {
            window.defer(cx, move |window, cx| closed(window, cx));
        }
        cx.notify();
    }

    /// Give Job `job`'s tab `id` a new title.
    pub fn retitle_view_tab(&mut self, job: &str, id: &str, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        if let Some(tab) = self.view_tabs.by_job.get_mut(job).and_then(|tabs| tabs.iter_mut().find(|one| one.id == id)) {
            tab.title = title.into();
            cx.notify();
        }
    }

    /// The ids of Job `job`'s view tabs, in the order they were opened, in every pane.
    pub fn view_tab_ids(&self, job: &str) -> Vec<String> {
        self.view_tabs.by_job.get(job).map(|tabs| tabs.iter().map(|one| one.id.clone()).collect()).unwrap_or_default()
    }

    /// The place Job `job`'s view tab `id` stands in.
    pub fn view_tab_pane(&self, job: &str, id: &str) -> Option<ViewPane> {
        let arrangement = self.arrangement_of(job)?;
        let terminal = super::place::terminal_key(id);
        let pane = super::pane_tree::panes_of(&arrangement.root).into_iter().find(|pane| pane.tabs.iter().any(|tab| tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal)))?;
        Some(super::place::view_pane_of(arrangement, &pane.id))
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
        let Some(title) = self.view_tabs.by_job.get(job).and_then(|tabs| tabs.iter().find(|one| one.id == id)).map(|one| one.title.clone()) else { return };
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

    pub(super) fn finish_view_rename(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rename) = self.view_tabs.renaming.take() else { return };
        let said = rename.input.read(cx).value().trim().to_string();
        window.focus(&self.focus.clone(), cx);
        let Some(tab) = self.view_tabs.by_job.get_mut(&rename.job).and_then(|tabs| tabs.iter_mut().find(|one| one.id == rename.id)) else { return cx.notify() };
        if keep && !said.is_empty() && said != tab.title.as_ref() {
            tab.title = said.clone().into();
            if let Some(renamed) = tab.renamed.clone() {
                window.defer(cx, move |window, cx| renamed(said, window, cx));
            }
        }
        cx.notify();
    }

    /// × on a view tab: its veto first, then the close.
    pub(super) fn press_close_view_tab(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let veto = self.view_tabs.by_job.get(job).and_then(|tabs| tabs.iter().find(|one| one.id == id)).and_then(|one| one.closing.clone());
        if veto.is_some_and(|closing| !closing(window, cx)) {
            return;
        }
        self.close_view_tab(job, id, window, cx);
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
