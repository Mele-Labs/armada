//! A write clash, on both rows of the pair (`ClashMark/ClashMark.tsx`). **One registry mark, `swords`,
//! with its tooltip**, named for the other holder and the paths that conflict; a press opens that
//! holder. The holder is `main` when an agent's changes no longer fit a main that moved, and there is
//! nothing of main to open, so that mark is a picture and not a button. Fleet tells the agents; this
//! draws what it says: a pair that touches one file without a conflict is not in the list, so
//! nothing is drawn. No count is drawn beside any of it.
//!
//! The row asks for its own mark (`ClashMark::in_row`), so the list does not carry clashes through its
//! fields: the window holds what Fleet last said once ([`hold`]) and every row reads it. A paused Job
//! is drawn as any other, and a press opens it.

pub mod scenes;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, Anchor, App, Bounds, ElementId, FocusHandle, Global, Pixels, Role, SharedString,
    Subscription, Window,
};
use ipc::{AllWriteClashes, ClashHolder, ClashSide};

use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, glyph, small};

/// One clash as a holder sees it: who it is with, and where the changes collide.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub with: ClashHolder,
    pub paths: Vec<String>,
}

impl Conflict {
    /// What the mark's tooltip says.
    fn said(&self) -> String {
        format!("Conflicts with {} · {}", self.with.label, self.paths.join(", "))
    }
}

/// Every clash Fleet says stands, whole, as `write_clashes.changed` carries it.
#[derive(Default)]
struct Held(AllWriteClashes);

impl Global for Held {}

/// Hand the window what Fleet said clashes. A lost connection hands it nothing, so no mark stands.
pub fn hold(clashes: AllWriteClashes, cx: &mut App) {
    cx.set_global(Held(clashes));
    cx.refresh_windows();
}

/// What each Job is in, `clashingOf`: the clashes with that Job as either side, each as the other holder.
fn conflicts_of(clashes: &AllWriteClashes, id: &str) -> Vec<Conflict> {
    clashes
        .repositories
        .iter()
        .flat_map(|repository| repository.clashes.iter())
        .filter_map(|clash| {
            let (mine, other) = if clash.one.kind == ClashSide::Session && clash.one.id == id {
                (true, &clash.other)
            } else if clash.other.kind == ClashSide::Session && clash.other.id == id {
                (true, &clash.one)
            } else {
                (false, &clash.one)
            };
            mine.then(|| Conflict { with: other.clone(), paths: clash.paths.clone() })
        })
        .collect()
}

/// Opens the other holder of a clash, as a press on the Job's row opens it. Set by the screen that
/// owns the list; with none set, the press does nothing.
#[derive(Clone)]
pub struct OpenHolder(pub Rc<dyn Fn(&ClashHolder, &mut Window, &mut App)>);

impl Global for OpenHolder {}

/// Where the mark was drawn, whether the pointer is on it, and the keys' side of it: the mark is a tab
/// stop, and its card opens when the keys arrive and shuts when they leave or Esc is pressed.
struct Hovered {
    on: bool,
    at: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
    by_keys: bool,
    _subs: [Subscription; 2],
}

#[derive(IntoElement)]
pub struct ClashMark {
    id: SharedString,
}

impl ClashMark {
    /// The mark of the Job `id`, in the Conflicts column of its row: the column keeps its width where
    /// the Job has no clash, so the marks line up down the list.
    pub fn in_row(id: SharedString) -> ClashMark {
        ClashMark { id }
    }
}

fn column(look: &Look) -> gpui::Div {
    div().flex_none().w(look.length(token::SPACE_4)).flex().items_center().justify_center()
}

impl RenderOnce for ClashMark {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let conflicts = cx.try_global::<Held>().map(|held| conflicts_of(&held.0, &self.id)).unwrap_or_default();
        if conflicts.is_empty() {
            return column(cx.look()).into_any_element();
        }
        let hover = window.use_keyed_state((ElementId::from("clash-mark"), self.id.clone()), cx, |window, cx| {
            let focus = cx.focus_handle().tab_stop(true);
            let arrived = cx.on_focus_in(&focus, window, |held: &mut Hovered, window, cx| {
                if window.last_input_was_keyboard() && !held.by_keys {
                    held.by_keys = true;
                    cx.notify();
                }
            });
            let left = cx.on_focus_out(&focus, window, |held: &mut Hovered, _, _, cx| {
                if held.by_keys {
                    held.by_keys = false;
                    cx.notify();
                }
            });
            Hovered { on: false, at: Rc::default(), focus, by_keys: false, _subs: [arrived, left] }
        });
        let look = cx.look();
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let (on, at, focus, by_keys) = {
            let held = hover.read(cx);
            (held.on, Rc::clone(&held.at), held.focus.clone(), held.by_keys)
        };
        let swords = |ink| glyph("swords", look.length(token::SPACE_4), ink);
        let watch = |hover: gpui::Entity<Hovered>| {
            move |over: &bool, _: &mut Window, cx: &mut App| {
                let over = *over;
                hover.update(cx, |held, cx| {
                    if held.on != over {
                        held.on = over;
                        cx.notify();
                    }
                });
            }
        };
        let place = canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full();
        let several = conflicts.len() > 1;
        let said: SharedString = if several {
            format!("Conflicts with {}", conflicts.iter().map(|one| one.with.label.as_str()).collect::<Vec<_>>().join(", ")).into()
        } else {
            conflicts[0].said().into()
        };
        let body = div()
            .id(SharedString::from(format!("clash-{}", self.id)))
            .relative()
            .flex_none()
            .child(place)
            .on_hover(watch(hover.clone()));
        let opens = (!several && conflicts[0].with.kind == ClashSide::Session).then(|| conflicts[0].with.clone());
        let mark = match opens {
            Some(holder) => labelled(
                body.cursor_pointer()
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .track_focus(&focus)
                    .focus_visible(move |style| style.border_color(ring))
                    .on_key_down({
                        let hover = hover.clone();
                        move |event, _, cx| {
                            if event.keystroke.key == "escape" {
                                hover.update(cx, |held, cx| {
                                    if held.by_keys {
                                        held.by_keys = false;
                                        cx.notify();
                                    }
                                });
                                cx.stop_propagation();
                            }
                        }
                    })
                    .child(swords(if on || by_keys { look.color(token::FG_DEFAULT) } else { amber }))
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(move |_, window, cx| {
                        // The row it sits on opens itself on a press; this opens the other holder.
                        cx.stop_propagation();
                        if let Some(open) = cx.try_global::<OpenHolder>().cloned() {
                            (open.0)(&holder, window, cx);
                        }
                    }),
                Role::Button,
                said.clone(),
            ),
            None => labelled(body.child(swords(amber)), Role::Image, said.clone()),
        };
        let bounds = hover.read(cx).at.get();
        let tip = bounds.filter(|_| on || by_keys).map(|from| {
            let at = point(from.origin.x + from.size.width, from.origin.y + from.size.height + look.length(token::SPACE_1));
            let body = if several { many_card(&conflicts, look) } else { one_card(&said, look) };
            deferred(
                anchored()
                    .position(at)
                    .anchor(Anchor::TopRight)
                    .snap_to_window_with_margin(look.length(token::SPACE_1))
                    .child(labelled(div().id("clash-card").child(body), Role::Tooltip, said.clone())),
            )
            .with_priority(1)
        });
        column(look).child(mark).children(tip).into_any_element()
    }
}

fn card_of(look: &Look) -> gpui::Div {
    card(look)
        .max_w(look.length(token::W_TOOLTIP))
        .gap(look.length(token::SPACE_1))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
}

/// One clash: its tooltip, the sentence on a line.
fn one_card(said: &str, look: &Look) -> gpui::Div {
    card_of(look).child(said.to_string())
}

/// Several clashes on one mark: a line for each other holder, under one heading.
fn many_card(conflicts: &[Conflict], look: &Look) -> gpui::Div {
    card_of(look)
        .child(small(look).text_color(look.color(token::FG_MUTED)).child("Conflicts with"))
        .children(conflicts.iter().map(|one| div().child(format!("{} · {}", one.with.label, one.paths.join(", ")))))
}
