//! One of a Job's shells as a pane: Electron's `ShellPanelPane` around `XtermScreen`
//! (`TerminalPane.tsx:327`, `XtermScreen.tsx`). A head with the shell's acts over a well that the
//! screen fills. It opens its shell once it knows its size in cells, as Electron's fits first, and
//! follows Fleet's output until the shell ends; then the head says Shell ended and offers a new one.
//!
//! **A self-contained view**, so the Job's panes mount it as a tab as it is.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

use alacritty_terminal::index::Side;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use gpui::{
    canvas, div, fill, point, prelude::*, pulsating_between, px, size, Animation, AnimationExt, App, Bounds, ClipboardItem, Context, ElementInputHandler, Entity, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, Font, FontFeatures, FontStyle, FontWeight, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Role, ScrollEvent, SharedString, Size, Task, TextRun, UTF16Selection, UnderlineStyle, StrikethroughStyle, Window,
};
use ipc::SessionRunMessage;

use super::colors::Palette;
use super::keys;
use super::screen::{Cells, Grain, Screen};
use super::shells::{Shells, Which};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, tip};

/// The key context a terminal's keys are bound under, so ⌘C copies its selection and not a text box's.
/// It is the registry's `terminal` scope, so an act scoped there, such as Enter for New shell, is heard here.
pub const KEYS: &str = "terminal";

/// Beside `KEYS` while the shell runs: a running shell keeps every key but a ⌘ chord.
pub const SHELL: &str = "shell";

/// The rows a run under a block keeps, as a program that addresses the screen expects; it shows only those it used.
const RUN_ROWS: u16 = 24;

/// How long the cursor shows and hides while it blinks (xterm.js's `cursorBlink`).
const BLINK: Duration = Duration::from_millis(600);

/// Output that arrives this soon after attaching to a running shell is its history, whose queries
/// were answered when they were first printed.
const REPLAY: Duration = Duration::from_millis(250);

/// How a pane's shell stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for its size, then for Fleet to open it.
    Opening,
    Running,
    /// It ended: Shell ended, New shell.
    Ended,
}

/// What the pane tells whoever holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneEvent {
    /// Fleet opened it under this id: `t1`.
    Opened(String),
    Ended,
    /// A program named the screen (OSC 0 or 2).
    Titled(String),
    /// Close on a run that ended: whoever holds it puts it away.
    Closed,
}

pub struct TerminalPane {
    job: SharedString,
    which: Which,
    terminal: Option<String>,
    run: Option<String>,
    phase: Phase,
    screen: Screen,
    focus: FocusHandle,
    shells: Rc<dyn Shells>,
    /// Typed before Fleet answered with the run, sent once it does (`job-shell.ts:415`).
    typed: Vec<u8>,
    asked: bool,
    replay_until: Option<Instant>,
    /// Where the grid was last drawn, and one cell's size: presses and the IME read them.
    grid: Rc<Cell<Option<(Bounds<Pixels>, Size<Pixels>)>>>,
    selecting: bool,
    /// Wheel travel short of a whole line, carried to the next turn.
    wheel: f32,
    blink_on: bool,
    /// What an input method is composing, drawn at the cursor until it is committed.
    marked: Option<String>,
    /// How a run exited: its code, none where it was stopped or ended by a signal, and whether it was stopped.
    exit: Option<(Option<i32>, bool)>,
    _read: Option<Task<()>>,
    _blink: Option<Task<()>>,
}

impl EventEmitter<PaneEvent> for TerminalPane {}

impl Focusable for TerminalPane {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TerminalPane {
    /// A pane for `job`'s shell `which`, opened once it is drawn and measured.
    pub fn new(job: impl Into<SharedString>, which: Which, shells: Rc<dyn Shells>, cx: &mut Context<Self>) -> Self {
        let terminal = match &which {
            Which::Named(id) => Some(id.clone()),
            _ => None,
        };
        let focus = cx.focus_handle();
        TerminalPane {
            job: job.into(),
            which,
            terminal,
            run: None,
            phase: Phase::Opening,
            screen: Screen::new(Cells::OPENED),
            focus,
            shells,
            typed: Vec::new(),
            asked: false,
            replay_until: None,
            grid: Rc::default(),
            selecting: false,
            wheel: 0.0,
            blink_on: true,
            marked: None,
            exit: None,
            _read: None,
            _blink: None,
        }
    }

    /// A pane on a shell already open, such as a terminal Job's first: it attaches as it is drawn.
    pub fn attached(job: impl Into<SharedString>, terminal: String, shells: Rc<dyn Shells>, cx: &mut Context<Self>) -> Self {
        Self::new(job, Which::Named(terminal), shells, cx)
    }

    pub fn terminal(&self) -> Option<&str> {
        self.terminal.as_deref()
    }

    pub fn job(&self) -> &SharedString {
        &self.job
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// A command's run, not a shell: Run on a block, or a Needs you run item.
    pub fn is_run(&self) -> bool {
        matches!(self.which, Which::Command(_))
    }

    /// Where its screen was last drawn.
    pub fn drawn(&self) -> Option<Bounds<Pixels>> {
        self.grid.get().map(|(bounds, _)| bounds)
    }

    /// How a run exited, once it has.
    pub fn exit(&self) -> Option<(Option<i32>, bool)> {
        self.exit
    }

    pub fn title(&self) -> Option<&str> {
        self.screen.title()
    }

    /// The pane's size became known or changed: open the shell at it, or tell Fleet.
    fn fit(&mut self, cells: Cells, cell: Size<Pixels>, cx: &mut Context<Self>) {
        let points = (f32::from(cell.width).round() as u16, f32::from(cell.height).round() as u16);
        let moved = self.screen.resize(cells, points);
        super::start::measured(cells, cx);
        if !self.asked && self.phase == Phase::Opening {
            self.asked = true;
            self.open(cells, cx);
        } else if moved {
            if let Some(run) = self.run.clone() {
                self.shells.resize(&run, cells, cx);
            }
        }
        if moved {
            cx.notify();
        }
    }

    fn open(&mut self, cells: Cells, cx: &mut Context<Self>) {
        let opening = self.shells.open(&self.job, self.which.clone(), cells, cx);
        cx.spawn(async move |this, cx| {
            let answer = opening.await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(underway) => this.adopt(underway.run_id, underway.terminal, cx),
                Err(why) => {
                    // A refused open prints the refusal and ends, as Electron's does (`job-shell.ts:470-476`); a refused
                    // run ends with code 1 (`sessions-wired.tsx:1252-1257`).
                    this.screen.feed(format!("{why}\r\n").as_bytes(), true);
                    this.exit = Some((Some(1), false));
                    this.end(cx);
                }
            });
        })
        .detach();
    }

    fn adopt(&mut self, run: String, terminal: Option<String>, cx: &mut Context<Self>) {
        // Attaching to a shell that was already running: its history comes first.
        if matches!(self.which, Which::Named(_) | Which::First) {
            self.replay_until = Some(Instant::now() + REPLAY);
        }
        if let Some(id) = terminal {
            self.terminal = Some(id.clone());
            // A new shell under the same id from here on.
            self.which = Which::Named(id.clone());
            cx.emit(PaneEvent::Opened(id));
        }
        self.phase = Phase::Running;
        self.run = Some(run.clone());
        let size = self.screen.size();
        self.shells.resize(&run, size, cx);
        let typed = std::mem::take(&mut self.typed);
        if !typed.is_empty() {
            self.shells.write(&run, String::from_utf8_lossy(&typed).into_owned(), cx);
        }
        self.read(run, cx);
        self.blink(cx);
        cx.notify();
    }

    fn read(&mut self, run: String, cx: &mut Context<Self>) {
        let messages = self.shells.observe(&run, cx);
        self._read = Some(cx.spawn(async move |this, cx| {
            while let Ok(first) = messages.recv().await {
                // Fold whatever else has arrived, then draw once.
                let mut batch = vec![first];
                while let Ok(more) = messages.try_recv() {
                    batch.push(more);
                }
                if this.update(cx, |this, cx| this.take(batch, cx)).is_err() {
                    return;
                }
            }
        }));
    }

    fn take(&mut self, batch: Vec<SessionRunMessage>, cx: &mut Context<Self>) {
        let replaying = self.replay_until.is_some_and(|until| Instant::now() < until);
        let title = self.screen.title().map(str::to_string);
        for message in batch {
            match message {
                SessionRunMessage::Output { text } => {
                    let replies = self.screen.feed(text.as_bytes(), replaying);
                    if !replies.is_empty() {
                        self.send(replies, cx);
                    }
                }
                SessionRunMessage::Missed { .. } => {
                    // Fleet dropped some: read the whole run again.
                    if let Some(run) = self.run.clone() {
                        self.screen = Screen::new(self.screen.size());
                        self.replay_until = Some(Instant::now() + REPLAY);
                        self.read(run, cx);
                    }
                    return;
                }
                SessionRunMessage::Exited { code, stopped } => {
                    self.exit = Some((code, stopped));
                    self.end(cx);
                }
                SessionRunMessage::Opened { .. } => {}
            }
        }
        if let Some(now) = self.screen.title().filter(|now| Some(*now) != title.as_deref()) {
            cx.emit(PaneEvent::Titled(now.to_string()));
        }
        self.blink_on = true;
        cx.notify();
    }

    fn end(&mut self, cx: &mut Context<Self>) {
        self.phase = Phase::Ended;
        self._blink = None;
        self.screen.feed(b"\x1b[?25l", true);
        cx.emit(PaneEvent::Ended);
        cx.notify();
    }

    /// Bytes for the shell: sent, or held until the run is known. Typing brings the screen back to the end.
    fn send(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        if self.phase == Phase::Ended {
            return;
        }
        match &self.run {
            Some(run) => self.shells.write(run, String::from_utf8_lossy(&bytes).into_owned(), cx),
            None => self.typed.extend(bytes),
        }
    }

    fn typed_bytes(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.screen.scroll_to_end();
        self.screen.unselect();
        self.blink_on = true;
        self.send(bytes, cx);
        cx.notify();
    }

    /// End shell: Fleet stops it, and its end arrives on the socket.
    pub fn end_shell(&mut self, cx: &mut Context<Self>) {
        if let Some(run) = self.run.clone() {
            self.shells.stop(&run, cx);
        }
    }

    /// New shell, in place of the one that ended, under the same id (`terminal.exited.toml`).
    pub fn new_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase != Phase::Ended {
            return;
        }
        self.screen = Screen::new(self.screen.size());
        self.phase = Phase::Opening;
        self.exit = None;
        self.run = None;
        self.asked = true;
        self.replay_until = None;
        let size = self.screen.size();
        self.open(size, cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Whether a program other than the shell holds it, as Fleet lists it, for the close that asks first.
    pub fn run_id(&self) -> Option<&str> {
        self.run.as_deref()
    }

    fn blink(&mut self, cx: &mut Context<Self>) {
        self._blink = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(BLINK).await;
            let going = this.update(cx, |this, cx| {
                this.blink_on = !this.blink_on;
                cx.notify();
            });
            if going.is_err() {
                return;
            }
        }));
    }

    // -- keys and the pointer -------------------------------------------------------------------

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        let mode = self.screen.mode();
        // Shift+PageUp and Shift+PageDown page through the history, as xterm.js's do.
        if key.modifiers.shift && !mode.contains(TermMode::ALT_SCREEN) && matches!(key.key.as_str(), "pageup" | "pagedown") {
            self.screen.scroll_page(key.key == "pageup");
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if let Some(bytes) = keys::bytes(key, mode) {
            self.typed_bytes(bytes, cx);
            cx.stop_propagation();
        }
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.screen.selected() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let bytes = keys::pasted(&text, self.screen.mode());
            self.typed_bytes(bytes, cx);
        }
    }

    /// The cell under `at`, and which half of it.
    fn cell_at(&self, at: gpui::Point<Pixels>) -> Option<(usize, usize, Side)> {
        let (bounds, cell) = self.grid.get()?;
        let x = f32::from(at.x - bounds.origin.x).max(0.0);
        let y = f32::from(at.y - bounds.origin.y).max(0.0);
        let width = f32::from(cell.width).max(1.0);
        let col = (x / width) as usize;
        let row = (y / f32::from(cell.height).max(1.0)) as usize;
        let side = if x - col as f32 * width < width / 2.0 { Side::Left } else { Side::Right };
        Some((row, col, side))
    }

    /// A press, release or wheel turn as the program asked to hear it (SGR, or X10 bytes).
    fn report(&mut self, button: u8, row: usize, col: usize, pressed: bool, cx: &mut Context<Self>) {
        let mode = self.screen.mode();
        let bytes = if mode.contains(TermMode::SGR_MOUSE) {
            format!("\x1b[<{button};{};{}{}", col + 1, row + 1, if pressed { 'M' } else { 'm' }).into_bytes()
        } else {
            let code = if pressed { button } else { 3 };
            vec![0x1b, b'[', b'M', 32 + code, 32u8.saturating_add((col + 1).min(223) as u8), 32u8.saturating_add((row + 1).min(223) as u8)]
        };
        self.send(bytes, cx);
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let Some((row, col, side)) = self.cell_at(event.position) else { return };
        if self.screen.mode().intersects(TermMode::MOUSE_MODE) && !event.modifiers.shift {
            self.report(0, row, col, true, cx);
            return;
        }
        let grain = match event.click_count {
            2 => Grain::Words,
            3.. => Grain::Lines,
            _ => Grain::Cells,
        };
        let at = self.screen.point(row, col);
        self.screen.select_from(at, side, grain);
        self.selecting = true;
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        if let Some((row, col, side)) = self.cell_at(event.position) {
            let at = self.screen.point(row, col);
            self.screen.select_to(at, side);
            cx.notify();
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            self.selecting = false;
            if self.screen.selected().is_none() {
                self.screen.unselect();
            }
            cx.notify();
            return;
        }
        if self.screen.mode().intersects(TermMode::MOUSE_MODE) && !event.modifiers.shift {
            if let Some((row, col, _)) = self.cell_at(event.position) {
                self.report(0, row, col, false, cx);
            }
        }
    }

    fn scroll(&mut self, event: &ScrollEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, cell)) = self.grid.get() else { return };
        self.wheel += f32::from(event.delta.pixel_delta(cell.height).y) / f32::from(cell.height).max(1.0);
        let lines = self.wheel.trunc() as i32;
        if lines == 0 {
            return;
        }
        self.wheel -= lines as f32;
        let mode = self.screen.mode();
        if mode.intersects(TermMode::MOUSE_MODE) {
            if let Some((row, col, _)) = self.cell_at(event.position) {
                for _ in 0..lines.unsigned_abs() {
                    self.report(if lines > 0 { 64 } else { 65 }, row, col, true, cx);
                }
            }
        } else if mode.contains(TermMode::ALT_SCREEN) && mode.contains(TermMode::ALTERNATE_SCROLL) {
            // A full-screen program without the mouse hears the wheel as arrows.
            let arrow = if lines > 0 { "up" } else { "down" };
            let key = gpui::Keystroke::parse(arrow).expect("an arrow");
            let bytes = keys::bytes(&key, mode).unwrap_or_default().repeat(lines.unsigned_abs() as usize);
            self.send(bytes, cx);
        } else {
            self.screen.scroll(lines);
            cx.notify();
        }
    }

    // -- drawing --------------------------------------------------------------------------------

    /// A run's head, as Electron's `runHead`: its mark, the command on one line, and Stop while it runs, then
    /// Run again and Close.
    fn run_head(&self, look: &Look, cx: &mut Context<Self>) -> gpui::AnyElement {
        let command = match &self.which {
            Which::Command(command) => command.clone(),
            _ => String::new(),
        };
        let (icon, colour, said, named) = match (self.phase != Phase::Ended, self.exit) {
            (true, _) => ("square-terminal", token::RUN_RUNNING, "Running".to_string(), "Running"),
            (false, Some((Some(0), _))) => ("circle-check", token::RUN_PASSED, "Exit 0".to_string(), "Passed"),
            (false, Some((Some(code), _))) => ("circle-x", token::RUN_FAILED, format!("Exit {code}"), "Failed"),
            (false, Some((None, false))) => ("circle-x", token::RUN_FAILED, "Ended by a signal".to_string(), "Failed"),
            (false, _) => ("circle-x", token::RUN_STOPPED, "Stopped".to_string(), "Stopped"),
        };
        let running = self.phase != Phase::Ended;
        let mark = div().id("run-mark").flex_none().flex().tooltip(tip(said)).child(if running {
            glyph(icon, look.length(token::SPACE_4), look.color(colour))
                .with_animation("run-pulse", Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(pulsating_between(0.45, 1.)), |mark, at| mark.opacity(at))
                .into_any_element()
        } else {
            glyph(icon, look.length(token::SPACE_4), look.color(colour)).into_any_element()
        });
        let line = div()
            .id("run-command")
            .flex_1()
            .min_w_0()
            .truncate()
            .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(command.clone()))
            .child(command.lines().next().unwrap_or_default().to_string());
        // Each key is named with the command it acts on, as several runs can be drawn at once; the tooltip is the act alone.
        let first = command.lines().next().unwrap_or_default().to_string();
        let keys = div().flex().flex_none().items_center().gap(look.length(token::SPACE_1));
        let keys = if running {
            keys.child(labelled(act("run-stop", "square", "Stop", look).on_click(cx.listener(|this, _, _, cx| this.end_shell(cx))), Role::Button, format!("Stop {first}")))
        } else {
            keys.child(labelled(act("run-again", "rotate-ccw", "Run again", look).on_click(cx.listener(|this, _, window, cx| this.new_shell(window, cx))), Role::Button, format!("Run again {first}")))
                .child(labelled(act("run-close", "x", "Close", look).on_click(cx.listener(|_, _, _, cx| cx.emit(PaneEvent::Closed))), Role::Button, format!("Close {first}")))
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_SM))
            .pl(look.length(token::SPACE_2))
            .pr(look.length(token::SPACE_1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(mark, Role::Image, named))
            .child(line)
            .child(keys)
            .into_any_element()
    }

    fn head(&self, look: &Look, cx: &mut Context<Self>) -> impl IntoElement {
        let running = self.phase != Phase::Ended;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_SM))
            .pl(look.length(token::SPACE_2))
            .pr(look.length(token::SPACE_1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div().flex_1().min_w_0().flex().items_center().gap(look.length(token::SPACE_2)).when(!running, |line| {
                    // Shell ended, and New shell beside the words, as a short text button.
                    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
                    let new = div()
                        .id("terminal-new")
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(gpui::transparent_black())
                        .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .child("New shell")
                        .tab_stop(true)
                        .focus_visible(move |style| style.border_color(ring))
                        .tooltip(|window, cx| {
                            let key = gpui::Keystroke::parse("enter").ok().map(gpui_component::kbd::Kbd::new);
                            gpui_component::tooltip::Tooltip::new("Open a new shell in this tab").key_binding(key).build(window, cx)
                        })
                        .on_click(cx.listener(|this, _, window, cx| this.new_shell(window, cx)));
                    line.child(
                        div()
                            .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(token::FG_MUTED))
                            .child("Shell ended"),
                    )
                    .child(labelled(new, Role::Button, "New shell"))
                }),
            )
            .when(running, |head| {
                let end = act("terminal-end", "power-off", "End shell", look).on_click(cx.listener(|this, _, _, cx| this.end_shell(cx)));
                head.child(labelled(end, Role::Button, "End shell"))
            })
    }

    /// What the screen holds, laid out for one frame: each row's words and colours, the cells
    /// painted behind them, and the cursor.
    fn frame(&self, look: &Look, focused: bool) -> Frame {
        let palette = Palette::of(look);
        let term = self.screen.term();
        let content = term.renderable_content();
        let colors = content.colors;
        let offset = content.display_offset as i32;
        let size = self.screen.size();
        let mut rows: Vec<Row> = (0..size.rows).map(|_| Row::default()).collect();
        let selection = content.selection;
        let mut last: Option<(usize, Hsla, Style)> = None;
        for indexed in content.display_iter {
            let row = (indexed.point.line.0 + offset) as usize;
            let col = indexed.point.column.0;
            if row >= rows.len() {
                continue;
            }
            let cell = &indexed.cell;
            let flags = cell.flags;
            let mut fg = cell.fg;
            if flags.contains(Flags::BOLD) {
                // Bold text in the eight takes their bright form, as xterm.js's `drawBoldTextInBrightColors`.
                if let Color::Named(name) = fg {
                    if (name as usize) < 8 {
                        fg = Color::Indexed(name as u8 + 8);
                    }
                }
            }
            let mut ink = palette.resolve(fg, colors);
            let mut ground = palette.resolve(cell.bg, colors);
            let plain_ground = matches!(cell.bg, Color::Named(NamedColor::Background));
            let mut painted = !plain_ground;
            if flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut ink, &mut ground);
                painted = true;
            }
            if flags.contains(Flags::DIM) {
                ink = ink.opacity(0.66);
            }
            if flags.contains(Flags::HIDDEN) {
                ink = gpui::transparent_black();
            }
            if selection.is_some_and(|range| range.contains(indexed.point)) {
                ground = palette.selection;
                painted = true;
            }
            let line = &mut rows[row];
            if painted {
                match line.backs.last_mut() {
                    Some(back) if back.2 == ground && back.0 + back.1 == col as u16 => back.1 += 1,
                    _ => line.backs.push((col as u16, 1, ground)),
                }
            }
            let ch = if flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) || cell.c == '\0' { ' ' } else { cell.c };
            let style = Style {
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.intersects(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE | Flags::UNDERCURL),
                strike: flags.contains(Flags::STRIKEOUT),
            };
            let start = line.text.len();
            line.text.push(ch);
            if let Some(extra) = cell.zerowidth() {
                line.text.extend(extra.iter());
            }
            let len = line.text.len() - start;
            match &mut last {
                Some((at, held, kept)) if *at == row && *held == ink && *kept == style => {
                    if let Some(run) = line.runs.last_mut() {
                        run.0 += len;
                    }
                }
                _ => {
                    line.runs.push((len, ink, style));
                    last = Some((row, ink, style));
                }
            }
        }
        let cursor = (self.phase == Phase::Running && content.mode.contains(TermMode::SHOW_CURSOR))
            .then(|| {
                let row = content.cursor.point.line.0 + offset;
                (row >= 0 && (row as usize) < rows.len()).then(|| Cursor {
                    row: row as usize,
                    col: content.cursor.point.column.0,
                    shape: if !focused { CursorShape::HollowBlock } else { content.cursor.shape },
                    shown: !focused || self.blink_on,
                })
            })
            .flatten();
        Frame {
            rows,
            cursor,
            background: palette.background,
            cursor_ink: palette.cursor,
            marked: self.marked.clone().filter(|_| focused),
        }
    }
}

/// How a run of words is set.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}

#[derive(Default)]
struct Row {
    text: String,
    /// Byte length, colour and style of each run along `text`.
    runs: Vec<(usize, Hsla, Style)>,
    /// First column, width in cells and colour of each painted ground.
    backs: Vec<(u16, u16, Hsla)>,
}

struct Cursor {
    row: usize,
    col: usize,
    shape: CursorShape,
    shown: bool,
}

struct Frame {
    rows: Vec<Row>,
    cursor: Option<Cursor>,
    background: Hsla,
    cursor_ink: Hsla,
    marked: Option<String>,
}

/// The mono face at a weight and slant.
fn face(family: &SharedString, bold: bool, italic: bool, heavy: f32) -> Font {
    Font {
        family: family.clone(),
        features: FontFeatures::disable_ligatures(),
        fallbacks: None,
        weight: if bold { FontWeight(heavy) } else { FontWeight::NORMAL },
        style: if italic { FontStyle::Italic } else { FontStyle::Normal },
    }
}

/// One cell's size in the mono face: the advance of `M` and the line height.
fn measure(family: &SharedString, text_size: Pixels, line: Pixels, window: &mut Window) -> Size<Pixels> {
    let run = TextRun { len: 1, font: face(family, false, false, 400.0), color: gpui::black(), background_color: None, underline: None, strikethrough: None };
    let shaped = window.text_system().shape_line("M".into(), text_size, &[run], None);
    size(shaped.width.max(px(1.0)), line)
}

/// An act on a pane's head drawn as its glyph alone, which the tooltip names.
fn act(id: &'static str, icon: &'static str, said: &'static str, look: &Look) -> gpui::Stateful<gpui::Div> {
    let hover = look.color(token::BG_HOVER);
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    div()
        .id(id)
        .flex_none()
        .size(look.length(token::H_CONTROL_SM))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .tab_stop(true)
        .focus_visible(move |style| style.border_color(ring))
        .hover(move |style| style.bg(hover))
        .tooltip(tip(said))
        .child(glyph(icon, look.length(token::SPACE_4), look.color(token::FG_MUTED)))
}

impl Render for TerminalPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let focused = self.focus.is_focused(window);
        let frame = self.frame(&look, focused);
        let family = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
        let text_size = look.length(token::TEXT_XS);
        let line = look.length(token::LEADING_XS);
        let heavy = look.number(token::WEIGHT_HEADING);
        let me = cx.entity();
        let grid = self.grid.clone();
        let focus = self.focus.clone();
        let name: SharedString = self.terminal.as_deref().map_or_else(|| "Terminal".into(), |id| format!("Terminal {}", id.trim_start_matches('t')).into());
        let is_run = self.is_run();

        let screen = canvas(
            {
                let me = me.clone();
                let family = family.clone();
                move |bounds, window, cx| {
                    let cell = measure(&family, text_size, line, window);
                    let cols = (f32::from(bounds.size.width) / f32::from(cell.width)).floor().max(1.0) as u16;
                    let rows = if is_run { RUN_ROWS } else { (f32::from(bounds.size.height) / f32::from(cell.height)).floor().max(1.0) as u16 };
                    let cells = Cells { cols, rows };
                    grid.set(Some((bounds, cell)));
                    let me = me.clone();
                    window.defer(cx, move |_, cx| me.update(cx, |pane, cx| pane.fit(cells, cell, cx)));
                    cell
                }
            },
            move |bounds, cell, window, cx| {
                window.handle_input(&focus, ElementInputHandler::new(bounds, me.clone()), cx);
                paint(&frame, bounds, cell, &family, text_size, heavy, window, cx);
            },
        )
        .size_full();

        let well = labelled(
            div()
                .id("terminal-screen")
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .bg(look.color(token::BG_SUNKEN))
                .cursor(gpui::CursorStyle::IBeam)
                .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
                .on_mouse_move(cx.listener(Self::mouse_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
                .on_scroll(cx.listener(Self::scroll))
                // A run shows as many of its rows as it has used.
                .when(is_run, |well| well.flex_none().h(line * self.screen.used_rows() as f32 + look.length(token::SPACE_2) * 2.))
                .child(screen),
            Role::TextInput,
            name,
        );

        div()
            .id("terminal")
            .key_context({
                let mut context = gpui::KeyContext::default();
                context.add(KEYS);
                if self.phase != Phase::Ended {
                    context.add(SHELL);
                }
                context
            })
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, key: &crate::keymap::Act, window, cx| {
                // Bound only once the shell has ended (`!shell`): Enter opens a new one in its place.
                if key.is("terminal_new_shell") && this.phase == Phase::Ended && !this.is_run() {
                    this.new_shell(window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(|this, _: &gpui_component::input::Copy, _, cx| this.copy(cx)))
            .on_action(cx.listener(|this, _: &gpui_component::input::Paste, _, cx| this.paste(cx)))
            .on_action(cx.listener(|this, _: &gpui_component::input::SelectAll, _, cx| {
                this.screen.select_all();
                cx.notify();
            }))
            .when(!is_run, |root| root.size_full())
            // `.armada-terminal`: a run under its block is a framed well.
            .when(is_run, |root| root.w_full().border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).rounded(look.length(token::RADIUS_MD)).overflow_hidden())
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .bg(look.color(token::BG_SUNKEN))
            .child(if is_run { self.run_head(&look, cx) } else { self.head(&look, cx).into_any_element() })
            .child(well)
    }
}

#[allow(clippy::too_many_arguments)]
fn paint(frame: &Frame, bounds: Bounds<Pixels>, cell: Size<Pixels>, family: &SharedString, text_size: Pixels, heavy: f32, window: &mut Window, cx: &mut App) {
    window.paint_quad(fill(bounds, frame.background));
    let origin = bounds.origin;
    for (at, row) in frame.rows.iter().enumerate() {
        let top = origin.y + cell.height * at as f32;
        for (col, width, colour) in &row.backs {
            let at = point(origin.x + cell.width * *col as f32, top);
            window.paint_quad(fill(Bounds::new(at, size(cell.width * *width as f32, cell.height)), *colour));
        }
        if row.text.trim_end().is_empty() {
            continue;
        }
        let runs: Vec<TextRun> = row
            .runs
            .iter()
            .map(|(len, colour, style)| TextRun {
                len: *len,
                font: face(family, style.bold, style.italic, heavy),
                color: *colour,
                background_color: None,
                underline: style.underline.then(|| UnderlineStyle { thickness: px(1.0), color: Some(*colour), wavy: false }),
                strikethrough: style.strike.then(|| StrikethroughStyle { thickness: px(1.0), color: Some(*colour) }),
            })
            .collect();
        let shaped = window.text_system().shape_line(row.text.clone().into(), text_size, &runs, Some(cell.width));
        let _ = shaped.paint(point(origin.x, top), cell.height, gpui::TextAlign::Left, None, window, cx);
    }
    if let Some(cursor) = frame.cursor.as_ref().filter(|cursor| cursor.shown) {
        let at = point(origin.x + cell.width * cursor.col as f32, origin.y + cell.height * cursor.row as f32);
        let whole = Bounds::new(at, cell);
        match cursor.shape {
            CursorShape::Underline => window.paint_quad(fill(Bounds::new(point(at.x, at.y + cell.height - px(2.0)), size(cell.width, px(2.0))), frame.cursor_ink)),
            CursorShape::Beam => window.paint_quad(fill(Bounds::new(at, size(px(2.0), cell.height)), frame.cursor_ink)),
            CursorShape::HollowBlock => window.paint_quad(gpui::outline(whole, frame.cursor_ink, gpui::BorderStyle::Solid)),
            CursorShape::Hidden => {}
            CursorShape::Block => {
                window.paint_quad(fill(whole, frame.cursor_ink));
                // The letter under a block cursor in the well's colour (`cursorAccent`).
                let row = &frame.rows[cursor.row];
                if let Some(ch) = row.text.chars().nth(cursor.col).filter(|ch| !ch.is_whitespace()) {
                    let text: SharedString = ch.to_string().into();
                    let run = TextRun { len: text.len(), font: face(family, false, false, heavy), color: frame.background, background_color: None, underline: None, strikethrough: None };
                    let shaped = window.text_system().shape_line(text, text_size, &[run], Some(cell.width));
                    let _ = shaped.paint(at, cell.height, gpui::TextAlign::Left, None, window, cx);
                }
            }
        }
        if let Some(marked) = &frame.marked {
            let run = TextRun {
                len: marked.len(),
                font: face(family, false, false, heavy),
                color: frame.cursor_ink,
                background_color: Some(frame.background),
                underline: Some(UnderlineStyle { thickness: px(1.0), color: Some(frame.cursor_ink), wavy: false }),
                strikethrough: None,
            };
            let shaped = window.text_system().shape_line(marked.clone().into(), text_size, &[run], Some(cell.width));
            let _ = shaped.paint(at, cell.height, gpui::TextAlign::Left, None, window, cx);
        }
    }
}

/// Text typed and composed: what an input method commits is typed into the shell, as xterm.js's
/// hidden textarea sends it.
impl EntityInputHandler for TerminalPane {
    fn text_for_range(&mut self, _: Range<usize>, _: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        None
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: 0..0, reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|marked| 0..marked.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(&mut self, _: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        if !text.is_empty() {
            self.typed_bytes(text.as_bytes().to_vec(), cx);
        }
    }

    fn replace_and_mark_text_in_range(&mut self, _: Option<Range<usize>>, text: &str, _: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = (!text.is_empty()).then(|| text.to_string());
        cx.notify();
    }

    fn bounds_for_range(&mut self, _: Range<usize>, _: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let (bounds, cell) = self.grid.get()?;
        let cursor = self.screen.term().grid().cursor.point;
        let row = cursor.line.0 + self.screen.scrolled() as i32;
        Some(Bounds::new(point(bounds.origin.x + cell.width * cursor.column.0 as f32, bounds.origin.y + cell.height * row.max(0) as f32), cell))
    }

    fn character_index_for_point(&mut self, _: gpui::Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        None
    }
}

/// Whether `pane` is the one holding the keys.
pub fn holds_keys(pane: &Entity<TerminalPane>, window: &Window, cx: &App) -> bool {
    pane.read(cx).focus.is_focused(window)
}
