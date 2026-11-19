//! The graph drawn: quads for the bars, the bands and the marks, a path for the context line, and text shaped where it is painted so a
//! label is measured against the band it stands over. The full chart (the Tokens tab) adds the bands with their labels, the compaction
//! marks, the cold-wake dots, the hour axis and the hover cursor; the small one (the Context card) is the bars and the line.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, px, quad, size, BorderStyle, Bounds, Hsla, IntoElement, PathBuilder, Pixels, SharedString, TextAlign, TextRun, Window};

use super::model::{fit, Graph, HEADROOM};
use crate::look::{token, Look};
use crate::settings::view::bone;
use crate::titlebar::parts::sans;

/// What the full chart keeps above and below its plot: a strip for the labels of the pauses, and the hour axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margins {
    pub strip: Pixels,
    pub axis: Pixels,
}

/// The margins the full chart of `graph` keeps at this look's type size, and none for the small one.
pub fn margins(graph: &Graph, full: bool, look: &Look) -> Margins {
    if !full {
        return Margins { strip: px(0.), axis: px(0.) };
    }
    let lead = look.length(token::LEADING_2XS);
    let gap = look.length(token::SPACE_1);
    Margins { strip: if graph.bands.is_empty() { gap } else { lead * 2. + gap * 2. }, axis: lead + gap * 2. }
}

fn quad_of(bounds: Bounds<Pixels>, fill: Hsla) -> gpui::PaintQuad {
    quad(bounds, px(0.), fill, px(0.), gpui::transparent_black(), BorderStyle::Solid)
}

/// The chart, as large as the relatively placed element it stands in. `at` receives the bounds the chart was laid out in, so a pointer over it can be read as a bucket.
pub fn chart(graph: Rc<Graph>, full: bool, cursor: Option<usize>, look: &Look, at: Rc<Cell<Option<Bounds<Pixels>>>>) -> impl IntoElement {
    let look = look.clone();
    canvas(
        move |bounds, _, _| at.set(Some(bounds)),
        move |bounds, _, window, cx| paint(&graph, full, cursor, &look, bounds, window, cx),
    )
    .absolute()
    .size_full()
}

fn paint(graph: &Graph, full: bool, cursor: Option<usize>, look: &Look, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut gpui::App) {
    let margin = margins(graph, full, look);
    let plot = Bounds::new(point(bounds.origin.x, bounds.origin.y + margin.strip), size(bounds.size.width, (bounds.size.height - margin.strip - margin.axis).max(px(1.))));
    let (w, h) = (f32::from(plot.size.width), f32::from(plot.size.height));
    let left = plot.origin.x;
    let bottom = plot.origin.y + plot.size.height;
    let hairline = look.length(token::BORDER_WIDTH);

    if full {
        for band in &graph.bands {
            let wash = if band.on_person { look.color(token::STATUS_AWAITING_REVIEW).opacity(0.16) } else { look.color(token::FG_MUTED).opacity(0.12) };
            let (x0, x1) = (left + px(band.x0 * w), left + px(band.x1 * w));
            window.paint_quad(quad_of(Bounds::new(point(x0, plot.origin.y), size((x1 - x0).max(hairline), plot.size.height)), wash));
        }
    }
    window.paint_quad(quad_of(Bounds::new(point(left, bottom - hairline), size(plot.size.width, hairline)), look.color(token::BORDER_SUBTLE)));

    if let Some(at) = cursor.filter(|_| full) {
        if let (Some(bar), Some(centre)) = (graph.bars.get(at), graph.centre_of(at)) {
            let column = Bounds::new(point(left + px(bar.x * w), plot.origin.y), size(px((bar.w * w).max(1.)), plot.size.height));
            window.paint_quad(quad_of(column, look.color(token::FG_DEFAULT).opacity(0.08)));
            window.paint_quad(quad_of(Bounds::new(point(left + px(centre * w) - hairline / 2., plot.origin.y), size(hairline, plot.size.height)), look.color(token::FG_DEFAULT).opacity(0.6)));
        }
    }

    for rect in graph.bar_rects(w, h) {
        let x = left + px(rect.x);
        let main = Bounds::new(point(x, bottom - px(rect.main_h) - hairline), size(px(rect.w), px(rect.main_h)));
        window.paint_quad(quad_of(main, look.color(token::SKETCH_BLUE)));
        if rect.sub_h > 0. {
            window.paint_quad(quad_of(Bounds::new(point(x, main.origin.y - px(rect.sub_h)), size(px(rect.w), px(rect.sub_h))), look.color(token::SKETCH_ORANGE)));
        }
    }

    let on_line = |across: f32, share: f32| point(left + px(across * w), bottom - hairline - px(share * h * HEADROOM));
    if graph.line.len() > 1 {
        let mut path = PathBuilder::stroke(look.length(token::BORDER_WIDTH) * 1.5);
        for (n, (across, share)) in graph.line.iter().enumerate() {
            let at = on_line(*across, *share);
            if n == 0 {
                path.move_to(at);
            } else {
                path.line_to(at);
            }
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, look.color(token::SKETCH_GREY));
        }
    }
    if !full {
        return;
    }

    // Compactions: a dashed rule each, and the first one named, in the room over the bars.
    let dash = look.length(token::SPACE_1);
    for across in &graph.marks {
        let x = left + px(across * w) - hairline / 2.;
        let mut y = plot.origin.y;
        while y < bottom {
            window.paint_quad(quad_of(Bounds::new(point(x, y), size(hairline, dash.min(bottom - y))), look.color(token::FG_SUBTLE)));
            y += dash * 1.75;
        }
    }
    let small = look.length(token::TEXT_2XS);
    let lead = look.length(token::LEADING_2XS);
    let face = sans(look);
    let system = window.text_system().clone();
    let shape = |said: &str, ink: Hsla| {
        let said = SharedString::from(said.to_string());
        let run = TextRun { len: said.len(), font: gpui::font(face.clone()), color: ink, ..Default::default() };
        system.shape_line(said, small, &[run], None)
    };
    let measure = |said: &str| f32::from(shape(said, gpui::black()).width);
    if let Some(across) = graph.marks.first() {
        let line = shape("Compacted", look.color(token::FG_SUBTLE));
        let mark = left + px(across * w);
        let pad = look.length(token::SPACE_1);
        let x = if mark + pad + line.width <= left + plot.size.width { mark + pad } else { mark - pad - line.width };
        let _ = line.paint(point(x, plot.origin.y + pad / 2.), lead, TextAlign::Left, None, window, cx);
    }

    // A cold wake: a dot where the Job woke on the context line.
    let side = look.length(token::SPACE_2);
    for band in graph.bands.iter().filter(|band| band.cold) {
        if let Some((across, share)) = band.wake {
            let at = on_line(across, share);
            let dot = Bounds::new(point(at.x - side / 2., at.y - side / 2.), size(side, side));
            window.paint_quad(quad(dot, side / 2., look.color(token::STATUS_ESCALATED), hairline, look.color(token::BG_RAISED), BorderStyle::Solid));
        }
    }

    // The pauses' labels over the chart, two short lines, each cut to the band it stands over.
    let pad = look.length(token::SPACE_1);
    for band in &graph.bands {
        let (x0, x1) = (left + px(band.x0 * w), left + px(band.x1 * w));
        let room = f32::from(x1 - x0 - pad * 2.);
        let ink = if band.on_person { look.color(token::STATUS_AWAITING_REVIEW) } else { look.color(token::FG_MUTED) };
        for (row, said) in [band.label.as_str(), band.duration.as_str()].into_iter().enumerate() {
            let Some(shown) = fit(said, room, &|text| measure(text)) else { continue };
            let line = shape(&shown, ink);
            let x = x0 + (x1 - x0 - line.width) / 2.;
            let _ = line.paint(point(x, bounds.origin.y + lead * row as f32 + pad / 2.), lead, TextAlign::Left, None, window, cx);
        }
    }

    // The hour axis.
    let mut free = left;
    for (across, said) in graph.ticks_in(&chrono::Local) {
        let x = left + px(across * w);
        window.paint_quad(quad_of(Bounds::new(point(x, bottom), size(hairline, pad)), look.color(token::BORDER_DEFAULT)));
        let line = shape(&said, look.color(token::FG_SUBTLE));
        let at = (x - line.width / 2.).max(free);
        if at + line.width <= left + plot.size.width + pad && at >= free {
            let _ = line.paint(point(at, bottom + pad * 1.5), lead, TextAlign::Left, None, window, cx);
            free = at + line.width + pad * 2.;
        }
    }
}

/// Shapes where the bars will be, while Fleet has not answered.
pub fn bones(look: &Look, tall: Pixels) -> gpui::Div {
    let heights = [0.3, 0.5, 0.4, 0.7, 0.55, 0.35, 0.8, 0.6, 0.45, 0.9, 0.5, 0.4, 0.65, 0.3, 0.55, 0.75, 0.4, 0.5, 0.35, 0.6];
    div().w_full().h(tall).flex().items_end().gap(look.length(token::SPACE_1)).children(heights.map(|share| div().flex_1().child(bone(look, px(1000.), tall * share).w_full())))
}
