//! Apple's web view, WKWebView, as the native Bridge holds a page. **The one file in the crate that
//! speaks `unsafe` for web pages** (`xtask/src/rules_unsafe.rs` lists it): objc2-web-kit declares nearly
//! every WebKit call unsafe, and AppKit's event monitor and first responder have no safe form either.
//! Everything outside this file sees `Native`, `Parent` and plain values, on the main thread only.
//!
//! A page is a real NSView, so it ports Armada Classic's shape directly: Classic lays a live
//! `WebContentsView` over the frame its pane reports. Here the frame is gpui's, and the view is a
//! subview of the gpui window's own NSView, put at those bounds. A page nobody has placed is in no window
//! at all, and WebKit still loads it, runs its scripts and snapshots it, so nothing is ever on screen.
#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSBezierPath, NSBitmapImageFileType, NSBitmapImageRep, NSColor, NSEvent, NSEventMask,
    NSEventType, NSImage, NSResponder, NSView, NSWindow,
};
use objc2_foundation::{
    NSDate, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRunLoop, NSSize, NSString,
    NSURL, NSURLRequest, NSUUID,
};
use objc2_web_kit::{
    WKContentWorld, WKNavigation, WKNavigationAction, WKNavigationActionPolicy,
    WKNavigationDelegate, WKNavigationType, WKSnapshotConfiguration, WKWebView, WKWebViewConfiguration,
    WKWebpagePreferences, WKWebsiteDataStore,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::frame::{frame_in, Frame};
use super::keys::{self, Press};

thread_local! {
    /// Pages alive now, so a window that never made one is not made to wait on any.
    static LIVE: Cell<usize> = const { Cell::new(0) };
    /// Asks made of WebKit that it has not answered yet: a load, a snapshot or a script. The walk check
    /// pumps the run loop while any is outstanding, since nothing else turns it.
    static PENDING: Cell<usize> = const { Cell::new(0) };
    /// The pages' views and the gpui views they sit in, for the key and pointer monitor.
    static HOSTED: RefCell<Vec<Hosted>> = const { RefCell::new(Vec::new()) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

struct Hosted {
    view: Retained<WKWebView>,
    parent: Retained<NSView>,
}

/// What a page tells whoever holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nav {
    Finished,
    Failed(String),
}

/// How a page is made.
pub struct Options {
    /// The persistent store its sign-ins live in (`frame::store_id`). `None`: nothing is kept.
    pub store: Option<[u8; 16]>,
    /// Whether the page's own scripts run.
    pub scripts: bool,
    /// Whether every request but the page's own text is refused: a sketch's mock draws only what it holds.
    pub sealed: bool,
    /// The page's appearance, where it is not the system's: dark when `Some(true)`.
    pub dark: Option<bool>,
}

fn pending_up() {
    PENDING.with(|pending| pending.set(pending.get() + 1));
}

fn pending_down() {
    PENDING.with(|pending| pending.set(pending.get().saturating_sub(1)));
}

/// Whether any page is alive.
pub fn in_use() -> bool {
    LIVE.with(|live| live.get() > 0)
}

/// Whether WebKit owes an answer.
pub fn pending() -> bool {
    PENDING.with(|pending| pending.get() > 0)
}

/// Runs the main run loop for up to `seconds`, so WebKit's answers are delivered. A gpui window on
/// screen turns it itself; the walk check has no such window and calls this.
pub fn pump(seconds: f64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(seconds));
}

struct NavState {
    events: async_channel::Sender<Nav>,
    sealed: bool,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Navigation` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = NavState]
    struct Navigation;

    unsafe impl NSObjectProtocol for Navigation {}

    unsafe impl WKNavigationDelegate for Navigation {
        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish(&self, _: &WKWebView, _: Option<&WKNavigation>) {
            pending_down();
            let _ = self.ivars().events.try_send(Nav::Finished);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail(&self, _: &WKWebView, _: Option<&WKNavigation>, error: &NSError) {
            pending_down();
            let _ = self.ivars().events.try_send(Nav::Failed(error.localizedDescription().to_string()));
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_first(&self, _: &WKWebView, _: Option<&WKNavigation>, error: &NSError) {
            pending_down();
            let _ = self.ivars().events.try_send(Nav::Failed(error.localizedDescription().to_string()));
        }

        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn decide(&self, _: &WKWebView, action: &WKNavigationAction, handler: &DynBlock<dyn Fn(WKNavigationActionPolicy)>) {
            // A sealed page is given its text once and goes nowhere after: a link, a redirect or a form is refused.
            // SAFETY: `action` is the live action WebKit passed this call, read on the main thread.
            let (kind, is_blank) = unsafe {
                let url = action.request().URL().map(|url| url.absoluteString().map(|text| text.to_string()).unwrap_or_default());
                (action.navigationType(), url.as_deref().is_none_or(|url| url.is_empty() || url.starts_with("about:")))
            };
            let allowed = !self.ivars().sealed || (kind == WKNavigationType::Other && is_blank);
            handler.call((if allowed { WKNavigationActionPolicy::Allow } else { WKNavigationActionPolicy::Cancel },));
        }
    }
);

impl Navigation {
    fn new(events: async_channel::Sender<Nav>, sealed: bool, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(NavState { events, sealed });
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSView's `hitTest:` and `drawRect:` may be overridden, and `RingView` implements no `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    struct RingView;

    impl RingView {
        // The ring is only drawn: a press goes through it to the page under it.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }

        // A rounded outline in Armada's accent, as a native screen's own ring draws it, with nothing inside it.
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let bounds = self.bounds();
            let inside = NSRect::new(
                NSPoint::new(bounds.origin.x + 1., bounds.origin.y + 1.),
                NSSize::new((bounds.size.width - 2.).max(0.), (bounds.size.height - 2.).max(0.)),
            );
            let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(inside, 6., 6.);
            path.setLineWidth(2.);
            NSColor::colorWithSRGBRed_green_blue_alpha(0.28, 0.52, 1., 1.).setStroke();
            path.stroke();
        }
    }
);

/// One page: a WKWebView, the ring over it that marks a walk step's target, and its way of telling
/// when it has loaded.
pub struct Native {
    view: Retained<WKWebView>,
    ring: Retained<RingView>,
    _navigation: Retained<Navigation>,
    mtm: MainThreadMarker,
}

/// The gpui window's own NSView, which a page is placed in.
pub struct Parent(Retained<NSView>);

impl Parent {
    /// The view's height and whether it counts from the top, for `frame_in`.
    pub fn geometry(&self) -> (f64, bool) {
        (self.0.bounds().size.height, self.0.isFlipped())
    }
}

/// The NSView of a gpui window, or `None` where it has none (the walk check's headless window).
pub fn parent_of(window: &gpui::Window) -> Option<Parent> {
    MainThreadMarker::new()?;
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return None };
    // SAFETY: gpui's macOS window hands out the pointer to its own live NSView, and it stays alive as long as
    // the window does; it is retained here for the length of this value, on the main thread.
    unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }.map(Parent)
}

fn rect(frame: Frame) -> NSRect {
    NSRect::new(NSPoint::new(frame.x, frame.y), NSSize::new(frame.width, frame.height))
}

impl Native {
    /// A page, or `None` off the main thread, where WebKit cannot be used.
    pub fn new(options: &Options, size: Frame, events: async_channel::Sender<Nav>) -> Option<Native> {
        let mtm = MainThreadMarker::new()?;
        // SAFETY: every call below is on the main thread, on objects made here, with arguments of the types
        // WebKit documents; the data store is a persistent one WebKit keeps by its identifier.
        let (view, navigation) = unsafe {
            let config = WKWebViewConfiguration::new(mtm);
            match options.store {
                Some(id) => {
                    // From its text, not its bytes: objc2's debug check refuses `initWithUUIDBytes:` on this
                    // macOS, so a debug build, which Fleet plays walks from, panicked on every web page.
                    let hex: String = id.iter().map(|byte| format!("{byte:02X}")).collect();
                    let text = format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]);
                    let uuid = NSUUID::from_string(&NSString::from_str(&text)).expect("a UUID's own text parses");
                    config.setWebsiteDataStore(&WKWebsiteDataStore::dataStoreForIdentifier(&uuid, mtm));
                }
                None => config.setWebsiteDataStore(&WKWebsiteDataStore::nonPersistentDataStore(mtm)),
            }
            if !options.scripts {
                let preferences = WKWebpagePreferences::new(mtm);
                preferences.setAllowsContentJavaScript(false);
                config.setDefaultWebpagePreferences(Some(&preferences));
            }
            let view = WKWebView::initWithFrame_configuration(mtm.alloc(), rect(size), &config);
            let navigation = Navigation::new(events, options.sealed, mtm);
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*navigation)));
            (view, navigation)
        };
        if let Some(dark) = options.dark {
            // SAFETY: `NSAppearanceNameDarkAqua` and the aqua name are AppKit's own constants.
            let name = unsafe { if dark { NSAppearanceNameDarkAqua } else { objc2_app_kit::NSAppearanceNameAqua } };
            view.setAppearance(NSAppearance::appearanceNamed(name).as_deref());
        }
        let ring = mtm.alloc::<RingView>().set_ivars(());
        // SAFETY: `NSView`'s designated initialiser, with a zero frame.
        let ring: Retained<RingView> = unsafe { msg_send![super(ring), initWithFrame: rect(Frame::new(0., 0., 0., 0.))] };
        ring.setHidden(true);
        LIVE.with(|live| live.set(live.get() + 1));
        Some(Native { view, ring, _navigation: navigation, mtm })
    }

    pub fn load_url(&self, url: &str) {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else { return };
        pending_up();
        // SAFETY: a request for a URL made on the main thread.
        unsafe {
            let request = NSURLRequest::requestWithURL(&url);
            self.view.loadRequest(&request);
        }
    }

    /// A local file, which may read the folder it is in (a fixture's own styles and scripts).
    pub fn load_file(&self, path: &str) {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let folder = url.URLByDeletingLastPathComponent();
        pending_up();
        // SAFETY: a file URL and the folder it sits in, as WebKit documents `loadFileURL:allowingReadAccessToURL:`.
        unsafe {
            self.view.loadFileURL_allowingReadAccessToURL(&url, folder.as_deref().unwrap_or(&url));
        }
    }

    pub fn load_html(&self, html: &str) {
        pending_up();
        // SAFETY: a string and no base URL, so the page has no origin to reach.
        unsafe {
            self.view.loadHTMLString_baseURL(&NSString::from_str(html), None);
        }
    }

    pub fn reload(&self) {
        pending_up();
        // SAFETY: a reload on the main thread.
        unsafe {
            self.view.reload();
        }
    }

    /// The view's size in points, which the layout and a snapshot follow.
    pub fn set_size(&self, width: f64, height: f64) {
        let at = self.view.frame().origin;
        self.view.setFrame(NSRect::new(at, NSSize::new(width, height)));
    }

    /// The page placed over the gpui window at `bounds` (measured from the window's top-left), and shown.
    /// The view is a subview of the window's NSView; its first placement adds it.
    pub fn place(&self, parent: &Parent, bounds: Frame) {
        let (height, flipped) = parent.geometry();
        let frame = frame_in(bounds, height, flipped);
        // SAFETY: reads the view's superview, on the main thread.
        let in_parent = unsafe { self.view.superview() }.is_some_and(|superview| Retained::as_ptr(&superview) == Retained::as_ptr(&parent.0));
        if !in_parent {
            self.ring.removeFromSuperview();
            parent.0.addSubview(&self.view);
            parent.0.addSubview(&self.ring);
            register(&self.view, &parent.0);
        }
        self.view.setFrame(rect(frame));
        self.view.setHidden(false);
        let _ = self.mtm;
    }

    /// The page hidden, keeping its place, scroll and state, as Classic parks a walk's view.
    pub fn park(&self) {
        self.view.setHidden(true);
        self.ring.setHidden(true);
    }

    /// The ring over `ring`, a box in the page's own points, or none. `bounds` is where the page is drawn.
    pub fn set_ring(&self, parent: &Parent, bounds: Frame, ring: Option<Frame>) {
        let Some(ring) = ring else {
            self.ring.setHidden(true);
            return;
        };
        let (height, flipped) = parent.geometry();
        let gap = 3.;
        let on_window = Frame::new(bounds.x + ring.x - gap, bounds.y + ring.y - gap, ring.width + gap * 2., ring.height + gap * 2.);
        self.ring.setFrame(rect(frame_in(on_window, height, flipped)));
        self.ring.setHidden(false);
    }

    /// Hides the ring where the page is not placed in a window.
    pub fn clear_ring(&self) {
        self.ring.setHidden(true);
    }

    /// The page's pixels as PNG bytes. `rect` is the part to take, in the page's points (`None`: what is
    /// shown); `width` is the picture's width in points, so a larger one is a sharper one.
    pub fn snapshot(&self, rect_in_page: Option<Frame>, width: f64, done: impl FnOnce(Option<Vec<u8>>) + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |image: *mut NSImage, _error: *mut NSError| {
            pending_down();
            let png = if image.is_null() {
                None
            } else {
                // SAFETY: WebKit passes a live image for the length of this call.
                let image = unsafe { &*image };
                image
                    .TIFFRepresentation()
                    .and_then(|tiff| NSBitmapImageRep::imageRepWithData(&tiff))
                    // SAFETY: a PNG encoding of a bitmap, with no properties.
                    .and_then(|rep| unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) })
                    .map(|data| data.to_vec())
            };
            if let Some(done) = done.borrow_mut().take() {
                done(png);
            }
        });
        // SAFETY: the configuration is made and filled on the main thread; the block is copied by WebKit.
        unsafe {
            let config = WKSnapshotConfiguration::new(self.mtm);
            if let Some(part) = rect_in_page {
                config.setRect(rect(part));
            }
            config.setSnapshotWidth(Some(&NSNumber::new_f64(width)));
            self.view.takeSnapshotWithConfiguration_completionHandler(Some(&config), &block);
        }
    }

    /// Runs `script` in a content world of Bridge's own, which shares the page's DOM and nothing else, and
    /// answers its string result (`None`: it threw or answered no string).
    pub fn evaluate(&self, script: &str, done: impl FnOnce(Option<String>) + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |result: *mut AnyObject, _error: *mut NSError| {
            pending_down();
            let text = if result.is_null() {
                None
            } else {
                // SAFETY: WebKit passes a live object for the length of this call.
                let object = unsafe { &*result };
                object.downcast_ref::<NSString>().map(|text| text.to_string())
            };
            if let Some(done) = done.borrow_mut().take() {
                done(text);
            }
        });
        // SAFETY: a script string and a named content world, evaluated on the main thread; the block is copied.
        unsafe {
            let world = WKContentWorld::worldWithName(&NSString::from_str("armada-walk-finder"), self.mtm);
            self.view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(&NSString::from_str(script), None, &world, Some(&block));
        }
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        LIVE.with(|live| live.set(live.get().saturating_sub(1)));
        unregister(&self.view);
        self.ring.removeFromSuperview();
        self.view.removeFromSuperview();
    }
}

fn register(view: &Retained<WKWebView>, parent: &Retained<NSView>) {
    HOSTED.with(|hosted| {
        let mut hosted = hosted.borrow_mut();
        hosted.retain(|one| Retained::as_ptr(&one.view) != Retained::as_ptr(view));
        hosted.push(Hosted { view: view.clone(), parent: parent.clone() });
    });
    install_monitor();
}

fn unregister(view: &Retained<WKWebView>) {
    HOSTED.with(|hosted| hosted.borrow_mut().retain(|one| Retained::as_ptr(&one.view) != Retained::as_ptr(view)));
}

/// The page whose view holds the keys, if one does, with the gpui view it is a subview of.
fn page_with_keys(window: &NSWindow) -> Option<(Retained<WKWebView>, Retained<NSView>)> {
    let responder = window.firstResponder()?;
    let focused: &NSResponder = &responder;
    HOSTED.with(|hosted| {
        hosted
            .borrow()
            .iter()
            .find(|one| {
                let page: &NSResponder = &one.view;
                std::ptr::eq(page, focused) || focused.downcast_ref::<NSView>().is_some_and(|view| view.isDescendantOf(&one.view))
            })
            .map(|one| (one.view.clone(), one.parent.clone()))
    })
}

/// Watches the window's presses. **A press in the page gives it the keys** (WebKit takes it itself); a press
/// anywhere else in the window gives them back to gpui; and a key goes back to Bridge exactly where
/// `keys::is_bridges` says, as Classic's `pane-keys.ts` hands it back. Every other key is the page's: it is
/// delivered to the page here, ahead of gpui, which would otherwise read it by a binding of its own.
fn install_monitor() {
    MONITOR.with(|monitor| {
        if monitor.borrow().is_some() {
            return;
        }
        let block = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit passes a live event for the length of this call.
            let event_ref = unsafe { event.as_ref() };
            let Some(mtm) = MainThreadMarker::new() else { return event.as_ptr() };
            let Some(window) = event_ref.window(mtm) else { return event.as_ptr() };
            let Some((page, parent)) = page_with_keys(&window) else {
                return event.as_ptr();
            };
            match event_ref.r#type() {
                NSEventType::KeyDown => {
                    let flags = event_ref.modifierFlags();
                    let press = Press {
                        code: event_ref.keyCode(),
                        command: flags.contains(objc2_app_kit::NSEventModifierFlags::Command),
                        option: flags.contains(objc2_app_kit::NSEventModifierFlags::Option),
                        control: flags.contains(objc2_app_kit::NSEventModifierFlags::Control),
                        shift: flags.contains(objc2_app_kit::NSEventModifierFlags::Shift),
                    };
                    if keys::is_bridges(press) {
                        // Bridge's own key: the keys go back to gpui, which reads the press by its bindings.
                        window.makeFirstResponder(Some(&parent));
                        return event.as_ptr();
                    }
                    // The page's key: delivered to the page and not offered to gpui. A command key is a key
                    // equivalent first (copy, paste, select all), and a key the page does not take as one is typed.
                    // SAFETY: the page is a live NSView that is first responder, and the event is live.
                    unsafe {
                        let taken: bool = msg_send![&*page, performKeyEquivalent: event_ref];
                        if !taken {
                            let _: () = msg_send![&*page, keyDown: event_ref];
                        }
                    }
                    std::ptr::null_mut()
                }
                NSEventType::LeftMouseDown | NSEventType::RightMouseDown => {
                    // A press outside the page gives the keys back to gpui.
                    let Some(content) = window.contentView() else { return event.as_ptr() };
                    // SAFETY: reads the content view's superview, on the main thread.
                    let Some(superview) = (unsafe { content.superview() }) else { return event.as_ptr() };
                    let at = superview.convertPoint_fromView(event_ref.locationInWindow(), None);
                    let inside = content.hitTest(at).is_some_and(|hit| hit.isDescendantOf(&page) || std::ptr::addr_eq(&*hit, &*page));
                    if !inside {
                        window.makeFirstResponder(Some(&parent));
                    }
                    event.as_ptr()
                }
                _ => event.as_ptr(),
            }
        });
        let mask = NSEventMask::KeyDown | NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown;
        // SAFETY: the handler returns the event it was given, or null where it took the event; AppKit copies the block.
        let token = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) };
        *monitor.borrow_mut() = token;
    });
}
