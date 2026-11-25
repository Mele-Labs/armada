//! The rows of the Docs, Tickets and Decisions cards. A doc or ticket says its title and where it was read by, then what it says,
//! then the Job that read it and the day; a decision says the question, then what he chose, then the Job and the day. The glyph
//! of a row is its kind in a hue, and every mark on it has a tooltip.

use gpui::{div, prelude::*, px, AnyElement, Div, ElementId, FontWeight, Role, SharedString, Stateful};

use super::asks::{frame, Spot};
use super::dossier::{pill, pill_quiet};
use super::page_model::{DecisionRow, ReadRow};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::tip;
use crate::titlebar::parts::{glyph, mono};

/// What the tooltip of a read's glyph says.
fn kind_said(glyph: &str) -> &'static str {
    match glyph {
        "ticket" => "A ticket",
        "globe" => "A web or MCP address",
        _ => "A file in the repository",
    }
}

fn hue_of(glyph: &str) -> Token {
    if glyph == "ticket" {
        token::STATUS_HANDED_IN
    } else {
        token::ACCENT
    }
}

/// The glyph's cell, in the column a row's mark stands in.
fn mark(look: &Look, id: SharedString, name: &str, ink: gpui::Hsla, said: &'static str) -> impl IntoElement {
    let cell = div().id(id).flex_none().flex().items_center().justify_center().w(look.length(token::SPACE_3)).h(look.length(token::LEADING_XS)).tooltip(tip(said));
    labelled(cell.child(glyph(name, look.length(token::SPACE_3), ink)), Role::Image, said)
}

fn line(look: &Look) -> Div {
    div().flex().flex_wrap().items_center().gap_x(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS))
}

fn last(look: &Look, words: &str) -> Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(words.to_string()))
}

/// A doc or a ticket a Job read. A doc whose address is gone has its title struck through and subtle, and says so in a chip.
/// `under` is the flag a press of Check left, drawn before the last line; a doc a check found agreeing says so last.
pub fn read(look: &Look, id: impl Into<ElementId>, row: &ReadRow, spot: Spot, under: Option<AnyElement>) -> Stateful<Div> {
    let id: ElementId = id.into();
    let key = format!("{id}");
    let ink = if row.gone { look.color(token::FG_SUBTLE) } else { look.color(hue_of(row.glyph)) };
    let title_ink = look.color(if row.gone { token::FG_SUBTLE } else { token::FG_DEFAULT });
    let mut title = div().min_w_0().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(title_ink).child(SharedString::from(row.title.clone()));
    if row.gone {
        title = title.line_through();
    }
    let head = line(look)
        .child(title)
        .when(!row.title.contains(row.target.as_str()), |head| head.child(div().min_w_0().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(row.target.clone()))));
    let gist = (!row.gist.is_empty()).then(|| {
        let tag = row.by_job.then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("{key}-by-job")))
                    .flex_none()
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip("The Job that read it wrote this line"))
                    .child("Job's words"),
                Role::Label,
                "Job's words",
            )
        });
        line(look).text_color(look.color(token::FG_MUTED)).child(div().min_w_0().child(SharedString::from(row.gist.clone()))).children(tag)
    });
    let drift = (!row.drifted.is_empty()).then(|| {
        let mut names = line(look).gap_y(look.length(token::SPACE_1)).child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Names the code no longer has:"));
        for name in &row.drifted {
            names = names.child(
                div()
                    .flex_none()
                    .px(look.length(token::SPACE_1) + px(1.))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                    .child(SharedString::from(name.clone())),
            );
        }
        labelled(names.id(SharedString::from(format!("{key}-drift"))), Role::Label, format!("Names the code no longer has: {}", row.drifted.join(", ")))
    });
    let gone = row.gone.then(|| labelled(pill_quiet(look, "gone").id(SharedString::from(format!("{key}-gone"))).tooltip(tip("This address no longer answers")), Role::Label, "gone"));
    let tail = (gone.is_some() || !row.last.is_empty()).then(|| line(look).gap_x(look.length(token::SPACE_2)).children(gone).child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(row.last.clone()))));
    let checked = row.checked.as_ref().map(|said| labelled(last(look, said).id(SharedString::from(format!("{key}-checked"))), Role::Label, said.clone()));
    let body = div().flex_1().min_w_0().flex().flex_col().child(head).children(gist).children(drift).children(under).children(tail).children(checked);
    frame(look, id, spot).child(mark(look, SharedString::from(format!("{key}-kind")), row.glyph, ink, kind_said(row.glyph))).child(body)
}

/// A decision: the question, what he chose as a chip in the done hue and his note in quotes, then the Job and the day.
pub fn decision(look: &Look, id: impl Into<ElementId>, row: &DecisionRow, spot: Spot) -> Stateful<Div> {
    let id: ElementId = id.into();
    let key = format!("{id}");
    let choice = line(look).child(pill(look, token::STATUS_COMPLETED_SUCCESS, row.choice.clone())).children(row.note.as_ref().map(|note| div().min_w_0().text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("\"{note}\"")))));
    let body = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.question.clone())))
        .child(choice)
        .children((!row.last.is_empty()).then(|| last(look, &row.last)));
    frame(look, id, spot).child(mark(look, SharedString::from(format!("{key}-kind")), "check", look.color(token::STATUS_COMPLETED_SUCCESS), "Settled by you")).child(body)
}
