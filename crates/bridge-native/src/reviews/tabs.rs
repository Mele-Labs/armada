//! What each tab of a pull request's page draws. Every tab reads the page as Fleet sends it.

use gpui::{div, prelude::*, AnyElement, Context, Div, ElementId, FontWeight, Role, SharedString, Toggled};
use gpui_component::text::TextView;
use ipc::{PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestVerdict};

use super::model::{Critique, Page, PageFile, Viewed};
use super::panes::place_line;
use super::{Ask, ReviewsScreen};
use crate::diff;
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
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_MUTED))
            .child(lead)
            .child(SharedString::from(said.clone())),
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
        let inner = match kind {
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
        let mut table = frame(look).flex().flex_col();
        table = table.child(band("changed-band-commits".into(), "New commits", look));
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
            .child(div().flex().gap(look.length(token::SPACE_2)).child(self.critique_button("changed-critique", "Critique these changes", page, look, cx)).child(diff))
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
            .h(look.length(token::H_CONTROL_LG))
            .bg(look.color(token::BG_SUNKEN))
            .cursor_pointer()
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_file(&path, viewed, cx)))
            .when(open, |head| head.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
            .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
            .child(tick)
            .child(mark(format!("file-mark-{at}"), name, ink, said, look))
            .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).child(SharedString::from(file.path.clone())))
            .children((file.viewed == Viewed::ChangedAfter).then(|| chip(format!("file-after-{at}"), "Changed after you viewed it".into(), token::STATUS_HANDED_IN, token::STATUS_HANDED_IN_BG, look)))
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

    /// Findings: what a critique of the range found, each one a draft away, and the notes for the Job
    /// that holds the pull request.
    fn findings_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
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
        div().flex().flex_col().gap(look.length(token::SPACE_4)).child(found).children(notes).into_any_element()
    }

    fn critique_part(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let muted = look.color(token::FG_MUTED);
        match &page.critique {
            Critique::None => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(status("findings-none", "No critique of this pull request".into(), glyph("bot", look.length(token::SPACE_4), muted).into_any_element(), look))
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
                let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading_tipped("findings-head", format!("Critique of {}", range.said), Some(range.commits.clone()), look));
                if findings.is_empty() {
                    column = column.child(status("findings-empty", "Critique found nothing".into(), glyph("circle-check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look));
                }
                for finding in findings {
                    let drafted = self.drafts.iter().any(|draft| draft.number == page.number && draft.finding.as_deref() == Some(&finding.id));
                    let id = finding.id.clone();
                    let sent = self.sent_findings.contains(&(page.number, finding.id.clone()));
                    let act: AnyElement = if sent {
                        status(format!("finding-sent-{}", finding.id), "Sent".into(), glyph("check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look)
                    } else if drafted {
                        status(format!("finding-drafted-{}", finding.id), "Drafted".into(), glyph("check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look)
                    } else {
                        labelled(
                            Btn::new(SharedString::from(format!("finding-add-{}", finding.id)), "Add as draft", Weight::Secondary)
                                .icon("plus")
                                .draw(look, cx)
                                .on_click(cx.listener(move |this, _, window, cx| this.add_draft(&id, window, cx))),
                            Role::Button,
                            "Add as draft",
                        )
                        .into_any_element()
                    };
                    let card = labelled(
                        frame(look)
                            .id(SharedString::from(format!("finding-{}", finding.id)))
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .p(look.length(token::SPACE_3))
                            .border_l(look.length(token::BORDER_WIDTH) * 3.)
                            .border_color(look.color(token::STATUS_AWAITING_REVIEW))
                            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(finding.title.clone())))
                            // One act per card, under its title, so a narrow pane keeps the text wide and a short one still shows it.
                            .child(div().flex().child(act))
                            .child(place_line(format!("finding-place-{}", finding.id), format!("{}:{}", finding.path, finding.line), look))
                            .child(words(format!("finding-said-{}", finding.id), finding.comment.clone(), look)),
                        Role::Article,
                        format!("Finding: {}", finding.title),
                    );
                    column = column.child(card);
                }
                column.into_any_element()
            }
        }
    }

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
