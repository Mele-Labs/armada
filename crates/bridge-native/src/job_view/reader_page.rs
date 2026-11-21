//! A page in the reader: its words are inside the web view (`page_tab.rs`'s `PageTab`, the sealed page with the kit's script
//! on it, so its row actions and votes work as they do in the page's own tab), with his notes in a gutter beside it. The
//! reader asks the page what it cannot know itself, through `web::reader`: the headings for the contents, the words he
//! selected for a note, and a scroll to a heading or to the words a note marks.
//!
//! Edit types in the page itself (`reader_page_edit.rs`). What moved since he last opened it is not marked: that needs
//! Fleet's side.
//! A note stands in order in the gutter, not level with its paragraph, as the page scrolls inside its own view.

use gpui::{div, prelude::*, AnyElement, Context, Entity, Role, SharedString, Window};

use super::acts::kbd;
use super::page_tab::PageTab;
use super::reader::{Reader, Room, Subject};
use super::row_tabs::RowKind;
use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::web::reader as ask;

impl JobView {
    /// The page a reader shows.
    pub(super) fn reader_page(&self, id: &str, name: &str) -> Option<Entity<PageTab>> {
        let held = self.reader_of(id, name)?;
        self.row_tabs.get(id)?.pages.get(&held.key).map(|(tab, _)| tab.clone())
    }

    /// The headings of what the reader shows, as `(place, level, words)`: a draft's by its blocks, a page's by what the page
    /// reported once it loaded. A page's place is the heading's own number.
    pub(super) fn reader_heads(&self, id: &str, held: &Reader, cx: &gpui::App) -> Vec<(usize, u8, String)> {
        if held.kind != RowKind::Document {
            return held.heads();
        }
        let Some(tab) = self.reader_page(id, &super::reader::reader_id(held.kind, &held.key)) else { return Vec::new() };
        tab.read(cx).heads().iter().enumerate().map(|(at, head)| (at, head.level, head.text.clone())).collect()
    }

    /// A page's words and the notes beside them: the page at the left, the gutter at the right; where there is no room for
    /// the gutter, the notes stand under the page.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reader_page_words(&self, id: &SharedString, name: &str, held: &Reader, subject: &Subject, room: Room, look: &Look, cx: &Context<Self>) -> AnyElement {
        let page = self.reader_page(id, name);
        let view = div().id(SharedString::from(format!("reader-page:{name}"))).flex_1().min_w_0().min_h_0().relative().children(page);
        let pins: Vec<AnyElement> = held.notes.iter().enumerate().map(|(place, note)| self.reader_pin(id, name, place, note, held.on == Some(place), look, cx)).collect();
        let add = (!subject.closed).then(|| self.reader_add_button(id, name, look, cx));
        // The mock of the talk in the margin, for the owner to judge.
        if crate::mock::scenario_is("planning-margin") {
            return div().flex_1().min_h_0().min_w_0().flex().child(view).child(super::reader_talk::margin(look)).into_any_element();
        }
        // Notes off: no margin and nothing under the page, so the page takes the width.
        if !held.notes_on {
            return div().flex_1().min_h_0().min_w_0().flex().child(view).into_any_element();
        }
        if room.gutter() {
            let gutter = div()
                .id(SharedString::from(format!("reader-gutter:{name}")))
                .flex_none()
                .w(super::reader::notes_margin(held.notes_on, room, look))
                .min_h_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_4))
                .border_l(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .overflow_y_scroll()
                .children(add)
                .children(pins);
            return div().flex_1().min_h_0().min_w_0().flex().child(view).child(gutter).into_any_element();
        }
        let under = (!pins.is_empty() || add.is_some()).then(|| {
            div()
                .id(SharedString::from(format!("reader-notes-under:{name}")))
                .flex_none()
                .max_h(look.length(token::SPACE_12) * 4.)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .overflow_y_scroll()
                .children(add)
                .children(pins)
        });
        div().flex_1().min_h_0().min_w_0().flex().flex_col().child(view).children(under).into_any_element()
    }

    /// The press that makes a note on what he selected in the page: N does the same where the reader holds the keys, and a
    /// page that holds them hears N itself.
    fn reader_add_button(&self, id: &SharedString, name: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, name_c) = (id.clone(), name.to_string());
        labelled(
            div()
                .id(SharedString::from(format!("reader-add-note:{name}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .tooltip(tip("Make a note on the words selected in the page"))
                .child(kbd(look, "N"))
                .child("add a note")
                .on_click(cx.listener(move |this, _, window, cx| this.reader_page_note(&job, &name_c, false, window, cx))),
            Role::Button,
            "Add a note",
        )
        .into_any_element()
    }

    /// N on a page: the page says what he selected, and the note is made on it with the caret in it. With nothing selected the
    /// note is on the block at the top of the view.
    pub(super) fn reader_page_note(&mut self, id: &SharedString, name: &str, toggle: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        let asked = tab.read(cx).ask(&ask::selection(), cx);
        let (job, reader) = (id.clone(), name.to_string());
        cx.spawn_in(window, async move |this, cx| {
            let picked = ask::picked_of(asked.recv().await.ok().flatten().as_deref()).unwrap_or_default();
            let _ = this.update_in(cx, |this, window, cx| {
                // With nothing selected, N is the switch of the margin; the margin's own button always makes a note.
                if toggle && picked.quote.trim().is_empty() {
                    this.reader_toggle_notes(&job, &reader, cx);
                } else {
                    this.reader_add_note(&job, &reader, picked.quote, picked.block, window, cx);
                }
            });
        })
        .detach();
    }

    /// A note's words, shown in the page: selected and scrolled to, or its block scrolled to where the words are not found.
    pub(super) fn reader_page_show(&mut self, id: &SharedString, name: &str, place: usize, cx: &mut Context<Self>) {
        let Some(note) = self.reader_of(id, name).and_then(|held| held.notes.get(place)).map(|one| (one.quote.clone(), one.block.clone())) else { return };
        let Some(tab) = self.reader_page(id, name) else { return };
        let asked = tab.read(cx).ask(&ask::show(&note.0, &note.1), cx);
        cx.spawn(async move |_, _| {
            asked.recv().await.ok();
        })
        .detach();
    }

    /// A heading of the contents: the page scrolls to it, and the contents mark it.
    pub(super) fn reader_page_go(&mut self, id: &SharedString, name: &str, head: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        let Some(block) = tab.read(cx).heads().get(head).map(|one| one.block.clone()) else { return };
        let asked = tab.read(cx).ask(&ask::go(&block), cx);
        if let Some(held) = self.reader_mut(id, name) {
            held.at_head = head;
        }
        cx.spawn(async move |_, _| {
            asked.recv().await.ok();
        })
        .detach();
        cx.notify();
    }

    /// An earlier revision of the page, or the newest again, read from Fleet and drawn in place of the one shown.
    pub(super) fn reader_page_revision(&mut self, id: &SharedString, name: &str, revision: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        if tab.read(cx).shown() == revision {
            return;
        }
        tab.update(cx, |tab, cx| tab.show_revision(revision, cx));
        if let Some(held) = self.reader_mut(id, name) {
            held.at_head = 0;
        }
        self.fetch_page(tab, window, cx);
    }
}
