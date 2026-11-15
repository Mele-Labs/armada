//! Notes on a sketch's parts (`ViewerNotes` on `SketchScene`): which part a point is on, which parts a key walks, and the
//! numbers that stand on them. A part is a node, or an element inside a `mock`, which the page reports with where it laid it
//! out. A node's id names it; an element is `<node>#<key>`, as `splitPart` splits it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{point, px, quad, size, BorderStyle, Bounds, Hsla, Pixels, Point, SharedString, TextAlign, TextRun, Window};

use super::geometry::{Pt, Rect};
use super::{Ink, View};
use crate::look::{token, Look};

/// A part a note may be about: its id, and the name a person reads for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Part {
    pub id: String,
    pub label: String,
}

/// A number standing on parts: a held note's, or the one being written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub n: usize,
    pub writing: bool,
    pub ids: Vec<String>,
}

/// What the drawing marks on top of itself: the notes' numbers, and the parts picked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Marks {
    pub notes: Vec<Mark>,
    pub picked: Vec<String>,
}

/// Where a part stands in scene units, and how deep: a ground is under a node, which is under an element of its page.
#[derive(Clone, Debug)]
pub(super) struct Placed {
    pub part: Part,
    pub rect: Rect,
    pub depth: u32,
    /// Whether a key reaches it: a node, not an element.
    pub node: bool,
}

/// What one drawing of a scene put where, kept for the pointer and the keys.
pub(super) struct Probe {
    pub view: View,
    pub placed: Vec<Placed>,
}

thread_local! {
    /// The last frame each reading view drew, by its key.
    static PROBES: RefCell<HashMap<SharedString, Rc<Probe>>> = RefCell::default();
}

pub(super) fn keep(key: &SharedString, probe: Probe) {
    PROBES.with(|held| held.borrow_mut().insert(key.clone(), Rc::new(probe)));
}

fn probe(key: &str) -> Option<Rc<Probe>> {
    PROBES.with(|held| held.borrow().get(key).cloned())
}

/// The innermost part holding the point; of two as deep, the smaller.
fn hit(placed: &[Placed], at: Pt) -> Option<&Placed> {
    placed
        .iter()
        .filter(|one| at.x >= one.rect.x && at.x <= one.rect.x + one.rect.w && at.y >= one.rect.y && at.y <= one.rect.y + one.rect.h)
        .max_by(|a, b| a.depth.cmp(&b.depth).then((b.rect.w * b.rect.h).total_cmp(&(a.rect.w * a.rect.h))))
}

/// The part under a point of the window, with where it stands there.
pub(crate) fn part_at(key: &str, at: Point<Pixels>) -> Option<(Part, Bounds<Pixels>)> {
    let probe = probe(key)?;
    let found = hit(&probe.placed, probe.view.back(at))?;
    Some((found.part.clone(), probe.view.rect(found.rect)))
}

/// Each node in the order the scene holds them, with where it stands: what Tab walks.
pub(crate) fn nodes(key: &str) -> Vec<(Part, Bounds<Pixels>)> {
    let Some(probe) = probe(key) else { return Vec::new() };
    probe.placed.iter().filter(|one| one.node).map(|one| (one.part.clone(), probe.view.rect(one.rect))).collect()
}

/// Where a part stands in the window, for the box that opens clear of it.
pub(crate) fn bounds_of(key: &str, id: &str) -> Option<Bounds<Pixels>> {
    let probe = probe(key)?;
    probe.placed.iter().find(|one| one.part.id == id).map(|one| probe.view.rect(one.rect))
}

impl View {
    /// A point of the window as a point of the scene: `at` undone.
    pub(super) fn back(&self, p: Point<Pixels>) -> Pt {
        Pt { x: self.centre.x + f32::from(p.x - self.origin.x) / self.zoom, y: self.centre.y + f32::from(p.y - self.origin.y) / self.zoom }
    }
}

/// A part's note numbers on its top right corner, and a ring on each part picked.
pub(super) fn paint(ink: &Ink, placed: &[Placed], marks: &Marks, window: &mut Window, cx: &mut gpui::App) {
    let (look, view) = (ink.look, ink.view);
    for one in placed.iter().filter(|one| marks.picked.contains(&one.part.id)) {
        let ring = Rect { x: one.rect.x - 2., y: one.rect.y - 2., w: one.rect.w + 4., h: one.rect.h + 4. };
        window.paint_quad(quad(view.rect(ring), view.len(f32::from(look.length(token::RADIUS_SM)) + 2.), gpui::transparent_black(), view.len(2.), look.color(token::EDGE_FOCUS), BorderStyle::Solid));
    }
    for one in placed {
        let here: Vec<&Mark> = marks.notes.iter().filter(|mark| mark.ids.contains(&one.part.id)).collect();
        if here.is_empty() {
            continue;
        }
        pins(&here, view.rect(one.rect), look, window, cx);
    }
}

/// One part's numbers, side by side with the last at the corner. They hold their own size, not the zoom's.
fn pins(here: &[&Mark], at: Bounds<Pixels>, look: &Look, window: &mut Window, cx: &mut gpui::App) {
    let (text, lead) = (look.length(token::TEXT_2XS), look.length(token::LEADING_2XS));
    let (gap, pad, edge) = (look.length(token::SPACE_1), look.length(token::SPACE_1), look.length(token::BORDER_WIDTH));
    let font = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_else(|| "IBM Plex Mono".into());
    let weight = gpui::FontWeight(look.number(token::WEIGHT_MEDIUM));
    let shaped: Vec<_> = here
        .iter()
        .map(|mark| {
            let said = SharedString::from(mark.n.to_string());
            let ink: Hsla = look.color(if mark.writing { token::ACCENT } else { token::FG_INVERSE });
            let run = TextRun { len: said.len(), font: gpui::Font { weight, ..gpui::font(font.clone()) }, color: ink, ..Default::default() };
            (mark.writing, window.text_system().shape_line(said, text, &[run], None))
        })
        .collect();
    let widths: Vec<Pixels> = shaped.iter().map(|(_, line)| (line.width + pad * 2.).max(lead)).collect();
    let total = widths.iter().fold(px(0.), |sum, width| sum + *width) + gap * (shaped.len().saturating_sub(1)) as f32;
    let mut x = at.right() + look.length(token::SPACE_2) - total;
    let y = at.top() - look.length(token::SPACE_2);
    for ((writing, line), width) in shaped.iter().zip(widths) {
        let bounds = Bounds::new(point(x, y), size(width, lead));
        let (fill, border) = if *writing { (look.color(token::BG_OVERLAY), look.color(token::ACCENT)) } else { (look.color(token::ACCENT), look.color(token::ACCENT)) };
        window.paint_quad(quad(bounds, look.length(token::RADIUS_SM), fill, edge, border, BorderStyle::Solid));
        let _ = line.paint(point(x + (width - line.width) / 2., y), lead, TextAlign::Left, None, window, cx);
        x += width + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: &str, rect: (f32, f32, f32, f32), depth: u32) -> Placed {
        Placed { part: Part { id: id.into(), label: id.into() }, rect: Rect { x: rect.0, y: rect.1, w: rect.2, h: rect.3 }, depth, node: depth <= 1 }
    }

    #[test]
    fn the_innermost_part_under_a_point_is_the_one_named() {
        let all = [placed("group", (0., 0., 400., 300.), 0), placed("m", (10., 10., 100., 80.), 1), placed("m#chip", (20., 20., 40., 20.), 2)];
        let at = |x: f32, y: f32| hit(&all, Pt { x, y }).map(|one| one.part.id.as_str());
        assert_eq!(at(30., 30.), Some("m#chip"), "an element of the page");
        assert_eq!(at(100., 80.), Some("m"), "the node, off its elements");
        assert_eq!(at(300., 250.), Some("group"), "a ground, where no node is");
        assert_eq!(at(500., 500.), None);
    }

    #[test]
    fn of_two_parts_as_deep_the_smaller_wins() {
        let all = [placed("a", (0., 0., 100., 100.), 1), placed("b", (10., 10., 20., 20.), 1)];
        assert_eq!(hit(&all, Pt { x: 15., y: 15. }).map(|one| one.part.id.as_str()), Some("b"));
    }
}
