//! What each tab of a pull request's page draws. Every tab reads the page as Fleet sends it.

use gpui::{anchored, deferred, div, prelude::*, Animation, AnimationExt, AnyElement, Context, Div, ElementId, FontWeight, Role, SharedString, Toggled, Transformation};
use ipc::{PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestVerdict};

use gpui_component::input::{Input, Textarea};

use super::live::Find;
use super::model::{found_nothing, named, on_day, Answer, ChangeSet, Critique, Effort, Explain, Finding, Flag, FlagKind, Kept, Mark, Page, PageFile, Practice, PracticeAt, ReviewAsk, Rewrite, Summary, Viewed, Wrong};
use super::panes::{grey, place_line};
use super::{AskingReview, ReviewsScreen, HUNKS_AT};
use crate::diff;
use crate::job_view::pulls::state_chip;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{card, glyph, icon_button, mono, spinner, tip, Btn, Weight};

/// The glyph on a tab and in the Add tab menu.
pub(super) fn glyph_of(kind: &str) -> &'static str {
    match kind {
        "pr-changed" => "git-compare",
        "pr-files" => "file-diff",
        "pr-conversation" => "message-square",
        "pr-checks" => "list-checks",
        "pr-findings" => "bot",
        "pr-comments" => "message-square-reply",
        "pr-overview" => "layers",
        "pr-change-set" => "list-tree",
        _ => "file-text",
    }
}

fn check_mark(state: PullRequestCheckState) -> (&'static str, Token, &'static str) {
    match state {
        PullRequestCheckState::Passed => ("check", token::RUN_PASSED, "Passed"),
        PullRequestCheckState::Failed => ("x", token::RUN_FAILED, "Failed"),
        PullRequestCheckState::Running => ("loader", token::STATUS_AWAITING_REVIEW, "Running"),
        PullRequestCheckState::Skipped => ("shield-off", token::FG_SUBTLE, "Skipped"),
    }
}

fn file_mark(status: PullRequestFileStatus) -> (&'static str, Token, &'static str) {
    match status {
        PullRequestFileStatus::Added => ("plus", token::RUN_PASSED, "Added"),
        PullRequestFileStatus::Removed => ("x", token::RUN_FAILED, "Removed"),
        PullRequestFileStatus::Modified => ("pencil", token::FG_MUTED, "Modified"),
        PullRequestFileStatus::Renamed => ("file-text", token::FG_MUTED, "Renamed"),
    }
}

/// A check's state as its glyph, turning slowly while it runs as the inbox's does; gpui draws it
/// still when reduce motion is on.
fn check_lead(id: String, state: PullRequestCheckState, look: &Look) -> AnyElement {
    let (name, ink, said) = check_mark(state);
    if state != PullRequestCheckState::Running {
        return mark(id, name, ink, said, look);
    }
    let turning = glyph(name, look.length(token::SPACE_4), look.color(ink)).with_animation(
        SharedString::from(format!("{id}-turn")),
        Animation::new(look.duration(token::DURATION_PULSE)).repeat(),
        |mark, at| mark.with_transformation(Transformation::rotate(gpui::percentage(at))),
    );
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said)).child(turning), Role::Image, said).into_any_element()
}

/// A check's state in words at a row's end: in its hue once settled, muted while it runs.
fn check_words(state: PullRequestCheckState, said: String, look: &Look) -> AnyElement {
    let (_, ink, _) = check_mark(state);
    let ink = if state == PullRequestCheckState::Running { token::FG_MUTED } else { ink };
    div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(ink)).child(SharedString::from(said)).into_any_element()
}

/// A state as a registry glyph that says itself in a tooltip.
fn mark(id: String, name: &'static str, ink: Token, said: &'static str, look: &Look) -> AnyElement {
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said)).child(glyph(name, look.length(token::SPACE_4), look.color(ink))), Role::Image, said).into_any_element()
}

fn heading(id: impl Into<SharedString>, words: String, look: &Look) -> AnyElement {
    heading_tipped(id, words, None, look)
}

/// A heading whose commits are said only in its tooltip.
fn heading_tipped(id: impl Into<SharedString>, words: String, said: Option<String>, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(ElementId::Name(id.into()))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .when_some(said, |head, said| head.tooltip(tip(said)))
            .child(SharedString::from(words.clone())),
        Role::Heading,
        words,
    )
    .into_any_element()
}

fn words(id: impl Into<SharedString>, said: String, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(ElementId::Name(id.into()))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(SharedString::from(said.clone())),
        Role::Label,
        said,
    )
    .into_any_element()
}

fn status(id: impl Into<SharedString>, said: String, lead: AnyElement, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(ElementId::Name(id.into()))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_MUTED))
            // The lead sits on the first line; the words wrap in a narrow pane.
            .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(lead))
            .child(div().flex_1().min_w_0().child(SharedString::from(said.clone()))),
        Role::Status,
        said,
    )
    .into_any_element()
}

/// A chip with its own fill.
fn chip(id: String, said: String, ink: Token, ground: Token, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(id))
            .flex_none()
            .px(look.length(token::SPACE_2))
            .h(look.length(token::H_BADGE))
            .flex()
            .items_center()
            .rounded(look.length(token::RADIUS_SM))
            .bg(look.color(ground))
            .text_color(look.color(ink))
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .child(SharedString::from(said.clone())),
        Role::Label,
        said,
    )
    .into_any_element()
}

/// A section's header band, with weight.
fn band(id: String, title: &'static str, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(id))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .bg(look.color(token::BG_SUNKEN))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_MUTED))
            .child(title),
        Role::Heading,
        title,
    )
    .into_any_element()
}

/// One row of a table: a glyph column, a narrow fixed key column where the rows have one, what it
/// says in the rest of the width, and what stands at its end.
fn table_row(id: String, name: String, lead: AnyElement, key: Option<AnyElement>, rest: Vec<AnyElement>, end: Option<AnyElement>, said: Option<String>, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(id))
            .when_some(said, |row, said| row.tooltip(tip(said)))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_SM))
            .child(div().flex_none().w(look.length(token::SPACE_4)).child(lead))
            .children(key.map(|key| div().flex_none().w(look.length(token::SPACE_12) * 2.).overflow_hidden().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(key)))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).children(rest))
            .children(end),
        Role::Row,
        name,
    )
    .into_any_element()
}

fn frame(look: &Look) -> Div {
    div().rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).overflow_hidden()
}

impl ReviewsScreen {
    pub(super) fn tab_body(&mut self, kind: &str, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        // The change set scrolls itself, so Show in change set can bring a hunk to its top.
        if kind == "pr-change-set" {
            return self.change_set_tab(page, look, cx);
        }
        let inner = match kind {
            "pr-overview" => self.overview_tab(page, look, cx),
            "pr-changed" => self.changed_tab(page, look, cx),
            "pr-files" => self.files_tab(page, look, cx),
            "pr-conversation" => conversation_tab(page, look),
            "pr-checks" => checks_tab(page, look),
            "pr-findings" => self.findings_tab(page, look, cx),
            "pr-comments" => comments_tab(page, look),
            _ => div().into_any_element(),
        };
        div().w_full().p(look.length(token::SPACE_4)).flex().flex_col().gap(look.length(token::SPACE_3)).child(inner).into_any_element()
    }

    /// Review this pull request: the picker, whose Start runs the change sets, the overview and each ticked tool.
    fn critique_button(&self, id: &'static str, name: &'static str, weight: Weight, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let finding = match self.live_finds(page.number) {
            Some(finds) => finds.sets == Find::Running || finds.flags == Find::Running,
            None => self.finding.contains_key(&page.number),
        };
        let running = finding || matches!(page.critique, Critique::Running { .. });
        labelled(
            Btn::new(id, name, weight).icon("bot").pending(running).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.open_review(name, window, cx))),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    /// The sheet under Review these changes: what to look for, how hard, and anything to focus on.
    pub(super) fn render_review_ask(&self, open: &AskingReview, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let caption = |id: &'static str, said: &'static str| {
            labelled(
                div()
                    .id(id)
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child(said),
                Role::Label,
                said,
            )
        };
        let muted = |id: &'static str, said: String| {
            labelled(
                div().id(id).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())),
                Role::Label,
                said,
            )
        };
        let heading = labelled(
            div()
                .id("review-ask-title")
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(open.title),
            Role::Heading,
            open.title,
        );
        let range = self.page().map_or_else(|| "the whole pull request".to_string(), |page| page.range().said);
        let mut tools = div().flex().flex_col().gap(look.length(token::SPACE_1));
        for tool in &self.review_tools {
            let on = open.ask.has(&tool.id);
            let ink = if on { look.color(token::ACCENT) } else { look.color(token::FG_MUTED) };
            let id = tool.id.clone();
            tools = tools.child(labelled(
                div()
                    .id(SharedString::from(format!("review-tool-{}", tool.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_SM))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_review_tool(&id, cx)))
                    .child(glyph(if on { "square-check" } else { "square" }, look.length(token::SPACE_4), ink))
                    .child(SharedString::from(tool.name.clone())),
                Role::CheckBox,
                tool.name.clone(),
            ));
        }
        let picked = open.ask.effort;
        let mut efforts = div().id("review-effort").flex().p(look.length(token::SPACE_1)).gap(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_SUNKEN));
        for effort in Effort::ALL {
            let on = effort == picked;
            let ink = if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
            efforts = efforts.child(labelled(
                div()
                    .id(SharedString::from(format!("review-effort-{}", effort.name())))
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(look.length(token::H_CONTROL))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(ink)
                    .when(on, |radio| radio.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .tooltip(tip(effort.costs()))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_effort(effort, cx)))
                    .child(effort.name()),
                Role::RadioButton,
                effort.name(),
            ));
        }
        let focus = labelled(div().id("review-ask-focus").key_context(crate::keymap::TEXT_BOX).child(Input::new(&open.focus)), Role::TextInput, "Anything to focus on?");
        let start = labelled(
            Btn::new("review-ask-start", "Start", Weight::Primary).icon("bot").draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.start_review(window, cx))),
            Role::Button,
            "Start",
        );
        let cancel = labelled(
            Btn::new("review-ask-cancel", "Cancel", Weight::Ghost).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.cancel_review(window, cx))),
            Role::Button,
            "Cancel",
        );
        let sheet = labelled(
            card(look)
                .id("review-ask")
                .track_focus(&self.dialog)
                .occlude()
                .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    if crate::room::of(window, cx).contains(&event.position) {
                        this.cancel_review(window, cx);
                    }
                }))
                .w(look.length(token::W_DIALOG))
                .p(look.length(token::SPACE_4))
                .gap(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(look.color(token::ACCENT))
                .child(heading)
                .child(muted("review-ask-range", format!("Reads {range}. Change sets and problems come to the Overview, and each tool's suggestions to Review comments.")))
                .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption("review-ask-what", "What to look for")).child(tools).when(self.review_tools.is_empty(), |list| {
                    list.child(muted("review-ask-no-tools", "No review tool is set up for this repository".to_string()))
                }))
                // Fleet's review runs take no effort and no focus, so the live sheet does not ask for them.
                .when(!self.is_live(), |sheet| {
                    sheet
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_1))
                                .child(caption("review-ask-effort", "Effort"))
                                .child(labelled(efforts, Role::RadioGroup, "Effort"))
                                .child(muted("review-ask-costs", picked.costs().to_string())),
                        )
                        .child(div().flex().flex_col().gap(look.length(token::SPACE_1)).child(caption("review-ask-focus-label", "Anything to focus on?")).child(focus))
                })
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(start)),
            Role::Dialog,
            open.title,
        );
        deferred(anchored().position(open.at).snap_to_window_with_margin(look.length(token::SPACE_3)).child(sheet)).with_priority(1).into_any_element()
    }

    /// What changed since he looked: new commits, the files they touched, replies to him, and checks.
    fn changed_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let Some(seen) = page.seen.clone() else {
            return status("changed-never", "You have not opened this pull request before".into(), glyph("eye", look.length(token::SPACE_4), look.color(token::FG_MUTED)).into_any_element(), look);
        };
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3));
        if seen.gone {
            column = column.child(heading_tipped("changed-head", format!("Since you looked {}", seen.when()), Some(seen.commits()), look));
            let said = "The commit you looked at is gone after a force push, so there is no diff since you looked.".to_string();
            let notice = labelled(
                div()
                    .id("changed-gone")
                    .tooltip(tip(seen.commits()))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::NOTICE_CAUTION))
                    .bg(look.color(token::NOTICE_CAUTION_BG))
                    .text_size(look.length(token::TEXT_SM))
                    .child(glyph("triangle-alert", look.length(token::SPACE_4), look.color(token::NOTICE_CAUTION)))
                    .child(SharedString::from(said.clone())),
                Role::Status,
                said,
            );
            let whole = labelled(
                Btn::new("changed-read-whole", "Read the whole PR", Weight::Secondary).icon("file-diff").draw(look, cx).on_click(cx.listener(|this, _, _, cx| {
                    this.whole = true;
                    this.bring_kind("pr-files", cx);
                })),
                Role::Button,
                "Read the whole PR",
            );
            return column
                .child(notice)
                .child(div().flex().gap(look.length(token::SPACE_2)).child(self.critique_button("changed-critique-whole", "Review the whole PR", Weight::Primary, page, look, cx)).child(whole))
                .into_any_element();
        }
        column = column.child(heading_tipped("changed-head", format!("Since you looked {}", seen.when()), Some(seen.commits()), look));
        column = column.children(self.summary_part(page, look, cx));
        let mut table = frame(look).flex().flex_col();
        table = table.child(band("changed-band-commits".into(), "New commits", look));
        if page.kept {
            table = table.child(waiting("changed-commits-wait", "Reading the commits since you looked", &[0.7, 0.5], look));
        }
        for commit in &page.new_commits {
            let by = [commit.author.clone(), commit.at.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
            let rest = vec![
                div().child(SharedString::from(commit.headline.clone())).into_any_element(),
                div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(by)).into_any_element(),
            ];
            let lead = glyph("git-commit-horizontal", look.length(token::SPACE_4), look.color(token::STATUS_HANDED_IN)).into_any_element();
            table = table.child(table_row(format!("changed-commit-{}", commit.sha), commit.headline.clone(), lead, None, rest, None, Some(format!("Commit {}", commit.sha)), look));
        }
        table = table.child(band("changed-band-files".into(), "Files changed", look));
        for (at, file) in page.files.iter().filter(|file| file.since).enumerate() {
            let (name, ink, said) = file_mark(file.status);
            let key = div()
                .flex()
                .gap(look.length(token::SPACE_2))
                .font_family(mono(look))
                .child(div().text_color(look.color(token::RUN_PASSED)).child(SharedString::from(format!("+{}", file.additions))))
                .child(div().text_color(look.color(token::RUN_FAILED)).child(SharedString::from(format!("−{}", file.deletions))))
                .into_any_element();
            let mut rest = vec![div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).child(SharedString::from(file.path.clone())).into_any_element()];
            if file.viewed == Viewed::ChangedAfter {
                // In a row of its own, so the chip keeps its own width.
                rest.push(div().flex().child(chip(format!("changed-after-{at}"), "You had viewed it".into(), token::STATUS_HANDED_IN, token::STATUS_HANDED_IN_BG, look)).into_any_element());
            }
            table = table.child(table_row(format!("changed-file-{at}"), file.path.clone(), mark(format!("changed-file-mark-{at}"), name, ink, said, look), Some(key), rest, None, None, look));
        }
        if page.kept {
            table = table.child(band("changed-band-replies".into(), "Replies to your comments", look));
            table = table.child(waiting("changed-replies-wait", "Reading replies to you", &[0.6], look));
        }
        if !page.replies.is_empty() {
            table = table.child(band("changed-band-replies".into(), "Replies to your comments", look));
            for (at, reply) in page.replies.iter().enumerate() {
                let place = format!("{}:{}", reply.path, reply.line);
                let rest = vec![
                    div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(place.clone())).into_any_element(),
                    div().child(SharedString::from(reply.body.clone())).into_any_element(),
                ];
                let key = div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).truncate().child(SharedString::from(reply.author.clone())).into_any_element();
                let lead = glyph("message-square-reply", look.length(token::SPACE_4), look.color(token::ACCENT)).into_any_element();
                table = table.child(table_row(format!("changed-reply-{at}"), format!("Reply on {place}"), lead, Some(key), rest, None, None, look));
            }
        }
        table = table.child(band("changed-band-checks".into(), "Checks", look));
        if page.kept {
            table = table.child(waiting("changed-checks-wait", "Reading checks", &[0.4, 0.5, 0.3], look));
        }
        for (at, check) in page.checks.iter().enumerate() {
            let (_, _, said) = check_mark(check.state);
            let state = match (check.state, check.seconds) {
                (PullRequestCheckState::Running, Some(seconds)) if seconds >= 60 => format!("Running, {} min", seconds / 60),
                _ => said.to_string(),
            };
            let rest = vec![div().child(SharedString::from(check.name.clone())).into_any_element()];
            let row_name = format!("{}: {state}", check.name);
            let end = check_words(check.state, state, look);
            table = table.child(table_row(format!("changed-check-{at}"), row_name, check_lead(format!("changed-check-mark-{at}"), check.state, look), None, rest, Some(end), None, look));
        }
        let table = labelled(table.id("changed-table"), Role::Table, "What changed");
        let diff = labelled(
            Btn::new("changed-diff", "Diff since you looked", Weight::Secondary).icon("git-compare").draw(look, cx).on_click(cx.listener(|this, _, _, cx| this.diff_since(cx))),
            Role::Button,
            "Diff since you looked",
        );
        column
            .child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).child(self.critique_button("changed-critique", "Review these changes", Weight::Primary, page, look, cx)).child(diff))
            .child(table)
            .into_any_element()
    }

    /// Files: the change since he looked, or the whole pull request, each file with its Viewed box.
    fn files_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let gone = page.seen.as_ref().is_none_or(|seen| seen.gone);
        let whole = self.whole || gone;
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3));
        if page.seen.is_some() {
            let radio = |id: &'static str, said: &'static str, on: bool, off: bool, to_whole: bool, cx: &mut Context<Self>| {
                let ink = if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
                let element = div()
                    .id(id)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(ink)
                    .when(on, |radio| radio.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .when(off, |radio| radio.opacity(0.5).tooltip(tip("The commit you looked at is gone")))
                    .when(!off, |radio| radio.cursor_pointer())
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !off {
                            this.whole = to_whole;
                            cx.notify();
                        }
                    }))
                    .child(said);
                labelled(element, Role::RadioButton, said)
            };
            let since = radio("files-since", "Since you looked", !whole, gone, false, cx);
            let all = radio("files-whole", "Whole PR", whole, false, true, cx);
            column = column.child(labelled(
                div().id("files-range").flex().p(look.length(token::SPACE_1)).gap(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_SUNKEN)).child(since).child(all),
                Role::RadioGroup,
                "Diff range",
            ));
        }
        let files: Vec<&PageFile> = page.files.iter().filter(|file| whole || file.since).collect();
        for (at, file) in files.into_iter().enumerate() {
            column = column.child(self.file_card(at, file, whole, look, cx));
        }
        column.into_any_element()
    }

    fn file_card(&self, at: usize, file: &PageFile, whole: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let viewed = file.viewed == Viewed::Yes;
        let path = file.path.clone();
        let tick_ink = if viewed { look.color(token::ACCENT) } else { look.color(token::FG_MUTED) };
        let tick = labelled(
            div()
                .id(SharedString::from(format!("file-viewed-{at}")))
                .flex_none()
                .w(look.length(token::SPACE_12) + look.length(token::SPACE_6))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_XS))
                .text_color(tick_ink)
                .aria_toggled(if viewed { Toggled::True } else { Toggled::False })
                .on_click(cx.listener(move |this, _, window, cx| {
                    // The row under it opens and shuts the file; the box only marks it viewed.
                    cx.stop_propagation();
                    this.toggle_viewed(&path, window, cx);
                }))
                .child(glyph(if viewed { "check" } else { "square" }, look.length(token::SPACE_4), tick_ink))
                .child("Viewed"),
            Role::CheckBox,
            format!("Viewed: {}", file.path),
        );
        let (name, ink, said) = file_mark(file.status);
        let open = self.file_open(&file.path, viewed);
        let path = file.path.clone();
        let head = div()
            .id(SharedString::from(format!("file-head-{at}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .min_h(look.length(token::H_CONTROL_LG))
            .bg(look.color(token::BG_SUNKEN))
            .cursor_pointer()
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_file(&path, viewed, cx)))
            .when(open, |head| head.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
            .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
            // Viewed, the file's mark and its path share a line with Changed after you viewed it when they fit. When
            // they do not, the chip wraps to a line of its own under them, so the path is never squeezed out; short
            // of room for the path alone, it gives way from the left so the file's name stays.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x(look.length(token::SPACE_3))
                    .gap_y(look.length(token::SPACE_1))
                    .child(
                        div()
                            .flex_auto()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .child(tick)
                            .child(mark(format!("file-mark-{at}"), name, ink, said, look))
                            .child(
                                div()
                                    .flex_auto()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis_start()
                                    .font_family(mono(look))
                                    .text_size(look.length(token::TEXT_XS))
                                    .child(SharedString::from(file.path.clone())),
                            ),
                    )
                    .children((file.viewed == Viewed::ChangedAfter).then(|| chip(format!("file-after-{at}"), "Changed after you viewed it".into(), token::STATUS_HANDED_IN, token::STATUS_HANDED_IN_BG, look))),
            )
            .child(
                div()
                    .flex_none()
                    .w(look.length(token::SPACE_12))
                    .flex()
                    .justify_end()
                    .gap(look.length(token::SPACE_1))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().text_color(look.color(token::RUN_PASSED)).child(SharedString::from(format!("+{}", file.additions))))
                    .child(div().text_color(look.color(token::RUN_FAILED)).child(SharedString::from(format!("−{}", file.deletions)))),
            );
        let head = labelled(head, Role::Button, file.path.clone());
        let body = open.then(|| {
            let patch = if whole { file.patch.as_str() } else { file.patch_since.as_deref().unwrap_or(&file.patch) };
            let drawn = diff::file_diff_of(&file.path, Some(patch), None);
            let lines = drawn.files.first().map(|one| one.lines.clone()).unwrap_or_default();
            let slots = diff::Slots { hues: self.hues_of(&file.path, &lines), ..Default::default() };
            diff::lines_with(SharedString::from(format!("file-diff-{at}-{whole}")), &lines, look, slots)
        });
        labelled(frame(look).id(SharedString::from(format!("file-{at}"))).flex().flex_col().child(head).children(body), Role::Article, format!("File: {}", file.path)).into_any_element()
    }

    /// Review comments: his draft comments, then the suggestions a critique or a tool made, each
    /// editable before it becomes a draft, then what already went, folded. Then the notes for the
    /// Job that holds the pull request.
    fn findings_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let drafts = self.drafts_part(page, look, cx);
        let found = self.critique_part(page, look, cx);
        let judged = self.judge_section(page, look, cx);
        let notes = page.holder.clone().map(|holder| {
            let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE));
            column = column.child(heading("notes-head", format!("Notes for {holder}"), look));
            for (at, note) in page.notes.iter().enumerate() {
                column = column.child(words(format!("note-{at}"), note.clone(), look));
            }
            column.child(if self.sent_notes.contains(&page.number) {
                status("notes-sent", format!("Sent to {holder}"), glyph("check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look)
            } else {
                labelled(
                    Btn::new("notes-send", "Send to job", Weight::Secondary).icon("send").draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.send_notes(window, cx))),
                    Role::Button,
                    "Send to job",
                )
                .into_any_element()
            })
        });
        div().flex().flex_col().gap(look.length(token::SPACE_4)).child(drafts).child(found).children(judged).children(notes).into_any_element()
    }

    /// His draft comments on this pull request, each in its own box, with an amber edge.
    fn drafts_part(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading("drafts-head", "Your draft comments. They go with your review.".into(), look));
        let mine: Vec<_> = self.drafts.iter().filter(|draft| draft.number == page.number).collect();
        if mine.is_empty() {
            column = column.child(words("drafts-none", "No draft comments".into(), look));
        }
        for draft in mine {
            let Some(boxed) = self.boxes.get(&draft.id) else { continue };
            let place = draft.place();
            let (edit_id, remove_id) = (draft.id.clone(), draft.id.clone());
            let edit = labelled(
                Btn::new(SharedString::from(format!("draft-edit-{}", draft.id)), "Edit", Weight::Ghost)
                    .draw(look, cx)
                    .on_click(cx.listener(move |this, _, window, cx| this.edit_draft(&edit_id, window, cx))),
                Role::Button,
                format!("Edit draft comment on {place}"),
            );
            let remove = labelled(
                Btn::new(SharedString::from(format!("draft-remove-{}", draft.id)), "Remove", Weight::Ghost)
                    .draw(look, cx)
                    .on_click(cx.listener(move |this, _, _, cx| this.discard_draft(&remove_id, cx))),
                Role::Button,
                format!("Remove draft comment on {place}"),
            );
            let body = div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(place_line(format!("draft-card-place-{}", draft.id), place.clone(), look))
                .children(draft.excerpt.as_deref().map(|lines| {
                    // The commented line is the excerpt's last, picked as a change set's hot line is.
                    let slots = diff::Slots { picked: lines.len().checked_sub(1), wrap: true, hues: self.hues_of(&draft.path, lines), ..Default::default() };
                    labelled(
                        frame(look).id(SharedString::from(format!("draft-card-diff-{}", draft.id))).child(diff::lines_with(SharedString::from(format!("draft-diff-{}", draft.id)), lines, look, slots)),
                        Role::Article,
                        format!("Diff at {place}"),
                    )
                }))
                .child(labelled(
                    div().id(SharedString::from(format!("draft-card-box-{}", draft.id))).key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&boxed.state)),
                    Role::TextInput,
                    format!("Draft comment on {place}"),
                ))
                // Words without glyphs, so both sit on one row in the side pane.
                .child(div().flex().items_center().flex_wrap().gap(look.length(token::SPACE_1)).child(edit).child(remove));
            column = column.child(edged(format!("draft-card-{}", draft.id), token::STATUS_AWAITING_REVIEW, body, format!("Draft comment on {place}"), look));
        }
        column.into_any_element()
    }

    fn critique_part(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let muted = look.color(token::FG_MUTED);
        // What he asked the review to look for and how hard, under its status or its heading.
        let asked = |asked: &ReviewAsk| {
            // Fleet's runs take no effort, so a live ask says only what it looked for.
            let said = if self.is_live() { asked.what(&self.review_tools) } else { asked.said(&self.review_tools) };
            let said = if asked.focus.is_empty() { said } else { format!("{said}. Focus: {}", asked.focus) };
            labelled(div().id("findings-asked").text_size(look.length(token::TEXT_XS)).text_color(muted).child(SharedString::from(said.clone())), Role::Label, said)
        };
        // A tool's run that failed or was refused says so above whatever the others found.
        let failed_said: Vec<String> = self.live_finds(page.number).map(|finds| finds.critique_failed.clone()).unwrap_or_default();
        let failed: Vec<AnyElement> = failed_said
            .iter()
            .cloned()
            .enumerate()
            .map(|(n, said)| status(format!("findings-failed-{n}"), said, glyph("circle-x", look.length(token::SPACE_4), look.color(token::RUN_FAILED)).into_any_element(), look))
            .collect();
        let part = match &page.critique {
            Critique::None => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(status("findings-none", "No suggestions on this pull request".into(), glyph("bot", look.length(token::SPACE_4), muted).into_any_element(), look))
                .child(div().flex().child(self.critique_button("findings-critique", "Review this pull request", Weight::Primary, page, look, cx)))
                .into_any_element(),
            Critique::Running { range, asked: ask } => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(status("findings-running", "Review running".into(), spinner("findings-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look))
                .child(labelled(
                    div()
                        .id("findings-range")
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(muted)
                        .tooltip(tip(range.commits.clone()))
                        .child(SharedString::from(format!("Reading {}", range.said))),
                    Role::Label,
                    format!("Reading {}", range.said),
                ))
                .child(asked(ask))
                .into_any_element(),
            Critique::Finished { range, findings, asked: ask } => {
                let head = div().flex().flex_col().gap(look.length(token::SPACE_1)).child(heading_tipped("findings-head", format!("Suggestions on {}", range.said), Some(range.commits.clone()), look)).child(asked(ask));
                let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(head);
                // What already went sits folded under the heading, where a long list cannot push it out of sight.
                column = column.children(self.sent_part(page, look, cx));
                // A drafted one is among the drafts above, a sent one in its folded line below.
                let open: Vec<_> = findings
                    .iter()
                    .filter(|finding| {
                        let key = (page.number, finding.id.clone());
                        !self.dismissed.contains(&key)
                            && !self.sent_findings.contains_key(&key)
                            && !self.drafts.iter().any(|draft| draft.number == page.number && draft.finding.as_deref() == Some(&finding.id))
                    })
                    .collect();
                // A run that failed never says nothing was found: what failed says so above.
                let clean = self.live_finds(page.number).is_none_or(|finds| super::live::map::clean_critique(&page.critique, finds));
                if found_nothing(findings, &failed_said) && clean {
                    column = column.child(self.clean_card("findings-clean", page, look, cx));
                } else if findings.is_empty() {
                    column = column.child(words("findings-empty", "No suggestions".into(), look));
                } else if open.is_empty() {
                    column = column.child(words("findings-done", "No suggestions left".into(), look));
                }
                for finding in open {
                    let Some(boxed) = self.suggestions.get(&(page.number, finding.id.clone())) else { continue };
                    let (add_id, dismiss_id) = (finding.id.clone(), finding.id.clone());
                    let add = labelled(
                        Btn::new(SharedString::from(format!("finding-add-{}", finding.id)), "Add as draft comment", Weight::Secondary)
                            .icon("plus")
                            .draw(look, cx)
                            .on_click(cx.listener(move |this, _, window, cx| this.add_draft(&add_id, window, cx))),
                        Role::Button,
                        "Add as draft comment",
                    );
                    let dismiss = labelled(
                        Btn::new(SharedString::from(format!("finding-dismiss-{}", finding.id)), "Dismiss", Weight::Ghost)
                            .draw(look, cx)
                            .on_click(cx.listener(move |this, _, window, cx| this.dismiss_suggestion(&dismiss_id, window, cx))),
                        Role::Button,
                        "Dismiss",
                    );
                    let place = format!("{}:{}", finding.path, finding.line);
                    let body = div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(finding.title.clone())))
                        .child(place_line(format!("finding-place-{}", finding.id), place, look))
                        .child(labelled(
                            div().id(SharedString::from(format!("finding-box-{}", finding.id))).key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&boxed.state)),
                            Role::TextInput,
                            format!("Suggestion: {}", finding.title),
                        ))
                        // The acts under the words, so a narrow pane keeps the text wide.
                        .child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).child(add).child(dismiss));
                    column = column.child(edged(format!("finding-{}", finding.id), token::ACCENT, body, format!("Suggestion: {}", finding.title), look));
                }
                column.into_any_element()
            }
        };
        if failed.is_empty() {
            return part;
        }
        div().flex().flex_col().gap(look.length(token::SPACE_3)).children(failed).child(part).into_any_element()
    }

    /// Where Check practices stands on a live change set, while its results are not in. None once
    /// they are in, and on the mock.
    fn practices_state(&self, page: &Page, set: &str, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let finds = self.live_finds(page.number)?;
        let find = finds.practices.get(set).cloned().unwrap_or_default();
        let press = |id: &'static str, name: &'static str, cx: &mut Context<Self>| {
            let set = set.to_string();
            labelled(
                Btn::new(id, name, Weight::Secondary).icon("list-checks").draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.check_practices(set.clone(), window, cx))),
                Role::Button,
                name,
            )
            .into_any_element()
        };
        let element = match find {
            Find::Done => return None,
            Find::Running => status("change-set-practices-running", "Checking practices".into(), spinner("change-set-practices-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look),
            Find::Failed(why) => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(status("change-set-practices-failed", format!("Check practices failed: {why}"), glyph("circle-x", look.length(token::SPACE_4), look.color(token::RUN_FAILED)).into_any_element(), look))
                .child(div().flex().child(press("change-set-practices-again", "Try again", cx)))
                .into_any_element(),
            Find::NotRun => div().flex().child(press("change-set-practices-check", "Check practices", cx)).into_any_element(),
        };
        Some(element)
    }

    /// Where the review's change sets and overview stand while their findings are not in: not run,
    /// running or failed, with the press that starts it. None once they are in, and on the mock unless
    /// it is running.
    fn find_problems_state(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(finds) = self.live_finds(page.number) else {
            return self.finding.contains_key(&page.number).then(|| {
                status("overview-flags-running", "Finding problems".into(), spinner("overview-flags-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look)
            });
        };
        if finds.flags == Find::Done {
            return None;
        }
        let press = |id: &'static str, name: &'static str, cx: &mut Context<Self>| {
            labelled(
                Btn::new(id, name, Weight::Secondary).icon("bot").draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.find_problems(window, cx))),
                Role::Button,
                name,
            )
            .into_any_element()
        };
        let failed = match (&finds.sets, &finds.flags) {
            (Find::Failed(why), _) | (_, Find::Failed(why)) => Some(why.clone()),
            _ => None,
        };
        let element = if finds.sets == Find::Running || finds.flags == Find::Running {
            status("overview-flags-running", "Finding problems".into(), spinner("overview-flags-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look)
        } else if let Some(why) = failed {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(status("overview-flags-failed", format!("Review failed: {why}"), glyph("circle-x", look.length(token::SPACE_4), look.color(token::RUN_FAILED)).into_any_element(), look))
                .child(div().flex().child(press("overview-find-again", "Try again", cx)))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(words("overview-flags-not-run", "No review has read this commit".into(), look))
                .child(div().flex().child(self.critique_button("overview-flags-critique", "Review this pull request", Weight::Secondary, page, look, cx)))
                .into_any_element()
        };
        Some(element)
    }

    /// What went with an earlier review, one folded line for each day it went.
    fn sent_part(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Critique::Finished { findings, .. } = &page.critique else { return None };
        let mut days: Vec<(String, Vec<&Finding>)> = Vec::new();
        for finding in findings {
            let Some(day) = self.sent_findings.get(&(page.number, finding.id.clone())) else { continue };
            match days.iter_mut().find(|(had, _)| had == day) {
                Some((_, sent)) => sent.push(finding),
                None => days.push((day.clone(), vec![finding])),
            }
        }
        if days.is_empty() {
            return None;
        }
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_1));
        for (day, sent) in days {
            let open = self.sent_open.contains(&(page.number, day.clone()));
            let said = format!("Sent with your review {}", on_day(&day));
            let toggle = day.clone();
            let hover = look.color(token::BG_HOVER);
            column = column.child(labelled(
                div()
                    .id(SharedString::from(format!("sent-{day}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .min_h(look.length(token::H_CONTROL))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .aria_expanded(open)
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_sent(&toggle, cx)))
                    .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)).flex_none())
                    .child(glyph("check", look.length(token::SPACE_3), look.color(token::RUN_PASSED)).flex_none())
                    // Its words wrap in a narrow pane rather than run past its edge.
                    .child(div().flex_1().min_w_0().child(SharedString::from(said.clone()))),
                Role::Button,
                said,
            ));
            if open {
                for finding in sent {
                    let place = format!("{}:{}", finding.path, finding.line);
                    column = column.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .pl(look.length(token::SPACE_6))
                            .child(div().text_size(look.length(token::TEXT_SM)).child(SharedString::from(finding.title.clone())))
                            .child(place_line(format!("sent-place-{}", finding.id), place, look)),
                    );
                }
            }
        }
        Some(column.into_any_element())
    }

    /// Neither the change sets nor the overview has run on this commit, nor started: the Overview offers
    /// to start the review.
    fn not_started(&self, page: &Page) -> bool {
        if page.kept {
            return false;
        }
        match self.live_finds(page.number) {
            Some(finds) => finds.sets == Find::NotRun && finds.flags == Find::NotRun,
            None => !page.overview.read && !self.finding.contains_key(&page.number),
        }
    }

    /// The one card that starts a review on a commit nothing has read: Review this pull request.
    fn start_card(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let title = "Start the review";
        let ink = look.color(token::ACCENT);
        let band = labelled(
            div()
                .id("overview-start-head")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .bg(ink.opacity(0.14))
                .text_color(ink)
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(glyph("scan-search", look.length(token::SPACE_4), ink))
                .child(title),
            Role::Heading,
            title,
        );
        let review = self.critique_button("overview-critique", "Review this pull request", Weight::Primary, page, look, cx);
        labelled(
            frame(look)
                .id("overview-start")
                .flex()
                .flex_col()
                .child(band)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_3))
                        .p(look.length(token::SPACE_3))
                        .child(words("overview-start-said", "Armada splits this pull request into change sets, then looks for blockers, risks and possible bugs in each.".into(), look))
                        .child(div().flex().child(review)),
                ),
            Role::Region,
            title,
        )
        .into_any_element()
    }

    /// No problems found, after a review that finished with nothing found and nothing failed: Approve
    /// sends an approval with no body at once; Approve with comments opens Review changes. Once he
    /// approved this commit, the card says so in place of both.
    fn clean_card(&self, id: &'static str, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let title = "No problems found";
        let hue = token::STATUS_COMPLETED_SUCCESS;
        let head = labelled(
            div()
                .id(SharedString::from(format!("{id}-title")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(glyph("circle-check", look.length(token::SPACE_4), look.color(hue)).flex_none())
                .child(title),
            Role::Heading,
            title,
        );
        let approved = page.mine.verdict == super::model::Verdict::Approved && page.mine.on.as_deref() == Some(page.head.as_str());
        let below = if approved {
            words(format!("{id}-approved"), "You approved this commit".into(), look)
        } else {
            let (refused, writing) = self.live_write(page.number);
            let refused = refused.map(|why| format!("Not sent: {why}"));
            let approve = labelled(
                Btn::new(SharedString::from(format!("{id}-approve")), "Approve", Weight::Primary)
                    .icon("check")
                    .disabled(writing)
                    .draw(look, cx)
                    .on_click(cx.listener(|this, _, window, cx| this.approve_bare(window, cx))),
                Role::Button,
                "Approve",
            );
            let comments = labelled(
                Btn::new(SharedString::from(format!("{id}-comments")), "Approve with comments", Weight::Secondary)
                    .icon("scroll-text")
                    .disabled(writing)
                    .draw(look, cx)
                    .on_click(cx.listener(|this, _, window, cx| this.approve_with_comments(window, cx))),
                Role::Button,
                "Approve with comments",
            );
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().flex().items_center().flex_wrap().gap(look.length(token::SPACE_2)).child(approve).child(comments))
                .children(refused.map(|said| {
                    labelled(
                        div()
                            .id(SharedString::from(format!("{id}-refused")))
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::RUN_FAILED))
                            .child(SharedString::from(said.clone())),
                        Role::Status,
                        said,
                    )
                }))
                .into_any_element()
        };
        let body = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(head).child(below);
        edged(id.into(), hue, body, title.into(), look)
    }

    /// The description's heading, with Summarize, Explain and Simplify at its right while it is hovered
    /// or one of their cards is open.
    fn why_head(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let shown = self.rewrites.get(&page.number).copied();
        let mut acts = div().flex_none().flex().items_center().gap(look.length(token::SPACE_1));
        for rewrite in Rewrite::ALL {
            let on = shown == Some(rewrite);
            let button = icon_button(SharedString::from(format!("overview-why-{}", rewrite.name())), rewrite.glyph(), look.color(if on { token::ACCENT } else { token::FG_MUTED }), look)
                .size(look.length(token::H_CONTROL))
                .tooltip(tip(rewrite.name()))
                .on_click(cx.listener(move |this, _, window, cx| this.ask_rewrite(rewrite, window, cx)));
            acts = acts.child(labelled(button, Role::Button, rewrite.name()));
        }
        let acts = if shown.is_some() { acts } else { acts.opacity(0.).group_hover("overview-why-group", |style| style.opacity(1.)) };
        div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().child(heading("overview-why-head", format!("Why, from {}", named(&page.author)), look))).child(acts).into_any_element()
    }

    /// The card above the description: what Summarize, Explain or Simplify wrote, or that it is writing.
    fn rewrite_card(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let rewrite = *self.rewrites.get(&page.number)?;
        let label = rewrite.label();
        let close = labelled(
            icon_button("overview-rewrite-close", "x", look.color(token::FG_MUTED), look).size(look.length(token::H_CONTROL)).tooltip(tip("Close")).on_click(cx.listener(|this, _, _, cx| this.close_rewrite(cx))),
            Role::Button,
            format!("Close {label}"),
        );
        let head = div()
            .flex()
            .items_center()
            .child(div().flex_1().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::ACCENT)).child(label))
            .child(close);
        let answer = match self.rewritten.get(&(page.number, rewrite)) {
            Some(Answer::Written(said)) => super::markdown::body("overview-rewrite-text", said, look),
            Some(Answer::Failed(why)) => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(status("overview-rewrite-failed", format!("{} failed: {why}", rewrite.name()), glyph("circle-x", look.length(token::SPACE_4), look.color(token::RUN_FAILED)).into_any_element(), look))
                .child(div().flex().child(labelled(
                    Btn::new("overview-rewrite-again", "Try again", Weight::Secondary).icon(rewrite.glyph()).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.ask_rewrite(rewrite, window, cx))),
                    Role::Button,
                    "Try again",
                )))
                .into_any_element(),
            Some(Answer::Writing) | None => status("overview-rewrite-writing", format!("Writing the {}", label.to_lowercase()), spinner("overview-rewrite-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look),
        };
        let body = div().flex().flex_col().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_SM)).child(head).child(answer);
        Some(edged("overview-rewrite".into(), token::ACCENT, body, label.into(), look))
    }

    /// Overview: why the author made it, the blockers, risks and possible bugs the review wrote, and
    /// the change sets, each with Review. Before a review has read the commit, one card starts it.
    fn overview_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let why = div()
            .group("overview-why-group")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(self.why_head(page, look, cx))
            .children(self.rewrite_card(page, look, cx))
            // The author writes markdown: headings, tables and links are drawn, not shown as their marks.
            .child(labelled(
                div().id("overview-why").text_size(look.length(token::TEXT_SM)).child(super::markdown::body("overview-why-text", &page.overview.why, look)),
                Role::Article,
                page.overview.why.clone(),
            ));
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(why);
        if self.not_started(page) {
            return column.child(div().pt(look.length(token::SPACE_2)).child(self.start_card(page, look, cx))).into_any_element();
        }
        column = column.child(div().pt(look.length(token::SPACE_2)).child(heading("overview-flags-head", "Blockers, risks and possible bugs".into(), look)));
        if page.kept {
            column = column.child(
                frame(look).p(look.length(token::SPACE_3)).child(grey("overview-flags-wait", "Finding problems in the commits since you looked", &[0.5, 0.9, 0.7], token::BG_SUNKEN, look)),
            );
        } else if let Some(state) = self.find_problems_state(page, look, cx) {
            column = column.child(state);
        } else {
            let open: Vec<&Flag> = page.overview.flags.iter().filter(|flag| !self.dismissed.contains(&(page.number, flag.id.clone()))).collect();
            // A run that failed never says nothing was found: Review comments says what failed.
            let clean = self.live_finds(page.number).is_none_or(|finds| super::live::map::clean_overview(finds, page.overview.flags.len()));
            if page.overview.flags.is_empty() && clean {
                column = column.child(self.clean_card("overview-clean", page, look, cx));
            } else if page.overview.flags.is_empty() {
                column = column.child(words("overview-flags-none", "No blockers, risks or possible bugs".into(), look));
            } else if open.is_empty() {
                column = column.child(words("overview-flags-done", "None left".into(), look));
            }
            for flag in open {
                column = column.child(self.flag_card(page, flag, look, cx));
            }
        }
        column = column.child(div().pt(look.length(token::SPACE_2)).child(heading("overview-sets-head", "Change sets".into(), look)));
        let mut table = frame(look).flex().flex_col();
        for set in &page.overview.sets {
            let mark = self.set_mark_of(page, &set.id);
            table = table.child(self.set_row(set, &mark, look, cx));
        }
        let sets = self.live_finds(page.number).map(|finds| finds.sets.clone());
        if page.kept {
            table = table.child(waiting("overview-sets-wait", "Writing the change sets of the commits since you looked", &[0.4, 0.8], look));
        } else if sets == Some(Find::Running) || self.finding.contains_key(&page.number) {
            table = table.child(waiting("overview-sets-wait", "Writing the change sets", &[0.4, 0.8], look));
        } else if sets.as_ref().is_some_and(|sets| *sets != Find::Done) {
            table = table.child(div().p(look.length(token::SPACE_3)).child(words("overview-sets-not-run", "Review this pull request writes the change sets".into(), look)));
        } else if page.overview.sets.is_empty() {
            table = table.child(div().p(look.length(token::SPACE_3)).child(words("overview-sets-none", "No change sets".into(), look)));
        }
        column.child(labelled(table.id("overview-sets"), Role::Table, "Change sets")).into_any_element()
    }

    /// A finding's card: a band hued by how serious it is, with its change set, then what and where,
    /// then its acts.
    fn flag_card(&self, page: &Page, flag: &Flag, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let hue = flag_hue(flag.kind);
        let ink = look.color(hue);
        let set = page.set(&flag.set).map(|set| set.title.clone()).unwrap_or_default();
        let set_chip = labelled(
            state_chip(look, format!("flag-set-{}", flag.id), "layers", format!("Change set: {set}"), set.clone(), token::FG_MUTED)
                .h(look.length(token::H_BADGE))
                .text_size(look.length(token::TEXT_XS)),
            Role::Label,
            format!("Change set: {set}"),
        );
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .bg(ink.opacity(0.14))
            .text_color(ink)
            .text_size(look.length(token::TEXT_XS))
            .child(glyph(flag.kind.glyph(), look.length(token::SPACE_4), ink))
            .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(flag.kind.said()))
            .child(div().flex_1())
            .child(set_chip);
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(flag.title.clone())))
            .child(div().child(SharedString::from(flag.said.clone())))
            .child(place_line(format!("flag-place-{}", flag.id), format!("{}:{}", flag.path, flag.line), look));
        let (add_id, show_id, drop_id) = (flag.id.clone(), flag.id.clone(), flag.id.clone());
        let add = match self.flag_state(page, &flag.id) {
            Some(said) => done_chip(format!("flag-done-{}", flag.id), said, look),
            None => labelled(
                Btn::new(SharedString::from(format!("flag-add-{}", flag.id)), "Add as draft comment", Weight::Primary)
                    .draw(look, cx)
                    .on_click(cx.listener(move |this, _, window, cx| this.add_draft(&add_id, window, cx))),
                Role::Button,
                "Add as draft comment",
            )
            .into_any_element(),
        };
        let show = labelled(
            Btn::new(SharedString::from(format!("flag-show-{}", flag.id)), "Show in change set", Weight::Secondary)
                .draw(look, cx)
                .on_click(cx.listener(move |this, _, window, cx| this.show_flag(&show_id, window, cx))),
            Role::Button,
            "Show in change set",
        );
        let drop = labelled(
            Btn::new(SharedString::from(format!("flag-drop-{}", flag.id)), "Not a problem", Weight::Ghost)
                .draw(look, cx)
                .on_click(cx.listener(move |this, _, window, cx| this.dismiss_suggestion(&drop_id, window, cx))),
            Role::Button,
            "Not a problem",
        );
        let foot = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_3))
            .child(add)
            .child(show)
            .child(drop);
        labelled(
            frame(look)
                .id(SharedString::from(format!("flag-{}", flag.id)))
                .flex()
                .child(div().flex_none().w(look.length(token::BORDER_WIDTH) * 3.).bg(ink))
                .child(div().flex_1().min_w_0().flex().flex_col().child(band).child(body).child(foot)),
            Role::Article,
            format!("{}: {}", flag.kind.said(), flag.title),
        )
        .into_any_element()
    }

    /// Where a finding went: a draft comment, or out with a review. None while it waits on him.
    fn flag_state(&self, page: &Page, flag: &str) -> Option<String> {
        if let Some(day) = self.sent_findings.get(&(page.number, flag.to_string())) {
            return Some(format!("Sent with your review {}", on_day(day)));
        }
        self.drafts.iter().any(|draft| draft.number == page.number && draft.finding.as_deref() == Some(flag)).then(|| "Added as draft comment".to_string())
    }

    /// One practice: its result's glyph, its words with what he said under them, and Wrong in a column
    /// of its own, held empty where a row has none.
    fn practice_row(&self, n: usize, practice: &Practice, kept: Kept, reason: Option<&str>, wrong: Option<Option<AnyElement>>, look: &Look) -> AnyElement {
        let (name, ink, said) = kept_mark(kept);
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(SharedString::from(practice.said.clone()))
            .children(reason.map(|reason| {
                let you_said = format!("You said: {reason}");
                labelled(
                    div()
                        .id(SharedString::from(format!("practice-said-{n}")))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(SharedString::from(you_said.clone())),
                    Role::Label,
                    you_said,
                )
            }));
        let column = wrong.map(|wrong| div().flex_none().w(look.length(token::LEADING_SM)).h(look.length(token::LEADING_SM)).children(wrong));
        labelled(
            div()
                .id(SharedString::from(format!("practice-{n}")))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(mark(format!("practice-mark-{n}"), name, ink, said, look)))
                .child(words)
                .children(column),
            Role::Row,
            format!("{said}: {}", practice.said),
        )
        .into_any_element()
    }

    /// Wrong: a ghost glyph that opens Doesn't apply here and Check is wrong, or on a result he
    /// corrected, Change the reason and Undo.
    fn wrong_button(&self, at: PracticeAt, practice: &Practice, corrected: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let name = format!("Wrong: {}", practice.said);
        let n = at.2;
        labelled(
            icon_button(SharedString::from(format!("practice-wrong-{n}")), "flag", look.color(token::FG_MUTED), look)
                .size(look.length(token::LEADING_SM))
                .rounded(look.length(token::RADIUS_SM))
                .tooltip(tip(if corrected { "Change or undo what you said" } else { "Say this result is wrong" }))
                .on_click(cx.listener({
                    let name = name.clone();
                    move |this, event: &gpui::ClickEvent, window, cx| {
                        let point = super::under_label(Role::Button, &name, event.position(), window, cx);
                        this.open_wrong(at.clone(), point, window, cx);
                    }
                })),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    /// The reason, in one line, under the practice he called wrong: Save keeps it, Cancel drops it.
    fn reason_field(&self, open: &super::Correcting, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let asks = open.wrong.asks();
        let save = labelled(
            Btn::new("practice-reason-save", "Save", Weight::Primary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.save_correction(window, cx))),
            Role::Button,
            "Save reason",
        );
        let cancel = labelled(
            Btn::new("practice-reason-cancel", "Cancel", Weight::Ghost).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.cancel_correcting(window, cx))),
            Role::Button,
            "Cancel reason",
        );
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            // Under the practice's words, past its glyph.
            .pl(look.length(token::SPACE_4) + look.length(token::SPACE_2))
            .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(open.wrong.said()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(div().id("practice-reason").flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).child(Input::new(&open.field)), Role::TextInput, asks))
                    .child(save)
                    .child(cancel),
            )
            .into_any_element()
    }

    /// What the findings on a set come to, leaving out those he called Not a problem.
    pub(super) fn set_mark_of(&self, page: &Page, set: &str) -> Mark {
        let open: Vec<&Flag> = page.overview.flags.iter().filter(|flag| !self.dismissed.contains(&(page.number, flag.id.clone()))).collect();
        Mark::of(set, &open)
    }

    /// A change set's row on the Overview: its glyph, title, what was found in it, purpose, and Review.
    fn set_row(&self, set: &ChangeSet, mark: &Mark, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let id = set.id.clone();
        let review = labelled(
            Btn::new(SharedString::from(format!("set-review-{}", set.id)), "Review", Weight::Secondary)
                .draw(look, cx)
                .on_click(cx.listener(move |this, _, _, cx| this.choose_set(&id, cx))),
            Role::Button,
            format!("Review {}", set.title),
        );
        labelled(
            div()
                .id(SharedString::from(format!("set-row-{}", set.id)))
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                // The glyph sits on the title's line; Review stays centred on the row.
                .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(set_mark(format!("set-mark-{}", set.id), mark, look.length(token::SPACE_4), look)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_x(look.length(token::SPACE_2))
                                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(set.title.clone())))
                                .when_some(mark.worst, |line, worst| {
                                    line.child(labelled(
                                        div().id(SharedString::from(format!("set-found-{}", set.id))).text_color(look.color(flag_hue(worst))).child(SharedString::from(mark.said.clone())),
                                        Role::Label,
                                        mark.said.clone(),
                                    ))
                                }),
                        )
                        .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(set.line()))),
                )
                .child(div().flex_none().self_center().child(review)),
            Role::Row,
            set.title.clone(),
        )
        .into_any_element()
    }

    /// A change set in focus: its purpose, the repository's practices for its kind, its hunks in the
    /// order the change runs with each finding beside its hunk, then How it works, How to test and Next.
    fn change_set_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let chosen = self.chosen.get(&page.number).cloned();
        let at = chosen.as_deref().and_then(|id| page.overview.sets.iter().position(|set| set.id == id)).unwrap_or(0);
        let scroller = div()
            .id("change-set")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.set_scroll)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_4));
        let Some(set) = page.overview.sets.get(at).cloned() else {
            return scroller.child(words("change-set-none", "No change sets".into(), look)).into_any_element();
        };
        let short = page.repository.rsplit('/').next().unwrap_or(&page.repository).to_string();
        let note = labelled(
            div()
                .id("change-set-kept")
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().h(look.length(token::LEADING_XS)).flex().items_center().child(glyph("hard-drive", look.length(token::SPACE_3), look.color(token::FG_MUTED))))
                .child(div().flex_1().min_w_0().child(SharedString::from(set.kept_note.clone()))),
            Role::Label,
            set.kept_note.clone(),
        );
        let live = self.is_live();
        let mut practices = div().flex().flex_col().gap(look.length(token::SPACE_1));
        let mut aside = Vec::new();
        for (n, practice) in set.practices.iter().enumerate() {
            // Fleet keeps no correction of a practice's result, so the live set offers no Wrong.
            if live {
                practices = practices.child(self.practice_row(n, practice, practice.kept, None, None, look));
                continue;
            }
            let at: PracticeAt = (page.number, set.id.clone(), n);
            let fixed = self.corrections.get(&at);
            let reason_field = self.correcting.as_ref().filter(|open| open.at == at).map(|open| self.reason_field(open, look, cx));
            if let Some(fixed) = fixed.filter(|fixed| fixed.wrong == Wrong::DoesNotApply) {
                let wrong = self.wrong_button(at.clone(), practice, true, look, cx);
                aside.push(self.practice_row(n, practice, Kept::DoesNotApply, Some(&fixed.reason), Some(Some(wrong)), look));
                aside.extend(reason_field);
                continue;
            }
            let kept = if fixed.is_some() { practice.kept.flipped() } else { practice.kept };
            // A result that does not apply has no reverse to call it; a corrected one can be undone.
            let wrong = (fixed.is_some() || practice.kept != Kept::DoesNotApply).then(|| self.wrong_button(at.clone(), practice, fixed.is_some(), look, cx));
            practices = practices.child(self.practice_row(n, practice, kept, fixed.map(|fixed| fixed.reason.as_str()), Some(wrong), look));
            practices = practices.children(reason_field);
        }
        if !aside.is_empty() {
            let open = self.aside_open.contains(&(page.number, set.id.clone()));
            let said = format!("{} set aside", aside.len());
            let (number, id) = (page.number, set.id.clone());
            let fold = labelled(
                div()
                    .id("practices-aside")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::LEADING_SM))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .aria_expanded(open)
                    .on_click(cx.listener(move |this, _, _, cx| this.fold_aside(number, id.clone(), cx)))
                    .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .child(SharedString::from(said.clone())),
                Role::Button,
                said,
            );
            practices = practices.child(fold);
            if open {
                practices = practices.children(aside);
            }
        }
        let lead: Vec<AnyElement> = vec![
            labelled(
                div()
                    .id("change-set-title")
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(SharedString::from(set.title.clone())),
                Role::Heading,
                set.title.clone(),
            )
            .into_any_element(),
            words("change-set-purpose", set.purpose.clone(), look),
            div().pt(look.length(token::SPACE_2)).child(heading("change-set-practices", format!("Practices for {} in {short}", set.kind), look)).into_any_element(),
            // Until the practices are checked on the live set, its note and list are where they stand.
            match self.practices_state(page, &set.id, look, cx) {
                Some(state) => state,
                None => note.into_any_element(),
            },
            labelled(practices.id("change-set-practice-list"), Role::Table, "Practices").into_any_element(),
            div().pt(look.length(token::SPACE_2)).child(heading("change-set-change", "The change, in the order it runs".into(), look)).into_any_element(),
        ];
        debug_assert_eq!(lead.len(), HUNKS_AT);
        let mut column = scroller.children(lead);
        for (n, hunk) in set.hunks.iter().enumerate() {
            let drawn = diff::file_diff_of(&hunk.path, Some(&hunk.patch), None);
            let lines = drawn.files.first().map(|one| one.lines.clone()).unwrap_or_default();
            let slots = diff::Slots { picked: hunk.hot, hues: self.hues_of(&hunk.path, &lines), ..Default::default() };
            let code = labelled(
                frame(look)
                    .id(SharedString::from(format!("hunk-{n}")))
                    .flex_1()
                    .min_w(look.length(token::SPACE_12) * 5.)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .h(look.length(token::H_CONTROL))
                            .px(look.length(token::SPACE_3))
                            .bg(look.color(token::BG_SUNKEN))
                            .border_b(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_XS))
                            .child(div().min_w_0().truncate().child(SharedString::from(hunk.path.clone()))),
                    )
                    .when(!hunk.caption.is_empty(), |code| {
                        code.child(labelled(
                            div()
                                .id(SharedString::from(format!("hunk-caption-{n}")))
                                .px(look.length(token::SPACE_3))
                                .py(look.length(token::SPACE_1))
                                .border_b(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_SUBTLE))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .child(SharedString::from(hunk.caption.clone())),
                            Role::Label,
                            hunk.caption.clone(),
                        ))
                    })
                    .child(diff::lines_with(SharedString::from(format!("hunk-diff-{}-{n}", set.id)), &lines, look, slots)),
                Role::Article,
                format!("File: {}", hunk.path),
            );
            let card = hunk.flag.as_deref().and_then(|flag| page.flag(flag)).filter(|flag| !self.dismissed.contains(&(page.number, flag.id.clone()))).map(|flag| self.fix_card(page, flag, look, cx));
            column = column.child(div().flex().flex_wrap().items_start().gap(look.length(token::SPACE_3)).child(code).children(card));
        }
        let explain = |explain: Explain, cx: &mut Context<Self>| {
            let key = (page.number, set.id.clone(), explain);
            let running = matches!(self.explains.get(&key), Some(None));
            let id = set.id.clone();
            labelled(
                Btn::new(SharedString::from(format!("explain-{}", explain.said())), explain.said(), Weight::Secondary)
                    .pending(running)
                    .draw(look, cx)
                    .on_click(cx.listener(move |this, _, window, cx| this.run_explain(id.clone(), explain, window, cx))),
                Role::Button,
                explain.said(),
            )
        };
        let next = page.overview.sets.get(at + 1).map(|next| {
            let id = next.id.clone();
            labelled(
                Btn::new("change-set-next", SharedString::from(next.title.clone()), Weight::Secondary).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| this.choose_set(&id, cx))),
                Role::Button,
                format!("Next: {}", next.title),
            )
        });
        let foot = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .pt(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            // Fleet cannot yet explain one change set, so the live set has no How it works or How to test.
            .when(!live, |foot| foot.child(explain(Explain::HowItWorks, cx)).child(explain(Explain::HowToTest, cx)))
            .child(div().flex_1())
            // Next: and its set wrap as one, never apart.
            .children(next.map(|next| {
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child("Next:"))
                    .child(next)
            }));
        column = column.child(foot);
        for explain in [Explain::HowItWorks, Explain::HowToTest] {
            let Some(Some(said)) = self.explains.get(&(page.number, set.id.clone(), explain)) else { continue };
            column = column.child(labelled(
                div()
                    .id(SharedString::from(format!("explain-said-{}", explain.said())))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(glyph("bot", look.length(token::SPACE_4), look.color(token::ACCENT))))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(explain.said()))
                            .child(SharedString::from(said.clone())),
                    ),
                Role::Article,
                format!("{}: {said}", explain.said()),
            ));
        }
        column.into_any_element()
    }

    /// The suggestion beside a hunk: what to do, in words he can edit, then Add as draft comment and Dismiss.
    fn fix_card(&self, page: &Page, flag: &Flag, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let name = format!("Suggestion: {}", flag.fix);
        let title = div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(flag.fix.clone()));
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_2)).child(title);
        match self.flag_state(page, &flag.id) {
            Some(said) => {
                let words_of = self.drafts.iter().find(|draft| draft.number == page.number && draft.finding.as_deref() == Some(&flag.id)).map_or_else(|| flag.comment.clone(), |draft| draft.body.clone());
                body = body
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(SharedString::from(words_of)))
                    .child(div().flex().child(done_chip(format!("fix-done-{}", flag.id), said, look)));
            }
            None => {
                let Some(boxed) = self.suggestions.get(&(page.number, flag.id.clone())) else { return div().into_any_element() };
                let (add_id, dismiss_id) = (flag.id.clone(), flag.id.clone());
                let add = labelled(
                    Btn::new(SharedString::from(format!("fix-add-{}", flag.id)), "Add as draft comment", Weight::Primary)
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.add_draft(&add_id, window, cx))),
                    Role::Button,
                    "Add as draft comment",
                );
                let dismiss = labelled(
                    Btn::new(SharedString::from(format!("fix-dismiss-{}", flag.id)), "Dismiss", Weight::Secondary)
                        .draw(look, cx)
                        .on_click(cx.listener(move |this, _, window, cx| this.dismiss_suggestion(&dismiss_id, window, cx))),
                    Role::Button,
                    "Dismiss",
                );
                body = body
                    .child(labelled(
                        div().id(SharedString::from(format!("fix-box-{}", flag.id))).key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&boxed.state)),
                        Role::TextInput,
                        name.clone(),
                    ))
                    .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).child(add).child(dismiss));
            }
        }
        div()
            .flex_1()
            .min_w(look.length(token::SPACE_12) * 5.)
            .child(edged(format!("fix-{}", flag.id), flag_hue(flag.kind), body, name, look))
            .into_any_element()
    }

    /// The summary of the new commits, written when he opens a pull request that changed.
    fn summary_part(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        if page.kept {
            return Some(
                div()
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::ACCENT_FAINT))
                    .child(grey("changed-summary-wait", "The summary waits for the commits since you looked", &[0.9, 0.6], token::BG_SUNKEN, look))
                    .into_any_element(),
            );
        }
        let (lead, said, role): (AnyElement, String, Role) = match &page.summary {
            Summary::None => return None,
            Summary::Writing => (spinner("summary-spin", look.length(token::SPACE_4), look.color(token::ACCENT), cx).into_any_element(), "Writing a summary of the new commits".into(), Role::Status),
            Summary::Written(said) => (glyph("bot", look.length(token::SPACE_4), look.color(token::ACCENT)).into_any_element(), said.clone(), Role::Label),
        };
        let writing = page.summary == Summary::Writing;
        Some(
            labelled(
                div()
                    .id("changed-summary")
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .when(writing, |box_| box_.text_color(look.color(token::FG_MUTED)))
                    .child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(lead))
                    .child(div().flex_1().min_w_0().child(SharedString::from(said.clone()))),
                role,
                said,
            )
            .into_any_element(),
        )
    }
}

/// A table's lines a kept page waits on the refresh for, in grey.
fn waiting(id: &str, name: &str, shares: &[f32], look: &Look) -> AnyElement {
    div()
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_3))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(grey(id, name, shares, token::BG_SUNKEN, look))
        .into_any_element()
}

/// A card with a hued left edge: amber for his drafts, blue for suggestions.
fn edged(id: String, hue: Token, body: Div, name: String, look: &Look) -> AnyElement {
    labelled(
        frame(look)
            .id(SharedString::from(id))
            .flex()
            .child(div().flex_none().w(look.length(token::BORDER_WIDTH) * 3.).bg(look.color(hue)))
            .child(body.flex_1().min_w_0().p(look.length(token::SPACE_3))),
        Role::Article,
        name,
    )
    .into_any_element()
}

fn conversation_tab(page: &Page, look: &Look) -> AnyElement {
    let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading("conversation-head", format!("#{} {}", page.number, page.title), look)).child(place_line(
        "conversation-branch".into(),
        format!("{} wants {} into {}", page.author, page.branch, page.base),
        look,
    ));
    column = column.child(div().text_size(look.length(token::TEXT_SM)).child(super::markdown::body("conversation-body", &page.body, look)));
    for entry in &page.conversation {
        let (id, author, at, body, verdict) = match entry {
            PullRequestEntry::Comment { id, author, at, body, .. } => (id, author, at, body, None),
            PullRequestEntry::Review { id, author, at, body, verdict, .. } => (id, author, at, body, Some(*verdict)),
        };
        let said = match verdict {
            Some(PullRequestVerdict::Approved) => format!("{author} approved"),
            Some(PullRequestVerdict::ChangesRequested) => format!("{author} asked for changes"),
            Some(_) => format!("{author} reviewed"),
            None => author.clone(),
        };
        column = column.child(labelled(
            frame(look)
                .id(SharedString::from(format!("entry-{id}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .child(div().flex().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(said.clone()))).child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(at.clone()))))
                .child(div().text_size(look.length(token::TEXT_SM)).child(super::markdown::body(format!("entry-body-{id}"), body, look))),
            Role::Article,
            format!("Comment by {said}"),
        ));
    }
    column.into_any_element()
}

fn checks_tab(page: &Page, look: &Look) -> AnyElement {
    let mut table = frame(look).flex().flex_col().child(band("checks-band".into(), "Checks", look));
    if page.kept {
        table = table.child(waiting("checks-wait", "Reading checks", &[0.4, 0.5, 0.3], look));
    }
    for (at, check) in page.checks.iter().enumerate() {
        let (_, _, said) = check_mark(check.state);
        let took = check.seconds.map_or_else(String::new, |seconds| format!("{}m {}s", seconds / 60, seconds % 60));
        let rest = vec![div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_1().min_w_0().child(SharedString::from(check.name.clone())))
            .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(took)))
            .into_any_element()];
        let end = check_words(check.state, said.to_string(), look);
        table = table.child(table_row(format!("check-{at}"), check.name.clone(), check_lead(format!("check-mark-{at}"), check.state, look), None, rest, Some(end), None, look));
    }
    labelled(table.id("checks-table"), Role::Table, "Checks").into_any_element()
}

fn comments_tab(page: &Page, look: &Look) -> AnyElement {
    let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3));
    if page.threads.is_empty() {
        column = column.child(words("comments-none", "No comments on lines".into(), look));
    }
    for thread in &page.threads {
        let place = thread.line.map_or_else(|| thread.path.clone(), |line| format!("{}:{line}", thread.path));
        let mut card = frame(look).id(SharedString::from(format!("thread-{}", thread.id))).flex().flex_col();
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_1))
                .bg(look.color(token::BG_SUNKEN))
                .child(div().flex_1().child(place_line(format!("thread-place-{}", thread.id), place.clone(), look)))
                .children(thread.resolved.then(|| mark(format!("thread-resolved-{}", thread.id), "circle-check", token::RUN_PASSED, "Resolved", look))),
        );
        for comment in &thread.comments {
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(comment.author.clone())))
                    .child(div().text_size(look.length(token::TEXT_SM)).child(super::markdown::body(format!("comment-body-{}", comment.id), &comment.body, look))),
            );
        }
        column = column.child(labelled(card, Role::Article, format!("Thread on {place}")));
    }
    column.into_any_element()
}

/// A finding's hue: red for a blocker, amber for a risk, blue for a possible bug.
fn flag_hue(kind: FlagKind) -> Token {
    match kind {
        FlagKind::Blocker => token::RUN_FAILED,
        FlagKind::Risk => token::STATUS_AWAITING_REVIEW,
        FlagKind::PossibleBug => token::ACCENT,
    }
}

/// A change set's worst finding in its card's glyph and hue, with the findings in its tooltip. A set
/// with none takes a quiet mark.
pub(super) fn set_mark(id: String, mark: &Mark, size: gpui::Pixels, look: &Look) -> AnyElement {
    let (name, ink, said) = match mark.worst {
        Some(kind) => (kind.glyph(), flag_hue(kind), mark.said.clone()),
        None => ("circle-check", token::FG_SUBTLE, NOTHING_FOUND.to_string()),
    };
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said.clone())).child(glyph(name, size, look.color(ink))), Role::Image, said).into_any_element()
}

const NOTHING_FOUND: &str = "No problems found";

fn kept_mark(kept: Kept) -> (&'static str, Token, &'static str) {
    match kept {
        Kept::Met => ("circle-check", token::RUN_PASSED, "Met"),
        Kept::Missed => ("circle-x", token::RUN_FAILED, "Missed"),
        Kept::DoesNotApply => ("circle", token::FG_SUBTLE, "Does not apply"),
    }
}

/// Where a finding went, in place of Add as draft comment: as tall as the buttons beside it.
fn done_chip(id: String, said: String, look: &Look) -> AnyElement {
    labelled(
        state_chip(look, id, "check", said.clone(), said.clone(), token::RUN_PASSED)
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .gap(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM)),
        Role::Status,
        said,
    )
    .into_any_element()
}
