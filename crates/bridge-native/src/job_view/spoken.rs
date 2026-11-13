//! What Listen said, as cards at the foot of the Job's thread: the amber edge, a pause or play disc, the bars that
//! fill as the line is said, the time, the words in quotes, Again and the voice's name. The scene is the owner's
//! sketch "one Job reporting in each way" (its Listen part), drawn from the same tokens.
//!
//! **Departures from the sketch:** Again is `rotate-ccw`, where the sketch draws `refresh-cw`, which the registry
//! keeps for churning. The words are upright: the Plex files ship no italic and gpui does not slant. A card that is queued or could not be spoken has its disc and Again off.

use std::time::Duration;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Hsla, Role, SharedString};

use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::listen::{m_ss, Card, State};
use crate::look::{token, Look};

/// How many bars the line is drawn in.
const BARS: usize = 22;

/// A bar's height as a share of the row: the sketch's pattern, the last rule that fits winning.
fn bar_height(at: usize) -> f32 {
    match at + 1 {
        n if n % 5 == 0 => 1.0,
        n if n % 3 == 0 => 0.55,
        n if n % 2 == 0 => 0.8,
        _ => 0.4,
    }
}

/// How many bars are filled for what has been said.
fn filled(played: Duration, total: Duration) -> usize {
    if total.is_zero() {
        return 0;
    }
    ((played.as_secs_f32() / total.as_secs_f32()) * BARS as f32).floor().clamp(0., BARS as f32) as usize
}

/// What the disc of a card that cannot be pressed says when it is hovered: why nothing can speak.
fn off_tip(state: State) -> &'static str {
    match state {
        State::Failed(why) => why.said(),
        _ => "Waiting for the line ahead",
    }
}

impl JobView {
    /// One spoken card of the shown Job.
    pub(super) fn spoken_card(&self, id: u64, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let listener = self.listen.read(cx);
        let card = listener.card(id)?.clone();
        let focus = listener.focuses(id)?.clone();
        let voice = listener.voice_name();
        let amber = look.color(token::SKETCH_AMBER);
        let live = matches!(card.state, State::Speaking | State::Paused | State::Spoken);
        let quoted = format!("\u{201c}{}\u{201d}", card.words);

        // The disc: pause while it speaks, play once it is paused or said. Off while it waits or could not be said.
        let (icon, said) = if card.state == State::Speaking { ("pause", "Pause") } else { ("play", "Play") };
        let disc_base = div().id(SharedString::from(format!("spoken-disc:{id}"))).flex_none().flex().items_center().justify_center().size(look.length(token::H_CONTROL_LG)).rounded_full();
        let disc: AnyElement = if live {
            let on_press = card.state == State::Speaking;
            labelled(
                disc_base
                    .bg(amber)
                    .cursor_pointer()
                    .track_focus(&focus.disc)
                    .focus_visible(|style| style.border_2().border_color(look.color(token::ACCENT)))
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Mark, look.color(token::FG_INVERSE)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.listen.update(cx, |listener, cx| if on_press { listener.pause(id, cx) } else { listener.again(id, cx) });
                    })),
                Role::Button,
                said,
            )
            .into_any_element()
        } else {
            let why = off_tip(card.state);
            labelled(
                // Dimmed as Again is, and no press: nothing can speak yet. The tooltip says why.
                disc_base.bg(look.color(token::BG_SUNKEN)).opacity(0.4).tooltip(tip(why)).child(glyph(look, "play", Size::Mark, look.color(token::FG_SUBTLE))),
                Role::Image,
                match card.state {
                    State::Failed(_) => "Not spoken",
                    _ => "Queued",
                },
            )
            .into_any_element()
        };

        // The bars say how much is said; a card that could not be spoken says why where they would be.
        let middle: AnyElement = match card.state {
            State::Failed(why) => labelled(
                div()
                    .id(SharedString::from(format!("spoken-fact:{id}")))
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(why.said())),
                Role::Status,
                why.said(),
            )
            .into_any_element(),
            _ => self.bars(&card, look, amber),
        };
        let time = (!card.failed()).then(|| {
            let said = format!("{} / {}", m_ss(card.played), m_ss(card.total));
            labelled(
                div()
                    .id(SharedString::from(format!("spoken-time:{id}")))
                    .flex_none()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .font_family(super::rows::mono(look))
                    .child(SharedString::from(said.clone())),
                Role::Timer,
                said,
            )
        });

        let again_on = live;
        let again = labelled(
            div()
                .id(SharedString::from(format!("spoken-again:{id}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::SURFACE_ROW))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .when(again_on, |button| {
                    button
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .track_focus(&focus.again)
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .on_click(cx.listener(move |this, _, _, cx| this.listen.update(cx, |listener, cx| listener.again(id, cx))))
                })
                .when(!again_on, |button| button.opacity(0.4))
                .tooltip(tip(if again_on { "Say it again" } else { "Not spoken yet" }))
                .child(glyph(look, "rotate-ccw", Size::Small, look.color(token::FG_MUTED)))
                .child("Again"),
            Role::Button,
            "Again",
        );

        let voice = voice.map(|name| {
            labelled(
                div()
                    .id(SharedString::from(format!("spoken-voice:{id}")))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(name.clone())),
                Role::Label,
                name,
            )
        });

        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("spoken:{id}")))
                    // Out past the words to the composer's edge, as the done card and the thread's other cards reach.
                    .mx(-super::bleed(look))
                    .flex()
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::SURFACE_CARD))
                    .shadow(look.shadows(token::SHADOW_CARD))
                    // The sketch's amber left edge, 3px.
                    .child(div().flex_none().w(look.length(token::SPACE_1) - look.length(token::BORDER_WIDTH)).bg(amber))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_2))
                            .p(look.length(token::SPACE_4))
                            .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).child(disc).child(middle).children(time))
                            .child(
                                div()
                                    .text_size(look.length(token::TEXT_SM))
                                    .line_height(look.length(token::TEXT_SM) * 1.5)
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                                    .child(SharedString::from(quoted)),
                            )
                            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(again).children(voice)),
                    ),
                Role::Article,
                card.words.clone(),
            )
            .into_any_element(),
        )
    }

    /// The row of bars: amber for what has been said, the strong border colour for the rest.
    fn bars(&self, card: &Card, look: &Look, amber: Hsla) -> AnyElement {
        let said = filled(card.played, card.total);
        let rest = look.color(token::BORDER_STRONG);
        let height = look.length(token::SPACE_6) + look.length(token::SPACE_1);
        labelled(
            div()
                .id(SharedString::from(format!("spoken-bars:{}", card.id)))
                .flex_1()
                // Room for the bars to read as bars: a narrower card wraps its time under them.
                .min_w(look.length(token::SPACE_12) * 2.5)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1) * 0.75)
                .h(height)
                .children((0..BARS).map(|at| {
                    // Each bar is as wide as the sketch's where there is room and narrower where there is not.
                    div().flex_1().min_w_0().max_w(look.length(token::SPACE_1)).h(height * bar_height(at)).rounded(look.length(token::SPACE_1) / 2.).bg(if at < said { amber } else { rest })
                })),
            Role::ProgressIndicator,
            "Progress",
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_that_cannot_speak_says_why_on_hover() {
        use crate::listen::Why;
        assert_eq!(off_tip(State::Failed(Why::HelperMissing)), "Armada Voice is missing");
        assert_eq!(off_tip(State::Failed(Why::VoiceMissing)), "The voice is missing");
        assert_eq!(off_tip(State::Queued), "Waiting for the line ahead");
    }

    #[test]
    fn the_bars_fill_with_what_is_said_and_follow_the_sketch_s_heights() {
        assert_eq!(filled(Duration::ZERO, Duration::from_secs(11)), 0);
        assert_eq!(filled(Duration::from_secs(6), Duration::from_secs(11)), 12);
        assert_eq!(filled(Duration::from_secs(11), Duration::from_secs(11)), BARS);
        assert_eq!(filled(Duration::from_secs(3), Duration::ZERO), 0);
        let heights: Vec<f32> = (0..6).map(bar_height).collect();
        assert_eq!(heights, [0.4, 0.8, 0.55, 0.8, 1.0, 0.55]);
    }
}
