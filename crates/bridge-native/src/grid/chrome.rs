//! What a block of the grid is drawn in, for any screen that keeps one: the cells a box is cut into, the card and its header band,
//! the view switch, Hide and the corner that resizes, and what a drag or an arrow does to the layout. A block's own rows are the
//! screen's; none of this knows what a block shows.

use gpui::{div, prelude::*, px, AnyElement, App, ClickEvent, Context, FontWeight, Hsla, MouseButton, MouseDownEvent, Pixels, Point, Render, Role, ScrollHandle, SharedString, Stateful, Toggled, Window};

use super::layout::{GridKind, Layout, Placed, View, COLUMNS};
use crate::labels::{labelled, Labelled};
use crate::look::{token, Look, Token};
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{glyph, tip};

/// A press or a click on a part of a block, as gpui hands it to a listener.
pub type Pressed = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// The room a block is drawn in: a box cut into six columns of rows of one height, with a gap between.
#[derive(Clone, Copy)]
pub struct Cells {
    pub col: Pixels,
    pub row: Pixels,
    pub gap: Pixels,
}

impl Cells {
    /// The cells of a box `width` wide, in rows `row` tall.
    pub fn across(width: Pixels, gap: Pixels, row: Pixels) -> Cells {
        let columns = f32::from(COLUMNS);
        Cells { col: (width - gap * (columns - 1.0)) / columns, row, gap }
    }

    pub fn left(&self, x: u8) -> Pixels {
        (self.col + self.gap) * f32::from(x)
    }

    pub fn top(&self, y: u8) -> Pixels {
        (self.row + self.gap) * f32::from(y)
    }

    pub fn wide(&self, w: u8) -> Pixels {
        self.col * f32::from(w) + self.gap * f32::from(w.saturating_sub(1))
    }

    pub fn tall(&self, h: u8) -> Pixels {
        self.row * f32::from(h) + self.gap * f32::from(h.saturating_sub(1))
    }

    /// The distance from one cell to the next, across and down: what a pointer offset is divided by.
    pub fn pitch(&self) -> (Pixels, Pixels) {
        (self.col + self.gap, self.row + self.gap)
    }

    /// The height of a grid `rows` tall.
    pub fn height(&self, rows: u8) -> Pixels {
        let rows = f32::from(rows);
        (self.row * rows + self.gap * (rows - 1.0).max(0.0)).max(px(0.))
    }

    /// The element laid over the cells `placed` takes.
    pub fn put<E: Styled, K>(&self, element: E, placed: &Placed<K>) -> E {
        element.absolute().left(self.left(placed.x)).top(self.top(placed.y)).w(self.wide(placed.w)).h(self.tall(placed.h))
    }
}

// ---- keys --------------------------------------------------------------------------------------

/// The step an arrow key takes, from the slot the registry's act carries: left, right, up, down.
pub fn arrow(slot: u8) -> (i16, i16) {
    match slot % 4 {
        0 => (-1, 0),
        1 => (1, 0),
        2 => (0, -1),
        _ => (0, 1),
    }
}

// ---- a drag ------------------------------------------------------------------------------------

/// A block being moved or resized, as the drag carries it.
#[derive(Clone)]
pub struct BlockDrag<K> {
    pub kind: K,
    pub resize: bool,
}

/// What follows the pointer while a block is dragged: nothing, since the block itself moves with it.
pub struct Ghost;

impl Render for Ghost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// What a drag asks of the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Move { x: i16, y: i16 },
    Resize { w: i16, h: i16 },
}

impl Step {
    pub fn apply<K: GridKind>(self, layout: &mut Layout<K>, kind: K) {
        match self {
            Step::Move { x, y } => layout.move_to(kind, x, y),
            Step::Resize { w, h } => layout.resize_to(kind, w, h),
        }
    }
}

/// The pointer, `at` from the grid's corner, has the block `now` follow to the cell under it, or take the size that reaches it.
/// A move keeps the offset of the cell it was lifted by in `grab`, which the caller clears when a press starts.
pub fn drag_step<K>(now: &Placed<K>, resize: bool, at: Point<Pixels>, pitch: (Pixels, Pixels), grab: &mut Option<(i16, i16)>) -> Step {
    let cell = ((at.x / pitch.0).floor() as i16, (at.y / pitch.1).floor() as i16);
    if resize {
        Step::Resize { w: cell.0 - i16::from(now.x) + 1, h: cell.1 - i16::from(now.y) + 1 }
    } else {
        let grab = *grab.get_or_insert((cell.0 - i16::from(now.x), cell.1 - i16::from(now.y)));
        Step::Move { x: cell.0 - grab.0, y: cell.1 - grab.1 }
    }
}

/// A header that lifts its block when it is pressed and dragged. `on_down` is the press, where the caller clears its `grab`.
pub fn lift<K: GridKind, E: Styled + StatefulInteractiveElement>(head: E, kind: K, on_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> E {
    head.cursor_grab().on_mouse_down(MouseButton::Left, on_down).on_drag(BlockDrag { kind, resize: false }, |_, _, _, cx| cx.new(|_| Ghost))
}

/// The corner of a block that resizes it when it is dragged.
pub fn corner<K: GridKind>(slug: &str, title: &str, kind: K, look: &Look, on_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Labelled {
    labelled(
        div()
            .id(SharedString::from(format!("{slug}-corner")))
            .absolute()
            .right_0()
            .bottom_0()
            .size(look.length(token::SPACE_4))
            .rounded_tl(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_HOVER))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_STRONG))
            .cursor(gpui::CursorStyle::ResizeUpLeftDownRight)
            .on_mouse_down(MouseButton::Left, on_down)
            .on_drag(BlockDrag { kind, resize: true }, |_, _, _, cx| cx.new(|_| Ghost)),
        Role::Splitter,
        format!("Resize {}", title),
    )
}

// ---- a block -----------------------------------------------------------------------------------

/// The card a block is drawn in, not yet placed: `Cells::put` places it, or `size_full` lets it cover its parent.
/// `border` is its edge, which the screen says since it is the screen that knows a block has failed.
pub fn frame(slug: &str, look: &Look, border: Hsla) -> Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("block-{slug}")))
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(border)
        .bg(look.color(token::BG_RAISED))
        .shadow(look.shadows(token::SHADOW_CARD))
        .text_color(look.color(token::FG_DEFAULT))
        .focus_visible({
            let ring = ring_colour(look);
            move |style| style.border_color(ring)
        })
}

/// The tinted band across the top of a block, in the hue of its kind. Its glyph, title and controls are children.
pub fn head(slug: &str, hue: Token, look: &Look) -> Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("{slug}-head")))
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_ROW_ACTION))
        .px(look.length(token::SPACE_3))
        .bg(look.color(hue).opacity(0.18))
}

/// The words on a block's header.
pub fn title_text(title: &str, look: &Look) -> gpui::Div {
    div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title.to_string()))
}

/// The header's mark, in the hue of the block's kind.
pub fn mark(name: &str, hue: Token, look: &Look) -> gpui::Svg {
    glyph(name, look.length(token::SPACE_4), look.color(hue))
}

/// The buttons that put a block in another view, one for each the block can draw. `pick` makes the press of one.
pub fn view_switch(slug: &str, title: &str, views: &[View], shown: View, look: &Look, pick: impl Fn(View) -> Pressed) -> Labelled {
    let ring = ring_colour(look);
    let mut group = div().id(SharedString::from(format!("{slug}-views"))).flex().items_center().gap(look.length(token::SPACE_1));
    for view in views {
        let (view, on) = (*view, *view == shown);
        let button = div()
            .id(SharedString::from(format!("{slug}-view-{}", view.word())))
            .flex_none()
            .h(look.length(token::H_BADGE))
            .px(look.length(token::SPACE_2))
            .flex()
            .items_center()
            .rounded(look.length(token::RADIUS_SM))
            .text_size(look.length(token::TEXT_2XS))
            .when(on, |button| button.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
            .when(!on, |button| button.text_color(look.color(token::FG_SUBTLE)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .tooltip(tip(view.said()))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .on_click(pick(view))
            .child(view.word());
        group = group.child(labelled(button, Role::Button, format!("{} view of {}", view.word(), title)));
    }
    labelled(group, Role::Group, format!("View of {}", title))
}

/// Hide, in the header of a block while the grid is edited.
pub fn hide_button(slug: &str, title: &str, look: &Look, on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Labelled {
    let side = look.length(token::H_BADGE);
    let ring = ring_colour(look);
    labelled(
        div()
            .id(SharedString::from(format!("{slug}-hide")))
            .flex_none()
            .size(side)
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip(format!("Hide {} from the board", title)))
            // A press on it is not the start of a drag of the block.
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .on_click(on_click)
            .child(glyph("eye-off", look.length(token::SPACE_4), look.color(token::FG_MUTED))),
        Role::Button,
        format!("Hide {}", title),
    )
}

/// A block's body, scrolled on its own. **While it has rows out of sight it keeps the wheel**, so the grid behind it
/// stands still; a body with nothing to scroll lets the wheel through.
pub fn scrolled(id: SharedString, handle: &ScrollHandle) -> Stateful<gpui::Div> {
    let held = handle.clone();
    div().id(id).track_scroll(handle).overflow_y_scroll().on_scroll_wheel(move |_, _, cx| {
        if held.max_offset().y > px(0.) {
            cx.stop_propagation();
        }
    })
}

/// A block's place while the grid's layout is not known yet: the card, with bones where its rows will be.
pub fn skeleton<K: GridKind>(placed: &Placed<K>, cells: Cells, look: &Look) -> AnyElement {
    let bones = (0..3).map(|at| bone(look, look.length(token::SPACE_12) * (3.0 - at as f32 * 0.5), look.length(token::SPACE_3)));
    cells
        .put(div().id(SharedString::from(format!("block-loading-{}", placed.kind.slug()))), placed)
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .shadow(look.shadows(token::SHADOW_CARD))
        .children(bones)
        .into_any_element()
}
