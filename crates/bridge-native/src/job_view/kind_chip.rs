//! The kind chip under each of his messages in a Job's thread: how Fleet sorted it (a new ask of some kind, a revision, an
//! answer or talk), changed with one press. A message with no sorting draws nothing. One press opens a menu of the seven
//! kinds, the current one marked; Revision and Answer go on to the Job's open asks, since each joins one. A press sends
//! `set_message_kind`, and Fleet's refusal is said in a toast in its own words.
//!
//! The thread draws from `&self`, so what was sorted is held by `store`, read off the main thread when the Job comes on
//! screen. `mock` answers where there is no Fleet. `model` works out every label, menu and request without a window.
//!
//! **Keys.** The chip is a tab stop (Tab reaches it from the thread), and Enter or Space on it opens its menu, which
//! takes the arrows, Enter and Esc as every menu does. `message_kind` (`⌥K`, scope `thread`) opens the menu of the chip that
//! holds focus, else of his newest sorted message.

use gpui::{canvas, div, prelude::*, AnyElement, App, ClickEvent, Context, FocusHandle, FontWeight, Pixels, Point, Role, SharedString, Window};

use super::rows::tip;
use super::{JobView, Page};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, Look};
use model::{Choice, Hue};

pub(crate) mod mock;
pub(crate) mod model;
pub(crate) mod store;
#[allow(dead_code)]
pub(crate) mod shots;

#[cfg(test)]
mod tests;

/// Hear which Job comes on screen, and the key that opens a chip's menu.
pub(crate) fn init(cx: &mut App) {
    super::on_job_shown(
        |job, _, cx| {
            if let Some(job) = job {
                store::shown(&job, cx);
            }
        },
        cx,
    );
    cx.on_action(|act: &Act, cx: &mut App| {
        if !act.is("message_kind") {
            return cx.propagate();
        }
        let (Some(view), Some(window)) = (JobView::live(cx), cx.active_window()) else { return cx.propagate() };
        // The key is being dispatched inside this window's own update, so the menu opens just after.
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| view.update(cx, |view, cx| view.kind_key(window, cx)));
        });
    });
}

/// The row id a chip's keys and bounds are kept under.
fn spot_key(row: &str) -> String {
    format!("kind:{row}")
}

/// The colours of a hue: its words and the fill under them, as the mock's `.k-*` classes.
fn paint(look: &Look, hue: Hue) -> (gpui::Hsla, gpui::Hsla) {
    match hue {
        Hue::Bug => (look.color(token::ERROR), look.color(token::ERROR).opacity(0.16)),
        Hue::Change => (look.color(token::ACCENT), look.color(token::ACCENT).opacity(0.16)),
        Hue::Idea => (look.color(token::STATUS_AWAITING_REVIEW), look.color(token::STATUS_AWAITING_REVIEW_BG)),
        Hue::Neutral => (look.color(token::FG_SUBTLE), look.color(token::BG_RAISED)),
    }
}

impl JobView {
    /// The chip under his message `id`, or none where the message is not sorted, so an empty slot stays empty.
    pub(super) fn kind_chip(&self, page: &Page, id: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let sorting = store::sorting(&job, id, cx)?;
        let label = model::label(&sorting);
        let (ink, fill) = paint(look, model::hue(&sorting));
        let spot = self.spot(page, &spot_key(id), cx);
        let row = id.to_string();
        let focus = spot.focus.clone();
        let edges = spot.bounds.clone();
        let drawn = spot.bounds.clone();
        let chip = div()
            .id(SharedString::from(spot_key(id)))
            .track_focus(&spot.focus)
            .self_start()
            .ml(super::bleed(look))
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_BADGE))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .bg(fill)
            .text_color(ink)
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .whitespace_nowrap()
            .cursor_pointer()
            .focus_visible(|style| style.border_color(look.color(token::FOCUS_RING)))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                let at = edges.get().bottom_left();
                this.open_kind_menu(row.clone(), at, Some(focus.clone()), window, cx);
            }))
            .tooltip(tip(model::sorted_by(&sorting)))
            .child(label)
            .child(canvas(move |bounds, _, _| drawn.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full());
        Some(labelled(chip, Role::Button, format!("Message kind: {label}")).into_any_element())
    }

    /// The first menu, at `at`, for his message `row`. A pick sends the kind, or goes on to the asks.
    fn open_kind_menu(&mut self, row: String, at: Point<Pixels>, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(sorting) = store::sorting(&job, &row, cx) else { return };
        let entries = model::kind_entries(&sorting, &store::asks(&job, cx));
        let again = back.clone();
        self.open_button_menu(
            "Message kind",
            entries,
            at,
            back,
            move |this, id, window, cx| this.kind_picked(&job, &row, id, at, again.clone(), window, cx),
            window,
            cx,
        );
    }

    /// A line of the first menu: Revision and Answer open the second, the rest are sent.
    #[allow(clippy::too_many_arguments)]
    fn kind_picked(&mut self, job: &str, row: &str, id: &str, at: Point<Pixels>, back: Option<FocusHandle>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(choice) = Choice::of(id) else { return };
        if !choice.joins() {
            if let Some(set) = model::request(job, row, choice, None) {
                store::send(job, set, self.fleet.clone(), cx);
            }
            return;
        }
        let Some(sorting) = store::sorting(job, row, cx) else { return };
        let entries = model::ask_entries(&sorting, &store::asks(job, cx));
        let (job, row) = (job.to_string(), row.to_string());
        self.open_button_menu(
            if choice == Choice::Revision { "Revision of" } else { "Answer to" },
            entries,
            at,
            back,
            move |this, id, _, cx| {
                let asks = store::asks(&job, cx);
                let Some(ask) = model::ask_picked(&asks, id) else { return };
                if let Some(set) = model::request(&job, &row, choice, Some(&ask.ask_id)) {
                    store::send(&job, set, this.fleet.clone(), cx);
                }
            },
            window,
            cx,
        );
    }

    /// `message_kind`: the menu of the chip that holds focus, else of his newest sorted message that was drawn.
    fn kind_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(page) = self.pages.get(&job) else { return };
        let rows: Vec<String> = store::sortings(&job, cx).into_iter().map(|one| one.row_id).collect();
        let held = |row: &String| page.keys.spot_of(&spot_key(row));
        let target = rows.iter().rev().find_map(|row| held(row).filter(|spot| spot.focus.is_focused(window)).map(|spot| (row, spot))).or_else(|| {
            rows.iter().rev().find_map(|row| held(row).filter(|spot| spot.bounds.get().size.width > gpui::px(0.)).map(|spot| (row, spot)))
        });
        let Some((row, spot)) = target else { return };
        let (row, at, back) = (row.clone(), spot.bounds.get().bottom_left(), Some(spot.focus.clone()));
        self.open_kind_menu(row, at, back, window, cx);
    }
}
