//! What each tab of a pull request's page draws. Every tab reads the page as Fleet sends it.

use gpui::{div, prelude::*, relative, AnyElement, Context, Div, ElementId, FontWeight, Role, SharedString, Toggled, Window};
use gpui_component::input::Textarea;
use gpui_component::text::TextView;
use ipc::{PullRequestCheckState, PullRequestEntry, PullRequestFileStatus, PullRequestVerdict, PullRequestWriteKind};

use super::model::{Critique, Page, PageFile, Viewed};
use super::panes::place_line;
use super::{Ask, ReviewsScreen};
use crate::diff;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{glyph, icon_button, mono, spinner, tip, Btn, Weight};

/// The glyph on a tab and in the Add tab menu.
pub(super) fn glyph_of(kind: &str) -> &'static str {
    match kind {
        "pr-changed" => "git-compare",
        "pr-files" => "file-diff",
        "pr-conversation" => "message-square",
        "pr-checks" => "list-checks",
        "pr-findings" => "bot",
        "pr-comments" => "message-square-reply",
        "pr-review" => "stamp",
        _ => "file-text",
    }
}

fn check_mark(state: PullRequestCheckState) -> (&'static str, Token, &'static str) {
    match state {
        PullRequestCheckState::Passed => ("circle-check", token::RUN_PASSED, "Passed"),
        PullRequestCheckState::Failed => ("circle-x", token::RUN_FAILED, "Failed"),
        PullRequestCheckState::Running => ("loader-circle", token::RUN_RUNNING, "Running"),
        PullRequestCheckState::Skipped => ("circle-dashed", token::FG_SUBTLE, "Skipped"),
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

fn verdict_said(kind: PullRequestWriteKind) -> &'static str {
    match kind {
        PullRequestWriteKind::Comment => "Comment",
        PullRequestWriteKind::Approve => "Approve",
        PullRequestWriteKind::RequestChanges => "Request changes",
    }
}

/// A state as a registry glyph that says itself in a tooltip.
fn mark(id: String, name: &'static str, ink: Token, said: &'static str, look: &Look) -> AnyElement {
    labelled(div().id(SharedString::from(id)).flex_none().tooltip(tip(said)).child(glyph(name, look.length(token::SPACE_4), look.color(ink))), Role::Image, said).into_any_element()
}

fn heading(id: impl Into<SharedString>, words: String, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(ElementId::Name(id.into()))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
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

/// One row of a table: a glyph column, a fixed key column, and what it says.
fn table_row(id: String, name: String, lead: AnyElement, key: String, rest: Vec<AnyElement>, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(id))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_SM))
            .child(div().flex_none().w(look.length(token::SPACE_4)).child(lead))
            .child(div().flex_none().w(relative(0.36)).font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(key)))
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
    pub(super) fn tab_body(&mut self, kind: &str, page: &Page, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let inner = match kind {
            "pr-changed" => self.changed_tab(page, look, cx),
            "pr-files" => self.files_tab(page, look, cx),
            "pr-conversation" => conversation_tab(page, look),
            "pr-checks" => checks_tab(page, look),
            "pr-findings" => self.findings_tab(page, look, cx),
            "pr-comments" => comments_tab(page, look),
            "pr-review" => self.review_tab(page, look, window, cx),
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
            column = column.child(heading("changed-head", format!("Since you looked, {}", seen.when), look));
            let said = format!("{} is gone after a force push, so there is no diff since you looked. The whole pull request is at {}.", seen.old, seen.new);
            let notice = labelled(
                div()
                    .id("changed-gone")
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
        column = column.child(heading("changed-head", format!("Since you looked, {} · {} → {}", seen.when, seen.old, seen.new), look));
        let mut table = frame(look).flex().flex_col();
        table = table.child(band("changed-band-commits".into(), "New commits", look));
        for commit in &page.new_commits {
            let by = commit.author.clone().unwrap_or_default();
            let rest = vec![
                div().child(SharedString::from(commit.headline.clone())).into_any_element(),
                div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(by)).into_any_element(),
            ];
            let lead = glyph("git-commit-horizontal", look.length(token::SPACE_4), look.color(token::STATUS_HANDED_IN)).into_any_element();
            table = table.child(table_row(format!("changed-commit-{}", commit.sha), format!("{} {}", commit.sha, commit.headline), lead, commit.sha.clone(), rest, look));
        }
        table = table.child(band("changed-band-files".into(), "Files changed", look));
        for (at, file) in page.files.iter().filter(|file| file.since).enumerate() {
            let (name, ink, said) = file_mark(file.status);
            let mut rest = vec![div()
                .flex()
                .gap(look.length(token::SPACE_2))
                .items_center()
                .child(div().text_color(look.color(token::RUN_PASSED)).child(SharedString::from(format!("+{}", file.additions))))
                .child(div().text_color(look.color(token::RUN_FAILED)).child(SharedString::from(format!("−{}", file.deletions))))
                .into_any_element()];
            if file.viewed == Viewed::ChangedAfter {
                rest.push(chip(format!("changed-after-{at}"), "You had viewed it".into(), token::STATUS_HANDED_IN, token::STATUS_HANDED_IN_BG, look));
            }
            table = table.child(table_row(format!("changed-file-{at}"), file.path.clone(), mark(format!("changed-file-mark-{at}"), name, ink, said, look), file.path.clone(), rest, look));
        }
        if !page.replies.is_empty() {
            table = table.child(band("changed-band-replies".into(), "Replies to your comments", look));
            for (at, reply) in page.replies.iter().enumerate() {
                let place = format!("{}:{}", reply.path, reply.line);
                let rest = vec![
                    div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(SharedString::from(reply.author.clone())).into_any_element(),
                    div().child(SharedString::from(reply.body.clone())).into_any_element(),
                ];
                let lead = glyph("message-square-reply", look.length(token::SPACE_4), look.color(token::ACCENT)).into_any_element();
                table = table.child(table_row(format!("changed-reply-{at}"), format!("Reply on {place}"), lead, place, rest, look));
            }
        }
        table = table.child(band("changed-band-checks".into(), "Checks", look));
        for (at, check) in page.checks.iter().enumerate() {
            let (name, ink, said) = check_mark(check.state);
            let rest = vec![div().text_color(look.color(token::FG_MUTED)).child(said).into_any_element()];
            table = table.child(table_row(format!("changed-check-{at}"), check.name.clone(), mark(format!("changed-check-mark-{at}"), name, ink, said, look), check.name.clone(), rest, look));
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
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_viewed(&path, cx)))
                .child(glyph(if viewed { "check" } else { "square" }, look.length(token::SPACE_4), tick_ink))
                .child("Viewed"),
            Role::CheckBox,
            format!("Viewed: {}", file.path),
        );
        let (name, ink, said) = file_mark(file.status);
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG))
            .bg(look.color(token::BG_SUNKEN))
            .when(!viewed, |head| head.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
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
        let body = (!viewed).then(|| {
            let patch = if whole { file.patch.as_str() } else { file.patch_since.as_deref().unwrap_or(&file.patch) };
            let drawn = diff::file_diff_of(&file.path, Some(patch), None);
            let lines = drawn.files.first().map(|one| one.lines.clone()).unwrap_or_default();
            diff::lines(SharedString::from(format!("file-diff-{at}-{whole}")), &lines, look)
        });
        labelled(frame(look).id(SharedString::from(format!("file-{at}"))).flex().flex_col().child(head).children(body), Role::Article, format!("File: {}", file.path)).into_any_element()
    }

    /// Findings: what a critique of the range found, each one a draft away.
    fn findings_tab(&mut self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
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
                .child(place_line("findings-range".into(), range.clone(), look))
                .into_any_element(),
            Critique::Finished { range, findings } => {
                let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(heading("findings-head", format!("Critique of {range}"), look));
                if findings.is_empty() {
                    column = column.child(status("findings-empty", "Critique found nothing".into(), glyph("circle-check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look));
                }
                for finding in findings {
                    let drafted = self.drafts.iter().any(|draft| draft.number == page.number && draft.finding.as_deref() == Some(&finding.id));
                    let id = finding.id.clone();
                    let act: AnyElement = if drafted {
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
                            .gap(look.length(token::SPACE_3))
                            .p(look.length(token::SPACE_3))
                            .border_l(look.length(token::BORDER_WIDTH) * 3.)
                            .border_color(look.color(token::STATUS_AWAITING_REVIEW))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(look.length(token::SPACE_1))
                                    .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(finding.title.clone())))
                                    .child(place_line(format!("finding-place-{}", finding.id), format!("{}:{}", finding.path, finding.line), look))
                                    .child(words(format!("finding-said-{}", finding.id), finding.comment.clone(), look)),
                            )
                            .child(div().flex_none().w(look.length(token::SPACE_12) * 2.5).flex().justify_end().child(act)),
                        Role::Article,
                        format!("Finding: {}", finding.title),
                    );
                    column = column.child(card);
                }
                column.into_any_element()
            }
        }
    }

    /// Review: the drafts, the body and the verdict, sent together; and the Job's notes.
    fn review_tab(&mut self, page: &Page, look: &Look, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_3));
        if let Some(kind) = self.submitted.get(&page.number) {
            column = column.child(status("review-sent", format!("Submitted as {}", verdict_said(*kind)), glyph("circle-check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look));
        }
        column = column.child(heading("review-drafts", "Drafts".into(), look));
        let mine: Vec<_> = self.drafts.iter().filter(|draft| draft.number == page.number).cloned().collect();
        if mine.is_empty() {
            column = column.child(words("review-no-drafts", "No drafts on this pull request".into(), look));
        }
        for draft in mine {
            let Some(boxed) = self.boxes.get(&draft.id) else { continue };
            let place = draft.place();
            let id = draft.id.clone();
            let discard = labelled(
                icon_button(SharedString::from(format!("draft-discard-{}", draft.id)), "x", look.color(token::FG_MUTED), look)
                    .tooltip(tip("Discard draft"))
                    .on_click(cx.listener(move |this, _, _, cx| this.discard_draft(&id, cx))),
                Role::Button,
                format!("Discard draft on {place}"),
            );
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().flex().items_center().child(div().flex_1().child(place_line(format!("draft-place-{}", draft.id), place.clone(), look))).child(discard))
                    .child(labelled(
                        div().id(SharedString::from(format!("draft-box-{}", draft.id))).key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&boxed.state)),
                        Role::TextInput,
                        format!("Draft on {place}"),
                    )),
            );
        }
        column = column.child(heading("review-body-head", "Review".into(), look));
        if let Some(body) = &self.body {
            column = column.child(labelled(
                div().id("review-body").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&body.state)),
                Role::TextInput,
                "Review body",
            ));
        }
        let verdicts = [PullRequestWriteKind::Comment, PullRequestWriteKind::Approve, PullRequestWriteKind::RequestChanges].map(|kind| {
            let on = self.verdict == kind;
            let said = verdict_said(kind);
            labelled(
                div()
                    .id(SharedString::from(format!("verdict-{said}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
                    .when(on, |radio| radio.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.verdict = kind;
                        cx.notify();
                    }))
                    .child(said),
                Role::RadioButton,
                said,
            )
        });
        let submit = labelled(
            Btn::new("review-submit", "Submit", Weight::Primary).icon("send").draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.submit(window, cx))),
            Role::Button,
            "Submit",
        );
        column = column.child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(labelled(
                    div().id("review-verdict").flex().p(look.length(token::SPACE_1)).gap(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_SUNKEN)).children(verdicts),
                    Role::RadioGroup,
                    "Verdict",
                ))
                .child(div().flex_1())
                .child(submit),
        );
        if let Some(holder) = page.holder.clone() {
            column = column.child(heading("review-notes-head", format!("Notes for {holder}"), look));
            for (at, note) in page.notes.iter().enumerate() {
                column = column.child(words(format!("review-note-{at}"), note.clone(), look));
            }
            let sent = self.sent_notes.contains(&page.number);
            column = column.child(if sent {
                status("review-notes-sent", format!("Sent to {holder}"), glyph("check", look.length(token::SPACE_4), look.color(token::RUN_PASSED)).into_any_element(), look)
            } else {
                labelled(
                    Btn::new("review-send-notes", "Send to job", Weight::Secondary).icon("send").draw(look, cx).on_click(cx.listener(|this, _, _, cx| this.send_notes(cx))),
                    Role::Button,
                    "Send to job",
                )
                .into_any_element()
            });
        }
        column.into_any_element()
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
        let rest = vec![div().flex().justify_between().child(div().text_color(look.color(token::FG_MUTED)).child(said)).child(div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(took))).into_any_element()];
        table = table.child(table_row(format!("check-{at}"), check.name.clone(), mark(format!("check-mark-{at}"), name, ink, said, look), check.name.clone(), rest, look));
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
