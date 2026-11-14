//! One picture of an HTML string, drawn in a hidden web view: what a sketch's mock nodes are drawn from.
//!
//! **Only the HTML given draws.** The page runs no script of its own, may load nothing from anywhere (a
//! content security policy put ahead of the HTML refuses every request but the page's own text and `data:`
//! images and fonts), and may go nowhere (the navigation delegate refuses every navigation but the first).
//! Its view is never put in a window of anyone's screen.

use std::sync::Arc;

use gpui::{App, RenderImage, Task};
use image::{Frame, ImageFormat};

use super::frame::Frame as Box_;
use super::wk::{Nav, Native, Options};

/// The page's appearance, which its `prefers-color-scheme` answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

/// One picture of an HTML string and how tall its content is.
pub struct Rendered {
    /// The page's pixels, `width * scale` across and `height * scale` down, ready for `paint_image` or `img`.
    pub image: Arc<RenderImage>,
    /// The content's height in points: the picture is drawn at this height for the width asked.
    pub height: f32,
}

/// The policy put ahead of the HTML. Later policies can only narrow it, so the page cannot lift it.
const SEAL: &str = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:\">";

/// The HTML with the seal ahead of it.
pub fn sealed(html: &str) -> String {
    format!("{SEAL}{html}")
}

/// The tallest a picture is made, in points: a mock taller than this is cut, not left unbounded.
const TALLEST: f32 = 8000.;

/// Draws `html` once, `width` points across, at `scale` pixels to the point, in the given appearance.
/// Answers the picture and the content's height in points. **Async**: the web view loads on its own time,
/// and the answer comes on gpui's executor. Off the main thread, where WebKit cannot be used, it answers an error.
pub fn render_html(html: String, width: f32, scale: f32, appearance: Appearance, cx: &mut App) -> Task<Result<Rendered, String>> {
    let (events, arrivals) = async_channel::unbounded();
    let options = Options { store: None, scripts: false, sealed: true, dark: Some(appearance == Appearance::Dark), clear: true };
    let width = width.max(1.);
    // One point tall at first: `scrollHeight` is never less than the view, so a taller view would be every page's least height.
    let Some(native) = Native::new(&options, Box_::new(0., 0., f64::from(width), 1.), events) else {
        return Task::ready(Err("a web view needs the main thread".into()));
    };
    native.load_html(&sealed(&html));
    cx.spawn(async move |cx| {
        match arrivals.recv().await {
            Ok(Nav::Finished) => {}
            Ok(Nav::Failed(why)) => return Err(why),
            Err(_) => return Err("the page was dropped".into()),
        }
        let height = content_height(&native).await.min(TALLEST).max(1.);
        native.set_size(f64::from(width), f64::from(height));
        // Asking again lays the page out at its new size before the picture is taken.
        content_height(&native).await;
        let (tx, rx) = async_channel::bounded(1);
        native.snapshot(Some(Box_::new(0., 0., f64::from(width), f64::from(height))), f64::from(width), move |png| {
            let _ = tx.try_send(png);
        });
        let png = rx.recv().await.ok().flatten().ok_or("the page gave no picture")?;
        let to = ((width * scale.max(0.5)).round() as u32, (height * scale.max(0.5)).round() as u32);
        let image = cx.background_executor().spawn(async move { decode(&png, Some(to)) }).await.ok_or("the picture would not decode")?;
        Ok(Rendered { image, height })
    })
}

async fn content_height(native: &Native) -> f32 {
    let (tx, rx) = async_channel::bounded(1);
    native.evaluate("String(Math.ceil(document.documentElement.scrollHeight))", move |answer| {
        let _ = tx.try_send(answer);
    });
    rx.recv().await.ok().flatten().and_then(|text| text.parse::<f32>().ok()).unwrap_or(0.)
}

/// A PNG as the image gpui paints: BGRA, one frame. `to`: the size in pixels it is drawn at, where it is not the one WebKit
/// made (the screen's own scale: 2 on a Retina display). A smaller size is resampled down; a larger is not made up.
pub fn decode(png: &[u8], to: Option<(u32, u32)>) -> Option<Arc<RenderImage>> {
    let mut rgba = image::load_from_memory_with_format(png, ImageFormat::Png).ok()?.into_rgba8();
    if let Some((width, height)) = to.filter(|(width, height)| *width > 0 && *height > 0 && *width < rgba.width() && *height < rgba.height()) {
        rgba = image::imageops::resize(&rgba, width, height, image::imageops::FilterType::Triangle);
    }
    for pixel in rgba.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new(vec![Frame::new(rgba)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seal_stands_ahead_of_the_html_it_guards() {
        let html = sealed("<p>hello</p>");
        assert!(html.starts_with("<meta http-equiv=\"Content-Security-Policy\""));
        assert!(html.ends_with("<p>hello</p>"));
        assert!(html.contains("default-src 'none'"));
        assert!(!html.contains("script-src"), "scripts are refused by default-src and by the preferences, never allowed");
    }

    #[test]
    fn a_png_decodes_to_bgra() {
        let mut one = image::RgbaImage::new(1, 1);
        one.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
        let mut png = Vec::new();
        one.write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png).unwrap();
        let decoded = decode(&png, None).unwrap();
        assert_eq!(decoded.as_bytes(0).unwrap(), &[30, 20, 10, 255]);
    }
}
