//! A drawn sketch as the upload a message carries (`session-uploads.ts`): its picture as a PNG, or its words where no picture can be drawn. Electron draws on a canvas; here the same picture is built as SVG and rasterised with resvg.
use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use base64::Engine as _;
use ipc::SessionUpload;
use resvg::{tiny_skia, usvg};

use crate::sketch::{Drawing, DrawnSketch};

const BOX_WIDTH: f32 = 160.0;
const LINE: f32 = 16.0;
const PAD: f32 = 12.0;
const MARGIN: f32 = 24.0;
const LINE_WIDTH: f32 = 1.5;
/// CSS `small`, the size Electron's canvas sets its font to.
const FONT_SIZE: f32 = 13.0;
/// Where a canvas stops drawing; past it Electron's `toBlob` gives nothing, so the words go instead.
const MOST_PIXELS: f32 = 16384.0;

/// A text width with no canvas to measure on: an even 0.6 of the font size a character, close enough for a sans face to wrap where Electron's would.
fn width_of(text: &str) -> f32 {
    text.chars().count() as f32 * FONT_SIZE * 0.6
}

/// The words wrapped to a box's width, as `wrapped` does: a word that does not fit starts the next line, and a word too long for a line keeps one to itself.
fn wrapped(body: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in body.split_whitespace() {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && width_of(&next) > BOX_WIDTH - PAD * 2.0 {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The sketch as words: each box, and what it joins. What goes where no picture can be drawn.
pub(crate) fn describe(drawing: &Drawing) -> String {
    // A later box with the same id wins, as it does in the Map the TS builds.
    let said = |id: &str| -> Option<String> {
        drawing.shapes.iter().rev().find(|one| one.id == id).map(|one| match one.body.trim() {
            "" => "(empty box)".to_string(),
            body => body.to_string(),
        })
    };
    let mut lines: Vec<String> = drawing.shapes.iter().map(|one| format!("Box: {}", said(&one.id).unwrap_or_default())).collect();
    for join in &drawing.joins {
        lines.push(format!("Join: {} -> {}", said(&join.from).unwrap_or_else(|| join.from.clone()), said(&join.to).unwrap_or_else(|| join.to.clone())));
    }
    if !drawing.strokes.is_empty() {
        lines.push(format!("{} lines drawn by hand", drawing.strokes.len()));
    }
    lines.join("\n")
}

/// Text for an XML body: the five reserved characters escaped, and the control characters XML cannot carry dropped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// The system's fonts, loaded once: scanning them takes far longer than drawing a sketch.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// A box with the lines its words wrap to, and the height they make.
struct Laid<'a> {
    id: &'a str,
    x: f32,
    y: f32,
    lines: Vec<String>,
    height: f32,
}

/// The sketch drawn to SVG, and the size of the picture it makes. `None` where there is nothing on the pad to draw, or it is too large for a picture.
fn svg_of(drawing: &Drawing) -> Option<(String, u32, u32)> {
    let boxes: Vec<Laid> = drawing
        .shapes
        .iter()
        .map(|one| {
            let lines = wrapped(&one.body);
            let height = PAD * 2.0 + lines.len().max(1) as f32 * LINE;
            Laid { id: &one.id, x: one.x, y: one.y, lines, height }
        })
        .collect();
    let mut points: Vec<(f32, f32)> = Vec::new();
    for one in &boxes {
        points.push((one.x, one.y));
        points.push((one.x + BOX_WIDTH, one.y + one.height));
    }
    points.extend(drawing.strokes.iter().flat_map(|one| one.points.iter().map(|p| (p.x, p.y))));
    if points.is_empty() {
        return None;
    }
    let least = |pick: fn(&(f32, f32)) -> f32| points.iter().map(pick).fold(f32::INFINITY, f32::min);
    let most = |pick: fn(&(f32, f32)) -> f32| points.iter().map(pick).fold(f32::NEG_INFINITY, f32::max);
    let left = least(|p| p.0) - MARGIN;
    let top = least(|p| p.1) - MARGIN;
    let (width, height) = ((most(|p| p.0) + MARGIN - left).ceil(), (most(|p| p.1) + MARGIN - top).ceil());
    if !(width >= 1.0 && height >= 1.0 && width <= MOST_PIXELS && height <= MOST_PIXELS) {
        return None;
    }

    let mut svg = String::new();
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="{width}" height="{height}" fill="white"/>"#
    );
    let _ = write!(svg, r#"<g fill="none" stroke="black" stroke-width="{LINE_WIDTH}">"#);
    // Joins run centre to centre, under the strokes and the boxes; one to a box that is not there is left out.
    let centre = |id: &str| boxes.iter().find(|one| one.id == id).map(|one| (one.x + BOX_WIDTH / 2.0 - left, one.y + one.height / 2.0 - top));
    for join in &drawing.joins {
        if let (Some(from), Some(to)) = (centre(&join.from), centre(&join.to)) {
            let _ = write!(svg, r#"<line x1="{}" y1="{}" x2="{}" y2="{}"/>"#, from.0, from.1, to.0, to.1);
        }
    }
    for stroke in &drawing.strokes {
        if stroke.points.is_empty() {
            continue;
        }
        let path: Vec<String> = stroke.points.iter().map(|p| format!("{},{}", p.x - left, p.y - top)).collect();
        let _ = write!(svg, r#"<polyline points="{}"/>"#, path.join(" "));
    }
    svg.push_str("</g>");
    for one in &boxes {
        let (x, y) = (one.x - left, one.y - top);
        let _ = write!(svg, r#"<rect x="{x}" y="{y}" width="{BOX_WIDTH}" height="{}" fill="white" stroke="black" stroke-width="{LINE_WIDTH}"/>"#, one.height);
        for (at, line) in one.lines.iter().enumerate() {
            let _ = write!(
                svg,
                r#"<text x="{}" y="{}" font-family="Helvetica, Arial, sans-serif" font-size="{FONT_SIZE}" fill="black" xml:space="preserve">{}</text>"#,
                x + PAD,
                y + PAD + (at + 1) as f32 * LINE - 4.0,
                escaped(line)
            );
        }
    }
    svg.push_str("</svg>");
    Some((svg, width as u32, height as u32))
}

/// The sketch drawn to a PNG, or `None` where there is nothing to draw or it would not draw.
pub(crate) fn picture(drawing: &Drawing) -> Option<Vec<u8>> {
    let (svg, width, height) = svg_of(drawing)?;
    let options = usvg::Options { fontdb: fonts(), ..usvg::Options::default() };
    let tree = usvg::Tree::from_str(&svg, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().ok()
}

/// A title as a file name: anything but a letter, digit, underscore, space, dot or dash becomes `_`. Electron replaces each UTF-16 unit, so a character outside the basic plane becomes two.
fn file_name(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | ' ' | '.' | '-') { c.to_string() } else { "_".repeat(c.len_utf16()) })
        .collect()
}

/// A drawn sketch as an upload: its picture, or its words where there is nothing to draw on.
pub(crate) fn upload(sketch: &DrawnSketch) -> SessionUpload {
    let engine = base64::engine::general_purpose::STANDARD;
    let name = file_name(&sketch.title);
    match picture(&sketch.drawing) {
        Some(png) => SessionUpload { name: format!("{name}.png"), media_type: "image/png".to_string(), data: engine.encode(png) },
        None => SessionUpload { name: format!("{name}.txt"), media_type: "text/plain".to_string(), data: engine.encode(describe(&sketch.drawing)) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::{attach, Join, Point, Shape, Stroke};

    fn shape(id: &str, x: f32, y: f32, body: &str) -> Shape {
        Shape { id: id.into(), x, y, body: body.into() }
    }

    /// Two boxes and the join between them, which is the smallest real pad.
    fn pad() -> Drawing {
        Drawing {
            shapes: vec![shape("b1", 0.0, 0.0, "the stat"), shape("b2", 280.0, 0.0, "the panel it opens")],
            joins: vec![Join { id: "b1-b2".into(), from: "b1".into(), to: "b2".into() }],
            strokes: vec![],
        }
    }

    #[test]
    fn words_wrap_to_the_box_and_a_long_word_keeps_a_line() {
        assert_eq!(wrapped("the stat"), ["the stat"]);
        assert_eq!(wrapped("the panel it opens"), ["the panel it", "opens"]);
        assert_eq!(wrapped("  a\n\tb  "), ["a b"]);
        assert_eq!(wrapped("short unbreakable_word_longer_than_a_line"), ["short", "unbreakable_word_longer_than_a_line"]);
        assert!(wrapped("   ").is_empty());
    }

    #[test]
    fn the_words_say_each_box_then_each_join() {
        assert_eq!(describe(&pad()), "Box: the stat\nBox: the panel it opens\nJoin: the stat -> the panel it opens");
    }

    #[test]
    fn the_words_name_an_empty_box_and_count_hand_drawn_lines() {
        let mut drawing = Drawing { shapes: vec![shape("b1", 0.0, 0.0, "  ")], ..Drawing::default() };
        drawing.joins.push(Join { id: "b1-b9".into(), from: "b1".into(), to: "b9".into() });
        drawing.strokes.push(Stroke { id: "s1".into(), points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 1.0 }] });
        assert_eq!(describe(&drawing), "Box: (empty box)\nJoin: (empty box) -> b9\n1 lines drawn by hand");
        assert_eq!(describe(&Drawing::default()), "");
    }

    #[test]
    fn two_boxes_and_a_join_make_a_png_of_the_size_the_canvas_would() {
        let png = picture(&pad()).expect("a picture");
        let decoded = image::load_from_memory(&png).expect("a decodable PNG");
        // x spans 0 to 440 and y 0 to 56 (the second box wraps to two lines), plus a 24 margin each side.
        assert_eq!((decoded.width(), decoded.height()), (488, 104));
        let white = decoded.to_rgba8().get_pixel(0, 0).0;
        assert_eq!(white, [255, 255, 255, 255]);
    }

    #[test]
    fn a_stroke_alone_makes_a_picture_and_a_join_alone_does_not() {
        let stroke = Drawing { strokes: vec![Stroke { id: "s1".into(), points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 50.0, y: 30.0 }] }], ..Drawing::default() };
        let decoded = image::load_from_memory(&picture(&stroke).unwrap()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (88, 68));
        let joins_only = Drawing { joins: pad().joins, ..Drawing::default() };
        assert!(picture(&joins_only).is_none());
        assert!(picture(&Drawing::default()).is_none());
    }

    #[test]
    fn markup_in_a_box_cannot_break_the_picture() {
        let drawing = Drawing { shapes: vec![shape("b1", 0.0, 0.0, "a < b & \"c\" </text><g> \u{1}")], ..Drawing::default() };
        assert!(picture(&drawing).is_some());
        assert_eq!(escaped("a<b&\"'\u{1}\n"), "a&lt;b&amp;&quot;&apos;\n");
    }

    #[test]
    fn a_picture_goes_up_as_a_png_named_for_its_title() {
        let sketch = attach(&pad(), "d1".into());
        let up = upload(&sketch);
        assert_eq!(up.name, "the stat.png");
        assert_eq!(up.media_type, "image/png");
        let bytes = base64::engine::general_purpose::STANDARD.decode(&up.data).unwrap();
        assert!(image::load_from_memory(&bytes).is_ok());
    }

    #[test]
    fn nothing_to_draw_goes_up_as_words() {
        let sketch = DrawnSketch { id: "d1".into(), title: "Sketch".into(), drawing: Drawing { joins: pad().joins, ..pad() } };
        let words_only = DrawnSketch { drawing: Drawing { joins: pad().joins, ..Drawing::default() }, ..sketch };
        let up = upload(&words_only);
        assert_eq!(up.name, "Sketch.txt");
        assert_eq!(up.media_type, "text/plain");
        let bytes = base64::engine::general_purpose::STANDARD.decode(&up.data).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), describe(&words_only.drawing));
    }

    #[test]
    fn a_title_becomes_a_file_name_as_the_regex_makes_it() {
        assert_eq!(file_name("the stat: v2/b.c-d_e"), "the stat_ v2_b.c-d_e");
        // One underscore for the accent, two for the emoji, as JS counts UTF-16 units.
        assert_eq!(file_name("é😀"), "___");
    }
}
