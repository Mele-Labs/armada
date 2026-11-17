//! A web page in a gpui window: the entity that holds it, and the element that reports where it is.
//!
//! **The element reports its bounds on every frame it is painted**, and the page is placed there as a
//! subview of the window's own NSView, as Classic's `LiveWalk` reports its frame to main. A page the
//! window did not paint this frame is hidden after it, and keeps its place, scroll and state, as Classic's
//! `job-walks.ts` parks a walk's view. While a gpui layer lies over the page (a sheet, a menu, a popover),
//! or where the window has no NSView (the walk check), the element draws the page's last snapshot in its
//! place, as `WalkPane.tsx`'s LiveWalk swaps in `pictures.page` while `holdLayerUp` holds.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    canvas, point, px, App, Bounds, Context, Corners, Entity, EventEmitter, Global, Hitbox, HitboxBehavior, IntoElement, Pixels,
    RenderImage, Styled, Task, WeakEntity, Window, WindowId,
};

use super::finder::Place;
use super::frame::Frame;
use super::wk::{self, Nav, Native, Options};

/// Where a page is in its life.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not drawn yet, so not loaded.
    Idle,
    Loading,
    Ready,
    Failed(String),
    /// WebKit could not be used: off the main thread.
    Absent,
}

/// What a page tells whoever watches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebPageEvent {
    /// The load ended, one way or the other (`Status`).
    Loaded,
    /// A new picture of the page is held.
    Pictured,
    /// What the page draws changed hands: live in the window, or a picture of it.
    Shown,
    /// The kit's script on a sealed page posted this, as JSON text (`kit::read` reads it).
    Posted(String),
}

/// How a page is drawn now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Not painted this frame.
    Parked,
    /// The page's own view is in the window at the element's bounds.
    Live,
    /// The element draws the last snapshot.
    Picture,
}

/// What a page is made from.
pub enum Source {
    Url(String),
    File(String),
    /// A page Fleet served sealed (`get_artifact_page`): loaded as text with no address, kept from going anywhere, and
    /// heard by the kit's handler. Its own policy lets the kit's script alone run.
    Sealed(String),
}

/// Takes the scroll bars out of a page laid out in a shell, so it fills its frame as the picture does. `display: none` on the
/// bar alone leaves its width to the layout; no overflow on the root frees it, and a script still scrolls what it asks to.
const NO_SCROLLBARS: &str = "(() => { const style = document.createElement('style'); style.textContent = '::-webkit-scrollbar { display: none } html { overflow: hidden !important }'; document.documentElement.appendChild(style); return '1'; })()";

pub struct WebPage {
    native: Option<Native>,
    /// What the page loads and how it is made, until its element first reports a frame: **the page is made then**, as
    /// Classic makes a walk's view when its pane first reports one, so a walk nobody has opened loads nothing.
    waiting: Option<(Option<String>, Source, async_channel::Sender<Nav>, Option<async_channel::Sender<String>>)>,
    status: Status,
    picture: Option<Arc<RenderImage>>,
    /// Where the element was painted last, in the window.
    bounds: Option<Bounds<Pixels>>,
    visible: Option<Bounds<Pixels>>,
    hitbox: Option<Hitbox>,
    /// A gpui layer lies over the page, so its picture draws and its view is hidden.
    held: bool,
    /// The window has no AppKit view of its own (the headless check): the page is laid out in a shell nobody sees, and a scroll
    /// bar the snapshot cannot draw would leave a blank strip at its edge and take its width from the layout.
    shell: bool,
    drawn: bool,
    mode: Mode,
    ring: Option<Place>,
    /// A snapshot is on its way, so none is asked for twice; `again` says the page changed since it was asked.
    pictured: bool,
    again: bool,
    /// The snapshot's wait and decode, which a newer ask replaces only once it has ended.
    deciding: Option<Task<()>>,
    _watching: Task<()>,
    /// The kit's posts, handed on as `WebPageEvent::Posted`.
    _hearing: Task<()>,
    /// The store of sign-ins this page keeps, where it is not its manifest's (`for_job_browser`).
    store: Option<[u8; 16]>,
    /// Where the page's address, title and history changes go once it is made (`watch_changes`).
    changes: Option<async_channel::Sender<()>>,
    /// Where a Job browser's page sends what it asks for once it is made (`hear_requests`).
    asks: Option<async_channel::Sender<wk::Request>>,
}

impl EventEmitter<WebPageEvent> for WebPage {}

impl WebPage {
    /// A page of the manifest `manifest`, to load `source` once it is first drawn. Pages of one manifest share one
    /// store of sign-ins, and two manifests share nothing (`frame::store_id`).
    pub fn new(manifest: Option<&str>, source: Source, cx: &mut Context<Self>) -> Self {
        let (events, arrivals) = async_channel::unbounded();
        let (posts, posted) = async_channel::unbounded::<String>();
        let kit = matches!(source, Source::Sealed(_)).then_some(posts);
        let hearing = cx.spawn(async move |this, cx| {
            while let Ok(text) = posted.recv().await {
                if this.update(cx, |_, cx| cx.emit(WebPageEvent::Posted(text))).is_err() {
                    break;
                }
            }
        });
        let watching = cx.spawn(async move |this, cx| {
            while let Ok(arrival) = arrivals.recv().await {
                let kept = this.update(cx, |page, cx| {
                    // A load that was replaced is not an end: the load that replaced it ends.
                    if matches!(arrival, Nav::Cancelled) {
                        return;
                    }
                    if matches!(arrival, Nav::Finished) && page.shell {
                        if let Some(native) = &page.native {
                            native.evaluate(NO_SCROLLBARS, |_| {});
                        }
                    }
                    page.status = match arrival {
                        Nav::Finished => Status::Ready,
                        Nav::Failed(why) => Status::Failed(why),
                        Nav::Cancelled => return,
                    };
                    cx.emit(WebPageEvent::Loaded);
                    cx.notify();
                });
                if kept.is_err() {
                    break;
                }
            }
        });
        WebPage {
            native: None,
            waiting: Some((manifest.map(str::to_string), source, events, kit)),
            status: Status::Idle,
            picture: None,
            bounds: None,
            visible: None,
            hitbox: None,
            held: false,
            shell: false,
            drawn: false,
            mode: Mode::Parked,
            ring: None,
            pictured: false,
            again: false,
            deciding: None,
            _watching: watching,
            _hearing: hearing,
            store: None,
            changes: None,
            asks: None,
        }
    }

    // ---- a Job's browser ----------------------------------------------------------------------------------------

    /// A page of a Job's browser: empty (`about:blank`) until something loads in it, and keeping its sign-ins in the one store
    /// every Job browser shares (`frame::job_browser_store_id`), never a manifest's.
    pub fn for_job_browser(cx: &mut Context<Self>) -> Self {
        let mut page = WebPage::new(None, Source::Url("about:blank".into()), cx);
        page.store = Some(super::frame::job_browser_store_id());
        page
    }

    /// Makes the page now, at `width` by `height` points, where nothing has drawn it yet. WebKit loads and runs a page that is in
    /// no window, which is how an agent's act reaches a tab nobody is looking at. Does nothing where it is made already.
    pub fn made_at(&mut self, width: f64, height: f64) {
        self.make(Frame::new(0., 0., width.max(1.), height.max(1.)));
    }

    /// Loads `url`: now where the page is made, else once it is first drawn.
    pub fn load_url(&mut self, url: &str, cx: &mut Context<Self>) {
        match &self.native {
            Some(native) => {
                native.load_url(url);
                self.status = Status::Loading;
            }
            None => {
                if let Some((_, source, ..)) = &mut self.waiting {
                    *source = Source::Url(url.to_string());
                }
            }
        }
        cx.notify();
    }

    /// The page's address, or none while it has none.
    pub fn url(&self) -> Option<String> {
        self.native.as_ref().and_then(Native::url)
    }

    pub fn title(&self) -> String {
        self.native.as_ref().map(Native::title).unwrap_or_default()
    }

    /// Whether the page can go back, and whether it can go forward.
    pub fn history(&self) -> (bool, bool) {
        self.native.as_ref().map_or((false, false), |native| (native.can_go_back(), native.can_go_forward()))
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(native) = &self.native {
            native.go_back();
            self.status = Status::Loading;
            cx.notify();
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(native) = &self.native {
            native.go_forward();
            self.status = Status::Loading;
            cx.notify();
        }
    }

    /// Hears the page's address, title and history move, including by a navigation inside the page. Ask once, before the page
    /// is made or after.
    pub fn watch_changes(&mut self) -> async_channel::Receiver<()> {
        let (told, hear) = async_channel::unbounded();
        match &self.native {
            Some(native) => native.watch(told),
            None => self.changes = Some(told),
        }
        hear
    }

    /// Hears what the page asks for: a permission, each to be answered once. Ask once, before the page is made.
    pub fn hear_requests(&mut self) -> async_channel::Receiver<wk::Request> {
        let (tx, hear) = async_channel::unbounded();
        self.asks = Some(tx);
        hear
    }

    /// The keys, if the page holds them, go back to the window's own view.
    pub fn release_keys(&self) {
        if let Some(native) = &self.native {
            native.release_keys();
        }
    }

    /// Runs `body` as an async function's body in the content world named `world`, awaiting what it returns. The answer is its
    /// string, `None` where it returned none, or the page's words for a throw. A page not made answers an error at once.
    pub fn call_async(&self, world: &str, body: &str) -> async_channel::Receiver<Result<Option<String>, String>> {
        let (tx, rx) = async_channel::bounded(1);
        match &self.native {
            Some(native) => native.call_async(world, body, move |answer| {
                let _ = tx.try_send(answer);
            }),
            None => {
                let _ = tx.try_send(Err("The page is not open.".to_string()));
            }
        }
        rx
    }

    /// Runs `script` in the content world named `world`; the answer is its string result (`None`: it threw or answered none).
    pub fn evaluate_in(&self, world: &str, script: &str) -> async_channel::Receiver<Option<String>> {
        let (tx, rx) = async_channel::bounded(1);
        match &self.native {
            Some(native) => native.evaluate_in(world, script, move |answer| {
                let _ = tx.try_send(answer);
            }),
            None => {
                let _ = tx.try_send(None);
            }
        }
        rx
    }

    /// The viewport as a PNG, as the page is laid out (`None`: nothing could be taken).
    pub fn png(&self) -> async_channel::Receiver<Option<Vec<u8>>> {
        let (tx, rx) = async_channel::bounded(1);
        match &self.native {
            Some(native) => {
                let (width, _) = native.size();
                native.snapshot(None, width.max(1.), move |png| {
                    let _ = tx.try_send(png);
                });
            }
            None => {
                let _ = tx.try_send(None);
            }
        }
        rx
    }

    /// Whether a pane draws the page now, live or as a picture.
    pub fn is_painted(&self) -> bool {
        self.mode != Mode::Parked
    }

    /// The page, made and loading, where it was not yet.
    fn make(&mut self, size: Frame) {
        let Some((manifest, source, events, kit)) = self.waiting.take() else { return };
        // A sealed page keeps nothing and goes nowhere: no store of sign-ins, and every navigation after its text refused.
        let sealed = matches!(source, Source::Sealed(_));
        let store = (!sealed).then(|| self.store.unwrap_or_else(|| super::frame::store_id(manifest.as_deref())));
        let options = Options { store, scripts: true, sealed, dark: None, clear: false, kit };
        self.native = Native::with_site(&options, size, events, self.asks.take().map(|requests| wk::Site { requests, scripts: vec![include_str!("../job_browser/location.js")] }));
        if let (Some(native), Some(told)) = (&self.native, self.changes.take()) {
            native.watch(told);
        }
        match (&self.native, &source) {
            (Some(native), Source::Url(url)) => native.load_url(url),
            (Some(native), Source::File(path)) => native.load_file(path),
            (Some(native), Source::Sealed(html)) => native.load_html(html),
            (None, _) => {}
        }
        self.status = if self.native.is_some() { Status::Loading } else { Status::Absent };
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The page's viewport size in points, as it was last painted.
    pub fn viewport(&self) -> (f64, f64) {
        self.bounds.map_or((0., 0.), |bounds| (f64::from(bounds.size.width), f64::from(bounds.size.height)))
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        if let Some(native) = &self.native {
            native.reload();
            self.status = Status::Loading;
            cx.notify();
        }
    }

    /// The ring over a step's target, a box in the page's points. Live, the page's own ring draws; as a
    /// picture, `ring_in_picture` hands it to the screen to draw.
    pub fn set_ring(&mut self, ring: Option<Place>, cx: &mut Context<Self>) {
        if self.ring != ring {
            self.ring = ring;
            cx.notify();
        }
    }

    /// The ring to draw over the picture, where the picture is what is shown.
    pub fn ring_in_picture(&self) -> Option<Place> {
        if self.mode == Mode::Picture {
            self.ring
        } else {
            None
        }
    }

    /// Runs `script` in the finder's world. The answer comes on the returned channel (`None`: nothing, or WebKit
    /// could not answer).
    pub fn ask(&self, script: &str) -> async_channel::Receiver<Option<String>> {
        let (tx, rx) = async_channel::bounded(1);
        match &self.native {
            Some(native) => native.evaluate(script, move |answer| {
                let _ = tx.try_send(answer);
            }),
            None => {
                let _ = tx.try_send(None);
            }
        }
        rx
    }

    /// Runs `script` in the page's own world, as the host answers the kit (`window.armadaHost`). The channel closes once it
    /// has run, or at once where there is no page yet.
    pub fn tell(&self, script: &str) -> async_channel::Receiver<()> {
        let (tx, rx) = async_channel::bounded(1);
        if let Some(native) = &self.native {
            native.tell(script, move || {
                let _ = tx.try_send(());
            });
        }
        rx
    }

    /// A fresh picture of the page, held for where it cannot be drawn live. Decoded off the main thread.
    pub fn take_picture(&mut self, cx: &mut Context<Self>) {
        let Some(native) = &self.native else { return };
        if self.pictured {
            self.again = true;
            return;
        }
        self.pictured = true;
        let (width, _) = self.viewport();
        let (tx, rx) = async_channel::bounded(1);
        let rect = (width > 0.).then(|| Frame::new(0., 0., width, self.viewport().1));
        native.snapshot(rect, width.max(1.), move |png| {
            let _ = tx.try_send(png);
        });
        let decode = cx.spawn(async move |this, cx| {
            let png = rx.recv().await.ok().flatten();
            let picture = match png {
                Some(png) => cx.background_executor().spawn(async move { super::render::decode(&png, None) }).await,
                None => None,
            };
            this.update(cx, |page, cx| {
                page.pictured = false;
                if std::mem::take(&mut page.again) {
                    page.take_picture(cx);
                }
                if let Some(picture) = picture {
                    page.picture = Some(picture);
                    cx.emit(WebPageEvent::Pictured);
                    cx.notify();
                }
                page.settle_hold(cx);
            })
            .ok();
        });
        self.deciding = Some(decode);
    }

    /// A held page hides its view once its picture is in, so the layer over it is not covered meanwhile.
    fn settle_hold(&mut self, cx: &mut Context<Self>) {
        if self.held && self.mode == Mode::Live {
            if let Some(native) = &self.native {
                native.park();
            }
            self.mode = Mode::Picture;
            cx.emit(WebPageEvent::Shown);
            cx.notify();
        }
    }

    /// Called as the element paints: place the page at `bounds`, or draw its picture.
    fn painted(&mut self, bounds: Bounds<Pixels>, visible: Bounds<Pixels>, hitbox: Hitbox, window: &mut Window, cx: &mut Context<Self>) {
        self.drawn = true;
        self.bounds = Some(bounds);
        self.visible = Some(visible);
        self.hitbox = Some(hitbox.clone());
        register(window, cx.entity().downgrade(), cx);
        let page = cx.entity().downgrade();
        cx.default_global::<Frames>().log.push(Painted { page, bounds, visible, hitbox });
        let frame = frame_of(bounds);
        self.make(frame);
        let Some(native) = &self.native else { return };
        let parent = wk::parent_of(window);
        let before = self.mode;
        match parent {
            Some(parent) if !self.held => {
                native.place(&parent, frame);
                native.set_ring(&parent, frame, self.ring.map(|place| place.frame));
                self.mode = Mode::Live;
            }
            Some(parent) => {
                // Held: the view stays until the picture is in (`settle_hold`).
                native.set_ring(&parent, frame, None);
                if self.mode == Mode::Parked {
                    self.mode = Mode::Picture;
                }
            }
            None => {
                // No window of AppKit's own: the page is laid out at the element's size in its shell, and drawn as a picture.
                native.set_size(frame.width, frame.height);
                native.clear_ring();
                self.shell = true;
                self.mode = Mode::Picture;
            }
        }
        if self.mode != before {
            cx.emit(WebPageEvent::Shown);
            cx.notify();
        }
    }

    /// After the frame: a page not painted is parked, and a painted one is held or let go as a layer comes or goes.
    fn after_draw(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.drawn {
            if let Some(native) = &self.native {
                native.park();
            }
            if self.mode != Mode::Parked {
                self.mode = Mode::Parked;
                cx.emit(WebPageEvent::Shown);
            }
            return;
        }
        self.drawn = false;
        let covered = self.covered(window);
        if covered != self.held {
            self.held = covered;
            if covered {
                // Its picture is taken before the view goes, so the layer over it is not left over a blank.
                self.take_picture(cx);
                if self.picture.is_some() {
                    self.settle_hold(cx);
                }
            } else if self.mode == Mode::Picture && wk::parent_of(window).is_some() {
                self.mode = Mode::Parked;
            }
            window.refresh();
            cx.notify();
        }
    }

    /// Whether anything that blocks the pointer lies over part of the page: nine points across its visible
    /// part are asked of gpui's hit test, as of the frame just drawn.
    fn covered(&self, window: &Window) -> bool {
        let (Some(hitbox), Some(visible)) = (&self.hitbox, self.visible) else { return false };
        if visible.size.width <= px(2.) || visible.size.height <= px(2.) {
            return false;
        }
        for across in [0.1f32, 0.5, 0.9] {
            for down in [0.1f32, 0.5, 0.9] {
                let at = point(visible.origin.x + visible.size.width * across, visible.origin.y + visible.size.height * down);
                if !hitbox.is_hovered_at(at, window) {
                    return true;
                }
            }
        }
        false
    }
}

fn frame_of(bounds: Bounds<Pixels>) -> Frame {
    Frame::new(f64::from(bounds.origin.x), f64::from(bounds.origin.y), f64::from(bounds.size.width), f64::from(bounds.size.height))
}

/// The pages each window has painted, so the frame's end can park the ones it did not.
#[derive(Default)]
struct Frames {
    by_window: HashMap<WindowId, Vec<WeakEntity<WebPage>>>,
    /// Each page painted since the last frame ended, where: what a view drawn from its cache paints again.
    log: Vec<Painted>,
}

impl Global for Frames {}

/// One page's paint: the page and where it was placed (`labels::kept`).
#[derive(Clone)]
pub struct Painted {
    page: WeakEntity<WebPage>,
    bounds: Bounds<Pixels>,
    visible: Bounds<Pixels>,
    hitbox: Hitbox,
}

/// Where the paints a view is about to make start.
pub fn painted_mark(cx: &App) -> usize {
    cx.try_global::<Frames>().map_or(0, |frames| frames.log.len())
}

/// The paints made since `mark`.
pub fn painted_since(mark: usize, cx: &App) -> Vec<Painted> {
    cx.try_global::<Frames>().map_or_else(Vec::new, |frames| frames.log.get(mark..).map_or_else(Vec::new, <[Painted]>::to_vec))
}

/// Paint each page again where it was: its view was drawn from gpui's cache, which runs no paint of ours.
pub fn paint_again(pages: &[Painted], window: &mut Window, cx: &mut App) {
    for one in pages {
        if let Some(page) = one.page.upgrade() {
            page.update(cx, |page, cx| page.painted(one.bounds, one.visible, one.hitbox.clone(), window, cx));
        }
    }
}

fn register(window: &Window, page: WeakEntity<WebPage>, cx: &mut App) {
    let id = window.window_handle().window_id();
    let pages = cx.default_global::<Frames>().by_window.entry(id).or_default();
    if !pages.iter().any(|one| one == &page) {
        pages.push(page);
    }
}

/// Called by the window's root as it finishes painting. **Once the frame is drawn**, every page of the window
/// the frame did not paint is parked, and every one it painted is checked for a layer over it.
pub fn after_frame(window: &mut Window, cx: &mut App) {
    if cx.has_global::<Frames>() {
        cx.global_mut::<Frames>().log.clear();
    }
    let Some(frames) = cx.try_global::<Frames>() else { return };
    let id = window.window_handle().window_id();
    if frames.by_window.get(&id).is_none_or(Vec::is_empty) {
        return;
    }
    window.defer(cx, move |window, cx| {
        let pages = cx.default_global::<Frames>().by_window.entry(id).or_default();
        pages.retain(|page| page.upgrade().is_some());
        let pages: Vec<_> = pages.iter().filter_map(WeakEntity::upgrade).collect();
        for page in pages {
            page.update(cx, |page, cx| page.after_draw(window, cx));
        }
    });
}

/// The element: it takes the room it is given, reports it, and draws the page's picture when the page
/// is not live. Give it a size (`size_full`, `flex_1`) as any element.
pub fn web_pane(page: &Entity<WebPage>) -> impl IntoElement + Styled {
    let prepaint_page = page.clone();
    let page = page.clone();
    canvas(
        move |bounds, window, _cx| {
            let _ = &prepaint_page;
            let visible = bounds.intersect(&window.content_mask().bounds);
            (window.insert_hitbox(bounds, HitboxBehavior::Normal), visible)
        },
        move |bounds, (hitbox, visible), window, cx| {
            let picture = page.update(cx, |page, cx| {
                page.painted(bounds, visible, hitbox, window, cx);
                (page.mode == Mode::Picture).then(|| page.picture.clone()).flatten()
            });
            if let Some(picture) = picture {
                window.paint_image(bounds, bounds, Corners::default(), picture, 0, false).ok();
            }
        },
    )
}
