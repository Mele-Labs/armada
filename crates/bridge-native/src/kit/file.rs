//! A piece's file: read when the piece is picked, edited in place, saved over the text it was read as, and
//! opened in the person's editor or in Finder. Fleet resolves every path; nothing here names one.

use gpui::{AppContext as _, Context, Entity, Focusable as _, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::{KitFile, KitPlace, OpenKitPiece, SaveKitFile};

use super::{wire, KitScreen, Pane, Source};

/// Where a piece's file stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FileState {
    Reading,
    Read(KitFile),
    /// Fleet would not read it, in its own words and with its code.
    Refused { code: Option<String>, said: String },
}

/// The file in the pane as a textarea: what it started from, whole, so a save can say what it is over.
pub(super) struct Edit {
    pub id: String,
    pub read: String,
    pub area: Entity<TextareaState>,
    _typed: Subscription,
}

impl Edit {
    /// Whether the text in the box is not what was read.
    pub fn changed(&self, cx: &gpui::App) -> bool {
        self.area.read(cx).value().as_ref() != self.read
    }
}

fn keep_key(id: &str) -> String {
    format!("kit:edit:{id}")
}

impl KitScreen {
    /// The file of the piece picked, read once while Kit is in front.
    pub(super) fn want_file(&mut self, cx: &mut Context<Self>) {
        let Some(row) = self.picked().filter(|one| one.gone.is_none()).map(|one| one.row.clone()) else { return };
        let Some(kind) = row.kind.file() else { return };
        if self.files.contains_key(&row.id) {
            return;
        }
        if let Source::Mock(kit) = &self.source {
            let state = match kit.file(&row) {
                Ok(file) => FileState::Read(file),
                Err(why) => FileState::Refused { code: why.code, said: why.said },
            };
            self.files.insert(row.id, state);
            return;
        }
        let Some(port) = self.port(cx) else { return };
        self.files.insert(row.id.clone(), FileState::Reading);
        let ask = wire::file(port, kind, &row.name, row.comes_with.as_deref(), cx);
        let id = row.id.clone();
        self.work.insert(
            "file",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| {
                    this.files.insert(
                        id,
                        match answer {
                            Ok(file) => FileState::Read(file),
                            Err(why) => FileState::Refused { code: why.code, said: why.said },
                        },
                    );
                    cx.notify();
                });
            }),
        );
    }

    /// The file read for the piece picked, where there is one.
    pub(super) fn file_of_picked(&self) -> Option<&KitFile> {
        match self.files.get(self.selected.as_ref()?) {
            Some(FileState::Read(file)) => Some(file),
            _ => None,
        }
    }

    /// An edit of one piece does not follow the keys to another: it is let go, and its kept draft stays.
    pub(super) fn leave_edit_of_another(&mut self) {
        if self.editing.as_ref().is_some_and(|edit| Some(&edit.id) != self.selected.as_ref()) {
            self.editing = None;
            self.moved = None;
        }
    }

    /// Whether Edit is offered: the file is read and no plugin brought it.
    pub(super) fn can_edit(&self) -> bool {
        self.file_of_picked().is_some_and(|file| file.plugin.is_none()) && self.picked().is_some_and(|one| one.gone.is_none())
    }

    /// Edit (E): the pane becomes a textarea over the text read, with a kept draft over it where one was left.
    pub(super) fn start_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() || !self.can_edit() {
            return;
        }
        let (Some(file), Some(id)) = (self.file_of_picked().cloned(), self.selected.clone()) else { return };
        let key = keep_key(&id);
        let first = crate::drafts::kept(&key, cx).unwrap_or_else(|| file.text.clone());
        let area = cx.new(|cx| {
            let state = TextareaState::new(window, cx).auto_grow(12, 5000);
            state
        });
        area.update(cx, |area, cx| area.set_value(first, window, cx));
        let read = file.text.clone();
        let typed = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = area.read(cx).value().to_string();
                if text == read {
                    crate::drafts::clear(&key, cx);
                } else {
                    crate::drafts::keep(&key, &text, cx);
                }
                this.moved = None;
                cx.notify();
            }
        });
        let handle = area.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        self.refused = None;
        self.moved = None;
        self.editing = Some(Edit { id, read: file.text, area, _typed: typed });
        cx.notify();
    }

    /// Esc: the edit is dropped, and so is the draft kept of it.
    pub(super) fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(edit) = self.editing.take() {
            crate::drafts::clear(&keep_key(&edit.id), cx);
        }
        self.moved = None;
        self.refused = None;
        self.focus(Pane::Detail, window, cx);
    }

    /// Save (⌘S): the text goes to disk over the text that was read, or Fleet says why not.
    pub(super) fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.editing.as_ref().filter(|edit| edit.changed(cx)) else { return };
        let Some(row) = self.listed.shown.iter().find(|one| one.row.id == edit.id).map(|one| one.row.clone()) else { return };
        let Some(kind) = row.kind.file() else { return };
        let (id, read, text) = (edit.id.clone(), edit.read.clone(), edit.area.read(cx).value().to_string());
        self.refused = None;
        self.moved = None;
        if let Source::Mock(kit) = &mut self.source {
            let answer = kit.save(&row, &read, &text);
            self.saved(id, answer, window, cx);
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        let ask = wire::save_file(port, SaveKitFile { kind, name: row.name.clone(), read, text, plugin: row.comes_with.clone() }, cx);
        self.work.insert(
            "save",
            cx.spawn_in(window, async move |this, cx| {
                let answer = ask.await;
                let _ = this.update_in(cx, |this, window, cx| this.saved(id, answer, window, cx));
            }),
        );
    }

    fn saved(&mut self, id: String, answer: Result<KitFile, wire::Saved>, window: &mut Window, cx: &mut Context<Self>) {
        match answer {
            Ok(file) => {
                crate::drafts::clear(&keep_key(&id), cx);
                self.files.insert(id, FileState::Read(file));
                self.editing = None;
                self.focus(Pane::Detail, window, cx);
            }
            Err(why) if why.code.as_deref() == Some(wire::FILE_MOVED) => {
                self.refused = Some(SharedString::from(why.said));
                self.moved = Some(why.on_disk);
            }
            Err(why) => self.refused = Some(SharedString::from(why.said)),
        }
        cx.notify();
    }

    /// Reload what is on disk, which is what Fleet says the file holds now: the edit is let go.
    pub(super) fn reload_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(Some(text)), Some(edit)) = (self.moved.clone(), self.editing.take()) else { return };
        crate::drafts::clear(&keep_key(&edit.id), cx);
        if let Some(FileState::Read(file)) = self.files.get_mut(&edit.id) {
            file.text = text;
        }
        self.moved = None;
        self.refused = None;
        self.focus(Pane::Detail, window, cx);
    }

    /// Open (⏎) in the person's editor, or Show in Finder: Fleet resolves the file and holds the guard.
    pub(super) fn open_piece(&mut self, place: KitPlace, cx: &mut Context<Self>) {
        let Some(row) = self.picked().filter(|one| one.gone.is_none()).map(|one| one.row.clone()) else { return };
        let Some(kind) = row.kind.opens() else { return };
        self.refused = None;
        let name = row.piece.as_ref().map_or_else(|| row.name.clone(), |piece| piece.name.clone());
        if self.is_mock() {
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be opened.", cx) };
        let ask = wire::open(port, OpenKitPiece { kind, name, plugin: row.comes_with.clone(), place }, cx);
        self.work.insert(
            "open",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| {
                    if let Err(why) = answer {
                        this.refuse(why.said, cx);
                    }
                });
            }),
        );
    }
}
