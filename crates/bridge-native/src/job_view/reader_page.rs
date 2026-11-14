//! A page in the reader: its words are inside the web view, so the page reports what he selects and where its blocks
//! stand, and the notes are pinned beside it.

use gpui::{div, prelude::*, AnyElement, Context, SharedString, Window};

use super::reader::{Reader, Room, Subject};
use super::JobView;
use crate::look::Look;

impl JobView {
    /// A page's words and the notes beside them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reader_page_words(&self, _id: &SharedString, _name: &str, _held: &Reader, _subject: &Subject, _room: Room, _look: &Look, _cx: &Context<Self>) -> AnyElement {
        div().into_any_element()
    }

    /// An earlier revision of the page, read from Fleet.
    pub(super) fn reader_page_revision(&mut self, _id: &SharedString, _revision: u32, _window: &mut Window, _cx: &mut Context<Self>) {}
}
