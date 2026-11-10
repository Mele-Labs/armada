//! The open Job's head: the Thread tab as the Job panes draw it, its menu, the acts under the
//! message box and the marks that say where the Job came from (`docs/spikes/native-jobs-inventory/
//! job-view.md`, sections 1.1 to 1.7).
//!
//! **A press answers at once.** An act puts what it asks for in an [`Overlay`] the same frame and
//! asks Fleet; the overlay is put back with Fleet's reason when it refuses, and let go of once
//! Fleet's own record has had time to arrive. With no Fleet, as on mock data, it stays.
//!
//! Only the Thread tab is drawn: the other panes and Add tab are a later step, so no entry or
//! button here stands for one.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    actions, anchored, deferred, div, point, prelude::*, svg, Anchor, Animation, AnimationExt, AnyElement, App, Bounds, ClickEvent,
    Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, KeyBinding, KeyDownEvent, MouseButton,
    MouseDownEvent, Pixels, Point, Role, SharedString, Stateful, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{JobState, SessionId, SessionRecord, SessionState, ThreadItem};

use super::wire::Done;
use super::JobView;
use crate::assets::icon_path;
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook, Look};

actions!(job_head, [TabMenu]);

/// The tab's own keys, which the registry does not carry: Electron binds them on the tab itself
/// (`PaneView.tsx:438-447`, Shift+F10 and the Menu key).
pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("shift-f10", TabMenu, Some("job-tab")), KeyBinding::new("menu", TabMenu, Some("job-tab"))]);
}

/// What a press has put on the screen that Fleet's record does not say yet.
#[derive(Clone, Default)]
pub(super) struct Overlay {
    pub paused: Option<bool>,
    pub auto_land: Option<bool>,
    pub stopped: bool,
    pub ended: bool,
    pub naming: bool,
    pub retro: bool,
    pub retried: bool,
}

struct Rename {
    job: SharedString,
    input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

struct OpenMenu {
    menu: Entity<Dropdown>,
    _subs: Vec<Subscription>,
}

struct OpenConfirm {
    confirm: Entity<Confirm>,
    _subs: Vec<Subscription>,
}

pub(super) struct Head {
    overlays: HashMap<SharedString, Overlay>,
    rename: Option<Rename>,
    menu: Option<OpenMenu>,
    confirm: Option<OpenConfirm>,
    /// The tab takes the keys (Shift+F10, Menu, a double press to rename).
    tab: FocusHandle,
    /// The band that holds the handoff's Retry takes `r`.
    band: FocusHandle,
    /// Fleet's reason for the last act it refused, by Job.
    said: HashMap<SharedString, SharedString>,
    /// Jobs whose handoff band was dismissed.
    dismissed: HashSet<SharedString>,
}

impl Head {
    pub fn new(cx: &mut Context<JobView>) -> Self {
        Head {
            overlays: HashMap::new(),
            rename: None,
            menu: None,
            confirm: None,
            tab: cx.focus_handle(),
            band: cx.focus_handle(),
            said: HashMap::new(),
            dismissed: HashSet::new(),
        }
    }

    pub fn stopped(&self, id: &str) -> bool {
        self.overlays.get(id).is_some_and(|overlay| overlay.stopped)
    }

    pub fn ended(&self, id: &str) -> bool {
        self.overlays.get(id).is_some_and(|overlay| overlay.ended)
    }

    pub fn renaming(&self) -> bool {
        self.rename.is_some()
    }
}

/// A sentence a menu entry or tooltip says.
pub(super) fn tip(said: SharedString) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
}

/// Where a labelled element was drawn in the last frame.
pub(super) fn drawn(window: &Window, cx: &App, role: Role, name: &str) -> Option<Bounds<Pixels>> {
    cx.try_global::<LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.bounds)
}

/// A menu's corner under an element's bottom left, as Electron anchors one.
pub(super) fn below_left(bounds: Bounds<Pixels>) -> Point<Pixels> {
    point(bounds.origin.x, bounds.origin.y + bounds.size.height)
}

/// A quiet icon button, `--h-control` square, as the toolbelt and the head draw them.
pub(super) fn icon_button(look: &Look, id: &'static str, icon: &'static str, tip_said: SharedString, lit: bool, off: bool) -> Stateful<gpui::Div> {
    icon_frame(look, id, icon, lit, off).tooltip(tip(tip_said))
}

/// A toolbelt act's button: an icon button whose tooltip wears the act's key.
pub(super) fn act_button(look: &Look, id: &'static str, icon: &'static str, act: &str, tip_said: SharedString, lit: bool, off: bool) -> Stateful<gpui::Div> {
    icon_frame(look, id, icon, lit, off).tooltip(keyed_tip(act, tip_said))
}

fn icon_frame(look: &Look, id: &'static str, icon: &'static str, lit: bool, off: bool) -> Stateful<gpui::Div> {
    let ink = if lit { look.color(token::FG_DEFAULT) } else { look.color(token::FG_SUBTLE) };
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_CONTROL))
        .rounded(look.length(token::RADIUS_MD))
        .text_color(ink)
        .when(lit, |button| button.bg(look.color(token::BG_HOVER)))
        .when(!off, |button| {
            button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
        })
        .child(svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(if lit { look.color(token::ACCENT) } else { ink }))
}

fn item(id: &'static str, label: impl Into<SharedString>, icon: &'static str) -> Entry {
    Entry::Item { id, label: label.into(), icon: Some(icon), danger: false, key: None, off: None }
}

/// A dialog over the window (`Dialog` in Electron): a title, its words, a list, and two buttons.
/// Enter takes the confirm, which has the focus first; Esc and a press outside give it up.
pub(super) struct Confirm {
    title: SharedString,
    body: Vec<SharedString>,
    list_name: SharedString,
    list: Vec<SharedString>,
    confirm: SharedString,
    cancel: SharedString,
    danger: bool,
    on_confirm: bool,
    focus: FocusHandle,
}

pub(super) struct Decided;

impl EventEmitter<Decided> for Confirm {}
impl EventEmitter<DismissEvent> for Confirm {}

impl Focusable for Confirm {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Confirm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(title: &str, body: &[&str], list: (&str, Vec<String>), confirm: &str, cancel: &str, danger: bool, cx: &mut Context<Self>) -> Self {
        Confirm {
            title: title.to_string().into(),
            body: body.iter().map(|line| line.to_string().into()).collect(),
            list_name: list.0.to_string().into(),
            list: list.1.into_iter().map(Into::into).collect(),
            confirm: confirm.to_string().into(),
            cancel: cancel.to_string().into(),
            danger,
            on_confirm: true,
            focus: cx.focus_handle(),
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => cx.emit(DismissEvent),
            "enter" => self.decide(self.on_confirm, cx),
            "left" | "right" | "tab" => {
                self.on_confirm = !self.on_confirm;
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn decide(&mut self, yes: bool, cx: &mut Context<Self>) {
        if yes {
            cx.emit(Decided);
        }
        cx.emit(DismissEvent);
    }
}

impl Render for Confirm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let size = window.viewport_size();
        let button = |id: &'static str, label: SharedString, primary: bool, yes: bool, cx: &Context<Self>| {
            let fill = if primary {
                if self.danger { look.color(token::STATUS_COMPLETED_FAILED) } else { look.color(token::ACCENT) }
            } else {
                look.color(token::BG_RAISED)
            };
            let ring = self.on_confirm == yes;
            labelled(
                div()
                    .id(id)
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_CONTROL_LG))
                    .px(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if ring { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                    .bg(fill)
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(if primary { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_DEFAULT) })
                    .cursor_pointer()
                    .child(label.clone())
                    .on_click(cx.listener(move |this, _, _, cx| this.decide(yes, cx))),
                Role::Button,
                label,
            )
        };
        let list = (!self.list.is_empty()).then(|| {
            labelled(
                div()
                    .id("confirm-list")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .children(self.list.iter().map(|row| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(row.clone()))),
                Role::List,
                self.list_name.clone(),
            )
        });
        let panel = div()
            .id("confirm")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(look.length(token::W_DIALOG))
            .max_w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::SPACE_6))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(self.title.clone()))
            .children(self.body.iter().map(|line| div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(line.clone())))
            .children(list)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(look.length(token::SPACE_2))
                    .child(button("confirm-cancel", self.cancel.clone(), false, false, cx))
                    .child(button("confirm-yes", self.confirm.clone(), true, true, cx)),
            );
        deferred(
            anchored().position(point(gpui::px(0.), gpui::px(0.))).anchor(Anchor::TopLeft).child(
                labelled(
                    div()
                        .id("scrim")
                        .occlude()
                        .w(size.width)
                        .h(size.height)
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(look.color(token::SCRIM))
                        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(DismissEvent)))
                        .child(panel),
                    Role::Dialog,
                    self.title.clone(),
                ),
            ),
        )
        .with_priority(2)
    }
}

impl JobView {
    /// The Job's state glyph, its word and hue, as the tab draws them; the right column's Now reads it too.
    pub(super) fn standing_mark(&self, id: &str, cx: &App) -> Option<(&'static str, SharedString, crate::look::Token, bool)> {
        mark_of(self.record(id, cx), self.head.overlays.get(id))
    }
}

/// How the Job's mark reads: glyph, words, hue, and whether it turns.
fn mark_of(job: Option<&SessionRecord>, overlay: Option<&Overlay>) -> Option<(&'static str, SharedString, crate::look::Token, bool)> {
    let job = job?;
    let state = job.mark?.state;
    let paused = overlay.and_then(|overlay| overlay.paused).unwrap_or_else(|| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()));
    let state = if overlay.is_some_and(|overlay| overlay.ended) {
        JobState::Ended
    } else if paused {
        JobState::Paused
    } else if overlay.is_some_and(|overlay| overlay.stopped) && matches!(state, JobState::Working | JobState::Running | JobState::Subagent) {
        JobState::Idle
    } else {
        state
    };
    let (icon, said) = crate::jobs::glyph_of(state, None);
    Some((icon, said, if state == JobState::Paused { token::FG_MUTED } else { crate::jobs::hue_of(state) }, state == JobState::Working))
}

impl JobView {
    pub(super) fn overlay_mut(&mut self, id: &SharedString) -> &mut Overlay {
        self.head.overlays.entry(id.clone()).or_default()
    }

    pub(super) fn paused(&self, id: &str, cx: &App) -> bool {
        self.head.overlays.get(id).and_then(|overlay| overlay.paused).unwrap_or_else(|| {
            self.record(id, cx).is_some_and(|job| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()))
        })
    }

    pub(super) fn auto_land(&self, id: &str, cx: &App) -> bool {
        self.head.overlays.get(id).and_then(|overlay| overlay.auto_land).unwrap_or_else(|| self.record(id, cx).is_some_and(|job| job.auto_land))
    }

    pub(super) fn title_of(&self, id: &str, cx: &App) -> Option<SharedString> {
        self.record(id, cx).and_then(|job| job.title.clone()).map(Into::into)
    }

    /// What the Job holds that Release gives back: its slot, branches and pull requests.
    fn holds(&self, id: &str, cx: &App) -> Vec<String> {
        self.record(id, cx)
            .map(|job| {
                job.attachments
                    .iter()
                    .filter(|one| one.state == ipc::AttachmentState::Standing && matches!(one.kind.as_str(), "slot" | "branch" | "pr" | "pull_request"))
                    .map(|one| match one.kind.as_str() {
                        "slot" => format!("Slot {}", one.target.rsplit('/').next().unwrap_or(&one.target)),
                        "branch" => format!("Branch {}", one.target),
                        _ => format!("Pull request #{}", one.target),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Fleet's refusal of the last act asked for this Job from its head.
    pub(super) fn head_said(&self, id: &str) -> Option<SharedString> {
        self.head.said.get(id).cloned()
    }

    /// An act Fleet is asked for. `clear` puts the overlay field back, on a refusal with Fleet's
    /// reason, and a beat after an answer once the record has had time to move.
    pub(super) fn ask_act(&mut self, id: SharedString, path: String, body: Result<String, ipc::Unencodable>, clear: fn(&mut Overlay), window: &mut Window, cx: &mut Context<Self>) {
        self.head.said.remove(&id);
        let job = id.clone();
        let done: Done = Box::new(move |this, answer, _, cx| {
            match answer {
                Ok(_) => this.let_go(job, clear, cx),
                Err(why) => {
                    clear(this.overlay_mut(&job));
                    this.head.said.insert(job, why.into());
                }
            }
            cx.notify();
        });
        self.post(path, body, window, cx, done);
        cx.notify();
    }

    /// Let go of an overlay field after Fleet's record has had time to arrive. On mock data nothing
    /// moves the record, so the field stays unless `mock_lets_go`.
    fn let_go(&mut self, id: SharedString, clear: fn(&mut Overlay), cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(2500)).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(overlay) = this.head.overlays.get_mut(&id) {
                    clear(overlay);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn session_id(id: &SharedString) -> SessionId {
        SessionId::carried(id.to_string())
    }

    // ---- the menu ------------------------------------------------------------------------

    /// Fork is offered for an ended Job, and for a Job whose context or age says a new one would
    /// serve (`sessions.tsx:329-330`).
    fn fork_offered(&self, id: &str, cx: &App) -> bool {
        let Some(job) = self.record(id, cx) else { return false };
        if self.closed(id, cx) {
            return true;
        }
        let tokens = job.usage.context_tokens.unwrap_or(0);
        let hours = self
            .thread(id, cx)
            .and_then(|held| chrono::DateTime::parse_from_rfc3339(held.head.trail.started.as_str()).ok())
            .map_or(0, |started| (chrono::Utc::now() - started.with_timezone(&chrono::Utc)).num_hours());
        tokens > 300_000 || hours >= 24
    }

    fn head_entries(&self, id: &SharedString, cx: &App) -> Vec<Entry> {
        let Some(job) = self.record(id, cx) else { return Vec::new() };
        let overlay = self.head.overlays.get(id.as_str());
        let live = !self.closed(id, cx);
        let hosted = job.hosted.is_some();
        let mut groups: Vec<Vec<Entry>> = Vec::new();
        let mut names = vec![item("rename", "Rename", "square-pen")];
        if hosted {
            names.push(if overlay.is_some_and(|overlay| overlay.naming) {
                Entry::Item { id: "name", label: "Naming".into(), icon: Some("wand-sparkles"), danger: false, key: None, off: Some("Fleet is naming the Job".into()) }
            } else {
                item("name", "Name it for me", "wand-sparkles")
            });
        }
        groups.push(names);
        let mut acts = Vec::new();
        if live && self.working(id, cx) {
            acts.push(item("stop", "Stop", "square"));
        }
        if live && hosted {
            acts.push(if self.paused(id, cx) { item("resume", "Resume", "play") } else { item("pause", "Pause", "circle-pause") });
            acts.push(Entry::Toggle { id: "land", label: "Auto land".into(), on: self.auto_land(id, cx) });
            let holds = self.holds(id, cx).is_empty();
            acts.push(Entry::Item {
                id: "release",
                label: "Release".into(),
                icon: Some("unplug"),
                danger: false,
                key: None,
                off: holds.then(|| "The Job holds nothing".into()),
            });
        }
        groups.push(acts);
        let mut ends = Vec::new();
        if self.fork_offered(id, cx) {
            ends.push(Entry::Item {
                id: "fork",
                label: "Fork".into(),
                icon: Some("git-fork"),
                danger: false,
                key: None,
                off: job.manifest_id.is_none().then(|| "The repository is not known".into()),
            });
        }
        ends.push(if overlay.is_some_and(|overlay| overlay.retro) {
            Entry::Item { id: "retro", label: "Writing the retro".into(), icon: Some("rewind"), danger: false, key: None, off: Some("Fleet is writing it".into()) }
        } else {
            item("retro", "Retro", "rewind")
        });
        if live && job.state != SessionState::Ended {
            ends.push(Entry::Item { id: "end", label: "End job".into(), icon: Some("power-off"), danger: true, key: None, off: None });
        }
        groups.push(ends);
        let mut entries = Vec::new();
        for group in groups.into_iter().filter(|group| !group.is_empty()) {
            if !entries.is_empty() {
                entries.push(Entry::Rule);
            }
            entries.extend(group);
        }
        entries
    }

    /// The tab's menu at `at`, from More, a right press on the tab or the keys. The keys go back
    /// to where they were when it shuts, unless the act that was picked took them (Rename).
    pub(super) fn open_head_menu(&mut self, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let entries = self.head_entries(&id, cx);
        if entries.is_empty() {
            return;
        }
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Tab actions", entries, at, Anchor::TopLeft, from_keys, cx).at_point(window, cx));
        let picked = cx.subscribe_in(&menu, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: act, .. } = picked;
            this.head_act(act, window, cx);
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &DismissEvent, window, cx| {
            this.head.menu = None;
            let tab = this.head.tab.clone();
            window.focus(back.as_ref().unwrap_or(&tab), cx);
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.head.menu = Some(OpenMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    fn on_tab_menu(&mut self, _: &TabMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.shown.as_ref().and_then(|id| drawn(window, cx, Role::Row, &self.tab_name(id, cx))).map(below_left) else { return };
        self.open_head_menu(at, true, window, cx);
    }

    fn tab_name(&self, id: &str, cx: &App) -> SharedString {
        self.title_of(id, cx).map_or_else(|| "Thread".into(), |title| format!("Thread: {title}").into())
    }

    pub(super) fn head_act(&mut self, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let session_id = Self::session_id(&id);
        let encoded = |body: Result<String, ipc::Unencodable>| body;
        match act {
            "rename" => cx.defer_in(window, |this, window, cx| this.start_rename(window, cx)),
            "name" => {
                self.overlay_mut(&id).naming = true;
                self.ask_act(id, "/sessions/name".into(), encoded(ipc::encode(&ipc::NameSession { session_id })), |overlay| overlay.naming = false, window, cx);
            }
            "stop" => {
                self.overlay_mut(&id).stopped = true;
                self.ask_act(id, "/sessions/stop".into(), encoded(ipc::encode(&ipc::StopSession { session_id })), |overlay| overlay.stopped = false, window, cx);
            }
            "pause" => {
                self.overlay_mut(&id).paused = Some(true);
                self.ask_act(id, "/sessions/pause".into(), encoded(ipc::encode(&ipc::PauseSession { session_id })), |overlay| overlay.paused = None, window, cx);
            }
            "resume" => {
                self.overlay_mut(&id).paused = Some(false);
                self.ask_act(id, "/sessions/resume".into(), encoded(ipc::encode(&ipc::ResumeSession { session_id })), |overlay| overlay.paused = None, window, cx);
            }
            "land" => {
                let on = !self.auto_land(&id, cx);
                self.overlay_mut(&id).auto_land = Some(on);
                self.ask_act(
                    id,
                    "/sessions/auto_land".into(),
                    encoded(ipc::encode(&ipc::SetSessionAutoLand { id: session_id, on })),
                    |overlay| overlay.auto_land = None,
                    window,
                    cx,
                );
            }
            "retro" => {
                self.overlay_mut(&id).retro = true;
                self.head.said.remove(&id);
                let job = id.clone();
                let done: Done = Box::new(move |this, answer, _, cx| {
                    this.overlay_mut(&job).retro = false;
                    if let Err(why) = answer {
                        this.head.said.insert(job, why.into());
                    }
                    cx.notify();
                });
                self.post_bare(format!("/sessions/{}/retro", super::wire::encoded(&id)), window, cx, done);
                cx.notify();
            }
            "fork" => self.fork(&id, window, cx),
            "release" => self.ask_release(&id, window, cx),
            "end" => self.ask_end(&id, window, cx),
            _ => {}
        }
        cx.notify();
    }

    /// Start a Job on a copy of this one's conversation, ending at its last message, and open it.
    fn fork(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest_id) = self.record(id, cx).and_then(|job| job.manifest_id.clone()) else { return };
        let at = (!self.closed(id, cx))
            .then(|| {
                self.thread(id, cx).and_then(|held| {
                    held.items.iter().rev().find_map(|item| match item {
                        ThreadItem::Row { row: ipc::SessionRow::Message { id, .. } } => Some(id.clone()),
                        _ => None,
                    })
                })
            })
            .flatten();
        let start = ipc::StartSession {
            manifest_id,
            title: None,
            model: None,
            effort: None,
            mode: None,
            fork: Some(ipc::ForkFrom { session_id: Self::session_id(id), at }),
            root: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        self.head.said.remove(id);
        let job = id.clone();
        let done: Done = Box::new(move |this, answer, _, cx| {
            match answer {
                Ok(bytes) => {
                    if let Ok(record) = ipc::decode::<SessionRecord>("SessionRecord", &bytes) {
                        this.open_job(record.id.as_str().to_string(), cx);
                    }
                }
                Err(why) => {
                    this.head.said.insert(job, why.into());
                }
            }
            cx.notify();
        });
        self.post("/sessions/start".into(), ipc::encode(&start), window, cx, done);
    }

    fn ask_release(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let holds = self.holds(id, cx);
        if holds.is_empty() {
            return;
        }
        let confirm = cx.new(|cx| {
            Confirm::new(
                "Release everything the Job holds",
                &["What it holds goes back. Nothing is deleted."],
                ("Holds", holds),
                "Release all",
                "Keep",
                false,
                cx,
            )
        });
        let job = id.clone();
        self.open_confirm(confirm, window, cx, move |this, window, cx| {
            let body = ipc::encode(&ipc::ReleaseSessionHolds { session_id: Self::session_id(&job), to_session_id: None, rows: None });
            this.ask_act(job.clone(), "/sessions/release_holds".into(), body, |_| {}, window, cx);
        });
    }

    /// End job: at once if the Job holds nothing in flight, else after the list says what ending loses.
    pub(super) fn ask_end(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let in_flight: Vec<String> = self
            .record(id, cx)
            .map(|job| job.in_flight.iter().map(|one| if one.state.is_empty() { one.label.clone() } else { format!("{}: {}", one.label, one.state) }).collect())
            .unwrap_or_default();
        let job = id.clone();
        if in_flight.is_empty() {
            self.end(&job, window, cx);
            return;
        }
        let confirm = cx.new(|cx| {
            Confirm::new("In flight", &["Ending the Job leaves these where they stand."], ("In flight", in_flight), "End job", "Keep going", true, cx)
        });
        self.open_confirm(confirm, window, cx, move |this, window, cx| this.end(&job, window, cx));
    }

    fn end(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.overlay_mut(id).ended = true;
        let body = ipc::encode(&ipc::CloseSession { session_id: Self::session_id(id) });
        self.ask_act(id.clone(), "/sessions/close".into(), body, |overlay| overlay.ended = false, window, cx);
    }

    pub(super) fn open_confirm(&mut self, confirm: Entity<Confirm>, window: &mut Window, cx: &mut Context<Self>, yes: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) {
        let back = window.focused(cx);
        let decided = cx.subscribe_in(&confirm, window, move |this, _, _: &Decided, window, cx| yes(this, window, cx));
        let dismissed = cx.subscribe_in(&confirm, window, move |this, _, _: &DismissEvent, window, cx| {
            this.head.confirm = None;
            let tab = this.head.tab.clone();
            window.focus(back.as_ref().unwrap_or(&tab), cx);
            cx.notify();
        });
        let keys = confirm.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.head.confirm = Some(OpenConfirm { confirm, _subs: vec![decided, dismissed] });
        cx.notify();
    }

    // ---- rename ------------------------------------------------------------------------

    /// The title becomes a field in the tab: Enter and leaving save, Esc puts it back (1.1).
    pub(super) fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.head.rename.is_some() {
            return;
        }
        let title = self.title_of(&id, cx).unwrap_or_default();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(title.to_string(), window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let entered = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_rename(true, window, cx),
            _ => {}
        });
        self.head.rename = Some(Rename { job: id, input, _subs: vec![entered] });
        cx.notify();
    }

    fn finish_rename(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rename) = self.head.rename.take() else { return };
        let was = self.title_of(&rename.job, cx).unwrap_or_default();
        let said = rename.input.read(cx).value().trim().to_string();
        let tab = self.head.tab.clone();
        window.focus(&tab, cx);
        if keep && !said.is_empty() && said != was.as_ref() {
            // The list's record takes the title, so the tab, the row, the review sheet and the palette read one.
            self.list.update(cx, |list, cx| list.retitle(&rename.job, said.clone(), cx));
            let body = ipc::encode(&ipc::RenameSession { session_id: Self::session_id(&rename.job), title: said });
            let job = rename.job;
            self.head.said.remove(&job);
            let done: Done = Box::new(move |this, answer, _, cx| {
                if let Err(why) = answer {
                    this.list.update(cx, |list, cx| list.untitle(&job, cx));
                    this.head.said.insert(job, why.into());
                }
                cx.notify();
            });
            self.post("/sessions/rename".into(), body, window, cx, done);
        }
        cx.notify();
    }

    // ---- drawing -----------------------------------------------------------------------

    /// The head: the Job's Thread tab (1.1). Its mark says the state, its title is the label, and
    /// More opens the menu. A tab strip with the Thread tab only.
    pub(super) fn render_head(&self, id: &SharedString, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let job = self.record(id, cx);
        let overlay = self.head.overlays.get(id.as_str());
        let title: SharedString = self.title_of(id, cx).unwrap_or_else(|| "Thread".into());
        let name = self.tab_name(id, cx);
        let mark = mark_of(job, overlay);
        let glyph = look.length(token::SPACE_4);
        let state = mark.map(|(icon, said, hue, working)| {
            let said_in: SharedString = format!("{said} \u{b7} {id}").into();
            let ink = look.color(hue);
            let drawn = if working {
                svg()
                    .path(icon_path("loader-circle"))
                    .size(glyph)
                    .text_color(ink)
                    .with_animation(
                        "head-working",
                        Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                        |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                    )
                    .into_any_element()
            } else {
                svg().path(icon_path(icon)).size(glyph).text_color(ink).into_any_element()
            };
            labelled(div().id("head-state").flex_none().child(drawn).tooltip(tip(said_in)), Role::Image, said)
        });
        let working = self.working(id, cx) && !self.closed(id, cx);
        let stop = working.then(|| {
            labelled(
                icon_button(look, "head-stop", "square", "Stop".into(), false, false)
                    .on_click(cx.listener(|this, _, window, cx| this.head_act("stop", window, cx))),
                Role::Button,
                "Stop",
            )
        });
        let more = labelled(
            icon_button(look, "head-more", "ellipsis", "More".into(), self.head.menu.is_some(), false).on_click(cx.listener(|this, _, window, cx| {
                let at = drawn(window, cx, Role::Button, "More").map_or_else(|| point(gpui::px(0.), gpui::px(0.)), below_left);
                this.open_head_menu(at, false, window, cx);
            })),
            Role::Button,
            "More",
        );
        let label: AnyElement = match &self.head.rename {
            Some(rename) => labelled(
                div()
                    .id("head-rename")
                    .key_context(crate::keymap::TEXT_BOX)
                    .flex_1()
                    .min_w(look.length(token::SPACE_12) * 5.)
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            this.finish_rename(false, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Input::new(&rename.input).appearance(false).cleanable(false)),
                Role::TextInput,
                format!("Rename {title}"),
            )
            .into_any_element(),
            None => div()
                .id("head-title")
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(title.clone())
                .tooltip(tip(title.clone()))
                .into_any_element(),
        };
        let tab = labelled(
            div()
                .id("head-tab")
                .key_context("job-tab")
                .track_focus(&self.head.tab)
                .on_action(cx.listener(Self::on_tab_menu))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, event: &MouseDownEvent, window, cx| this.open_head_menu(event.position, false, window, cx)),
                )
                .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                    if event.click_count() >= 2 {
                        this.start_rename(window, cx);
                    } else if !this.head.renaming() {
                        let tab = this.head.tab.clone();
                        window.focus(&tab, cx);
                    }
                }))
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_1))
                .bg(look.color(token::BG_RAISED))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_GLASS))
                .children(state)
                .child(label)
                .children(stop)
                .child(more),
            Role::Row,
            name,
        );
        let said = self.head.said.get(id).cloned().map(|why| {
            labelled(
                div()
                    .id("head-said")
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(why),
                Role::Status,
                "Not done",
            )
        });
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_stretch()
                    .h(look.length(token::H_CONTROL_LG))
                    .bg(look.color(token::BG_SUNKEN))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_GLASS))
                    .child(div().max_w(look.length(token::SPACE_12) * 8.).min_w_0().flex().h_full().child(tab)),
            )
            .children(said)
            .children(self.head.menu.as_ref().map(|open| open.menu.clone()))
            .children(self.head.confirm.as_ref().map(|open| open.confirm.clone()))
            .into_any_element()
    }

    /// The Jobs this one came from, above its thread: Forked from, Started by (1.6). A press opens
    /// that Job.
    pub(super) fn render_origin(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let job = self.record(id, cx)?;
        let from = |kind: &str| job.attachments.iter().find(|one| one.kind == kind).map(|one| one.target.clone());
        let lines: Vec<(&'static str, String)> = [("forked_from", "Forked from"), ("started_by", "Started by")]
            .into_iter()
            .filter_map(|(kind, words)| from(kind).map(|target| (words, target)))
            .collect();
        if lines.is_empty() {
            return None;
        }
        let rows = lines.into_iter().map(|(words, target)| {
            let title: SharedString = self.record(&target, cx).and_then(|origin| origin.title.clone()).unwrap_or_else(|| target.clone()).into();
            let name: SharedString = format!("{words} {title}").into();
            labelled(
                div()
                    .id(SharedString::from(format!("origin-{words}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                    .child(name.clone())
                    .on_click(cx.listener(move |this, _, _, cx| this.open_job(target.clone(), cx))),
                Role::Button,
                name,
            )
        });
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .pt(look.length(token::SPACE_3))
                .children(rows)
                .into_any_element(),
        )
    }

    /// The band on the message box's top edge when handing off to a new Job failed (1.7): its
    /// reason, and Retry, which `r` presses while the band has the keys.
    pub(super) fn render_handoff_band(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let job = self.record(id, cx)?;
        let failed = job.attachments.iter().find(|one| one.kind == "handoff_failed" && one.state == ipc::AttachmentState::Standing)?;
        if self.head.dismissed.contains(id) || self.head.overlays.get(id.as_str()).is_some_and(|overlay| overlay.retried) {
            return None;
        }
        let why: SharedString = failed.detail.get("reason").cloned().unwrap_or_default().into();
        let hue = look.color(token::STATUS_COMPLETED_FAILED);
        let retry = labelled(
            div()
                .id("handoff-retry")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .child(svg().path(icon_path("rotate-cw")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                .child("Retry")
                .child(div().text_color(look.color(token::FG_SUBTLE)).child("r"))
                .on_click(cx.listener(|this, _, window, cx| this.retry_handoff(window, cx))),
            Role::Button,
            "Retry",
        );
        let fix = labelled(
            div()
                .id("handoff-dispatch")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .child("Dispatch job to fix")
                .child(div().text_color(look.color(token::FG_SUBTLE)).child("d"))
                .on_click(cx.listener(|this, _, window, cx| this.dispatch_handoff_fix(window, cx))),
            Role::Button,
            "Dispatch job to fix",
        );
        let band = labelled(
            div()
                .id("handoff-band")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_3))
                .rounded_t(look.length(token::RADIUS_MD))
                .bg(hue)
                .text_size(look.length(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_INVERSE))
                .cursor_pointer()
                .child(svg().path(icon_path("x")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_INVERSE)))
                .child("FAILED TO HAND OFF TO NEW JOB")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(id) = this.shown.clone() {
                        this.head.dismissed.insert(id);
                    }
                    cx.notify();
                })),
            Role::Button,
            "Failed to hand off to new job",
        );
        Some(
            div()
                .id("handoff")
                .key_context("thread")
                .track_focus(&self.head.band)
                .flex_none()
                .flex()
                .flex_col()
                .child(band)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .border_x(look.length(token::BORDER_WIDTH))
                        .border_color(hue)
                        .bg(look.color(token::BG_RAISED))
                        .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(why))
                        .child(retry)
                        .child(fix),
                )
                .into_any_element(),
        )
    }

    pub(super) fn retry_handoff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.overlay_mut(&id).retried = true;
        // The band goes with the press, so the keys go back to the box.
        self.to_composer(window, cx);
        let body = ipc::encode(&ipc::RetryHandoff { session_id: Self::session_id(&id) });
        self.ask_act(id, "/sessions/handoff/retry".into(), body, |overlay| overlay.retried = false, window, cx);
    }

    /// Dispatch job to fix (`handoff_dispatch`, `d`): a Job on Armada's own repository, told the failure, and
    /// opened, as Electron's failure toast dispatches one (`failure-session.ts`).
    pub(super) fn dispatch_handoff_fix(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(record) = self.record(&id, cx).cloned() else { return };
        let Some(manifest_id) = self.report_manifest(&record, cx) else { return };
        let reason = record.attachments.iter().find(|one| one.kind == "handoff_failed").and_then(|one| one.detail.get("reason").cloned());
        let mut text = format!("Failed to hand off to new job\n\ncode: handoff_failed\nsession_id: {id}");
        if let Some(reason) = reason {
            text.push_str(&format!("\nreason: {reason}"));
        }
        let start = ipc::StartSession {
            manifest_id,
            title: None,
            model: None,
            effort: None,
            mode: None,
            fork: None,
            root: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        let done: Done = Box::new(move |this, answer, window, cx| {
            let Ok(bytes) = answer else { return };
            let Ok(started) = ipc::decode::<SessionRecord>("SessionRecord", &bytes) else { return };
            let fixer = started.id.as_str().to_string();
            let body = ipc::SendSessionMessageBody {
                message: ipc::SendSessionMessage { session_id: SessionId::carried(fixer.clone()), text, attachments: Vec::new(), mentions: Vec::new() },
                aim: None,
            };
            let opened: Done = Box::new(move |this, answer, _, cx| {
                if answer.is_ok() {
                    this.open_job(fixer, cx);
                }
            });
            this.post("/sessions/message".into(), ipc::encode(&body), window, cx, opened);
        });
        self.post("/sessions/start".into(), ipc::encode(&start), window, cx, done);
    }

    /// Whether `r` presses Retry: the band is up and has the keys.
    pub(super) fn handoff_band_focused(&self, window: &Window, cx: &App) -> bool {
        self.shown.as_ref().is_some_and(|id| {
            self.record(id, cx).is_some_and(|job| job.attachments.iter().any(|one| one.kind == "handoff_failed" && one.state == ipc::AttachmentState::Standing))
        }) && self.head.band.contains_focused(window, cx)
    }

    /// The Job's acts and marks, under the message box (1.5): Auto land, Pause or Resume, Release,
    /// then the marks. The acts that open panes and sheets later are not drawn.
    pub(super) fn render_toolbelt(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let look = cx.look();
        let job = self.record(id, cx)?;
        let hosted = job.hosted.is_some();
        let live = !self.closed(id, cx);
        let paused = self.paused(id, cx);
        let land = self.auto_land(id, cx);
        let pending = self.fleet.is_some();
        let overlay = self.head.overlays.get(id.as_str());
        let sending = |was: bool, now: bool| pending && was != now;
        // Land, Pause and Release act on a hosted Job that is live; the Judge reads any Job.
        let hosted_acts = hosted && live;
        let land_act = hosted_acts.then(|| {
            let land_tip: SharedString =
                if overlay.and_then(|overlay| overlay.auto_land).is_some() && sending(job.auto_land, land) { "Sending".into() } else if land { "Auto land on".into() } else { "Auto land off".into() };
            labelled(
                act_button(look, "act-land", "plane-landing", "land", land_tip, land, false).on_click(cx.listener(|this, _, window, cx| this.head_act("land", window, cx))),
                Role::Button,
                "Auto land",
            )
        });
        let pause_act = hosted_acts.then(|| {
            let paused_was = job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some());
            let pause_name: SharedString = if paused { "Resume".into() } else { "Pause".into() };
            let pause_tip: SharedString = if sending(paused_was, paused) { "Sending".into() } else if paused { "Paused".into() } else { "Pause".into() };
            labelled(
                act_button(look, "act-pause", if paused { "play" } else { "circle-pause" }, "pause", pause_tip, paused, false).on_click(cx.listener(
                    move |this, _, window, cx| this.head_act(if paused { "resume" } else { "pause" }, window, cx),
                )),
                Role::Button,
                pause_name,
            )
        });
        let release_act = hosted_acts.then(|| {
            let holds = !self.holds(id, cx).is_empty();
            let release_tip: SharedString = if holds { "Release or hand over everything the Job holds".into() } else { "The Job holds nothing".into() };
            labelled(
                act_button(look, "act-release", "unplug", "release", release_tip, false, !holds).on_click(cx.listener(|this, _, window, cx| this.head_act("release", window, cx))),
                Role::Button,
                "Release everything the Job holds",
            )
        });
        let acts = Some(labelled(
            div()
                .id("job-acts")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .children(land_act)
                .children(pause_act)
                .child(self.judge_act(id, &look, cx))
                .child(self.report_act(id, cx))
                .children(release_act),
            Role::Toolbar,
            "Job acts",
        ));
        // The context ring after the acts, as `SessionFrame`'s tools order it. Compact goes as a line, as a
        // quick line does: the native app types into no terminal, so Fleet refuses what it cannot deliver.
        let ring = crate::context_ring::Fill::of(&job.usage, job.context_breakdown.as_ref(), job.compact).map(|fill| {
            let me = cx.entity().downgrade();
            crate::context_ring::ContextRing::new(id.clone(), Some(fill)).on_compact(Rc::new(move |_, window, cx| {
                if let Some(me) = me.upgrade() {
                    me.update(cx, |this, cx| this.send_line("/compact".into(), window, cx));
                }
            }))
        });
        let marks = {
            let stale = job.mod_out_of_date && !matches!(job.mark.map(|mark| mark.state), Some(JobState::Quiet));
            let mod_mark = stale.then(|| {
                labelled(
                    div()
                        .id("mark-mod")
                        .flex_none()
                        .child(svg().path(icon_path("rotate-cw")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                        .tooltip(tip("Mod out of date: run /reload-plugins".into())),
                    Role::Image,
                    "Mod out of date: run /reload-plugins",
                )
            });
            let tools_mark = job.tools_missing.as_ref().map(|missing| {
                let said: SharedString = format!(
                    "Tools missing: {}",
                    missing
                        .missing
                        .iter()
                        .map(|one| match one {
                            ipc::ToolMissing::Mod => "the armada mod",
                            ipc::ToolMissing::Door => "Fleet's door",
                            ipc::ToolMissing::TaskTools => "task tools",
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into();
                labelled(
                    div()
                        .id("mark-tools")
                        .flex_none()
                        .child(svg().path(icon_path("unplug")).size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_AWAITING_REVIEW)))
                        .tooltip(tip(said.clone())),
                    Role::Image,
                    said,
                )
            });
            div().flex().items_center().gap(look.length(token::SPACE_2)).children(mod_mark).children(tools_mark)
        };
        let aa = labelled(
            div()
                .id("aa")
                .track_focus(&self.composer.aa)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_SUBTLE))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .aria_expanded(self.composer.popup.is_some())
                .child("Aa")
                .tooltip(tip("Thread look".into()))
                .on_click(cx.listener(|this, _, window, cx| this.open_aa(window, cx))),
            Role::Button,
            "Thread look",
        );
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL_SM))
                .mt(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .child(aa)
                .children(acts)
                .children(ring)
                .child(div().flex_1())
                .child(marks)
                .into_any_element(),
        )
    }
}

/// The toolbelt's keys, as Electron's flight controls wear them (`flight-order.ts`): ⌥⇧6 Auto land,
/// ⌥⇧7 Judge, ⌥⇧8 Report, ⌥⇧9 Release, and `p` Pause. ⌥⇧1 to ⌥⇧5 are the Job head's views.
const TILE_KEYS: [(&str, &str); 5] = [("land", "alt-shift-6"), ("judge", "alt-shift-7"), ("report", "alt-shift-8"), ("release", "alt-shift-9"), ("pause", "p")];

/// A toolbelt act's tooltip, with the key it wears beside the sentence, as Electron's `Tooltip shortcut`.
pub(super) fn keyed_tip(act: &str, said: SharedString) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    let key = TILE_KEYS.iter().find(|(one, _)| *one == act).and_then(|(_, key)| gpui::Keystroke::parse(key).ok());
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx)
}

impl JobView {
    /// A press of a toolbelt act's key, from anywhere on the deck but a text box, where ⌥⇧ types a
    /// character, and not while a dialog or menu is over the Job.
    pub fn tile_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        if self.reporting.is_some() || self.head.confirm.is_some() || self.composer.popup.is_some() {
            return false;
        }
        if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            return false;
        }
        let keys = &event.keystroke;
        let bare = !keys.modifiers.modified();
        let chord = keys.modifiers.alt && keys.modifiers.shift && !keys.modifiers.platform && !keys.modifiers.control;
        let hosted = self.record(&id, cx).is_some_and(|job| job.hosted.is_some()) && !self.closed(&id, cx);
        match (chord, bare, keys.key.as_str()) {
            (true, _, "6") if hosted => self.head_act("land", window, cx),
            (true, _, "7") => self.open_judge_pane(window, cx),
            (true, _, "8") if self.record(&id, cx).is_some_and(|record| self.report_manifest(record, cx).is_some()) => self.toggle_report(window, cx),
            (true, _, "9") if hosted && !self.holds(&id, cx).is_empty() => self.head_act("release", window, cx),
            (_, true, "p") if hosted => {
                let paused = self.paused(&id, cx);
                self.head_act(if paused { "resume" } else { "pause" }, window, cx);
            }
            _ => return false,
        }
        true
    }
}
