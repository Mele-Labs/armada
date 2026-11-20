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
use std::rc::Rc;

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, ProtocolObject};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAppearance, NSAppearanceCustomization, NSBackingStoreType, NSWindowStyleMask, NSAppearanceNameDarkAqua, NSBezierPath, NSBitmapImageFileType, NSBitmapImageRep, NSColor, NSEvent, NSEventMask,
    NSEventType, NSImage, NSResponder, NSView, NSWindow,
};
use objc2_foundation::{
    NSDate, NSDictionary, NSError, NSJSONSerialization, NSJSONWritingOptions, NSKeyValueObservingOptions, NSObjectNSKeyValueObserverRegistration, NSNumber, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRunLoop,
    NSSize, NSString, NSHTTPURLResponse, NSURL, NSURLRequest, NSURLResponse, NSUUID,
};
use objc2_web_kit::{
    WKContentRuleList, WKContentRuleListStore, WKContentWorld, WKDownload, WKDownloadDelegate, WKFrameInfo, WKNavigationResponse, WKNavigationResponsePolicy, WKWindowFeatures, WKMediaCaptureType, WKNavigation, WKNavigationAction, WKNavigationActionPolicy,
    WKNavigationDelegate, WKNavigationType, WKPermissionDecision, WKScriptMessage, WKScriptMessageHandler, WKScriptMessageHandlerWithReply, WKSecurityOrigin, WKSnapshotConfiguration, WKUIDelegate, WKUserContentController, WKUserScript, WKUserScriptInjectionTime, WKWebView, WKWebViewConfiguration,
    WKWebpagePreferences, WKWebsiteDataStore,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::acts::{self, DownloadWay, PopupWay, Way};
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

/// The content world of Bridge's own the kit runs in: it shares the page's DOM and nothing else, so no script of the
/// page's reaches the kit's handler, its `armadaHost` or its `armadaHostPress`.
pub const KIT_WORLD: &str = "armada-kit";

/// The kit's script, from the one file Fleet serves to Classic and Pocket inline.
const KIT_SCRIPT: &str = include_str!("../../../fleet/src/session_host/page_kit.js");

/// Run in the page's own world before any script of its: WebRTC reaches the network, and the page's policy does not govern it.
const NO_PEERS: &str = r#"(() => {
  for (const name of ["RTCPeerConnection", "webkitRTCPeerConnection", "RTCDataChannel", "RTCSessionDescription", "RTCIceCandidate"]) {
    try { Object.defineProperty(window, name, { value: undefined, writable: false, enumerable: false, configurable: false }); } catch (_) {}
  }
})();"#;

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
    /// Where the kit's posts go, as JSON text: a sealed page's `window.webkit.messageHandlers.armada` in `KIT_WORLD`, where
    /// Bridge runs the kit's script, since Fleet serves the page without it (`host_kit`). `None`: no kit and no handler.
    pub kit: Option<async_channel::Sender<String>>,
}

/// A reply WebKit is owed once. **Let go unanswered, it answers `fallback`**: WebKit raises where a handler it handed out is dropped
/// uncalled, and a tab that closes with a page's ask waiting is such a drop.
pub struct Reply<T: 'static> {
    call: Option<Box<dyn FnOnce(T)>>,
    fallback: Option<T>,
}

impl<T: 'static> Reply<T> {
    pub fn new(fallback: T, call: impl FnOnce(T) + 'static) -> Reply<T> {
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
    #[serde(default)]
    target: String,
}

/// What a page's script asks, by the kind it posted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// `navigator.geolocation`, which the page's own script asks about before it reaches the real one.
    Location,
    /// `fetch`, `XMLHttpRequest`, `WebSocket` or `EventSource` reached `target`, a host name or address that is plainly local.
    LocalNetwork { target: String },
    /// The page starts, or takes the focus again, and asks whether fullscreen is blocked for it. Answered `false` where it is.
    Fullscreen,
}

fn said_of(text: &str) -> Option<Said> {
    let posted: Posted = ipc::decode("a page script's post", text.as_bytes()).ok()?;
    match posted.kind.as_str() {
        "location" => Some(Said::Location),
        "local_network" => Some(Said::LocalNetwork { target: posted.target }),
        "fullscreen" => Some(Said::Fullscreen),
        _ => None,
    }
}

/// What a Job browser's page asks of the one that holds it, each to be answered once.
pub enum Request {
    /// The camera, the microphone or both, for `origin`, a web address (`webView:requestMediaCapturePermissionForOrigin:`).
    Media { origin: String, camera: bool, microphone: bool, reply: Reply<bool> },
    /// What a script of Bridge's own in the page asks, for the page whose address is `origin`. The reply is `true` where it is given.
    Site { origin: String, said: Said, reply: Reply<bool> },
    /// The page is made, and holds its first load until it is given its content rule list (`Native::set_rules`).
    Made,
    /// A popup or a download the owner's choice is Ask for, which WebKit was told no to meanwhile: carried out by `Native::carry_out`
    /// once he allows it.
    Act { act: Act, address: String },
}

/// What a page asks to do that is held back for the owner's say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Popup,
    Download,
}

/// What a Job browser's page may do, by the owner's choices for the host it shows. The tab that holds the page keeps it current
/// (`Native::policy`), since WebKit asks for a popup and a download in the same turn and cannot wait on a read of the file.
#[derive(Clone, Debug, Default)]
pub struct Policy {
    pub host: Option<String>,
    pub popups: Way,
    pub downloads: Way,
}

/// The policy as the page and its popups share it.
pub type SharedPolicy = Rc<RefCell<Policy>>;

/// Where a Job browser's page sends what it asks, and the choices it is held to.
#[derive(Clone)]
struct Reach {
    policy: SharedPolicy,
    requests: async_channel::Sender<Request>,
}

impl Reach {
    fn ask(&self, act: Act, address: String) {
        let _ = self.requests.try_send(Request::Act { act, address });
    }
}

/// Where a Job browser's page sends what it asks.
pub struct Site {
    pub requests: async_channel::Sender<Request>,
    pub policy: SharedPolicy,
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
    /// A Job browser's page, held to the owner's choices: it goes only to the web, and downloads as he chose.
    reach: Option<Reach>,
    /// A popup's: it counts for no load the page's holder waits on.
    quiet: bool,
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
            if !self.ivars().quiet {
                pending_down();
            }
            let _ = self.ivars().events.try_send(Nav::Finished);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn did_fail(&self, _: &WKWebView, _: Option<&WKNavigation>, error: &NSError) {
            if !self.ivars().quiet {
                pending_down();
            }
            let _ = self.ivars().events.try_send(failure(error));
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn did_fail_first(&self, _: &WKWebView, _: Option<&WKNavigation>, error: &NSError) {
            if !self.ivars().quiet {
                pending_down();
            }
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
            if let Some(reach) = &self.ivars().reach {
                // SAFETY: the live action WebKit passed this call, read on the main thread.
                let (download, main) = unsafe { (action.shouldPerformDownload(), action.targetFrame().is_none_or(|frame| frame.isMainFrame())) };
                let url = unsafe { action.request().URL().and_then(|url| url.absoluteString()).map(|text| text.to_string()).unwrap_or_default() };
                let policy = reach.policy.borrow().clone();
                let answer = if download {
                    match acts::download_way(policy.host.as_deref(), policy.downloads) {
                        DownloadWay::Take => WKNavigationActionPolicy::Download,
                        DownloadWay::Ask => {
                            reach.ask(Act::Download, url);
                            WKNavigationActionPolicy::Cancel
                        }
                        DownloadWay::Refuse => WKNavigationActionPolicy::Cancel,
                    }
                } else if main && !(acts::only_web(&url) || acts::is_blank(&url)) {
                    // Only the web is loaded: a file, a script address or another app's scheme is refused (`will-navigate`).
                    WKNavigationActionPolicy::Cancel
                } else {
                    WKNavigationActionPolicy::Allow
                };
                handler.call((answer,));
                return;
            }
            let allowed = !self.ivars().sealed || (kind == WKNavigationType::Other && is_blank);
            handler.call((if allowed { WKNavigationActionPolicy::Allow } else { WKNavigationActionPolicy::Cancel },));
        }

        #[unsafe(method(webView:decidePolicyForNavigationResponse:decisionHandler:))]
        fn decide_response(&self, _: &WKWebView, response: &WKNavigationResponse, handler: &DynBlock<dyn Fn(WKNavigationResponsePolicy)>) {
            // SAFETY: the live response WebKit passed this call, read on the main thread.
            let (main, can_show, disposition, url) = unsafe {
                let raw = response.response();
                let disposition = raw.downcast_ref::<NSHTTPURLResponse>().and_then(|http| http.valueForHTTPHeaderField(&NSString::from_str("Content-Disposition"))).map(|text| text.to_string());
                let url = raw.URL().and_then(|url| url.absoluteString()).map(|text| text.to_string()).unwrap_or_default();
                (response.isForMainFrame(), response.canShowMIMEType(), disposition, url)
            };
            let shown = if can_show { WKNavigationResponsePolicy::Allow } else { WKNavigationResponsePolicy::Cancel };
            let answer = match &self.ivars().reach {
                // A file the page cannot show, or the server said to save, is a download, as the owner's choice says.
                Some(reach) if main && acts::is_download(can_show, disposition.as_deref()) => {
                    let policy = reach.policy.borrow().clone();
                    match acts::download_way(policy.host.as_deref(), policy.downloads) {
                        DownloadWay::Take => WKNavigationResponsePolicy::Download,
                        DownloadWay::Ask => {
                            reach.ask(Act::Download, url);
                            WKNavigationResponsePolicy::Cancel
                        }
                        DownloadWay::Refuse => WKNavigationResponsePolicy::Cancel,
                    }
                }
                _ => shown,
            };
            handler.call((answer,));
        }

        #[unsafe(method(webView:navigationAction:didBecomeDownload:))]
        fn action_became_download(&self, _: &WKWebView, _: &WKNavigationAction, download: &WKDownload) {
            into_downloads(download, self.mtm());
        }

        #[unsafe(method(webView:navigationResponse:didBecomeDownload:))]
        fn response_became_download(&self, _: &WKWebView, _: &WKNavigationResponse, download: &WKDownload) {
            into_downloads(download, self.mtm());
        }
    }
);

thread_local! {
    /// The one delegate every download is given: kept for the process, since a download outlives the page and the tab that began it.
    static DOWNLOADS: RefCell<Option<Retained<Downloads>>> = const { RefCell::new(None) };
}

/// A download given to the delegate that writes it into the Downloads folder.
fn into_downloads(download: &WKDownload, mtm: MainThreadMarker) {
    let sink = DOWNLOADS.with(|sink| sink.borrow_mut().get_or_insert_with(|| Downloads::new(mtm)).clone());
    // SAFETY: the live download WebKit made, on the main thread; its delegate is held weakly, and `DOWNLOADS` holds it.
    unsafe { download.setDelegate(Some(ProtocolObject::from_ref(&*sink))) };
}

/// Where a download is written: `~/Downloads`, under the name the server suggested, or under it with a number where one has that name.
fn downloads_path(suggested: &str) -> Option<std::path::PathBuf> {
    let folder = std::path::Path::new(&std::env::var_os("HOME")?).join("Downloads");
    std::fs::create_dir_all(&folder).ok()?;
    Some(acts::unique_path(&folder, suggested, |path| path.exists()))
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Downloads` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct Downloads;

    unsafe impl NSObjectProtocol for Downloads {}

    unsafe impl WKDownloadDelegate for Downloads {
        #[unsafe(method(download:decideDestinationUsingResponse:suggestedFilename:completionHandler:))]
        fn decide_destination(&self, _: &WKDownload, _: &NSURLResponse, suggested: &NSString, handler: &DynBlock<dyn Fn(*mut NSURL)>) {
            let Some(path) = downloads_path(&suggested.to_string()) else {
                // No place to write it: the download is cancelled.
                handler.call((std::ptr::null_mut(),));
                return;
            };
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            handler.call((Retained::as_ptr(&url).cast_mut(),));
        }
    }
);

impl Downloads {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

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
    fn new(events: async_channel::Sender<Nav>, sealed: bool, reach: Option<Reach>, quiet: bool, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(NavState { events, sealed, reach, quiet });
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// A popup the page opened: its window, the page in it and the delegates that page is held to.
struct Popup {
    window: Retained<NSWindow>,
    view: Retained<WKWebView>,
    _ui: Retained<PageUi>,
    _navigation: Retained<Navigation>,
}

type Popups = Rc<RefCell<Vec<Popup>>>;

/// What a Job browser's page and the popups it opens say to Bridge, and the popups it has open.
struct Ui {
    reach: Reach,
    popups: Popups,
}

/// A window of its own, in a plain NSWindow, for a page the opener made with `configuration`, so `window.opener` works. It goes with the
/// opener's tab (`Native`'s `Drop`). A window the owner closed is let go the next time one opens.
fn open_popup(ui: &Ui, configuration: &WKWebViewConfiguration, title: &str, mtm: MainThreadMarker) -> Retained<WKWebView> {
    let size = Frame::new(0., 0., POPUP_WIDTH, POPUP_HEIGHT);
    // SAFETY: every call is on the main thread, on objects made here, with arguments of the types AppKit and WebKit document; the view
    // is made with the configuration WebKit gave, as it requires.
    let (view, window) = unsafe {
        let view = WKWebView::initWithFrame_configuration(mtm.alloc(), rect(size), configuration);
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            rect(size),
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        );
        // The window is held by `Popup`, not by AppKit, so closing it does not free it under the page.
        window.setReleasedWhenClosed(false);
        (view, window)
    };
    window.setTitle(&NSString::from_str(title));
    window.setContentView(Some(&view));
    let (events, _) = async_channel::unbounded();
    let navigation = Navigation::new(events, false, Some(ui.reach.clone()), true, mtm);
    let page_ui = PageUi::new(Ui { reach: ui.reach.clone(), popups: ui.popups.clone() }, mtm);
    // SAFETY: the delegates are held by `Popup` for as long as the view is, on the main thread.
    unsafe {
        view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*navigation)));
        view.setUIDelegate(Some(ProtocolObject::from_ref(&*page_ui)));
    }
    window.center();
    window.makeKeyAndOrderFront(None);
    let mut popups = ui.popups.borrow_mut();
    popups.retain(|popup| popup.window.isVisible() || popup.window.isMiniaturized());
    popups.push(Popup { window, view: view.clone(), _ui: page_ui, _navigation: navigation });
    view
}

const POPUP_WIDTH: f64 = 800.;
const POPUP_HEIGHT: f64 = 600.;

/// A page's JavaScript dialog, as a sheet on the window the page is in, and `done` with whether its first button was pressed. A page in no
/// window has no place for one: it is answered as `acts::unplaced_answer` says.
fn show_dialog(view: &WKWebView, frame: &WKFrameInfo, message: &NSString, kind: acts::Dialog, done: impl Fn(bool) + 'static, mtm: MainThreadMarker) {
    let Some(window) = view.window() else {
        done(acts::unplaced_answer(kind).unwrap_or(false));
        return;
    };
    // SAFETY: the live frame WebKit passed this call, read on the main thread.
    let origin = unsafe { frame.securityOrigin() };
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(&format!("{} says", origin_host(&origin))));
    alert.setInformativeText(message);
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    if kind == acts::Dialog::Confirm {
        alert.addButtonWithTitle(&NSString::from_str("Cancel"));
    }
    let keep = alert.clone();
    let finished = RcBlock::new(move |response: isize| {
        let _alert = &keep;
        // NSAlertFirstButtonReturn.
        done(response == 1000);
    });
    alert.beginSheetModalForWindow_completionHandler(&window, Some(&finished));
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `PageUi` implements no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ui]
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
            let _ = self.ivars().reach.requests.try_send(Request::Media { origin: origin_url(origin), camera, microphone, reply });
        }

        #[unsafe(method_id(webView:createWebViewWithConfiguration:forNavigationAction:windowFeatures:))]
        fn create_web_view(&self, _: &WKWebView, configuration: &WKWebViewConfiguration, action: &WKNavigationAction, _: &WKWindowFeatures) -> Option<Retained<WKWebView>> {
            // SAFETY: the live action WebKit passed this call, read on the main thread.
            let wants = unsafe { action.request().URL().and_then(|url| url.absoluteString()).map(|text| text.to_string()).unwrap_or_default() };
            let policy = self.ivars().reach.policy.borrow().clone();
            match acts::popup_way(policy.host.as_deref(), &wants, policy.popups) {
                PopupWay::Refuse => None,
                PopupWay::Ask => {
                    // No window now: the owner is asked, and a window with the address opens after his Allow, without `window.opener`.
                    self.ivars().reach.ask(Act::Popup, wants);
                    None
                }
                PopupWay::Open => Some(open_popup(self.ivars(), configuration, policy.host.as_deref().unwrap_or_default(), self.mtm())),
            }
        }

        #[unsafe(method(webViewDidClose:))]
        fn did_close(&self, view: &WKWebView) {
            let gone: Vec<Popup> = {
                let mut popups = self.ivars().popups.borrow_mut();
                let (gone, kept) = popups.drain(..).partition(|popup| std::ptr::eq(&*popup.view, view));
                *popups = kept;
                gone
            };
            for popup in gone {
                popup.window.close();
            }
        }

        #[unsafe(method(webView:runJavaScriptAlertPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn alert(&self, view: &WKWebView, message: &NSString, frame: &WKFrameInfo, handler: &DynBlock<dyn Fn()>) {
            let handler = handler.copy();
            show_dialog(view, frame, message, acts::Dialog::Alert, move |_| handler.call(()), self.mtm());
        }

        #[unsafe(method(webView:runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:))]
        fn confirm(&self, view: &WKWebView, message: &NSString, frame: &WKFrameInfo, handler: &DynBlock<dyn Fn(Bool)>) {
            let handler = handler.copy();
            show_dialog(view, frame, message, acts::Dialog::Confirm, move |yes| handler.call((Bool::new(yes),)), self.mtm());
        }

        #[unsafe(method(webView:runJavaScriptTextInputPanelWithPrompt:defaultText:initiatedByFrame:completionHandler:))]
        fn prompt(&self, _: &WKWebView, _: &NSString, _: Option<&NSString>, _: &WKFrameInfo, handler: &DynBlock<dyn Fn(*mut NSString)>) {
            // `prompt` is not supported, as in Classic: it is answered with nothing, as Cancel does.
            handler.call((std::ptr::null_mut(),));
        }
    }
);

impl PageUi {
    fn new(ui: Ui, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(ui);
        // SAFETY: `NSObject`'s `init` takes no arguments and the ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// A security origin's host: `example.com`.
fn origin_host(origin: &WKSecurityOrigin) -> String {
    // SAFETY: a read of the live origin WebKit passed this call, on the main thread.
    unsafe { origin.host().to_string() }
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

/// What a Job browser's page waits on before its first load: its content rule list, so the first request is already checked.
struct Gate {
    open: Cell<bool>,
    /// The address asked for while the gate was shut.
    queued: RefCell<Option<String>>,
    /// The list in force, and the name WebKit's store keeps it by.
    current: RefCell<Option<(Retained<WKContentRuleList>, String)>>,
    /// Which compile is the latest: an older one that finishes late is let go.
    generation: Cell<u64>,
}

thread_local! {
    /// Numbers the pages, so each one's rule lists are named apart in WebKit's store.
    static SERIAL: Cell<u64> = const { Cell::new(0) };
}

/// A rule list, taken off the page's controller and out of WebKit's store.
fn drop_rules(controller: &WKUserContentController, old: (Retained<WKContentRuleList>, String), mtm: MainThreadMarker) {
    // SAFETY: the list this page's controller holds, removed on the main thread, and the name it was stored by.
    unsafe {
        controller.removeContentRuleList(&old.0);
        if let Some(store) = WKContentRuleListStore::defaultStore(mtm) {
            // WebKit calls the handler without asking whether there is one: a nil one crashed the app as a Browser tab closed.
            let done = RcBlock::new(|_: *mut NSError| {});
            store.removeContentRuleListForIdentifier_completionHandler(Some(&NSString::from_str(&old.1)), Some(&done));
        }
    }
}

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
    /// Where a Job browser's page waits for its rule list.
    gate: Option<std::rc::Rc<Gate>>,
    serial: u64,
    /// What the page asks, the choices it is held to, and the popups it has open, which close with it.
    ui: Option<Ui>,
    /// The observer of the address, title and history, once `watch` has been asked: `Drop` takes it off.
    watching: RefCell<Option<Retained<Watching>>>,
    /// The named content worlds Bridge's scripts run in, held for the page's life: WebKit clears a world nobody holds, and with it
    /// the finder an earlier script put there.
    worlds: RefCell<Vec<(String, Retained<WKContentWorld>)>>,
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
        let gate = site.is_some().then(|| std::rc::Rc::new(Gate { open: Cell::new(false), queued: RefCell::new(None), current: RefCell::new(None), generation: Cell::new(0) }));
        let serial = SERIAL.with(|serial| {
            serial.set(serial.get() + 1);
            serial.get()
        });
        let made = site.as_ref().map(|site| site.requests.clone());
        let ui = site.as_ref().map(|site| Ui { reach: Reach { policy: site.policy.clone(), requests: site.requests.clone() }, popups: Popups::default() });
        // SAFETY: every call below is on the main thread, on objects made here, with arguments of the types
        // WebKit documents; the data store is a persistent one WebKit keeps by its identifier.
        let (view, navigation, kit, ui_delegate, site) = unsafe {
            let config = WKWebViewConfiguration::new(mtm);
            // The kit and its handler are in a world of Bridge's own, and never the page's: there a page script could post a
            // press with no click of his. The page's world loses WebRTC before its own scripts run.
            let kit = options.kit.clone().map(|posts| {
                let controller = config.userContentController();
                let world = WKContentWorld::worldWithName(&NSString::from_str(KIT_WORLD), mtm);
                let kit = KitPosts::new(posts, mtm);
                controller.addScriptMessageHandler_contentWorld_name(ProtocolObject::from_ref(&*kit), &world, &NSString::from_str(KIT_HANDLER));
                let script = WKUserScript::initWithSource_injectionTime_forMainFrameOnly_inContentWorld(mtm.alloc(), &NSString::from_str(KIT_SCRIPT), WKUserScriptInjectionTime::AtDocumentEnd, true, &world);
                controller.addUserScript(&script);
                let peers = WKUserScript::initWithSource_injectionTime_forMainFrameOnly(mtm.alloc(), &NSString::from_str(NO_PEERS), WKUserScriptInjectionTime::AtDocumentStart, false);
                controller.addUserScript(&peers);
                (kit, world)
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
            if site.is_some() {
                // Element fullscreen is off in WKWebView until it is asked for. A site the owner blocked it for is refused in the
                // page (`fullscreen.js`), since the preference is copied into the view and cannot be changed after.
                config.preferences().setElementFullscreenEnabled(true);
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
            let navigation = Navigation::new(events, options.sealed, ui.as_ref().map(|ui| ui.reach.clone()), false, mtm);
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*navigation)));
            let ui_delegate = ui.as_ref().map(|ui| PageUi::new(Ui { reach: ui.reach.clone(), popups: ui.popups.clone() }, mtm));
            if let Some(delegate) = &ui_delegate {
                view.setUIDelegate(Some(ProtocolObject::from_ref(&**delegate)));
            }
            (view, navigation, kit, ui_delegate, messages)
        };
        let (kit, worlds) = match kit {
            Some((kit, world)) => (Some(kit), vec![(KIT_WORLD.to_string(), world)]),
            None => (None, Vec::new()),
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
        // The page's holder is told, to give it a rule list: its first load waits.
        if let Some(made) = made {
            let _ = made.try_send(Request::Made);
        }
        Some(Native { view, ring, _navigation: navigation, kit, _ui: ui_delegate, site, gate, serial, ui, watching: RefCell::new(None), worlds: RefCell::new(worlds), mtm })
    }

    pub fn load_url(&self, url: &str) {
        if let Some(gate) = self.gate.as_ref().filter(|gate| !gate.open.get()) {
            // A Job browser's first load waits for its rule list, so no request goes out unchecked.
            *gate.queued.borrow_mut() = Some(url.to_string());
            return;
        }
        let Some(url) = NSURL::URLWithString(&NSString::from_str(url)) else { return };
        pending_up();
        // SAFETY: a request for a URL made on the main thread.
        unsafe {
            let request = NSURLRequest::requestWithURL(&url);
            self.view.loadRequest(&request);
        }
    }

    /// A popup or a download the owner was asked about and allowed.
    pub fn carry_out(&self, act: Act, address: &str) {
        let (Some(ui), true) = (&self.ui, acts::only_web(address)) else { return };
        let Some(url) = NSURL::URLWithString(&NSString::from_str(address)) else { return };
        // SAFETY: calls on the main thread, on the live view and objects made here.
        unsafe {
            let request = NSURLRequest::requestWithURL(&url);
            match act {
                Act::Popup => {
                    let host = ui.reach.policy.borrow().host.clone().unwrap_or_default();
                    // The page's own configuration, so its scripts and rules apply; it cannot be the opener's `window.opener`.
                    open_popup(ui, &self.view.configuration(), &host, self.mtm).loadRequest(&request);
                }
                Act::Download => {
                    let mtm = self.mtm;
                    let began = RcBlock::new(move |download: NonNull<WKDownload>| into_downloads(download.as_ref(), mtm));
                    self.view.startDownloadUsingRequest_completionHandler(&request, &began);
                }
            }
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

    /// The keys go to the page, as a press in it gives them, so what he types reaches it: Edit puts the caret in its words.
    pub fn take_keys(&self) {
        if let Some(window) = self.view.window() {
            window.makeFirstResponder(Some(&self.view));
        }
    }

    /// The page's content rule list, from JSON: compiled by WebKit, and put in place of the one before it (`None`: no list). **The first
    /// answer opens the page's gate**, and the load it held goes out then. A compile that failed leaves the page without a list, and
    /// goes on: a page that cannot load is worse than one that is not guarded from the local network.
    pub fn set_rules(&self, json: Option<&str>) {
        let Some(gate) = self.gate.clone() else { return };
        let generation = gate.generation.get() + 1;
        gate.generation.set(generation);
        let (view, mtm, weak) = (self.view.clone(), self.mtm, std::rc::Rc::downgrade(&gate));
        let Some(json) = json else {
            apply_rules(&gate, &view, None, mtm);
            return;
        };
        let name = format!("armada-job-browser-{}-{generation}", self.serial);
        let named = name.clone();
        let block = RcBlock::new(move |list: *mut WKContentRuleList, error: *mut NSError| {
            let Some(gate) = weak.upgrade() else { return };
            // SAFETY: WebKit passes a live list, or none, for the length of this call; it is retained to keep it.
            let list = if list.is_null() { None } else { unsafe { Retained::retain(list) } };
            if gate.generation.get() != generation {
                // A newer list is on its way: this one is let go.
                if let Some(list) = list {
                    // SAFETY: the page's own content controller, on the main thread.
                    drop_rules(&*unsafe { view.configuration().userContentController() }, (list, named.clone()), mtm);
                }
                return;
            }
            if !error.is_null() {
                // SAFETY: WebKit passes a live error for the length of this call.
                let why = unsafe { &*error }.localizedDescription().to_string();
                crate::log::Note::warn("a Job browser's content rule list did not compile").field("why", why).emit();
            }
            apply_rules(&gate, &view, list.map(|list| (list, named.clone())), mtm);
        });
        // SAFETY: a name and a JSON text, compiled on the main thread; the block is copied by WebKit.
        unsafe {
            match WKContentRuleListStore::defaultStore(mtm) {
                Some(store) => store.compileContentRuleListForIdentifier_encodedContentRuleList_completionHandler(Some(&NSString::from_str(&name)), Some(&NSString::from_str(json)), Some(&block)),
                // No store to compile in: the page goes on without a list.
                None => apply_rules(&gate, &self.view, None, mtm),
            }
        }
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
                // AppKit autoreleases the TIFF and the CGImages it makes on the way to a PNG, about 128 MB for a tall page, and
                // nothing drained them where WebKit calls back: 22 such buffers were held after two hours. Our own pool frees them here.
                objc2::rc::autoreleasepool(|_| {
                    image
                        .TIFFRepresentation()
                        .and_then(|tiff| NSBitmapImageRep::imageRepWithData(&tiff))
                        // SAFETY: a PNG encoding of a bitmap, with no properties.
                        .and_then(|rep| unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) })
                        .map(|data| data.to_vec())
                })
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

    /// The whole page as one PDF, laid out at the view's width, or WebKit's reason it made none.
    pub fn pdf(&self, done: impl FnOnce(Result<Vec<u8>, String>) + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |data: *mut objc2_foundation::NSData, error: *mut NSError| {
            pending_down();
            let made = if data.is_null() {
                // SAFETY: WebKit passes a live error, or null, for the length of this call.
                let why = unsafe { error.as_ref() }.map(|error| error.localizedDescription().to_string());
                Err(why.unwrap_or_else(|| "WebKit made no PDF".into()))
            } else {
                // SAFETY: WebKit passes live data for the length of this call.
                Ok(unsafe { &*data }.to_vec())
            };
            if let Some(done) = done.borrow_mut().take() {
                done(made);
            }
        });
        // SAFETY: no configuration, so WebKit takes the page's whole bounds; on the main thread, and the block is copied.
        unsafe {
            self.view.createPDFWithConfiguration_completionHandler(None, &block);
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

    /// Runs `script` in the kit's world, where its `window.armadaHost` is, and says when it has run. A script handed to WebKit
    /// is not the page's, so the page's policy does not refuse it.
    pub fn tell(&self, script: &str, done: impl FnOnce() + 'static) {
        pending_up();
        let done = RefCell::new(Some(done));
        let block = RcBlock::new(move |_: *mut AnyObject, _: *mut NSError| {
            pending_down();
            if let Some(done) = done.borrow_mut().take() {
                done();
            }
        });
        let world = self.world(KIT_WORLD);
        // SAFETY: a script string and the kit's world, evaluated on the main thread; the block is copied.
        unsafe {
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

    /// The content world named `name`, the one this page holds.
    fn world(&self, name: &str) -> Retained<WKContentWorld> {
        let mut worlds = self.worlds.borrow_mut();
        if let Some((_, world)) = worlds.iter().find(|(held, _)| held == name) {
            return world.clone();
        }
        // SAFETY: a world named by Bridge, made on the main thread.
        let world = unsafe { WKContentWorld::worldWithName(&NSString::from_str(name), self.mtm) };
        worlds.push((name.to_string(), world.clone()));
        world
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
        let world = self.world(world);
        // SAFETY: a script string and a named content world, evaluated on the main thread; the block is copied.
        unsafe {
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
        let world = self.world(world);
        unsafe {
            let none = NSDictionary::<NSString, AnyObject>::new();
            self.view.callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(&NSString::from_str(body), Some(&none), None, &world, Some(&block));
        }
    }
}

/// A compiled list is put in place of the one before it, and the gate opens, with the load it held going out.
fn apply_rules(gate: &Gate, view: &Retained<WKWebView>, list: Option<(Retained<WKContentRuleList>, String)>, mtm: MainThreadMarker) {
    // SAFETY: the page's own content controller and view, on the main thread.
    unsafe {
        let controller = view.configuration().userContentController();
        if let Some(old) = gate.current.borrow_mut().take() {
            drop_rules(&controller, old, mtm);
        }
        if let Some((list, name)) = list {
            controller.addContentRuleList(&list);
            *gate.current.borrow_mut() = Some((list, name));
        }
        gate.open.set(true);
        if let Some(url) = gate.queued.borrow_mut().take().and_then(|url| NSURL::URLWithString(&NSString::from_str(&url))) {
            pending_up();
            view.loadRequest(&NSURLRequest::requestWithURL(&url));
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
            let world = self.world(KIT_WORLD);
            // SAFETY: the page's own content controller, on the main thread; the handler and the scripts it holds are let go.
            unsafe {
                let controller = self.view.configuration().userContentController();
                controller.removeScriptMessageHandlerForName_contentWorld(&NSString::from_str(KIT_HANDLER), &world);
                controller.removeAllUserScripts();
            }
        }
        if let Some(gate) = &self.gate {
            if let Some(old) = gate.current.borrow_mut().take() {
                // SAFETY: the page's own content controller, on the main thread.
                drop_rules(&*unsafe { self.view.configuration().userContentController() }, old, self.mtm);
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
        if let Some(ui) = &self.ui {
            // The popups a page opened go with its tab.
            let popups: Vec<Popup> = ui.popups.borrow_mut().drain(..).collect();
            for popup in popups {
                popup.window.close();
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
