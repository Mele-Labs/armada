//! What the editor draws over the canvas once the draft has a change: where it applies, the bar with Changes,
//! Discard and Publish, and the history of changes, each with its own Undo.

use gpui::{div, prelude::*, svg, AnyElement, Context, FocusHandle, FontWeight, Role, SharedString, Window};

use super::draft::{Author, Target};
use super::{LanesEditor, Press};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::change_band::filled;
use crate::settings::view::{ghost, ghost_with, mono, ringed, small, tip};

/// The line beside Publish: a save reaches every Job running on the workflow, as Fleet reads steps live.
pub const REACHES: &str = "Running Jobs on this workflow take the change";

impl LanesEditor {
    /// The draft bar and, under it, the history while it is open. None while the draft has no change.
    pub(super) fn render_bar(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if self.draft.is_empty() {
            return None;
        }
        let keys = &self.keys;
        let off = self.busy || self.draft.target().is_none();
        let changes = ringed(look, ghost_with(look, "workflow-draft-changes", Some(if self.history_open { "chevron-up" } else { "chevron-down" }), "Changes", false), &keys.changes, window)
            .track_focus(&keys.changes)
            .on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Changes, window, cx)));
        let discard = ringed(look, ghost(look, "workflow-draft-discard", "Discard", self.busy), &keys.discard, window)
            .track_focus(&keys.discard)
            .when(!self.busy, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Discard, window, cx))));
        let publish = ringed(look, filled(look, "workflow-draft-publish".into(), "Publish", off), &keys.publish, window)
            .track_focus(&keys.publish)
            .when(!off, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Publish, window, cx))));
        let reaches = labelled(small(look, REACHES, look.color(token::FG_MUTED)).id("workflow-draft-reaches").min_w_0(), Role::Label, REACHES);
        let acts = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3))
            .gap_y(look.length(token::SPACE_1))
            .child(labelled(changes, Role::Button, "Changes"))
            .child(div().flex_auto())
            .child(reaches)
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(discard, Role::Button, "Discard")).child(labelled(publish, Role::Button, "Publish")));
        let refused = self.refused.as_ref().map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("workflow-draft-refused"), Role::Alert, said.clone()));
        let frame = div()
            .id("workflow-draft")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .children(self.question_shown().then(|| self.render_question(look, window, cx)))
            .child(acts)
            .children(refused)
            .children(self.history_open.then(|| self.render_history(look, window, cx)));
        Some(frame.into_any_element())
    }

    /// Where it applies: this repository's own copy, or Kit's for every project.
    fn render_question(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let keys = &self.keys;
        let here = Target::Repository(self.subject.repository.clone());
        let choices = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(self.radio(look, "workflow-draft-repository", "This repository", self.draft.target() == Some(&here), &keys.repository, Press::Repository, window, cx))
            .child(self.radio(look, "workflow-draft-every", "Every project", self.draft.target() == Some(&Target::EveryProject), &keys.every, Press::Every, window, cx));
        let ask = div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child("Where does this apply?");
        labelled(div().id("workflow-draft-where").flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_3)).gap_y(look.length(token::SPACE_1)).child(ask).child(choices), Role::RadioGroup, "Where does this apply?").into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn radio(&self, look: &Look, id: &'static str, said: &'static str, on: bool, focus: &FocusHandle, press: Press, window: &Window, cx: &Context<Self>) -> AnyElement {
        let ring = look.color(if on { token::ACCENT } else { token::BORDER_STRONG });
        let dot = div()
            .flex_none()
            .size(look.length(token::SPACE_4))
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(ring)
            .flex()
            .items_center()
            .justify_center()
            .when(on, |dot| dot.child(div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::ACCENT))));
        let row = div()
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .child(dot)
            .child(said);
        let row = ringed(look, row, focus, window).track_focus(focus).on_click(cx.listener(move |this, _, window, cx| this.pressed(press, window, cx)));
        labelled(row, Role::RadioButton, said).into_any_element()
    }

    /// Each change in the order it was made, who made it, the line it was said in, and its Undo.
    fn render_history(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let rows = self.draft.edits().iter().enumerate().map(|(n, edit)| {
            let (glyph, who) = match edit.author {
                Author::Person => ("user", "You"),
                Author::Agent => ("bot", "Agent"),
            };
            let mark = labelled(
                div().id(SharedString::from(format!("workflow-change-by-{n}"))).flex_none().tooltip(tip(who)).child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
                Role::Image,
                who,
            );
            let said = SharedString::from(edit.said.clone());
            let undo = self.undos.get(n).map(|focus| {
                let button = ringed(look, ghost(look, SharedString::from(format!("workflow-undo-{n}")), "Undo", self.busy), focus, window)
                    .track_focus(focus)
                    .when(!self.busy, |button| button.on_click(cx.listener(move |this, _, window, cx| this.pressed(Press::Undo(n), window, cx))));
                labelled(button, Role::Button, format!("Undo {}", edit.said))
            });
            labelled(
                div()
                    .id(SharedString::from(format!("workflow-change-{n}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_w_0()
                    .child(mark)
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(said.clone()))
                    .children(undo),
                Role::ListItem,
                said,
            )
        });
        labelled(div().id("workflow-history").flex().flex_col().gap(look.length(token::SPACE_1)).children(rows), Role::Region, "Changes").into_any_element()
    }
}
