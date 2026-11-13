//! What each tab of a pull request's page draws. Every tab reads the page as Fleet sends it.

use gpui::{div, prelude::*, AnyElement, Context, Div, ElementId, FontWeight, Role, SharedString, Toggled};
use gpui_component::text::TextView;
use ipc::{PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestVerdict};

use gpui_component::input::Textarea;

use super::model::{named, on_day, ChangeSet, Critique, Explain, Finding, Flag, FlagKind, Kept, Mark, Page, PageFile, Summary, Viewed};
use super::panes::{grey, place_line};
use super::{Ask, ReviewsScreen, HUNKS_AT};
use crate::diff;
use crate::job_view::pulls::state_chip;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{glyph, mono, spinner, tip, Btn, Weight};

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

/// One row of a table: a glyph column, a narrow fixed key column, and what it says.
fn table_row(id: String, name: String, lead: AnyElement, key: AnyElement, rest: Vec<AnyElement>, said: Option<String>, look: &Look) -> AnyElement {
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
            .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).overflow_hidden().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(key))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).children(rest)),
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

    fn critique_button(&self, id: &'static str, name: &'static str, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let running = matches!(page.critique, Critique::Running { .. });
        labelled(
            Btn::new(id, name, Weight::Primary).icon("bot").pending(running).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.ask(Ask::Critique, window, cx))),
            Role::Button,
            name,
        )
        .into_any_element()
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
                .child(div().flex().gap(look.length(token::SPACE_2)).child(self.critique_button("changed-critique-whole", "Critique the whole PR", page, look, cx)).child(whole))
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
            let key = div().into_any_element();
            table = table.child(table_row(format!("changed-commit-{}", commit.sha), commit.headline.clone(), lead, key, rest, Some(format!("Commit {}", commit.sha)), look));
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
            table = table.child(table_row(format!("changed-file-{at}"), file.path.clone(), mark(format!("changed-file-mark-{at}"), name, ink, said, look), key, rest, None, look));
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
                table = table.child(table_row(format!("changed-reply-{at}"), format!("Reply on {place}"), lead, key, rest, None, look));
            }
        }
        table = table.child(band("changed-band-checks".into(), "Checks", look));
        if page.kept {
            table = table.child(waiting("changed-checks-wait", "Reading checks", &[0.4, 0.5, 0.3], look));
        }
        for (at, check) in page.checks.iter().enumerate() {
            let (name, ink, said) = check_mark(check.state);
            let state = match (check.state, check.seconds) {
                (PullRequestCheckState::Running, Some(seconds)) if seconds >= 60 => format!("Running, {} min", seconds / 60),
                _ => said.to_string(),
            };
            let rest = vec![div().child(SharedString::from(check.name.clone())).into_any_element()];
            let key = div().text_color(look.color(ink)).child(SharedString::from(state.clone())).into_any_element();
            let row_name = format!("{}: {state}", check.name);
            table = table.child(table_row(format!("changed-check-{at}"), row_name, mark(format!("changed-check-mark-{at}"), name, ink, said, look), key, rest, None, look));
        }
        let table = labelled(table.id("changed-table"), Role::Table, "What changed");
        let diff = labelled(
            Btn::new("changed-diff", "Diff since you looked", Weight::Secondary).icon("git-compare").draw(look, cx).on_click(cx.listener(|this, _, _, cx| this.diff_since(cx))),
            Role::Button,
            "Diff since you looked",
        );
        column
            .child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).child(self.critique_button("changed-critique", "Critique these changes", page, look, cx)).child(diff))
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
                .on_click(cx.listener(move |this, _, _, cx| {
                    // The row under it opens and shuts the file; the box only marks it viewed.
                    cx.stop_propagation();
                    this.toggle_viewed(&path, cx);
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
            diff::lines(SharedString::from(format!("file-diff-{at}-{whole}")), &lines, look)
        });
        labelled(frame(look).id(SharedString::from(format!("file-{at}"))).flex().flex_col().child(head).children(body), Role::Article, format!("File: {}", file.path)).into_any_element()
    }

    /// Review comments: his draft comments, then the suggestions a critique or a tool made, each
    /// editable before it becomes a draft, then what already went, folded. Then the notes for the
    /// Job that holds the pull request.
    fn findings_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let drafts = self.drafts_part(page, look, cx);
        let found = self.critique_part(page, look, cx);
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
                    Btn::new("notes-send", "Send to job", Weight::Secondary).icon("send").draw(look, cx).on_click(cx.listener(|this, _, _, cx| this.send_notes(cx))),
                    Role::Button,
                    "Send to job",
                )
                .into_any_element()
            })
        });
        div().flex().flex_col().gap(look.length(token::SPACE_4)).child(drafts).child(found).children(notes).into_any_element()
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
        match &page.critique {
            Critique::None => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(status("findings-none", "No suggestions on this pull request".into(), glyph("bot", look.length(token::SPACE_4), muted).into_any_element(), look))
                .child(div().flex().child(self.critique_button("findings-critique", "Critique this pull request", page, look, cx)))
                .into_any_element(),
            Critique::Running { range } => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(status("findings-running", "Critique running".into(), spinner("findings-spin", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING), cx).into_any_element(), look))
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
                .into_any_element(),
            Critique::Finished { range, findings } => {
                let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading_tipped("findings-head", format!("Suggestions on {}", range.said), Some(range.commits.clone()), look));
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
                if findings.is_empty() {
                    column = column.child(status("findings-empty", "Critique found nothing".into(), glyph("circle-check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look));
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
        }
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

    /// Overview: why the author made it, the blockers, risks and possible bugs Find problems wrote, and
    /// the change sets, each with Review.
    fn overview_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let mut column = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(heading("overview-why-head", format!("Why, from {}", named(&page.author)), look))
            .child(words("overview-why", page.overview.why.clone(), look))
            .child(div().pt(look.length(token::SPACE_2)).child(heading("overview-flags-head", "Blockers, risks and possible bugs".into(), look)));
        if page.kept {
            column = column.child(
                frame(look).p(look.length(token::SPACE_3)).child(grey("overview-flags-wait", "Finding problems in the commits since you looked", &[0.5, 0.9, 0.7], token::BG_SUNKEN, look)),
            );
        } else {
            let open: Vec<&Flag> = page.overview.flags.iter().filter(|flag| !self.dismissed.contains(&(page.number, flag.id.clone()))).collect();
            if page.overview.flags.is_empty() {
                column = column.child(status("overview-flags-none", "Find problems found nothing".into(), glyph("circle-check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look));
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
            table = table.child(self.set_row(set, look, cx));
        }
        if page.kept {
            table = table.child(waiting("overview-sets-wait", "Writing the change sets of the commits since you looked", &[0.4, 0.8], look));
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
            .child(div().flex_1())
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

    /// A change set's row on the Overview: its glyph, title, purpose, and Review.
    fn set_row(&self, set: &ChangeSet, look: &Look, cx: &mut Context<Self>) -> AnyElement {
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
                .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(set_mark(format!("set-mark-{}", set.id), set.mark, look.length(token::SPACE_4), look)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(set.title.clone())))
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
        let mut practices = div().flex().flex_col().gap(look.length(token::SPACE_1));
        for (n, practice) in set.practices.iter().enumerate() {
            let (name, ink, said) = kept_mark(practice.kept);
            practices = practices.child(labelled(
                div()
                    .id(SharedString::from(format!("practice-{n}")))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(mark(format!("practice-mark-{n}"), name, ink, said, look)))
                    .child(div().flex_1().min_w_0().child(SharedString::from(practice.said.clone()))),
                Role::Row,
                format!("{said}: {}", practice.said),
            ));
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
            note.into_any_element(),
            labelled(practices.id("change-set-practice-list"), Role::Table, "Practices").into_any_element(),
            div().pt(look.length(token::SPACE_2)).child(heading("change-set-change", "The change, in the order it runs".into(), look)).into_any_element(),
        ];
        debug_assert_eq!(lead.len(), HUNKS_AT);
        let mut column = scroller.children(lead);
        for (n, hunk) in set.hunks.iter().enumerate() {
            let drawn = diff::file_diff_of(&hunk.path, Some(&hunk.patch), None);
            let lines = drawn.files.first().map(|one| one.lines.clone()).unwrap_or_default();
            let slots = diff::Slots { picked: hunk.hot, ..Default::default() };
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
            .child(explain(Explain::HowItWorks, cx))
            .child(explain(Explain::HowToTest, cx))
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
    column = column.child(div().text_size(look.length(token::TEXT_SM)).child(TextView::markdown(ElementId::Name("conversation-body".into()), page.body.clone())));
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
                .child(div().text_size(look.length(token::TEXT_SM)).child(TextView::markdown(ElementId::Name(format!("entry-body-{id}").into()), body.clone()))),
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
        let (name, ink, said) = check_mark(check.state);
        let took = check.seconds.map_or_else(String::new, |seconds| format!("{}m {}s", seconds / 60, seconds % 60));
        let rest = vec![div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_1().min_w_0().child(SharedString::from(check.name.clone())))
            .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(took)))
            .into_any_element()];
        let key = div().text_color(look.color(ink)).child(said).into_any_element();
        table = table.child(table_row(format!("check-{at}"), check.name.clone(), mark(format!("check-mark-{at}"), name, ink, said, look), key, rest, None, look));
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
                    .child(div().text_size(look.length(token::TEXT_SM)).child(SharedString::from(comment.body.clone()))),
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

/// A change set's worst finding as a registry glyph that says itself in a tooltip.
pub(super) fn set_mark(id: String, mark: Mark, size: gpui::Pixels, look: &Look) -> AnyElement {
    let (name, ink, said) = match mark {
        Mark::Blocked => ("circle-x", token::RUN_FAILED, "A blocker"),
        Mark::Risky => ("triangle-alert", token::STATUS_AWAITING_REVIEW, "A risk"),
        Mark::Clear => ("circle-check", token::RUN_PASSED, "Nothing blocks it"),
    };
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said)).child(glyph(name, size, look.color(ink))), Role::Image, said).into_any_element()
}

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
