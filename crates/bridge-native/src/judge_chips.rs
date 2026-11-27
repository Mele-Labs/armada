//! The Judge's box as the Job view and the Reviews page both draw it: a row of judge types, one chip each, the chosen one in
//! the Judge's hue, and the field that asks for anything in particular. Where the choice is kept, and what a press does,
//! is the screen's: this draws, and hands a press or an arrow back.

use std::rc::Rc;

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, Role, SharedString, Toggled, Window};
use gpui_component::input::{Textarea, TextareaState};

use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::tip;

/// What a screen has read of the types: none yet, what Fleet said, or why it could not.
pub type Read<'a> = Option<&'a Result<crate::judge_types::Listed, String>>;

/// The type a choice means in a list read: the one chosen, else Adversarial, else the first.
pub fn chosen_in<'a>(list: &'a [ipc::JudgeType], chosen: &str) -> Option<&'a ipc::JudgeType> {
    list.iter().find(|one| one.id == chosen).or_else(|| list.iter().find(|one| one.id == crate::judge_types::DEFAULT)).or_else(|| list.first())
}

/// The id one step along the row from the choice, the ends holding: `None` where the choice does not move.
pub fn stepped(list: &[ipc::JudgeType], chosen: &str, delta: isize) -> Option<String> {
    let at = chosen_in(list, chosen).and_then(|one| list.iter().position(|other| other.id == one.id)).unwrap_or(0);
    let to = at.saturating_add_signed(delta).min(list.len().saturating_sub(1));
    (to != at).then(|| list[to].id.clone())
}

/// Fleet's reason for a refused act, drawn under the buttons of the box or the card it came from.
pub fn refusal(look: &Look, key: &str, why: SharedString) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("refused:{key}")))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::STATUS_COMPLETED_FAILED))
            .child(why),
        Role::Status,
        "Refused",
    )
    .into_any_element()
}

/// The row of judge types: a chip for each, the chosen one in the Judge's hue. Bones while the list is read, Fleet's reason
/// where it could not be. `pick` takes a chip pressed, `step` an arrow: ← and → move the choice one type along.
pub fn chips<V: 'static>(
    key: &str,
    read: Read,
    chosen: &str,
    focus: &FocusHandle,
    look: &Look,
    cx: &Context<V>,
    pick: impl Fn(&mut V, String, &mut Window, &mut Context<V>) + 'static,
    step: impl Fn(&mut V, isize, &mut Window, &mut Context<V>) + 'static,
) -> AnyElement {
    let hue = look.color(token::STATUS_AWAITING_REVIEW);
    let tall = look.length(token::H_CONTROL_SM);
    let list = match read {
        None => {
            let bones = [0.8, 1.1, 0.7].into_iter().enumerate().map(|(at, wide)| {
                div().flex_none().h(tall).w(look.length(token::SPACE_12) * wide).rounded(look.length(token::RADIUS_MD)).bg(look.color(token::FG_DEFAULT).opacity(0.07)).with_animation(
                    SharedString::from(format!("judge-types-{key}-{at}")),
                    Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                    |shape, at| shape.opacity(at),
                )
            });
            return labelled(div().id(SharedString::from(format!("judge-types:{key}"))).flex().gap(look.length(token::SPACE_2)).children(bones), Role::Status, "Judge types").into_any_element();
        }
        Some(Err(why)) => return refusal(look, key, why.clone().into()),
        Some(Ok(list)) => list,
    };
    let Some(on) = chosen_in(list, chosen).map(|one| one.id.clone()) else { return div().into_any_element() };
    let pick = Rc::new(pick);
    let chip = |one: &ipc::JudgeType| {
        let (id, selected) = (one.id.clone(), one.id == on);
        let tip_text = crate::judge_types::first_sentence(&one.prompt);
        let pick = pick.clone();
        let chip = div()
            .id(SharedString::from(format!("judge-type:{key}:{}", one.id)))
            .flex_none()
            .flex()
            .items_center()
            .h(tall)
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if selected { hue } else { look.color(token::BORDER_DEFAULT) })
            .when(selected, |chip| chip.bg(hue.opacity(0.14)).text_color(hue).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
            .when(!selected, |chip| chip.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER))))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .aria_toggled(if selected { Toggled::True } else { Toggled::False })
            .when(!tip_text.is_empty(), |chip| chip.tooltip(tip(tip_text.clone())))
            .child(SharedString::from(one.name.clone()))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                pick(this, id.clone(), window, cx);
            }));
        labelled(chip, Role::RadioButton, one.name.clone()).into_any_element()
    };
    let row = div()
        .id(SharedString::from(format!("judge-types:{key}")))
        .track_focus(focus)
        .flex()
        .flex_wrap()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            let delta = match event.keystroke.key.as_str() {
                "left" => -1,
                "right" => 1,
                _ => return,
            };
            if !event.keystroke.modifiers.modified() {
                cx.stop_propagation();
                step(this, delta, window, cx);
            }
        }))
        .children(list.iter().map(chip));
    labelled(row, Role::RadioGroup, "Judge type").into_any_element()
}

/// The field under the chips, which asks for anything in particular.
pub fn prompt_field(key: &str, area: &Entity<TextareaState>, look: &Look) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("judge-box:{key}")))
            .key_context(crate::keymap::TEXT_BOX)
            .w_full()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(Textarea::new(area).appearance(false).aria_label("Anything in particular?")),
        Role::TextInput,
        "Anything in particular?",
    )
    .into_any_element()
}
