//! The pictures his messages carried, drawn in the bubble under the words as Electron's `Enlargeable`
//! thumbnails (`SessionThread.tsx` `Sent`), and opened larger over the window on a press, where it zooms as
//! `ZoomablePicture` does: a wheel or pinch about the pointer, a drag when zoomed, a double press between fit
//! and 2.5x, `+`, `-`, `0` and the arrows. A picture is read from Fleet on its row's first draw
//! (`get_session_file`) and kept, so a thread read again after a restart draws it the same.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    anchored, deferred, div, img, point, prelude::*, px, size, Anchor, AnyElement, App, Context, FocusHandle, Image, ImageFormat, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, Point, Role, ScrollDelta, ScrollWheelEvent, SharedString, Size, Window,
};
use ipc::SentFile;

use super::rows::{glyph, Size as GlyphSize};
use super::wire::{encoded, Done};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// One picture, as far as it has been read.
pub(super) enum Picture {
    Reading,
    /// `focus` is the thumbnail's, and `seen` says the last frame drew it.
    Ready { image: Arc<Image>, natural: Size<f32>, focus: FocusHandle, seen: std::cell::Cell<bool> },
    /// Fleet would not give it: the file is named in a chip instead.
    Unread,
}

/// The picture open over the window, and how far it is zoomed and moved from the window's middle.
pub(super) struct Viewing {
    image: Arc<Image>,
    natural: Size<f32>,
    name: SharedString,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    scale: f32,
    pan: Point<f32>,
    /// The press held: where it began, where the pointer was last, and whether it has travelled.
    press: Option<(Point<f32>, Point<f32>, bool)>,
}

/// `ZoomablePicture`'s numbers: fit is 1, the most is 8, a double press goes to 2.5, a key steps by
/// 1.5 and pans 64, a press that travels further than 8 is a drag, and a pinch and a wheel each have
/// their own speed.
const MOST: f32 = 8.;
const DOUBLE_TAP_SCALE: f32 = 2.5;
const KEY_STEP: f32 = 1.5;
const KEY_PAN: f32 = 64.;
const TAP_SLOP: f32 = 8.;
const PINCH_SPEED: f32 = 0.01;
const WHEEL_SPEED: f32 = 0.002;
const LINE_HEIGHT: f32 = 16.;

fn limited(scale: f32) -> f32 {
    scale.clamp(1., MOST)
}

/// How far one axis may be moved: an edge of a picture larger than the window never comes inside it,
/// and a smaller one stays in the middle.
fn panned(pan: f32, length: f32, scale: f32, whole: f32) -> f32 {
    let half = length * scale / 2.;
    if half * 2. <= whole {
        return 0.;
    }
    pan.clamp(whole / 2. - half, half - whole / 2.)
}

#[derive(Default)]
pub(super) struct Pictures {
    held: HashMap<String, Picture>,
    pub(super) viewing: Option<Viewing>,
}

fn key(job: &str, file: &str) -> String {
    format!("{job}\n{file}")
}

/// The formats gpui draws.
fn format_of(media_type: &str) -> Option<ImageFormat> {
    Some(match media_type {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::Webp,
        _ => return None,
    })
}

/// Whether a file is a picture the thread draws.
pub(super) fn is_picture(file: &SentFile) -> bool {
    format_of(&file.media_type).is_some()
}

/// The picture a `Read` looked at, by its absolute path: named by its last part, its type from its
/// ending. `None` for one gpui does not draw.
pub(super) fn read_picture(path: &str) -> Option<SentFile> {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    let media_type = match name.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return None,
    };
    Some(SentFile { id: path.to_string(), name, media_type: media_type.to_string() })
}

/// A picture's width and height, read from its header: PNG, GIF, JPEG and WebP. Anything else is fitted
/// whole inside the thumbnail's box.
pub(super) fn natural_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let be32 = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let be16 = |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_be_bytes([b[0], b[1]])));
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some((be32(16)?, be32(20)?));
    }
    if bytes.starts_with(b"GIF8") {
        let le16 = |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
        return Some((le16(6)?, le16(8)?));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // The frame header's height and width, past the segments before it.
        let mut at = 2;
        while at + 9 < bytes.len() {
            if bytes[at] != 0xFF {
                return None;
            }
            let marker = bytes[at + 1];
            if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                return Some((be16(at + 7)?, be16(at + 5)?));
            }
            at += 2 + be16(at + 2)? as usize;
        }
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let le = |at: usize, n: usize| bytes.get(at..at + n).map(|b| b.iter().rev().fold(0u32, |all, byte| all << 8 | u32::from(*byte)));
        return match bytes.get(12..16)? {
            b"VP8 " => Some((le(26, 2)? & 0x3FFF, le(28, 2)? & 0x3FFF)),
            b"VP8L" => {
                let bits = le(21, 4)?;
                Some(((bits & 0x3FFF) + 1, (bits >> 14 & 0x3FFF) + 1))
            }
            b"VP8X" => Some((le(24, 3)? + 1, le(27, 3)? + 1)),
            _ => None,
        };
    }
    None
}

/// `natural` made to fit inside `most`, keeping its shape and never made larger.
pub(super) fn fitted(natural: Size<f32>, most: Size<f32>) -> Size<f32> {
    if natural.width <= 0. || natural.height <= 0. {
        return most;
    }
    let scale = (most.width / natural.width).min(most.height / natural.height).min(1.);
    size(natural.width * scale, natural.height * scale)
}

/// What a mock thread's pictures read as, with no Fleet to ask: a window with a sidebar and a few rows, in
/// the shape its name says (`Wide`, `Tall`), else 960 by 600.
fn mock_picture(name: &str) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};
    let (w, h) = if name.starts_with("Wide") { (1600u32, 400u32) } else if name.starts_with("Tall") { (480, 1440) } else { (960, 600) };
    let (wf, hf) = (w as f32, h as f32);
    let side = (wf * 0.24).min(230.);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" rx="18" fill="#0f141b"/>
<rect width="{w}" height="52" rx="18" fill="#1a212b"/><rect y="34" width="{w}" height="18" fill="#1a212b"/>
<circle cx="30" cy="26" r="7" fill="#ff5f57"/><circle cx="54" cy="26" r="7" fill="#febc2e"/><circle cx="78" cy="26" r="7" fill="#28c840"/>
<rect x="20" y="72" width="{side}" height="{body}" rx="12" fill="#161c25"/>
<rect x="36" y="92" width="{chip}" height="34" rx="8" fill="#23406a"/>
<rect x="36" y="140" width="{line}" height="14" rx="7" fill="#2b3442"/><rect x="36" y="172" width="{line2}" height="14" rx="7" fill="#2b3442"/>
<rect x="{mx}" y="72" width="{mw}" height="{body}" rx="12" fill="#161c25"/>
<rect x="{tx}" y="104" width="{tw}" height="20" rx="10" fill="#3a4556"/>
<rect x="{tx}" y="148" width="{cw}" height="12" rx="6" fill="#2b3442"/><rect x="{tx}" y="172" width="{cw2}" height="12" rx="6" fill="#2b3442"/>
<rect x="{tx}" y="216" width="{cw}" height="{card}" rx="12" fill="#1d2633" stroke="#e5a03a" stroke-width="3"/>
<rect x="{cx}" y="244" width="{cw3}" height="14" rx="7" fill="#e5a03a"/><rect x="{cx}" y="{by}" width="120" height="30" rx="8" fill="#3b82f6"/>
</svg>"##,
        body = hf - 92.,
        chip = side - 32.,
        line = side * 0.74,
        line2 = side * 0.65,
        mx = side + 40.,
        mw = wf - side - 60.,
        tx = side + 70.,
        tw = (wf - side - 120.) * 0.6,
        cw = wf - side - 120.,
        cw2 = (wf - side - 120.) * 0.9,
        card = (hf - 300.).min(170.).max(80.),
        cx = side + 94.,
        cw3 = (wf - side - 168.) * 0.4,
        by = 216. + (hf - 300.).min(170.).max(80.) - 46.,
    );
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

impl JobView {
    /// Read the pictures among `files` that have not been asked for yet. The row draws a placeholder until one comes.
    pub(super) fn want_pictures(&mut self, job: &SharedString, files: &[SentFile], window: &mut Window, cx: &mut Context<Self>) {
        self.want(job, files, false, window, cx);
    }

    /// Read the pictures `Read` calls looked at, from the Job's own tree (`GET /sessions/repository_file`).
    /// A refusal, or a file that came back as text, leaves the row without one.
    pub(super) fn want_read_pictures(&mut self, job: &SharedString, files: &[SentFile], window: &mut Window, cx: &mut Context<Self>) {
        self.want(job, files, true, window, cx);
    }

    fn want(&mut self, job: &SharedString, files: &[SentFile], from_tree: bool, window: &mut Window, cx: &mut Context<Self>) {
        for file in files.iter().filter(|file| is_picture(file)) {
            let held = key(job, &file.id);
            if self.pictures.held.contains_key(&held) {
                continue;
            }
            self.pictures.held.insert(held.clone(), Picture::Reading);
            if self.fleet.is_none() {
                // Asked in the middle of the list's own layout: the answer lands after it, as Fleet's would.
                let (media_type, name) = (file.media_type.clone(), file.name.clone());
                cx.spawn(async move |this, cx| {
                    let _ = this.update(cx, |this, cx| this.keep_picture(held, &media_type, mock_picture(&name), cx));
                })
                .detach();
                continue;
            }
            if from_tree {
                let path = format!("/sessions/repository_file?session_id={}&path={}", encoded(job), encoded(&file.id));
                let done: Done = Box::new(move |this, answer, _, cx| {
                    use base64::Engine as _;
                    let picture = answer.ok().and_then(|body| serde_json::from_slice::<ipc::RepositoryFile>(&body).ok()).and_then(|read| match read {
                        ipc::RepositoryFile::Picture { bytes, media_type } => base64::engine::general_purpose::STANDARD.decode(bytes).ok().map(|bytes| (media_type, bytes)),
                        ipc::RepositoryFile::Text { .. } => None,
                    });
                    match picture {
                        Some((media_type, bytes)) => this.keep_picture(held, &media_type, Some(bytes), cx),
                        None => this.keep_picture(held, "", None, cx),
                    }
                });
                self.get(path, window, cx, done);
                continue;
            }
            let path = format!("/sessions/file?session_id={}&file={}", encoded(job), encoded(&file.id));
            let media_type = file.media_type.clone();
            let done: Done = Box::new(move |this, answer, _, cx| {
                this.keep_picture(held, &media_type, answer.ok().filter(|bytes| !bytes.is_empty()), cx);
            });
            self.get(path, window, cx, done);
        }
    }

    /// The bytes of a picture he sent this run, kept so his own bubble draws it without asking Fleet.
    pub(super) fn sent_picture(&mut self, job: &str, file: &str, media_type: &str, bytes: &[u8], cx: &mut Context<Self>) {
        self.keep_picture(key(job, file), media_type, Some(bytes.to_vec()), cx);
    }

    fn keep_picture(&mut self, held: String, media_type: &str, bytes: Option<Vec<u8>>, cx: &mut Context<Self>) {
        let picture = match (bytes, format_of(media_type)) {
            (Some(bytes), Some(format)) => {
                let most = thumb_most(cx.look());
                let natural = natural_size(&bytes).map_or(most, |(w, h)| size(w as f32, h as f32));
                Picture::Ready { image: Arc::new(Image::from_bytes(format, bytes)), natural, focus: cx.focus_handle(), seen: std::cell::Cell::new(false) }
            }
            _ => Picture::Unread,
        };
        self.pictures.held.insert(held, picture);
        // The row grows from its placeholder to the picture's own shape.
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            page.list.remeasure();
        }
        cx.notify();
    }

    /// A file of his message inside the bubble: its picture, a placeholder while it is read, or `None` for a chip.
    pub(super) fn picture_tile(&self, file: &SentFile, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.as_ref()?;
        let most = thumb_most(look);
        let tile = |id: String| {
            div()
                .id(SharedString::from(id))
                .flex_none()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
        };
        match self.pictures.held.get(&key(job, &file.id)) {
            None | Some(Picture::Reading) => Some(
                tile(format!("picture-reading:{}", file.id))
                    .w(px(most.width))
                    .h(px(most.height))
                    .bg(look.color(token::BG_SUNKEN))
                    .into_any_element(),
            ),
            Some(Picture::Unread) => None,
            Some(Picture::Ready { image, natural, focus, seen }) => {
                seen.set(true);
                let shown = fitted(*natural, most);
                let (image, natural, name) = (image.clone(), *natural, SharedString::from(file.name.clone()));
                let (pressed, keyed) = ((image.clone(), name.clone()), (image.clone(), name.clone()));
                Some(
                    labelled(
                        tile(format!("picture:{}", file.id))
                            .w(px(shown.width))
                            .h(px(shown.height))
                            .cursor_pointer()
                            .track_focus(&focus.clone().tab_stop(true))
                            .focus_visible(|style| style.border_color(look.color(token::FOCUS_RING)))
                            .child(img(image).size_full().object_fit(ObjectFit::Contain))
                            .on_click(cx.listener(move |this, _, window, cx| this.open_picture(pressed.0.clone(), natural, pressed.1.clone(), window, cx)))
                            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    cx.stop_propagation();
                                    this.open_picture(keyed.0.clone(), natural, keyed.1.clone(), window, cx);
                                }
                            })),
                        Role::Button,
                        name,
                    )
                    .into_any_element(),
                )
            }
        }
    }

    /// Start a frame: say whether the thumbnail holding the keys was left undrawn by the last one, as a
    /// picture scrolled away is, then forget what was drawn. Keys travel from the focused element.
    pub(super) fn picture_lost(&self, window: &Window) -> bool {
        let mut lost = false;
        for picture in self.pictures.held.values() {
            if let Picture::Ready { focus, seen, .. } = picture {
                lost |= focus.is_focused(window) && !seen.get();
                seen.set(false);
            }
        }
        lost
    }

    pub(super) fn open_picture(&mut self, image: Arc<Image>, natural: Size<f32>, name: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        let back = window.focused(cx);
        window.focus(&focus, cx);
        self.pictures.viewing = Some(Viewing { image, natural, name, focus, back, scale: 1., pan: point(0., 0.), press: None });
        cx.notify();
    }

    /// Esc, Close or a press beside the picture: it shuts and the keys go back to the thumbnail.
    fn close_picture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(viewing) = self.pictures.viewing.take() {
            if let Some(back) = viewing.back {
                window.focus(&back, cx);
            }
        }
        cx.notify();
    }

    /// The picture fitted inside the window less `--space-6` all round, never made larger than it is.
    fn fit_of(viewing: &Viewing, window: &Window, cx: &App) -> Size<f32> {
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let inset = f32::from(cx.look().length(token::SPACE_6)) * 2.;
        fitted(viewing.natural, size(f32::from(viewport.width) - inset, f32::from(viewport.height) - inset))
    }

    /// Zoom to `scale` about `at`, a point measured from the window's middle; the pan follows, held to the edges.
    fn zoom_about(&mut self, at: Point<f32>, scale: f32, window: &Window, cx: &mut Context<Self>) {
        let Some(fit) = self.pictures.viewing.as_ref().map(|viewing| Self::fit_of(viewing, window, cx)) else { return };
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let Some(viewing) = self.pictures.viewing.as_mut() else { return };
        let to = limited(scale);
        let by = to / viewing.scale;
        let pan = point(at.x - (at.x - viewing.pan.x) * by, at.y - (at.y - viewing.pan.y) * by);
        viewing.scale = to;
        viewing.pan = point(panned(pan.x, fit.width, to, f32::from(viewport.width)), panned(pan.y, fit.height, to, f32::from(viewport.height)));
        cx.notify();
    }

    fn pan_by(&mut self, by: Point<f32>, window: &Window, cx: &mut Context<Self>) {
        let Some(fit) = self.pictures.viewing.as_ref().map(|viewing| Self::fit_of(viewing, window, cx)) else { return };
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let Some(viewing) = self.pictures.viewing.as_mut() else { return };
        viewing.pan = point(
            panned(viewing.pan.x + by.x, fit.width, viewing.scale, f32::from(viewport.width)),
            panned(viewing.pan.y + by.y, fit.height, viewing.scale, f32::from(viewport.height)),
        );
        cx.notify();
    }

    fn zoom_by(&mut self, by: f32, window: &Window, cx: &mut Context<Self>) {
        let scale = self.pictures.viewing.as_ref().map_or(1., |viewing| viewing.scale);
        self.zoom_about(point(0., 0.), scale * by, window, cx);
    }

    /// The viewer's keys: Esc, `+` or `=`, `-`, `0`, and the arrows while zoomed.
    fn picture_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform || key.modifiers.control || key.modifiers.alt {
            return;
        }
        let zoomed = self.pictures.viewing.as_ref().is_some_and(|viewing| viewing.scale > 1.);
        match key.key.as_str() {
            "escape" => self.close_picture(window, cx),
            "+" | "=" => self.zoom_by(KEY_STEP, window, cx),
            "-" => self.zoom_by(1. / KEY_STEP, window, cx),
            "0" => self.zoom_about(point(0., 0.), 1., window, cx),
            "left" if zoomed => self.pan_by(point(KEY_PAN, 0.), window, cx),
            "right" if zoomed => self.pan_by(point(-KEY_PAN, 0.), window, cx),
            "up" if zoomed => self.pan_by(point(0., KEY_PAN), window, cx),
            "down" if zoomed => self.pan_by(point(0., -KEY_PAN), window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// A press let go without travelling: beside the picture at fit it closes; a second in a row toggles fit and 2.5x there.
    fn picture_tap(&mut self, at: Point<f32>, clicks: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(viewing) = self.pictures.viewing.as_ref() else { return };
        let fit = Self::fit_of(viewing, window, cx);
        let (w, h) = (fit.width * viewing.scale / 2., fit.height * viewing.scale / 2.);
        let on = (at.x - viewing.pan.x).abs() <= w && (at.y - viewing.pan.y).abs() <= h;
        if viewing.scale <= 1. && !on {
            self.close_picture(window, cx);
        } else if clicks >= 2 {
            let scale = if viewing.scale > 1. { 1. } else { DOUBLE_TAP_SCALE };
            self.zoom_about(at, scale, window, cx);
        }
    }

    /// The picture open over the whole window, with Zoom out, Zoom in and Close at its top right.
    pub(super) fn render_picture(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let viewing = self.pictures.viewing.as_ref()?;
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        let middle = point(f32::from(viewport.width) / 2., f32::from(viewport.height) / 2.);
        let fit = Self::fit_of(viewing, window, cx);
        let shown = size(fit.width * viewing.scale, fit.height * viewing.scale);
        let from_middle = move |at: Point<gpui::Pixels>| point(f32::from(at.x - room.origin.x) - middle.x, f32::from(at.y - room.origin.y) - middle.y);
        let button = |id: &'static str, icon: &'static str, said: &'static str, key: &'static str, act: fn(&mut Self, &mut Window, &mut Context<Self>)| {
            let key = gpui::Keystroke::parse(key).ok();
            labelled(
                div()
                    .id(id)
                    .size(look.length(token::H_CONTROL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph(look, icon, GlyphSize::Small, look.color(token::FG_DEFAULT)))
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))),
                Role::Button,
                said,
            )
        };
        let group = || {
            div().flex().rounded(look.length(token::RADIUS_MD)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_OVERLAY))
        };
        let bar = div()
            .absolute()
            .top(look.length(token::SPACE_3))
            .right(look.length(token::SPACE_3))
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(
                group()
                    .child(button("picture-zoom-out", "minus", "Zoom out", "-", |this, window, cx| this.zoom_by(1. / KEY_STEP, window, cx)))
                    .child(button("picture-zoom-in", "plus", "Zoom in", "+", |this, window, cx| this.zoom_by(KEY_STEP, window, cx))),
            )
            .child(group().child(button("picture-close", "x", "Close", "escape", |this, window, cx| this.close_picture(window, cx))));
        Some(
            deferred(
                anchored().position(room.origin).anchor(Anchor::TopLeft).child(labelled(
                    div()
                        .id("picture-view")
                        .occlude()
                        .track_focus(&viewing.focus)
                        .relative()
                        .overflow_hidden()
                        .w(viewport.width)
                        .h(viewport.height)
                        .bg(look.color(token::SCRIM))
                        .on_key_down(cx.listener(Self::picture_key))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, _| {
                            if let Some(viewing) = this.pictures.viewing.as_mut() {
                                let at = from_middle(event.position);
                                viewing.press = Some((at, at, false));
                            }
                        }))
                        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                            let Some((from, was, moved)) = this.pictures.viewing.as_ref().and_then(|viewing| viewing.press) else { return };
                            if event.pressed_button != Some(MouseButton::Left) {
                                return;
                            }
                            let now = from_middle(event.position);
                            let moved = moved || (now.x - from.x).hypot(now.y - from.y) > TAP_SLOP;
                            if let Some(viewing) = this.pictures.viewing.as_mut() {
                                viewing.press = Some((from, now, moved));
                            }
                            if moved && this.pictures.viewing.as_ref().is_some_and(|viewing| viewing.scale > 1.) {
                                this.pan_by(point(now.x - was.x, now.y - was.y), window, cx);
                            }
                        }))
                        .on_mouse_up(MouseButton::Left, cx.listener(move |this, event: &MouseUpEvent, window, cx| {
                            let Some((_, _, moved)) = this.pictures.viewing.as_mut().and_then(|viewing| viewing.press.take()) else { return };
                            if !moved {
                                this.picture_tap(from_middle(event.position), event.click_count, window, cx);
                            }
                        }))
                        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, window, cx| {
                            let delta = match event.delta {
                                ScrollDelta::Pixels(by) => f32::from(by.y),
                                ScrollDelta::Lines(by) => by.y * LINE_HEIGHT,
                            };
                            // gpui's wheel is the page's moving, the DOM's `deltaY` the finger's: the sign turns.
                            let speed = if event.modifiers.control { PINCH_SPEED } else { WHEEL_SPEED };
                            let scale = this.pictures.viewing.as_ref().map_or(1., |viewing| viewing.scale);
                            this.zoom_about(from_middle(event.position), scale * (delta * speed).exp(), window, cx);
                        }))
                        .child(
                            div()
                                .absolute()
                                .left(px(middle.x - shown.width / 2. + viewing.pan.x))
                                .top(px(middle.y - shown.height / 2. + viewing.pan.y))
                                .w(px(shown.width))
                                .h(px(shown.height))
                                .overflow_hidden()
                                .rounded(look.length(token::RADIUS_MD))
                                .shadow(look.shadows(token::SHADOW_OVERLAY))
                                .child(img(viewing.image.clone()).size_full().object_fit(ObjectFit::Contain)),
                        )
                        .child(bar),
                    Role::Dialog,
                    viewing.name.clone(),
                )),
            )
            .with_priority(3),
        )
    }
}

/// The thumbnail's box: `max-width: calc(var(--space-12) * 4)`, `max-height: calc(var(--space-12) * 3)`.
fn thumb_most(look: &Look) -> Size<f32> {
    let unit = f32::from(look.length(token::SPACE_12));
    size(unit * 4., unit * 3.)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_fits_the_thumbnail_keeping_its_shape_and_is_never_made_larger() {
        let most = size(192., 144.);
        // The scale is a float divide, so a fit lands within a hair of the whole pixel.
        let fit = |picture: Size<f32>| {
            let got = fitted(picture, most);
            size((got.width * 100.).round() / 100., (got.height * 100.).round() / 100.)
        };
        assert_eq!(fit(size(2880., 1800.)), size(192., 120.));
        assert_eq!(fit(size(600., 1200.)), size(72., 144.));
        assert_eq!(fit(size(100., 50.)), size(100., 50.));
    }

    #[test]
    fn a_zoomed_picture_moves_only_as_far_as_its_edges_and_one_that_fits_stays_in_the_middle() {
        assert_eq!(panned(500., 400., 2., 1000.), 0.);
        assert_eq!(panned(500., 800., 2., 1000.), 300.);
        assert_eq!(panned(-500., 800., 2., 1000.), -300.);
        assert_eq!(limited(20.), MOST);
        assert_eq!(limited(0.5), 1.);
    }

    #[test]
    fn a_pictures_size_is_read_from_its_header() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(1440u32.to_be_bytes());
        png.extend(900u32.to_be_bytes());
        assert_eq!(natural_size(&png), Some((1440, 900)));
        let gif = [b"GIF89a".as_slice(), &640u16.to_le_bytes(), &480u16.to_le_bytes()].concat();
        assert_eq!(natural_size(&gif), Some((640, 480)));
        // A JFIF segment, then the frame header: precision, height 300, width 400.
        let jpeg = [&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00][..], &[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01, 0x2C, 0x01, 0x90, 0x03][..]].concat();
        assert_eq!(natural_size(&jpeg), Some((400, 300)));
        assert_eq!(natural_size(b"RIFF....WEBP"), None);
        // A lossless WebP, 300 by 200: the width less one in 14 bits, then the height less one.
        let bits: u32 = 299 | 199 << 14;
        let webp = [b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".as_slice(), &bits.to_le_bytes()].concat();
        assert_eq!(natural_size(&webp), Some((300, 200)));
        assert!(mock_picture("Pasted file.png").is_some_and(|bytes| natural_size(&bytes) == Some((960, 600))));
        assert!(mock_picture("Wide index.png").is_some_and(|bytes| natural_size(&bytes) == Some((1600, 400))));
        assert!(mock_picture("Tall phone.png").is_some_and(|bytes| natural_size(&bytes) == Some((480, 1440))));
    }
}
