//! The inbox column: Search or ask, the Lists filter, and the pull requests in their four groups.
//! A refresh keeps the last inbox drawn under a Refreshing mark.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::PullReviewChecks;

use super::model::{Group, InboxRow, Verdict};
use super::{panel, ReviewsScreen};
use crate::jobs::{group_line, list_bar, list_control, list_search, row_card, row_edge, row_wash};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, icon_button, small, spinner, tip};

/// A group's hue, on its header band and the glyph of each row's state.
pub(super) fn hue(group: Group) -> crate::look::Token {
    match group {
        Group::FirstLook => token::STATUS_AWAITING_REVIEW,
        Group::Changed => token::STATUS_HANDED_IN,
        Group::Waiting => token::STATUS_NOT_STARTED,
        Group::Done => token::STATUS_COMPLETED_SUCCESS,
    }
}

/// The checks' state as a registry glyph, its hue and what its tooltip says. Passing checks
/// draw nothing: only a state that wants him is marked.
pub(super) fn checks_mark(checks: PullReviewChecks) -> Option<(&'static str, crate::look::Token, &'static str)> {
    match checks {
        PullReviewChecks::Passed => None,
        PullReviewChecks::Failed => Some(("x", token::RUN_FAILED, "Checks failed")),
        PullReviewChecks::Running => Some(("loader", token::STATUS_AWAITING_REVIEW, "Checks running")),
    }
}

/// His review's hue, and the ground of its chip.
pub(super) fn verdict_hue(verdict: Verdict) -> (crate::look::Token, crate::look::Token) {
    match verdict {
        Verdict::NotReviewed => (token::STATUS_AWAITING_REVIEW, token::STATUS_AWAITING_REVIEW_BG),
        Verdict::Commented => (token::FG_MUTED, token::BG_SUNKEN),
        Verdict::ChangesRequested => (token::STATUS_COMPLETED_FAILED, token::STATUS_COMPLETED_FAILED_BG),
        Verdict::Approved => (token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
    }
}

impl ReviewsScreen {
    /// The rows the search and the Lists filter leave, in the order the groups draw them.
    fn shown_rows(&self, cx: &gpui::App) -> Vec<&InboxRow> {
        let query = self.search.read(cx).value().trim().to_lowercase();
        Group::ALL
            .iter()
            .flat_map(|group| self.inbox.rows.iter().filter(move |row| row.group == *group))
            .filter(|row| self.lists.is_none_or(|list| row.lists.contains(&list)))
            .filter(|row| {
                query.is_empty()
                    || row.title.to_lowercase().contains(&query)
                    || row.repository.to_lowercase().contains(&query)
                    || format!("#{}", row.number).contains(&query)
                    || row.author.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// The rows the arrows move through: those of groups that are not folded.
    fn reachable(&self, cx: &gpui::App) -> Vec<u64> {
        self.shown_rows(cx).into_iter().filter(|row| !self.folded.contains(&row.group)).map(|row| row.number).collect()
    }

    pub(super) fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("move_focus") {
            let rows = self.reachable(cx);
            let by: isize = if act.slot % 2 == 0 { 1 } else { -1 };
            let at = self.focused.and_then(|number| rows.iter().position(|one| *one == number));
            let next = match at {
                Some(at) => (at as isize + by).clamp(0, rows.len() as isize - 1) as usize,
                None => 0,
            };
            self.focused = rows.get(next).copied();
            cx.notify();
        } else if act.is("open_focused") {
            if let Some(number) = self.focused {
                self.press_row(number, window, cx);
            }
        } else {
            cx.propagate();
        }
    }

    pub(super) fn render_inbox(&mut self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let refreshing = self.inbox.refreshing;
        let muted = look.color(token::FG_MUTED);
        let mark: AnyElement = if refreshing {
            labelled(
                div()
                    .id("inbox-refreshing")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .h(look.length(token::H_BADGE))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::STATUS_RUNNING_BG))
                    .text_color(look.color(token::STATUS_RUNNING))
                    .text_size(look.length(token::TEXT_2XS))
                    .tooltip(tip("Reading the forge again. The rows are as the last read left them."))
                    .child(spinner("inbox-refresh-spin", look.length(token::SPACE_3), look.color(token::STATUS_RUNNING), cx))
                    .child("Refreshing"),
                Role::Status,
                "Refreshing",
            )
            .into_any_element()
        } else {
            labelled(
                icon_button("inbox-refresh", "refresh-cw", muted, look)
                    .tooltip(tip("Refresh"))
                    .on_click(cx.listener(|this, _, window, cx| this.start_refresh(window, cx))),
                Role::Button,
                "Refresh",
            )
            .into_any_element()
        };
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                labelled(
                    div()
                        .id("inbox-title")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child(glyph("git-pull-request", look.length(token::SPACE_4), look.color(token::ACCENT)))
                        .child("Reviews"),
                    Role::Heading,
                    "Reviews",
                ),
            )
            .child(mark);
        let lists_said = match self.lists {
            None => "All",
            Some(ipc::PullReviewList::Requested) => "Requested",
            Some(ipc::PullReviewList::Team) => "Team",
            Some(ipc::PullReviewList::Assigned) => "Assigned",
            Some(ipc::PullReviewList::Mine) => "Mine",
        };
        let lists_name = format!("Lists: {lists_said}");
        let ink = if self.lists.is_none() { token::FG_MUTED } else { token::ACCENT };
        let lists = labelled(
            list_control(look)
                .id("inbox-lists")
                .text_color(look.color(ink))
                .tooltip(tip("The forge's lists: requested of you, of your team, assigned, your own"))
                .on_click(cx.listener({
                    let name = lists_name.clone();
                    move |this, event: &gpui::ClickEvent, window, cx| {
                        let at = super::under_label(Role::Button, &name, event.position(), window, cx);
                        this.open_lists(at, window, cx);
                    }
                }))
                .child(SharedString::from(lists_name.clone()))
                .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(ink))),
            Role::Button,
            lists_name,
        );
        let search = list_bar(look)
            .child(
                labelled(
                    div().id("inbox-search").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(list_search(&self.search, look)),
                    Role::SearchInput,
                    "Search or ask",
                ),
            )
            .child(lists);

        let rows = self.shown_rows(cx).into_iter().cloned().collect::<Vec<_>>();
        let mut body: Vec<AnyElement> = Vec::new();
        for group in Group::ALL {
            let mine: Vec<&InboxRow> = rows.iter().filter(|row| row.group == group).collect();
            if mine.is_empty() && group != Group::Done {
                continue;
            }
            let folded = self.folded.contains(&group);
            body.push(self.group_head(group, folded, body.is_empty(), look, cx));
            if folded {
                continue;
            }
            for (at, row) in mine.into_iter().enumerate() {
                body.push(self.inbox_row(row, at > 0, look, cx));
            }
        }
        if rows.is_empty() {
            body.push(
                labelled(
                    div().id("inbox-none").p(look.length(token::SPACE_4)).text_size(look.length(token::TEXT_SM)).text_color(muted).child("No pull request matches"),
                    Role::Status,
                    "No pull request matches",
                )
                .into_any_element(),
            );
        }
        let _ = window;
        labelled(
            panel(look)
                .id("inbox")
                .flex_none()
                .w(look.length(token::W_GUIDE_LIST))
                .h_full()
                .flex()
                .flex_col()
                .child(head)
                .child(search)
                .child(
                    labelled(
                        div()
                            .id("inbox-rows")
                            .key_context("list")
                            .track_focus(&self.rows)
                            .on_action(cx.listener(Self::on_act))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .p(look.length(token::SPACE_3))
                            .when(refreshing, |rows| rows.opacity(0.75))
                            .children(body),
                        Role::List,
                        "Pull requests",
                    ),
                ),
            Role::Region,
            "Inbox",
        )
        .into_any_element()
    }

    /// A group's heading, as the Jobs list heads a project, with its fold and its state's glyph.
    fn group_head(&self, group: Group, folded: bool, first: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ink = look.color(hue(group));
        let name = group.said();
        let hover = look.color(token::FG_DEFAULT);
        labelled(
            group_line(look, first)
                .id(SharedString::from(format!("group-{name}")))
                .gap(look.length(token::SPACE_2))
                .cursor_pointer()
                .hover(move |style| style.text_color(hover))
                .aria_expanded(!folded)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.folded.remove(&group) {
                        this.folded.insert(group);
                    }
                    cx.notify();
                }))
                .child(glyph(if folded { "chevron-right" } else { "chevron-down" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(glyph(group.glyph(), look.length(token::SPACE_3), ink))
                .child(name),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    /// A pull request's row, as the Jobs list draws a Job's: its card edged in its group's hue.
    fn inbox_row(&self, row: &InboxRow, after_row: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let number = row.number;
        let open = self.open == Some(number) || self.waiting == Some(number);
        let focused = self.focused == Some(number);
        let lamp = look.color(hue(row.group));
        let mut meta = format!("#{number} · {}", row.short_repository());
        if let Some(count) = row.new_commits {
            meta.push_str(&if count == 1 { " · 1 new commit".to_string() } else { format!(" · {count} new commits") });
        }
        meta.push_str(&format!(" · {}", row.age));
        let checks = checks_mark(row.checks).map(|(name, ink, said)| {
            labelled(
                div().id(SharedString::from(format!("checks-{number}"))).tooltip(tip(said)).child(glyph(name, look.length(token::SPACE_4), look.color(ink))),
                Role::Image,
                said,
            )
        });
        let weight = if open { token::WEIGHT_HEADING } else { token::WEIGHT_BODY };
        let card = row_card(look, SharedString::from(format!("pr-row-{number}")), open, focused, true, look.length(token::SPACE_2))
            .cursor_pointer()
            .aria_selected(open)
            .on_click(cx.listener(move |this, _, window, cx| this.press_row(number, window, cx)))
            .when(!open, |card| card.child(row_wash(look, lamp)))
            .child(row_edge(look, lamp))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(weight)))
                            .child(SharedString::from(row.title.clone())),
                    )
                    // The checks' column keeps its width on every row, so the titles line up whether it is marked or not.
                    .child(div().flex_none().w(look.length(token::SPACE_4)).children(checks)),
            )
            .child(labelled(
                small(look).id(SharedString::from(format!("meta-{number}"))).text_color(look.color(token::FG_MUTED)).truncate().child(SharedString::from(meta.clone())),
                Role::Label,
                meta,
            ));
        div()
            .flex_none()
            .flex()
            .when(after_row, |line| line.mt(look.length(token::SPACE_2)))
            .child(labelled(card, Role::Row, row.name()))
            .into_any_element()
    }
}
