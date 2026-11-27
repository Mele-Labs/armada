//! The feed in the column at the right: what happened since he last looked and needs no decision from him, newest first,
//! under Working, the cards he picked that the Operator is still carrying out.
//! One row for each entry: a glyph that says its kind in its own hue, its title, what was said under it, and its time. A row
//! with a Job opens it. It draws what `operator_page` serves in `feed` and derives nothing.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::operator_page::{OperatorFeedEntry, OperatorFeedKind, OperatorPage};

use super::face;
use super::page::OperatorPageView;
use super::view::{minute, pressed};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, spinner, tip};

impl OperatorPageView {
    pub(super) fn feed(&self, page: &OperatorPage, look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let since = page.looked_at.as_ref().and_then(|at| minute(at.as_str(), now));
        let head = div()
            .id("feed-head")
            .flex()
            .items_baseline()
            .gap(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_3))
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Since you last looked"))
            .child(div().flex_1())
            .children(page.looked_at.as_ref().map(|at| self.stamp(at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)));
        let rows = page.feed.iter().enumerate().map(|(at, entry)| self.feed_row(at, entry, look, cx));
        labelled(
            div().id("operator-feed").flex().flex_col().gap(look.length(token::SPACE_1)).child(labelled(head, Role::Heading, since.map_or("Since you last looked".to_string(), |since| format!("Since you last looked, {since}")))).children(rows),
            Role::Region,
            "Since you last looked",
        )
        .into_any_element()
    }

    /// Working: the cards he picked that the Operator is still carrying out, newest pick first, in a group of their own above
    /// what already happened. None while there are none.
    pub(super) fn working_group(&self, page: &OperatorPage, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if page.working.is_empty() {
            return None;
        }
        let head = div().id("working-head").pb(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Working");
        let rows = page.working.iter().map(|turn| self.working_row(turn, look, cx));
        Some(labelled(div().id("operator-working").flex().flex_col().gap(look.length(token::SPACE_1)).child(labelled(head, Role::Heading, "Working")).children(rows), Role::Region, "Working").into_any_element())
    }

    /// A card he picked that the Operator has not settled: a turning ring, his option as the title, the card's own title under it
    /// and what the Operator did, in a faint accent frame so it reads as live. A press brings the card to the front to read.
    fn working_row(&self, turn: &ipc::operator_page::OperatorTurnItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let option = face::picked_option(turn).unwrap_or_default();
        let title = face::card_title(turn);
        let name = format!("{option}, {title}");
        let id = turn.id.clone();
        let accent = look.color(token::ACCENT);
        let ring = look.edge(token::FOCUS_RING).map_or(accent, |(_, colour)| colour);
        let row = div()
            .id(SharedString::from(format!("working-{}", turn.id)))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(accent.opacity(0.4))
            .bg(look.color(token::ACCENT_FAINT))
            .cursor_pointer()
            .hover(move |style| style.border_color(accent))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip("Show the card, your pick and what the Operator did"))
            .child(
                div()
                    .id(SharedString::from(format!("working-mark-{}", turn.id)))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::SPACE_6))
                    .rounded_full()
                    .bg(accent.opacity(0.14))
                    .tooltip(tip("The Operator is working on it"))
                    .child(spinner(SharedString::from(format!("working-spin-{}", turn.id)), look.length(token::SPACE_4), accent, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(option))
                    .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(title))
                    .children(self.steps_list("feed", turn, look, cx).map(|steps| div().mt(look.length(token::SPACE_1)).child(steps))),
            )
            .children(turn.card.as_ref().and_then(|card| card.picked.as_ref()).map(|picked| self.stamp(picked.at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)));
        labelled(pressed(row, cx, move |this, window, cx| {
            this.bring_working(&id, cx);
            window.focus(&this.focus, cx);
        }), Role::Button, name)
        .into_any_element()
    }

    fn feed_row(&self, at: usize, entry: &OperatorFeedEntry, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (mark, hue, word) = face::feed_kind(entry.kind);
        let title: SharedString = match (&entry.kind, &entry.number) {
            (OperatorFeedKind::Merged, Some(number)) => format!("#{number} {}", entry.title).into(),
            _ => entry.title.clone().into(),
        };
        let said = entry.said.clone().filter(|said| !said.is_empty());
        let name = match &said {
            Some(said) => format!("{title}, {said}"),
            None => title.to_string(),
        };
        let row = div()
            .id(SharedString::from(format!("feed-{at}")))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .child(
                div()
                    .id(SharedString::from(format!("feed-mark-{at}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::SPACE_6))
                    .rounded_full()
                    .bg(look.color(hue).opacity(0.14))
                    .tooltip(tip(word))
                    .child(glyph(mark, look.length(token::SPACE_4), look.color(hue))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(title.clone()))
                    .children(said.map(|said| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said)))),
            )
            .child(self.stamp(entry.at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx));
        match entry.session_id.clone() {
            Some(job) => {
                let said = format!("Open {}", entry.title);
                labelled(pressed(row.cursor_pointer().hover(|style| style.bg(cx.look().color(token::BG_HOVER))).tooltip(tip(said)).focusable().tab_stop(true), cx, move |this, _, cx| this.open_job(job.clone(), cx)), Role::Row, name).into_any_element()
            }
            None => labelled(row, Role::Row, name).into_any_element(),
        }
    }
}
