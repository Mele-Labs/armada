//! Editing a message he sent, in place in its bubble (`docs/concepts/session/editing-a-sent-message.md`).
//! A right press on his bubble offers Edit, and ↑ in an empty box edits the last one he sent. Enter saves,
//! Shift+Enter puts a new line, Esc puts the words back. The new words stand in the bubble the moment
//! they are saved, until Fleet's row reads them or Fleet refuses.

use std::rc::Rc;

use gpui::{div, prelude::*, Context, Entity, Role, SharedString, Subscription, Window};
use gpui_component::input::{Escape, InputEvent, Textarea, TextareaState};
use ipc::{EditSessionMessage, SessionRow, SessionVoice, ThreadItem};

use super::menus::Pick;
use super::wire::Done;
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The bubble open for new words.
pub(super) struct Editing {
    /// The message row's id.
    row: String,
    /// Its place among the thread's items.
    ix: usize,
    /// The words it held when the edit opened: the same words saved are no change.
    before: String,
    area: Entity<TextareaState>,
    _saving: Subscription,
}

/// The menu of his own bubble.
pub(super) fn entries() -> Vec<Entry> {
    vec![Entry::Act { id: "edit", label: "Edit".into(), icon: Some("square-pen"), danger: false }]
}

impl JobView {
    /// The pick of his bubble's menu.
    pub(super) fn edit_pick(&self, ix: usize, row: &str, text: &str) -> Pick {
        let (row, text) = (row.to_string(), text.to_string());
        Rc::new(move |this, id, window, cx| {
            if id == "edit" {
                this.open_message_edit(ix, row.clone(), text.clone(), window, cx);
            }
        })
    }

    /// Open the bubble of row `row`, at item `ix`, with `text` in it and the caret at its end.
    pub(super) fn open_message_edit(&mut self, ix: usize, row: String, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.closed(&id, cx) {
            return;
        }
        let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 12).submit_on_enter(true));
        area.update(cx, |area, cx| {
            area.set_value(text.clone(), window, cx);
            let end = area.value().len();
            area.set_selected_range(end..end, cx);
        });
        let saving = cx.subscribe_in(&area, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { shift: false, .. } => this.save_message_edit(window, cx),
            // The box grows a line at a time, and the row under it moves down with it.
            InputEvent::Change => this.message_edit_grew(cx),
            _ => {}
        });
        let Some(page) = self.pages.get_mut(&id) else { return };
        let earlier = page.editing.replace(Editing { row, ix, before: text, area: area.clone(), _saving: saving }).map(|one| one.ix);
        page.list.scroll_to_reveal_item(ix + page.lead);
        area.update(cx, |area, cx| area.focus(window, cx));
        if let Some(earlier) = earlier {
            self.card_changed(earlier, cx);
        }
        self.card_changed(ix, cx);
    }

    /// ↑ in an empty box edits the last message he sent. Returns whether there was one.
    pub(super) fn edit_last(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.shown.clone() else { return false };
        if !self.composer.input.read(cx).value().is_empty() || self.composer.holds(&id) {
            return false;
        }
        let Some(page) = self.pages.get(&id) else { return false };
        let last = self.thread(&id, cx).and_then(|held| {
            held.items.iter().enumerate().rev().find_map(|(ix, item)| match item {
                ThreadItem::Row { row: SessionRow::Message { id: row, from: SessionVoice::You, text, .. } } if !super::rows::is_act(text) => {
                    Some((ix, row.clone(), page.edits.get(row).cloned().unwrap_or_else(|| text.clone())))
                }
                _ => None,
            })
        });
        let Some((ix, row, text)) = last else { return false };
        self.open_message_edit(ix, row, text, window, cx);
        true
    }

    /// Enter: the new words go to Fleet and stand in the bubble at once. Words left empty or the
    /// same as before are a cancel.
    pub(super) fn save_message_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(editing) = self.pages.get_mut(&id).and_then(|page| page.editing.take()) else { return };
        let text = super::format::sent_text(editing.area.read(cx).value().trim());
        let ix = editing.ix;
        self.box_focus(window, cx);
        if text.is_empty() || text == editing.before.trim() {
            self.card_changed(ix, cx);
            return;
        }
        let row = editing.row;
        match self.fleet {
            Some(_) => {
                if let Some(page) = self.pages.get_mut(&id) {
                    page.edits.insert(row.clone(), text.clone());
                    page.refused = None;
                }
                let body = ipc::encode(&EditSessionMessage { session_id: Self::session_id(&id), row_id: row.clone(), text });
                let job = id.clone();
                let done: Done = Box::new(move |this, answer, _, cx| {
                    let Err(why) = answer else { return };
                    // Refused: the bubble reads its old words again, and Fleet's reason stands over the box.
                    if let Some(page) = this.pages.get_mut(&job) {
                        page.edits.remove(&row);
                        page.refused = Some(why.into());
                        page.list.remeasure();
                    }
                    cx.notify();
                });
                self.post("/sessions/message/edit".into(), body, window, cx, done);
            }
            None => {
                // Mock data: the turn was written to the agent, so the row is marked as Fleet marks it.
                let at = ipc::Instant::carried(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
                if let Some(held) = self.mock.get_mut(id.as_str()) {
                    if let Some(ThreadItem::Row { row: SessionRow::Message { text: words, edited_at, .. } }) = held.items.get_mut(ix) {
                        *words = text;
                        *edited_at = Some(at);
                    }
                    held.version += 1;
                    held.mark_dirty(ix);
                }
                self.sync(&id, cx);
            }
        }
        self.card_changed(ix, cx);
    }

    /// Esc: the bubble reads its words again and the keys go back to the box.
    pub(super) fn cancel_message_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(editing) = self.pages.get_mut(&id).and_then(|page| page.editing.take()) else { return };
        self.box_focus(window, cx);
        self.card_changed(editing.ix, cx);
    }

    fn message_edit_grew(&mut self, cx: &mut Context<Self>) {
        let Some(ix) = self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.editing.as_ref()).map(|one| one.ix) else { return };
        self.card_changed(ix, cx);
    }

    fn box_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.composer.input.update(cx, |input, cx| input.focus(window, cx));
    }

    /// The box in place of the bubble's words, while row `row` is being edited.
    pub(super) fn edit_box(&self, row: &str, look: &Look, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let editing = self.shown.as_ref().and_then(|id| self.pages.get(id))?.editing.as_ref().filter(|one| one.row == row)?;
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("edit:{row}")))
                    .key_context(crate::keymap::TEXT_BOX)
                    .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                        this.cancel_message_edit(window, cx);
                        cx.stop_propagation();
                    }))
                    .w_full()
                    .text_size(look.length(token::TEXT_BASE))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Textarea::new(&editing.area).appearance(false).aria_label("Edit your message")),
                Role::TextInput,
                "Edit your message",
            )
            .into_any_element(),
        )
    }

}
