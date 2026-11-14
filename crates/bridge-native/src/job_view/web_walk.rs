//! A Job's walk of a web page, drawn as a tab beside its thread: the walk's bar over the page, as Armada
//! Classic's live walk tab draws it (`WalkPane.tsx`'s `LiveWalk`, `job-walks.ts`, the bar from `views.ts`).
//!
//! **The page is a WKWebView placed over the empty frame this tab reports** (`crate::web`). Its steps are the
//! ones the Job passed with `show_window` (`ipc::WalkStep`): each is found in the page by role and name, and
//! ringed, as `finder.ts` finds them. A press is a click the page hears. The pages stay alive while the
//! tab is hidden, and keep their scroll and state.
//!
//! A page a Job serves and has not shown as a walk (its dev server) takes the same tab with its name and Reload alone.
//! A Job held on another Mac is drawn from the address Fleet answered on this one (`web_pages.rs`).
//!
//! Left out, as Classic has more here: popups, site permissions, the burn and annotation layer, held notes,
//! local-network prompts, takes, and acts other than a press (type, key, select, paste, drag), which ring their
//! target and go on.

use std::time::Duration;

use gpui::{
    actions, div, prelude::*, px, Context, Entity, EventEmitter, FocusHandle, FontWeight, KeyBinding, Role, SharedString,
    Subscription, Task, Window,
};
use ipc::{WalkStep, WalkTarget};

use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::walk::bar::{button, icon_button, Weight};
use crate::web::{finder, web_pane, Mode, Source, Status, WebPage, WebPageEvent};

actions!(web_walk, [Next, Back, Reload, Approve]);

/// The bar's keys, as Classic's walk bar binds them (`QUEUE_KEYS`): ⌥⌘→ and ⌥⌘← move the step, ⌘R loads the page again and ⌘↵ approves.
pub fn bind(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("alt-cmd-right", Next, Some("web-walk")),
        KeyBinding::new("alt-cmd-left", Back, Some("web-walk")),
        KeyBinding::new("cmd-r", Reload, Some("web-walk")),
        KeyBinding::new("cmd-enter", Approve, Some("web-walk")),
    ]);
}

/// How long a step waits for its target, as Classic's `PATIENCE_MS`.
const PATIENCE: Duration = Duration::from_secs(5);
/// How often the page is asked again while its target is not drawn.
const RETRY: Duration = Duration::from_millis(150);

/// What the tab tells the Job view.
pub enum WebWalkEvent {
    /// The owner pressed Approve (⌘↵): the Job is told, as the bar sends it.
    Approve,
}

/// The target a step aims at (`targetOf`).
pub fn target_of(step: &WalkStep) -> Option<&WalkTarget> {
    step.press
        .as_ref()
        .or(step.look.as_ref())
        .or(step.hover.as_ref())
        .or(step.later.as_ref())
        .or(step.into.as_ref())
        .or(step.on.as_ref())
        .or(step.within.as_ref())
        .or(step.paste.as_ref())
        .or(step.drag.as_ref())
}

/// What the step does to its target, said plainly (`verb`).
fn verb_of(step: &WalkStep) -> &'static str {
    if step.hover.is_some() {
        "hover over"
    } else if step.key.is_some() {
        "press a key on"
    } else if step.paste.is_some() {
        "paste into"
    } else if step.select.is_some() {
        "select words in"
    } else if step.press.is_some() {
        "press"
    } else if step.drag.is_some() {
        "drag"
    } else if step.typed.is_some() {
        "type into"
    } else {
        "look at"
    }
}

/// A target as a person would say it (`saidOf`).
fn said_of(target: &WalkTarget) -> String {
    let own = if target.role == "text" {
        format!("the words \u{201C}{}\u{201D}", target.name.as_deref().unwrap_or(""))
    } else {
        match &target.name {
            Some(name) => format!("a {} named \u{201C}{name}\u{201D}", target.role),
            None => format!("a {}", target.role),
        }
    };
    match &target.within {
        Some(outer) => format!("{own} in {}", said_of(outer)),
        None => own,
    }
}

/// Why a walk holds on a step, as the bar says it (`heldAt`).
fn held_at(at: usize, steps: &[WalkStep]) -> String {
    let Some(step) = steps.get(at) else { return String::new() };
    let target = target_of(step).map(said_of).unwrap_or_default();
    format!("Step {} of {}: nothing to {}, no {} came within {} seconds", at + 1, steps.len(), verb_of(step), target.trim_start_matches("a "), PATIENCE.as_secs())
}

/// The page an address is: a local file or a web address.
pub fn source_of(url: &str) -> Source {
    match url.strip_prefix("file://") {
        Some(path) => Source::File(path.split(['?', '#']).next().unwrap_or(path).to_string()),
        None => Source::Url(url.to_string()),
    }
}

/// What a tab shows: the Job's page as it named it (`url`, which the walk is approved by), and the address it is drawn
/// from (`here`), which is `url` itself unless the Job is held on another Mac and Fleet answered an address on this one.
pub struct Shown {
    pub job: SharedString,
    pub url: String,
    pub here: String,
    pub title: String,
    pub steps: Vec<WalkStep>,
    /// The Job's dev server or the page it showed last, not a walk: the bar is its name and Reload alone.
    pub live: bool,
    pub manifest: Option<String>,
}

pub struct WebWalkTab {
    job: SharedString,
    url: String,
    title: SharedString,
    steps: Vec<WalkStep>,
    /// A page the Job serves, not a walk of it: no steps and no Approve.
    live: bool,
    /// The step on screen, from 0.
    at: usize,
    page: Entity<WebPage>,
    /// What the bar says beside Reload: why a step is held, or that the Job was told.
    said: Option<SharedString>,
    /// The step being found and ringed, so a faster press does not leave two walking.
    finding: bool,
    /// The step the page was last asked for: a loaded, painted page plays its first step once.
    started: bool,
    /// Back reloads the page and plays the steps before the one it goes to, so what an earlier step pressed is pressed again.
    replay: Option<usize>,
    /// The Job view's row-tab handle, which a press on the tab, Tab or a pane's move already puts the keys on: the walk's own keys
    /// (⌥⌘→, ⌥⌘←, ⌘R, ⌘↵) need it drawn here, or the keys land on a handle nothing draws.
    pub focus: FocusHandle,
    _page: Subscription,
    _find: Option<Task<()>>,
}

impl EventEmitter<WebWalkEvent> for WebWalkTab {}

impl WebWalkTab {
    pub fn new(shown: Shown, focus: FocusHandle, cx: &mut Context<Self>) -> Self {
        let Shown { job, url, here, title, steps, live, manifest } = shown;
        let source = source_of(&here);
        let page = cx.new(|cx| WebPage::new(manifest.as_deref(), source, cx));
        let watch = cx.subscribe(&page, |this, _, _: &WebPageEvent, cx| {
            this.start(cx);
            cx.notify();
        });
        WebWalkTab { job, url, title: title.into(), steps, live, at: 0, page, said: None, finding: false, started: false, replay: None, focus, _page: watch, _find: None }
    }

    pub fn job(&self) -> &SharedString {
        &self.job
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// The first step plays once the page has loaded and has a place on screen.
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.started {
            return;
        }
        let page = self.page.read(cx);
        if *page.status() != Status::Ready || page.mode() == Mode::Parked {
            return;
        }
        self.started = true;
        if self.steps.is_empty() {
            self.page.update(cx, |page, cx| page.take_picture(cx));
        } else if let Some(to) = self.replay.take().filter(|to| *to > 0) {
            self.play_to(to, cx);
        } else {
            self.go(0, cx);
        }
    }

    /// The steps before `to` that press something are pressed again on a fresh page, each once its target is there, and
    /// step `to` is then shown (`Replay to the step before`).
    fn play_to(&mut self, to: usize, cx: &mut Context<Self>) {
        let targets: Vec<_> = self.steps[..to].iter().filter(|step| step.press.is_some()).filter_map(|step| target_of(step).cloned()).collect();
        self.finding = true;
        let page = self.page.clone();
        self._find = Some(cx.spawn(async move |this, cx| {
            for target in targets {
                let mut waited = Duration::ZERO;
                loop {
                    let (ask, viewport) = page.read_with(cx, |page, _| (page.ask(&finder::look(&target)), page.viewport()));
                    let answer = ask.recv().await.ok().flatten();
                    if finder::place_of(answer.as_deref(), viewport).is_some() {
                        let press = page.read_with(cx, |page, _| page.ask(&finder::press()));
                        press.recv().await.ok();
                        break;
                    }
                    if waited >= PATIENCE {
                        break;
                    }
                    cx.background_executor().timer(RETRY).await;
                    waited += RETRY;
                }
            }
            this.update(cx, |this, cx| {
                this.finding = false;
                this.go(to, cx);
            })
            .ok();
        }));
        cx.notify();
    }

    /// Step `to` on screen: its target is found, scrolled to the middle of the page and ringed.
    fn go(&mut self, to: usize, cx: &mut Context<Self>) {
        let Some(step) = self.steps.get(to) else { return };
        self.at = to;
        self.said = None;
        let target = target_of(step).cloned();
        self.page.update(cx, |page, cx| page.set_ring(None, cx));
        let Some(target) = target else {
            cx.notify();
            return;
        };
        self.finding = true;
        let page = self.page.clone();
        self._find = Some(cx.spawn(async move |this, cx| {
            let mut waited = Duration::ZERO;
            loop {
                let (ask, viewport) = page.read_with(cx, |page, _| (page.ask(&finder::look(&target)), page.viewport()));
                let answer = ask.recv().await.ok().flatten();
                if let Some(place) = finder::place_of(answer.as_deref(), viewport) {
                    this.update(cx, |this, cx| {
                        this.finding = false;
                        this.page.update(cx, |page, cx| {
                            page.set_ring(Some(place), cx);
                            page.take_picture(cx);
                        });
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                if waited >= PATIENCE {
                    this.update(cx, |this, cx| {
                        this.finding = false;
                        this.said = Some(held_at(to, &this.steps).into());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                cx.background_executor().timer(RETRY).await;
                waited += RETRY;
            }
        }));
        cx.notify();
    }

    /// Next: the ringed target is acted on, and the walk goes on (`Act on the ringed target and go on`).
    fn next(&mut self, _: &Next, _window: &mut Window, cx: &mut Context<Self>) {
        if self.finding || self.at + 1 >= self.steps.len() {
            return;
        }
        let to = self.at + 1;
        if self.steps[self.at].press.is_some() {
            let ask = self.page.read(cx).ask(&finder::press());
            cx.spawn(async move |this, cx| {
                ask.recv().await.ok();
                this.update(cx, |this, cx| this.go(to, cx)).ok();
            })
            .detach();
        } else {
            self.go(to, cx);
        }
    }

    fn back(&mut self, _: &Back, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.finding && self.at > 0 {
            self.replay = Some(self.at - 1);
            self.started = false;
            self.page.update(cx, |page, cx| page.reload(cx));
        }
    }

    fn reload(&mut self, _: &Reload, _window: &mut Window, cx: &mut Context<Self>) {
        self.started = false;
        self.page.update(cx, |page, cx| page.reload(cx));
    }

    fn approve(&mut self, _: &Approve, _window: &mut Window, cx: &mut Context<Self>) {
        if self.live {
            return;
        }
        self.said = Some("Approved".into());
        cx.emit(WebWalkEvent::Approve);
        cx.notify();
    }

    fn bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let first = self.at == 0;
        let last = self.at + 1 >= self.steps.len();
        let said = self.steps.get(self.at).map(|step| SharedString::from(step.say.clone()));
        // Classic's walk bar: two rows of 44px, each padded 12 with 8 between.
        let row = || {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::SPACE_8) + look.length(token::SPACE_2) + look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
        };
        let top = row()
            .child(
                div()
                    .id("web-walk-name")
                    .flex_none()
                    .max_w(look.length(token::SPACE_8) * 8.)
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(self.title.clone()),
            )
            .child(div().flex_1().min_w_0().flex().items_center().children(self.said.clone().map(|said| {
                labelled(
                    div().id("web-walk-said").truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(said.clone()),
                    Role::Status,
                    said,
                )
            })))
            .child(icon_button("web-walk-reload", "refresh-cw", "Reload", Some("\u{2318}R"), Weight::Quiet, true, cx, cx.listener(|this, _, window, cx| this.reload(&Reload, window, cx))))
            .children((!self.live).then(|| div().min_w(look.length(token::SPACE_8) * 3.).w(look.length(token::SPACE_8) * 6.).child(button("web-walk-approve", "Approve", Some("\u{2318}\u{21B5}"), Weight::Primary, true, cx, cx.listener(|this, _, window, cx| this.approve(&Approve, window, cx))))));
        let line = row()
            .child(icon_button("web-walk-back", "chevron-left", "Replay to the step before", Some("\u{2325}\u{2318}\u{2190}"), Weight::Quiet, !first && !self.finding, cx, cx.listener(|this, _, window, cx| this.back(&Back, window, cx))))
            .child(
                div()
                    .flex_none()
                    .min_w(look.length(token::SPACE_8) + look.length(token::SPACE_4))
                    .flex()
                    .justify_center()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("{} of {}", self.at + 1, self.steps.len().max(1))),
            )
            .child(icon_button("web-walk-next", "chevron-right", "Act on the ringed target and go on", Some("\u{2325}\u{2318}\u{2192}"), Weight::Primary, !last && !self.finding, cx, cx.listener(|this, _, window, cx| this.next(&Next, window, cx))))
            .children(said.map(|said| {
                labelled(
                    div().id("web-walk-step").flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(said.clone()),
                    Role::Status,
                    said,
                )
            }));
        labelled(
            div()
                .id("web-walk-bar")
                .flex()
                .flex_col()
                .flex_none()
                .bg(look.color(token::BG_SUNKEN))
                // A press on the bar leaves the keys where they were.
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
                .child(top)
                .children((!self.live).then_some(line)),
            Role::Toolbar,
            format!("Walk {}", self.title),
        )
    }
}

impl Render for WebWalkTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = self.bar(cx);
        let look = cx.look();
        // Over a picture of the page the ring is gpui's; over the live page it is the page's own view.
        let ring = self.page.read(cx).ring_in_picture().filter(|_| self.steps.get(self.at).is_some_and(|step| target_of(step).is_some()));
        let gap = look.length(token::FOCUS_RING_WIDTH) * 2.;
        let page = labelled(
            div()
                .id("web-walk-page")
                .relative()
                .flex_1()
                .min_h(px(0.))
                .child(web_pane(&self.page).size_full())
                .children(ring.map(|place| {
                    labelled(
                        div()
                            .id("web-walk-ring")
                            .absolute()
                            .left(px(place.frame.x as f32) - gap)
                            .top(px(place.frame.y as f32) - gap)
                            .w(px(place.frame.width as f32) + gap * 2.)
                            .h(px(place.frame.height as f32) + gap * 2.)
                            .rounded(px(place.radius as f32).max(look.length(token::RADIUS_SM)) + gap)
                            .border(look.length(token::FOCUS_RING_WIDTH))
                            .border_color(look.color(token::ACCENT)),
                        Role::Image,
                        "The target of this step",
                    )
                })),
            Role::Region,
            "Walk",
        );
        div()
            .id("web-walk")
            .key_context("web-walk")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::next))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(Self::reload))
            .on_action(cx.listener(Self::approve))
            .size_full()
            .flex()
            .flex_col()
            .child(bar)
            .child(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(role: &str, name: &str) -> WalkTarget {
        WalkTarget { role: role.into(), name: Some(name.into()), exact: None, within: None }
    }

    fn step(say: &str) -> WalkStep {
        WalkStep { say: say.into(), press: None, look: None, hover: None, later: None, typed: None, into: None, key: None, on: None, select: None, within: None, paste: None, drag: None, by: None }
    }

    #[test]
    fn a_step_aims_at_the_target_its_verb_names() {
        let press = WalkStep { press: Some(target("button", "Add")), ..step("Add one") };
        let typed = WalkStep { typed: Some("hi".into()), into: Some(target("textbox", "Name")), ..step("Name it") };
        assert_eq!(target_of(&press).map(|one| one.role.as_str()), Some("button"));
        assert_eq!(target_of(&typed).map(|one| one.role.as_str()), Some("textbox"));
        assert_eq!(target_of(&step("nothing")), None);
    }

    #[test]
    fn a_held_step_says_its_place_and_its_target_as_a_person_would() {
        let steps = vec![WalkStep { press: Some(WalkTarget { within: Some(Box::new(target("dialog", "New"))), ..target("button", "Save") }), ..step("Save it") }];
        assert_eq!(
            held_at(0, &steps),
            "Step 1 of 1: nothing to press, no button named \u{201C}Save\u{201D} in a dialog named \u{201C}New\u{201D} came within 5 seconds"
        );
    }

    #[test]
    fn a_local_file_is_loaded_as_a_file_and_anything_else_as_an_address() {
        assert!(matches!(source_of("file:///tmp/walk/page.html?x=1"), Source::File(path) if path == "/tmp/walk/page.html"));
        assert!(matches!(source_of("https://example.com/a"), Source::Url(url) if url == "https://example.com/a"));
    }
}
