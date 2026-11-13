//! The pull request's head, with Your review pinned under its title: where his review stands, what
//! came after it, and Review changes, whose sheet takes the panes' place until Send or Cancel. It is
//! not a pane, so no layout moves it.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use gpui_component::input::Textarea;

use super::inbox::verdict_hue;
use super::model::{Page, Verdict};
use super::panes::place_line;
use super::{panel, ReviewsScreen};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, icon_button, mono, tip, Btn, Weight};

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
        let meta = div()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .truncate()
            .child(SharedString::from(format!("{} · {} wants {} into {}", page.repository, page.author, page.branch, page.base)));
        panel(look)
            .flex_none()
            .flex()
            .flex_col()
            .child(div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_2)).flex().flex_col().gap(look.length(token::SPACE_1)).child(heading).child(meta))
            .child(self.render_bar(page, look, cx))
            .into_any_element()
    }

    /// Your review at rest: its state and day, what came since, the drafts waiting, and Review changes.
    fn render_bar(&self, page: &Page, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let mine = &page.mine;
        let (hue, ground) = verdict_hue(mine.verdict);
        let said = mine.chip();
        let state = labelled(
            div()
                .id("mine-state")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_BADGE))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(ground))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(hue))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(hue))
                .tooltip(tip(mine.tip()))
                .child(glyph(mine.verdict.glyph(), look.length(token::SPACE_3), look.color(hue)))
                .child(SharedString::from(said.clone())),
            Role::Status,
            said,
        );
        let since = mine.since().map(|since| {
            labelled(
                div().id("mine-since").flex_none().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(since.clone())),
                Role::Label,
                since,
            )
        });
        let drafts = self.drafts.iter().filter(|draft| draft.number == page.number).count();
        let drafted = (drafts > 0).then(|| {
            let said = if drafts == 1 { "1 draft".to_string() } else { format!("{drafts} drafts") };
            labelled(
                div()
                    .id("mine-drafts")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_color(look.color(token::ACCENT))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .tooltip(tip("Drafts go with your next review"))
                    .child(glyph("pencil", look.length(token::SPACE_3), look.color(token::ACCENT)))
                    .child(SharedString::from(said.clone())),
                Role::Label,
                said,
            )
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
                .children(since)
                .children(drafted)
                .child(div().flex_1())
                .child(review),
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
            drafts = drafts.child(labelled(
                div().id("sheet-no-drafts").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child("No drafts"),
                Role::Label,
                "No drafts",
            ));
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
        let send = |id: &'static str, said: &'static str, icon: &'static str, weight: Weight, verdict: Verdict, cx: &mut Context<Self>| {
            labelled(
                Btn::new(id, said, weight).icon(icon).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| this.submit(verdict, window, cx))),
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
                        .child(div().flex_1())
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
