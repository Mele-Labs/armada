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

/// A part of a page a person can point at, where the page laid it out, in the page's own points: the nearest element
/// carrying `data-part`, else the nearest with words (`partOf` in `mock-frame.ts`).
#[derive(Clone, Debug, PartialEq)]
pub struct PagePart {
    /// Where the part is found again: its `data-part` (with its place among the same name when the name repeats), else its path from the body.
    pub key: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// How many elements stand between it and the body, so the innermost part under a point is the one named.
    pub depth: u32,
}

/// One picture of an HTML string and how tall its content is.
pub struct Rendered {
    /// The page's pixels, `width * scale` across and `height * scale` down, ready for `paint_image` or `img`.
    pub image: Arc<RenderImage>,
    /// The content's height in points: the picture is drawn at this height for the width asked.
    pub height: f32,
    /// What the page holds that a note may name, with where each stands.
    pub parts: Vec<PagePart>,
}

/// Asks the page for its parts, as `partOf`, `partKey` and `partName` of `mock-frame.ts` name them: every element with a
/// `data-part` or words of its own, once for its part. The answer is JSON.
const PARTS: &str = r#"(function(){var b=document.body,out=[],seen={};
var words=function(e){return (e.textContent||'').replace(/\s+/g,' ').trim();};
var own=function(e){for(var c=e.firstChild;c;c=c.nextSibling){if(c.nodeType===3&&(c.textContent||'').trim()!=='')return true;}return false;};
var inside=function(e){return e!==null&&e!==b&&e!==document.documentElement;};
var partOf=function(e){var a;for(a=e;inside(a);a=a.parentElement){if(a.hasAttribute('data-part'))return a;}for(a=e;inside(a);a=a.parentElement){if(words(a)!=='')return a;}return null;};
var key=function(e){var n=e.getAttribute('data-part');if(n!==null){var same=[].filter.call(b.querySelectorAll('[data-part]'),function(o){return o.getAttribute('data-part')===n;});return same.length>1?n+'['+(same.indexOf(e)+1)+']':n;}var path=[];for(var a=e;a&&a!==b;a=a.parentElement){path.unshift(a.tagName.toLowerCase()+':nth-child('+([].indexOf.call(a.parentElement.children,a)+1)+')');}return ['body'].concat(path).join(' > ');};
var label=function(e){var n=e.getAttribute('data-part');if(n!==null)return n;var s=words(e);return s.length>40?s.slice(0,40).replace(/\s+$/,'')+'\u2026':s;};
[].forEach.call(b.querySelectorAll('*'),function(e){if(out.length>=400)return;if(!e.hasAttribute('data-part')&&!own(e))return;var p=partOf(e);if(p===null)return;var k=key(p);if(seen[k])return;seen[k]=1;var r=p.getBoundingClientRect();if(r.width<1||r.height<1)return;var d=0;for(var a=p;a&&a!==b;a=a.parentElement)d++;out.push({k:k,l:label(p),x:r.left+window.scrollX,y:r.top+window.scrollY,w:r.width,h:r.height,d:d});});
return JSON.stringify(out);})()"#;

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
    let options = Options { store: None, scripts: false, sealed: true, dark: Some(appearance == Appearance::Dark), clear: true, kit: None };
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
            Ok(Nav::Cancelled) => return Err("cancelled".into()),
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
        let parts = parts_of(&native).await;
        let to = ((width * scale.max(0.5)).round() as u32, (height * scale.max(0.5)).round() as u32);
        let image = cx.background_executor().spawn(async move { decode(&png, Some(to)) }).await.ok_or("the picture would not decode")?;
        Ok(Rendered { image, height, parts })
    })
}

/// A document printed to PDF, as Classic's `printToPdf`: `html` as it is (a sealed page carries its own policy), laid out
/// `width` points across in a view nobody sees with its scripts off, made as tall as its content, then the whole of it one
/// PDF. Off the main thread, or where the page or WebKit fails, it answers why.
pub fn pdf_of(html: String, width: f32, cx: &mut App) -> Task<Result<Vec<u8>, String>> {
    let (events, arrivals) = async_channel::unbounded();
    let options = Options { store: None, scripts: false, sealed: true, dark: None, clear: false, kit: None };
    let width = width.max(1.);
    let Some(native) = Native::new(&options, Box_::new(0., 0., f64::from(width), 1.), events) else {
        return Task::ready(Err("a web view needs the main thread".into()));
    };
    native.load_html(&html);
    cx.spawn(async move |_| {
        // Any end but Finished is no PDF; named this way it holds whatever other ends a load may gain.
        match arrivals.recv().await {
            Ok(Nav::Failed(why)) => return Err(why),
            Ok(arrived) if !matches!(arrived, Nav::Finished) => return Err("the page did not load".into()),
            Ok(_) => {}
            Err(_) => return Err("the page was dropped".into()),
        }
        let height = content_height(&native).await.max(1.);
        native.set_size(f64::from(width), f64::from(height));
        content_height(&native).await;
        let (tx, rx) = async_channel::bounded(1);
        native.pdf(move |pdf| {
            let _ = tx.try_send(pdf);
        });
        rx.recv().await.map_err(|_| "WebKit made no PDF".to_string())?
    })
}

async fn content_height(native: &Native) -> f32 {
    let (tx, rx) = async_channel::bounded(1);
    native.evaluate("String(Math.ceil(document.documentElement.scrollHeight))", move |answer| {
        let _ = tx.try_send(answer);
    });
    rx.recv().await.ok().flatten().and_then(|text| text.parse::<f32>().ok()).unwrap_or(0.)
}

async fn parts_of(native: &Native) -> Vec<PagePart> {
    let (tx, rx) = async_channel::bounded(1);
    native.evaluate(PARTS, move |answer| {
        let _ = tx.try_send(answer);
    });
    rx.recv().await.ok().flatten().map(|text| read_parts(&text)).unwrap_or_default()
}

/// The page's answer: a list of `{k, l, x, y, w, h, d}`. Anything that does not read is no parts, and a mock then takes a note as a whole.
fn read_parts(text: &str) -> Vec<PagePart> {
    #[derive(serde::Deserialize)]
    struct One {
        k: String,
        l: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        d: u32,
    }
    serde_json::from_str::<Vec<One>>(text).map_or_else(
        |_| Vec::new(),
        |all| all.into_iter().map(|one| PagePart { key: one.k, label: one.l, x: one.x, y: one.y, w: one.w, h: one.h, depth: one.d }).collect(),
    )
}

/// Takes a picture that is no longer shown off every window's GPU atlas. gpui keeps a painted `RenderImage`'s texture until
/// told, so a picture replaced without this stays on the GPU for the run: a tall page 960 points across is 120 MB. Deferred, so no
/// window is mid-update and every one is reached; a clone still drawn somewhere is uploaded again when next painted.
pub fn let_go(image: Arc<RenderImage>, cx: &mut App) {
    cx.defer(move |cx| cx.drop_image(image, None));
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
    fn the_pages_parts_are_read_and_a_bad_answer_is_none() {
        let read = read_parts(r#"[{"k":"chip","l":"chip","x":4,"y":2,"w":60.5,"h":20,"d":1}]"#);
        assert_eq!(read, vec![PagePart { key: "chip".into(), label: "chip".into(), x: 4., y: 2., w: 60.5, h: 20., depth: 1 }]);
        assert!(read_parts("not json").is_empty());
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
