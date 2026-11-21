//! A mock for the owner to judge, drawn only in the `planning-margin` scenario: the talk about a page stands in its
//! margin, one thread beside each passage. A question is answered there and the page stays as it is until he presses
//! Make this change; a change the Job proposes stands in the page as a suggestion with Accept and Reject; a settled thread
//! leaves the open list for the page's Decisions. Nothing here reaches Fleet.

use gpui::{div, prelude::*, px, AnyElement, FontWeight, Hsla, SharedString};

use super::acts::{button, Tone};
use super::sketch_card::mixed;
use crate::assets::icon_path;
use crate::look::{token, Look};

/// The margin's width: room for a thread's words at a reading measure.
const WIDTH: f32 = 380.;

/// Who said a line of a thread.
#[derive(Clone, Copy)]
enum Who {
    You,
    Agent,
}

fn glyph(look: &Look, name: &str, ink: Hsla) -> gpui::Svg {
    gpui::svg().path(icon_path(name)).flex_none().size(look.length(token::TEXT_SM)).text_color(ink)
}

fn said(look: &Look, who: Who, at: &str, words: &str) -> AnyElement {
    let (name, icon, ink) = match who {
        Who::You => ("You", "user", look.color(token::YOU)),
        Who::Agent => ("Agent", "bot", look.color(token::ACCENT)),
    };
    div()
        .flex()
        .gap(look.length(token::SPACE_2))
        .child(div().pt(px(2.)).child(glyph(look, icon, ink)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .flex()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(name))
                        .child(div().text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(at.to_string()))),
                )
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(words.to_string()))),
        )
        .into_any_element()
}

/// The passage a thread is on, cut to one line, in his colour.
fn quote(look: &Look, words: &str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_2))
        .py(px(2.))
        .rounded(look.length(token::RADIUS_SM))
        .bg(mixed(look, token::YOU, 0.12))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .text_color(look.color(token::FG_MUTED))
        .child(div().min_w_0().truncate().child(SharedString::from(format!("\u{201c}{words}\u{201d}"))))
        .into_any_element()
}

/// A chip that says where a thread stands.
fn chip(look: &Look, icon: &str, words: &str, ink: crate::look::Token) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .bg(mixed(look, ink, 0.16))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(mixed(look, ink, 0.4))
        .text_size(look.length(token::TEXT_2XS))
        .text_color(look.color(ink))
        .child(gpui::svg().path(icon_path(icon)).flex_none().size(look.length(token::TEXT_2XS)).text_color(look.color(ink)))
        .child(SharedString::from(words.to_string()))
        .into_any_element()
}

/// A box he types a reply in, as the composer draws one, empty.
fn reply(look: &Look, id: &str, placeholder: &str) -> AnyElement {
    div()
        .id(SharedString::from(format!("talk-reply:{id}")))
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_XS))
        .text_color(look.color(token::FG_SUBTLE))
        .child(SharedString::from(placeholder.to_string()))
        .into_any_element()
}

fn card(look: &Look, id: &str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("talk-thread:{id}")))
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .p(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if on { mixed(look, token::YOU, 0.55) } else { look.color(token::BORDER_DEFAULT) })
        .bg(look.color(token::BG_RAISED))
        .when(on, |card| card.shadow_md())
}

/// One line of the suggestion: the words taken out or put in, in the diff's colours.
fn change_line(look: &Look, added: bool, words: &str) -> AnyElement {
    let (bg, fg, sign) = if added { (token::DIFF_ADD_BG, token::DIFF_ADD_FG, "+") } else { (token::DIFF_DEL_BG, token::DIFF_DEL_FG, "\u{2212}") };
    div()
        .flex()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_2))
        .py(px(2.))
        .bg(look.color(bg))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(fg))
        .child(div().flex_none().child(sign))
        .child(div().flex_1().min_w_0().when(!added, |line| line.line_through()).child(SharedString::from(words.to_string())))
        .into_any_element()
}

fn tab(look: &Look, words: &str, on: bool) -> AnyElement {
    div()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .when(on, |tab| tab.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)))
        .when(!on, |tab| tab.text_color(look.color(token::FG_MUTED)))
        .child(SharedString::from(words.to_string()))
        .into_any_element()
}

/// The margin: Open and Settled at its head, then a thread for each passage he or the Job raised.
pub(super) fn margin(look: &Look) -> AnyElement {
    let head = div()
        .flex()
        .items_center()
        .justify_between()
        .child(div().flex().gap(look.length(token::SPACE_1)).p(px(2.)).rounded(look.length(token::RADIUS_MD)).bg(look.color(token::BG_SUNKEN)).child(tab(look, "Open", true)).child(tab(look, "Settled", false)))
        .child(div().flex().items_center().gap(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(super::acts::kbd(look, "N")).child("comment on a passage"));

    let asked = card(look, "nine", true)
        .child(quote(look, "Notices that arrive wait until 9:00 and come in as one digest"))
        .child(said(look, Who::You, "10:31", "Why 9:00? I start at 8."))
        .child(said(look, Who::Agent, "10:31", "9:00 is the work start in Settings. Quiet hours could end at your own work start instead, 8:00 for you."))
        .child(div().flex().child(chip(look, "message-square", "Answered. The page is unchanged", token::ACCENT)))
        .child(reply(look, "nine", "Reply"))
        .child(
            div()
                .flex()
                .gap(look.length(token::SPACE_2))
                .child(button(look, "talk-make:nine", "Make this change", Tone::Secondary, false, None, true))
                .child(button(look, "talk-settle:nine", "Settle", Tone::Ghost, false, None, true)),
        );

    let suggested = card(look, "branch", false)
        .child(quote(look, "A failed check still interrupts."))
        .child(said(look, Who::You, "10:40", "Only main should wake me."))
        .child(said(look, Who::Agent, "10:41", "Suggested in the page:"))
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_MD))
                .overflow_hidden()
                .child(change_line(look, false, "A failed check still interrupts."))
                .child(change_line(look, true, "A failed check on main still interrupts; one on a branch waits for the digest.")),
        )
        .child(
            div()
                .flex()
                .gap(look.length(token::SPACE_2))
                .child(button(look, "talk-accept:branch", "Accept", Tone::Primary, false, None, true))
                .child(button(look, "talk-reject:branch", "Reject", Tone::Secondary, false, None, true)),
        );

    let drafting = card(look, "group", false)
        .child(quote(look, "grouped by Job"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::ACCENT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child("Could the digest group by project instead?")
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Not sent"))
                        .child(
                            div()
                                .flex()
                                .gap(look.length(token::SPACE_2))
                                .child(button(look, "talk-hold:group", "Hold", Tone::Secondary, false, None, true))
                                .child(button(look, "talk-send:group", "Send now", Tone::Primary, false, Some("\u{2318}\u{21b5}"), true)),
                        ),
                ),
        );

    // A comment he held: it waits with the others until he sends them together.
    let held = card(look, "sound", false)
        .child(quote(look, "The sound stays off until the first notice after 9:00."))
        .child(said(look, Who::You, "10:52", "Could the sound come back at my work start too?"))
        .child(div().flex().child(chip(look, "clock", "Held", token::YOU)));

    // The comments he held, sent in one go: the agent answers them together.
    let batch = div()
        .id("talk-batch")
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(mixed(look, token::YOU, 0.12))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(mixed(look, token::YOU, 0.4))
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .child(glyph(look, "clock", look.color(token::YOU)))
        .child(div().flex_1().min_w_0().truncate().child("1 comment held"))
        .child(button(look, "talk-send-held", "Send held", Tone::Secondary, false, Some("\u{21e7}\u{2318}\u{21b5}"), true));

    let settled = div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .pt(look.length(token::SPACE_2))
        .border_t(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Settled today"))
        .child(
            div()
                .id("talk-settled:weekends")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph(look, "circle-check", look.color(token::STATUS_COMPLETED_SUCCESS)))
                .child(div().flex_1().min_w_0().truncate().child("Weekends are quiet all day"))
                .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::ACCENT)).child("In Decisions")),
        );

    div()
        .id("talk-margin")
        .flex_none()
        .w(px(WIDTH))
        .min_h_0()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_4))
        .border_l(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_BASE))
        .overflow_y_scroll()
        .child(head)
        .child(batch)
        .child(asked)
        .child(suggested)
        .child(drafting)
        .child(held)
        .child(settled)
        .into_any_element()
}

/// The page beside the thread: a revision the Job wrote while he reads waits here, and the page he reads stays where he
/// scrolled it, his notes with it, until he presses Show it.
pub(super) fn newer_strip(look: &Look) -> AnyElement {
    div()
        .id("talk-newer")
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .mx(look.length(token::SPACE_4))
        .mt(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .bg(mixed(look, token::ACCENT, 0.12))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(mixed(look, token::ACCENT, 0.4))
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .child(glyph(look, "refresh-cw", look.color(token::ACCENT)))
        .child(div().flex_1().min_w_0().truncate().child("Revision 3 is ready: work start, and failed checks on main"))
        .child(button(look, "talk-show-changes", "Show changes", Tone::Secondary, false, None, true))
        .child(button(look, "talk-show-it", "Show it", Tone::Primary, false, None, true))
        .into_any_element()
}

/// What he sent from a page, as the mocks draw it in the thread: the page and the section it is about, the passage, his
/// whole note, and Open, which goes to that spot. Fleet's line for the agent and the page's path are never shown.
/// `None` where `text` is not a note sent from a page.
pub(super) fn sent_card(look: &Look, id: &str, text: &str) -> Option<AnyElement> {
    if let Some(card) = round_card(look, id, text) {
        return Some(card);
    }
    let mut lines = text.lines();
    let first = lines.next()?;
    let rest = first.strip_prefix("Sent back document: ")?;
    let (title, revision) = rest.rsplit_once(", revision ").unwrap_or((rest, ""));
    let mut section = "";
    let mut passage = "";
    let mut note: Vec<&str> = Vec::new();
    for line in lines {
        if line.starts_with("Revise the page at ") {
            continue;
        }
        if let Some(words) = line.strip_prefix("Section: ") {
            section = words;
        } else if let Some(words) = line.strip_prefix("Passage: ") {
            passage = words;
        } else {
            note.push(line);
        }
    }
    let where_ = if section.is_empty() { format!("Revision {revision}") } else { format!("{section} \u{b7} Revision {revision}") };
    let card = div()
        .id(SharedString::from(format!("sent-card:{id}")))
        .w_full()
        .max_w(px(560.))
        .flex()
        .flex_col()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(mixed(look, token::YOU, 0.45))
        .bg(look.color(token::BG_RAISED))
        .overflow_hidden()
        .child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .bg(mixed(look, token::YOU, 0.14))
                .child(glyph(look, "file-text", look.color(token::YOU)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.to_string())))
                        .child(div().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(where_))),
                )
                .child(chip(look, "reply", "Sent back", token::YOU))
                .child(button(look, SharedString::from(format!("sent-open:{id}")), "Open", Tone::Secondary, false, None, true)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_3))
                .when(!passage.is_empty(), |body| body.child(quote(look, passage)))
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(note.join("\n").trim().to_string()))),
        );
    Some(div().w_full().flex().justify_end().child(card).into_any_element())
}

/// One round of the talk in a page's margin, as the margin mock draws it in the thread: one line, the page, what came of
/// the round, and Open, which goes to the margin. The talk itself stays in the margin. `None` where `text` is not a round.
fn round_card(look: &Look, id: &str, text: &str) -> Option<AnyElement> {
    let mut lines = text.lines();
    let title = lines.next()?.strip_prefix("Margin round: ")?;
    let summary = lines.next().unwrap_or("");
    let waiting = lines.next().is_some_and(|state| state.trim() == "waiting");
    let (icon, ink) = if waiting { ("messages-square", token::YOU) } else { ("circle-check", token::STATUS_COMPLETED_SUCCESS) };
    let line = div()
        .id(SharedString::from(format!("round-card:{id}")))
        .w_full()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if waiting { mixed(look, token::YOU, 0.45) } else { look.color(token::BORDER_SUBTLE) })
        .bg(if waiting { mixed(look, token::YOU, 0.10) } else { look.color(token::BG_RAISED) })
        .text_size(look.length(token::TEXT_SM))
        .child(glyph(look, icon, look.color(ink)))
        .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.to_string())))
        .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(summary.to_string())))
        .child(button(look, SharedString::from(format!("round-open:{id}")), "Open", if waiting { Tone::Secondary } else { Tone::Ghost }, false, None, true));
    Some(line.into_any_element())
}
