//! The box that asks a project's librarian: his words, the files he dropped or pasted, and the screen he asked from go to
//! Fleet on Enter, and the librarian does at most one thing with them. A card under the box says what it did, with Undo, and
//! Move for a filed doc. The dashboard mounts it in its header with the board on screen; the Features page mounts it with a
//! context of its own. `docs/concepts/feature-catalog.md`, *Asking the librarian*.

use gpui::{deferred, div, prelude::*, AnyElement, ClipboardEntry, ClipboardItem, Context, Entity, EventEmitter, ExternalPaths, FocusHandle, Focusable as _, KeyDownEvent, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{AskedBoard, BoardLayout, Instant, LibrarianAct, LibrarianAnswered, LibrarianAskUndone};

use crate::fleet::FleetConnection;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, ring_colour};
use crate::settings::wire::Refused;
use crate::titlebar::parts::{card, glyph, spinner, tip, Btn, Weight};

/// Asks the box offers when it takes the keys, so a person sees what it could be asked.
pub const EXAMPLE_ASKS: [&str; 5] = ["Show errors as a graph", "Add a Docs block", "File this link under a feature", "What runs this project?", "Hide Checks on this board"];

/// Where the dashboard's box keeps what was typed. A box of another context keeps it under this and its context.
pub const KEEP_ASK: &str = "project.ask";

/// The context of the dashboard's box.
pub const DASHBOARD: &str = "dashboard";

/// Where an ask goes: Fleet, or the walks' and tests' stand-in.
pub enum AskSource {
    Live(Entity<FleetConnection>),
    Mock(Box<MockLibrarian>),
}

/// The librarian of the mock Fleet: it keeps what it was sent and answers at once.
#[derive(Default)]
pub struct MockLibrarian {
    pub sent: Vec<ipc::AskLibrarian>,
    pub undone: Vec<Instant>,
    pub refiled: Vec<(Instant, String)>,
    /// The answer the next ask gets; none answers as the mock does.
    pub next: Option<Result<LibrarianAnswered, Refused>>,
}

impl MockLibrarian {
    fn answer(&mut self, ask: ipc::AskLibrarian) -> Result<LibrarianAnswered, Refused> {
        self.sent.push(ask.clone());
        if let Some(next) = self.next.take() {
            return next;
        }
        let calls = u32::try_from(self.sent.len()).unwrap_or(u32::MAX);
        let asked_at = Instant::carried(format!("2026-11-24T01:{:02}:00Z", calls % 60));
        let act = ask.files.first().map(|file| LibrarianAct::FiledDoc { target: file.clone(), title: short_name(file), feature: "start-a-job".into(), feature_title: "Start a Job".into() });
        let reply = match &act {
            Some(LibrarianAct::FiledDoc { title, feature_title, .. }) => format!("Filed {title} under {feature_title}"),
            _ => "The Run block starts the app on port 5173".to_string(),
        };
        Ok(LibrarianAnswered { project: ask.project, asked_at, reply, act, calls, limit: 40 })
    }
}

/// What the box tells the screen it sits in.
pub enum AskBoxEvent {
    /// Fleet keeps this board now, none being the preset: the board draws it at once and sends nothing.
    Board(Option<BoardLayout>),
    /// An ask's board change was taken back: the board reads what Fleet keeps.
    BoardAgain,
    /// The might-lists were ranked by an ask, or its Undo put them back: the Issues block reads them again.
    IssuesAgain,
}

impl EventEmitter<AskBoxEvent> for AskBox {}

/// The list of example asks under the box: open while the box has the keys, and the row the arrows are on.
#[derive(Default)]
struct Examples {
    open: bool,
    at: Option<usize>,
}

/// What the card under the box says.
enum Reply {
    Answered(LibrarianAnswered),
    /// Undo took the act back: Fleet's line of what it took back.
    Undone(String),
    /// Fleet refused the ask, the Undo or the Move.
    Refused(String),
}

/// The features a filed doc can move to, under the card, and the row the arrows are on.
struct MoveList {
    features: Option<Result<Vec<(String, String)>, String>>,
    at: usize,
    focus: FocusHandle,
}

pub struct AskBox {
    source: AskSource,
    project: Option<String>,
    /// The box's own context, and the one an ask of [`AskBox::prefill`] goes with instead, once.
    context: String,
    once: Option<String>,
    keep: String,
    pub(crate) input: Entity<InputState>,
    examples: Examples,
    /// Paths and links he dropped or pasted, as he gave them.
    files: Vec<String>,
    board: Option<AskedBoard>,
    /// The board the last ask went with, which a mock Undo of a board change puts back.
    asked_board: Option<AskedBoard>,
    waiting: bool,
    /// An Undo or a Move is on its way.
    acting: bool,
    reply: Option<Reply>,
    /// The asks spent tonight and the night's limit, from the last answer.
    spent: Option<(u32, u32)>,
    moving: Option<MoveList>,
    pub(crate) undo_focus: FocusHandle,
    move_focus: FocusHandle,
    task: Option<Task<()>>,
    read: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

/// The line a refusal shows in the card: the librarian's own codes said plainly, anything else as Fleet said it.
pub(crate) fn refusal_line(why: &Refused) -> String {
    match why.code.as_deref() {
        Some("fleet.librarian_asks_spent") => "Tonight's asks are spent".to_string(),
        Some("fleet.librarian_ask_blank") => "The ask has no words".to_string(),
        Some("fleet.librarian_ask_unknown") => "Fleet no longer holds that ask".to_string(),
        _ => why.said.clone(),
    }
}

/// What a chip calls a path or a link: its last part.
fn short_name(file: &str) -> String {
    let bare = file.split_once("://").map_or(file, |(_, rest)| rest).trim_end_matches('/');
    bare.rsplit('/').next().filter(|last| !last.is_empty()).unwrap_or(bare).to_string()
}

/// The paths and links a pasted text is, where it is nothing else: each word one of them.
fn pasted_files(text: &str) -> Option<Vec<String>> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let file = |word: &&str| word.starts_with('/') || word.starts_with("~/") || word.starts_with("http://") || word.starts_with("https://") || word.starts_with("file://");
    (!words.is_empty() && words.iter().all(file)).then(|| words.into_iter().map(str::to_string).collect())
}

/// Every feature of a library, in its order, by slug and title.
fn features_of(library: &ipc::FeatureLibrary) -> Vec<(String, String)> {
    library.areas.iter().flat_map(|area| &area.features).map(|one| (one.slug.clone(), one.title.clone())).collect()
}

impl AskBox {
    pub fn new(source: AskSource, project: Option<String>, context: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let keep = if context == DASHBOARD { KEEP_ASK.to_string() } else { format!("{KEEP_ASK}.{context}") };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask the librarian"));
        if let Some(words) = crate::drafts::kept(&keep, cx) {
            input.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let kept = keep.clone();
        let subs = vec![cx.subscribe_in(&input, window, move |this, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(&kept, cx);
                } else {
                    crate::drafts::keep(&kept, &text, cx);
                }
            }
            // The examples are offered while the box has the keys and no reply stands under it.
            InputEvent::Focus => {
                if this.reply.is_none() && !this.waiting {
                    this.examples = Examples { open: true, at: None };
                    cx.notify();
                }
            }
            InputEvent::Blur => {
                this.examples = Examples::default();
                cx.notify();
            }
            // Enter takes the example the arrows are on, and sends otherwise.
            InputEvent::PressEnter { .. } => match this.examples.at.filter(|_| this.examples.open) {
                Some(at) => this.use_example(at, window, cx),
                None => this.send(window, cx),
            },
        })];
        AskBox {
            source,
            project,
            context: context.to_string(),
            once: None,
            keep,
            input,
            examples: Examples::default(),
            files: Vec::new(),
            board: None,
            asked_board: None,
            waiting: false,
            acting: false,
            reply: None,
            spent: None,
            moving: None,
            undo_focus: cx.focus_handle().tab_stop(true),
            move_focus: cx.focus_handle().tab_stop(true),
            task: None,
            read: None,
            _subs: subs,
        }
    }

    /// The project asks go to. Another project's reply does not stand under the box.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project != project {
            self.project = project;
            self.reply = None;
            self.moving = None;
            cx.notify();
        }
    }

    /// The board on screen, which an ask from the dashboard goes with. Nothing of it is drawn here.
    pub fn set_board(&mut self, board: Option<AskedBoard>) {
        self.board = board;
    }

    /// Another screen's "Fix with the librarian": `words` are in the box, the next ask goes with `context`, and the box has the
    /// keys. A send or Esc puts the box's own context back.
    #[allow(dead_code)] // Live metrics' "Fix with the librarian" calls it.
    pub fn prefill(&mut self, words: &str, context: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.once = (context != self.context).then(|| context.to_string());
        self.input.update(cx, |state, cx| {
            state.set_value(words.to_string(), window, cx);
            state.focus(window, cx);
        });
        crate::drafts::keep(&self.keep, words, cx);
        cx.notify();
    }

    /// The context the next ask goes with.
    #[allow(dead_code)] // Read by the tests and by the screens that prefill.
    pub fn asking_from(&self) -> &str {
        self.once.as_deref().unwrap_or(&self.context)
    }

    /// The stand-in librarian, on the mock.
    #[cfg(test)]
    pub(crate) fn mock(&mut self) -> Option<&mut MockLibrarian> {
        match &mut self.source {
            AskSource::Mock(mock) => Some(mock),
            AskSource::Live(_) => None,
        }
    }

    fn port(&self, cx: &gpui::App) -> Option<u16> {
        match &self.source {
            AskSource::Live(fleet) => fleet.read(cx).held().port(),
            AskSource::Mock(_) => None,
        }
    }

    // ---- the examples ----------------------------------------------------------------------------

    /// A press or Enter on an example: it is the box's words, and the box keeps the keys.
    fn use_example(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = EXAMPLE_ASKS.get(at) else { return };
        self.examples = Examples::default();
        self.input.update(cx, |state, cx| state.set_value(*text, window, cx));
        crate::drafts::keep(&self.keep, text, cx);
        cx.notify();
    }

    /// ↓ opens the examples where they are shut, and ↑ ↓ move along them; Esc puts them away, then the card, with the keys still
    /// in the box. Tab goes to the card's first button where it has one.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let last = EXAMPLE_ASKS.len() - 1;
        match (event.keystroke.key.as_str(), self.examples.open) {
            ("down", false) if !self.waiting => self.examples = Examples { open: true, at: Some(0) },
            ("down", true) => self.examples.at = Some(self.examples.at.map_or(0, |at| (at + 1).min(last))),
            ("up", true) => self.examples.at = self.examples.at.and_then(|at| at.checked_sub(1)),
            ("escape", true) => self.examples = Examples::default(),
            ("escape", false) if self.reply.is_some() || self.once.is_some() => self.close_card(window, cx),
            ("tab", false) if !event.keystroke.modifiers.shift && self.has_buttons() && self.input.read(cx).focus_handle(cx).is_focused(window) => window.focus(&self.undo_focus, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Esc: the card goes, the box's own context is back, and the keys are in the box.
    fn close_card(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reply = None;
        self.moving = None;
        self.once = None;
        self.input.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn has_buttons(&self) -> bool {
        matches!(&self.reply, Some(Reply::Answered(answer)) if answer.act.is_some())
    }

    // ---- files -----------------------------------------------------------------------------------

    fn add_files(&mut self, files: impl IntoIterator<Item = String>, cx: &mut Context<Self>) {
        for file in files {
            if !self.files.contains(&file) {
                self.files.push(file);
            }
        }
        cx.notify();
    }

    /// A paste of files, or of text that is only paths and links, puts them on the box as chips; any other paste is words.
    fn pasted(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let dropped: Vec<String> = item
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                ClipboardEntry::ExternalPaths(paths) => Some(paths.paths().iter().map(|path| path.to_string_lossy().into_owned()).collect::<Vec<_>>()),
                _ => None,
            })
            .flatten()
            .collect();
        let files = if dropped.is_empty() { item.text().as_deref().and_then(pasted_files) } else { Some(dropped) };
        match files {
            Some(files) if !self.waiting => {
                self.add_files(files, cx);
                true
            }
            _ => false,
        }
    }

    fn remove_file(&mut self, file: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.files.retain(|one| one != file);
        self.input.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    // ---- asking ----------------------------------------------------------------------------------

    /// Enter: the words, the files, the context and the board go to the librarian, and the box waits, read-only, for its answer.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.waiting {
            return;
        }
        let words = self.input.read(cx).value().trim().to_string();
        let Some(project) = self.project.clone() else { return };
        if words.is_empty() && self.files.is_empty() {
            return;
        }
        let context = self.once.take().unwrap_or_else(|| self.context.clone());
        let ask = ipc::AskLibrarian { project, context, words, files: self.files.clone(), board: self.board.clone() };
        self.asked_board = ask.board.clone();
        self.examples = Examples::default();
        self.reply = None;
        self.moving = None;
        self.waiting = true;
        let task = match &mut self.source {
            AskSource::Live(fleet) => match fleet.read(cx).held().port() {
                Some(port) => super::wire::ask_librarian(port, &ask, cx),
                None => crate::settings::wire::unreachable(),
            },
            AskSource::Mock(mock) => Task::ready(mock.answer(ask)),
        };
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| this.answered(answer, window, cx));
        }));
        cx.notify();
    }

    /// The answer: what the librarian did is under the box, and the box is empty for the next ask. A refusal leaves his words.
    fn answered(&mut self, answer: Result<LibrarianAnswered, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        self.waiting = false;
        match answer {
            Ok(answer) => {
                self.spent = Some((answer.calls, answer.limit));
                self.files.clear();
                self.input.update(cx, |state, cx| state.set_value("", window, cx));
                crate::drafts::clear(&self.keep, cx);
                match &answer.act {
                    Some(LibrarianAct::Board { layout }) => cx.emit(AskBoxEvent::Board(Some(layout.clone()))),
                    Some(LibrarianAct::Ranked) => cx.emit(AskBoxEvent::IssuesAgain),
                    _ => {}
                }
                self.reply = Some(Reply::Answered(answer));
            }
            Err(why) => self.reply = Some(Reply::Refused(refusal_line(&why))),
        }
        cx.notify();
    }

    /// Undo: the act of the ask on the card is taken back, and the card says what was.
    fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(Reply::Answered(answer)) = &self.reply else { return };
        let Some(act) = answer.act.clone() else { return };
        if self.acting {
            return;
        }
        let body = ipc::UndoLibrarianAsk { project: answer.project.clone(), asked_at: answer.asked_at.clone() };
        let task = match &mut self.source {
            AskSource::Live(fleet) => match fleet.read(cx).held().port() {
                Some(port) => super::wire::undo_librarian_ask(port, &body, cx),
                None => crate::settings::wire::unreachable(),
            },
            AskSource::Mock(mock) => {
                mock.undone.push(body.asked_at.clone());
                Task::ready(Ok(LibrarianAskUndone { project: body.project, asked_at: body.asked_at, reply: "Put back as it was".to_string() }))
            }
        };
        self.acting = true;
        self.moving = None;
        self.task = Some(cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| this.undone(act, answer, cx));
        }));
        cx.notify();
    }

    fn undone(&mut self, act: LibrarianAct, answer: Result<LibrarianAskUndone, Refused>, cx: &mut Context<Self>) {
        self.acting = false;
        match answer {
            Ok(done) => {
                match act {
                    // Fleet put back the board it kept before; the mock puts back the one the ask went with.
                    LibrarianAct::Board { .. } => match &self.source {
                        AskSource::Live(_) => cx.emit(AskBoxEvent::BoardAgain),
                        AskSource::Mock(_) => cx.emit(AskBoxEvent::Board(self.asked_board.as_ref().filter(|board| !board.preset).map(|board| board.layout.clone()))),
                    },
                    LibrarianAct::Ranked => cx.emit(AskBoxEvent::IssuesAgain),
                    LibrarianAct::FiledDoc { .. } => {}
                }
                self.reply = Some(Reply::Undone(done.reply));
            }
            Err(why) => self.reply = Some(Reply::Refused(refusal_line(&why))),
        }
        cx.notify();
    }

    // ---- Move ------------------------------------------------------------------------------------

    /// Move: the project's features open under the card, read off the main thread, with the keys on them.
    fn open_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.acting {
            return;
        }
        let Some(project) = self.project.clone() else { return };
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.moving = Some(MoveList { features: None, at: 0, focus });
        let task = match &self.source {
            AskSource::Live(_) => match self.port(cx) {
                Some(port) => super::wire::library(port, &project, cx),
                None => crate::settings::wire::unreachable(),
            },
            AskSource::Mock(_) => Task::ready(Ok(super::mock::library_of(&project))),
        };
        let filed = match &self.reply {
            Some(Reply::Answered(LibrarianAnswered { act: Some(LibrarianAct::FiledDoc { feature, .. }), .. })) => feature.clone(),
            _ => String::new(),
        };
        self.read = Some(cx.spawn(async move |this, cx| {
            let read = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(moving) = this.moving.as_mut() {
                    // The feature it is filed under is not a place to move it to.
                    moving.features = Some(read.map(|library| features_of(&library).into_iter().filter(|(slug, _)| *slug != filed).collect()).map_err(|why| why.said));
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    fn close_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.moving.take().is_some() {
            window.focus(&self.move_focus, cx);
            cx.notify();
        }
    }

    fn on_move_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(moving) = self.moving.as_mut() else { return };
        let count = moving.features.as_ref().and_then(|read| read.as_ref().ok()).map_or(0, Vec::len);
        match event.keystroke.key.as_str() {
            "escape" => self.close_move(window, cx),
            "down" => moving.at = (moving.at + 1).min(count.saturating_sub(1)),
            "up" => moving.at = moving.at.saturating_sub(1),
            "enter" => {
                let at = moving.at;
                self.refile(at, window, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// A feature picked: the doc the ask filed moves under it, and the card says where.
    fn refile(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((slug, title)) = self.moving.as_ref().and_then(|moving| moving.features.as_ref()?.as_ref().ok()?.get(at).cloned()) else { return };
        let Some(Reply::Answered(answer)) = &self.reply else { return };
        let body = ipc::RefileLibrarianAsk { project: answer.project.clone(), asked_at: answer.asked_at.clone(), feature: slug.clone() };
        let before = answer.clone();
        let task = match &mut self.source {
            AskSource::Live(fleet) => match fleet.read(cx).held().port() {
                Some(port) => super::wire::refile_librarian_ask(port, &body, cx),
                None => crate::settings::wire::unreachable(),
            },
            AskSource::Mock(mock) => {
                mock.refiled.push((body.asked_at.clone(), slug.clone()));
                let mut moved = before;
                if let Some(LibrarianAct::FiledDoc { title: doc, feature, feature_title, .. }) = moved.act.as_mut() {
                    moved.reply = format!("Moved {doc} to {title}");
                    *feature = slug;
                    *feature_title = title;
                }
                Task::ready(Ok(moved))
            }
        };
        self.moving = None;
        self.acting = true;
        window.focus(&self.move_focus, cx);
        self.task = Some(cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.acting = false;
                this.reply = Some(match answer {
                    Ok(answer) => Reply::Answered(answer),
                    Err(why) => Reply::Refused(refusal_line(&why)),
                });
                cx.notify();
            });
        }));
        cx.notify();
    }

    // ---- drawing ---------------------------------------------------------------------------------

    fn file_chip(&self, at: usize, file: &str, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let name = short_name(file);
        let gone = file.to_string();
        labelled(
            div()
                .id(SharedString::from(format!("ask-file-{at}")))
                .flex_none()
                .max_w(look.length(token::SPACE_12) * 1.5)
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_1))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .bg(look.color(token::ASK_FEATURES).opacity(0.16))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .tooltip(tip(file.to_string()))
                .on_click(cx.listener(move |this, _, window, cx| this.remove_file(&gone, window, cx)))
                .child(div().min_w_0().truncate().child(SharedString::from(name.clone())))
                .child(glyph("x", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
            Role::Button,
            format!("Remove {name}"),
        )
        .into_any_element()
    }

    fn examples(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let rows = EXAMPLE_ASKS.iter().enumerate().map(|(at, text)| {
            let on = self.examples.at == Some(at);
            labelled(
                div()
                    .id(SharedString::from(format!("project-ask-example-{at}")))
                    .flex()
                    .items_center()
                    .h(look.length(token::H_ROW_ACTION))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .when(on, |row| row.bg(look.color(token::BG_HOVER)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(move |style| style.border_color(ring))
                    // A press keeps the keys in the box, which a press on anything else would take.
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.use_example(at, window, cx)))
                    .child(div().min_w_0().truncate().child(*text)),
                Role::MenuItem,
                *text,
            )
        });
        labelled(card(look).id("project-ask-examples").p(look.length(token::SPACE_1)).children(rows), Role::Menu, "Example asks").into_any_element()
    }

    /// The card under the box: what the librarian did, with Undo and Move; its answer; or why it did nothing.
    fn reply_card(&self, reply: &Reply, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let size = look.length(token::SPACE_4);
        let (mark, line) = match reply {
            Reply::Answered(answer) if answer.act.is_some() => (Some(("circle-check", token::STATUS_COMPLETED_SUCCESS, "Done")), answer.reply.clone()),
            Reply::Answered(answer) => (None, answer.reply.clone()),
            Reply::Undone(line) => (None, line.clone()),
            Reply::Refused(line) => (Some(("triangle-alert", token::DEGRADED, "Not done")), line.clone()),
        };
        let mark = mark.map(|(name, hue, said)| labelled(div().id("ask-reply-mark").flex_none().pt(look.length(token::SPACE_1) / 2.).tooltip(tip(said)).child(glyph(name, size, look.color(hue))), Role::Image, said));
        let head = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .children(mark)
            .child(labelled(div().id("ask-reply-line").flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).child(SharedString::from(line.clone())), Role::Status, line));
        let buttons = match reply {
            Reply::Answered(answer) if answer.act.is_some() => {
                let undo = labelled(
                    Btn::new("ask-undo", "Undo", Weight::Secondary)
                        .icon("undo-2")
                        .pending(self.acting)
                        .draw(look, cx)
                        .track_focus(&self.undo_focus)
                        .tooltip(tip("Take back what the librarian did"))
                        .when(!self.acting, |button| button.on_click(cx.listener(|this, _, _, cx| this.undo(cx)))),
                    Role::Button,
                    "Undo",
                );
                let filed = matches!(answer.act, Some(LibrarianAct::FiledDoc { .. }));
                let move_to = filed.then(|| {
                    labelled(
                        Btn::new("ask-move", "Move", Weight::Secondary)
                            .icon("folder-input")
                            .disabled(self.acting)
                            .draw(look, cx)
                            .track_focus(&self.move_focus)
                            .tooltip(tip("File it under another feature"))
                            .when(!self.acting, |button| button.on_click(cx.listener(|this, _, window, cx| this.open_move(window, cx)))),
                        Role::Button,
                        "Move",
                    )
                });
                Some(div().flex().items_center().gap(look.length(token::SPACE_2)).child(undo).children(move_to))
            }
            _ => None,
        };
        labelled(
            card(look)
                .id("ask-reply")
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && this.moving.is_none() {
                        cx.stop_propagation();
                        this.close_card(window, cx);
                    }
                }))
                .child(head)
                .children(buttons),
            Role::Group,
            "Librarian",
        )
        .into_any_element()
    }

    /// The features a filed doc can move to, under the card: ↑ ↓ and Enter pick one, Esc shuts them.
    fn move_list(&self, moving: &MoveList, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let body: AnyElement = match &moving.features {
            None => {
                let bones = (0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3.0 - at as f32 * 0.5), look.length(token::SPACE_3)));
                labelled(div().id("ask-move-loading").flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_2)).children(bones), Role::Image, "Loading").into_any_element()
            }
            Some(Err(why)) => div().p(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(why.clone())).into_any_element(),
            Some(Ok(features)) => div()
                .flex()
                .flex_col()
                .children(features.iter().enumerate().map(|(at, (_, title))| {
                    let on = at == moving.at;
                    labelled(
                        div()
                            .id(SharedString::from(format!("ask-move-{at}")))
                            .flex()
                            .items_center()
                            .h(look.length(token::H_ROW_ACTION))
                            .px(look.length(token::SPACE_3))
                            .rounded(look.length(token::RADIUS_MD))
                            .text_size(look.length(token::TEXT_XS))
                            .when(on, |row| row.bg(look.color(token::BG_HOVER)))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(move |style| style.border_color(ring))
                            .on_click(cx.listener(move |this, _, window, cx| this.refile(at, window, cx)))
                            .child(div().min_w_0().truncate().child(SharedString::from(title.clone()))),
                        Role::ListBoxOption,
                        title.clone(),
                    )
                }))
                .into_any_element(),
        };
        labelled(
            card(look)
                .id("ask-move-list")
                .track_focus(&moving.focus)
                .on_key_down(cx.listener(Self::on_move_key))
                .mt(look.length(token::SPACE_1))
                .max_h(look.length(token::SPACE_12) * 5.0)
                .overflow_y_scroll()
                .p(look.length(token::SPACE_1))
                .child(body),
            Role::ListBox,
            "Features",
        )
        .into_any_element()
    }
}

impl Render for AskBox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let mark = if self.waiting {
            spinner("ask-waiting", look.length(token::SPACE_4), look.color(token::ASK_FEATURES), cx).into_any_element()
        } else {
            glyph("sparkles", look.length(token::SPACE_4), look.color(token::ASK_FEATURES)).into_any_element()
        };
        let chips: Vec<AnyElement> = self.files.clone().iter().enumerate().map(|(at, file)| self.file_chip(at, file, &look, cx)).collect();
        let entity = cx.entity().downgrade();
        let field = Input::new(&self.input).appearance(false).disabled(self.waiting).on_paste(move |item, _, cx| entity.update(cx, |this, cx| this.pasted(item, cx)).unwrap_or(false));
        let spent = self.spent.map(|(calls, limit)| format!("{calls} of {limit} asks tonight"));
        let under: Option<AnyElement> = if self.examples.open {
            Some(self.examples(&look, cx))
        } else {
            self.reply.as_ref().map(|reply| {
                let card = self.reply_card(reply, &look, cx);
                let list = self.moving.as_ref().map(|moving| self.move_list(moving, &look, cx));
                div().flex().flex_col().child(card).children(list).into_any_element()
            })
        };
        let under = under.map(|under| deferred(div().absolute().top_full().mt(look.length(token::SPACE_1)).left_0().w_full().child(under)).with_priority(2));
        div()
            .relative()
            .flex_none()
            .child(labelled(
                div()
                    .id("project-ask")
                    .w(look.length(token::SPACE_12) * 4.0)
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .when_some(spent, |field, spent| field.tooltip(tip(spent)))
                    .capture_key_down(cx.listener(Self::on_key))
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.add_files(paths.paths().iter().map(|path| path.to_string_lossy().into_owned()), cx)))
                    .child(mark)
                    .children(chips)
                    .child(div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_XS)).child(field)),
                Role::TextInput,
                "Ask the librarian",
            ))
            .children(under)
    }
}

#[cfg(test)]
mod tests {
    use super::{pasted_files, short_name};

    #[test]
    fn a_paste_of_only_paths_and_links_is_files_and_any_other_paste_is_words() {
        assert_eq!(pasted_files("/Users/n/notes.md https://example.com/a/b"), Some(vec!["/Users/n/notes.md".to_string(), "https://example.com/a/b".to_string()]));
        assert_eq!(pasted_files("~/x.txt\n"), Some(vec!["~/x.txt".to_string()]));
        assert_eq!(pasted_files("see /Users/n/notes.md"), None, "words around a path are words");
        assert_eq!(pasted_files("  "), None);
    }

    #[test]
    fn a_chip_names_a_file_or_a_link_by_its_last_part() {
        assert_eq!(short_name("/Users/n/docs/plan.md"), "plan.md");
        assert_eq!(short_name("https://example.com/guide/"), "guide");
        assert_eq!(short_name("https://example.com"), "example.com");
    }
}
