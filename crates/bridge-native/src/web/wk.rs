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
    NSDate, NSDictionary, NSError, NSJSONSerialization, NSJSONWritingOptions, NSKeyValueObservingOptions, NSObjectNSKeyValueObserverRegistration, NSNumber, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRunLoop,
    NSSize, NSString, NSURL, NSURLRequest, NSUUID,
};
use objc2_web_kit::{
    WKContentWorld, WKFrameInfo, WKMediaCaptureType, WKNavigation, WKNavigationAction, WKNavigationActionPolicy,
    WKNavigationDelegate, WKNavigationType, WKPermissionDecision, WKScriptMessage, WKScriptMessageHandler, WKScriptMessageHandlerWithReply, WKSecurityOrigin, WKSnapshotConfiguration, WKUIDelegate, WKUserContentController, WKUserScript, WKUserScriptInjectionTime, WKWebView, WKWebViewConfiguration,
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
    /// Every gpui view a page has sat in, kept after the page goes: the view the keys go back to.
    static PARENTS: RefCell<Vec<Retained<NSView>>> = const { RefCell::new(Vec::new()) };
}

struct Hosted {
    view: Retained<WKWebView>,
    parent: Retained<NSView>,
    /// A sealed page the kit draws on: ⌘↵, its Send, stays with the page.
    kit: bool,
}

/// The name the kit's script posts to: `window.webkit.messageHandlers.armada`.
const KIT_HANDLER: &str = "armada";

/// What a page tells whoever holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nav {
    Finished,
    Failed(String),
    /// A load a redirect or the next load replaced (NSURLErrorCancelled): not an end, since the load that replaced it ends.
    Cancelled,
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
    /// Whether the view paints no ground of its own, so what the page leaves transparent stays so in its
    /// picture: a sketch's mock sits on the canvas, as Classic's frame does. A live page keeps its ground.
    pub clear: bool,
    /// Where the kit's posts go, as JSON text: a sealed page's `window.webkit.messageHandlers.armada`. `None`: the page
    /// has no such handler.
    pub kit: Option<async_channel::Sender<String>>,
}

/// A reply WebKit is owed once. **Let go unanswered, it answers `fallback`**: WebKit raises where a handler it handed out is dropped
/// uncalled, and a tab that closes with a page's ask waiting is such a drop.
pub struct Reply<T: 'static> {
    call: Option<Box<dyn FnOnce(T)>>,
    fallback: Option<T>,
}

impl<T: 'static> Reply<T> {
    fn new(fallback: T, call: impl FnOnce(T) + 'static) -> Reply<T> {
        Reply { call: Some(Box::new(call)), fallback: Some(fallback) }
    }

    pub fn send(mut self, answer: T) {
        self.fallback = None;
        if let Some(call) = self.call.take() {
            call(answer);
        }
    }
}

impl<T: 'static> Drop for Reply<T> {
    fn drop(&mut self) {
        if let (Some(call), Some(fallback)) = (self.call.take(), self.fallback.take()) {
            call(fallback);
        }
    }
}

/// What a Job browser's own script says to Bridge (`armadaSite`), as it posts it.
#[derive(serde::Deserialize)]
struct Posted {
    kind: String,
}

/// What a page's script asks, by the kind it posted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// `navigator.geolocation`, which the page's own script asks about before it reaches the real one.
    Location,
}

fn said_of(text: &str) -> Option<Said> {
    let posted: Posted = ipc::decode("a page script's post", text.as_bytes()).ok()?;
    match posted.kind.as_str() {
        "location" => Some(Said::Location),
        _ => None,
    }
}

/// What a Job browser's page asks of the one that holds it, each to be answered once.
pub enum Request {
    /// The camera, the microphone or both, for `origin`, a web address (`webView:requestMediaCapturePermissionForOrigin:`).
    Media { origin: String, camera: bool, microphone: bool, reply: Reply<bool> },
    /// What a script of Bridge's own in the page asks, for the page whose address is `origin`. The reply is `true` where it is given.
    Site { origin: String, said: Said, reply: Reply<bool> },
}

/// Where a Job browser's page sends what it asks.
pub struct Site {
    pub requests: async_channel::Sender<Request>,
    /// Scripts run in the page's own world in every frame, at document start. Each posts to the handler named `SITE_HANDLER`.
    pub scripts: Vec<&'static str>,
}

/// The name Bridge's scripts in a Job browser's page post to: `window.webkit.messageHandlers.armadaSite`, which answers.
const SITE_HANDLER: &str = "armadaSite";

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
            let _ = self.ivars().events.try_send(failure(error));
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_first(&self, _: &WKWebView, _: Option<&WKNavigation>, error: &NSError) {
            pending_down();
            let _ = self.ivars().events.try_send(failure(error));
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

/// NSURLErrorCancelled, in NSURLErrorDomain.
const URL_CANCELLED: isize = -999;

fn failure(error: &NSError) -> Nav {
    if error.code() == URL_CANCELLED && error.domain().to_string() == "NSURLErrorDomain" {
        Nav::Cancelled
    } else {
        Nav::Failed(error.localizedDescription().to_string())
    }
}

impl Navigation {
    fn new(events: async_channel::Sender<Nav>, sealed: bool, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(NavState { events, sealed });
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `PageUi` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = async_channel::Sender<Request>]
    struct PageUi;

    unsafe impl NSObjectProtocol for PageUi {}

    unsafe impl WKUIDelegate for PageUi {
        #[unsafe(method(webView:requestMediaCapturePermissionForOrigin:initiatedByFrame:type:decisionHandler:))]
        fn request_media(&self, _: &WKWebView, origin: &WKSecurityOrigin, _: &WKFrameInfo, kind: WKMediaCaptureType, handler: &DynBlock<dyn Fn(WKPermissionDecision)>) {
            let handler = handler.copy();
            let reply = Reply::new(false, move |grant| handler.call((if grant { WKPermissionDecision::Grant } else { WKPermissionDecision::Deny },)));
            let camera = kind == WKMediaCaptureType::Camera || kind == WKMediaCaptureType::CameraAndMicrophone;
            let microphone = kind == WKMediaCaptureType::Microphone || kind == WKMediaCaptureType::CameraAndMicrophone;
            // Sent nowhere (no one hears it), the reply is let go, and denies.
            let _ = self.ivars().try_send(Request::Media { origin: origin_url(origin), camera, microphone, reply });
        }
    }
);

impl PageUi {
    fn new(requests: async_channel::Sender<Request>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(requests);
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// A security origin as a web address: `https://example.com:8443/`.
fn origin_url(origin: &WKSecurityOrigin) -> String {
    // SAFETY: reads of the live origin WebKit passed this call, on the main thread.
    let (scheme, host, port) = unsafe { (origin.protocol().to_string(), origin.host().to_string(), origin.port()) };
    if port > 0 {
        format!("{scheme}://{host}:{port}/")
    } else {
        format!("{scheme}://{host}/")
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `SiteMessages` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = async_channel::Sender<Request>]
    struct SiteMessages;

    unsafe impl NSObjectProtocol for SiteMessages {}

    unsafe impl WKScriptMessageHandlerWithReply for SiteMessages {
        #[unsafe(method(userContentController:didReceiveScriptMessage:replyHandler:))]
        fn did_receive(&self, _: &WKUserContentController, message: &WKScriptMessage, handler: &DynBlock<dyn Fn(*mut AnyObject, *mut NSString)>) {
            let handler = handler.copy();
            // A refusal is the answer a page gets from a reply let go: the promise is fulfilled with `false`.
            let reply = Reply::new(false, move |given| {
                let answer = NSNumber::new_bool(given);
                handler.call((Retained::as_ptr(&answer).cast::<AnyObject>().cast_mut(), std::ptr::null_mut()));
            });
            // SAFETY: `message` is the live message WebKit passed this call, read on the main thread; its body is a property-list object.
            let (text, origin) = unsafe {
                let body = message.body();
                let text = NSJSONSerialization::isValidJSONObject(&body)
                    .then(|| NSJSONSerialization::dataWithJSONObject_options_error(&body, NSJSONWritingOptions::empty()).ok())
                    .flatten()
                    .and_then(|data| String::from_utf8(data.to_vec()).ok());
                (text, origin_url(&message.frameInfo().securityOrigin()))
            };
            // The page's host is the frame's own, never what the script says.
            let Some(said) = text.as_deref().and_then(said_of) else { return };
            let _ = self.ivars().try_send(Request::Site { origin, said, reply });
        }
    }
);

impl SiteMessages {
    fn new(requests: async_channel::Sender<Request>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(requests);
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `KitPosts` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = async_channel::Sender<String>]
    struct KitPosts;

    unsafe impl NSObjectProtocol for KitPosts {}

    unsafe impl WKScriptMessageHandler for KitPosts {
        #[unsafe(method(userContentController:didReceiveScriptMessage:))]
        fn did_receive(&self, _: &WKUserContentController, message: &WKScriptMessage) {
            // The kit posts a plain object; it crosses as JSON text, and anything that is not JSON is dropped here.
            // SAFETY: `message` is the live message WebKit passed this call, and its body is a property-list object.
            let text = unsafe {
                let body = message.body();
                NSJSONSerialization::isValidJSONObject(&body)
                    .then(|| NSJSONSerialization::dataWithJSONObject_options_error(&body, NSJSONWritingOptions::empty()).ok())
                    .flatten()
                    .and_then(|data| String::from_utf8(data.to_vec()).ok())
            };
            if let Some(text) = text {
                let _ = self.ivars().try_send(text);
            }
        }
    }
);

impl KitPosts {
    fn new(posts: async_channel::Sender<String>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(posts);
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Watching` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = async_channel::Sender<()>]
    struct Watching;

    unsafe impl NSObjectProtocol for Watching {}

    impl Watching {
        // Key-value observing: the page's address, title or history moved, including by a navigation inside the page.
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe(&self, _key: Option<&NSString>, _object: Option<&AnyObject>, _change: Option<&NSDictionary<NSString, AnyObject>>, _context: *mut std::ffi::c_void) {
            let _ = self.ivars().try_send(());
        }
    }
);

impl Watching {
    fn new(told: async_channel::Sender<()>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(told);
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// What a page's key-value observer watches: its address, title and whether it can go back or forward.
const WATCHED: [&str; 4] = ["URL", "title", "canGoBack", "canGoForward"];

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
    /// The kit's handler, which the page's content controller holds until `Drop` takes it off.
    kit: Option<Retained<KitPosts>>,
    /// A Job browser's page's own say on what it asks for, which its view holds only weakly.
    _ui: Option<Retained<PageUi>>,
    /// Bridge's own scripts' handler, which the page's content controller holds until `Drop` takes it off.
    site: Option<Retained<SiteMessages>>,
    /// The observer of the address, title and history, once `watch` has been asked: `Drop` takes it off.
    watching: RefCell<Option<Retained<Watching>>>,
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
        Native::with_site(options, size, events, None)
    }

    /// A Job browser's page: `new`, with the page's asks sent to `site`.
    pub fn with_site(options: &Options, size: Frame, events: async_channel::Sender<Nav>, site: Option<Site>) -> Option<Native> {
        let mtm = MainThreadMarker::new()?;
        // SAFETY: every call below is on the main thread, on objects made here, with arguments of the types
        // WebKit documents; the data store is a persistent one WebKit keeps by its identifier.
        let (view, navigation, kit, ui, site) = unsafe {
            let config = WKWebViewConfiguration::new(mtm);
            // The kit's handler is in the page's own world, where the kit's script runs.
            let kit = options.kit.clone().map(|posts| {
                let kit = KitPosts::new(posts, mtm);
                config.userContentController().addScriptMessageHandler_name(ProtocolObject::from_ref(&*kit), &NSString::from_str(KIT_HANDLER));
                kit
            });
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
            let messages = site.as_ref().map(|site| {
                let controller = config.userContentController();
                let messages = SiteMessages::new(site.requests.clone(), mtm);
                controller.addScriptMessageHandlerWithReply_contentWorld_name(ProtocolObject::from_ref(&*messages), &WKContentWorld::pageWorld(mtm), &NSString::from_str(SITE_HANDLER));
                for source in &site.scripts {
                    let script = WKUserScript::initWithSource_injectionTime_forMainFrameOnly(mtm.alloc(), &NSString::from_str(source), WKUserScriptInjectionTime::AtDocumentStart, false);
                    controller.addUserScript(&script);
                }
                messages
            });
            let view = WKWebView::initWithFrame_configuration(mtm.alloc(), rect(size), &config);
            let navigation = Navigation::new(events, options.sealed, mtm);
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*navigation)));
            let ui = site.map(|site| PageUi::new(site.requests, mtm));
            if let Some(ui) = &ui {
                view.setUIDelegate(Some(ProtocolObject::from_ref(&**ui)));
            }
            (view, navigation, kit, ui, messages)
        };
        if options.clear {
            // SAFETY: key-value coding on the live view, on the main thread; `drawsBackground` is a boolean key
            // WKWebView answers on macOS, and an `NSNumber` is the object KVC takes for a boolean.
            let _: () = unsafe { msg_send![&*view, setValue: &*NSNumber::new_bool(false), forKey: &*NSString::from_str("drawsBackground")] };
        }
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
        Some(Native { view, ring, _navigation: navigation, kit, _ui: ui, site, watching: RefCell::new(None), mtm })
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
            register(&self.view, &parent.0, self.kit.is_some());
        }
        self.view.setFrame(rect(frame));
        self.view.setHidden(false);
        let _ = self.mtm;
    }

    /// The keys, if the page holds them, go back to the gpui view it sits in (`give_keys_back`).
    pub fn release_keys(&self) {
        give_keys_back(&self.view);
    }

    /// The page hidden, keeping its place, scroll and state, as Classic parks a walk's view.
    pub fn park(&self) {
        give_keys_back(&self.view);
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

    /// Runs `script` in the page's own world, where the kit's `window.armadaHost` is, and says when it has run. A script
    /// handed to WebKit is not the page's, so the page's policy does not refuse it.
    pub fn tell(&self, script: &str, done: impl FnOnce() + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |_: *mut AnyObject, _: *mut NSError| {
            pending_down();
            if let Some(done) = done.borrow_mut().take() {
                done();
            }
        });
        // SAFETY: a script string and the page's world, evaluated on the main thread; the block is copied.
        unsafe {
            let world = WKContentWorld::pageWorld(self.mtm);
            self.view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(&NSString::from_str(script), None, &world, Some(&block));
        }
    }

    // ---- the Job browser's page --------------------------------------------------------------------------------

    /// The page's address as WebKit holds it, or none while it has none.
    pub fn url(&self) -> Option<String> {
        // SAFETY: a property read on the main thread.
        unsafe { self.view.URL() }.and_then(|url| url.absoluteString()).map(|text| text.to_string())
    }

    pub fn title(&self) -> String {
        // SAFETY: a property read on the main thread.
        unsafe { self.view.title() }.map(|title| title.to_string()).unwrap_or_default()
    }

    pub fn can_go_back(&self) -> bool {
        // SAFETY: a property read on the main thread.
        unsafe { self.view.canGoBack() }
    }

    pub fn can_go_forward(&self) -> bool {
        // SAFETY: a property read on the main thread.
        unsafe { self.view.canGoForward() }
    }

    /// One step back in the page's history, where it has one.
    pub fn go_back(&self) {
        // SAFETY: a navigation on the main thread; none is made where there is no item to go to.
        if unsafe { self.view.goBack() }.is_some() {
            pending_up();
        }
    }

    pub fn go_forward(&self) {
        // SAFETY: a navigation on the main thread; none is made where there is no item to go to.
        if unsafe { self.view.goForward() }.is_some() {
            pending_up();
        }
    }

    /// The view's size in points.
    pub fn size(&self) -> (f64, f64) {
        let size = self.view.frame().size;
        (size.width, size.height)
    }

    /// Tells `told` each time the page's address, title or history moves, a navigation inside the page too. Asking twice
    /// replaces the first.
    pub fn watch(&self, told: async_channel::Sender<()>) {
        self.unwatch();
        let watcher = Watching::new(told, self.mtm);
        // SAFETY: key-value observing on the live view with an observer of Bridge's own, on the main thread; `Drop` takes it off.
        unsafe {
            for key in WATCHED {
                self.view.addObserver_forKeyPath_options_context(&watcher, &NSString::from_str(key), NSKeyValueObservingOptions::New, std::ptr::null_mut());
            }
        }
        *self.watching.borrow_mut() = Some(watcher);
    }

    fn unwatch(&self) {
        if let Some(watcher) = self.watching.borrow_mut().take() {
            // SAFETY: the observer added by `watch`, taken off the same view for the same keys, on the main thread.
            unsafe {
                for key in WATCHED {
                    self.view.removeObserver_forKeyPath(&watcher, &NSString::from_str(key));
                }
            }
        }
    }

    /// Runs `script` in the content world named `world`, which shares the page's DOM and nothing else, and answers its string
    /// result (`None`: it threw or answered no string).
    pub fn evaluate_in(&self, world: &str, script: &str, done: impl FnOnce(Option<String>) + 'static) {
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
            let world = WKContentWorld::worldWithName(&NSString::from_str(world), self.mtm);
            self.view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(&NSString::from_str(script), None, &world, Some(&block));
        }
    }

    /// Runs `body` as the body of an async function in the content world named `world`, and answers what it returns once any
    /// promise has settled: its string, or `None` where it returned none. A throw answers `Err` with the page's words for it.
    /// (`callAsyncJavaScript` awaits, which `evaluateJavaScript` does not.)
    pub fn call_async(&self, world: &str, body: &str, done: impl FnOnce(Result<Option<String>, String>) + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
            pending_down();
            let answer = if !error.is_null() {
                // SAFETY: WebKit passes a live error for the length of this call.
                let error = unsafe { &*error };
                Err(thrown(error))
            } else if result.is_null() {
                Ok(None)
            } else {
                // SAFETY: WebKit passes a live object for the length of this call.
                let object = unsafe { &*result };
                Ok(object.downcast_ref::<NSString>().map(|text| text.to_string()))
            };
            if let Some(done) = done.borrow_mut().take() {
                done(answer);
            }
        });
        // SAFETY: a function body and no arguments, in a named content world, on the main thread; the block is copied.
        unsafe {
            let world = WKContentWorld::worldWithName(&NSString::from_str(world), self.mtm);
            let none = NSDictionary::<NSString, AnyObject>::new();
            self.view.callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(&NSString::from_str(body), Some(&none), None, &world, Some(&block));
        }
    }
}

/// A script's throw in words: the page's own message where WebKit kept one, else WebKit's.
fn thrown(error: &NSError) -> String {
    let own = error.userInfo().objectForKey(&NSString::from_str("WKJavaScriptExceptionMessage")).and_then(|said| said.downcast_ref::<NSString>().map(|text| text.to_string()));
    own.unwrap_or_else(|| error.localizedDescription().to_string())
}

impl Drop for Native {
    fn drop(&mut self) {
        LIVE.with(|live| live.set(live.get().saturating_sub(1)));
        self.unwatch();
        if self.kit.take().is_some() {
            // SAFETY: the page's own content controller, on the main thread; the handler it holds is let go.
            unsafe {
                self.view.configuration().userContentController().removeScriptMessageHandlerForName(&NSString::from_str(KIT_HANDLER));
            }
        }
        if self.site.take().is_some() {
            // SAFETY: the page's own content controller, on the main thread; the handler and scripts it holds are let go.
            unsafe {
                let controller = self.view.configuration().userContentController();
                controller.removeScriptMessageHandlerForName_contentWorld(&NSString::from_str(SITE_HANDLER), &WKContentWorld::pageWorld(self.mtm));
                controller.removeAllUserScripts();
            }
        }
        give_keys_back(&self.view);
        unregister(&self.view);
        self.ring.removeFromSuperview();
        self.view.removeFromSuperview();
    }
}

fn register(view: &Retained<WKWebView>, parent: &Retained<NSView>, kit: bool) {
    HOSTED.with(|hosted| {
        let mut hosted = hosted.borrow_mut();
        hosted.retain(|one| Retained::as_ptr(&one.view) != Retained::as_ptr(view));
        hosted.push(Hosted { view: view.clone(), parent: parent.clone(), kit });
    });
    PARENTS.with(|parents| {
        let mut parents = parents.borrow_mut();
        if !parents.iter().any(|one| Retained::as_ptr(one) == Retained::as_ptr(parent)) {
            parents.push(parent.clone());
        }
    });
    install_monitor();
}

/// A page about to be hidden or let go that holds the keys gives them to the gpui view it sits in. AppKit
/// would otherwise leave them with the window itself, where no view hears a key and every press beeps, and
/// gpui's view never takes them back on a click.
fn give_keys_back(view: &Retained<WKWebView>) {
    let Some(window) = view.window() else { return };
    let Some(responder) = window.firstResponder() else { return };
    let focused: &NSResponder = &responder;
    let page: &NSResponder = view;
    let held = std::ptr::eq(page, focused) || focused.downcast_ref::<NSView>().is_some_and(|one| one.isDescendantOf(view));
    // SAFETY: reads the view's superview, on the main thread.
    if let Some(parent) = held.then(|| unsafe { view.superview() }).flatten() {
        window.makeFirstResponder(Some(&parent));
    }
}

/// Where the window itself has the keys, which a page let go of the wrong way leaves, the gpui view a page
/// sat in takes them back before the event is handed on.
fn heal_keys(window: &NSWindow) {
    let held_by_window = window.firstResponder().is_none_or(|responder| {
        let focused: &NSResponder = &responder;
        let own: &NSResponder = window;
        std::ptr::eq(focused, own)
    });
    if !held_by_window {
        return;
    }
    let parent = PARENTS.with(|parents| {
        parents.borrow().iter().find(|one| one.window().is_some_and(|theirs| std::ptr::eq(&*theirs, window))).cloned()
    });
    if let Some(parent) = parent {
        window.makeFirstResponder(Some(&parent));
    }
}

fn unregister(view: &Retained<WKWebView>) {
    HOSTED.with(|hosted| hosted.borrow_mut().retain(|one| Retained::as_ptr(&one.view) != Retained::as_ptr(view)));
}

/// The page whose view holds the keys, if one does, with the gpui view it is a subview of.
fn page_with_keys(window: &NSWindow) -> Option<(Retained<WKWebView>, Retained<NSView>, bool)> {
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
            .map(|one| (one.view.clone(), one.parent.clone(), one.kit))
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
            let Some((page, parent, kit)) = page_with_keys(&window) else {
                heal_keys(&window);
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
                    if keys::is_bridges(press) && !(kit && keys::is_kits(press)) {
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
