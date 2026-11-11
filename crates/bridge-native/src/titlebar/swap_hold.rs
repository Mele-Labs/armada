//! The window while a swap onto the snapshot is under way, and the line it leaves when the swap was undone
//! (`RS/swap-hold.tsx`), and the chime when the snapshot turns ready (`RS/fleet-build.ts`).
//!
//! **Fleet stops during a swap**, so the window would draw a lost connection for the minute it takes. From the
//! press until the report no longer says `swapping`, the window is dimmed and takes no presses and no keys but
//! ⌘ and ⌃ ones, with one banner and the seconds. Nothing is unmounted: the dim is a layer over the screen. The
//! report is marked `swapping` on the press and stays so while Fleet is down, so the hold needs no state about
//! the swap but when it began. A Fleet that swapped back leaves `failed` as a line starting `Swapped back`.

use std::time::{Duration, Instant};

use gpui::{anchored, deferred, div, prelude::*, px, Anchor, AnyElement, Context, FontWeight, Pixels, Role};
use ipc::SnapshotState;

use super::parts::{spinner, Btn, Weight};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look};

const SWAPPED_BACK: &str = "Swapped back";

impl TitleBar {
    /// Called after the face changed from outside the bar, as a report from Fleet or a scene does: starts and
    /// stops the hold, forgets a swap-back line put away that is gone, and chimes when the snapshot turned ready.
    pub fn faced(&mut self, cx: &mut Context<Self>) {
        let holding = self.face.build.as_ref().is_some_and(|held| held.report.swapping);
        match (holding, self.swap_since.is_some()) {
            (true, false) => {
                self.swap_since = Some(Instant::now());
                // The seconds on the banner count, so the bar draws once a second while the window is held.
                self._swap_tick = Some(cx.spawn(async move |this, cx| loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }));
                // A held window takes no keys either: a key with no ⌘ or ⌃ goes nowhere.
                self._swap_keys = Some(cx.intercept_keystrokes(|event, _, cx| {
                    let key = &event.keystroke.modifiers;
                    if !key.platform && !key.control {
                        cx.stop_propagation();
                    }
                }));
            }
            (false, true) => {
                self.swap_since = None;
                self._swap_tick = None;
                self._swap_keys = None;
            }
            _ => {}
        }
        if self.swapped_back().is_none() {
            self.swap_dismissed = None;
        }
        // Only the move counts: a report that already holds a ready snapshot when the window opens is not news.
        if let Some(held) = &self.face.build {
            let now = held.report.snapshot.as_ref().map(|snapshot| snapshot.state);
            if self.snapshot_was.is_some_and(|was| was != Some(SnapshotState::Ready)) && now == Some(SnapshotState::Ready) {
                cx.emit(Asked::Chime);
            }
            self.snapshot_was = Some(now);
        }
        cx.notify();
    }

    /// The line a swap back left, until it is put away or the next report carries none.
    fn swapped_back(&self) -> Option<String> {
        let held = self.face.build.as_ref()?;
        (!held.report.swapping).then_some(())?;
        held.report.failed.clone().filter(|line| line.starts_with(SWAPPED_BACK))
    }

    /// The dim and the banner. Nothing when the window is neither held nor carrying a swap back.
    pub(super) fn swap_layer(&self, look: &Look, room: gpui::Bounds<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let viewport = room.size;
        let holding = self.face.build.as_ref().is_some_and(|held| held.report.swapping);
        let line = self.swapped_back().filter(|line| self.swap_dismissed.as_ref() != Some(line));
        if !holding && line.is_none() {
            return None;
        }
        let width = look.length(token::PALETTE_WIDTH).min(viewport.width);
        // The window's top is the title row's, which is where the window starts when something stands above it.
        let top = self.at.row.get().map_or(px(0.), |row| row.origin.y);
        let banner = if holding {
            let seconds = self.swap_since.map_or(0, |since| since.elapsed().as_secs());
            banner(
                look,
                "swap-hold-banner",
                "Swapping to the build",
                format!("{seconds} s"),
                look.color(token::BORDER_DEFAULT),
                Some(spinner("swap-hold-spin", look.length(token::SPACE_4), look.color(token::FG_MUTED), cx).into_any_element()),
                None,
                Role::Status,
            )
        } else {
            let said = line.clone().unwrap_or_default();
            let rest = said[SWAPPED_BACK.len()..].trim_start_matches(|c: char| c.is_whitespace() || ":,.-".contains(c)).to_string();
            let dismissed = said.clone();
            banner(
                look,
                "swap-hold-swapped-back",
                SWAPPED_BACK,
                rest,
                look.color(token::STATUS_AWAITING_REVIEW),
                None,
                Some(
                    labelled(
                        Btn::new("swap-hold-dismiss", "Dismiss", Weight::Ghost).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| {
                            this.swap_dismissed = Some(dismissed.clone());
                            cx.notify();
                        })),
                        Role::Button,
                        "Dismiss",
                    )
                    .into_any_element(),
                ),
                Role::Alert,
            )
        };
        let dim = holding.then(|| {
            div().id("swap-hold").absolute().top_0().left_0().w(viewport.width).h(viewport.height).bg(look.color(token::SCRIM)).occlude()
        });
        Some(
            deferred(
                anchored()
                    .position(room.origin)
                    .anchor(Anchor::TopLeft)
                    .child(div().relative().w(viewport.width).h(viewport.height).children(dim).child(
                        div().absolute().top(top + look.length(token::SPACE_6)).left((viewport.width - width) / 2.).w(width).child(banner),
                    )),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }
}

/// An Alert on the overlay ground: a frame in its hue, a title, a body, an icon before and an act after.
#[allow(clippy::too_many_arguments)]
fn banner(
    look: &Look,
    id: &'static str,
    title: &'static str,
    body: String,
    edge: gpui::Hsla,
    lead: Option<AnyElement>,
    action: Option<AnyElement>,
    role: Role,
) -> AnyElement {
    labelled(
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .children(lead)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(body),
                    ),
            )
            .children(action),
        role,
        title,
    )
    .into_any_element()
}
