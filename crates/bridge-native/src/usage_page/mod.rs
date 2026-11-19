//! The Usage page: where the burn was wasted, kind by kind, largest first. A kind opens a panel under the list with its
//! fix and the Jobs it hit.
//!
//! It leads with waste, over the window the switch names. `model.rs` works out what the page draws from the reading and
//! nothing there touches the screen; the reading is mock data until Fleet serves it (`mock.rs`). Like Kit and Features it
//! is a place of the Operator's screen (`operator/screen.rs`), reached by name from the palette: Classic has no tab for it.

mod mock;
pub mod model;
#[cfg(test)]
mod tests;
mod view;
pub mod walk;

use std::collections::HashMap;

use gpui::{prelude::*, App, Context, FocusHandle, Global, KeyDownEvent, ListAlignment, ListState, Window};

use crate::keymap::Act;
use model::{Board, JobLine, Selection, Span};

/// Set once a Usage page exists in the app, so the palette offers the row that brings it up and no other build draws one that does nothing.
pub struct Mounted;

impl Global for Mounted {}

/// A reading that may not have answered yet.
enum Load {
    Reading,
    Read(Board),
}

pub struct UsageScreen {
    pub within: FocusHandle,
    /// The list takes the keys.
    keys: FocusHandle,
    /// The mock scenario the reading is made from.
    scenario: &'static str,
    span: Span,
    load: Load,
    selection: Selection,
    /// The Jobs' titles: the page's own, and those the Jobs list holds.
    titles: HashMap<String, String>,
    /// The table of the open kind's Jobs, and the list that draws it.
    lines: Vec<JobLine>,
    jobs: ListState,
}

impl UsageScreen {
    /// On the mock reading of the scenario last named, the week's unless it names another.
    pub fn mock(cx: &mut Context<Self>) -> Self {
        cx.set_global(Mounted);
        let scenario = [mock::LOADING, mock::EMPTY].into_iter().find(|name| crate::mock::scenario_is(name)).unwrap_or(mock::WEEK);
        let mut this = UsageScreen {
            within: cx.focus_handle(),
            keys: cx.focus_handle(),
            scenario,
            span: Span::Week,
            load: Load::Reading,
            selection: Selection::default(),
            titles: mock::titles(),
            lines: Vec::new(),
            jobs: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
        };
        this.read(cx);
        this
    }

    /// The reading for the span now held. The loading scenario never answers.
    fn read(&mut self, cx: &mut Context<Self>) {
        self.load = match self.scenario {
            mock::LOADING => Load::Reading,
            mock::EMPTY => Load::Read(Board::new(mock::empty(self.span))),
            _ => Load::Read(Board::new(mock::waste(self.span))),
        };
        self.settle(cx);
    }

    /// What is held changed: the selection holds only kinds that are there, and the table is made again.
    fn settle(&mut self, cx: &mut Context<Self>) {
        let kinds: Vec<String> = self.board().map(|board| board.kinds().into_iter().map(str::to_string).collect()).unwrap_or_default();
        let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
        self.selection.keep(&kinds);
        self.lines = match (self.board(), self.selection.opened.as_deref()) {
            (Some(board), Some(kind)) => board.lines_of(kind, &self.titles),
            _ => Vec::new(),
        };
        self.jobs.reset(self.lines.len());
        cx.notify();
    }

    fn board(&self) -> Option<&Board> {
        match &self.load {
            Load::Read(board) => Some(board),
            Load::Reading => None,
        }
    }

    /// The titles of the Jobs the app holds, kept beside the page's own.
    pub fn set_titles(&mut self, titles: &HashMap<String, String>, cx: &mut Context<Self>) {
        if !titles.iter().any(|(id, title)| self.titles.get(id) != Some(title)) {
            return;
        }
        self.titles.extend(titles.iter().map(|(id, title)| (id.clone(), title.clone())));
        self.settle(cx);
    }

    // ---- what the keys do ------------------------------------------------------------------

    fn kinds_now(&self) -> Vec<String> {
        self.board().map(|board| board.kinds().into_iter().map(str::to_string).collect()).unwrap_or_default()
    }

    /// J and K, Down and Up.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let kinds = self.kinds_now();
        let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
        self.selection.step(&kinds, by);
        cx.notify();
    }

    /// Enter, Space or a press on a row: its panel opens, or shuts where it is open.
    fn open(&mut self, kind: Option<String>, cx: &mut Context<Self>) {
        let Some(kind) = kind.or_else(|| self.selection.cursor.clone()) else { return };
        self.selection.toggle(&kind);
        self.settle(cx);
    }

    /// Esc: the open panel shuts. False where none was open, so the key goes on.
    fn close(&mut self, cx: &mut Context<Self>) -> bool {
        let closed = self.selection.close();
        if closed {
            self.settle(cx);
        }
        closed
    }

    /// A window picked: the reading for it, the panel kept where its kind is still there.
    fn pick(&mut self, span: Span, cx: &mut Context<Self>) {
        if self.span != span {
            self.span = span;
            self.read(cx);
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.close(cx) {
            cx.stop_propagation();
        }
    }

    fn on_act(&mut self, act: &Act, _window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "usage_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "usage_open" => self.open(None, cx),
            "usage_window" => match Span::at(act.slot) {
                Some(span) => self.pick(span, cx),
                None => cx.propagate(),
            },
            _ => cx.propagate(),
        }
    }

    // ---- entering ----------------------------------------------------------------------------

    /// Brought to the front: the keys go to the list.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.keys, cx);
        cx.notify();
    }
}

/// Whether the app holds a Usage page, so the palette can offer it.
pub fn offered(cx: &App) -> bool {
    cx.has_global::<Mounted>()
}

/// Whether a mock scenario of this name opens on the Usage page.
pub fn is_scenario(name: &str) -> bool {
    mock::is_usage(name)
}
