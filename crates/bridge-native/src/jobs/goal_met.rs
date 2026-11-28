//! The Goal met? card a sub project's orchestrator raises (`WaitingSource::GoalMet`), drawn the same on its board and on the
//! orchestrator's own Needs you: the goal, then each pull request that landed as `#number title`, then the answers its caller
//! wires, Agree and Not yet.

use gpui::{div, prelude::*, svg, AnyElement, ElementId, FontWeight, Role, SharedString};

use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The card, its head band in the hue of a Job waiting on him, with `answers` at its foot.
pub(crate) fn card(look: &Look, id: &'static str, goal: &SharedString, landed: &[(u64, SharedString)], answers: impl IntoElement) -> AnyElement {
    let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
    let pulls = landed.iter().enumerate().map(|(at, (number, title))| {
        let name: SharedString = format!("#{number} {title}").into();
        labelled(
            div()
                .id(ElementId::NamedInteger(format!("{id}-landed").into(), at as u64))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .when(at > 0, |line| line.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .text_size(look.length(token::TEXT_SM))
                .child(svg().path(icon_path("git-merge")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::STATUS_COMPLETED_SUCCESS)))
                .child(div().flex_none().font_family(mono.clone()).text_color(look.color(token::FG_MUTED)).child(format!("#{number}")))
                .child(div().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(title.clone())),
            Role::ListItem,
            name,
        )
    });
    let landed_list = (!landed.is_empty()).then(|| {
        div()
            .flex()
            .flex_col()
            .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Landed"))
            .child(labelled(div().id(ElementId::Name(format!("{id}-landed").into())).flex().flex_col().children(pulls), Role::List, "Landed"))
    });
    labelled(
        crate::parts::card::card(look, id, None)
            .flex_none()
            .child(crate::parts::card::head(look, ElementId::Name(format!("{id}-head").into()), Some(look.color(token::STATUS_AWAITING_REVIEW)), "Goal met?"))
            .child(
                crate::parts::card::body(look, ElementId::Name(format!("{id}-body").into()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Goal"))
                            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(goal.clone())),
                    )
                    .children(landed_list)
                    .child(answers),
            ),
        Role::Group,
        "Goal met?",
    )
    .into_any_element()
}
