//! Where the lanes, nodes and chips stand on the canvas, and how the view pans to them (`layout.ts`).

use std::collections::{HashMap, HashSet};

use ipc::{LaneEdge, LaneKind, LaneNodeKind, LaneView};

use super::{Placement, Rect, View, CHIP_H, COL_W, DOT, DOT_GAP, HEAD, HOME, LANE_MIN, LEAD, NODE_W, PAD, STATE_H, STEP_H, SUB_CHARS, SUB_LINE};

/// The lanes fill `height`, and stand at the least the lane height tall each when it is short (`place`).
pub(super) fn place(lanes: &[LaneView], height: f32) -> Placement {
    place_at(lanes, height, HEAD)
}

/// `place` with `head` points of room at the left for the lane heads: none where the canvas draws without them.
pub(super) fn place_at(lanes: &[LaneView], height: f32, head: f32) -> Placement {
    let last = lanes.iter().flat_map(|lane| lane.nodes.iter()).map(|node| node.col).max().unwrap_or(0) as f32;
    let origin = head + PAD + LEAD;
    let tail = if lanes.iter().flat_map(|lane| lane.nodes.iter()).any(|node| node.kind == LaneNodeKind::End) { DOT_GAP + DOT } else { 0. };
    let width = origin + PAD + last * COL_W + NODE_W + tail;
    let lane_h = LANE_MIN.max((height / lanes.len().max(1) as f32).floor());
    let mut bands = Vec::new();
    let mut items = HashMap::new();
    for (at, lane) in lanes.iter().enumerate() {
        let y = at as f32 * lane_h;
        // A step that names what it waits on is as tall as its words need.
        let lines = if lane.kind == LaneKind::Workflow {
            lane.nodes.iter().map(|node| node.waiting.iter().map(|said| (said.chars().count() as f32 / SUB_CHARS).ceil()).sum::<f32>() - 2.).fold(0_f32, f32::max)
        } else {
            0.
        };
        let h = if lane.kind == LaneKind::Workflow { STEP_H } else { STATE_H } + lines * SUB_LINE;
        bands.push(Rect { x: 0., y, w: width, h: lane_h });
        for node in &lane.nodes {
            let x = origin + node.col as f32 * COL_W;
            let rect = match node.kind {
                LaneNodeKind::Start => Rect { x: x - DOT_GAP - DOT, y: y + (lane_h - DOT) / 2., w: DOT, h: DOT },
                LaneNodeKind::End => Rect { x: x + NODE_W + DOT_GAP, y: y + (lane_h - DOT) / 2., w: DOT, h: DOT },
                _ => Rect { x, y: y + (lane_h - h) / 2., w: NODE_W, h },
            };
            items.insert(node.id.clone(), rect);
        }
    }
    Placement { bands, items, lane_h, width, height: lanes.len() as f32 * lane_h, head }
}

/// The lanes of a canvas with no heads and no rail, placed and zoomed so the whole of them shows in `frame`:
/// as far out as that takes, past `MIN_ZOOM` where it must, and never in past 1. The lanes fill the frame's
/// height at that zoom. Not yet measured, the canvas is at `HOME`.
pub(super) fn place_fitted(lanes: &[LaneView], frame: (f32, f32)) -> (Placement, View) {
    let bare = place_at(lanes, frame.1, 0.);
    if frame.0 <= 0. || frame.1 <= 0. {
        return (bare, HOME);
    }
    let least = lanes.len().max(1) as f32 * LANE_MIN;
    let zoom = (frame.0 / bare.width).min(frame.1 / least).min(1.);
    (place_at(lanes, frame.1 / zoom, 0.), View { x: 0., y: 0., zoom })
}

/// A time of day in his clock, as the chips read it (`clock`).
pub(crate) fn clock(at: &ipc::Instant) -> String {
    chrono::DateTime::parse_from_rfc3339(at.as_str()).map(|at| crate::settings::time::clock(at.with_timezone(&chrono::Local))).unwrap_or_default()
}

/// The quiet words beside a chip's name (`RuleView.meta`): what it waits for, else when it fired.
pub(super) fn meta(edge: &LaneEdge) -> Option<String> {
    edge.waits.clone().or_else(|| edge.fired_at.as_ref().map(clock))
}

/// The chip's size as its words will draw it (`chipSize`).
pub(super) fn chip_size(edge: &LaneEdge) -> (f32, f32) {
    if edge.quiet {
        return (CHIP_H, CHIP_H);
    }
    let words = edge.label.chars().count() as f32 * 6.6 + meta(edge).map_or(0., |meta| 12. + meta.chars().count() as f32 * 7.9);
    // A rule set otherwise somewhere carries the split's glyph after its words.
    let differs = if edge.differs.is_empty() { 0. } else { 18. };
    ((words + 44. + differs).ceil(), CHIP_H)
}

/// The shape of React Flow's bezier between a bottom and a top handle, both from the one parameter.
pub(super) fn rise(t: f32) -> f32 {
    1.5 * t * (1. - t) + t * t * t
}
pub(super) fn slide(t: f32) -> f32 {
    3. * t * t - 2. * t * t * t
}

/// Where along its curve an edge crosses the height `y` (`crossing`).
pub(super) fn crossing(from: (f32, f32), to: (f32, f32), y: f32) -> f32 {
    let along = (y - from.1) / (to.1 - from.1);
    let (mut low, mut high) = (0_f32, 1_f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.;
        if rise(mid) < along {
            low = mid;
        } else {
            high = mid;
        }
    }
    from.0 + (to.0 - from.0) * slide((low + high) / 2.)
}

pub(super) fn touches(a: &Rect, b: &Rect, margin: f32) -> bool {
    a.x < b.x + b.w + margin && b.x < a.x + a.w + margin && a.y < b.y + b.h + margin && b.y < a.y + a.h + margin
}

/// Where each rule's chip sits, by its edge's id, as its centre and size (`placeChips`). On the edge, in the
/// gutter between two lanes; the ones with words are placed first, so a chip with only its mark moves.
pub(super) fn place_chips(edges: &[LaneEdge], placement: &Placement) -> HashMap<String, Rect> {
    let mut out = HashMap::new();
    let mut taken: Vec<Rect> = Vec::new();
    let ordered = edges.iter().filter(|edge| !edge.quiet).chain(edges.iter().filter(|edge| edge.quiet));
    for edge in ordered {
        let (Some(from), Some(to)) = (placement.items.get(&edge.from), placement.items.get(&edge.to)) else { continue };
        let down = to.y > from.y;
        let start = (from.x + from.w / 2., if down { from.y + from.h } else { from.y });
        let end = (to.x + to.w / 2., if down { to.y } else { to.y + to.h });
        let lanes = ((from.y / placement.lane_h).floor() as i32, (to.y / placement.lane_h).floor() as i32);
        let lines: Vec<f32> = if lanes.0 == lanes.1 {
            vec![lanes.0 as f32 * placement.lane_h]
        } else if down {
            (lanes.0 + 1..=lanes.1).map(|at| at as f32 * placement.lane_h).collect()
        } else {
            (lanes.1 + 1..=lanes.0).rev().map(|at| at as f32 * placement.lane_h).collect()
        };
        let (w, h) = chip_size(edge);
        let fit = |x: f32, y: f32| -> Rect {
            let left = (x - w / 2.).max(placement.head + PAD / 2.).min((placement.head + PAD / 2.).max(placement.width - w - PAD / 2.));
            Rect { x: left, y: y - h / 2., w, h }
        };
        let mut chosen = None;
        for &y in &lines {
            let sits = fit(if lanes.0 == lanes.1 { (start.0 + end.0) / 2. } else { crossing(start, end, y) }, y);
            if !taken.iter().any(|one| touches(one, &sits, 4.)) {
                chosen = Some(sits);
                break;
            }
        }
        let chosen = chosen.unwrap_or_else(|| {
            // Every gutter on its way is taken where it crosses: slide along the first until nothing is there.
            let first = lines[0];
            let from = fit(crossing(start, end, first), first);
            let mut sits = from;
            for _ in 0..12 {
                let Some(hit) = taken.iter().find(|one| touches(one, &sits, 4.)) else { break };
                sits.x = hit.x + hit.w + 6.;
            }
            let (left_edge, right_edge) = (placement.head + PAD / 2., placement.width - PAD / 2.);
            if sits.x + sits.w <= right_edge {
                return sits;
            }
            // Past the content's right edge no view can reach it: slide the other way, and failing that
            // stand at the edge over what is there.
            let mut back = from;
            for _ in 0..12 {
                let Some(hit) = taken.iter().find(|one| touches(one, &back, 4.)) else { break };
                back.x = hit.x - back.w - 6.;
            }
            if back.x >= left_edge {
                back
            } else {
                Rect { x: (right_edge - sits.w).max(left_edge), ..sits }
            }
        });
        taken.push(chosen);
        out.insert(edge.id.clone(), Rect { x: chosen.x + chosen.w / 2., y: chosen.y + chosen.h / 2., w: chosen.w, h: chosen.h });
    }
    out
}

/// The view that brings `rects` into the frame, or `None` where they already are (`panTo`). The heads stand
/// over the left of the frame, so what is brought in clears them. With `keep`, the first rect is what was
/// picked: when not all fit, the others go before it does.
pub(super) fn pan_to(rects: &[Rect], view: View, frame: (f32, f32), content: (f32, f32), follow: bool, earliest: bool, keep: bool) -> Option<View> {
    if rects.is_empty() || frame.0 == 0. || frame.1 == 0. {
        return None;
    }
    let zoom = view.zoom;
    let (left, right, gap) = (HEAD + PAD, PAD, PAD / 2.);
    let usable = frame.0 - left - right;
    let mut sorted = rects.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
    let bound = |set: &[Rect]| {
        let x = set.iter().map(|one| one.x).fold(f32::INFINITY, f32::min);
        let y = set.iter().map(|one| one.y).fold(f32::INFINITY, f32::min);
        Rect { x, y, w: set.iter().map(|one| one.x + one.w).fold(f32::NEG_INFINITY, f32::max) - x, h: set.iter().map(|one| one.y + one.h).fold(f32::NEG_INFINITY, f32::max) - y }
    };
    let picked = keep.then(|| rects[0]);
    let mut kept = sorted;
    while kept.len() > 1 && bound(&kept).w * zoom > usable {
        let (first, last) = (0, kept.len() - 1);
        let side = if earliest { last } else { first };
        kept.remove(if picked == Some(kept[side]) { first + last - side } else { side });
    }
    let at = bound(&kept);
    let mut x = view.x;
    let from = at.x * zoom + view.x;
    let to = (at.x + at.w) * zoom + view.x;
    if from < left || to > frame.0 - right {
        x = if follow || from < left || at.w * zoom > usable { left - at.x * zoom } else { frame.0 - right - (at.x + at.w) * zoom };
    }
    let mut y = view.y;
    let top = at.y * zoom + view.y;
    let bottom = (at.y + at.h) * zoom + view.y;
    if top < gap || bottom > frame.1 - gap {
        y = if top < gap || at.h * zoom > frame.1 - gap * 2. { gap - at.y * zoom } else { frame.1 - gap - (at.y + at.h) * zoom };
    }
    // Never past the lanes: nothing is left of the first column or right of the last.
    x = 0_f32.min(if content.0 * zoom > frame.0 { x.max(frame.0 - content.0 * zoom) } else { 0. });
    y = 0_f32.min(if content.1 * zoom > frame.1 { y.max(frame.1 - content.1 * zoom) } else { 0. });
    ((x - view.x).abs() >= 1. || (y - view.y).abs() >= 1.).then_some(View { x, y, zoom })
}

/// The node an arrow reaches: the nearest whose centre lies that way, one off to the side costing twice
/// what one straight on does (`nearest`). The circles at the workflow's ends are skipped.
pub(super) fn nearest(items: &HashMap<String, Rect>, from: &str, key: &str, skip: &HashSet<String>) -> Option<String> {
    let here = items.get(from)?;
    let (cx, cy) = (here.x + here.w / 2., here.y + here.h / 2.);
    let mut best: Option<(String, f32)> = None;
    for (id, one) in items {
        if id == from || skip.contains(id) {
            continue;
        }
        let (dx, dy) = (one.x + one.w / 2. - cx, one.y + one.h / 2. - cy);
        let (along, across) = match key {
            "right" => (dx, dy.abs()),
            "left" => (-dx, dy.abs()),
            "down" => (dy, dx.abs()),
            _ => (-dy, dx.abs()),
        };
        if along <= 0. || across > along * 3. {
            continue;
        }
        let cost = along + across * 2.;
        if best.as_ref().is_none_or(|(_, was)| cost < *was) {
            best = Some((id.clone(), cost));
        }
    }
    best.map(|(id, _)| id)
}

/// Which sides an edge leaves and enters by (`sidesOf`): across within a lane, a way back under it, and
/// down or up the lanes by the bottom and the top.
#[derive(Clone, Copy)]
pub(super) enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

pub(super) fn sides_of(from: &Rect, to: &Rect) -> (Side, Side) {
    if ((from.y + from.h / 2.) - (to.y + to.h / 2.)).abs() < 1. {
        return if to.x >= from.x { (Side::Right, Side::Left) } else { (Side::Bottom, Side::Bottom) };
    }
    if to.y > from.y {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

pub(super) fn handle(rect: &Rect, side: Side) -> (f32, f32) {
    match side {
        Side::Top => (rect.x + rect.w / 2., rect.y),
        Side::Right => (rect.x + rect.w, rect.y + rect.h / 2.),
        Side::Bottom => (rect.x + rect.w / 2., rect.y + rect.h),
        Side::Left => (rect.x, rect.y + rect.h / 2.),
    }
}

/// React Flow's bezier between two handles (`getBezierPath` at curvature 0.25): its four points.
pub(super) fn bezier(from: &Rect, to: &Rect) -> [(f32, f32); 4] {
    let (out, into) = sides_of(from, to);
    let (s, t) = (handle(from, out), handle(to, into));
    let offset = |distance: f32| if distance >= 0. { 0.5 * distance } else { 0.25 * 25. * (-distance).sqrt() };
    let control = |side: Side, a: (f32, f32), b: (f32, f32)| match side {
        Side::Left => (a.0 - offset(a.0 - b.0), a.1),
        Side::Right => (a.0 + offset(b.0 - a.0), a.1),
        Side::Top => (a.0, a.1 - offset(a.1 - b.1)),
        Side::Bottom => (a.0, a.1 + offset(b.1 - a.1)),
    };
    [s, control(out, s, t), control(into, t, s), t]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lanes_canvas::{LanesHost, LanesState, MIN_ZOOM};
    use gpui::{App, SharedString};
    use ipc::JobLanesView;

    fn fixture() -> JobLanesView {
        ipc::decode::<JobLanesView>("JobLanesView", include_bytes!("../../fixtures/job-lanes.json")).unwrap()
    }

    #[test]
    fn a_compact_canvas_zooms_to_the_width_it_is_given_with_no_head_inset() {
        let lanes = fixture();
        let bare = place_at(&lanes.lanes, 1000., 0.);
        for width in [340., 1400.] {
            let (placement, view) = place_fitted(&lanes.lanes, (width, 1000.));
            assert_eq!(placement.head, 0.);
            // The Start dot stands inside PAD + LEAD of the frame's left, where the heads' 144 would have been.
            let first = placement.items.values().map(|rect| rect.x).fold(f32::INFINITY, f32::min);
            assert!(first < PAD + LEAD, "{first}");
            assert_eq!((view.x, view.y), (0., 0.));
            assert!(placement.width * view.zoom <= width + 0.01, "{width}: {} at {}", placement.width, view.zoom);
            assert_eq!(placement.width, bare.width);
        }
        // Wide enough, it stays at its own size; narrow, it goes out past what the rail allows.
        assert_eq!(place_fitted(&lanes.lanes, (1400., 1000.)).1.zoom, 1.);
        let narrow = place_fitted(&lanes.lanes, (340., 1000.)).1.zoom;
        assert!(narrow < MIN_ZOOM && (narrow - 340. / bare.width).abs() < 1e-4, "{narrow}");
        // The lanes fill the frame's height at that zoom, and a frame not yet measured is at home.
        let (placement, view) = place_fitted(&lanes.lanes, (340., 1000.));
        assert!((placement.height * view.zoom - 1000.).abs() < lanes.lanes.len() as f32 * view.zoom + 0.01);
        assert_eq!(place_fitted(&lanes.lanes, (0., 0.)).1, HOME);
    }

    struct Bare;
    impl LanesHost for Bare {
        fn lanes_state(&self) -> &LanesState {
            unreachable!()
        }
        fn lanes_of<'a>(&'a self, _: &SharedString, _: &'a App) -> Option<&'a JobLanesView> {
            unreachable!()
        }
    }

    #[test]
    fn a_host_that_sets_nothing_keeps_the_heads_the_rail_and_the_words() {
        let (host, id) = (Bare, SharedString::from("canvas"));
        assert!(!host.lanes_compact(&id));
        assert!(host.lanes_fit(&id));
        assert!(!host.lanes_chosen(&id, "workflow/plan"));
        assert_eq!(host.changed_said(&id), "Changed on this Job");
        // Not compact, the canvas is placed with the heads' inset as before.
        let lanes = fixture();
        let placement = place(&lanes.lanes, 600.);
        assert_eq!(placement.head, HEAD);
        let first = lanes.lanes[0].nodes.iter().find(|node| node.id == "workflow/plan").unwrap();
        assert_eq!(placement.items[&first.id].x, HEAD + PAD + LEAD);
    }
}
