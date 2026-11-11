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
    anchored, canvas, deferred, div, point, prelude::*, Anchor, App, Bounds, ElementId, FocusHandle, FontWeight, Global, Pixels, Role,
    SharedString, Subscription, Window,
};
use ipc::{AllWriteClashes, ClashHolder, ClashSide};

use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, glyph, mono, small};

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
    if cx.try_global::<Held>().is_some_and(|held| held.0 == clashes) {
        return;
    }
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

/// Opens a file a clash names, as the Job's files panel does. Set by the screen that owns the list; with
/// none set, the paths are drawn as chips and nothing is pressable.
#[derive(Clone)]
pub struct OpenPath(pub Rc<dyn Fn(&str, &mut Window, &mut App)>);

impl Global for OpenPath {}

/// Where the mark was drawn, whether the pointer is on it, and the keys' side of it: the mark is a tab
/// stop, and its card opens when the keys arrive and shuts when they leave or Esc is pressed. The card
/// holds pressable paths, so it stays open while the pointer or the keys are in it.
struct Hovered {
    on: bool,
    at: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
    by_keys: bool,
    /// The card's own side: the pointer on it, a key in it, and Esc having put it away.
    card: FocusHandle,
    /// The first path of the card, where Tab goes from the mark.
    first: FocusHandle,
    over_card: bool,
    in_card: bool,
    quiet: bool,
    _subs: [Subscription; 4],
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
            let card = cx.focus_handle();
            let first = cx.focus_handle().tab_stop(true);
            let arrived = cx.on_focus_in(&focus, window, |held: &mut Hovered, window, cx| {
                if window.last_input_was_keyboard() && !held.by_keys && !held.quiet {
                    held.by_keys = true;
                    cx.notify();
                }
            });
            let left = cx.on_focus_out(&focus, window, |held: &mut Hovered, _, _, cx| {
                held.quiet = false;
                if held.by_keys {
                    held.by_keys = false;
                    cx.notify();
                }
            });
            let entered = cx.on_focus_in(&card, window, |held: &mut Hovered, _, cx| {
                held.in_card = true;
                cx.notify();
            });
            let exited = cx.on_focus_out(&card, window, |held: &mut Hovered, _, _, cx| {
                held.in_card = false;
                cx.notify();
            });
            Hovered {
                on: false,
                at: Rc::default(),
                focus,
                by_keys: false,
                card,
                first,
                over_card: false,
                in_card: false,
                quiet: false,
                _subs: [arrived, left, entered, exited],
            }
        });
        let look = cx.look();
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let (on, at, focus, by_keys, card_focus, first, in_card) = {
            let held = hover.read(cx);
            (held.on, Rc::clone(&held.at), held.focus.clone(), held.by_keys, held.card.clone(), held.first.clone(), held.over_card || held.in_card)
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
                        let first = first.clone();
                        move |event, window, cx| {
                            if event.keystroke.key == "escape" {
                                hover.update(cx, |held, cx| {
                                    if held.by_keys {
                                        held.by_keys = false;
                                        cx.notify();
                                    }
                                });
                                cx.stop_propagation();
                            } else if event.keystroke.key == "tab" && !event.keystroke.modifiers.shift && hover.read(cx).by_keys && cx.has_global::<OpenPath>() {
                                // The card's paths come next, though the card is drawn after the rows below it.
                                window.focus(&first, cx);
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
        let tip = bounds.filter(|_| on || by_keys || in_card).map(|from| {
            // The card starts at the mark's foot, with its gap inside its own hit area, so the pointer
            // crosses from the mark to the card without leaving either.
            let at = point(from.origin.x + from.size.width, from.origin.y + from.size.height);
            let body = if several { many_card(&conflicts, &self.id, look) } else { one_card(&conflicts[0], &self.id, &first, look) };
            let hit = div()
                .id("clash-card")
                .occlude()
                .pt(look.length(token::SPACE_1))
                .track_focus(&card_focus)
                .tab_group()
                .on_hover({
                    let hover = hover.clone();
                    move |over: &bool, _: &mut Window, cx: &mut App| {
                        let over = *over;
                        hover.update(cx, |held, cx| {
                            if held.over_card != over {
                                held.over_card = over;
                                cx.notify();
                            }
                        });
                    }
                })
                .on_key_down({
                    let hover = hover.clone();
                    let focus = focus.clone();
                    let first = first.clone();
                    move |event, window, cx| {
                        if event.keystroke.key == "tab" && event.keystroke.modifiers.shift && first.is_focused(window) {
                            window.focus(&focus, cx);
                            cx.stop_propagation();
                        } else if event.keystroke.key == "escape" {
                            hover.update(cx, |held, cx| {
                                held.quiet = true;
                                held.by_keys = false;
                                held.in_card = false;
                                held.over_card = false;
                                held.on = false;
                                cx.notify();
                            });
                            window.focus(&focus, cx);
                            cx.stop_propagation();
                        }
                    }
                })
                .child(body);
            deferred(
                anchored()
                    .position(at)
                    .anchor(Anchor::TopRight)
                    .snap_to_window_with_margin(look.length(token::SPACE_1))
                    .child(labelled(hit, Role::Tooltip, said.clone())),
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

/// One clash: who it is with, and the files that collide.
fn one_card(one: &Conflict, key: &str, first: &FocusHandle, look: &Look) -> gpui::Div {
    card_of(look).child(format!("Conflicts with {}", one.with.label)).child(ClashPaths {
        id: format!("{key}:{}", one.with.id).into(),
        label: one.with.label.clone().into(),
        paths: one.paths.clone(),
        first: Some(first.clone()),
    })
}

/// Several clashes on one mark: a heading, then each other holder with its files.
fn many_card(conflicts: &[Conflict], key: &str, look: &Look) -> gpui::Div {
    card_of(look).child(small(look).text_color(look.color(token::FG_MUTED)).child("Conflicts with")).children(conflicts.iter().map(|one| {
        div().flex().flex_col().gap(look.length(token::SPACE_1)).pb(look.length(token::SPACE_1)).child(one.with.label.clone()).child(
            div().pl(look.length(token::SPACE_3)).child(ClashPaths {
                id: format!("{key}:{}", one.with.id).into(),
                label: one.with.label.clone().into(),
                paths: one.paths.clone(),
                first: None,
            }),
        )
    }))
}

/// Paths drawn while a long list is folded. A list of one more than this is drawn whole: "1 more" saves nothing.
const SHOWN: usize = 3;

/// The paths a clash names (`ClashPaths`): a long list folded to its first few and one chip to open the rest.
/// With [`OpenPath`] set, each path is a press, named `Open <path>`. The fold reads `<n> more` or `Fewer`.
#[derive(IntoElement)]
struct ClashPaths {
    id: SharedString,
    label: SharedString,
    paths: Vec<String>,
    /// The handle of the first path, where the keys come into the list.
    first: Option<FocusHandle>,
}

/// A path as a chip: the directory receding, the basename kept whole. The directory is cut from its left
/// edge to what fits, so the part nearest the file is the part that stays.
fn path_chip(path: &str, look: &Look) -> gpui::Div {
    let cut = path.rfind('/').map_or(0, |at| at + 1);
    let (directory, basename) = path.split_at(cut);
    let room = (f32::from(look.length(token::W_TOOLTIP)) - 6. * f32::from(look.length(token::SPACE_3))) / (0.6 * f32::from(look.length(token::TEXT_XS)));
    let keep = (room as usize).saturating_sub(basename.chars().count()).max(1);
    let directory = if directory.chars().count() > keep {
        let tail: String = directory.chars().skip(directory.chars().count() - (keep - 1)).collect();
        format!("\u{2026}{tail}")
    } else {
        directory.to_string()
    };
    div()
        .flex()
        .min_w_0()
        .font_family(mono(look))
        .whitespace_nowrap()
        .child(div().text_color(look.color(token::FG_SUBTLE)).child(directory))
        .child(div().flex_none().text_color(look.color(token::FG_MUTED)).child(basename.to_string()))
}

impl RenderOnce for ClashPaths {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let all = window.use_keyed_state((ElementId::from("clash-paths"), self.id.clone()), cx, |_, _| false);
        let look = cx.look();
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let open = cx.try_global::<OpenPath>().cloned();
        let folds = self.paths.len() > SHOWN + 1;
        let whole = *all.read(cx);
        let drawn = if folds && !whole { &self.paths[..SHOWN] } else { &self.paths[..] };
        let rows = drawn.iter().enumerate().map(|(at, path)| {
            let chip = path_chip(path, look);
            match &open {
                None => chip.into_any_element(),
                Some(open) => {
                    let open = open.clone();
                    let path = path.clone();
                    let said = format!("Open {path}");
                    let row = div().id(SharedString::from(format!("clash-path-{}-{at}", self.id)));
                    let row = match (&self.first, at) {
                        (Some(first), 0) => row.track_focus(first),
                        _ => row.focusable().tab_stop(true),
                    };
                    labelled(
                        row
                            .min_w_0()
                            .rounded(look.length(token::RADIUS_SM))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(gpui::transparent_black())
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(move |style| style.border_color(ring))
                            .child(chip)
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                (open.0)(&path, window, cx);
                            }),
                        Role::Button,
                        said,
                    )
                    .into_any_element()
                }
            }
        });
        let fold = folds.then(|| {
            let said: SharedString = if whole { "Fewer".into() } else { format!("{} more", self.paths.len() - SHOWN).into() };
            labelled(
                div()
                    .id(SharedString::from(format!("clash-fold-{}", self.id)))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .font_family(mono(look))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .child(said.clone())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        all.update(cx, |whole, cx| {
                            *whole = !*whole;
                            cx.notify();
                        });
                    }),
                Role::Button,
                said,
            )
        });
        labelled(
            div().id(SharedString::from(format!("clash-files-{}", self.id))).flex().flex_col().items_start().min_w_0().gap(look.length(token::SPACE_1)).children(rows).children(fold),
            Role::List,
            format!("Conflicting files with {}", self.label),
        )
    }
}
