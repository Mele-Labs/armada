//! The options of a Needs you question that the asker drew as stills (`ipc::OptionPicture`). Each still is
//! read from Fleet once (`GET /sessions/shown`), off the main thread, and kept by gpui's asset cache. Stills
//! sit side by side where the column fits them and stack where it does not. A press on one picks its option,
//! and the pointer on a named part shows that part's label over its box.

use std::sync::Arc;

use gpui::{
    anchored, deferred, div, img, point, prelude::*, px, relative, AnyElement, App, Asset, ClickEvent, FocusHandle, FontWeight, Global, Hsla,
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

/// The still open at its own size over the window: its card's key, and the focus to give back on Esc.
struct Enlarged {
    key: SharedString,
    focus: FocusHandle,
    back: Option<FocusHandle>,
}

impl Global for Enlarged {}

thread_local! {
    /// The window's size at the last frame that drew stills: what the full-size view covers.
    static ROOM: std::cell::Cell<gpui::Size<gpui::Pixels>> = std::cell::Cell::new(gpui::size(px(1200.), px(800.)));
}

fn enlarge(key: SharedString, window: &mut Window, cx: &mut App) {
    let focus = cx.focus_handle();
    let back = window.focused(cx);
    window.focus(&focus, cx);
    cx.set_global(Enlarged { key, focus, back });
    window.refresh();
}

fn shrink(window: &mut Window, cx: &mut App) {
    if let Some(back) = cx.try_global::<Enlarged>().and_then(|open| open.back.clone()) {
        window.focus(&back, cx);
    }
    cx.remove_global::<Enlarged>();
    window.refresh();
}

/// One still with its parts' boxes, `w` by `h`. `named` shows every part's label; otherwise a label shows
/// while the pointer is on its box.
fn still(id: &SharedString, picture: &OptionPicture, source: &StillAt, (w, h): (f32, f32), named: bool, hue: Hsla, look: &Look) -> gpui::Div {
    let source = source.clone();
    let parts = picture.parts.iter().map(|part| {
        let [x, y, pw, ph] = part_box(picture, part);
        let name = SharedString::from(format!("{id}:{}", part.name));
        let label = div()
            .absolute()
            .bottom_full()
            .left_0()
            .mb(px(2.))
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

/// One card for each option the asker drew as a still, or nothing when none was: the still, then the
/// option's number, label and what choosing it means. A press on the card picks it; the corner button and
/// Space open the still at its own size without picking. `chosen` is the option picked; `pick` makes the
/// press handler for an option, by its place.
#[allow(clippy::too_many_arguments)]
pub fn stills(
    key: &str,
    options: &[WaitingOption],
    chosen: Option<usize>,
    port: Option<u16>,
    hue: Hsla,
    look: &Look,
    pick: &dyn Fn(usize) -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    cx: &App,
) -> Option<AnyElement> {
    let drawn = drawn(options);
    if drawn.is_empty() {
        return None;
    }
    let open = cx.try_global::<Enlarged>().map(|open| (open.key.clone(), open.focus.clone()));
    let cards = drawn.into_iter().map(|(at, picture)| {
        let option = &options[at];
        let label = option.label.clone();
        let on = chosen == Some(at);
        let source = StillAt { port, image: picture.image.clone() };
        let card_key = SharedString::from(format!("still:{key}:{at}"));
        let (w, h) = card_size(picture);
        let expand_key = card_key.clone();
        let expand = labelled(
            div()
                .id(SharedString::from(format!("{card_key}:expand")))
                .absolute()
                .top(look.length(token::SPACE_1))
                .right(look.length(token::SPACE_1))
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_OVERLAY).opacity(0.85))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .hover(|style| style.border_color(hue))
                .cursor_pointer()
                .tooltip(crate::titlebar::parts::tip("Full size (Space)"))
                .child(crate::titlebar::parts::glyph("maximize-2", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    enlarge(expand_key.clone(), window, cx);
                }),
            Role::Button,
            format!("Full size: {label}"),
        );
        let space_key = card_key.clone();
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
            .cursor_pointer()
            .bg(if on { hue.opacity(0.14) } else { look.color(token::FG_DEFAULT).opacity(0.04) })
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { hue } else { look.color(token::FG_DEFAULT).opacity(0.07) })
            .hover(|style| style.border_color(hue.opacity(0.6)))
            .on_click(pick(at))
            .on_key_down(move |event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "space" {
                    cx.stop_propagation();
                    enlarge(space_key.clone(), window, cx);
                }
            })
            .child(still(&card_key, picture, &source, (w, h), false, hue, look).w_full().max_w(px(w)).h_auto().aspect_ratio(w / h).child(expand))
            .child(
                div()
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
            );
        let enlarged = open.as_ref().filter(|(open_key, _)| *open_key == card_key).map(|(_, focus)| full_size(&card_key, picture, &source, &label, focus, hue, look));
        div().flex_none().max_w_full().child(labelled(card, Role::Button, format!("Picture of {label}"))).children(enlarged).into_any_element()
    });
    let room = gpui::canvas(|_, window, _| ROOM.with(|room| room.set(window.viewport_size())), |_, _, _, _| {}).absolute().size_0();
    Some(labelled(div().id(SharedString::from(format!("stills:{key}"))).relative().flex().flex_wrap().gap(look.length(token::SPACE_3)).children(cards).child(room), Role::Group, "Pictures").into_any_element())
}

/// The still at its own size, or less where the window is smaller: the room left inside the dim and the frame
/// around it, and under the frame's head.
fn natural_in(picture: &OptionPicture, room: gpui::Size<gpui::Pixels>, look: &Look) -> (f32, f32) {
    let (w, h) = (picture.width.max(1) as f32, picture.height.max(1) as f32);
    let inset = (f32::from(look.length(token::SPACE_8)) + f32::from(look.length(token::SPACE_4))) * 2.;
    let head = f32::from(look.length(token::SPACE_4)) + f32::from(look.length(token::LEADING_BASE)) + f32::from(look.length(token::SPACE_3));
    let scale = ((f32::from(room.width) - inset) / w).min((f32::from(room.height) - inset - head) / h).clamp(0.1, 1.);
    (w * scale, h * scale)
}

/// The still over the window at its own size, every part named; Esc or a press beside it closes it.
#[allow(clippy::too_many_arguments)]
fn full_size(key: &SharedString, picture: &OptionPicture, source: &StillAt, label: &str, focus: &FocusHandle, hue: Hsla, look: &Look) -> AnyElement {
    let id = SharedString::from(format!("{key}:full"));
    let room = ROOM.with(|room| room.get());
    let body = div()
        .id(SharedString::from(format!("{key}:full-body")))
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .p(look.length(token::SPACE_4))
        .max_w_full()
        .max_h_full()
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
                .child(crate::titlebar::parts::kbd("Esc", look)),
        )
        .child(still(&id, picture, source, natural_in(picture, room, look), true, hue, look).mt(look.length(token::SPACE_4)));
    deferred(
        anchored().position(point(px(0.), px(0.))).child(
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
                    .on_key_down(|event: &gpui::KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "escape" | "space") {
                            cx.stop_propagation();
                            shrink(window, cx);
                        }
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
}
