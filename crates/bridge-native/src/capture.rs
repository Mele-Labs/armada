//! Capture: the screen under a crosshair, as Electron's walk capture (`StudioCapture`) and its dev
//! annotation layer draw it. **One capture, two homes**: the walk window arms it over a walk's
//! screen, and the annotation layer (`annotate.rs`) arms it over the app. The element under the
//! pointer is outlined with its name; a press holds it, and a drag holds the box drawn instead, for
//! a spot that is not one element. The note is written in a card against what was held. Each goes
//! with a picture of the whole window, its area outlined and numbered by its place among the notes
//! sent, as Electron burns them (`burn.ts`).
//!
//! [`Capture`] is the entity both mount over their screen. It reads the labelled elements the
//! window drew (`labels.rs`), holds the pointer, takes the picture and writes the note, and tells its
//! home through [`CaptureEvent`] when a note is written or the person puts it away. What a note is
//! then, a held note for Send notes or a pin for a Job, is the home's.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    anchored, deferred, div, Anchor, point, prelude::*, Bounds, Context, CursorStyle, Entity, EventEmitter, ExternalPaths, FocusHandle,
    Focusable, MouseButton, Pixels, Point, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

use crate::annotate::tools::{self, Tools};
use crate::labels::{labelled, Drawn, LabelBook};
use crate::look::{token, ActiveLook};
use crate::walk::bar::{button, Weight};

/// Below this, in each direction, a press that moved is still a press and not a box.
pub const SLOP: f32 = 4.;

/// What a note is written on.
#[derive(Clone, Debug)]
pub enum Held {
    Element(Drawn),
    Region(Bounds<Pixels>),
    /// The whole window, with the screen it was on: no element, so no outline and no pin.
    Window { bounds: Bounds<Pixels>, screen: Option<SharedString> },
}

impl Held {
    pub fn bounds(&self) -> Bounds<Pixels> {
        match self {
            Held::Element(drawn) => drawn.bounds,
            Held::Region(bounds) => *bounds,
            Held::Window { bounds, .. } => *bounds,
        }
    }

    /// The words the card heads with, and the note's place carries: the element's role and name,
    /// or that it is a box.
    pub fn said(&self) -> SharedString {
        match self {
            Held::Element(drawn) => match role_word(drawn.role) {
                Some(role) => format!("{role} · {}", drawn.name).into(),
                None => drawn.name.clone(),
            },
            Held::Region(_) => "A box you drew".into(),
            Held::Window { screen, .. } => format!("Whole window · {}", screen.as_deref().unwrap_or("no screen marked")).into(),
        }
    }
}

/// A role in the words a walk step uses for it.
pub fn role_word(role: Role) -> Option<&'static str> {
    Some(match role {
        Role::Button => "Button",
        Role::Row => "Row",
        Role::List => "List",
        Role::ListItem => "List item",
        Role::Heading => "Heading",
        Role::TextInput | Role::SearchInput => "Field",
        Role::Toolbar => "Toolbar",
        Role::Status => "Status",
        Role::Link => "Link",
        Role::Menu => "Menu",
        Role::MenuItem | Role::MenuItemCheckBox => "Menu item",
        Role::Image => "Mark",
        Role::Dialog => "Dialog",
        Role::ListBox => "List",
        Role::ListBoxOption => "Option",
        Role::RadioGroup => "Choices",
        Role::RadioButton => "Choice",
        Role::Tab => "Tab",
        Role::TabList => "Tabs",
        Role::Group => "Group",
        _ => return None,
    })
}

/// The labelled element of `drawn` under `at` and inside `within`: the smallest that holds the
/// point, so a row wins over its list.
pub fn element_at(drawn: &[Drawn], at: Point<Pixels>, within: Bounds<Pixels>) -> Option<Drawn> {
    drawn
        .iter()
        .filter(|one| one.bounds.contains(&at) && within.contains(&one.bounds.center()))
        .min_by(|a, b| area(a.bounds).total_cmp(&area(b.bounds)))
        .cloned()
}

fn area(bounds: Bounds<Pixels>) -> f32 {
    f32::from(bounds.size.width) * f32::from(bounds.size.height)
}

/// The box from `from` to `to`, whichever way it was dragged, or `None` for a press that barely moved.
pub fn region(from: Point<Pixels>, to: Point<Pixels>) -> Option<Bounds<Pixels>> {
    let (x, y) = ((f32::from(to.x) - f32::from(from.x)).abs(), (f32::from(to.y) - f32::from(from.y)).abs());
    if x < SLOP && y < SLOP {
        return None;
    }
    let origin = Point { x: from.x.min(to.x), y: from.y.min(to.y) };
    Some(Bounds { origin, size: gpui::size(gpui::px(x), gpui::px(y)) })
}

/// Digits three wide and five tall, a row to a `u8`, for the number burned beside a ring.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

/// The window's picture with `ring` outlined in `ink` and `number` in a pill at its top left, as a
/// PNG in base64. `scale` is the window's pixels per point.
pub fn ringed(picture: &image::RgbaImage, ring: Bounds<Pixels>, scale: f32, number: usize, ink: [u8; 4], on_ink: [u8; 4]) -> Option<String> {
    let mut out = picture.clone();
    let (width, height) = (out.width() as i64, out.height() as i64);
    let mut fill = |x0: i64, y0: i64, x1: i64, y1: i64, color: [u8; 4]| {
        for y in y0.max(0)..y1.min(height) {
            for x in x0.max(0)..x1.min(width) {
                out.put_pixel(x as u32, y as u32, image::Rgba(color));
            }
        }
    };
    let at = |value: Pixels| (f32::from(value) * scale).round() as i64;
    let (x0, y0) = (at(ring.origin.x), at(ring.origin.y));
    let (x1, y1) = (x0 + at(ring.size.width), y0 + at(ring.size.height));
    let edge = (2. * scale).round().max(1.) as i64;
    fill(x0, y0, x1, y0 + edge, ink);
    fill(x0, y1 - edge, x1, y1, ink);
    fill(x0, y0, x0 + edge, y1, ink);
    fill(x1 - edge, y0, x1, y1, ink);
    // The pill: each digit cell `dot` wide, padded by two cells, above the ring where it fits.
    let digits: Vec<usize> = number.to_string().bytes().map(|digit| (digit - b'0') as usize).collect();
    let dot = (3. * scale).round().max(1.) as i64;
    let (pill_w, pill_h) = (dot * (4 * digits.len() as i64 + 3), dot * 9);
    let top = if y0 - pill_h >= 0 { y0 - pill_h } else { y0 };
    fill(x0, top, x0 + pill_w, top + pill_h, ink);
    for (place, digit) in digits.iter().enumerate() {
        for (row, bits) in DIGITS[*digit].iter().enumerate() {
            for column in 0..3 {
                if bits & (0b100 >> column) != 0 {
                    let x = x0 + dot * (2 + 4 * place as i64 + column);
                    let y = top + dot * (2 + row as i64);
                    fill(x, y, x + dot, y + dot, on_ink);
                }
            }
        }
    }
    let mut png = std::io::Cursor::new(Vec::new());
    out.write_to(&mut png, image::ImageFormat::Png).ok()?;
    use base64::Engine as _;
    Some(base64::engine::general_purpose::STANDARD.encode(png.into_inner()))
}

/// The card's words, which differ between homes: a walk sends a note on to Send notes, the layer
/// saves it as a pin.
#[derive(Clone, Copy)]
pub struct Words {
    /// The card's confirming button.
    pub save: &'static str,
    /// A band over the card's path, with its crosshair: the layer's `New note`. A walk has none.
    pub band: Option<&'static str>,
    /// The card's own name, for VoiceOver and the walk's aim.
    pub card: &'static str,
    /// The layer's card carries the tools and the pictures; a walk's does not.
    pub tools: bool,
}

/// A note written on what was held.
pub struct Noted {
    pub said: String,
    pub held: Held,
    /// The window drawn with nothing of Capture's over it, and its pixels per point.
    pub picture: Option<(Arc<image::RgbaImage>, f32)>,
    /// The card's tools, with the changes and pictures that stand in them.
    pub tools: Option<Entity<Tools>>,
}

pub enum CaptureEvent {
    Noted(Noted),
    /// Esc, or Cancel: the person put it away.
    Closed,
}

impl EventEmitter<CaptureEvent> for Capture {}

/// Capture while it is armed: what the pointer is over, a box being dragged, and once a press
/// lets go, what is held, its picture and the note being written on it.
pub struct Capture {
    /// Where aim and a box may hold: the screen's own area, kept by whoever mounts this.
    within: Rc<Cell<Option<Bounds<Pixels>>>>,
    words: Words,
    /// The place the card's words are kept for, so a card put away and opened again finds them.
    keep: Option<String>,
    hover: Option<Drawn>,
    from: Option<Point<Pixels>>,
    to: Option<Point<Pixels>>,
    held: Option<Held>,
    /// The frame drawn for the picture: nothing of Capture's over the screen.
    clean: bool,
    picture: Option<(Arc<image::RgbaImage>, f32)>,
    note: Option<(Entity<TextareaState>, Subscription)>,
    tools: Option<Entity<Tools>>,
    /// ⌘V reads a picture off the clipboard before the field sees it, while the card is up.
    paste: Option<Subscription>,
    focus: FocusHandle,
}

impl Focusable for Capture {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Capture {
    pub fn new(within: Rc<Cell<Option<Bounds<Pixels>>>>, words: Words, keep: Option<String>, cx: &mut Context<Self>) -> Self {
        Capture {
            within,
            words,
            keep,
            hover: None,
            from: None,
            to: None,
            held: None,
            clean: false,
            picture: None,
            note: None,
            tools: None,
            paste: None,
            focus: cx.focus_handle(),
        }
    }

    /// The card's tools, while the card is up.
    pub fn tools(&self) -> Option<Entity<Tools>> {
        self.tools.clone()
    }

    /// A note on the whole window (⌥⌘S): the window as it stands is the picture, and the card opens
    /// for what he says. A note half written is not thrown away for it.
    pub fn whole_window(&mut self, screen: Option<SharedString>, cx: &mut Context<Self>) {
        if self.held.is_some() {
            return;
        }
        let Some(bounds) = self.within.get() else { return };
        self.hover = None;
        self.from = None;
        self.to = None;
        self.held = Some(Held::Window { bounds, screen });
        self.clean = true;
        cx.notify();
    }

    /// A note on the element under `at`, as a press on it would begin one: what a point menu's Report asks.
    pub fn element_at_point(&mut self, at: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
        if self.held.is_some() {
            return;
        }
        let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
        let Some(found) = self.within.get().and_then(|within| element_at(&drawn, at, within)) else { return };
        self.hover = None;
        self.from = None;
        self.to = None;
        self.held = Some(Held::Element(found));
        self.clean = true;
        cx.notify();
    }

    fn pointer_moved(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.held.is_some() {
            return;
        }
        if self.from.is_some() {
            self.to = Some(at);
        } else {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            self.hover = self.within.get().and_then(|within| element_at(&drawn, at, within));
        }
        cx.notify();
    }

    /// Whether the person has put anything on the note: words, a change or a picture. A stray press does
    /// not throw that away, while a note with nothing on it moves to what is pressed next.
    fn begun(&self, cx: &gpui::App) -> bool {
        let words = self.note.as_ref().is_some_and(|(input, _)| !input.read(cx).value().trim().is_empty());
        let tools = self.tools.as_ref().is_some_and(|tools| {
            let tools = tools.read(cx);
            !tools.changes().is_empty() || !tools.examples().is_empty()
        });
        words || tools
    }

    fn pressed(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        if self.held.is_some() && self.begun(cx) {
            return;
        }
        self.from = Some(at);
        self.to = Some(at);
        cx.notify();
    }

    /// A press lets go: a box if it was dragged, else the element under it. The window draws once
    /// with nothing of Capture's over the screen, and that frame is the note's picture.
    fn released(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.held.is_some() && self.begun(cx) {
            self.from = None;
            self.to = None;
            cx.notify();
            return;
        }
        let from = self.from.take().unwrap_or(at);
        self.to = None;
        // A press holds what is under it, which is what the last move aimed at when a pointer made both.
        let under = |this: &mut Self| {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            let under = this.within.get().and_then(|within| element_at(&drawn, at, within));
            this.hover.take().filter(|_| under.is_none()).or(under)
        };
        let held = region(from, at).map(Held::Region).or_else(|| under(self).map(Held::Element));
        let Some(held) = held else {
            cx.notify();
            return;
        };
        // A note with nothing on it gives way to what was pressed.
        self.note = None;
        self.tools = None;
        self.paste = None;
        self.held = Some(held);
        self.clean = true;
        cx.notify();
    }

    fn pictured(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let scale = window.scale_factor();
        self.picture = window.render_to_image().ok().map(|picture| (Arc::new(picture), scale));
        self.clean = false;
        let kept = self.keep.as_deref().and_then(|key| crate::drafts::kept(key, cx));
        let placeholder = "What should change here";
        let input = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx).auto_grow(3, 8).placeholder(placeholder);
            if let Some(kept) = &kept {
                state.set_value(kept.clone(), window, cx);
            }
            state
        });
        let keep = self.keep.clone();
        let watch = cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                if let Some(key) = &keep {
                    crate::drafts::keep(key, input.read(cx).value().as_ref(), cx);
                }
            }
            InputEvent::PressEnter { secondary: true, .. } => {
                // The key also put a line break in the field; the note is the words without it.
                let said = input.read(cx).value().trim().to_string();
                this.write(said, window, cx);
            }
            _ => {}
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.note = Some((input, watch));
        self.tools = self.words.tools.then(|| self.held.as_ref().map(tools::scope_of)).flatten().map(|scope| cx.new(|_| Tools::new(scope)));
        if self.tools.is_some() {
            let me = cx.entity().downgrade();
            self.paste = Some(cx.intercept_keystrokes(move |event, window, cx| {
                let key = &event.keystroke;
                if key.key != "v" || !key.modifiers.platform || key.modifiers.alt || key.modifiers.shift {
                    return;
                }
                let Some(me) = me.upgrade() else { return };
                let taken = me.update(cx, |this, cx| {
                    this.focus.contains_focused(window, cx) && this.tools.clone().is_some_and(|tools| tools.update(cx, |tools, cx| tools.paste(cx)))
                });
                if taken {
                    cx.stop_propagation();
                }
            }));
        }
        cx.notify();
    }

    /// The card's words are the note. Nothing is written for blank words.
    fn write(&mut self, said: String, _window: &mut Window, cx: &mut Context<Self>) {
        if said.trim().is_empty() {
            return;
        }
        let Some(held) = self.held.take() else { return };
        if let Some(key) = &self.keep {
            crate::drafts::clear(key, cx);
        }
        let picture = self.picture.take();
        self.paste = None;
        let tools = self.tools.take();
        cx.emit(CaptureEvent::Noted(Noted { said, held, picture, tools }));
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.paste = None;
        cx.emit(CaptureEvent::Closed);
    }
}

impl gpui::Render for Capture {
    /// What Capture draws over the screen: a crosshair cover that takes the pointer, the outline
    /// and name of what it is over or the box being dragged, and the note card on what is held.
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let root = div()
            .id("capture")
            .key_context("capture")
            .track_focus(&self.focus)
            // The field would keep Esc for itself.
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.close(cx);
                }
            }))
            .absolute()
            .inset_0();
        let Some(within) = self.within.get() else { return root };
        let edge = look.length(token::FOCUS_RING_WIDTH);
        let outline = |bounds: Bounds<Pixels>| {
            div()
                .absolute()
                .left(bounds.origin.x)
                .top(bounds.origin.y)
                .w(bounds.size.width)
                .h(bounds.size.height)
                .border(edge)
                .border_color(look.color(token::ACCENT))
                .bg(look.color(token::ACCENT).opacity(0.08))
        };
        if self.clean {
            // This frame has nothing of Capture's over the screen: once it is drawn it is the picture.
            let this = cx.entity().downgrade();
            window.defer(cx, move |window, cx| {
                let _ = this.update(cx, |this, cx| {
                    if this.clean {
                        this.pictured(window, cx);
                    }
                });
            });
            return root;
        }
        let mut layer = root;
        // The cover takes the pointer while a note is held too, so a press outside it never reaches the screen.
        layer = layer.child(
            div()
                .id("capture-cover")
                .absolute()
                .left(within.origin.x)
                .top(within.origin.y)
                .w(within.size.width)
                .h(within.size.height)
                .cursor(CursorStyle::Crosshair)
                .occlude()
                .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| this.pointer_moved(event.position, window, cx)))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| this.pressed(event.position, cx)))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &gpui::MouseUpEvent, window, cx| this.released(event.position, window, cx))),
        );
        let dragged = self.from.zip(self.to).and_then(|(from, to)| region(from, to));
        if let Some(bounds) = dragged {
            layer = layer.child(outline(bounds));
        } else if let Some(hover) = self.hover.as_ref().filter(|_| self.held.is_none()) {
            let name = Held::Element(hover.clone()).said();
            layer = layer.child(outline(hover.bounds)).child(
                div()
                    .absolute()
                    .left(hover.bounds.origin.x)
                    .top(hover.bounds.origin.y - look.length(token::H_BADGE) - edge)
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::ACCENT))
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(look.color(token::TEXT_ON_ACCENT))
                    .child(name),
            );
        }
        if let Some(held) = &self.held {
            let bounds = held.bounds();
            let whole = matches!(held, Held::Window { .. });
            if !whole {
                layer = layer.child(outline(bounds));
            }
            if let Some((input, _)) = &self.note {
                let input = input.clone();
                let sent = input.clone();
                let gap = look.length(token::SPACE_2);
                let band = self.words.band.map(|band| {
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::ACCENT))
                        .child(gpui::svg().path(crate::assets::icon_path("crosshair")).size(look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2.).flex_none().text_color(look.color(token::ACCENT)))
                        .child(band)
                });
                let tools = self.tools.clone();
                let dropped = self.tools.clone();
                let card = div()
                    .id("capture-card")
                    .occlude()
                    .key_context("capture-note")
                    .w(look.length(token::SPACE_8) * 11.)
                    .max_h(window.viewport_size().height - gap * 2.)
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .p(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .children(band)
                    .child(
                        div()
                            .truncate()
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(held.said()),
                    )
                    // The text-box key context, so typing here fires no list key; the walk window and the annotation layer share this note box.
                    .child(labelled(div().id("capture-note").key_context(crate::keymap::TEXT_BOX).child(Textarea::new(&input)), Role::TextInput, "Note"))
                    .children(tools)
                    .on_drop(move |paths: &ExternalPaths, _, cx| {
                        if let Some(tools) = &dropped {
                            tools::dropped(tools, paths, cx);
                        }
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(look.length(token::SPACE_2))
                            .child(button("capture-cancel", "Cancel", Some("esc"), Weight::Quiet, true, cx, cx.listener(|this, _, _, cx| this.close(cx))))
                            .child(button("capture-send", self.words.save, Some("⌘↵"), Weight::Primary, true, cx, cx.listener(move |this, _, window, cx| {
                                let said = sent.read(cx).value().to_string();
                                this.write(said, window, cx);
                            }))),
                    );
                layer = layer.child(placed(bounds, whole, window, gap, look.length(token::SPACE_8) * 11., labelled(card, Role::Dialog, self.words.card)));
            }
        }
        layer
    }
}

/// A card put where all of it is on screen: under what it is about, or over it when that is low on
/// the screen, and at the top of the window for a note on the whole of it.
pub fn placed(bounds: Bounds<Pixels>, whole: bool, window: &Window, gap: Pixels, width: Pixels, card: impl IntoElement) -> impl IntoElement {
    let view = window.viewport_size();
    let (corner, at) = if whole {
        (Anchor::TopLeft, point(bounds.origin.x + (bounds.size.width - width) / 2., bounds.origin.y + gap * 2.))
    } else if bounds.origin.y + bounds.size.height > view.height * 0.6 {
        (Anchor::BottomLeft, point(bounds.origin.x, bounds.origin.y - gap))
    } else {
        (Anchor::TopLeft, point(bounds.origin.x, bounds.origin.y + bounds.size.height + gap))
    };
    deferred(anchored().anchor(corner).position(at).snap_to_window_with_margin(gap).child(card))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    fn drawn(name: &str, role: Role, x: f32, y: f32, w: f32, h: f32) -> Drawn {
        let bounds = Bounds { origin: point(px(x), px(y)), size: size(px(w), px(h)) };
        Drawn { role, name: name.to_string().into(), bounds, shown: bounds, radius: None }
    }

    #[test]
    fn the_smallest_element_under_the_pointer_is_the_one_held() {
        let screen = Bounds { origin: point(px(0.), px(100.)), size: size(px(800.), px(600.)) };
        let all = vec![
            drawn("Jobs", Role::List, 10., 110., 300., 500.),
            drawn("Audit the docs index", Role::Row, 20., 200., 280., 36.),
            drawn("Walk", Role::Toolbar, 0., 0., 800., 88.),
        ];
        assert_eq!(element_at(&all, point(px(50.), px(210.)), screen).map(|one| one.name), Some("Audit the docs index".into()));
        assert_eq!(element_at(&all, point(px(50.), px(500.)), screen).map(|one| one.name), Some("Jobs".into()));
        // The walk bar is not the screen.
        assert!(element_at(&all, point(px(50.), px(40.)), screen).is_none());
    }

    #[test]
    fn a_drag_past_the_slop_is_a_box_either_way_and_a_press_is_not() {
        assert!(region(point(px(10.), px(10.)), point(px(12.), px(13.))).is_none());
        let drawn = region(point(px(100.), px(80.)), point(px(40.), px(20.))).expect("a box");
        assert_eq!(drawn.origin, point(px(40.), px(20.)));
        assert_eq!(drawn.size, size(px(60.), px(60.)));
    }

    #[test]
    fn a_ring_is_burned_at_the_windows_scale_with_its_number_and_a_ring_past_the_edge_is_cut() {
        let picture = image::RgbaImage::new(200, 200);
        let ink = [255, 0, 0, 255];
        let ring = Bounds { origin: point(px(20.), px(40.)), size: size(px(30.), px(40.)) };
        let burned = ringed(&picture, ring, 2., 12, ink, [255; 4]).expect("a picture");
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD.decode(burned).expect("base64");
        let back = image::load_from_memory(&bytes).expect("a PNG").to_rgba8();
        assert_eq!((back.width(), back.height()), (200, 200), "the whole window, not a crop");
        assert_eq!(back.get_pixel(40, 140).0, ink, "the ring's left edge, at twice the points");
        assert_eq!(back.get_pixel(70, 120).0, [0; 4], "inside the ring is left as it was");
        assert_eq!(back.get_pixel(41, 79).0, ink, "the number's pill stands above the ring");
        assert!(ringed(&picture, Bounds { origin: point(px(90.), px(40.)), size: size(px(50.), px(50.)) }, 2., 1, ink, [255; 4]).is_some());
    }
}
