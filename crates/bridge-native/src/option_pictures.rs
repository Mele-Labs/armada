//! The options of a Needs you question that the asker drew as stills (`ipc::OptionPicture`). Each still is
//! read from Fleet once (`GET /sessions/shown`), off the main thread, and kept by gpui's asset cache. Stills
//! sit side by side where the column fits them and stack where it does not. A press on a still opens it large,
//! never picks (owner, 13 Nov: a press to look cast his vote); the option's words under it pick. The pointer
//! on a named part shows that part's label over its box.

use std::sync::Arc;

use gpui::{
    anchored, deferred, div, img, prelude::*, px, relative, AnyElement, App, Asset, ClickEvent, FocusHandle, FontWeight, Global, Hsla,
    Image, ImageCacheError, ImageFormat, ObjectFit, RenderImage, Role, SharedString, Window,
};
use ipc::{OptionPicture, PicturePart, WaitingOption};

use crate::labels::labelled;
use crate::look::{token, Look};


/// A still Fleet keeps: its address, and the port Fleet answers on (`None` reads the mock's copy).
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct StillAt {
    pub port: Option<u16>,
    pub image: String,
}

/// gpui's loader for a still: read, then decoded, both away from the main thread.
pub enum Still {}

impl Asset for Still {
    type Source = StillAt;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(source: StillAt, cx: &mut App) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        let executor = cx.background_executor().clone();
        async move {
            let bytes = executor.spawn(async move { read(&source) }).await.map_err(|why| ImageCacheError::Io(Arc::new(std::io::Error::other(why))))?;
            let format = format_of(&bytes).ok_or_else(|| ImageCacheError::Io(Arc::new(std::io::Error::other("not a PNG or JPEG"))))?;
            Image::from_bytes(format, bytes).to_image_data(renderer).map_err(|why| ImageCacheError::Other(Arc::new(why)))
        }
    }
}

fn read(source: &StillAt) -> Result<Vec<u8>, String> {
    let Some(port) = source.port else {
        return crate::mock::shown_still(&source.image).map(<[u8]>::to_vec).ok_or_else(|| format!("no mock still at {}", source.image));
    };
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let (status, bytes) = runtime.block_on(crate::transport::get(port, &source.image))?;
    if (200..300).contains(&status) { Ok(bytes) } else { Err(format!("Fleet answered {status}")) }
}

/// What the asker may pass: a PNG or a JPEG, told by its first bytes.
fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, ..] => Some(ImageFormat::Jpeg),
        _ => None,
    }
}

/// A part's box as fractions of its still, kept inside the still.
pub fn part_box(picture: &OptionPicture, part: &PicturePart) -> [f32; 4] {
    let (w, h) = (picture.width.max(1) as f32, picture.height.max(1) as f32);
    let x = (part.x as f32 / w).clamp(0., 1.);
    let y = (part.y as f32 / h).clamp(0., 1.);
    [x, y, (part.w as f32 / w).clamp(0., 1. - x), (part.h as f32 / h).clamp(0., 1. - y)]
}

/// The options that carry a still, by their place among all the options.
pub fn drawn(options: &[WaitingOption]) -> Vec<(usize, &OptionPicture)> {
    options.iter().enumerate().filter_map(|(at, option)| option.picture.as_ref().map(|picture| (at, picture))).collect()
}

/// The widest a still is drawn in its card. Each card is the still's own shape at that width, so cards
/// side by side and stacked keep their height.
const WIDEST: f32 = 400.;

/// A still's size in its card: its own size, scaled down to `WIDEST`.
pub fn card_size(picture: &OptionPicture) -> (f32, f32) {
    let (w, h) = (picture.width.max(1) as f32, picture.height.max(1) as f32);
    let scale = (WIDEST / w).min(1.);
    (w * scale, h * scale)
}

/// One still as the full-size view draws it.
#[derive(Clone)]
struct Shown {
    key: SharedString,
    picture: OptionPicture,
    source: StillAt,
    label: String,
    hue: Hsla,
}

/// The still open at its own size over the window: every still of its question in order (← and → move
/// between them), which one is open, and the focus to give back on Esc. The window's root draws it
/// (`over`), so no panel it was pressed in limits its size.
struct Enlarged {
    ring: Vec<Shown>,
    at: usize,
    focus: FocusHandle,
    back: Option<FocusHandle>,
}

impl Global for Enlarged {}

fn enlarge(ring: Vec<Shown>, at: usize, window: &mut Window, cx: &mut App) {
    let focus = cx.focus_handle();
    let back = window.focused(cx);
    window.focus(&focus, cx);
    cx.set_global(Enlarged { ring, at, focus, back });
    window.refresh();
}

/// The still `by` places along from the open one, wrapping at either end.
fn step_to(by: isize, window: &mut Window, cx: &mut App) {
    let Some(open) = cx.try_global::<Enlarged>() else { return };
    let next = (open.at as isize + by).rem_euclid(open.ring.len().max(1) as isize) as usize;
    cx.global_mut::<Enlarged>().at = next;
    window.refresh();
}

/// Whether an opened still has the keys: it is drawn at the window's root, outside any screen, so a walk
/// that hands keys back to its screen leaves them here.
pub fn holds_keys(window: &Window, cx: &App) -> bool {
    cx.try_global::<Enlarged>().is_some_and(|open| open.focus.is_focused(window))
}

/// The open still over the whole window, for the window's root to draw last; nothing while none is open.
pub fn over(window: &Window, cx: &App) -> Option<AnyElement> {
    let open = cx.try_global::<Enlarged>()?;
    let shown = open.ring.get(open.at)?;
    Some(full_size(shown, &open.focus, open.ring.len() > 1, crate::room::of(window, cx), crate::look::ActiveLook::look(cx)))
}

fn shrink(window: &mut Window, cx: &mut App) {
    if let Some(back) = cx.try_global::<Enlarged>().and_then(|open| open.back.clone()) {
        window.focus(&back, cx);
    }
    cx.remove_global::<Enlarged>();
    window.refresh();
}

/// One still with its parts' boxes, `w` by `h`. `named` shows every part's label; otherwise a label shows
/// while the pointer is on its box. A label hangs under its box's right end, where a row's words
/// seldom are, and over it when the box reaches the still's foot.
fn still(id: &SharedString, picture: &OptionPicture, source: &StillAt, (w, h): (f32, f32), named: bool, hue: Hsla, look: &Look) -> gpui::Div {
    let source = source.clone();
    let tiers = label_tiers(picture, w);
    let step = f32::from(look.length(token::LEADING_2XS)) + 4.;
    let parts = picture.parts.iter().zip(tiers).map(|(part, tier)| {
        let [x, y, pw, ph] = part_box(picture, part);
        let name = SharedString::from(format!("{id}:{}", part.name));
        let label = div().absolute().right_0();
        let gap = px(2. + tier as f32 * step);
        let label = if y + ph > 0.88 { label.bottom_full().mb(gap) } else { label.top_full().mt(gap) };
        let label = label
            .px(look.length(token::SPACE_1))
            .py(px(1.))
            .rounded(look.length(token::RADIUS_SM))
            .bg(hue)
            .text_color(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .whitespace_nowrap()
            .child(SharedString::from(part.label.clone()));
        let part_box = div()
            .id(name.clone())
            .group(name.clone())
            .absolute()
            .left(relative(x))
            .top(relative(y))
            .w(relative(pw))
            .h(relative(ph))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if named { hue } else { gpui::transparent_black() })
            .hover(|style| style.border_color(hue).bg(hue.opacity(0.12)))
            .child(if named { label } else { label.opacity(0.).group_hover(name, |style| style.opacity(1.)) });
        labelled(part_box, Role::Image, part.label.clone())
    });
    div()
        .relative()
        .flex_none()
        .w(px(w))
        .h(px(h))
        .rounded(look.length(token::RADIUS_MD))
        .bg(look.color(token::BG_SUNKEN))
        .child(img(move |window: &mut Window, cx: &mut App| window.use_asset::<Still>(&source, cx)).size_full().rounded(look.length(token::RADIUS_MD)).object_fit(ObjectFit::Fill))
        .children(parts)
}

/// How many label heights each part's label drops below (or rises above) its box, so labels whose boxes end
/// side by side never sit on one another. A label's width is guessed from its letters.
fn label_tiers(picture: &OptionPicture, w: f32) -> Vec<usize> {
    let mut placed: Vec<(bool, usize, f32, f32)> = Vec::new();
    picture
        .parts
        .iter()
        .map(|part| {
            let [x, y, pw, ph] = part_box(picture, part);
            let below = y + ph <= 0.88;
            let end = (x + pw) * w;
            let start = end - (part.label.chars().count() as f32 * 6.5 + 8.);
            let tier = (0..).find(|tier| !placed.iter().any(|&(side, at, a, b)| side == below && at == *tier && start < b && a < end)).unwrap_or(0);
            placed.push((below, tier, start, end));
            tier
        })
        .collect()
}

/// One card for each option the asker drew as a still, or nothing when none was: the still, then the
/// option's number, label and what choosing it means. A press on the still, or Space on the card, opens it
/// large; only a press on its words picks it. `chosen` is the option picked; `pick` makes the press handler
/// for an option, by its place.
#[allow(clippy::too_many_arguments)]
pub fn stills(
    key: &str,
    options: &[WaitingOption],
    chosen: Option<usize>,
    port: Option<u16>,
    hue: Hsla,
    look: &Look,
    pick: &dyn Fn(usize) -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    _cx: &App,
) -> Option<AnyElement> {
    let drawn = drawn(options);
    if drawn.is_empty() {
        return None;
    }
    let ring = ring_of(key, options, port, hue);
    let cards = drawn.into_iter().enumerate().map(|(place, (at, picture))| {
        let option = &options[at];
        let label = option.label.clone();
        let on = chosen == Some(at);
        let source = StillAt { port, image: picture.image.clone() };
        let card_key = SharedString::from(format!("still:{key}:{at}"));
        let (w, h) = card_size(picture);
        let (space_ring, press_ring) = (ring.clone(), ring.clone());
        let picture_press = labelled(
            still(&card_key, picture, &source, (w, h), false, hue, look)
                .id(SharedString::from(format!("{card_key}:picture")))
                .w_full()
                .max_w(px(w))
                .h_auto()
                .aspect_ratio(w / h)
                .cursor_pointer()
                .tooltip(crate::titlebar::parts::tip("Larger (Space)"))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    enlarge(press_ring.clone(), place, window, cx);
                }),
            Role::Button,
            format!("Picture of {label}"),
        );
        let card = div()
            .id(card_key.clone())
            .focusable()
            .flex_none()
            .w(px(w) + look.length(token::SPACE_2) * 2.)
            .max_w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .bg(if on { hue.opacity(0.14) } else { look.color(token::FG_DEFAULT).opacity(0.04) })
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { hue } else { look.color(token::FG_DEFAULT).opacity(0.07) })
            .on_key_down(move |event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "space" {
                    cx.stop_propagation();
                    enlarge(space_ring.clone(), place, window, cx);
                }
            })
            .child(picture_press)
            .child(labelled(
                div()
                    .id(SharedString::from(format!("{card_key}:pick")))
                    .cursor_pointer()
                    .rounded(look.length(token::RADIUS_MD))
                    .py(look.length(token::SPACE_1))
                    .hover(|style| style.bg(hue.opacity(0.12)))
                    .on_click(pick(at))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_1))
                    .child(div().flex_none().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_SUBTLE)).child(format!("{}", at + 1)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(label.clone())))
                            .children(option.description.as_ref().filter(|said| !said.is_empty()).map(|said| {
                                div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone()))
                            })),
                    ),
                Role::Button,
                format!("Choose {label}"),
            ));
        div().flex_none().max_w_full().child(labelled(card, Role::Group, label.clone())).into_any_element()
    });
    Some(labelled(div().id(SharedString::from(format!("stills:{key}"))).flex().flex_wrap().gap(look.length(token::SPACE_3)).children(cards), Role::Group, "Pictures").into_any_element())
}

/// Every still of a question, in order, as the full-size view steps through them.
fn ring_of(key: &str, options: &[WaitingOption], port: Option<u16>, hue: Hsla) -> Vec<Shown> {
    drawn(options)
        .into_iter()
        .map(|(at, picture)| Shown { key: SharedString::from(format!("still:{key}:{at}")), picture: picture.clone(), source: StillAt { port, image: picture.image.clone() }, label: options[at].label.clone(), hue })
        .collect()
}

/// Option `at`'s still at `size` for the band's focus view, every part named when `named`, else on the pointer. A
/// press opens it full size, where ← and → reach the question's other stills; it never picks.
#[allow(clippy::too_many_arguments)]
pub fn large(key: &str, options: &[WaitingOption], at: usize, size: (f32, f32), named: bool, port: Option<u16>, hue: Hsla, look: &Look) -> Option<AnyElement> {
    let ring = ring_of(key, options, port, hue);
    let place = drawn(options).iter().position(|(one, _)| *one == at)?;
    let Shown { key, picture, source, label, .. } = ring.get(place)?.clone();
    Some(
        labelled(
            still(&key, &picture, &source, size, named, hue, look)
                .id(SharedString::from(format!("{key}:large")))
                .cursor_pointer()
                .tooltip(crate::titlebar::parts::tip("Full size"))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    enlarge(ring.clone(), place, window, cx);
                }),
            Role::Button,
            format!("Picture of {label}"),
        )
        .into_any_element(),
    )
}

/// Option `at`'s still as a thumbnail `w` by `h`, cropped to fill it: to glance at, not to press.
pub fn thumb(key: &str, options: &[WaitingOption], at: usize, (w, h): (f32, f32), port: Option<u16>, look: &Look) -> Option<AnyElement> {
    let option = options.get(at)?;
    let source = StillAt { port, image: option.picture.as_ref()?.image.clone() };
    Some(
        labelled(
            div()
                .id(SharedString::from(format!("thumb:{key}:{at}")))
                .flex_none()
                .w(px(w))
                .h(px(h))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_SUNKEN))
                .overflow_hidden()
                .child(img(move |window: &mut Window, cx: &mut App| window.use_asset::<Still>(&source, cx)).size_full().rounded(look.length(token::RADIUS_MD)).object_fit(ObjectFit::Cover)),
            Role::Image,
            format!("Picture of {}", option.label),
        )
        .into_any_element(),
    )
}

/// The still whole at its own size, or scaled down to fit the room left inside the dim, the frame around it
/// and the frame's head (owner, 13 Nov: a zoom shows the whole picture). A still so tall that fitting it
/// would leave it narrower than `READABLE` keeps that width and scrolls in the frame instead.
fn natural_in(picture: &OptionPicture, room: gpui::Size<gpui::Pixels>, look: &Look) -> (f32, f32) {
    let (w, h) = (picture.width.max(1) as f32, picture.height.max(1) as f32);
    let inset = (f32::from(look.length(token::SPACE_8)) + f32::from(look.length(token::SPACE_4))) * 2.;
    let head = f32::from(look.length(token::SPACE_3)) + f32::from(look.length(token::SPACE_4)) + f32::from(look.length(token::LEADING_BASE)).max(f32::from(look.length(token::H_CONTROL_SM)));
    let across = ((f32::from(room.width) - inset) / w).max(0.01);
    let down = ((f32::from(room.height) - inset - head) / h).max(0.01);
    let fit = across.min(down);
    let scale = if w * fit < READABLE.min(w * across) { (READABLE / w).min(across) } else { fit };
    let scale = scale.min(1.);
    (w * scale, h * scale)
}

/// The narrowest a tall still is drawn full size before it scrolls rather than shrinks further.
const READABLE: f32 = 480.;

/// The still over the window at its own size, every part named; ← and → move to the still beside it, and Esc
/// or a press beside it closes it. Nothing here picks.
fn full_size(shown: &Shown, focus: &FocusHandle, many: bool, within: gpui::Bounds<gpui::Pixels>, look: &Look) -> AnyElement {
    let room = within.size;
    let Shown { key, picture, source, label, hue } = shown;
    let hue = *hue;
    let id = SharedString::from(format!("{key}:full"));
    let inside = look.length(token::SPACE_8) * 2.;
    let body = div()
        .id(SharedString::from(format!("{key}:full-body")))
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .p(look.length(token::SPACE_4))
        // In pixels, not a share of the dim: inside Needs you a share came out as the panel's height.
        .max_w(room.width - inside)
        .max_h(room.height - inside)
        .overflow_scroll()
        .rounded(look.length(token::RADIUS_LG))
        .bg(look.color(token::BG_OVERLAY))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .on_click(|_, _, cx| cx.stop_propagation())
        .child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(div().flex_1().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(label.to_string())))
                .when(many, |head| head.child(crate::titlebar::parts::kbd("\u{2190}", look)).child(crate::titlebar::parts::kbd("\u{2192}", look)))
                .child(crate::titlebar::parts::kbd("Esc", look)),
        )
        .child(still(&id, picture, source, natural_in(picture, room, look), true, hue, look).mt(look.length(token::SPACE_4)));
    deferred(
        anchored().position(within.origin).child(
            labelled(
                div()
                    .id(id.clone())
                    .track_focus(focus)
                    .occlude()
                    .w(room.width)
                    .h(room.height)
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(look.length(token::SPACE_8))
                    .bg(gpui::black().opacity(0.6))
                    .on_click(|_, window, cx| shrink(window, cx))
                    .on_key_down(|event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                        "escape" | "space" => {
                            cx.stop_propagation();
                            shrink(window, cx);
                        }
                        "left" => {
                            cx.stop_propagation();
                            step_to(-1, window, cx);
                        }
                        "right" => {
                            cx.stop_propagation();
                            step_to(1, window, cx);
                        }
                        _ => {}
                    })
                    .child(body),
                Role::Dialog,
                format!("Full size: {label}"),
            ),
        ),
    )
    .with_priority(2)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(parts: Vec<PicturePart>) -> OptionPicture {
        OptionPicture { image: "/sessions/shown?session_id=j&path=a.png".into(), width: 1000, height: 900, parts }
    }

    fn part(x: u32, y: u32, w: u32, h: u32) -> PicturePart {
        PicturePart { name: "p".into(), label: "P".into(), x, y, w, h }
    }

    #[test]
    fn a_part_box_is_its_share_of_the_still_and_never_reaches_past_it() {
        let still = picture(vec![]);
        let [x, y, w, h] = part_box(&still, &part(473, 134, 258, 24));
        assert!((x - 0.473).abs() < 1e-4 && (y - 134. / 900.).abs() < 1e-4 && (w - 0.258).abs() < 1e-4 && (h - 24. / 900.).abs() < 1e-4);
        let [x, _, w, h] = part_box(&still, &part(900, 880, 300, 100));
        assert!((x + w - 1.).abs() < 1e-4, "a box past the right edge stops at it");
        assert!(h <= 20. / 900. + 1e-4, "a box past the foot stops at it");
    }

    #[test]
    fn a_still_opened_full_size_fits_the_room_whole_unless_fitting_would_leave_it_too_narrow() {
        let look = crate::look::Look::dark();
        let room = |w: f32, h: f32| gpui::size(px(w), px(h));
        let small = OptionPicture { width: 300, height: 120, ..picture(vec![]) };
        assert_eq!(natural_in(&small, room(1400., 820.), &look), (300., 120.), "a small still keeps its own size");
        let wide = OptionPicture { width: 1520, height: 1080, ..picture(vec![]) };
        let (w, h) = natural_in(&wide, room(820., 730.), &look);
        assert!(h <= 730. - 96. - 30. && w < 820., "a big still shrinks until its whole height fits: {w} by {h}");
        assert!(((w / h) - 1520. / 1080.).abs() < 1e-3, "and keeps its shape");
        let tall = OptionPicture { width: 760, height: 2400, ..picture(vec![]) };
        let (w, h) = natural_in(&tall, room(1400., 820.), &look);
        assert!((w - READABLE).abs() < 1e-3 && h > 820., "a tall still keeps a readable width and scrolls: {w} by {h}");
    }

    #[test]
    fn labels_of_boxes_that_end_side_by_side_stack_rather_than_cover_each_other() {
        let part = |name: &str, x: u32, w: u32| PicturePart { name: name.into(), label: name.into(), x, y: 134, w, h: 24 };
        let tiers = label_tiers(&OptionPicture { width: 760, height: 540, ..picture(vec![part("Main act", 623, 80), part("More menu", 707, 24), part("Row note", 29, 100)]) }, 760.);
        assert_eq!(tiers, vec![0, 1, 0], "the second label drops a line; one far off keeps its place");
    }

    #[test]
    fn only_options_with_a_still_are_drawn_and_keep_their_place_among_all() {
        let words = WaitingOption { label: "Neither".into(), description: Some("Keep it as is".into()), picture: None };
        let with = |label: &str| WaitingOption { label: label.into(), description: None, picture: Some(picture(vec![])) };
        let options = vec![with("A"), words, with("B")];
        let places: Vec<usize> = drawn(&options).into_iter().map(|(at, _)| at).collect();
        assert_eq!(places, vec![0, 2]);
        assert!(drawn(&options[1..2]).is_empty());
    }

    #[test]
    fn a_still_keeps_its_own_shape_in_its_card_and_is_never_drawn_past_its_size() {
        let wide = OptionPicture { width: 760, height: 380, ..picture(vec![]) };
        let (w, h) = card_size(&wide);
        assert!((w - 400.).abs() < 1e-3 && (h - 200.).abs() < 1e-3, "760x380 is drawn 400x200");
        let small = OptionPicture { width: 300, height: 120, ..picture(vec![]) };
        assert_eq!(card_size(&small), (300., 120.), "a still narrower than a card keeps its own size");
    }

    #[test]
    fn a_still_is_read_as_png_or_jpeg_from_its_first_bytes() {
        assert!(matches!(format_of(&[0x89, b'P', b'N', b'G', 13, 10]), Some(ImageFormat::Png)));
        assert!(matches!(format_of(&[0xFF, 0xD8, 0xFF]), Some(ImageFormat::Jpeg)));
        assert!(format_of(b"GIF89a").is_none());
    }

    /// The read a running Fleet answers: a real socket, the still's address sent as Fleet gave it, and the PNG
    /// bytes back whole. Mock data never takes this path.
    #[test]
    fn a_still_is_read_from_fleet_over_its_own_port_with_its_address_whole() {
        use std::io::{Read, Write};
        let png: Vec<u8> = [&[0x89, b'P', b'N', b'G', 13, 10, 26, 10][..], &[7u8; 4000][..]].concat();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let served = png.clone();
        let fleet = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut asked = Vec::new();
            let mut chunk = [0u8; 1024];
            while !asked.windows(4).any(|end| end == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap();
                asked.extend_from_slice(&chunk[..n]);
            }
            let head = format!("HTTP/1.1 200 OK\r\ncontent-type: image/png\r\ncontent-length: {}\r\n\r\n", served.len());
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&served).unwrap();
            String::from_utf8(asked).unwrap()
        });
        let image = "/sessions/shown?session_id=j-1&path=%2FUsers%2Fme%2FApplication%20Support%2Fc70e.png".to_string();
        let bytes = read(&StillAt { port: Some(port), image: image.clone() }).unwrap();
        let asked = fleet.join().unwrap();
        assert!(asked.starts_with(&format!("GET {image} HTTP/1.1\r\n")), "{asked}");
        assert_eq!(bytes, png);
        assert!(matches!(format_of(&bytes), Some(ImageFormat::Png)));
    }

    #[test]
    fn a_still_fleet_will_not_give_is_a_failed_read() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let fleet = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut chunk = [0u8; 1024];
            let _ = stream.read(&mut chunk).unwrap();
            stream.write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n").unwrap();
        });
        let read = read(&StillAt { port: Some(port), image: "/sessions/shown?session_id=j&path=gone.png".into() });
        fleet.join().unwrap();
        assert_eq!(read, Err("Fleet answered 404".to_string()));
    }
}
