//! The pull request's head, with Your review pinned under its title: where his review stands, what
//! came after it, and Review changes, whose sheet takes the panes' place until Send or Cancel. It is
//! not a pane, so no layout moves it.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use gpui_component::input::Textarea;

use super::inbox::verdict_hue;
use super::model::{self, Page, PullState, Verdict};
use super::panes::{grey, place_line};
use super::{panel, ReviewsScreen};
use crate::job_view::pulls::state_chip;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{icon_button, mono, tip, Btn, Weight};

/// The head's state chip hue: the registry's open pull request, and grey once it is a draft or ended.
fn pull_hue(state: PullState) -> Token {
    match state {
        PullState::Open => token::STEP_RUNNING,
        PullState::Draft | PullState::Closed => token::STATUS_NOT_STARTED,
        PullState::Gone => token::FG_MUTED,
    }
}

impl ReviewsScreen {
    pub(super) fn render_head(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let title = format!("#{} {}", page.number, page.title);
        let heading = labelled(
            div()
                .id("page-title")
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .truncate()
                .child(SharedString::from(title.clone())),
            Role::Heading,
            title,
        );
        let chip = |chip: gpui::Stateful<gpui::Div>| chip.h(look.length(token::H_BADGE)).text_size(look.length(token::TEXT_XS));
        let author = labelled(chip(state_chip(look, "page-author".into(), "user", format!("Opened by {}", page.author), page.author.clone(), token::FG_MUTED)), Role::Label, format!("Author: {}", page.author));
        let branch = format!("{} → {}", page.branch, page.base);
        let branch = labelled(
            chip(state_chip(look, "page-branch".into(), "git-branch", format!("{} wants {} into {}", page.author, page.branch, page.base), branch.clone(), token::ACCENT)).font_family(mono(look)),
            Role::Label,
            format!("Branch: {branch}"),
        );
        let pull = PullState::of(page.draft, page.ended);
        let state = labelled(chip(state_chip(look, "page-state".into(), pull.glyph(), pull.said().into(), pull.said(), pull_hue(pull))), Role::Status, pull.said());
        let repository = labelled(
            div().id("page-repository").min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(page.repository.clone())),
            Role::Label,
            format!("Repository: {}", page.repository),
        );
        let meta = div().flex().items_center().gap(look.length(token::SPACE_2)).min_w_0().child(author).child(branch).child(state).child(repository);
        // The address goes to the clipboard or the browser; a page Fleet gave none has neither.
        let copied = self.copied.as_ref().is_some_and(|(number, _)| *number == page.number);
        let acts = (!page.address.is_empty()).then(|| {
            let copy = labelled(
                icon_button("page-copy-link", if copied { "check" } else { "link" }, look.color(if copied { token::RUN_PASSED } else { token::FG_MUTED }), look)
                    .tooltip(tip(if copied { "Link copied" } else { "Copy link" }))
                    .on_click(cx.listener(|this, _, window, cx| this.copy_link(window, cx))),
                Role::Button,
                "Copy link",
            );
            let browse = labelled(
                icon_button("page-open-browser", "external-link", look.color(token::FG_MUTED), look).tooltip(tip("Open in browser")).on_click(cx.listener(|this, _, _, cx| this.open_in_browser(cx))),
                Role::Button,
                "Open in browser",
            );
            let said = copied.then(|| labelled(div().id("page-copied").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Link copied"), Role::Status, "Link copied"));
            div().flex_none().flex().items_center().gap(look.length(token::SPACE_1)).children(said).child(copy).child(browse)
        });
        // Judge sits beside Copy link, where a Job holds the pull request.
        let beside = div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).children(self.judge_button(page, look, cx)).children(acts);
        let title_row = div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().child(heading)).child(beside);
        panel(look)
            .flex_none()
            .flex()
            .flex_col()
            .child(div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_2)).flex().flex_col().gap(look.length(token::SPACE_2)).child(title_row).child(meta))
            .child(self.render_bar(page, look, cx))
            .into_any_element()
    }

    /// Your review at rest: its state and day, what came since, the drafts waiting, and Review changes.
    fn render_bar(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let mine = &page.mine;
        let (hue, _) = verdict_hue(mine.verdict);
        let said = mine.chip();
        let chip = |chip: gpui::Stateful<gpui::Div>| chip.h(look.length(token::H_BADGE)).text_size(look.length(token::TEXT_XS));
        let state = labelled(chip(state_chip(look, "mine-state".into(), mine.verdict.glyph(), mine.tip(), said.clone(), hue)), Role::Status, said);
        // Who is still asked: his own chip where he is, then the others in a muted line.
        let asked = model::asked(&page.reviewers, &self.me);
        let asked_you = asked.you.then(|| {
            let said = "Review requested from you";
            labelled(chip(state_chip(look, "mine-asked".into(), "user-check", said.into(), said, token::ACCENT)), Role::Status, said)
        });
        let asked_others = asked.others.map(|others| {
            labelled(
                div().id("mine-asked-others").min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(others.clone())),
                Role::Label,
                others,
            )
        });
        // What came since waits in grey on a kept page; a page that ended says how, in its place.
        let since = match (page.ended, page.kept) {
            (Some(ended), _) => {
                let (icon, said) = ended.said();
                Some(labelled(chip(state_chip(look, "mine-ended".into(), icon, said.into(), said, token::FG_MUTED)), Role::Status, said).into_any_element())
            }
            (None, true) => Some(div().flex_none().w(look.length(token::SPACE_12) * 2.).child(grey("mine-since-wait", "Reading what came since", &[1.], token::BORDER_DEFAULT, look)).into_any_element()),
            (None, false) => mine.since().map(|since| {
                labelled(
                    div().id("mine-since").flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(since.clone())),
                    Role::Label,
                    since,
                )
                .into_any_element()
            }),
        };
        let drafts = self.drafts.iter().filter(|draft| draft.number == page.number).count();
        let drafted = (drafts > 0).then(|| {
            let said = if drafts == 1 { "1 draft comment".to_string() } else { format!("{drafts} draft comments") };
            labelled(chip(state_chip(look, "mine-drafts".into(), "pencil", "Draft comments go with your next review".into(), said.clone(), token::ACCENT)), Role::Label, said)
        });
        // Nothing to say on an approval that nothing has changed since: the button steps back.
        let weight = if mine.verdict == Verdict::Approved && !mine.fresh() { Weight::Secondary } else { Weight::Primary };
        let review = labelled(
            Btn::new("mine-review", "Review changes", weight)
                .icon("send")
                .draw(look, cx)
                .on_click(cx.listener(|this, _, window, cx| this.open_sheet(!this.sheet, window, cx))),
            Role::Button,
            "Review changes",
        );
        labelled(
            div()
                .id("mine")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .bg(look.color(token::BG_SUNKEN))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(state)
                .children(asked_you)
                .children(asked_others)
                .children(since)
                .children(drafted)
                .child(div().flex_1())
                .when(page.ended.is_none(), |bar| bar.child(review)),
            Role::Region,
            "Your review",
        )
        .into_any_element()
    }

    /// Review changes: the drafts that go, a summary, and the verdict that sends them.
    pub(super) fn render_sheet(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let title = "Review changes".to_string();
        let heading = labelled(
            div()
                .id("sheet-title")
                .tooltip(tip(format!("At commit {}", page.head)))
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(SharedString::from(title.clone())),
            Role::Heading,
            title.clone(),
        );
        let close = labelled(
            icon_button("sheet-close", "x", look.color(token::FG_MUTED), look).size(look.length(token::H_CONTROL)).tooltip(tip("Close")).on_click(cx.listener(|this, _, window, cx| this.open_sheet(false, window, cx))),
            Role::Button,
            "Close Review changes",
        );
        let heading = div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().flex_1().min_w_0().child(heading)).child(close);
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
        let mut drafts = div().flex().flex_col().gap(look.length(token::SPACE_3));
        let mine: Vec<_> = self.drafts.iter().filter(|draft| draft.number == page.number).collect();
        if mine.is_empty() {
            // Nothing drafted and no review run on it yet: the sheet offers the same picker as Review this pull request.
            let start = (!self.ran_review(page)).then(|| {
                labelled(
                    Btn::new("sheet-start-review", "Start review", Weight::Secondary).icon("bot").draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.open_review("Start review", window, cx))),
                    Role::Button,
                    "Start review",
                )
            });
            drafts = drafts.child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(labelled(
                        div().id("sheet-no-drafts").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child("No drafts"),
                        Role::Label,
                        "No drafts",
                    ))
                    .children(start),
            );
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
            drafts = drafts.child(
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
        let summary = self.summary.as_ref().map(|summary| {
            labelled(
                div().id("sheet-summary").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).child(Textarea::new(&summary.state)),
                Role::TextInput,
                "Summary",
            )
        });
        // While the review goes, its buttons wait; a refusal says why in the forge's words beside them.
        let (refused, writing) = self.live_write(page.number);
        let refused = refused.map(str::to_string);
        let send = |id: &'static str, said: &'static str, icon: &'static str, weight: Weight, verdict: Verdict, cx: &mut Context<Self>| {
            labelled(
                Btn::new(id, said, weight).icon(icon).disabled(writing).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.submit(verdict, window, cx))),
                Role::Button,
                said,
            )
        };
        let cancel = labelled(
            Btn::new("sheet-cancel", "Cancel", Weight::Ghost).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.open_sheet(false, window, cx))),
            Role::Button,
            "Cancel",
        );
        let comment = send("sheet-comment", "Comment", "scroll-text", Weight::Secondary, Verdict::Commented, cx);
        let changes = send("sheet-changes", "Request changes", "ban", Weight::Secondary, Verdict::ChangesRequested, cx);
        let approve = send("sheet-approve", "Approve", "check", Weight::Primary, Verdict::Approved, cx);
        labelled(
            panel(look)
                .id("sheet")
                .size_full()
                .flex()
                .flex_col()
                .border_t(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(look.color(token::ACCENT))
                .child(
                    div()
                        .id("sheet-body")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p(look.length(token::SPACE_4))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_4))
                        .child(heading)
                        .child(div().flex().flex_col().gap(look.length(token::SPACE_2)).child(caption("sheet-drafts", "Drafts that will be sent")).child(drafts))
                        .child(div().flex().flex_col().gap(look.length(token::SPACE_2)).child(caption("sheet-summary-said", "Summary")).children(summary)),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_3))
                        .bg(look.color(token::BG_SUNKEN))
                        .border_t(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(cancel)
                        .child(match refused {
                            Some(why) => labelled(
                                div()
                                    .id("sheet-refused")
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(look.length(token::TEXT_XS))
                                    .line_height(look.length(token::LEADING_XS))
                                    .text_color(look.color(token::RUN_FAILED))
                                    .child(SharedString::from(format!("Not sent: {why}"))),
                                Role::Status,
                                format!("Not sent: {why}"),
                            )
                            .into_any_element(),
                            None => div().flex_1().into_any_element(),
                        })
                        .child(comment)
                        .child(changes)
                        .child(approve),
                ),
            Role::Region,
            title,
        )
        .into_any_element()
    }
}
