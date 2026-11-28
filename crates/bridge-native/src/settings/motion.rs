//! Settings › Motion (`MotionSettings`): how fast Bridge moves. One figure today, the seconds one turn of the busy ring
//! takes, stepped by Slower and Faster, with the ring itself turning beside the stepper at the speed it names.
//!
//! The figure Fleet last said he set is held on the thread, so `parts/ring.rs` reads it without a `Look` or a context.

use std::time::Duration;

use gpui::{div, prelude::*, svg, AnyElement, App, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::motion::{Motion, DEFAULT_RING_SECONDS, RING_SECONDS};

use super::entries::Entry;
use super::overlay::SettingsOverlay;
use super::rows::{self, rows_card, split_row, Row, Spread};
use super::view::{ghost, ringed};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

thread_local! {
    /// What Fleet last said he set (`motion`): read once a connection, and again after each save.
    static HELD: std::cell::Cell<Motion> = std::cell::Cell::new(Motion::default());
}

/// What he has set, for anything drawn on this thread.
pub fn held() -> Motion {
    HELD.with(std::cell::Cell::get)
}

/// Take Fleet's `motion` text, on a read or after a save. Text that is not a motion is what ships.
pub fn hold(text: &str) {
    HELD.with(|held| held.set(ipc::motion::read(text)));
}

/// Seconds one turn of the busy ring takes.
pub fn ring_seconds() -> u32 {
    held().ring_seconds
}

/// One turn of the busy ring, as a duration.
pub fn ring_turn() -> Duration {
    Duration::from_secs(u64::from(ring_seconds()))
}

/// `seconds` a step off, kept inside the range.
pub fn stepped(seconds: u32, by: i32) -> u32 {
    seconds.saturating_add_signed(by).clamp(*RING_SECONDS.start(), *RING_SECONDS.end())
}

/// The three stops of the page, in tab order.
pub const STOPS: [&str; 3] = ["motion:less", "motion:more", "motion:reset"];

impl SettingsOverlay {
    /// Whether the keys are on the stepper, whose arrows are its own and not the sidebar's.
    pub(super) fn on_motion(&self, window: &Window) -> bool {
        STOPS.iter().any(|name| self.prefs.keys.get(*name).is_some_and(|focus| focus.is_focused(window)))
    }

    fn motion(&self) -> Motion {
        ipc::motion::read(&self.prefs.held().map(|held| held.motion.clone()).unwrap_or_default())
    }

    fn motion_to(&mut self, seconds: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.save_pref("motion", Motion { ring_seconds: seconds }.text(), window, cx);
    }

    pub(super) fn motion_stops(&self, _cx: &App) -> Vec<gpui::FocusHandle> {
        STOPS.iter().filter_map(|name| self.prefs.keys.get(*name).cloned()).collect()
    }

    pub(super) fn render_motion(&self, entry: &'static Entry, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let spread = self.spread(look, window, cx);
        let card = match &self.prefs.read {
            None => self.row_bones(look, spread),
            Some(Err(said)) => labelled(super::view::refusal(look, said.clone()).id("motion-unread"), Role::Alert, said.clone()).into_any_element(),
            Some(Ok(_)) => self.motion_rows(spread, look, window, cx),
        };
        self.rows_page(entry, rows::purpose_of(entry).into(), false, Vec::new(), vec![card], look, window, cx)
    }

    /// Fewer or More: a press, Enter or Space, and the arrows on either, step the figure.
    fn motion_side(&self, name: &'static str, glyph: &'static str, said: &'static str, by: i32, seconds: u32, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let to = stepped(seconds, by);
        let dead = !self.live || to == seconds;
        let handle = self.prefs.keys.get(name).cloned();
        let button = div()
            .id(SharedString::from(name))
            .flex_none()
            .w(look.length(token::H_CONTROL))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .when(!dead, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(if dead { look.color(token::FG_SUBTLE) } else { look.color(token::FG_MUTED) }));
        let button = match handle {
            Some(focus) => ringed(look, button.track_focus(&focus), &focus, window),
            None => button,
        };
        let button = button.when(!dead, |button| {
            button
                .on_click(cx.listener(move |this, _, window, cx| this.motion_to(to, window, cx)))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let step = match event.keystroke.key.as_str() {
                        "enter" | "space" => by,
                        "right" | "up" => 1,
                        "left" | "down" => -1,
                        _ => return,
                    };
                    let now = this.motion().ring_seconds;
                    this.motion_to(stepped(now, step), window, cx);
                }))
        });
        labelled(button, Role::Button, said).into_any_element()
    }

    fn motion_rows(&self, spread: Spread, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let seconds = self.motion().ring_seconds;
        let shipped = seconds == DEFAULT_RING_SECONDS && self.prefs.held().is_none_or(|held| held.motion.is_empty());
        let edge = look.length(token::BORDER_WIDTH);
        let value = labelled(
            div()
                .id("motion-value")
                .flex_none()
                .min_w(look.length(token::SPACE_8) + look.length(token::SPACE_2))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l(edge)
                .border_r(edge)
                .border_color(look.color(token::BORDER_SUBTLE))
                .text_size(look.length(token::TEXT_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(if seconds == DEFAULT_RING_SECONDS { look.color(token::FG_DEFAULT) } else { look.color(token::ACCENT) })
                .child(SharedString::from(format!("{seconds} s"))),
            Role::Status,
            format!("Seconds per turn: {seconds}"),
        );
        let stepper = div()
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_CONTROL))
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_MD))
            .border(edge)
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .child(self.motion_side("motion:less", "minus", "Faster: one second less per turn", -1, seconds, look, window, cx))
            .child(value)
            .child(self.motion_side("motion:more", "plus", "Slower: one second more per turn", 1, seconds, look, window, cx));
        let off = !self.live || shipped;
        let reset = ghost(look, "motion-reset", "Reset", off);
        let reset = match self.prefs.keys.get("motion:reset").cloned() {
            Some(focus) => ringed(look, reset.track_focus(&focus), &focus, window),
            None => reset,
        }
        .when(!off, |reset| {
            reset.on_click(cx.listener(|this, _, window, cx| this.save_pref("motion", String::new(), window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.save_pref("motion", String::new(), window, cx);
                }
            }))
        });
        // The ring as the busy button draws it, at the speed on the stepper before it is saved.
        let radius = look.length(token::RADIUS_MD);
        let reset = labelled(reset, Role::Button, "Reset to default");
        let preview = labelled(
            div()
                .id("motion-preview")
                .relative()
                .flex_none()
                .w(look.length(token::SPACE_12) * 2.)
                .h(look.length(token::H_CONTROL))
                .rounded(radius)
                .border(edge)
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .child(crate::parts::ring::ring_overlay_in(look, SharedString::from(format!("motion-preview-ring-{seconds}")), radius, look.color(token::STATUS_RUNNING), seconds)),
            Role::Label,
            format!("Ring preview at {seconds} seconds per turn"),
        );
        let control = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(stepper).child(reset).child(preview))
            .children(self.prefs.refused.get("motion").map(|said| labelled(super::view::refusal(look, said.clone()).id("motion-refused"), Role::Alert, said.clone()).into_any_element()));
        let row = Row {
            id: "row-motion-ring".into(),
            title: "Busy ring".into(),
            tip: None,
            chips: Vec::new(),
            says: vec![rows::says(look, format!("Seconds one turn takes, {} to {}.", RING_SECONDS.start(), RING_SECONDS.end())).into_any_element()],
            changed: !shipped,
            glyph: None,
            control: control.into_any_element(),
        };
        rows_card(look, "settings-card-motion", vec![split_row(look, spread, row, true).into_any_element()]).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_motion_is_what_fleet_last_said_and_reads_six_where_none_is_set() {
        assert_eq!(ring_seconds(), 6);
        assert_eq!(ring_turn(), Duration::from_secs(6));
        hold(r#"{"ringSeconds":12}"#);
        assert_eq!(ring_seconds(), 12);
        assert_eq!(ring_turn(), Duration::from_secs(12));
        hold("");
        assert_eq!(ring_seconds(), 6);
    }

    #[test]
    fn a_value_outside_the_range_or_malformed_reads_as_six() {
        for text in [r#"{"ringSeconds":1}"#, r#"{"ringSeconds":21}"#, r#"{"ringSeconds":0}"#, r#"{"ringSeconds":2.5}"#, r#"{"ringSeconds":"9"}"#, "{", "[]", r#"{"ringSeconds":9,"x":1}"#] {
            hold(r#"{"ringSeconds":12}"#);
            hold(text);
            assert_eq!(ring_seconds(), 6, "{text}");
        }
        hold(r#"{"ringSeconds":2}"#);
        assert_eq!(ring_seconds(), 2, "the slowest edge is inside");
        hold(r#"{"ringSeconds":20}"#);
        assert_eq!(ring_seconds(), 20, "the other edge is inside");
    }

    #[test]
    fn a_step_stops_at_either_end_of_the_range() {
        assert_eq!(stepped(6, 1), 7);
        assert_eq!(stepped(6, -1), 5);
        assert_eq!(stepped(2, -1), 2);
        assert_eq!(stepped(20, 1), 20);
        assert_eq!(stepped(0, 0), 2);
    }

    use gpui::{Entity, TestAppContext, VisualTestContext};

    use super::super::mock::MockFleet;
    use super::super::wire::Wire;

    fn opened(cx: &mut TestAppContext) -> (Entity<SettingsOverlay>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| SettingsOverlay::new(Wire::Mock(Box::new(MockFleet::new())), window, cx));
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.show(None, false, window, cx);
                this.pick("bridge:motion", cx);
            })
        });
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, _| window.refresh());
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        (overlay, cx)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| super::super::view::bounds_of(window, cx, role, name).is_some())
    }

    #[gpui::test]
    fn the_page_draws_a_stepper_a_reset_and_a_ring_at_the_speed_it_names(cx: &mut TestAppContext) {
        let (_overlay, cx) = opened(cx);
        assert!(drawn(cx, Role::Status, "Seconds per turn: 6"), "the figure reads six");
        assert!(drawn(cx, Role::Button, "Faster: one second less per turn"));
        assert!(drawn(cx, Role::Button, "Slower: one second more per turn"));
        assert!(drawn(cx, Role::Button, "Reset to default"));
        assert!(drawn(cx, Role::Label, "Ring preview at 6 seconds per turn"), "the ring turns beside the stepper");
    }

    #[gpui::test]
    fn the_arrows_step_the_figure_and_the_ring_follows_before_fleet_answers_and_reset_takes_it_back(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| {
            let more = overlay.read(cx).prefs.keys["motion:more"].clone();
            window.focus(&more, cx);
        });
        draw(cx);
        cx.simulate_keystrokes("right right");
        draw(cx);
        assert!(drawn(cx, Role::Status, "Seconds per turn: 8"));
        assert!(drawn(cx, Role::Label, "Ring preview at 8 seconds per turn"));
        cx.simulate_keystrokes("left");
        draw(cx);
        assert!(drawn(cx, Role::Status, "Seconds per turn: 7"));
        let shown = overlay.read_with(cx, |this, _| this.motion().ring_seconds);
        assert_eq!((shown, ring_seconds()), (7, 7), "Fleet's answer is held for every ring");
        cx.update(|window, cx| {
            let reset = overlay.read(cx).prefs.keys["motion:reset"].clone();
            window.focus(&reset, cx);
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(drawn(cx, Role::Status, "Seconds per turn: 6"));
        assert_eq!(ring_seconds(), 6);
    }

    #[gpui::test]
    fn the_stepper_stops_at_the_ends_of_the_range(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| {
            let less = overlay.read(cx).prefs.keys["motion:less"].clone();
            window.focus(&less, cx);
        });
        draw(cx);
        for _ in 0..8 {
            cx.simulate_keystrokes("left");
            draw(cx);
        }
        let now = overlay.read_with(cx, |this, _| this.motion().ring_seconds);
        assert_eq!((now, ring_seconds()), (2, 2), "six less eight steps stops at two");
    }
}
