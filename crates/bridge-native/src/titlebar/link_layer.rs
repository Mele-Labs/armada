//! The centred Fleet panel as the window draws it (`fleet-link-panel.html`): a dimmed screen below
//! the title row that takes no presses and no keys, and one panel in its middle until the state
//! clears. Its state is `link_panel.rs`'s; this file reads the disk for it, holds its keys, and
//! runs its presses.
//!
//! **The disk is read off the main thread, once a second, only while Fleet does not answer.** A walk
//! or a test sets [`StandIn`]: then nothing is read or run, and the scene says what the disk holds.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use gpui::{
    anchored, deferred, div, prelude::*, px, Anchor, AnyElement, AnyWindowHandle, Bounds, ClipboardItem, Context, Div, FocusHandle, FontWeight, Global, Hsla,
    Keystroke, Pixels, Role, SharedString, Stateful, Subscription, Task, Window,
};

use super::link_panel::{self, Act, Body, Disk, Log, Mark, Panel, Tone, Unit};
use super::parts::{glyph, kbd, mono, sans, spinner, tip};
use super::{Asked, Restarting, TitleBar};
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, Look};
use crate::runtime_file::{holder_of, Holder};

/// Paint order: above the screens' own layers, under the bar's popovers and menus while it stands.
pub(super) const LAYER: usize = 3;

/// The most of a log read: its tail, where its last lines are.
const TAIL: u64 = 64 * 1024;

/// What a walk or a test stands in for the disk and the presses with: the disk a scene says was
/// read, and what Start Fleet answers. While it is set, nothing is read, opened or run.
#[derive(Clone, Default)]
pub struct StandIn {
    pub disk: Disk,
    pub refusal: Option<String>,
}

impl Global for StandIn {}

/// The panel the bar holds, and what it needs to stand: the disk last read, its keys, its presses.
pub(super) struct LinkPanel {
    pub panel: Option<Panel>,
    disk: Disk,
    act_focus: FocusHandle,
    log_focus: FocusHandle,
    /// What had the keys before the panel took them.
    keys_were: Option<FocusHandle>,
    window: AnyWindowHandle,
    /// Start Fleet or Reopen Armada, pressed and not answered.
    pending: Option<Act>,
    /// What a press's command said when it failed.
    refused: Option<String>,
    _keys: Option<Subscription>,
    _poll: Option<Task<()>>,
    _run: Option<Task<()>>,
}

impl LinkPanel {
    pub fn new(window: &Window, cx: &mut Context<TitleBar>) -> Self {
        LinkPanel::blank(window.window_handle(), &cx.focus_handle(), &cx.focus_handle())
    }

    /// Nothing stands and nothing is read, as a walk's screen built again starts.
    pub fn clear(&mut self) {
        *self = LinkPanel::blank(self.window, &self.act_focus, &self.log_focus);
    }

    fn blank(window: AnyWindowHandle, act_focus: &FocusHandle, log_focus: &FocusHandle) -> Self {
        LinkPanel {
            panel: None,
            disk: Disk::default(),
            act_focus: act_focus.clone(),
            log_focus: log_focus.clone(),
            keys_were: None,
            window,
            pending: None,
            refused: None,
            _keys: None,
            _poll: None,
            _run: None,
        }
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|home| !home.is_empty()).map(PathBuf::from)
}

fn support(home: &Path) -> PathBuf {
    home.join("Library").join("Application Support").join("Armada")
}

fn log_path(log: Log) -> Option<PathBuf> {
    let name = match log {
        Log::Restart => "restart.log",
        Log::Fleet => "com.armada.fleet.log",
    };
    Some(home()?.join("Library").join("Logs").join("Armada").join(name))
}

/// Read what the panel is worked out from. Blocking, and it asks `ps`: off the main thread only.
fn read_disk() -> Disk {
    let Some(home) = home() else { return Disk::default() };
    let support = support(&home);
    let lock = support.join("restart.lock");
    // The lock is a symlink whose target is the restart's pid; a dead pid is a restart that ended.
    let restart = std::fs::read_link(&lock)
        .ok()
        .and_then(|target| target.to_str()?.trim().parse::<u32>().ok())
        .filter(|pid| holder_of(*pid) != Holder::Vacant)
        .and_then(|_| std::fs::symlink_metadata(&lock).ok()?.modified().ok())
        .map(DateTime::<Local>::from);
    Disk {
        restart,
        restart_log: log_path(Log::Restart).and_then(|path| tail(&path)),
        fleet_log: log_path(Log::Fleet).and_then(|path| tail(&path)),
        current: std::fs::read_link(support.join("builds").join("current")).ok().and_then(|target| target.file_name()?.to_str().map(String::from)),
        app_build: app_build(&support),
    }
}

/// The kept build this app runs from: its binary sits at `<build>/Armada.app/Contents/MacOS/Armada`.
fn app_build(support: &Path) -> Option<String> {
    let exe = std::fs::canonicalize(std::env::current_exe().ok()?).ok()?;
    let builds = std::fs::canonicalize(support.join("builds")).ok()?;
    kept_build_of(&exe, &builds)
}

/// The folder of `builds` that `exe` sits in, by its name.
fn kept_build_of(exe: &Path, builds: &Path) -> Option<String> {
    exe.ancestors().find(|one| one.parent() == Some(builds))?.file_name()?.to_str().map(String::from)
}

/// The last `TAIL` bytes of a file, from its first whole line.
fn tail(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let from = file.metadata().ok()?.len().saturating_sub(TAIL);
    file.seek(SeekFrom::Start(from)).ok()?;
    let mut bytes = Vec::new();
    file.take(TAIL).read_to_end(&mut bytes).ok()?;
    Some(whole_lines(String::from_utf8_lossy(&bytes).into_owned(), from > 0))
}

/// A read that began inside a line drops what it has of that line.
fn whole_lines(text: String, cut: bool) -> String {
    if !cut {
        return text;
    }
    text.split_once('\n').map_or_else(String::new, |(_, rest)| rest.to_string())
}

/// What a command said when it failed: its stderr, else its stdout, else its exit.
fn said_of(out: &Output) -> String {
    let words = |bytes: &[u8]| Some(String::from_utf8_lossy(bytes).trim().to_string()).filter(|said| !said.is_empty());
    words(&out.stderr).or_else(|| words(&out.stdout)).unwrap_or_else(|| format!("exited {}", out.status.code().unwrap_or(1)))
}

fn ran(command: &mut Command) -> Result<(), String> {
    let out = command.output().map_err(|why| format!("{} would not run: {why}", command.get_program().to_string_lossy()))?;
    if out.status.success() { Ok(()) } else { Err(said_of(&out)) }
}

/// `armada fleet start`, from the build `builds/current` names.
fn start_fleet() -> Result<(), String> {
    let home = home().ok_or("HOME is not set")?;
    ran(Command::new(support(&home).join("builds").join("current").join("armada")).args(["fleet", "start"]))
}

/// This app's own launchd job, restarted: it opens `builds/current`.
fn reopen_app() -> Result<(), String> {
    let uid = Command::new("id").arg("-u").output().ok().and_then(|out| String::from_utf8(out.stdout).ok()).map(|text| text.trim().to_string()).ok_or("id -u would not run")?;
    ran(Command::new("launchctl").args(["kickstart", "-k", &format!("gui/{uid}/com.armada.fleet.native")]))
}

/// The state's ink and its ground.
fn tone_of(tone: Option<Tone>, look: &Look) -> (Hsla, Hsla) {
    let (ink, ground) = match tone {
        None => (token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
        Some(Tone::Stopped) => (token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG),
        Some(Tone::Restarting) => (token::STATUS_RUNNING, token::STATUS_RUNNING_BG),
        Some(Tone::Older) => (token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG),
    };
    (look.color(ink), look.color(ground))
}

impl TitleBar {
    /// The face's colour while the panel stands.
    pub(super) fn panel_ink(&self, look: &Look) -> Option<Hsla> {
        self.link.panel.as_ref().map(|panel| tone_of(Some(panel.tone), look).0)
    }

    /// Work the panel out again from the connection and the disk last read. It reads the disk
    /// again each second while Fleet does not answer, and takes the keys while it stands.
    pub(super) fn settle_panel(&mut self, cx: &mut Context<Self>) {
        let (now, wall) = (Instant::now(), Local::now());
        let from = self.link_read.as_ref().and_then(|link| link.dropped).map_or(self.opened, |dropped| dropped.max(self.opened));
        let since = wall - chrono::Duration::from_std(now.saturating_duration_since(from)).unwrap_or_default();
        let live = self.face.fleet.connection.is_live();
        let swapping = self.face.build.as_ref().is_some_and(|held| held.report.swapping);
        // The swap's own hold covers a Fleet stopped for a swap.
        let mut panel = if swapping { None } else { link_panel::panel_of(&self.face.fleet.connection, &self.link.disk, since, wall) };
        // A Restart Fleet pressed here keeps its panel while launchd brings Fleet back.
        if panel.is_none() && !live && self.face.fleet.restarting == Restarting::Working {
            panel = self.link.panel.clone();
        }
        if self.link_read.is_some() && !live {
            self.poll(cx);
        } else {
            self.link._poll = None;
        }
        if panel == self.link.panel {
            return;
        }
        let was = std::mem::replace(&mut self.link.panel, panel);
        let (before, after) = (was.as_ref().map(|one| (one.tone, one.act)), self.link.panel.as_ref().map(|one| (one.tone, one.act)));
        if before.map(|(tone, _)| tone) != after.map(|(tone, _)| tone) {
            self.link.pending = None;
            self.link.refused = None;
        }
        match (before, after) {
            (None, Some(_)) => self.hold_keys(cx),
            (Some(_), None) => self.release_keys(cx),
            (Some((_, was_act)), Some((_, act))) if was_act != act => self.focus_panel(cx),
            _ => {}
        }
        cx.notify();
    }

    /// A scene's disk and Start Fleet's answer, in place of the Mac's.
    pub(super) fn stand_in(&mut self, stand_in: StandIn, cx: &mut Context<Self>) {
        self.link.disk = stand_in.disk.clone();
        cx.set_global(stand_in);
        self.settle_panel(cx);
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        if self.link._poll.is_some() {
            return;
        }
        self.link._poll = Some(cx.spawn(async move |this, cx| loop {
            let stood_in = this.update(cx, |_, cx| cx.try_global::<StandIn>().map(|stand_in| stand_in.disk.clone())).ok().flatten();
            let disk = match stood_in {
                Some(disk) => disk,
                None => cx.background_executor().spawn(async { read_disk() }).await,
            };
            let read = this.update(cx, |bar, cx| {
                bar.link.disk = disk;
                bar.settle_panel(cx);
            });
            if read.is_err() {
                break;
            }
            cx.background_executor().timer(Duration::from_secs(1)).await;
        }));
    }

    /// The panel takes the keys: Return and Tab are its own, and nothing behind it hears any.
    fn hold_keys(&mut self, cx: &mut Context<Self>) {
        let (me, here) = (cx.entity().downgrade(), self.link.window.window_id());
        self.link._keys = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != here {
                return;
            }
            let Some(me) = me.upgrade() else { return };
            if me.update(cx, |bar, cx| bar.panel_key(&event.keystroke, window, cx)) {
                cx.stop_propagation();
            }
        }));
        let (handle, bar) = (self.link.window, cx.entity().downgrade());
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(bar) = bar.upgrade() {
                    bar.update(cx, |bar, cx| {
                        bar.link.keys_were = window.focused(cx);
                        bar.focus_panel_in(window, cx);
                    });
                }
            });
        });
    }

    /// The keys go back where they were before the panel stood.
    fn release_keys(&mut self, cx: &mut Context<Self>) {
        self.link._keys = None;
        let Some(were) = self.link.keys_were.take() else { return };
        let handle = self.link.window;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| window.focus(&were, cx));
        });
    }

    fn focus_panel(&mut self, cx: &mut Context<Self>) {
        let (handle, bar) = (self.link.window, cx.entity().downgrade());
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(bar) = bar.upgrade() {
                    bar.update(cx, |bar, cx| bar.focus_panel_in(window, cx));
                }
            });
        });
    }

    /// The button where there is one, else Open log.
    fn focus_panel_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = &self.link.panel else { return };
        let handle = if panel.act.is_some() { &self.link.act_focus } else { &self.link.log_focus };
        window.focus(handle, cx);
        cx.notify();
    }

    /// A key while the panel stands: whether it is swallowed. The bar's own popover or menu has the
    /// keys while one is open; the capture keys and ⌘Q pass.
    fn panel_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.popup.is_some() || self.menu.is_some() || self.dialog.is_some() {
            return false;
        }
        let held = &key.modifiers;
        let bare = !held.platform && !held.control && !held.alt && !held.function;
        match key.key.as_str() {
            "enter" if bare && !held.shift => {
                if self.link.log_focus.is_focused(window) {
                    self.open_log(cx);
                } else if let Some(act) = self.link.panel.as_ref().and_then(|panel| panel.act) {
                    self.panel_pressed(act, cx);
                }
                true
            }
            "tab" if bare => {
                let has_act = self.link.panel.as_ref().is_some_and(|panel| panel.act.is_some());
                let has_log = self.link.panel.as_ref().is_some_and(|panel| panel.log.is_some());
                let on_act = self.link.act_focus.is_focused(window);
                let to = if has_act && (!on_act || !has_log) { &self.link.act_focus } else { &self.link.log_focus };
                window.focus(to, cx);
                cx.notify();
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

    fn panel_pressed(&mut self, act: Act, cx: &mut Context<Self>) {
        match act {
            Act::RestartFleet => {
                if self.face.fleet.restarting != Restarting::Working {
                    self.restart_fleet_pressed(cx);
                }
            }
            Act::StartFleet | Act::ReopenApp => self.run_act(act, cx),
        }
    }

    /// Start Fleet and Reopen Armada run their command off the main thread; a failure stands under the units.
    fn run_act(&mut self, act: Act, cx: &mut Context<Self>) {
        if self.link.pending.is_some() {
            return;
        }
        self.link.pending = Some(act);
        self.link.refused = None;
        cx.notify();
        if let Some(stand_in) = cx.try_global::<StandIn>() {
            if act == Act::StartFleet {
                if let Some(refusal) = stand_in.refusal.clone() {
                    self.link.pending = None;
                    self.link.refused = Some(refusal);
                }
            }
            Note::info("not run: a walk stands in for the command").field("act", act.label(false)).emit();
            return;
        }
        let ran = cx.background_executor().spawn(async move {
            match act {
                Act::StartFleet => start_fleet(),
                Act::ReopenApp => reopen_app(),
                Act::RestartFleet => Ok(()),
            }
        });
        self.link._run = Some(cx.spawn(async move |this, cx| {
            let said = ran.await;
            let _ = this.update(cx, |bar, cx| {
                bar.link.pending = None;
                if let Err(why) = said {
                    Note::warn("a Fleet panel press failed").field("act", act.label(false)).field("said", why.as_str()).emit();
                    bar.link.refused = Some(why);
                }
                cx.notify();
            });
        }));
    }

    fn open_log(&mut self, cx: &mut Context<Self>) {
        let Some(log) = self.link.panel.as_ref().and_then(|panel| panel.log) else { return };
        if cx.has_global::<StandIn>() {
            Note::info("not opened: a walk stands in for the log").emit();
            return;
        }
        let Some(path) = log_path(log) else { return };
        cx.background_executor()
            .spawn(async move {
                if let Err(why) = Command::new("open").arg(&path).status() {
                    Note::warn("the log would not open").field("why", why.to_string().as_str()).emit();
                }
            })
            .detach();
    }

    /// Copy debug info, from the Fleet popover: the state's code and words, and this app's protocol.
    pub(super) fn copy_debug_info(&mut self, cx: &mut Context<Self>) {
        let statement = self.face.fleet.connection.statement();
        let mut lines = vec![statement.code.to_string(), statement.headline.clone()];
        lines.extend(statement.detail.clone());
        lines.extend(self.link.panel.as_ref().map(|panel| panel.title.to_string()));
        lines.push(format!("protocol {}", ipc::ProtocolId::current()));
        cx.write_to_clipboard(ClipboardItem::new_string(lines.join("\n")));
        cx.emit(Asked::CopyDebugInfo);
    }

    /// The dim below the title row and the panel in its middle. Nothing while no state stands.
    pub(super) fn link_layer(&self, look: &Look, room: Bounds<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let panel = self.link.panel.as_ref()?;
        let (ink, ground) = tone_of(Some(panel.tone), look);
        // The dim starts under the title row, which stays live: its face and menu still answer.
        let top = self.at.row.get().map_or(px(0.), |row| (row.origin.y + row.size.height - room.origin.y).max(px(0.)));
        let width = (look.length(token::W_DIALOG_WIDE) + look.length(token::SPACE_8) * 2.).min(room.size.width - look.length(token::SPACE_6) * 2.);
        let head_glyph: AnyElement = match panel.tone {
            Tone::Restarting => spinner("link-panel-turning", look.length(token::SPACE_4), ink, cx).into_any_element(),
            Tone::Stopped => glyph("circle-x", look.length(token::SPACE_4), ink).into_any_element(),
            Tone::Older => glyph("triangle-alert", look.length(token::SPACE_4), ink).into_any_element(),
        };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(ground)
            .text_color(ink)
            .text_size(look.length(token::TEXT_BASE))
            .line_height(look.length(token::LEADING_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(head_glyph)
            .child(div().flex_1().min_w_0().truncate().child(panel.title))
            .child(labelled(
                div()
                    .id("link-panel-since")
                    .flex_none()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(panel.since.clone()),
                Role::Timer,
                panel.since.clone(),
            ));
        let why = labelled(
            div()
                .id("link-panel-why")
                .px(look.length(token::SPACE_4))
                .pt(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(panel.why),
            Role::Label,
            panel.why,
        );
        let wire = labelled(
            div()
                .id("link-panel-wire")
                .flex_none()
                .w(look.length(token::SPACE_12) * 2.)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_MUTED))
                .child(div().w_full().border_t(look.length(token::BORDER_WIDTH) * 2.).border_dashed().border_color(ink))
                .child(div().w_full().px(look.length(token::SPACE_1)).text_center().child(panel.link)),
            Role::Status,
            panel.link,
        );
        let sides = div()
            .flex()
            .items_stretch()
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .child(self.unit("link-panel-fleet", "server", "Fleet", &panel.fleet, look, cx))
            .child(wire)
            .child(self.unit("link-panel-app", "layers", "This app", &panel.app, look, cx));
        let refused = match &self.face.fleet.restarting {
            Restarting::Failed { why, detail } if panel.act == Some(Act::RestartFleet) => Some(format!("{}: {detail}", why.said().0)),
            _ => self.link.refused.clone(),
        };
        let refusal = refused.map(|said| {
            labelled(
                div()
                    .id("link-panel-refused")
                    .mx(look.length(token::SPACE_4))
                    .mb(look.length(token::SPACE_4))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .font_family(mono(look))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(glyph("circle-x", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().whitespace_normal().child(said.clone())),
                Role::Status,
                said,
            )
        });
        let note = panel.note.map(|said| {
            labelled(
                div().id("link-panel-note").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(said),
                Role::Label,
                said,
            )
            .into_any_element()
        });
        let (left_note, right_note) = if panel.act.is_some() { (note, None) } else { (None, note) };
        let log = panel.log.map(|_| {
            let button = self
                .panel_button("link-panel-open-log", "Open log", glyph("file-text", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)).into_any_element(), None, &self.link.log_focus, look)
                .on_click(cx.listener(|this, _, _, cx| this.open_log(cx)));
            labelled(button, Role::Button, "Open log").into_any_element()
        });
        let act = panel.act.map(|act| {
            let pending = self.link.pending == Some(act) || (act == Act::RestartFleet && self.face.fleet.restarting == Restarting::Working);
            let label = act.label(pending);
            // Under way, the glyph turns, and the button takes no press.
            let mark = if pending {
                spinner("link-panel-pending", look.length(token::SPACE_4), ink, cx).into_any_element()
            } else {
                glyph(act.glyph(), look.length(token::SPACE_4), ink).into_any_element()
            };
            let mut button = self.panel_button(SharedString::from(format!("link-panel-{}", act.glyph())), label, mark, Some((ink, ground)), &self.link.act_focus, look);
            if !pending {
                button = button.child(kbd("⏎", look)).cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.panel_pressed(act, cx)));
            }
            labelled(button, Role::Button, label).into_any_element()
        });
        let foot = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .children(log)
            .children(left_note)
            .child(div().flex_1())
            .children(right_note)
            .children(act);
        let card = labelled(
            div()
                .id("link-panel")
                .w(width)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(ink)
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .font_family(sans(look))
                .text_color(look.color(token::FG_DEFAULT))
                .child(head)
                .child(why)
                .child(sides)
                .children(refusal)
                .child(foot),
            Role::Dialog,
            panel.title,
        );
        // The design fades the screen and then lays black at 0.55 over it. The scrim token is the fade;
        // the overlay shadow's black is the 0.55. Either alone leaves the dark theme's list near full
        // strength, and a screen that is not live has to read as off.
        let black = look.shadows(token::SHADOW_OVERLAY).first().map(|layer| layer.color);
        let scrim = div()
            .id("link-panel-scrim")
            .absolute()
            .top(top)
            .left_0()
            .w(room.size.width)
            .h((room.size.height - top).max(px(0.)))
            .bg(look.color(token::SCRIM))
            .occlude()
            .child(div().size_full().flex().items_center().justify_center().when_some(black, |dim, black| dim.bg(black)).child(card));
        Some(
            deferred(anchored().position(room.origin).anchor(Anchor::TopLeft).child(div().relative().w(room.size.width).h(room.size.height).child(scrim)))
                .with_priority(LAYER)
                .into_any_element(),
        )
    }

    /// A button of the foot: in the state's colours where `tone` is given, else plain.
    fn panel_button(&self, id: impl Into<gpui::ElementId>, label: &'static str, mark: AnyElement, tone: Option<(Hsla, Hsla)>, focus: &FocusHandle, look: &Look) -> Stateful<Div> {
        let (ink, ground, edge) = match tone {
            Some((ink, ground)) => (ink, ground, ink),
            None => (look.color(token::FG_DEFAULT), look.color(token::BG_HOVER), look.color(token::BORDER_DEFAULT)),
        };
        // Return presses the focused button, so focus shows from the start, not only after Tab: a ring
        // in the button's own colour, held off it by the foot's ground.
        let ring_ink = if tone.is_some() { ink } else { Self::focus_ring(look) };
        let hair = look.length(token::BORDER_WIDTH);
        let ring = |spread: Pixels, color: Hsla| gpui::BoxShadow { color, offset: gpui::point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: spread, inset: false };
        let rings = vec![ring(hair * 4., ring_ink), ring(hair * 2., look.color(token::BG_RAISED))];
        div()
            .id(id.into())
            .track_focus(focus)
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(ink)
            .focus(move |style| style.shadow(rings))
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
            .child(mark)
            .child(label)
    }

    /// Fleet or this app: its band in its state's colour, green when fine, and what it holds.
    fn unit(&self, id: &'static str, icon: &'static str, who: &'static str, unit: &Unit, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (ink, ground) = tone_of(unit.tone, look);
        let edge = if unit.tone.is_some() { ink } else { look.color(token::BORDER_DEFAULT) };
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(ground)
            .text_color(ink)
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(glyph(icon, look.length(token::SPACE_4), ink))
            .child(div().text_color(look.color(token::FG_DEFAULT)).child(who))
            .child(format!("· {}", unit.state));
        let small = |child: Div| child.text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS));
        let body: AnyElement = match &unit.body {
            Body::Builds { build, protocol } => {
                let key = |word: &'static str| div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)).child(word);
                let (build_said, build_row) = match build {
                    Some(build) => {
                        let (chip_ink, chip_ground, word) = if build.current {
                            (token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG, "current")
                        } else {
                            (token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG, "older")
                        };
                        let value = format!("{} · {}", build.commit, build.at);
                        let chip = div()
                            .flex_none()
                            .px(look.length(token::SPACE_2))
                            .rounded_full()
                            .bg(look.color(chip_ground))
                            .text_color(look.color(chip_ink))
                            .font_family(sans(look))
                            .child(word);
                        (format!("Build, {value}, {word}"), div().flex().items_center().gap(look.length(token::SPACE_2)).min_w_0().child(div().truncate().child(value)).child(chip))
                    }
                    None => ("Build, not a kept build".to_string(), div().text_color(look.color(token::FG_SUBTLE)).font_family(sans(look)).child("not a kept build")),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(
                        small(div()).id(SharedString::from(format!("{id}-build"))).flex().items_center().child(key("Build")).child(build_row.font_family(mono(look))),
                        Role::Label,
                        build_said,
                    ))
                    .child(labelled(
                        small(div()).id(SharedString::from(format!("{id}-protocol"))).flex().items_center().child(key("Protocol")).child(div().font_family(mono(look)).child(protocol.clone())),
                        Role::Label,
                        format!("Protocol, {protocol}"),
                    ))
                    .into_any_element()
            }
            Body::LastWords { from, line } => labelled(
                small(div())
                    .id(SharedString::from(format!("{id}-last-words")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_SUBTLE)).child(format!("Last words, {from}")))
                    .child(div().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).whitespace_normal().child(line.clone())),
                Role::Group,
                format!("Last words, {from}"),
            )
            .into_any_element(),
            Body::Steps(steps) => labelled(
                small(div()).id(SharedString::from(format!("{id}-steps"))).flex().flex_col().gap(look.length(token::SPACE_1)).children(steps.iter().enumerate().map(|(at, step)| {
                    let size = look.length(token::SPACE_4) * 0.875;
                    let (mark, colour, weight, word): (AnyElement, Hsla, f32, &str) = match step.mark {
                        Mark::Done => {
                            let colour = look.color(token::STATUS_COMPLETED_SUCCESS);
                            (glyph("circle-check", size, colour).into_any_element(), colour, look.number(token::WEIGHT_BODY), "done")
                        }
                        Mark::Now => {
                            let colour = look.color(token::STATUS_RUNNING);
                            (spinner(SharedString::from(format!("{id}-step-now")), size, colour, cx).into_any_element(), colour, look.number(token::WEIGHT_HEADING), "now")
                        }
                        Mark::Next => {
                            let colour = look.color(token::FG_SUBTLE);
                            (glyph("circle", size, colour).into_any_element(), colour, look.number(token::WEIGHT_BODY), "next")
                        }
                    };
                    labelled(
                        div()
                            .id(SharedString::from(format!("{id}-step-{at}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_color(colour)
                            .font_weight(FontWeight(weight))
                            .tooltip(tip(step.line.clone()))
                            .child(mark)
                            .child(div().flex_1().min_w_0().truncate().child(step.text.clone())),
                        Role::ListItem,
                        format!("{}, {word}", step.text),
                    )
                })),
                Role::List,
                "Restart steps",
            )
            .into_any_element(),
            Body::Empty => div().into_any_element(),
        };
        labelled(
            div()
                .id(id)
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge)
                .bg(look.color(token::BG_RAISED))
                .child(band)
                .child(div().flex_1().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).child(body)),
            Role::Group,
            format!("{who}, {}", unit.state),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_apps_build_is_the_folder_of_builds_its_binary_sits_in() {
        let builds = Path::new("/Users/user/Library/Application Support/Armada/builds");
        let exe = builds.join("2026-11-13T004852Z-126b4bb/Armada.app/Contents/MacOS/Armada");
        assert_eq!(kept_build_of(&exe, builds).as_deref(), Some("2026-11-13T004852Z-126b4bb"));
        assert_eq!(kept_build_of(Path::new("/src/crates/bridge-native/target/release/bridge-native"), builds), None);
    }

    #[test]
    fn a_tail_read_from_inside_a_line_drops_that_line() {
        assert_eq!(whole_lines("cut half\nwhole one\nwhole two\n".into(), true), "whole one\nwhole two\n");
        assert_eq!(whole_lines("first\nsecond\n".into(), false), "first\nsecond\n");
        assert_eq!(whole_lines("no newline yet".into(), true), "");
    }
}
