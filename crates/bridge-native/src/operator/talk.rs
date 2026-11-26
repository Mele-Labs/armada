//! The talk under the card: the Operator session's own thread and box, drawn by the Job view as a Job's are
//! (`crate::job_view::operator_thread_on_page`). What stays here is the page's part of it: the tag Fleet puts on his words
//! about a card, the watch that folds the card in front once the talk overflows its space, and the new-card strip where a
//! card that would take the front then waits.

use gpui::{canvas, div, prelude::*, Animation, AnimationExt as _, AnyElement, BoxShadow, Context, FontWeight, Role, SharedString};

use super::page::OperatorPageView;
use super::view::{press, pressed, Weight};
use crate::labels::labelled;
use crate::look::{token, ActiveLook as _, Look};
use crate::titlebar::parts::{glyph, tip};

/// The card a message is about, as Fleet tags his words with it (`[About card turn:7 "Title"]` on a line of its own),
/// and his words after the tag.
pub(crate) fn tag_of(text: &str) -> Option<(&str, &str, &str)> {
    let rest = text.strip_prefix("[About card ")?;
    let (line, words) = rest.split_once('\n').unwrap_or((rest, ""));
    let line = line.trim_end().trim_end_matches(']');
    let (id, title) = line.split_once(' ').unwrap_or((line, ""));
    Some((id, title.trim().trim_matches('"'), words.trim_start()))
}

impl OperatorPageView {
    /// The talk's list measured as it is drawn: once it overflows the room it was given the card in front folds, and its whole
    /// height is that room. A short window that squeezes the talk under a card drawn whole folds nothing.
    pub(super) fn talk_watch(&self) -> AnyElement {
        let (long, tall) = (self.talk_long.clone(), self.talk_tall.clone());
        canvas(
            move |_, window, cx| {
                let Some(state) = crate::job_view::operator_list(cx) else { return };
                let line = cx.look().length(token::SPACE_12);
                let seen = state.viewport_bounds().size.height;
                let whole = seen + state.max_offset_for_scrollbar().y;
                let room = whole.min(line * 3.).max(line);
                if !long.get() && state.max_offset_for_scrollbar().y > gpui::px(0.) && seen + gpui::px(0.5) >= room {
                    long.set(true);
                    window.refresh();
                }
                if (whole - tall.get()).abs() > gpui::px(0.5) {
                    tall.set(whole);
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_0()
        .into_any_element()
    }

    /// The new-card strip: a card that would have taken the front while the card was folded or the talk in use waits here,
    /// with how many came, the newest's title, and Show. It glows once as the newest arrives, then holds.
    pub(super) fn strip(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let newest = self.fresh.last()?;
        let turn = self.items().iter().find(|turn| &turn.id == newest)?;
        let title = super::face::card_title(turn);
        let you = look.color(token::YOU);
        let count = self.fresh.len();
        let said = if count == 1 { "A new card".to_string() } else { format!("{count} new cards") };
        let show = press("strip-show", "Show".into(), None, Some(crate::keymap::shown("send_message").into()), Weight::Plain, look).tooltip(tip("Bring the newest card in front"));
        let band = div()
            .id("new-card-strip")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .pl(look.length(token::SPACE_4))
            .pr(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(you)
            .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
            .child(labelled(
                div().id("strip-bell").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).tooltip(tip(said.clone())).child(glyph("bell", look.length(token::SPACE_4), you)).child(
                    div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(you).child(count.to_string()),
                ),
                Role::Status,
                said,
            ))
            .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.clone()))
            .child(labelled(pressed(show, cx, |this, window, cx| this.show_fresh(window, cx)), Role::Button, "Show"));
        let glow = look.length(token::SPACE_3);
        let band = band.with_animation(SharedString::from(format!("strip-glow:{newest}")), Animation::new(look.duration(token::DURATION_PULSE)), move |band, at| {
            band.shadow(vec![BoxShadow { color: you.opacity(0.6 * (1. - at)), offset: gpui::point(gpui::px(0.), gpui::px(0.)), blur_radius: glow, spread_radius: glow * (0.25 * (1. - at)), inset: false }])
        });
        Some(labelled(div().id("new-card-strip-region").flex_none().child(band), Role::Region, format!("New card: {title}")).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tagged_send_reads_as_its_card_and_his_words() {
        assert_eq!(tag_of("[About card turn:7 \"Main is red\"]\nWhy?"), Some(("turn:7", "Main is red", "Why?")));
        assert_eq!(tag_of("Why?"), None);
    }
}
