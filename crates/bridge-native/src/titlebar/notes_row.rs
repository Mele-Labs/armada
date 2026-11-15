//! The switcher's In this build row: it opens the sheet of the running build's notes again, while the build has some.

use gpui::{prelude::*, AnyElement, Context, Role};

use super::parts::{tip, Btn, Weight};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::Look;

impl TitleBar {
    /// The row, or none while the running build holds no notes.
    pub(super) fn notes_row(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        self.face.build_notes.as_ref().filter(|read| !read.notes.is_empty())?;
        let button = Btn::new("build-notes", "In this build", Weight::Secondary)
            .icon("eye")
            .full()
            .draw(look, cx)
            .tooltip(tip("What each Job said to look at in this build"))
            .on_click(cx.listener(|this, _, window, cx| this.leave_with(Asked::BuildNotes, window, cx)));
        Some(labelled(button, Role::Button, "In this build").into_any_element())
    }
}
