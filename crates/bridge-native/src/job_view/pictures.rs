//! The pictures his messages carried, drawn in the bubble under the words as Electron's `Enlargeable`
//! thumbnails (`SessionThread.tsx` `Sent`), and opened larger over the window on a press. A picture is read
//! from Fleet on its row's first draw (`get_session_file`) and kept, so a thread read again after a restart
//! draws it the same.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    anchored, deferred, div, img, point, prelude::*, px, size, Anchor, AnyElement, Context, FocusHandle, Image, ImageFormat, KeyDownEvent,
    MouseButton, ObjectFit, Role, SharedString, Size, Window,
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
    Ready { image: Arc<Image>, natural: Size<f32> },
    /// Fleet would not give it: the file is named in a chip instead.
    Unread,
}

/// The picture open over the window.
pub(super) struct Viewing {
    image: Arc<Image>,
    natural: Size<f32>,
    name: SharedString,
    focus: FocusHandle,
    back: Option<FocusHandle>,
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

/// A picture's width and height, read from its header: PNG, GIF and JPEG. Anything else is drawn in the
/// thumbnail's whole box.
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

/// What a mock thread's pictures read as, with no Fleet to ask: a window with a sidebar and a few rows.
fn mock_picture() -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};
    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="960" height="600" viewBox="0 0 960 600">
<rect width="960" height="600" rx="18" fill="#0f141b"/>
<rect width="960" height="52" rx="18" fill="#1a212b"/><rect y="34" width="960" height="18" fill="#1a212b"/>
<circle cx="30" cy="26" r="7" fill="#ff5f57"/><circle cx="54" cy="26" r="7" fill="#febc2e"/><circle cx="78" cy="26" r="7" fill="#28c840"/>
<rect x="20" y="72" width="230" height="508" rx="12" fill="#161c25"/>
<rect x="36" y="92" width="198" height="34" rx="8" fill="#23406a"/>
<rect x="36" y="140" width="170" height="14" rx="7" fill="#2b3442"/><rect x="36" y="172" width="150" height="14" rx="7" fill="#2b3442"/>
<rect x="36" y="204" width="182" height="14" rx="7" fill="#2b3442"/><rect x="36" y="236" width="120" height="14" rx="7" fill="#2b3442"/>
<rect x="270" y="72" width="670" height="508" rx="12" fill="#161c25"/>
<rect x="300" y="104" width="380" height="20" rx="10" fill="#3a4556"/>
<rect x="300" y="148" width="600" height="12" rx="6" fill="#2b3442"/><rect x="300" y="172" width="560" height="12" rx="6" fill="#2b3442"/>
<rect x="300" y="196" width="590" height="12" rx="6" fill="#2b3442"/>
<rect x="300" y="240" width="610" height="170" rx="12" fill="#1d2633" stroke="#e5a03a" stroke-width="3"/>
<rect x="324" y="268" width="240" height="14" rx="7" fill="#e5a03a"/><rect x="324" y="300" width="520" height="12" rx="6" fill="#3a4556"/>
<rect x="324" y="324" width="480" height="12" rx="6" fill="#3a4556"/><rect x="324" y="360" width="120" height="30" rx="8" fill="#3b82f6"/>
<rect x="300" y="440" width="610" height="110" rx="14" fill="#11161d" stroke="#2b3442" stroke-width="2"/>
</svg>"##;
    let tree = usvg::Tree::from_str(SVG, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(960, 600)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

impl JobView {
    /// Read the pictures among `files` that have not been asked for yet. The row draws a placeholder until one comes.
    pub(super) fn want_pictures(&mut self, job: &SharedString, files: &[SentFile], window: &mut Window, cx: &mut Context<Self>) {
        for file in files.iter().filter(|file| is_picture(file)) {
            let held = key(job, &file.id);
            if self.pictures.held.contains_key(&held) {
                continue;
            }
            self.pictures.held.insert(held.clone(), Picture::Reading);
            if self.fleet.is_none() {
                // Asked in the middle of the list's own layout: the answer lands after it, as Fleet's would.
                let media_type = file.media_type.clone();
                cx.spawn(async move |this, cx| {
                    let _ = this.update(cx, |this, cx| this.keep_picture(held, &media_type, mock_picture(), cx));
                })
                .detach();
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
                Picture::Ready { image: Arc::new(Image::from_bytes(format, bytes)), natural }
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
            Some(Picture::Ready { image, natural }) => {
                let shown = fitted(*natural, most);
                let (image, natural, name) = (image.clone(), *natural, SharedString::from(file.name.clone()));
                let (pressed, keyed) = ((image.clone(), name.clone()), (image.clone(), name.clone()));
                Some(
                    labelled(
                        tile(format!("picture:{}", file.id))
                            .w(px(shown.width))
                            .h(px(shown.height))
                            .cursor_pointer()
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|style| style.border_color(look.color(token::FOCUS_RING)))
                            .child(img(image).size_full().object_fit(ObjectFit::Cover))
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

    fn open_picture(&mut self, image: Arc<Image>, natural: Size<f32>, name: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        let back = window.focused(cx);
        window.focus(&focus, cx);
        self.pictures.viewing = Some(Viewing { image, natural, name, focus, back });
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

    /// The picture open over the whole window, fitted to it, never made larger than it is.
    pub(super) fn render_picture(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let viewing = self.pictures.viewing.as_ref()?;
        let look = cx.look();
        let viewport = window.viewport_size();
        let room = size(f32::from(viewport.width) * 0.9, f32::from(viewport.height) * 0.85);
        let shown = fitted(viewing.natural, room);
        let close = labelled(
            div()
                .id("picture-close")
                .absolute()
                .top(look.length(token::SPACE_4))
                .right(look.length(token::SPACE_4))
                .size(look.length(token::H_CONTROL_LG))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph(look, "x", GlyphSize::Small, look.color(token::FG_DEFAULT)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, window, cx| this.close_picture(window, cx))),
            Role::Button,
            "Close",
        );
        Some(
            deferred(
                anchored().position(point(px(0.), px(0.))).anchor(Anchor::TopLeft).child(labelled(
                    div()
                        .id("picture-view")
                        .occlude()
                        .track_focus(&viewing.focus)
                        .w(viewport.width)
                        .h(viewport.height)
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(look.color(token::SCRIM))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                this.close_picture(window, cx);
                            }
                        }))
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_picture(window, cx)))
                        .child(
                            div()
                                .w(px(shown.width))
                                .h(px(shown.height))
                                .overflow_hidden()
                                .rounded(look.length(token::RADIUS_LG))
                                .child(img(viewing.image.clone()).size_full().object_fit(ObjectFit::Contain)),
                        )
                        .child(close),
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
        assert_eq!(fitted(size(2880., 1800.), most), size(192., 120.));
        assert_eq!(fitted(size(600., 1200.), most), size(72., 144.));
        assert_eq!(fitted(size(100., 50.), most), size(100., 50.));
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
        assert!(mock_picture().is_some_and(|bytes| natural_size(&bytes) == Some((960, 600))));
    }
}
