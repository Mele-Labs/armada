//! The inbox column: Search or ask, the Lists filter, and the pull requests in their four groups.
//! A refresh keeps the last inbox drawn under a Refreshing mark.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::PullReviewChecks;

use super::model::{Group, InboxRow, Verdict};
use super::{panel, ReviewsScreen};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, icon_button, mono, small, spinner, tip};

/// A group's hue, on its header band and the glyph of each row's state.
pub(super) fn hue(group: Group) -> crate::look::Token {
    match group {
        Group::FirstLook => token::STATUS_AWAITING_REVIEW,
        Group::Changed => token::STATUS_HANDED_IN,
        Group::Waiting => token::STATUS_NOT_STARTED,
        Group::Done => token::STATUS_COMPLETED_SUCCESS,
    }
}

fn band(group: Group) -> crate::look::Token {
    match group {
        Group::FirstLook => token::STATUS_AWAITING_REVIEW_BG,
        Group::Changed => token::STATUS_HANDED_IN_BG,
        Group::Waiting => token::STATUS_NOT_STARTED_BG,
        Group::Done => token::STATUS_COMPLETED_SUCCESS_BG,
    }
}

/// The checks' state as a registry glyph, its hue and what its tooltip says.
pub(super) fn checks_mark(checks: PullReviewChecks) -> (&'static str, crate::look::Token, &'static str) {
    match checks {
        PullReviewChecks::Passed => ("shield-check", token::RUN_PASSED, "Checks passed"),
        PullReviewChecks::Failed => ("shield-x", token::RUN_FAILED, "Checks failed"),
        PullReviewChecks::Running => ("loader-circle", token::RUN_RUNNING, "Checks running"),
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
        let hover = look.color(token::BG_HOVER);
        let lists = labelled(
            div()
                .id("inbox-lists")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .tooltip(tip("The forge's lists: requested of you, of your team, assigned, your own"))
                .on_click(cx.listener({
                    let name = lists_name.clone();
                    move |this, event: &gpui::ClickEvent, window, cx| {
                        let at = super::under_label(Role::Button, &name, event.position(), window, cx);
                        this.open_lists(at, window, cx);
                    }
                }))
                .child(glyph("filter", look.length(token::SPACE_3), muted))
                .child(SharedString::from(lists_name.clone()))
                .child(glyph("chevron-down", look.length(token::SPACE_3), muted)),
            Role::Button,
            lists_name,
        );
        let search = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                labelled(
                    div()
                        .id("inbox-search")
                        .flex_1()
                        .min_w_0()
                        .key_context(crate::keymap::TEXT_BOX)
                        .text_size(look.length(token::TEXT_SM))
                        .child(gpui_component::input::Input::new(&self.search).prefix(glyph("search", look.length(token::SPACE_4), muted))),
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
            body.push(self.group_head(group, folded, look, cx));
            if folded {
                continue;
            }
            for row in mine {
                body.push(self.inbox_row(row, look, cx));
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

    fn group_head(&self, group: Group, folded: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ink = look.color(hue(group));
        let name = group.said();
        let hover = look.color(token::BG_HOVER);
        labelled(
            div()
                .id(SharedString::from(format!("group-{name}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_ROW_HEADER))
                .px(look.length(token::SPACE_3))
                .bg(look.color(band(group)))
                .border_l(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(ink)
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .aria_expanded(!folded)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.folded.remove(&group) {
                        this.folded.insert(group);
                    }
                    cx.notify();
                }))
                .child(glyph(if folded { "chevron-right" } else { "chevron-down" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(glyph(group.glyph(), look.length(token::SPACE_4), ink))
                .child(name),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    fn inbox_row(&self, row: &InboxRow, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let number = row.number;
        let open = self.open == Some(number) || self.waiting == Some(number);
        let focused = self.focused == Some(number);
        let (checks_glyph, checks_hue, checks_said) = checks_mark(row.checks);
        let hover = look.color(token::BG_HOVER);
        let bar = look.color(token::ROW_FOCUS_BAR);
        let chip = row.new_commits.map(|count| {
            let said = if count == 1 { "+1 commit".to_string() } else { format!("+{count} commits") };
            labelled(
                div()
                    .id(SharedString::from(format!("chip-{number}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::STATUS_HANDED_IN_BG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::STATUS_HANDED_IN))
                    .text_color(look.color(token::STATUS_HANDED_IN))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .tooltip(tip("Commits pushed since you looked"))
                    .child(SharedString::from(said.clone())),
                Role::Label,
                said,
            )
        });
        let meta = format!("#{number} · {} · {}", row.short_repository(), row.age);
        let mine_said = format!("Your review: {}", row.mine.said());
        labelled(
            div()
                .id(SharedString::from(format!("pr-row-{number}")))
                .flex_none()
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .border_l(look.length(token::BORDER_WIDTH) * 2.)
                .when(open, |line| line.bg(look.color(token::SURFACE_ROW_SELECTED)).border_color(bar))
                .when(focused && !open, |line| line.border_color(bar))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .aria_selected(open)
                .on_click(cx.listener(move |this, _, window, cx| this.press_row(number, window, cx)))
                .child(
                    // The checks' state, in a column of its own on every row.
                    labelled(
                        div().id(SharedString::from(format!("checks-{number}"))).flex_none().pt(look.length(token::SPACE_1)).tooltip(tip(checks_said)).child(glyph(
                            checks_glyph,
                            look.length(token::SPACE_4),
                            look.color(checks_hue),
                        )),
                        Role::Image,
                        checks_said,
                    ),
                )
                .child(
                    // His review's state, in the next column.
                    labelled(
                        div().id(SharedString::from(format!("mine-{number}"))).flex_none().pt(look.length(token::SPACE_1)).tooltip(tip(mine_said.clone())).child(glyph(
                            row.mine.glyph(),
                            look.length(token::SPACE_4),
                            look.color(verdict_hue(row.mine).0),
                        )),
                        Role::Image,
                        mine_said,
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .child(
                            div()
                                .text_size(look.length(token::TEXT_SM))
                                .line_height(look.length(token::LEADING_SM))
                                .when(open, |title| title.font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                                .child(SharedString::from(row.title.clone())),
                        )
                        .child(
                            // The meta line gives way first, so the chip always shows whole at its end.
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .child(small(look).flex_1().min_w_0().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).truncate().child(SharedString::from(meta)))
                                .children(chip),
                        ),
                ),
            Role::Row,
            row.name(),
        )
        .into_any_element()
    }
}
