//! The pictures his messages carried, drawn in the bubble under the words as Electron's `Enlargeable`
//! thumbnails (`SessionThread.tsx` `Sent`), and opened larger over the window on a press, where it zooms with the one
//! helper every picture view uses (`zoomer.rs`). A picture is read from Fleet on its row's first draw
//! (`get_session_file`) and kept, so a thread read again after a restart draws it the same.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{anchored, deferred, div, img, prelude::*, px, size, Anchor, AnyElement, App, Context, FocusHandle, Image, ImageFormat, KeyDownEvent, MouseButton, ObjectFit, Role, SharedString, Size, Window};
use ipc::SentFile;

use super::rows::{glyph, Size as GlyphSize};
use super::wire::{encoded, Done};
use super::zoomer::{self, Stage, Zoomer, Zooms};
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

/// The picture open over the window, and how far it is zoomed and moved.
pub(super) struct Viewing {
    image: Arc<Image>,
    natural: Size<f32>,
    name: SharedString,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    zoom: Zoomer,
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

/// Where a picture is read from: a file he sent, the Job's own tree, or a file the Job showed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum From {
    Sent,
    Tree,
    Shown,
}

/// Where Fleet serves a file the Job showed, by the path its ledger keeps: a picture's bytes under its type.
pub(super) fn shown_path(job: &str, path: &str) -> String {
    format!("/sessions/shown?session_id={}&path={}", encoded(job), encoded(path))
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

pub(super) use super::zoomer::fitted;

/// What a mock thread's pictures read as, with no Fleet to ask: a window with a sidebar and a few rows, in
/// the shape its name says (`Wide`, `Tall`), else 960 by 600.
pub(super) fn mock_picture(name: &str) -> Option<Vec<u8>> {
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
        self.want(job, files, From::Sent, window, cx);
    }

    /// Read a picture the Job showed (`show_file`), as Fleet keeps it (`GET /sessions/shown`).
    pub(super) fn want_shown_pictures(&mut self, job: &SharedString, files: &[SentFile], window: &mut Window, cx: &mut Context<Self>) {
        self.want(job, files, From::Shown, window, cx);
    }

    /// Read the pictures `Read` calls looked at, from the Job's own tree (`GET /sessions/repository_file`).
    /// A refusal, or a file that came back as text, leaves the row without one.
    pub(super) fn want_read_pictures(&mut self, job: &SharedString, files: &[SentFile], window: &mut Window, cx: &mut Context<Self>) {
        self.want(job, files, From::Tree, window, cx);
    }

    fn want(&mut self, job: &SharedString, files: &[SentFile], from: From, window: &mut Window, cx: &mut Context<Self>) {
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
            if from == From::Tree {
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
            let path = match from {
                From::Shown => shown_path(job, &file.id),
                _ => format!("/sessions/file?session_id={}&file={}", encoded(job), encoded(&file.id)),
            };
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
        self.pictures.viewing = Some(Viewing { image, natural, name, focus, back, zoom: Zoomer::default() });
        cx.notify();
    }

    /// Esc or Close: it shuts and the keys go back to the thumbnail.
    fn close_picture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(viewing) = self.pictures.viewing.take() {
            if let Some(back) = viewing.back {
                window.focus(&back, cx);
            }
        }
        cx.notify();
    }

    /// The viewer's keys: Esc, and the zoom's own.
    fn picture_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.key == "escape" && !key.modifiers.modified() {
            self.close_picture(window, cx);
        } else if !zoomer::keyed(self, key, window, cx) {
            return;
        }
        cx.stop_propagation();
    }

    /// The picture open over the whole window, with Zoom out, Zoom in and Close at its top right and the zoom's corner at the foot.
    pub(super) fn render_picture(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let viewing = self.pictures.viewing.as_ref()?;
        let look = cx.look();
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        // The first frame is the room's own size; the probe inside has the view's from then on.
        let view = viewing.zoom.view().unwrap_or(size(f32::from(viewport.width), f32::from(viewport.height)));
        let stage = Stage::of_picture(viewing.natural, view, f32::from(look.length(token::SPACE_6)));
        let shown = size(stage.fit.width * viewing.zoom.scale, stage.fit.height * viewing.zoom.scale);
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
            .child(group().child(button("picture-close", "x", "Close", "escape", |this, window, cx| this.close_picture(window, cx))));
        Some(
            deferred(
                anchored().position(room.origin).anchor(Anchor::TopLeft).child(labelled(
                    zoomer::bind(
                        div()
                            .id("picture-view")
                            .occlude()
                            .track_focus(&viewing.focus)
                            .relative()
                            .overflow_hidden()
                            .w(viewport.width)
                            .h(viewport.height)
                            .bg(look.color(token::BG_SUNKEN))
                            .on_key_down(cx.listener(Self::picture_key)),
                        cx,
                        |_, _, _, _| {},
                    )
                    .child(viewing.zoom.probe())
                    .child(
                        div()
                            .absolute()
                            .left(px(view.width / 2. - shown.width / 2. + viewing.zoom.pan.x))
                            .top(px(view.height / 2. - shown.height / 2. + viewing.zoom.pan.y))
                            .w(px(shown.width))
                            .h(px(shown.height))
                            .overflow_hidden()
                            .child(img(viewing.image.clone()).size_full().object_fit(ObjectFit::Contain)),
                    )
                    .child(bar)
                    .child(zoomer::corner(look, viewing.zoom.readout(stage), |this, _, cx| zoomer::refit(this, cx), cx)),
                    Role::Dialog,
                    viewing.name.clone(),
                )),
            )
            .with_priority(3),
        )
    }
}

impl Zooms for JobView {
    fn zoomer(&mut self) -> Option<&mut Zoomer> {
        if let Some(viewing) = self.pictures.viewing.as_mut() {
            return Some(&mut viewing.zoom);
        }
        self.sketch_reading.as_mut().map(|reading| &mut reading.zoom)
    }

    fn stage(&self, view: Size<f32>, _: &Window, cx: &App) -> Option<Stage> {
        if let Some(viewing) = self.pictures.viewing.as_ref() {
            return Some(Stage::of_picture(viewing.natural, view, f32::from(cx.look().length(token::SPACE_6))));
        }
        let reading = self.sketch_reading.as_ref()?;
        let (fit, actual) = crate::scene::fit_of(&Self::drawn_key(reading))?;
        Some(Stage { view, fit, actual })
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
    fn a_picture_the_job_showed_is_asked_of_fleet_at_its_ledger_path() {
        let shown = "/Users/me/Library/Application Support/Armada/attachments/pictures/j/c70e.png";
        assert_eq!(
            shown_path("j-1", shown),
            "/sessions/shown?session_id=j-1&path=%2FUsers%2Fme%2FLibrary%2FApplication%20Support%2FArmada%2Fattachments%2Fpictures%2Fj%2Fc70e.png"
        );
        assert_eq!(read_picture(shown).map(|file| file.media_type), Some("image/png".to_string()));
        assert!(read_picture("/Users/me/notes.md").is_none());
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
