//! The centred crash panel: on the first launch after Armada quit on a panic, or on a crash macOS reported
//! and no panic did, a dimmed screen below the
//! title row and one red panel in its middle (`crash.rs` holds what it reads). It is drawn as the Fleet
//! panel is (`link_layer.rs`) and takes the keys the same way: Return starts a Job to fix it, Esc is Later,
//! Tab moves between the three buttons and Space presses the one the keys are on.
//!
//! A walk or a test sets [`CrashStandIn`]: then nothing is read, opened, sent or marked on disk, and what
//! each press would have done is written down in it instead.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, AnyWindowHandle, Bounds, Context, Entity, FocusHandle, FontWeight, Global, Keystroke, Pixels, Role, SharedString, Subscription, Task, Window};
use ipc::{DispatchJob, ManifestId};

use super::parts::{glyph, kbd, mono, sans, spinner};
use super::{Asked, TitleBar};
use crate::crash::{self, Crash, KeptDraft};
use crate::fleet::FleetConnection;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, Look};

/// What the panel shows: the crash, what he had typed that was kept, and the files the Job would get.
#[derive(Clone)]
pub struct Offer {
    pub crash: Crash,
    /// macOS's own report of the same crash, if one was found.
    pub macos: Option<PathBuf>,
    pub kept: Vec<KeptDraft>,
}

impl Offer {
    /// The names of the files the Job gets.
    fn names(&self) -> Vec<String> {
        let name = |path: &Path| path.file_name().and_then(|name| name.to_str()).unwrap_or("report").to_string();
        std::iter::once(name(&self.crash.file)).chain(self.macos.as_deref().map(name)).collect()
    }
}

/// What a press did, as a walk or a test stood in for it.
#[derive(Clone, Debug, PartialEq)]
pub enum Pressed {
    /// Start a Job to fix it, sent to Fleet.
    Started(Box<DispatchJob>),
    /// The crash was marked seen.
    Seen,
    /// The crash file was opened.
    Opened,
}

/// What a walk or a test stands in for the disk, the wire and Fleet's answer with.
#[derive(Clone)]
pub struct CrashStandIn {
    /// The files the Job gets, by name.
    pub files: Vec<(String, Vec<u8>)>,
    /// The Job Fleet started, or why it would not.
    pub answer: Result<String, String>,
    pub done: Rc<RefCell<Vec<Pressed>>>,
}

impl Global for CrashStandIn {}

/// What Fleet said to the dispatch.
struct Dispatched {
    /// The Job it started, which opens.
    job: Option<String>,
    /// The Operator's hold on the dispatch, or the idea kept: a toast says what became of it.
    held: Option<String>,
    words: String,
}

pub(super) struct CrashPanel {
    shown: Option<Offer>,
    wire: Option<Entity<FleetConnection>>,
    open_focus: FocusHandle,
    later_focus: FocusHandle,
    start_focus: FocusHandle,
    keys_were: Option<FocusHandle>,
    window: AnyWindowHandle,
    sending: bool,
    refused: Option<String>,
    _keys: Option<Subscription>,
    _task: Option<Task<()>>,
}

impl CrashPanel {
    pub fn new(window: &Window, cx: &mut Context<TitleBar>) -> Self {
        CrashPanel {
            shown: None,
            wire: None,
            open_focus: cx.focus_handle(),
            later_focus: cx.focus_handle(),
            start_focus: cx.focus_handle(),
            keys_were: None,
            window: window.window_handle(),
            sending: false,
            refused: None,
            _keys: None,
            _task: None,
        }
    }

    /// Nothing stands, as a walk's screen built again starts.
    pub fn clear(&mut self) {
        self.shown = None;
        self.sending = false;
        self.refused = None;
        self._keys = None;
        self._task = None;
    }

    pub fn is_shown(&self) -> bool {
        self.shown.is_some()
    }

    pub fn refused(&self) -> Option<&str> {
        self.refused.as_deref()
    }
}

/// Read the newest crash not yet seen, with macOS's report of it and what was kept; where no panic wrote one,
/// macOS's report of a run that did not quit. `last_try` forgets the ended runs macOS wrote nothing of.
/// Blocking: off the main thread.
pub fn read_offer(crashes: &Path, reports: Option<&Path>, last_try: bool, title_of: &dyn Fn(&str) -> Option<String>) -> Option<Offer> {
    if let Some(crash) = crash::newest_unseen(crashes) {
        let macos = reports.and_then(|dir| crash::macos_report(dir, &crash));
        return Some(Offer { crash, macos, kept: crash::kept_now(title_of) });
    }
    let runs = crash::ended_runs(crashes, std::process::id());
    let crash = reports.and_then(|dir| crash::newest_macos(dir, &runs));
    if crash.is_some() || last_try {
        crash::forget_runs(&runs, crash.as_ref());
    }
    Some(Offer { crash: crash?, macos: None, kept: crash::kept_now(title_of) })
}

/// How often the start looks again for macOS's report, and how long between: macOS writes it a moment after
/// the process goes, and launchd may have opened this run first.
const REPORT_TRIES: u32 = 15;
const REPORT_PAUSE: std::time::Duration = std::time::Duration::from_secs(2);

/// The live app's start: read what a crash left, and stand the panel if there is one. The wire is Fleet's.
pub fn offer_crash(fleet: &Entity<FleetConnection>, cx: &mut gpui::App) {
    let (Some(crashes), reports) = (crash::crashes_dir(), crash::macos_reports_dir()) else { return };
    let executor = cx.background_executor().clone();
    let read = cx.background_executor().spawn(async move {
        for left in (0..REPORT_TRIES).rev() {
            if let Some(offer) = read_offer(&crashes, reports.as_deref(), left == 0, &|_| None) {
                return Some(offer);
            }
            if crash::ended_runs(&crashes, std::process::id()).is_empty() {
                return None;
            }
            executor.timer(REPORT_PAUSE).await;
        }
        None
    });
    let fleet = fleet.clone();
    cx.spawn(async move |cx| {
        let Some(offer) = read.await else { return };
        // The bar is drawn with the window, a moment after this was asked for.
        for _ in 0..40 {
            let offered = cx.update(|cx| {
                let bar = super::live(cx)?;
                bar.update(cx, |bar, cx| bar.crash_offered(Some(offer.clone()), Some(fleet.clone()), cx));
                Some(())
            });
            if offered.is_some() {
                return;
            }
            cx.background_executor().timer(std::time::Duration::from_millis(250)).await;
        }
    })
    .detach();
}

/// The time a crash is shown at.
fn clock(crash: &Crash) -> String {
    crash.at.with_timezone(&Local).format("%H:%M").to_string()
}

impl TitleBar {
    /// The panel stands for `offer`, or goes where there is none.
    pub fn crash_offered(&mut self, offer: Option<Offer>, wire: Option<Entity<FleetConnection>>, cx: &mut Context<Self>) {
        let was = self.crash.shown.is_some();
        self.crash.shown = offer;
        self.crash.wire = wire;
        self.crash.sending = false;
        self.crash.refused = None;
        match (was, self.crash.shown.is_some()) {
            (false, true) => self.hold_crash_keys(cx),
            (true, false) => self.release_crash_keys(cx),
            _ => {}
        }
        cx.notify();
    }

    /// Whether the panel stands, and what a refused start said.
    pub fn crash_state(&self) -> (bool, Option<String>) {
        (self.crash.is_shown(), self.crash.refused().map(str::to_string))
    }

    /// The panel takes the keys: Return, Esc and Tab are its own, and nothing behind it hears any.
    fn hold_crash_keys(&mut self, cx: &mut Context<Self>) {
        let (me, here) = (cx.entity().downgrade(), self.crash.window.window_id());
        self.crash._keys = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != here {
                return;
            }
            let Some(me) = me.upgrade() else { return };
            if me.update(cx, |bar, cx| bar.crash_key(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        }));
        let (handle, bar) = (self.crash.window, cx.entity().downgrade());
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(bar) = bar.upgrade() {
                    bar.update(cx, |bar, cx| {
                        bar.crash.keys_were = window.focused(cx);
                        window.focus(&bar.crash.start_focus, cx);
                        cx.notify();
                    });
                }
            });
        });
    }

    /// The keys go back where they were before the panel stood.
    fn release_crash_keys(&mut self, cx: &mut Context<Self>) {
        self.crash._keys = None;
        let Some(were) = self.crash.keys_were.take() else { return };
        let handle = self.crash.window;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| window.focus(&were, cx));
        });
    }

    /// A key while the panel stands: whether it is swallowed. The bar's own popover or menu has the keys while
    /// one is open, and the Fleet panel, which is the graver state, has them before this one; the capture keys
    /// and ⌘Q pass.
    fn crash_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.crash.shown.is_none() || self.link.panel.is_some() || self.popup.is_some() || self.menu.is_some() || self.dialog.is_some() {
            return false;
        }
        let held = &key.modifiers;
        let bare = !held.platform && !held.control && !held.alt && !held.function;
        match key.key.as_str() {
            "escape" if bare => {
                self.later(cx);
                true
            }
            "tab" if bare => {
                let stops = [self.crash.open_focus.clone(), self.crash.later_focus.clone(), self.crash.start_focus.clone()];
                let at = stops.iter().position(|stop| stop.is_focused(window)).unwrap_or(2);
                let to = if held.shift { (at + 2) % 3 } else { (at + 1) % 3 };
                window.focus(&stops[to], cx);
                cx.notify();
                true
            }
            // Return and Space press the button that has the keys, as the Fleet panel's do; Start has them first.
            "enter" | "space" if bare && !held.shift => {
                if self.crash.open_focus.is_focused(window) {
                    self.open_crash_file(cx);
                } else if self.crash.later_focus.is_focused(window) {
                    self.later(cx);
                } else {
                    self.start_fix(cx);
                }
                true
            }
            word => {
                let passes = held.platform
                    && !held.control
                    && matches!((held.alt, held.shift, word), (false, false, "q") | (true, false, "a" | "s" | "c") | (false, true, "d"));
                !passes
            }
        }
    }

    /// The crash is marked seen, off the main thread.
    fn mark_crash_seen(&mut self, cx: &mut Context<Self>) {
        let Some(crash) = self.crash.shown.as_ref().map(|offer| offer.crash.clone()) else { return };
        if let Some(stand_in) = cx.try_global::<CrashStandIn>() {
            stand_in.done.borrow_mut().push(Pressed::Seen);
            return;
        }
        cx.background_executor()
            .spawn(async move {
                if let Err(why) = crash::mark_seen(&crash) {
                    Note::warn("a crash was not marked seen").field("why", why.to_string()).emit();
                }
            })
            .detach();
    }

    /// The panel goes.
    fn close_crash(&mut self, cx: &mut Context<Self>) {
        self.crash_offered(None, None, cx);
    }

    /// Later: seen, and gone.
    fn later(&mut self, cx: &mut Context<Self>) {
        if self.crash.sending {
            return;
        }
        self.mark_crash_seen(cx);
        self.close_crash(cx);
    }

    /// Open crash file: the file in the system's own app for it.
    fn open_crash_file(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.crash.shown.as_ref().map(|offer| offer.crash.file.clone()) else { return };
        if let Some(stand_in) = cx.try_global::<CrashStandIn>() {
            stand_in.done.borrow_mut().push(Pressed::Opened);
            return;
        }
        cx.background_executor()
            .spawn(async move {
                if let Err(why) = std::process::Command::new("open").arg(&path).status() {
                    Note::warn("the crash file would not open").field("why", why.to_string()).emit();
                }
            })
            .detach();
    }

    /// Start a Job to fix it: the dispatch goes to Fleet, and the Job it names opens. A refusal stays in the panel.
    fn start_fix(&mut self, cx: &mut Context<Self>) {
        if self.crash.sending {
            return;
        }
        let Some(offer) = self.crash.shown.clone() else { return };
        self.crash.sending = true;
        self.crash.refused = None;
        cx.notify();
        if let Some(stand_in) = cx.try_global::<CrashStandIn>().cloned() {
            let job = crash::fix_dispatch(&offer.crash, stand_in.files.clone(), ManifestId::carried("armada"));
            let words = job.start.request.clone().unwrap_or_default();
            stand_in.done.borrow_mut().push(Pressed::Started(Box::new(job)));
            let answer = stand_in.answer.map(|job| Dispatched { job: Some(job), held: None, words });
            self.crash_answered(answer, cx);
            return;
        }
        let Some(fleet) = self.crash.wire.clone() else { return self.crash_answered(Err("Fleet is not connected".into()), cx) };
        let held = fleet.read(cx).held();
        let (port, repositories) = (held.port(), held.repositories.clone().unwrap_or_default());
        let Some(port) = port else { return self.crash_answered(Err("Fleet is not connected".into()), cx) };
        let Some(repository) = crash::armada_repository(&crate::dispatch::live::served(&repositories)) else {
            return self.crash_answered(Err("Fleet serves no repository named armada".into()), cx);
        };
        let ran = cx.background_executor().spawn(async move {
            let job = crash::fix_it(&offer.crash, offer.macos.as_deref(), repository).map_err(|why| why.to_string())?;
            let words = job.start.request.clone().unwrap_or_default();
            let body = ipc::encode(&job).map_err(|why| why.to_string())?;
            let bytes = crate::dispatch::live::ask(port, "/sessions/dispatch".into(), Some(body))?;
            let done = ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())?;
            Ok::<_, crate::refusal::Failed>(Dispatched { job: done.session.map(|record| record.id.as_str().to_string()), held: done.operator.or(done.parked), words })
        });
        self.crash._task = Some(cx.spawn(async move |this, cx| {
            let answer = ran.await;
            let _ = this.update(cx, |bar, cx| {
                if let Err(failed) = &answer {
                    failed.tell_refusal("The Job to fix the crash was not started", cx);
                }
                bar.crash_answered(answer.map_err(String::from), cx)
            });
        }));
    }

    /// Fleet answered. A dispatch taken marks the crash seen, closes the panel and opens the Job it started; a
    /// refusal keeps the panel, with why.
    fn crash_answered(&mut self, answer: Result<Dispatched, String>, cx: &mut Context<Self>) {
        self.crash.sending = false;
        match answer {
            Ok(done) => {
                self.mark_crash_seen(cx);
                self.close_crash(cx);
                match (done.job, done.held) {
                    (Some(job), _) => cx.emit(Asked::OpenJob(job)),
                    (None, Some(held)) => crate::toasts::dispatch_held(&held, &done.words, cx),
                    (None, None) => {}
                }
            }
            Err(why) => {
                Note::warn("a Job to fix a crash was not started").field("why", why.as_str()).emit();
                self.crash.refused = Some(why);
                cx.notify();
            }
        }
    }

    /// The dim below the title row and the panel in its middle. Nothing while no crash stands, or while the
    /// Fleet panel does: Fleet being down is the graver state.
    pub(super) fn crash_layer(&self, look: &Look, room: Bounds<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let offer = self.crash.shown.as_ref().filter(|_| self.link.panel.is_none())?;
        let (ink, ground) = (look.color(token::STATUS_COMPLETED_FAILED), look.color(token::STATUS_COMPLETED_FAILED_BG));
        let hair = look.length(token::BORDER_WIDTH);
        let pad = look.length(token::SPACE_4);
        let at = clock(&offer.crash);
        let caption = |text: &'static str| {
            div()
                .px(pad)
                .pt(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .child(text.to_uppercase())
        };

        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(pad)
            .py(look.length(token::SPACE_3))
            .bg(ground)
            .text_color(ink)
            .text_size(look.length(token::TEXT_BASE))
            .line_height(look.length(token::LEADING_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(glyph("circle-x", look.length(token::SPACE_4), ink))
            .child(div().flex_1().min_w_0().truncate().child("Armada quit"))
            .child(labelled(
                div()
                    .id("crash-panel-time")
                    .flex_none()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(at.clone()),
                Role::Timer,
                at,
            ));
        // A panic writes what was typed before the app goes; a crash macOS reported writes nothing more, so the
        // panel claims only what the files held.
        let said: &'static str = if offer.crash.run.is_some() { "It stopped on an error and reopened." } else { "It stopped on an error and reopened. What you were typing is back where it was." };
        let why = labelled(
            div().id("crash-panel-why").px(pad).pt(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(said),
            Role::Label,
            said,
        );
        let error_box = |id: &'static str, lines: Vec<String>, said: String| {
            labelled(
                div()
                    .id(id)
                    .mx(pad)
                    .mt(look.length(token::SPACE_3))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(hair)
                    .border_color(ink)
                    .bg(ground)
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .font_family(mono(look))
                    .text_color(ink)
                    .child(glyph("circle-x", look.length(token::SPACE_4), ink))
                    .child(div().flex_1().min_w_0().flex().flex_col().children(lines.into_iter().map(|line| div().whitespace_normal().child(line)))),
                Role::Status,
                said,
            )
        };
        let mut panic_lines = vec![offer.crash.panic.clone()];
        if !offer.crash.location.is_empty() {
            panic_lines.push(offer.crash.location.clone());
        }
        let panic = error_box("crash-panel-panic", panic_lines.clone(), panic_lines.join(", "));

        let kept = (!offer.kept.is_empty()).then(|| {
            let rows = offer.kept.iter().enumerate().map(|(row, one)| {
                labelled(
                    div()
                        .id(SharedString::from(format!("crash-panel-kept-{row}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .when(row > 0, |one| one.border_t(hair).border_color(look.color(token::BORDER_SUBTLE)))
                        .child(glyph("circle-check", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_SUCCESS)))
                        .child(
                            div()
                                .flex_none()
                                .w(look.length(token::SPACE_12) * 2.)
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .child(one.kind),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(look.length(token::TEXT_SM))
                                .line_height(look.length(token::LEADING_SM))
                                .text_color(look.color(token::FG_DEFAULT))
                                .child(one.first_line.clone()),
                        ),
                    Role::ListItem,
                    format!("{}, {}", one.kind, one.first_line),
                )
            });
            div().flex().flex_col().child(caption("Kept")).child(labelled(
                div()
                    .id("crash-panel-kept")
                    .mx(pad)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(hair)
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .children(rows),
                Role::List,
                "Kept",
            ))
        });

        let chips = offer.names().into_iter().enumerate().map(|(at, name)| {
            labelled(
                div()
                    .id(SharedString::from(format!("crash-panel-file-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(hair)
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph("file", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(name.clone())),
                Role::ListItem,
                name,
            )
        });
        let files = div().flex().flex_col().child(caption("The Job gets")).child(labelled(
            div().id("crash-panel-files").px(pad).flex().flex_wrap().gap(look.length(token::SPACE_2)).children(chips),
            Role::List,
            "The Job gets",
        ));
        let refusal = self.crash.refused.clone().map(|said| error_box("crash-panel-refused", vec![said.clone()], said));

        let open = labelled(
            self.panel_button(
                "crash-panel-open",
                "Open crash file",
                Some(glyph("file-text", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)).into_any_element()),
                None,
                &self.crash.open_focus,
                look,
            )
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| this.open_crash_file(cx))),
            Role::Button,
            "Open crash file",
        );
        let later = labelled(
            self.panel_button("crash-panel-later", "Later", None, None, &self.crash.later_focus, look)
                .child(kbd("esc", look))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.later(cx))),
            Role::Button,
            "Later",
        );
        let sending = self.crash.sending;
        let mark = if sending {
            spinner("crash-panel-sending", look.length(token::SPACE_4), ink, cx).into_any_element()
        } else {
            glyph("wrench", look.length(token::SPACE_4), ink).into_any_element()
        };
        let mut button = self.panel_button("crash-panel-start", "Start a Job to fix it", Some(mark), Some((ink, ground)), &self.crash.start_focus, look);
        if !sending {
            button = button.child(kbd("⏎", look)).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.start_fix(cx)));
        }
        let start = labelled(button, Role::Button, "Start a Job to fix it");
        let foot = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(pad)
            .py(look.length(token::SPACE_3))
            .mt(look.length(token::SPACE_4))
            .border_t(hair)
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(open)
            .child(div().flex_1())
            .child(later)
            .child(start);

        let width = (look.length(token::W_DIALOG_WIDE) + look.length(token::SPACE_8) * 2.).min(room.size.width - look.length(token::SPACE_6) * 2.);
        let card = labelled(
            div()
                .id("crash-panel")
                .w(width)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(hair)
                .border_color(ink)
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .font_family(sans(look))
                .text_color(look.color(token::FG_DEFAULT))
                .child(head)
                .child(why)
                .child(panic)
                .children(kept)
                .child(files)
                .children(refusal)
                .child(foot),
            Role::Dialog,
            "Armada quit",
        );
        Some(self.dim_over("crash-panel-scrim", look, room, card.into_any_element()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::titlebar::{scenes, AskedSink};
    use gpui::{TestAppContext, VisualTestContext};

    /// A bar with the crash panel standing from the scene's stand-in, and what the bar asked and the stand-in did.
    fn stood(cx: &mut TestAppContext) -> (Entity<TitleBar>, &mut VisualTestContext, Rc<RefCell<Vec<Pressed>>>, Rc<RefCell<Vec<Asked>>>) {
        cx.update(crate::init_app);
        let asked = Rc::new(RefCell::new(Vec::new()));
        let heard = Rc::clone(&asked);
        cx.update(|cx| cx.set_global(AskedSink(Rc::new(move |one: &Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        let (offer, stand_in) = scenes::crash_scene("crash-panel", "mock-02").expect("the scene");
        let done = Rc::clone(&stand_in.done);
        cx.update(|_, cx| cx.set_global(stand_in));
        bar.update(cx, |bar, cx| bar.crash_offered(Some(offer), None, cx));
        cx.run_until_parked();
        (bar, cx, done, asked)
    }

    #[gpui::test]
    fn return_sends_one_dispatch_with_both_files_marks_the_crash_seen_and_opens_the_job(cx: &mut TestAppContext) {
        let (bar, cx, done, asked) = stood(cx);
        assert!(bar.read_with(cx, |bar, _| bar.crash_state().0), "the panel stands");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        let done = done.borrow();
        let sent: Vec<&DispatchJob> = done.iter().filter_map(|one| if let Pressed::Started(job) = one { Some(job.as_ref()) } else { None }).collect();
        assert_eq!(sent.len(), 1, "one Job is started: {done:?}");
        let Some(ipc::DispatchThen::Message(message)) = sent[0].then.first() else { panic!("the dispatch carries the first message") };
        let names: Vec<&str> = message.attachments.iter().map(|one| one.name.as_str()).collect();
        assert_eq!(names, ["crash-2026-11-14T110102Z.txt", "Armada-2026-11-14-111520.ips"]);
        assert!(message.attachments.iter().all(|one| one.media_type == "text/plain"));
        assert_eq!(done.iter().filter(|one| **one == Pressed::Seen).count(), 1, "the crash is marked seen once");
        assert!(!bar.read_with(cx, |bar, _| bar.crash_state().0), "the panel is gone");
        assert_eq!(asked.borrow().as_slice(), [Asked::OpenJob("mock-02".into())], "the Job it started opens");
    }

    #[gpui::test]
    fn a_refused_start_keeps_the_panel_with_what_fleet_said_and_marks_nothing_seen(cx: &mut TestAppContext) {
        cx.update(crate::init_app);
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        let (offer, stand_in) = scenes::crash_scene("crash-panel-refused", "mock-02").expect("the scene");
        let done = Rc::clone(&stand_in.done);
        cx.update(|_, cx| cx.set_global(stand_in));
        bar.update(cx, |bar, cx| bar.crash_offered(Some(offer), None, cx));
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(bar.read_with(cx, |bar, _| bar.crash_state()), (true, Some("Fleet serves no repository named armada".to_string())));
        assert!(!done.borrow().contains(&Pressed::Seen), "a refusal leaves the crash unseen");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(done.borrow().iter().filter(|one| matches!(one, Pressed::Started(_))).count(), 2, "Return tries again");
    }

    #[gpui::test]
    fn escape_marks_the_crash_seen_and_sends_nothing(cx: &mut TestAppContext) {
        let (bar, cx, done, asked) = stood(cx);
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        assert_eq!(done.borrow().as_slice(), [Pressed::Seen]);
        assert!(!bar.read_with(cx, |bar, _| bar.crash_state().0), "the panel is gone");
        assert!(asked.borrow().is_empty(), "no Job opens");
    }

    #[gpui::test]
    fn a_crash_macos_reported_stands_the_panel_and_its_job_gets_the_report(cx: &mut TestAppContext) {
        cx.update(crate::init_app);
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        let (offer, stand_in) = scenes::crash_scene("crash-panel-signal", "mock-02").expect("the scene");
        let done = Rc::clone(&stand_in.done);
        cx.update(|_, cx| cx.set_global(stand_in));
        bar.update(cx, |bar, cx| bar.crash_offered(Some(offer), None, cx));
        cx.run_until_parked();
        let named = |cx: &mut VisualTestContext, role: &str, name: &str| {
            cx.update(|window, cx| {
                let role = crate::labels::role_named(role).expect("a role");
                cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).is_some()
            })
        };
        assert!(named(cx, "dialog", "Armada quit"), "the same panel stands");
        assert!(named(cx, "status", "EXC_BAD_ACCESS (SIGSEGV), KERN_INVALID_ADDRESS at 0x0000000000000010, WTF::Detail::CallableWrapper<-[WKContentRuleListStore removeContentRuleListForIdentifier:completionHandler:]::$_4, void, std::__1::error_code>::call(std::__1::error_code) in WebKit"), "the exception and the frame it faulted in");
        assert!(named(cx, "label", "It stopped on an error and reopened."), "it claims nothing of what was typed");
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        let done = done.borrow();
        let Some(Pressed::Started(job)) = done.iter().find(|one| matches!(one, Pressed::Started(_))) else { panic!("a Job is started: {done:?}") };
        let Some(ipc::DispatchThen::Message(message)) = job.then.first() else { panic!("the dispatch carries the first message") };
        assert_eq!(message.attachments.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Armada-2026-11-17-162203.ips"]);
        assert!(message.text.contains("on this crash, which macOS reported."), "{}", message.text);
        assert!(done.contains(&Pressed::Seen), "the crash is marked seen");
    }

    #[test]
    fn a_panic_report_comes_before_macos_report_of_the_run_and_the_report_follows_once_it_is_seen() {
        let root = std::env::temp_dir().join(format!("armada-crash-panel-signal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (crashes, reports) = (root.join("crashes"), root.join("reports"));
        std::fs::create_dir_all(&reports).unwrap();
        crash::begin_run(&crashes, 4242).unwrap();
        crash::begin_run(&crashes, 4343).unwrap();
        crash::write_report(&crashes, chrono::Utc::now(), "panic: one\n").unwrap();

        let first = read_offer(&crashes, Some(&reports), true, &|_| None).expect("an offer");
        assert_eq!((first.crash.panic.as_str(), first.crash.run.is_none()), ("one", true), "the panic report first");
        crash::mark_seen(&first.crash).unwrap();

        assert!(read_offer(&crashes, Some(&reports), false, &|_| None).is_none(), "macOS has not written its report yet");
        assert!(crashes.join("run-4242").exists() && crashes.join("run-4343").exists(), "runs with no report are kept until the last try");

        let text = "{\"bundleID\":\"dev.armada.bridge.native\"}\n{\"pid\":4242,\"exception\":{\"type\":\"EXC_BAD_ACCESS\",\"signal\":\"SIGSEGV\"},\"faultingThread\":0,\"threads\":[{\"frames\":[{\"imageIndex\":0,\"symbol\":\"call\"}]}],\"usedImages\":[{\"name\":\"WebKit\"}]}";
        std::fs::write(reports.join("Armada-2026-11-17-162203.ips"), text).unwrap();
        let next = read_offer(&crashes, Some(&reports), false, &|_| None).expect("macOS's report");
        assert_eq!((next.crash.panic.as_str(), next.crash.location.as_str(), next.macos.as_deref()), ("EXC_BAD_ACCESS (SIGSEGV)", "call in WebKit", None));
        assert_eq!(next.crash.file, reports.join("Armada-2026-11-17-162203.ips"), "Open crash file opens macOS's report");
        assert!(crashes.join("run-4242").exists() && !crashes.join("run-4343").exists(), "the offered run is kept and the one with no report forgotten");
        crash::mark_seen(&next.crash).unwrap();
        assert!(read_offer(&crashes, Some(&reports), true, &|_| None).is_none(), "seen, it shows once");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[gpui::test]
    fn no_panel_stands_when_there_is_no_unseen_crash(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("armada-crash-panel-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_offer(&dir, None, false, &|_| None).is_none(), "an empty folder has no crash");
        let seen = crash::write_report(&dir, chrono::Utc::now(), "panic: one\n").unwrap();
        crash::mark_seen(&crash::newest_unseen(&dir).unwrap()).unwrap();
        assert!(seen.exists() && read_offer(&dir, None, false, &|_| None).is_none(), "a crash already seen is not offered");
        let _ = std::fs::remove_dir_all(&dir);

        cx.update(crate::init_app);
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        bar.update(cx, |bar, cx| bar.crash_offered(None, None, cx));
        cx.run_until_parked();
        assert_eq!(bar.read_with(cx, |bar, _| bar.crash_state()), (false, None));
        let drawn = cx.update(|window, cx| crate::labels::role_named("dialog").and_then(|role| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, "Armada quit")).is_some().then_some(role)));
        assert!(drawn.is_none(), "nothing named Armada quit is drawn");
    }
}
