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
    div, prelude::*, svg, Animation, AnimationExt, AnyElement, Context, Entity, FocusHandle, Role, SharedString, Subscription, Task,
    Window,
};
use gpui_component::input::{Input, InputEvent, InputState};

use super::acts::is_web;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::web::{web_pane, Status, WebPage, WebPageEvent};

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
    _subscriptions: Vec<Subscription>,
    _changes: Task<()>,
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
            _subscriptions: vec![loaded, entered],
            _changes: changes,
        }
    }

    pub fn page(&self) -> &Entity<WebPage> {
        &self.page
    }

    /// The tab that holds this page now: a page an agent opened takes the first tab there is.
    pub fn attach(&mut self, tab: String) {
        self.tab = tab;
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

impl Render for BrowserTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
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
            div().id("job-browser").key_context("job-browser").size_full().flex().flex_col().min_w_0().bg(look.color(token::BG_SUNKEN)).child(head).child(page),
            Role::Group,
            "Job browser",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::LabelBook;

    /// The window's root, which clears the book of what was drawn before it, as the app's own root does.
    struct Rooted(Entity<BrowserTab>);

    impl Render for Rooted {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn drawn(cx: &mut gpui::TestAppContext) -> (Entity<BrowserTab>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let last: LastShown = Rc::default();
        let (root, cx) = cx.add_window_view(|window, cx| Rooted(cx.new(|cx| BrowserTab::new("mock-01".into(), "t1".into(), last, window, cx))));
        let tab = root.read_with(cx, |root, _| root.0.clone());
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
