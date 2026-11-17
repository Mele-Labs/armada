//! A Job's Browser tab: the agent's mark, Back, Forward and Reload, and the address, over the page. Classic's `JobBrowserPane`
//! (`job-browser/JobBrowserPane.tsx`, `job-browser.css`), in native's own components.
//!
//! **The page is a WKWebView laid over the plain flex body this tab draws** (`crate::web`), never inside a scrolling parent, where a
//! clip would leave a still picture. It lives as long as the tab does: hidden, not destroyed, while another tab is in front.
//! The page's address, title and history are heard as WebKit changes them (key-value observing), so a navigation inside the
//! page moves the address too.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    actions, div, prelude::*, svg, Animation, AnimationExt, AnyElement, App, AsyncWindowContext, Context, Entity, EventEmitter, FocusHandle,
    KeyBinding, Role, SharedString, Subscription, Task, WeakEntity, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};

use super::acts::is_web;
use super::asks::{self, decide, AskSlot, Rules};
use super::choices::{host_of, Answer, Choice, Permission, Settings, Store, NO_REPOSITORY};
use super::rules::{self, rules_json};
use super::site_panel::{self, press_of, stepped, Press, Scope, SiteState};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::web::{web_pane, Request, Said, Status, WebPage, WebPageEvent};

actions!(job_browser, [DismissAsk]);

/// The strip's own key. Classic hears Esc on the window while its strip is up (`AskStrip`); here it is the strip's, where focus is.
pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", DismissAsk, Some(ASK_CONTEXT))]);
    site_panel::bind(cx);
}

const ASK_CONTEXT: &str = "job-browser-ask";

/// What a Browser tab tells the Job view that holds it, to tell Fleet.
pub enum BrowserTabEvent {
    /// The page asks the owner, and Fleet is to carry the ask in Needs you.
    Raised { tab: String, text: String, options: Vec<ipc::WaitingOption> },
    /// The ask is over, though not by Fleet's own answer.
    Ended { tab: String },
}

impl EventEmitter<BrowserTabEvent> for BrowserTab {}

/// The size a page is laid out at before a pane has drawn it, so an act on a tab nobody is looking at reaches a page of a
/// usual size.
const UNSEEN: (f64, f64) = (1024., 768.);

/// What an agent's mark says while nothing runs.
pub const NO_ACT: &str = "No agent act running";

/// Whose key each of the head's three acts is: the history act's two slots, and Reload's own.
pub const BACK_KEY: &str = "\u{2318}[";
pub const FORWARD_KEY: &str = "\u{2318}]";
pub const RELOAD_KEY: &str = "\u{2318}R";

/// Each Job's Browser tab that was drawn last, shared by every tab, for the Job's page to choose by (`jobs_page`).
pub type LastShown = Rc<RefCell<HashMap<String, String>>>;

pub struct BrowserTab {
    job: SharedString,
    /// The tab's id in the Job's panes, or [`super::UNATTACHED`] for a page an agent opened before any tab was there to hold it.
    tab: String,
    page: Entity<WebPage>,
    address: Entity<InputState>,
    /// The page's address as the page last told it, which the field shows until something else is typed.
    url: String,
    title: String,
    back: bool,
    forward: bool,
    /// The sentence of the act running on this page, where one is.
    acting: Option<SharedString>,
    /// Loads an act is waiting on, each with the address it asked for.
    waiting: Vec<(async_channel::Sender<Result<(), String>>, String)>,
    last: LastShown,
    /// What Back, Forward and Reload are reached by Tab with, in that order.
    keys: [FocusHandle; 3],
    /// The repository whose choices answer this page: the Job's manifest, else `none`.
    repository: String,
    /// Where the owner's choices are kept: Classic's own file. None where the Mac has no home folder.
    store: Option<Store>,
    /// The one ask the page may have waiting, and what answers the page that made it.
    asks: AskSlot<async_channel::Sender<Answer>>,
    /// The strip's focus: its frame, where Esc is heard, and its two presses in Tab order.
    strip: [FocusHandle; 3],
    /// Where the focus was before the strip took it, which it goes back to.
    before: Option<FocusHandle>,
    /// The lock's popover, and the confirmation Allow on Local network waits on.
    pub(super) site: SiteState,
    /// The content rule list the page was last given, which is none (`Some(None)`) where nothing is refused; `None` before the first.
    rules_seen: Option<Option<String>>,
    _subscriptions: Vec<Subscription>,
    _changes: Task<()>,
    _asked: Task<()>,
}

impl BrowserTab {
    pub fn new(job: SharedString, tab: String, last: LastShown, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let page = cx.new(WebPage::for_job_browser);
        let address = cx.new(|cx| InputState::new(window, cx).placeholder("Address"));
        let hear = page.update(cx, |page, _| page.watch_changes());
        let loaded = cx.subscribe_in(&page, window, |this, _, event: &WebPageEvent, window, cx| this.page_told(event, window, cx));
        let entered = cx.subscribe_in(&address, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.navigate(window, cx);
            }
        });
        let requests = page.update(cx, |page, _| page.hear_requests());
        let asked = cx.spawn_in(window, async move |this, cx| {
            while let Ok(request) = requests.recv().await {
                if this.update_in(cx, |this, window, cx| this.requested(request, window, cx)).is_err() {
                    break;
                }
            }
        });
        let changes = cx.spawn_in(window, async move |this, cx| {
            while hear.recv().await.is_ok() {
                // A navigation moves several of the page's properties at once: one look at the page answers them all.
                while hear.try_recv().is_ok() {}
                if this.update_in(cx, |this, window, cx| this.refresh(window, cx)).is_err() {
                    break;
                }
            }
        });
        BrowserTab {
            job,
            tab,
            page,
            address,
            url: String::new(),
            title: String::new(),
            back: false,
            forward: false,
            acting: None,
            waiting: Vec::new(),
            last,
            keys: [cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)],
            repository: NO_REPOSITORY.to_string(),
            store: Store::classic(),
            asks: AskSlot::default(),
            strip: [cx.focus_handle(), cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)],
            before: None,
            rules_seen: None,
            site: SiteState::new(cx),
            _subscriptions: vec![loaded, entered],
            _changes: changes,
            _asked: asked,
        }
    }

    pub fn page(&self) -> &Entity<WebPage> {
        &self.page
    }

    /// The tab that holds this page now: a page an agent opened takes the first tab there is, and an ask it had before there was a
    /// tab to key it by is raised now.
    pub fn attach(&mut self, tab: String, cx: &mut Context<Self>) {
        self.tab = tab;
        self.raise_to_fleet(cx);
    }

    pub(super) fn site(&self) -> &SiteState {
        &self.site
    }

    /// The address the page last told, or none while it has none.
    pub(super) fn address_now(&self) -> &str {
        &self.url
    }

    /// The choice a row shows: this site's, or the default for every site.
    pub(super) fn site_value(&self, permission: Permission) -> Choice {
        match (self.site.scope, host_of(&self.url)) {
            (Scope::Site, Some(host)) => self.site.settings.effective(&self.repository, &host, permission),
            _ => self.site.settings.default_of(permission),
        }
    }

    /// The lock pressed: Site settings opens, or closes.
    pub(super) fn toggle_site(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.site.open {
            self.close_site(window, cx);
            return;
        }
        self.site.open = true;
        self.site.scope = Scope::Site;
        // The keys are Bridge's while it is up, and the first group has them.
        self.page.update(cx, |page, _| page.release_keys());
        let first = self.site.scopes.clone();
        window.defer(cx, move |window, cx| window.focus(&first, cx));
        // What Classic wrote since the file was last read is shown.
        let store = self.store.clone();
        cx.spawn_in(window, async move |this, cx| {
            let settings = match store {
                Some(store) => cx.background_executor().spawn(async move { store.read() }).await,
                None => Settings::default(),
            };
            let _ = this.update(cx, |this, cx| {
                this.site.settings = settings;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Esc, a press outside or the lock again: the popover goes and the keys go back to the lock.
    pub(super) fn close_site(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.site.open {
            return;
        }
        self.site.open = false;
        window.focus(&self.site.lock, cx);
        cx.notify();
    }

    pub(super) fn set_scope(&mut self, scope: Scope, _window: &mut Window, cx: &mut Context<Self>) {
        if self.site.scope != scope {
            self.site.scope = scope;
            cx.notify();
        }
    }

    pub(super) fn step_scope(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let at = Scope::ALL.iter().position(|one| *one == self.site.scope).unwrap_or(0) as isize;
        let next = Scope::ALL[(at + by).rem_euclid(Scope::ALL.len() as isize) as usize];
        self.set_scope(next, window, cx);
    }

    /// An arrow in a row's group of segments.
    pub(super) fn step_choice(&mut self, permission: Permission, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let next = stepped(self.site_value(permission), by);
        self.press_choice(permission, next, window, cx);
    }

    /// A segment chosen in a row. Allow on Local network is asked about first, with the popover put away meanwhile.
    pub(super) fn press_choice(&mut self, permission: Permission, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let now = self.site_value(permission);
        if choice == now {
            return;
        }
        match press_of(permission, now, choice) {
            Press::Set => self.apply_choice(permission, choice, self.site.scope, window, cx),
            Press::Confirm => {
                self.site.confirm = Some(self.site.scope);
                self.site.open = false;
                let allow = self.site.confirm_allow.clone();
                window.defer(cx, move |window, cx| window.focus(&allow, cx));
                cx.notify();
            }
        }
    }

    pub(super) fn confirm_local_network(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(scope) = self.site.confirm.take() {
            self.apply_choice(Permission::LocalNetwork, Choice::Allow, scope, window, cx);
        }
        window.focus(&self.site.lock, cx);
        cx.notify();
    }

    pub(super) fn cancel_local_network(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.site.confirm = None;
        window.focus(&self.site.lock, cx);
        cx.notify();
    }

    /// A choice kept: shown at once, written to Classic's file off the main thread, and given to the page's rule list.
    fn apply_choice(&mut self, permission: Permission, choice: Choice, scope: Scope, window: &mut Window, cx: &mut Context<Self>) {
        let (store, repository) = (self.store.clone(), self.repository.clone());
        let host = host_of(&self.url);
        let settings = std::mem::take(&mut self.site.settings);
        self.site.settings = match (scope, &host) {
            (Scope::Site, Some(host)) => settings.with_choice(&repository, host, permission, choice),
            _ => settings.with_default(permission, choice),
        };
        cx.notify();
        let Some(store) = store else { return };
        cx.spawn_in(window, async move |this, cx| {
            let written = cx
                .background_executor()
                .spawn(async move {
                    match (scope, host) {
                        (Scope::Site, Some(host)) => store.choose(&repository, &host, permission, choice),
                        _ => store.set_default(permission, choice),
                    }
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Ok(written) = written {
                    this.site.settings = written;
                }
                this.refresh_rules(window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// The repository whose choices answer this page.
    pub fn set_repository(&mut self, key: String) {
        self.repository = key;
    }

    /// Whether Fleet knows this tab by an id of its own, as a page opened before any tab was there to hold it is not.
    fn held(&self) -> bool {
        self.tab != super::UNATTACHED
    }

    /// The page asks for a permission, to be answered once.
    fn requested(&mut self, request: Request, window: &mut Window, cx: &mut Context<Self>) {
        match request {
            Request::Media { origin, camera, microphone, reply } => {
                let mut wants = Vec::new();
                if camera {
                    wants.push(Permission::Camera);
                }
                if microphone {
                    wants.push(Permission::Microphone);
                }
                // A request that names neither asks for both (`permissionsOf`).
                if wants.is_empty() {
                    wants = vec![Permission::Camera, Permission::Microphone];
                }
                self.permit(wants, host_of(&origin), move |given| reply.send(given), window, cx);
            }
            Request::Site { origin, said, reply } => match said {
                Said::Location => {
                    // The position itself still needs macOS location access for the app: a Block here, and the Mac's own answer after
                    // an Allow, are two different gates, and the page gets the Mac's refusal as its own PERMISSION_DENIED or
                    // POSITION_UNAVAILABLE.
                    self.permit(vec![Permission::Location], host_of(&origin), move |given| reply.send(given), window, cx);
                }
                Said::LocalNetwork { target } => {
                    // Only a request to a more private space than the page's own is asked about. The reach has gone out already, or been
                    // refused by the rule list; an Allow here lifts the refusal for what the page does next.
                    let host = host_of(&origin);
                    let from = host.as_deref().map(|host| rules::space_of(rules::hostname(host)));
                    if let (Some(host), Some(from)) = (host, from) {
                        if rules::needs_check(from, rules::space_of(&target)) {
                            self.permit(vec![Permission::LocalNetwork], Some(host), move |given| reply.send(given), window, cx);
                        }
                    }
                }
            },
            Request::Made => self.refresh_rules(window, cx),
        }
    }

    /// The page's content rule list, built from the choices as the file holds them now, and given to the page where it differs from the one
    /// it has. The first call opens the page's first load.
    fn refresh_rules(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (store, repository) = (self.store.clone(), self.repository.clone());
        cx.spawn_in(window, async move |this, cx| {
            let settings = match store {
                Some(store) => cx.background_executor().spawn(async move { store.read() }).await,
                None => Settings::default(),
            };
            let json = rules_json(&settings, &repository);
            let _ = this.update(cx, |this, cx| {
                if this.rules_seen.as_ref() == Some(&json) {
                    return;
                }
                this.rules_seen = Some(json.clone());
                this.page.update(cx, |page, _| page.set_rules(json.as_deref()));
            });
        })
        .detach();
    }

    /// Decides what a page asked for from the owner's choices, asking him where a choice is Ask, and says the decision to `done`. A page
    /// whose host is no web address is refused.
    fn permit(&mut self, wants: Vec<Permission>, host: Option<String>, done: impl FnOnce(bool) + 'static, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = host else { return done(false) };
        let rules = TabRules { tab: cx.entity().downgrade(), cx: window.to_async(cx), store: self.store.clone(), repository: self.repository.clone(), host };
        cx.spawn_in(window, async move |_, _| {
            let given = decide(&wants, &rules).await;
            done(given);
        })
        .detach();
    }

    /// The page asks the owner: its strip draws, and Fleet is told. A second ask while one waits is refused as dismissed.
    fn ask(&mut self, permission: Permission, host: String, window: &mut Window, cx: &mut Context<Self>) -> async_channel::Receiver<Answer> {
        let (settle, answer) = async_channel::bounded(1);
        match self.asks.raise(permission, host, settle) {
            Err(settle) => {
                let _ = settle.try_send(Answer::Dismiss);
            }
            Ok(()) => {
                self.raise_to_fleet(cx);
                // The keys are Bridge's while the strip waits, and where the owner was is kept to go back to. A tab nobody is looking at
                // takes no focus: the ask is in Needs you for him.
                if self.shown(cx) {
                    self.page.update(cx, |page, _| page.release_keys());
                    self.before = window.focused(cx);
                    window.focus(&self.strip[1], cx);
                }
                cx.notify();
            }
        }
        answer
    }

    /// Fleet carries the ask the tab holds, in Needs you.
    fn raise_to_fleet(&mut self, cx: &mut Context<Self>) {
        if !self.held() {
            return;
        }
        if let Some(asking) = self.asks.asking() {
            cx.emit(BrowserTabEvent::Raised { tab: self.tab.clone(), text: asks::text(asking.permission, &asking.host), options: asks::options(asking.permission) });
        }
    }

    /// The ask is over, answered here or in Needs you. `fleet_knows`: Fleet said so itself, so it is not told again.
    pub fn answered(&mut self, answer: Answer, fleet_knows: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ended) = self.asks.end(fleet_knows, self.held()) else { return };
        if ended.tell_fleet {
            cx.emit(BrowserTabEvent::Ended { tab: self.tab.clone() });
        }
        let _ = ended.asking.settle.try_send(answer);
        if let Some(back) = self.before.take() {
            if self.strip.iter().any(|one| one.contains_focused(window, cx)) {
                window.focus(&back, cx);
            }
        }
        cx.notify();
    }

    /// The tab is closing, or its Job is going: its ask is dismissed. Whether Fleet is still to be told it is over.
    pub fn close(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(ended) = self.asks.end(false, self.held()) else { return false };
        let _ = ended.asking.settle.try_send(Answer::Dismiss);
        cx.notify();
        ended.tell_fleet
    }

    /// Whether a pane draws the page now.
    pub fn shown(&self, cx: &gpui::App) -> bool {
        self.page.read(cx).is_painted()
    }

    /// The agent's mark: lit with the act's sentence while an act runs on this page, dim again when none does.
    pub fn set_acting(&mut self, sentence: Option<SharedString>, cx: &mut Context<Self>) {
        if self.acting != sentence {
            self.acting = sentence;
            cx.notify();
        }
    }

    /// The page made now where nothing has drawn it yet, so an act reaches it.
    pub fn ensure_made(&mut self, cx: &mut Context<Self>) {
        self.page.update(cx, |page, _| page.made_at(UNSEEN.0, UNSEEN.1));
    }

    /// Loads `url`, and answers on the returned channel once the load ended: `Ok`, or WebKit's words for why it failed. A load
    /// that was replaced by the next one is not an end.
    pub fn begin_load(&mut self, url: &str, cx: &mut Context<Self>) -> async_channel::Receiver<Result<(), String>> {
        let (tx, rx) = async_channel::bounded(1);
        self.waiting.push((tx, url.to_string()));
        self.page.update(cx, |page, cx| {
            page.made_at(UNSEEN.0, UNSEEN.1);
            page.load_url(url, cx);
        });
        rx
    }

    fn page_told(&mut self, event: &WebPageEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            WebPageEvent::Loaded => {
                self.refresh_rules(window, cx);
                self.refresh(window, cx);
                self.loads_ended(cx);
            }
            WebPageEvent::Shown => {
                if self.page.read(cx).is_painted() {
                    self.last.borrow_mut().insert(self.job.to_string(), self.tab.clone());
                }
                cx.notify();
            }
            _ => {}
        }
    }

    /// The loads an act waits on end with the page's status, apart from the empty page that was loading before the address the
    /// act asked for replaced it.
    fn loads_ended(&mut self, cx: &mut Context<Self>) {
        let page = self.page.read(cx);
        let ended = match page.status() {
            Status::Ready => Ok(()),
            Status::Failed(why) => Err(why.clone()),
            _ => return,
        };
        let blank = page.url().is_none_or(|url| url == "about:blank");
        for (tell, asked) in std::mem::take(&mut self.waiting) {
            if ended.is_ok() && blank && asked != "about:blank" {
                self.waiting.push((tell, asked));
            } else {
                let _ = tell.try_send(ended.clone());
            }
        }
    }

    /// The head as the page last told it: its address, title and whether it can go back or forward.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (url, title, (back, forward)) = {
            let page = self.page.read(cx);
            (page.url().unwrap_or_default(), page.title(), page.history())
        };
        // A Job browser made empty shows no address until a page is opened in it.
        let url = if url == "about:blank" { String::new() } else { url };
        if url != self.url {
            self.url = url.clone();
            self.address.update(cx, |field, cx| field.set_value(url, window, cx));
        }
        self.title = title;
        (self.back, self.forward) = (back, forward);
        cx.notify();
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.back {
            self.page.update(cx, |page, cx| page.go_back(cx));
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if self.forward {
            self.page.update(cx, |page, cx| page.go_forward(cx));
        }
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| page.reload(cx));
    }

    /// Enter in the address: goes to what was typed, and only the web (`navigate`).
    fn navigate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.address.read(cx).value().trim().to_string();
        if typed.is_empty() || typed == self.url || !is_web(&typed) {
            return;
        }
        self.page.update(cx, |page, cx| {
            page.made_at(UNSEEN.0, UNSEEN.1);
            page.load_url(&typed, cx);
        });
    }

    /// The agent's mark: a registry glyph, dim at rest and lit with a gentle pulse only while an act runs, the act's sentence its
    /// tooltip. It takes no press, and holds the size of an icon button so lighting it moves nothing.
    fn mark(&self, look: &Look) -> AnyElement {
        let said = self.acting.clone().unwrap_or_else(|| NO_ACT.into());
        let side = look.length(token::H_CONTROL_SM);
        let glyph = svg().path(icon_path("bot")).size(look.length(token::SPACE_4));
        let body = div().id("job-browser-mark").flex_none().flex().items_center().justify_center().size(side).tooltip({
            let said = said.clone();
            move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
        });
        let body = if self.acting.is_some() {
            let pulse = look.duration(token::DURATION_PULSE).mul_f32(1.5);
            body.child(
                glyph
                    .text_color(look.color(token::ACCENT))
                    .with_animation("job-browser-mark-pulse", Animation::new(pulse).repeat().with_easing(gpui::pulsating_between(0.35, 1.)), |glyph, at| glyph.opacity(at)),
            )
        } else {
            body.child(glyph.text_color(look.color(token::FG_SUBTLE)))
        };
        labelled(body, Role::Image, said).into_any_element()
    }

    /// One of the head's icon buttons: reached by Tab, pressed by Enter or Space, and drawn muted and out of the tab order while
    /// there is nothing for it to do.
    #[allow(clippy::too_many_arguments)]
    fn control(
        &self,
        id: &'static str,
        handle: &FocusHandle,
        icon: &'static str,
        name: &'static str,
        key: &'static str,
        on: bool,
        look: &Look,
        press: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &Context<Self>,
    ) -> AnyElement {
        let said: SharedString = format!("{name}  {key}").into();
        let hover = look.color(token::BG_HOVER);
        let ring = look.color(token::ACCENT);
        let body = div()
            .id(id)
            .size(look.length(token::H_CONTROL_SM))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .child(svg().path(icon_path(icon)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx));
        let body = if on {
            body.track_focus(handle)
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)))
        } else {
            body.opacity(0.5)
        };
        labelled(body, Role::Button, name).into_any_element()
    }
}

/// The owner's choices and the tab's ask, as deciding a page's request reads them.
struct TabRules {
    tab: WeakEntity<BrowserTab>,
    cx: AsyncWindowContext,
    store: Option<Store>,
    repository: String,
    host: String,
}

impl Rules for TabRules {
    async fn choice(&self, permission: Permission) -> Choice {
        // The file is read afresh each time, off the main thread, since Classic may have written it.
        let settings = match self.store.clone() {
            Some(store) => self.cx.background_executor().spawn(async move { store.read() }).await,
            None => Settings::default(),
        };
        settings.effective(&self.repository, &self.host, permission)
    }

    async fn ask(&self, permission: Permission) -> Answer {
        let mut cx = self.cx.clone();
        let host = self.host.clone();
        let Ok(answer) = self.tab.update_in(&mut cx, |tab, window, cx| tab.ask(permission, host, window, cx)) else { return Answer::Dismiss };
        answer.recv().await.unwrap_or(Answer::Dismiss)
    }

    async fn remember(&self, permission: Permission, choice: Choice) {
        let Some(store) = self.store.clone() else { return };
        let (repository, host) = (self.repository.clone(), self.host.clone());
        let _ = self.cx.background_executor().spawn(async move { store.choose(&repository, &host, permission, choice) }).await;
        // What was allowed lifts the refusal for what the page does next.
        let mut cx = self.cx.clone();
        let _ = self.tab.update_in(&mut cx, |tab, window, cx| tab.refresh_rules(window, cx));
    }
}

impl BrowserTab {
    /// The page's ask, over the page under the head and never over it: the page's own view moves down and stays live while it waits.
    /// Block and Allow, and Esc leaves the site undecided (`AskStrip`).
    fn strip(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let asking = self.asks.asking()?;
        let [frame, block, allow] = self.strip.clone();
        let words = asks::strip_words(asking.permission, &asking.host);
        let risk = asks::strip_risk(asking.permission);
        let ring = look.color(token::ACCENT);
        let choice = |id: &'static str, handle: &FocusHandle, said: &'static str, primary: bool, answer: Answer, cx: &Context<Self>| {
            let body = div()
                .id(id)
                .track_focus(handle)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if primary { look.color(token::ACCENT) } else { gpui::transparent_black() })
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .cursor_pointer()
                .focus_visible(move |style| style.border_color(ring))
                .on_click(cx.listener(move |this, _, window, cx| this.answered(answer, false, window, cx)))
                .child(said);
            let body = if primary {
                let hover = look.color(token::ACCENT_HOVER);
                body.bg(look.color(token::ACCENT)).text_color(look.color(token::TEXT_ON_ACCENT)).hover(move |style| style.bg(hover))
            } else {
                let hover = look.color(token::BG_HOVER);
                body.text_color(look.color(token::FG_MUTED)).hover(move |style| style.bg(hover).text_color(look.color(token::FG_DEFAULT)))
            };
            labelled(body, Role::Button, said)
        };
        let words = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(div().child(SharedString::from(words)))
            .children(risk.map(|risk| div().text_color(look.color(token::FG_MUTED)).child(risk)));
        let acts = div()
            .flex()
            .flex_none()
            .ml_auto()
            .gap(look.length(token::SPACE_1))
            .child(choice("job-browser-ask-block", &block, "Block", false, Answer::Block, cx))
            .child(choice("job-browser-ask-allow", &allow, "Allow", true, Answer::Allow, cx));
        Some(
            labelled(
                div()
                    .id("job-browser-ask")
                    .key_context(ASK_CONTEXT)
                    .track_focus(&frame)
                    .on_action(cx.listener(|this, _: &DismissAsk, window, cx| this.answered(Answer::Dismiss, false, window, cx)))
                    .flex()
                    .flex_none()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_w_0()
                    .py(look.length(token::SPACE_1))
                    .pr(look.length(token::SPACE_1))
                    .pl(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_OVERLAY))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(words)
                    .child(acts),
                Role::AlertDialog,
                "Site request",
            )
            .into_any_element(),
        )
    }
}

impl Render for BrowserTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let strip = self.strip(&look, cx);
        let lock = self.site_lock(&look, cx);
        let layers = self.site_layers(&look, window, cx);
        let [back_key, forward_key, reload_key] = self.keys.clone();
        let acts = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .child(self.control("job-browser-back", &back_key, "chevron-left", "Back", BACK_KEY, self.back, &look, |this, _, cx| this.go_back(cx), cx))
            .child(self.control("job-browser-forward", &forward_key, "chevron-right", "Forward", FORWARD_KEY, self.forward, &look, |this, _, cx| this.go_forward(cx), cx))
            .child(self.control("job-browser-reload", &reload_key, "refresh-cw", "Reload", RELOAD_KEY, true, &look, |this, _, cx| this.reload(cx), cx));
        let address = labelled(
            div()
                .id("job-browser-address")
                .flex_1()
                .min_w_0()
                .key_context(crate::keymap::TEXT_BOX)
                .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                .text_size(look.length(token::TEXT_SM))
                .child(Input::new(&self.address)),
            Role::TextInput,
            "Address",
        );
        let head = labelled(
            div()
                .id("job-browser-head")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL_SM))
                .py(look.length(token::SPACE_1))
                .pr(look.length(token::SPACE_1))
                .pl(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(self.mark(&look))
                .child(acts)
                .children(lock)
                .child(address),
            Role::Toolbar,
            "Job browser",
        );
        // The page's own element is a plain flex body: it takes what the head leaves and reports where it is drawn.
        let page = labelled(
            div().id("job-browser-page").flex_1().min_h_0().flex().flex_col().bg(look.color(token::BG_BASE)).child(web_pane(&self.page).size_full()),
            Role::Region,
            "Page",
        );
        labelled(
            div().id("job-browser").key_context("job-browser").size_full().flex().flex_col().min_w_0().bg(look.color(token::BG_SUNKEN)).child(head).children(strip).child(page).children(layers),
            Role::Group,
            "Job browser",
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::labels::LabelBook;

    /// The window's root, which clears the book of what was drawn before it, as the app's own root does.
    struct Rooted(Entity<BrowserTab>);

    impl Render for Rooted {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    /// A file of choices of the test's own, so no test reads or writes the owner's.
    fn scratch_store() -> Store {
        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let folder = std::env::temp_dir().join(format!("armada-tab-test-{}-{}", std::process::id(), COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::create_dir_all(&folder).unwrap();
        Store::at(folder.join("site-permissions.json"))
    }

    fn drawn(cx: &mut gpui::TestAppContext) -> (Entity<BrowserTab>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let last: LastShown = Rc::default();
        let (root, cx) = cx.add_window_view(|window, cx| Rooted(cx.new(|cx| BrowserTab::new("mock-01".into(), "t1".into(), last, window, cx))));
        let tab = root.read_with(cx, |root, _| root.0.clone());
        tab.update(cx, |tab, _| tab.store = Some(scratch_store()));
        (tab, cx)
    }

    fn names(cx: &mut gpui::VisualTestContext, role: Role) -> Vec<String> {
        cx.update(|window, cx| {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            drawn.iter().filter(|one| one.role == role).map(|one| one.name.to_string()).collect()
        })
    }

    #[gpui::test]
    fn the_head_has_the_mark_back_forward_reload_and_the_address_named_as_a_person_reads_them(cx: &mut gpui::TestAppContext) {
        let (_, cx) = drawn(cx);
        let buttons = names(cx, Role::Button);
        assert_eq!(buttons, ["Back", "Forward", "Reload"], "Back and Forward are named while muted, so VoiceOver finds them");
        assert_eq!(names(cx, Role::TextInput), ["Address"]);
        assert_eq!(names(cx, Role::Toolbar), ["Job browser"]);
        assert_eq!(names(cx, Role::Group), ["Job browser"]);
        assert_eq!(names(cx, Role::Image), [NO_ACT], "the mark reads that nothing is running");
    }

    #[gpui::test]
    fn the_mark_reads_the_act_that_is_running_and_goes_dim_again(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        tab.update(cx, |tab, cx| tab.set_acting(Some("Clicking Save".into()), cx));
        cx.run_until_parked();
        assert_eq!(names(cx, Role::Image), ["Clicking Save"]);
        tab.update(cx, |tab, cx| tab.set_acting(None, cx));
        cx.run_until_parked();
        assert_eq!(names(cx, Role::Image), [NO_ACT]);
    }

    /// Each event the tab told, as words.
    fn told(tab: &Entity<BrowserTab>, cx: &mut gpui::VisualTestContext) -> Rc<RefCell<Vec<String>>> {
        let seen: Rc<RefCell<Vec<String>>> = Rc::default();
        let into = seen.clone();
        cx.update(|_, app| {
            app.subscribe(tab, move |_, event: &BrowserTabEvent, _| {
                into.borrow_mut().push(match event {
                    BrowserTabEvent::Raised { tab, text, options } => format!("raised {tab}: {text} [{}]", options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>().join(", ")),
                    BrowserTabEvent::Ended { tab } => format!("ended {tab}"),
                })
            })
            .detach();
        });
        seen
    }

    fn ask(tab: &Entity<BrowserTab>, permission: Permission, cx: &mut gpui::VisualTestContext) -> async_channel::Receiver<Answer> {
        tab.update_in(cx, |tab, window, cx| tab.ask(permission, "meet.example.com".into(), window, cx))
    }

    #[gpui::test]
    fn a_page_ask_draws_a_strip_with_block_and_allow_and_goes_to_needs_you(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let seen = told(&tab, cx);
        assert!(names(cx, Role::AlertDialog).is_empty(), "no strip while nothing is asked");
        let _answer = ask(&tab, Permission::Camera, cx);
        cx.run_until_parked();
        assert_eq!(names(cx, Role::AlertDialog), ["Site request"]);
        assert_eq!(names(cx, Role::Button), ["Back", "Forward", "Reload", "Block", "Allow"], "Block, then Allow");
        assert_eq!(*seen.borrow(), ["raised t1: Allow Camera for meet.example.com? [Allow, Block]"]);
    }

    #[gpui::test]
    fn answering_in_the_tab_settles_the_page_and_tells_fleet_the_ask_is_over(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let seen = told(&tab, cx);
        let answer = ask(&tab, Permission::Microphone, cx);
        tab.update_in(cx, |tab, window, cx| tab.answered(Answer::Allow, false, window, cx));
        cx.run_until_parked();
        assert_eq!(answer.try_recv().ok(), Some(Answer::Allow));
        assert!(names(cx, Role::AlertDialog).is_empty(), "the strip goes");
        assert_eq!(seen.borrow().last().map(String::as_str), Some("ended t1"));
    }

    #[gpui::test]
    fn an_answer_from_fleet_settles_the_strip_and_does_not_end_the_ask_it_answered(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let seen = told(&tab, cx);
        let answer = ask(&tab, Permission::Camera, cx);
        tab.update_in(cx, |tab, window, cx| tab.answered(Answer::Block, true, window, cx));
        cx.run_until_parked();
        assert_eq!(answer.try_recv().ok(), Some(Answer::Block));
        assert!(names(cx, Role::AlertDialog).is_empty());
        assert_eq!(seen.borrow().len(), 1, "only the raise: Fleet took its own item off");
    }

    #[gpui::test]
    fn a_second_ask_while_one_waits_is_dismissed_and_the_first_stays(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let seen = told(&tab, cx);
        let first = ask(&tab, Permission::Camera, cx);
        let second = ask(&tab, Permission::Microphone, cx);
        assert_eq!(second.try_recv().ok(), Some(Answer::Dismiss));
        assert!(first.try_recv().is_err(), "the first still waits");
        cx.run_until_parked();
        assert_eq!(seen.borrow().len(), 1, "Fleet holds one item a tab");
    }

    #[gpui::test]
    fn closing_the_tab_dismisses_its_ask_and_says_fleet_is_to_be_told(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let answer = ask(&tab, Permission::Camera, cx);
        assert!(tab.update(cx, |tab, cx| tab.close(cx)));
        assert_eq!(answer.try_recv().ok(), Some(Answer::Dismiss));
        assert!(!tab.update(cx, |tab, cx| tab.close(cx)), "nothing left to end");
    }

    #[gpui::test]
    fn esc_dismisses_the_ask_and_a_dismissal_is_a_dismissal_not_a_block(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        cx.update(|_, app| bind(app));
        let seen = told(&tab, cx);
        let answer = ask(&tab, Permission::Camera, cx);
        cx.run_until_parked();
        // A tab nobody is looking at takes no focus, so the owner's keys are put on the strip as a press of Tab would.
        tab.update_in(cx, |tab, window, cx| {
            let allow = tab.strip[2].clone();
            window.focus(&allow, cx);
        });
        cx.simulate_keystrokes("escape");
        assert_eq!(answer.try_recv().ok(), Some(Answer::Dismiss));
        assert_eq!(seen.borrow().last().map(String::as_str), Some("ended t1"));
    }

    #[gpui::test]
    fn a_page_opened_before_any_tab_raises_nothing_until_a_tab_holds_it(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let last: LastShown = Rc::default();
        let (root, cx) = cx.add_window_view(|window, cx| Rooted(cx.new(|cx| BrowserTab::new("mock-01".into(), super::super::UNATTACHED.into(), last, window, cx))));
        let tab = root.read_with(cx, |root, _| root.0.clone());
        tab.update(cx, |tab, _| tab.store = Some(scratch_store()));
        let seen = told(&tab, cx);
        let _answer = ask(&tab, Permission::Camera, cx);
        assert!(seen.borrow().is_empty(), "Fleet knows no tab by that id");
        tab.update(cx, |tab, cx| tab.attach("t9".into(), cx));
        assert_eq!(*seen.borrow(), ["raised t9: Allow Camera for meet.example.com? [Allow, Block]"]);
    }

    /// A tab whose page has told a web address, so its lock is drawn.
    fn on_a_site(cx: &mut gpui::TestAppContext) -> (Entity<BrowserTab>, &mut gpui::VisualTestContext) {
        let (tab, cx) = drawn(cx);
        cx.update(|_, app| bind(app));
        tab.update(cx, |tab, cx| {
            tab.url = "https://example.com/a".into();
            cx.notify();
        });
        cx.run_until_parked();
        (tab, cx)
    }

    fn value(tab: &Entity<BrowserTab>, permission: Permission, cx: &mut gpui::VisualTestContext) -> Choice {
        tab.read_with(cx, |tab, _| tab.site_value(permission))
    }

    #[gpui::test]
    fn a_page_with_no_web_address_has_no_lock_and_one_with_a_host_has_it_named_site_settings(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        assert!(!names(cx, Role::Button).contains(&"Site settings".to_string()));
        tab.update(cx, |tab, cx| {
            tab.url = "https://example.com/a".into();
            cx.notify();
        });
        cx.run_until_parked();
        assert_eq!(names(cx, Role::Button), ["Back", "Forward", "Reload", "Site settings"]);
        assert!(names(cx, Role::Dialog).is_empty(), "the popover is closed until the lock is pressed");
    }

    #[gpui::test]
    fn the_lock_opens_a_popover_with_the_site_and_nine_rows_of_ask_allow_block(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = on_a_site(cx);
        tab.update_in(cx, |tab, window, cx| tab.toggle_site(window, cx));
        cx.run_until_parked();
        assert_eq!(names(cx, Role::Dialog), ["Site settings"]);
        let groups = names(cx, Role::RadioGroup);
        assert_eq!(groups.len(), 1 + Permission::ALL.len(), "the scope, then a row for each permission");
        assert_eq!(groups[0], "Settings for");
        assert_eq!(&groups[1..], Permission::ALL.map(Permission::label));
        assert_eq!(names(cx, Role::RadioButton).len(), 2 + 3 * Permission::ALL.len());
        assert_eq!(value(&tab, Permission::Camera, cx), Choice::Ask);
        assert_eq!(value(&tab, Permission::Popups, cx), Choice::Allow, "the shipped default shows");
    }

    #[gpui::test]
    fn arrows_step_a_row_esc_closes_and_the_lock_has_the_focus_again(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = on_a_site(cx);
        tab.update_in(cx, |tab, window, cx| tab.toggle_site(window, cx));
        cx.run_until_parked();
        tab.update_in(cx, |tab, window, cx| {
            let row = tab.site.rows[Permission::ALL.iter().position(|one| *one == Permission::Camera).unwrap()].clone();
            window.focus(&row, cx);
        });
        cx.simulate_keystrokes("right");
        assert_eq!(value(&tab, Permission::Camera, cx), Choice::Allow, "Ask, then Allow");
        cx.simulate_keystrokes("right");
        assert_eq!(value(&tab, Permission::Camera, cx), Choice::Block);
        cx.simulate_keystrokes("left");
        assert_eq!(value(&tab, Permission::Camera, cx), Choice::Allow);
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(names(cx, Role::Dialog).is_empty());
        let lock = tab.read_with(cx, |tab, _| tab.site.lock.clone());
        assert!(cx.update(|window, _| lock.is_focused(window)), "Esc gives the keys back to the lock");
    }

    #[gpui::test]
    fn all_sites_sets_the_default_and_this_site_sets_the_host(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = on_a_site(cx);
        tab.update_in(cx, |tab, window, cx| {
            tab.toggle_site(window, cx);
            tab.press_choice(Permission::Notifications, Choice::Block, window, cx);
        });
        assert_eq!(value(&tab, Permission::Notifications, cx), Choice::Block);
        tab.update_in(cx, |tab, window, cx| tab.set_scope(Scope::All, window, cx));
        assert_eq!(value(&tab, Permission::Notifications, cx), Choice::Ask, "the default is its own");
        tab.update_in(cx, |tab, window, cx| tab.press_choice(Permission::Clipboard, Choice::Allow, window, cx));
        assert_eq!(value(&tab, Permission::Clipboard, cx), Choice::Allow);
        tab.update_in(cx, |tab, window, cx| tab.set_scope(Scope::Site, window, cx));
        assert_eq!(value(&tab, Permission::Clipboard, cx), Choice::Allow, "a site without its own takes the default");
    }

    #[gpui::test]
    fn allow_on_local_network_asks_first_and_cancel_keeps_the_choice(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = on_a_site(cx);
        tab.update_in(cx, |tab, window, cx| {
            tab.toggle_site(window, cx);
            tab.press_choice(Permission::LocalNetwork, Choice::Allow, window, cx);
        });
        cx.run_until_parked();
        assert_eq!(value(&tab, Permission::LocalNetwork, cx), Choice::Ask, "nothing is set before the confirm");
        assert_eq!(names(cx, Role::Dialog), ["Allow local network for example.com?"]);
        assert_eq!(names(cx, Role::Button).iter().filter(|one| *one == "Cancel" || *one == "Allow").count(), 2);
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert!(names(cx, Role::Dialog).is_empty());
        assert_eq!(value(&tab, Permission::LocalNetwork, cx), Choice::Ask);
        tab.update_in(cx, |tab, window, cx| {
            tab.toggle_site(window, cx);
            tab.press_choice(Permission::LocalNetwork, Choice::Allow, window, cx);
            tab.confirm_local_network(window, cx);
        });
        assert_eq!(value(&tab, Permission::LocalNetwork, cx), Choice::Allow);
        tab.update_in(cx, |tab, window, cx| tab.press_choice(Permission::LocalNetwork, Choice::Block, window, cx));
        assert_eq!(value(&tab, Permission::LocalNetwork, cx), Choice::Block, "Block and Ask take no confirm");
    }

    #[gpui::test]
    fn an_empty_page_shows_no_address_and_a_page_shows_its_own(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let said = |tab: &Entity<BrowserTab>, cx: &mut gpui::VisualTestContext| tab.read_with(cx, |tab, cx| tab.address.read(cx).value().to_string());
        assert_eq!(said(&tab, cx), "");
        tab.update_in(cx, |tab, window, cx| {
            tab.address.update(cx, |field, cx| field.set_value("https://example.com/a", window, cx));
        });
        assert_eq!(said(&tab, cx), "https://example.com/a");
    }
}
