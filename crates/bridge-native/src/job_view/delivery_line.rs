//! The line Fleet writes when how a Job's work lands was kept for its project or answered by a setting. It says
//! so in Fleet's words, and its last sentence, "Change it in Settings", is a link to the setting.

use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, Role, SharedString};

use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::SettingsAt;
use crate::titlebar::{Asked, AskedSink};

/// Fleet's sentence split before the one that says where to change it: the words, then the link's. A sentence with no
/// such part gives the link "How work lands".
pub(super) fn words_and_link(said: &str) -> (&str, &str) {
    match said.find("Change it") {
        Some(at) => (said[..at].trim_end(), &said[at..]),
        None => (said, "How work lands"),
    }
}

impl JobView {
    /// The project the Job is in, by id and name: what its link has Settings pick. None for a Job in no project.
    fn project_of(&self, job: &str, cx: &Context<Self>) -> Option<(String, String)> {
        let id = self.list.read(cx).record(job)?.hosted.as_ref()?.project.clone()?;
        let projects = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().projects.clone(),
            None => Some(crate::mock::projects()),
        };
        let name = projects.and_then(|all| all.projects.into_iter().find(|one| one.id == id)).map_or_else(|| id.clone(), |one| one.name);
        Some((id, name))
    }

    /// Where the line's link takes the keys.
    pub(super) fn delivery_at(&self, job: &str, cx: &Context<Self>) -> SettingsAt {
        SettingsAt::delivery(self.project_of(job, cx))
    }

    /// One quiet line in the thread: how the work lands, as Fleet said it, and a link to the setting.
    pub(super) fn delivery_line(&self, id: &str, said: &str, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (words, link) = words_and_link(said);
        let spot = self.spot(page, &format!("delivery:{id}"), cx);
        let at = self.shown.as_ref().map(|job| self.delivery_at(job, cx));
        let open = move |cx: &mut gpui::App| {
            if let (Some(at), Some(sink)) = (at.clone(), cx.try_global::<AskedSink>().cloned()) {
                (sink.0)(&Asked::SettingsAt(at), cx);
            }
        };
        let press = open.clone();
        let link = labelled(
            div()
                .id(SharedString::from(format!("delivery-link:{id}")))
                .track_focus(&spot.focus)
                .max_w_full()
                .whitespace_normal()
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_color(look.color(token::ACCENT))
                .cursor_pointer()
                .hover(|link| link.bg(look.color(token::ACCENT_FAINT)))
                .focus_visible(|link| link.border_color(look.color(token::ACCENT)))
                .tooltip(tip("Opens How work lands in Settings"))
                .on_key_down(cx.listener(move |_, event: &KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        open(cx);
                    }
                }))
                .on_click(cx.listener(move |_, _, _, cx| press(cx)))
                .child(SharedString::from(link.to_string())),
            Role::Link,
            link.to_string(),
        );
        labelled(
            div()
                .id(SharedString::from(format!("delivery:{id}")))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(glyph(look, "git-merge", Size::Small, look.color(token::FG_SUBTLE))))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x(look.length(token::SPACE_2))
                        .child(div().max_w_full().whitespace_normal().child(SharedString::from(words.to_string())))
                        .child(link),
                ),
            Role::Label,
            said.to_string(),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleets_sentence_is_split_before_the_words_that_say_where_to_change_it() {
        let (words, link) = words_and_link("Kept for Armada: its Jobs land this way from now on. Change it in Settings, in the project's settings.");
        assert_eq!(words, "Kept for Armada: its Jobs land this way from now on.");
        assert_eq!(link, "Change it in Settings, in the project's settings.");
    }

    #[test]
    fn a_sentence_that_does_not_say_where_to_change_it_still_has_a_link_to_the_setting() {
        assert_eq!(words_and_link("Open PR only, as Shop is set."), ("Open PR only, as Shop is set.", "How work lands"));
    }
}
