//! What the editor draws over the canvas once the draft has a change: where it applies, the bar with Changes,
//! Discard and Publish, and the history of changes, each with its own Undo.

use gpui::{div, prelude::*, svg, AnyElement, Context, FocusHandle, FontWeight, Role, SharedString, Window};

use ipc::DraftBy;

use super::draft::Target;
use super::{LanesEditor, Press};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::change_band::filled;
use crate::settings::view::{ghost, ghost_with, mono, ringed, tip};

/// What the Undo of a change says where a change shown after it edits the same words.
const LATER_EDITS: &str = "A later change edits the same words";

/// The button that throws the drafts away where Fleet says the workflow changed since they began.
const START_OVER: &str = "Start from the published text";

/// What the band says where Armada has shipped a new version of a workflow he copied.
const ARMADA_CHANGED: &str = "Armada changed this workflow since you copied it";

impl LanesEditor {
    /// The band over the Changes card: Armada changed the workflow since it was copied. None for a copy with no base, and for
    /// Armada's own. It draws no button until the owner answers how he chooses what to take.
    pub(super) fn render_armada_band(&self, look: &Look) -> Option<AnyElement> {
        if !self.armada_changed {
            return None;
        }
        let said = div()
            .flex_auto()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(svg().flex_none().path(icon_path("git-compare")).size(look.length(token::SPACE_4)).text_color(look.color(token::ACCENT)))
            .child(div().min_w_0().child(ARMADA_CHANGED));
        let band = div()
            .id("workflow-armada-changed")
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3))
            .gap_y(look.length(token::SPACE_1))
            .min_w_0()
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::ACCENT))
            .bg(look.color(token::ACCENT_MUTED))
            .child(said);
        Some(labelled(band, Role::Group, ARMADA_CHANGED).into_any_element())
    }

    /// The draft bar and, under it, the history while it is open. None while the draft has no change.
    pub(super) fn render_bar(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        // The ask a failed move leaves, and a sentence Fleet refused with, stand alone where there is no draft.
        let editing = self.has_edits();
        if !editing && self.ask.is_none() && self.refused.is_none() {
            return None;
        }
        let keys = &self.keys;
        let off = self.busy || self.target.is_none();
        let changes = ringed(look, ghost_with(look, "workflow-draft-changes", Some(if self.history_open { "chevron-up" } else { "chevron-down" }), "Changes", false), &keys.changes, window)
            .track_focus(&keys.changes)
            .on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Changes, window, cx)));
        let discard = ringed(look, ghost(look, "workflow-draft-discard", "Discard", self.busy), &keys.discard, window)
            .track_focus(&keys.discard)
            .when(!self.busy, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Discard, window, cx))));
        let publish = ringed(look, filled(look, "workflow-draft-publish".into(), "Publish", off), &keys.publish, window)
            .track_focus(&keys.publish)
            .when(!off, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Publish, window, cx))));
        let acts = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3))
            .gap_y(look.length(token::SPACE_1))
            .child(labelled(changes, Role::Button, "Changes"))
            .child(div().flex_auto())
            .when(self.ask.is_none(), |acts| acts.child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(discard, Role::Button, "Discard")).child(labelled(publish, Role::Button, "Publish"))));
        // What Fleet refused with, else the loader's sentence for a draft that would not load.
        // A rule whose sentence is open says it under the sentence, so the bar does not say it twice.
        let unfit = self
            .draft
            .as_ref()
            .and_then(|draft| draft.unfit.clone())
            .or_else(|| self.pieces.iter().filter(|piece| !self.sentence.as_ref().is_some_and(|open| open.is_rule(&piece.id))).find_map(|piece| piece.unfit.clone()));
        let refused = self.refused.clone().or(unfit).map(|said| labelled(crate::settings::view::refusal(look, said.clone()).id("workflow-draft-refused"), Role::Alert, said));
        let start_over = self.stale.then(|| {
            let button = ringed(look, ghost(look, "workflow-draft-restart", START_OVER, self.busy), &keys.restart, window)
                .track_focus(&keys.restart)
                .when(!self.busy, |button| button.on_click(cx.listener(|this, _, window, cx| this.pressed(Press::Restart, window, cx))));
            labelled(button, Role::Button, START_OVER)
        });
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
            .children(editing.then_some(acts))
            .children(refused)
            .children(self.render_ask(look, window, cx))
            .children(start_over)
            .children(self.history_open.then(|| self.render_history(look, window, cx)));
        Some(frame.into_any_element())
    }

    /// Where it applies: this repository's own copy, or Kit's for every project.
    fn render_question(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let keys = &self.keys;
        let here = Target::Repository(self.subject.repository.clone());
        // The project being edited is offered first, by its name.
        let project = self.subject.project.as_ref().map(|(id, name)| {
            let on = self.target.as_ref() == Some(&Target::Project(id.clone()));
            self.radio(look, "workflow-draft-project", name.clone(), on, &keys.project, Press::Project, window, cx)
        });
        let choices = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .children(project)
            .child(self.radio(look, "workflow-draft-repository", "This repository", self.target.as_ref() == Some(&here), &keys.repository, Press::Repository, window, cx))
            .child(self.radio(look, "workflow-draft-every", "Every project", self.target.as_ref() == Some(&Target::EveryProject), &keys.every, Press::Every, window, cx));
        let ask = div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_MUTED)).child("Where does this apply?");
        labelled(div().id("workflow-draft-where").flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_3)).gap_y(look.length(token::SPACE_1)).child(ask).child(choices), Role::RadioGroup, "Where does this apply?").into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn radio(&self, look: &Look, id: &'static str, said: impl Into<SharedString>, on: bool, focus: &FocusHandle, press: Press, window: &Window, cx: &Context<Self>) -> AnyElement {
        let said: SharedString = said.into();
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
            .child(said.clone());
        let row = ringed(look, row, focus, window).track_focus(focus).on_click(cx.listener(move |this, _, window, cx| this.pressed(press, window, cx)));
        labelled(row, Role::RadioButton, said).into_any_element()
    }

    /// Each change in the order it was made, who made it, the line it was said in, and its Undo.
    fn render_history(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let rows = self.rows.iter().enumerate().map(|(n, row)| {
            let (glyph, who) = match row.by {
                DraftBy::You => ("user", "You"),
                DraftBy::Agent => ("bot", "Agent"),
            };
            let mark = labelled(
                div().id(SharedString::from(format!("workflow-change-by-{n}"))).flex_none().tooltip(tip(who)).child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED))),
                Role::Image,
                who,
            );
            let said = SharedString::from(row.said.clone());
            let undo = self.undos.get(n).map(|focus| {
                let off = self.busy || row.blocked;
                let button = ringed(look, ghost(look, SharedString::from(format!("workflow-undo-{n}")), "Undo", off), focus, window)
                    .track_focus(focus)
                    .when(row.blocked, |button| button.tooltip(tip(LATER_EDITS)))
                    .when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.pressed(Press::Undo(n), window, cx))));
                labelled(button, Role::Button, format!("Undo {}", row.said))
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
