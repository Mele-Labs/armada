//! Ported from `packages/components/src/compositions/SketchScene/route.ts`.

use std::collections::{HashMap, HashSet};

use ipc::ScenePath;

use super::geometry::{Pt, Rect};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

const SIDES: [Side; 4] = [Side::Right, Side::Left, Side::Bottom, Side::Top];

fn out_of(side: Side) -> Pt {
    match side {
        Side::Top => Pt { x: 0.0, y: -1.0 },
        Side::Right => Pt { x: 1.0, y: 0.0 },
        Side::Bottom => Pt { x: 0.0, y: 1.0 },
        Side::Left => Pt { x: -1.0, y: 0.0 },
    }
}

fn side_of(r: Rect, side: Side) -> Pt {
    match side {
        Side::Top => Pt {
            x: r.x + r.w / 2.0,
            y: r.y,
        },
        Side::Bottom => Pt {
            x: r.x + r.w / 2.0,
            y: r.y + r.h,
        },
        Side::Left => Pt {
            x: r.x,
            y: r.y + r.h / 2.0,
        },
        Side::Right => Pt {
            x: r.x + r.w,
            y: r.y + r.h / 2.0,
        },
    }
}

/// Whether the run from `a` to `b` passes through the inside of `r`; running along its edge does not.
fn cuts(a: Pt, b: Pt, r: Rect) -> bool {
    a.x.min(b.x) < r.x + r.w - 1.0
        && a.x.max(b.x) > r.x + 1.0
        && a.y.min(b.y) < r.y + r.h - 1.0
        && a.y.max(b.y) > r.y + 1.0
}

/// Corners where the way goes straight on are dropped, and so are runs of no length.
fn tidy(points: &[Pt]) -> Vec<Pt> {
    let mut kept: Vec<Pt> = Vec::new();
    for &one in points {
        let last = kept.last().copied();
        if let Some(last) = last {
            if last.x == one.x && last.y == one.y {
                continue;
            }
        }
        let before = kept
            .len()
            .checked_sub(2)
            .and_then(|at| kept.get(at))
            .copied();
        if let (Some(before), Some(last)) = (before, last) {
            if (before.x == last.x && last.x == one.x) || (before.y == last.y && last.y == one.y) {
                kept.pop();
            }
        }
        kept.push(one);
    }
    kept
}

/// The sides a way leaves and reaches, and the points on them: what `route_all` hands back once it has spread them.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ends {
    pub out: Side,
    pub into: Side,
    pub start: Pt,
    pub end: Pt,
}

/// Which side of `r` the point is on.
fn side_at(r: Rect, p: Pt) -> Side {
    if p.x == r.x {
        Side::Left
    } else if p.x == r.x + r.w {
        Side::Right
    } else if p.y == r.y {
        Side::Top
    } else {
        Side::Bottom
    }
}

/// Where a run between the boxes may go: halfway, and just outside any box near the two.
fn lanes(near: &[Rect], from: Rect, to: Rect, gap: f32, stub: f32, horizontal: bool) -> Vec<f32> {
    let key = |r: &Rect| if horizontal { r.x } else { r.y };
    let size = |r: &Rect| if horizontal { r.w } else { r.h };
    let lo = key(&from).min(key(&to)) - gap * 2.0;
    let hi = (key(&from) + size(&from)).max(key(&to) + size(&to)) + gap * 2.0;
    let mut found: Vec<f32> = Vec::new();
    for one in near {
        for lane in [key(one) - stub, key(one) + size(one) + stub] {
            if lane >= lo && lane <= hi && !found.contains(&lane) {
                found.push(lane);
            }
        }
    }
    found
}

/// The corners of the way from `from` to `to`, its ends on their sides. `others` are the boxes it must not
/// cross; its own two are kept out of everything but the first and last runs.
/// **`need` is how far the way should run straight from each end**, which a head there asks for: a way that
/// cannot give it, because a box is in the road, keeps the short run it had.
#[allow(dead_code)]
pub(crate) fn route(
    from: Rect,
    to: Rect,
    others: &[Rect],
    gap: f32,
    ends: Option<&Ends>,
    need: (f32, f32),
) -> Vec<Pt> {
    let stub = gap / 2.0;
    let mut near = vec![from, to];
    near.extend_from_slice(others);
    let xs = lanes(&near, from, to, gap, stub, true);
    let ys = lanes(&near, from, to, gap, stub, false);

    let outs: Vec<Side> = match ends {
        Some(ends) => vec![ends.out],
        None => SIDES.to_vec(),
    };
    let intos: Vec<Side> = match ends {
        Some(ends) => vec![ends.into],
        None => SIDES.to_vec(),
    };

    let mut best: Option<Vec<Pt>> = None;
    let mut best_cost = f32::INFINITY;
    for &out in &outs {
        for &into in &intos {
            let start = match ends {
                Some(ends) => ends.start,
                None => side_of(from, out),
            };
            let end = match ends {
                Some(ends) => ends.end,
                None => side_of(to, into),
            };
            // A longer stub at an end that wants a straight run, tried beside the short one.
            let mut leads: Vec<(f32, f32)> = vec![(stub, stub)];
            for lead in [(stub.max(need.0), stub), (stub, stub.max(need.1)), (stub.max(need.0), stub.max(need.1))] {
                if !leads.contains(&lead) {
                    leads.push(lead);
                }
            }
            for lead in leads {
                let a = Pt {
                    x: start.x + out_of(out).x * lead.0,
                    y: start.y + out_of(out).y * lead.0,
                };
                let b = Pt {
                    x: end.x + out_of(into).x * lead.1,
                    y: end.y + out_of(into).y * lead.1,
                };
                let mut middles: Vec<Vec<Pt>> = vec![
                    vec![Pt { x: b.x, y: a.y }],
                    vec![Pt { x: a.x, y: b.y }],
                    vec![
                        Pt {
                            x: (a.x + b.x) / 2.0,
                            y: a.y,
                        },
                        Pt {
                            x: (a.x + b.x) / 2.0,
                            y: b.y,
                        },
                    ],
                    vec![
                        Pt {
                            x: a.x,
                            y: (a.y + b.y) / 2.0,
                        },
                        Pt {
                            x: b.x,
                            y: (a.y + b.y) / 2.0,
                        },
                    ],
                ];
                for &x in &xs {
                    middles.push(vec![Pt { x, y: a.y }, Pt { x, y: b.y }]);
                }
                for &y in &ys {
                    middles.push(vec![Pt { x: a.x, y }, Pt { x: b.x, y }]);
                }
                for middle in middles {
                    let mut full = vec![start, a];
                    full.extend(middle);
                    full.push(b);
                    full.push(end);
                    let points = tidy(&full);
                    // A run short of what an end wants costs, so a way with room takes the long stub, and one
                    // without keeps the short one rather than cutting through a box.
                    let run = |p: Pt, q: Pt| (q.x - p.x).hypot(q.y - p.y);
                    let n = points.len();
                    let mut cost = if n >= 2 {
                        ((need.0 - run(points[0], points[1])).max(0.0) + (need.1 - run(points[n - 2], points[n - 1])).max(0.0)) * SHORT_RUN
                    } else {
                        0.0
                    };
                    let mut at = 1;
                    while at < points.len() && cost < best_cost {
                        let p = points[at - 1];
                        let q = points[at];
                        cost += (q.x - p.x).abs() + (q.y - p.y).abs() + 24.0;
                        for &one in others {
                            if cuts(p, q, one) {
                                cost += 100_000.0;
                            }
                        }
                        let inner = at > 1 && at < points.len() - 1;
                        if inner && (cuts(p, q, from) || cuts(p, q, to)) {
                            cost += 100_000.0;
                        }
                        // The first run leaves its box and the last reaches its own: back through either is a crossing too.
                        if at == 1 && cuts(p, q, to) {
                            cost += 100_000.0;
                        }
                        if at == points.len() - 1 && cuts(p, q, from) {
                            cost += 100_000.0;
                        }
                        at += 1;
                    }
                    if cost < best_cost {
                        best_cost = cost;
                        best = Some(points);
                    }
                }
            }
        }
    }
    best.unwrap_or_else(|| vec![side_of(from, Side::Right), side_of(to, Side::Left)])
}

/// What each unit a run falls short of what its head wants costs: less than going round a box, more than a bend.
const SHORT_RUN: f32 = 2.0;

/// One command of a drawn way: a move, a run, a rounded corner as its control point and its end, a curve as its two
/// control points and its end, or a hop over another arrow as its end, its radius and which side it bulges to.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Seg {
    Move(Pt),
    Line(Pt),
    Quad(Pt, Pt),
    Cubic(Pt, Pt, Pt),
    Arc(Pt, f32, bool),
}

/// JS `Math.round`: halves go up, so a negative half goes towards zero.
fn js_round(n: f32) -> f32 {
    (n + 0.5).floor()
}

/// A coordinate to a tenth, as the path text carried it.
fn tenth(n: f32) -> f32 {
    js_round(n * 10.0) / 10.0
}

fn tenth_pt(p: Pt) -> Pt {
    Pt {
        x: tenth(p.x),
        y: tenth(p.y),
    }
}

/// The way as segments, its corners rounded to `radius` or as much as the runs either side allow.
#[allow(dead_code)]
pub(crate) fn rounded_path(points: &[Pt], radius: f32) -> Vec<Seg> {
    let Some(&first) = points.first() else {
        return Vec::new();
    };
    let mut segs = vec![Seg::Move(tenth_pt(first))];
    for at in 1..points.len().saturating_sub(1) {
        let p = points[at - 1];
        let c = points[at];
        let n = points[at + 1];
        let into = (c.x - p.x).hypot(c.y - p.y);
        let away = (n.x - c.x).hypot(n.y - c.y);
        let r = radius.min(into / 2.0).min(away / 2.0);
        let s = Pt {
            x: c.x - ((c.x - p.x) / into) * r,
            y: c.y - ((c.y - p.y) / into) * r,
        };
        let e = Pt {
            x: c.x + ((n.x - c.x) / away) * r,
            y: c.y + ((n.y - c.y) / away) * r,
        };
        segs.push(Seg::Line(tenth_pt(s)));
        segs.push(Seg::Quad(tenth_pt(c), tenth_pt(e)));
    }
    let last = points.last().copied().unwrap_or(first);
    segs.push(Seg::Line(tenth_pt(last)));
    segs
}

/// The chords a curve is read as: for where a hop may fall on it, where a dot is along it and where its words sit.
const CUBIC_CUTS: usize = 32;

/// The point `t` along a cubic.
pub(crate) fn cubic_at(p0: Pt, c1: Pt, c2: Pt, p3: Pt, t: f32) -> Pt {
    let u = 1.0 - t;
    Pt {
        x: u * u * u * p0.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * p3.x,
        y: u * u * u * p0.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * p3.y,
    }
}

/// A cubic's slope at `t`.
fn cubic_slope(p0: Pt, c1: Pt, c2: Pt, p3: Pt, t: f32) -> Pt {
    let u = 1.0 - t;
    Pt {
        x: 3.0 * (u * u * (c1.x - p0.x) + 2.0 * u * t * (c2.x - c1.x) + t * t * (p3.x - c2.x)),
        y: 3.0 * (u * u * (c1.y - p0.y) + 2.0 * u * t * (c2.y - c1.y) + t * t * (p3.y - c2.y)),
    }
}

/// The part of a cubic from `ta` to `tb`, as the control points and end of a cubic that starts at `ta`'s point.
fn cubic_part(p0: Pt, c1: Pt, c2: Pt, p3: Pt, ta: f32, tb: f32) -> Seg {
    let (a, b) = (cubic_at(p0, c1, c2, p3, ta), cubic_at(p0, c1, c2, p3, tb));
    let (sa, sb) = (cubic_slope(p0, c1, c2, p3, ta), cubic_slope(p0, c1, c2, p3, tb));
    let k = (tb - ta) / 3.0;
    Seg::Cubic(Pt { x: a.x + sa.x * k, y: a.y + sa.y * k }, Pt { x: b.x - sb.x * k, y: b.y - sb.y * k }, b)
}

/// The points of a semicircle from `from` to `to`, bulging to the side `sweep` names (SVG's sweep flag).
pub(crate) fn arc_points(from: Pt, to: Pt, sweep: bool, cuts: usize) -> Vec<Pt> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let long = dx.hypot(dy);
    if long <= 0.0 {
        return vec![to];
    }
    let (ux, uy) = (dx / long, dy / long);
    let side = if sweep { 1.0 } else { -1.0 };
    let (nx, ny) = (uy * side, -ux * side);
    let (cx, cy, r) = ((from.x + to.x) / 2.0, (from.y + to.y) / 2.0, long / 2.0);
    (1..=cuts)
        .map(|k| {
            let angle = std::f32::consts::PI * k as f32 / cuts as f32;
            Pt { x: cx - ux * r * angle.cos() + nx * r * angle.sin(), y: cy - uy * r * angle.cos() + ny * r * angle.sin() }
        })
        .collect()
}

/// The way as a line through points: a rounded corner is read as its chord and a curve as its chords, a hop as its end.
pub(crate) fn polyline(path: &[Seg]) -> Vec<Pt> {
    let mut points: Vec<Pt> = Vec::new();
    for seg in path {
        match *seg {
            Seg::Move(p) | Seg::Line(p) | Seg::Quad(_, p) | Seg::Arc(p, _, _) => points.push(p),
            Seg::Cubic(c1, c2, p) => {
                let from = points.last().copied().unwrap_or(c1);
                for k in 1..=CUBIC_CUTS {
                    points.push(cubic_at(from, c1, c2, p, k as f32 / CUBIC_CUTS as f32));
                }
            }
        }
    }
    points
}

/// Where a way starts and ends, each with the point just along it, which says which way a head there points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tips {
    pub start: Pt,
    pub after_start: Pt,
    pub end: Pt,
    pub before_end: Pt,
}

pub(crate) fn tips(path: &[Seg]) -> Option<Tips> {
    let Some(&Seg::Move(start)) = path.first() else { return None };
    let after_start = match path.get(1)? {
        Seg::Move(p) | Seg::Line(p) | Seg::Arc(p, _, _) => *p,
        Seg::Quad(c, _) | Seg::Cubic(c, _, _) => *c,
    };
    let (mut last, mut before) = (start, start);
    for seg in path {
        match *seg {
            Seg::Move(p) => {
                last = p;
                before = p;
            }
            Seg::Line(p) | Seg::Arc(p, _, _) => {
                before = last;
                last = p;
            }
            Seg::Quad(ctrl, p) => {
                before = ctrl;
                last = p;
            }
            Seg::Cubic(_, c2, p) => {
                before = c2;
                last = p;
            }
        }
    }
    Some(Tips { start, after_start, end: last, before_end: before })
}

/// Where a run from `a` to `b` crosses a line through `under`: the share of the run each crossing is at, inside both.
fn crossings(a: Pt, b: Pt, under: &[Pt]) -> Vec<f32> {
    let (rx, ry) = (b.x - a.x, b.y - a.y);
    let mut found = Vec::new();
    for pair in under.windows(2) {
        let (c, d) = (pair[0], pair[1]);
        let (sx, sy) = (d.x - c.x, d.y - c.y);
        let den = rx * sy - ry * sx;
        if den.abs() < 1e-6 {
            continue;
        }
        let t = ((c.x - a.x) * sy - (c.y - a.y) * sx) / den;
        let u = ((c.x - a.x) * ry - (c.y - a.y) * rx) / den;
        if (0.0..1.0).contains(&t) && (0.0..1.0).contains(&u) {
            found.push(t);
        }
    }
    found
}

/// Nothing hops closer than this to where a run ends, so a hop never sits on a corner.
const HOP_MARGIN: f32 = 1.0;

/// Whether a hop's semicircle takes SVG's sweep of `true` or `false` to bulge up, or to the right on a run that goes up and down.
fn hop_sweep(from: Pt, to: Pt) -> bool {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let long = dx.hypot(dy).max(1e-6);
    let (nx, ny) = (dy / long, -dx / long);
    ny < -1e-3 || (ny.abs() <= 1e-3 && nx > 0.0)
}

/// Every way hopping the ways before it: each is drawn over the ones laid earlier, so a crossing is one hop, on the
/// later way. `keeps` are how far from each way's ends it must stay straight, as `hop` takes them.
pub(crate) fn hop_all(paths: &[Option<Vec<Seg>>], r: f32, keeps: &[(f32, f32)]) -> Vec<Option<Vec<Seg>>> {
    let lines: Vec<Option<Vec<Pt>>> = paths.iter().map(|path| path.as_deref().map(polyline)).collect();
    paths
        .iter()
        .enumerate()
        .map(|(at, path)| {
            let path = path.as_ref()?;
            let under: Vec<Vec<Pt>> = lines[..at].iter().flatten().cloned().collect();
            Some(hop(path, &under, r, keeps.get(at).copied().unwrap_or_default()))
        })
        .collect()
}

fn end_of(seg: Seg) -> Pt {
    match seg {
        Seg::Move(p) | Seg::Line(p) | Seg::Quad(_, p) | Seg::Cubic(_, _, p) | Seg::Arc(p, _, _) => p,
    }
}

/// How long a command is from `from`, a corner and a hop as their chords.
fn length_of(from: Pt, seg: &Seg) -> f32 {
    match *seg {
        Seg::Move(_) => 0.0,
        Seg::Line(p) | Seg::Quad(_, p) | Seg::Arc(p, _, _) => (p.x - from.x).hypot(p.y - from.y),
        Seg::Cubic(c1, c2, p) => {
            let mut was = from;
            let mut sum = 0.0;
            for k in 1..=CUBIC_CUTS {
                let at = cubic_at(from, c1, c2, p, k as f32 / CUBIC_CUTS as f32);
                sum += (at.x - was.x).hypot(at.y - was.y);
                was = at;
            }
            sum
        }
    }
}

/// `path` with a hop of radius `r` wherever it crosses a way in `under`: a semicircle in place of the run, so the way
/// laid first stays whole. **No hop falls within `keep` of an end of the path**, where a head wants its run straight,
/// nor on a corner, nor so near another hop that the two would touch.
pub(crate) fn hop(path: &[Seg], under: &[Vec<Pt>], r: f32, keep: (f32, f32)) -> Vec<Seg> {
    if under.is_empty() || r <= 0.0 {
        return path.to_vec();
    }
    let mut total = 0.0;
    let mut cursor = Pt::default();
    for seg in path {
        total += length_of(cursor, seg);
        cursor = end_of(*seg);
    }
    // Whether a hop centred `d` along a run `long` that starts `base` along the path fits clear of the ends.
    let clear = |base: f32, d: f32, long: f32| d - r >= HOP_MARGIN && d + r <= long - HOP_MARGIN && base + d - r >= keep.0 && base + d + r <= total - keep.1;

    let mut out: Vec<Seg> = Vec::new();
    let mut cursor = Pt::default();
    let mut base = 0.0;
    for seg in path {
        let long = length_of(cursor, seg);
        match *seg {
            Seg::Line(p) if long > 0.0 => {
                let mut at: Vec<f32> = under.iter().flat_map(|one| crossings(cursor, p, one)).map(|t| t * long).filter(|d| clear(base, *d, long)).collect();
                at.sort_by(f32::total_cmp);
                let (ux, uy) = ((p.x - cursor.x) / long, (p.y - cursor.y) / long);
                let on = |d: f32| Pt { x: cursor.x + ux * d, y: cursor.y + uy * d };
                let mut spent: Option<f32> = None;
                for d in at {
                    if spent.is_some_and(|spent| d - r < spent + HOP_MARGIN) {
                        continue;
                    }
                    let (a, b) = (on(d - r), on(d + r));
                    out.push(Seg::Line(a));
                    out.push(Seg::Arc(b, r, hop_sweep(a, b)));
                    spent = Some(d + r);
                }
                out.push(Seg::Line(p));
            }
            Seg::Cubic(c1, c2, p) if long > 0.0 => {
                // The curve as chords, each with the distance along it where it starts.
                let samples: Vec<Pt> = std::iter::once(cursor).chain((1..=CUBIC_CUTS).map(|k| cubic_at(cursor, c1, c2, p, k as f32 / CUBIC_CUTS as f32))).collect();
                let mut walked = vec![0.0_f32];
                for pair in samples.windows(2) {
                    walked.push(walked.last().copied().unwrap_or(0.0) + (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y));
                }
                let mut at: Vec<f32> = Vec::new();
                for one in under {
                    for (k, pair) in samples.windows(2).enumerate() {
                        let chord = walked[k + 1] - walked[k];
                        at.extend(crossings(pair[0], pair[1], one).into_iter().map(|t| walked[k] + t * chord));
                    }
                }
                at.retain(|d| clear(base, *d, long));
                at.sort_by(f32::total_cmp);
                // The parameter at a distance along the curve, by its chords.
                let param = |d: f32| -> f32 {
                    let k = walked.iter().rposition(|w| *w <= d).unwrap_or(0).min(CUBIC_CUTS - 1);
                    let chord = (walked[k + 1] - walked[k]).max(1e-6);
                    (k as f32 + (d - walked[k]) / chord) / CUBIC_CUTS as f32
                };
                let mut t_from = 0.0_f32;
                let mut spent: Option<f32> = None;
                for d in at {
                    if spent.is_some_and(|spent| d - r < spent + HOP_MARGIN) {
                        continue;
                    }
                    let (ta, tb) = (param(d - r), param(d + r));
                    let (a, b) = (cubic_at(cursor, c1, c2, p, ta), cubic_at(cursor, c1, c2, p, tb));
                    if ta > t_from {
                        out.push(cubic_part(cursor, c1, c2, p, t_from, ta));
                    }
                    out.push(Seg::Arc(b, (b.x - a.x).hypot(b.y - a.y) / 2.0, hop_sweep(a, b)));
                    t_from = tb;
                    spent = Some(d + r);
                }
                if spent.is_some() {
                    out.push(cubic_part(cursor, c1, c2, p, t_from, 1.0));
                } else {
                    out.push(*seg);
                }
            }
            _ => out.push(*seg),
        }
        base += long;
        cursor = end_of(*seg);
    }
    out
}

/// An arrow to route: its id, the ids of the two boxes it joins, how it runs and whether it carries a head at its
/// start and at its end.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct RouteEdge<'a> {
    pub id: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub path: ScenePath,
    pub heads: (bool, bool),
}

/// The corners of an arrowhead whose tip is `at`, `len` long and `half` either side of its axis, pointing away from `from`.
/// The tip is where the way ends: the way is clipped at the box's edge, and the head never reaches into the box.
pub(crate) fn head_triangle(at: Pt, from: Pt, len: f32, half: f32) -> Option<[Pt; 3]> {
    let (dx, dy) = (at.x - from.x, at.y - from.y);
    let long = dx.hypot(dy);
    if long <= 0.0 {
        return None;
    }
    let (ux, uy) = (dx / long, dy / long);
    let back = Pt { x: at.x - ux * len, y: at.y - uy * len };
    Some([at, Pt { x: back.x - uy * half, y: back.y + ux * half }, Pt { x: back.x + uy * half, y: back.y - ux * half }])
}

/// The pair of sides a straight run joins: the one whose line meets both boxes nearest square, so a head, which is
/// as wide as it is long, lies outside the box it points at. Where no pair leaves and arrives outwards, the sides
/// that face each other.
fn straight_sides(from: Rect, to: Rect) -> (Side, Side) {
    let mut best: Option<(Side, Side)> = None;
    let mut least = f32::INFINITY;
    for out in SIDES {
        for into in SIDES {
            let (a, b) = (side_of(from, out), side_of(to, into));
            let long = (b.x - a.x).hypot(b.y - a.y);
            if long <= 0.0 {
                continue;
            }
            let (ux, uy) = ((b.x - a.x) / long, (b.y - a.y) / long);
            let leave = ux * out_of(out).x + uy * out_of(out).y;
            let arrive = -(ux * out_of(into).x + uy * out_of(into).y);
            if leave <= 0.0 || arrive <= 0.0 {
                continue;
            }
            let lean = leave.min(arrive).clamp(-1.0, 1.0).acos();
            if lean < least - 1e-4 {
                least = lean;
                best = Some((out, into));
            }
        }
    }
    best.unwrap_or_else(|| facing(from, to))
}

/// The sides of two boxes that face each other: across where they are further apart than down, else down.
fn facing(from: Rect, to: Rect) -> (Side, Side) {
    let across = (to.x + to.w / 2.0) - (from.x + from.w / 2.0);
    let down = (to.y + to.h / 2.0) - (from.y + from.h / 2.0);
    if across.abs() / ((from.w + to.w) / 2.0).max(1.0) >= down.abs() / ((from.h + to.h) / 2.0).max(1.0) {
        if across >= 0.0 { (Side::Right, Side::Left) } else { (Side::Left, Side::Right) }
    } else if down >= 0.0 {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

/// A curve from `ends.start` out of its side to `ends.end` into its side: a straight stub of `need` where a head
/// stands, then one cubic that leaves and arrives square to the boxes. A box too near leaves less than `need`.
fn curve(ends: &Ends, need: (f32, f32)) -> Vec<Seg> {
    let (out, into) = (out_of(ends.out), out_of(ends.into));
    let far = (ends.end.x - ends.start.x).hypot(ends.end.y - ends.start.y);
    let (lead, tail) = (need.0.min(far * 0.4), need.1.min(far * 0.4));
    let a = Pt { x: ends.start.x + out.x * lead, y: ends.start.y + out.y * lead };
    let b = Pt { x: ends.end.x + into.x * tail, y: ends.end.y + into.y * tail };
    let k = (b.x - a.x).hypot(b.y - a.y) / 2.0;
    let mut segs = vec![Seg::Move(tenth_pt(ends.start))];
    if lead > 0.0 {
        segs.push(Seg::Line(tenth_pt(a)));
    }
    segs.push(Seg::Cubic(
        tenth_pt(Pt { x: a.x + out.x * k, y: a.y + out.y * k }),
        tenth_pt(Pt { x: b.x + into.x * k, y: b.y + into.y * k }),
        tenth_pt(b),
    ));
    if tail > 0.0 {
        segs.push(Seg::Line(tenth_pt(ends.end)));
    }
    segs
}

/// An end of a way, by the box and side it is on, with where the box at its other end sits along that side.
struct End {
    edge: usize,
    start: bool,
    side: Side,
    along: f32,
}

/// Every arrow's way, by id in edge order, among boxes by id in the order given. Only boxes are in the way: a
/// group or a lane is named in `ground_ids`, and a box whose outline touches its frame only at each side's
/// middle in `pointed_ids`.
/// **Arrows that leave or reach one side of a box are spread along it**, in the order of the boxes at their other
/// ends, so two never meet at one point and their heads never pile up.
/// **`run` is how far a way goes straight into a head** (the elbows add their corner's radius to it), and `radius`
/// rounds the elbows' corners. An arrow that runs `Straight` joins the sides of its boxes that face each other, and
/// one that runs as a `Curve` leaves and arrives by the sides an elbow would take.
#[allow(dead_code)]
pub(crate) fn route_all(
    rects: &[(String, Rect)],
    edges: &[RouteEdge<'_>],
    gap: f32,
    ground_ids: &HashSet<String>,
    pointed_ids: &HashSet<String>,
    run: f32,
    radius: f32,
) -> Vec<(String, Vec<Seg>)> {
    let rect_of: HashMap<&str, Rect> = rects.iter().map(|(id, r)| (id.as_str(), *r)).collect();
    let others_of = |edge: &RouteEdge<'_>| -> Vec<Rect> {
        rects
            .iter()
            .filter(|(id, _)| {
                id.as_str() != edge.from && id.as_str() != edge.to && !ground_ids.contains(id)
            })
            .map(|(_, r)| *r)
            .collect()
    };
    // The edges whose two boxes are both here, with those two boxes.
    let joined: Vec<(&RouteEdge<'_>, Rect, Rect)> = edges
        .iter()
        .filter_map(|edge| {
            let from = rect_of.get(edge.from)?;
            let to = rect_of.get(edge.to)?;
            Some((edge, *from, *to))
        })
        .collect();
    // How far an elbow runs straight from each of its ends, and a curve: the head's run, and a corner's radius more for the elbow.
    let need = |edge: &RouteEdge<'_>, extra: f32| (if edge.heads.0 { run + extra } else { 0.0 }, if edge.heads.1 { run + extra } else { 0.0 });
    let first: Vec<Vec<Pt>> = joined
        .iter()
        .map(|(edge, from, to)| match edge.path {
            ScenePath::Straight => {
                let (out, into) = straight_sides(*from, *to);
                vec![side_of(*from, out), side_of(*to, into)]
            }
            _ => route(*from, *to, &others_of(edge), gap, None, need(edge, radius)),
        })
        .collect();

    let mut by_side: Vec<(&str, Vec<End>)> = Vec::new();
    let mut by_side_at: HashMap<(&str, Side), usize> = HashMap::new();
    for (k, (edge, from, to)) in joined.iter().enumerate() {
        let way = &first[k];
        for (start, box_id, r, o) in [(true, edge.from, from, to), (false, edge.to, to, from)] {
            let at = if start { way.first() } else { way.last() };
            let side = side_at(*r, at.copied().unwrap_or_default());
            let along = if side == Side::Left || side == Side::Right {
                o.y + o.h / 2.0
            } else {
                o.x + o.w / 2.0
            };
            let slot = *by_side_at.entry((box_id, side)).or_insert_with(|| {
                by_side.push((box_id, Vec::new()));
                by_side.len() - 1
            });
            by_side[slot].1.push(End {
                edge: k,
                start,
                side,
                along,
            });
        }
    }
    let mut moved: Vec<Ends> = joined
        .iter()
        .map(|_| Ends {
            out: Side::Right,
            into: Side::Left,
            start: Pt::default(),
            end: Pt::default(),
        })
        .collect();
    for (box_id, ends) in &mut by_side {
        let r = rect_of.get(*box_id).copied().unwrap_or_default();
        ends.sort_by(|a, b| {
            a.along
                .partial_cmp(&b.along)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let count = ends.len();
        for (k, one) in ends.iter().enumerate() {
            // A rounded or pointed outline meets its frame only at each side's middle, so its arrows are not spread.
            let share = if pointed_ids.contains(*box_id) {
                0.5
            } else {
                (k + 1) as f32 / (count + 1) as f32
            };
            let point = if one.side == Side::Left || one.side == Side::Right {
                Pt {
                    x: if one.side == Side::Left { r.x } else { r.x + r.w },
                    y: r.y + r.h * share,
                }
            } else {
                Pt {
                    x: r.x + r.w * share,
                    y: if one.side == Side::Top { r.y } else { r.y + r.h },
                }
            };
            let held = &mut moved[one.edge];
            if one.start {
                held.out = one.side;
                held.start = point;
            } else {
                held.into = one.side;
                held.end = point;
            }
        }
    }
    // An arrow between two facing sides keeps one straight run: where one end is alone on its side and the other is
    // spread, the lone end moves to the other's offset, so long as that is still inside its box. A box with a
    // pointed outline meets its arrows at its middle only, so its end stays.
    let mut crowd: HashMap<(&str, Side), usize> = HashMap::new();
    for (box_id, ends) in &by_side {
        for one in ends {
            *crowd.entry((*box_id, one.side)).or_insert(0) += 1;
        }
    }
    for (k, (edge, from, to)) in joined.iter().enumerate() {
        let held = moved[k];
        let facing_sides = matches!((held.out, held.into), (Side::Right, Side::Left) | (Side::Left, Side::Right) | (Side::Top, Side::Bottom) | (Side::Bottom, Side::Top));
        let alone = |box_id: &str, side: Side| crowd.get(&(box_id, side)).copied() == Some(1);
        let (start_alone, end_alone) = (alone(edge.from, held.out), alone(edge.to, held.into));
        if !facing_sides || start_alone == end_alone {
            continue;
        }
        let across = held.out == Side::Right || held.out == Side::Left;
        let inside = |r: Rect, at: Pt| if across { at.y > r.y && at.y < r.y + r.h } else { at.x > r.x && at.x < r.x + r.w };
        if start_alone && !pointed_ids.contains(edge.from) {
            let at = if across { Pt { x: held.start.x, y: held.end.y } } else { Pt { x: held.end.x, y: held.start.y } };
            if inside(*from, at) {
                moved[k].start = at;
            }
        } else if end_alone && !pointed_ids.contains(edge.to) {
            let at = if across { Pt { x: held.end.x, y: held.start.y } } else { Pt { x: held.start.x, y: held.end.y } };
            if inside(*to, at) {
                moved[k].end = at;
            }
        }
    }
    joined
        .iter()
        .enumerate()
        .map(|(k, (edge, from, to))| {
            let ways = match edge.path {
                ScenePath::Elbow => rounded_path(&route(*from, *to, &others_of(edge), gap, Some(&moved[k]), need(edge, radius)), radius),
                ScenePath::Straight => vec![Seg::Move(tenth_pt(moved[k].start)), Seg::Line(tenth_pt(moved[k].end))],
                ScenePath::Curve => curve(&moved[k], need(edge, 0.0)),
            };
            (edge.id.to_string(), ways)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn pt(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }

    #[test]
    fn side_by_side_boxes_route_right_to_left_straight() {
        let way = route(
            rect(0.0, 0.0, 100.0, 50.0),
            rect(200.0, 0.0, 100.0, 50.0),
            &[],
            40.0,
            None,
            (0.0, 0.0),
        );
        assert_eq!(way, vec![pt(100.0, 25.0), pt(200.0, 25.0)]);
    }

    #[test]
    fn a_box_in_between_forces_a_detour() {
        let middle = rect(150.0, 0.0, 100.0, 50.0);
        let way = route(
            rect(0.0, 0.0, 100.0, 50.0),
            rect(300.0, 0.0, 100.0, 50.0),
            &[middle],
            40.0,
            None,
            (0.0, 0.0),
        );
        assert!(way.len() > 2);
        for pair in way.windows(2) {
            assert!(!cuts(pair[0], pair[1], middle));
        }
    }

    #[test]
    fn rounded_path_cuts_each_corner_by_the_radius() {
        let segs = rounded_path(&[pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0)], 10.0);
        assert_eq!(
            segs,
            vec![
                Seg::Move(pt(0.0, 0.0)),
                Seg::Line(pt(90.0, 0.0)),
                Seg::Quad(pt(100.0, 0.0), pt(100.0, 10.0)),
                Seg::Line(pt(100.0, 100.0)),
            ]
        );
    }

    fn edge<'a>(id: &'a str, from: &'a str, to: &'a str, path: ScenePath, heads: (bool, bool)) -> RouteEdge<'a> {
        RouteEdge { id, from, to, path, heads }
    }

    fn ways_of(rects: &[(String, Rect)], edges: &[RouteEdge<'_>]) -> Vec<(String, Vec<Seg>)> {
        route_all(rects, edges, 32.0, &HashSet::new(), &HashSet::new(), 32.0, 8.0)
    }

    fn long(a: Pt, b: Pt) -> f32 {
        (b.x - a.x).hypot(b.y - a.y)
    }

    fn two_boxes(second: Rect) -> Vec<(String, Rect)> {
        vec![("a".to_string(), rect(0.0, 0.0, 100.0, 50.0)), ("b".to_string(), second)]
    }

    /// The straight run into the end of an elbow: the last run less the corner rounded off it.
    fn run_in(segs: &[Seg]) -> f32 {
        let Some(&Seg::Line(end)) = segs.last() else { return 0.0 };
        let at = segs.len() - 2;
        let before = match segs[at] {
            Seg::Quad(_, p) | Seg::Line(p) | Seg::Move(p) | Seg::Cubic(_, _, p) | Seg::Arc(p, _, _) => p,
        };
        long(before, end)
    }

    #[test]
    fn route_all_gives_a_lone_pair_the_straight_run() {
        let rects = two_boxes(rect(200.0, 0.0, 100.0, 50.0));
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (false, true))]);
        assert_eq!(ways, vec![("e".to_string(), vec![Seg::Move(pt(100.0, 25.0)), Seg::Line(pt(200.0, 25.0))])]);
    }

    #[test]
    fn an_elbow_runs_straight_two_head_lengths_into_its_head() {
        let rects = two_boxes(rect(300.0, 200.0, 100.0, 50.0));
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (false, true))]);
        let segs = &ways[0].1;
        assert!(segs.iter().any(|seg| matches!(seg, Seg::Quad(..))), "an elbow with a corner: {segs:?}");
        assert!(run_in(segs) >= 32.0, "{} short of two heads: {segs:?}", run_in(segs));
    }

    #[test]
    fn a_start_head_gets_the_same_straight_run_out_of_its_box() {
        let rects = two_boxes(rect(300.0, 200.0, 100.0, 50.0));
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (true, true))]);
        let segs = &ways[0].1;
        let (Some(&Seg::Move(start)), Some(&Seg::Line(next))) = (segs.first(), segs.get(1)) else { panic!("{segs:?}") };
        assert!(long(start, next) >= 32.0, "{segs:?}");
        assert!(run_in(segs) >= 32.0, "{segs:?}");
    }

    #[test]
    fn an_end_with_no_head_keeps_its_short_run() {
        let rects = two_boxes(rect(300.0, 200.0, 100.0, 50.0));
        let headed = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (false, true))]);
        let bare = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (false, false))]);
        assert!(run_in(&bare[0].1) < 32.0 || bare[0].1.len() <= 2, "{:?} against {:?}", bare[0].1, headed[0].1);
    }

    #[test]
    fn boxes_too_near_for_the_run_keep_the_short_one_and_cut_no_box() {
        let to = rect(132.0, 80.0, 100.0, 50.0);
        let rects = two_boxes(to);
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Elbow, (false, true))]);
        let points = polyline(&ways[0].1);
        for pair in points.windows(2).skip(1).take(points.len().saturating_sub(3)) {
            assert!(!cuts(pair[0], pair[1], to), "{points:?}");
        }
    }

    #[test]
    fn a_curve_leaves_and_arrives_square_with_a_straight_run_into_its_head() {
        let rects = two_boxes(rect(300.0, 200.0, 100.0, 50.0));
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Curve, (false, true))]);
        let segs = &ways[0].1;
        assert!(segs.iter().any(|seg| matches!(seg, Seg::Cubic(..))), "{segs:?}");
        assert!(run_in(segs) >= 32.0, "{segs:?}");
        // The head points the way the last run goes: square to the side it arrives at.
        let tips = tips(segs).expect("a way");
        assert!(tips.end.x == tips.before_end.x || tips.end.y == tips.before_end.y, "{tips:?}");
    }

    #[test]
    fn a_straight_path_joins_the_facing_sides_in_one_run() {
        let rects = two_boxes(rect(300.0, 200.0, 100.0, 50.0));
        let ways = ways_of(&rects, &[edge("e", "a", "b", ScenePath::Straight, (false, true))]);
        let segs = &ways[0].1;
        assert_eq!(segs.len(), 2, "{segs:?}");
        assert!(matches!(segs[0], Seg::Move(_)) && matches!(segs[1], Seg::Line(_)), "{segs:?}");
        let Seg::Move(start) = segs[0] else { return };
        let Seg::Line(end) = segs[1] else { return };
        // The line meets both boxes nearest square from the right side of one to the left side of the other.
        assert_eq!(start, pt(100.0, 25.0), "leaves the side that meets it nearest square");
        assert_eq!(end, pt(300.0, 225.0));
        let level = ways_of(&two_boxes(rect(300.0, 0.0, 100.0, 50.0)), &[edge("e", "a", "b", ScenePath::Straight, (false, true))]);
        assert_eq!(level[0].1, vec![Seg::Move(pt(100.0, 25.0)), Seg::Line(pt(300.0, 25.0))]);
    }

    fn inside(r: Rect, p: Pt) -> bool {
        p.x > r.x + 0.01 && p.x < r.x + r.w - 0.01 && p.y > r.y + 0.01 && p.y < r.y + r.h - 0.01
    }

    fn on_edge(r: Rect, p: Pt) -> bool {
        let across = p.x >= r.x - 0.01 && p.x <= r.x + r.w + 0.01;
        let down = p.y >= r.y - 0.01 && p.y <= r.y + r.h + 0.01;
        ((p.x - r.x).abs() < 0.01 || (p.x - r.x - r.w).abs() < 0.01) && down || ((p.y - r.y).abs() < 0.01 || (p.y - r.y - r.h).abs() < 0.01) && across
    }

    #[test]
    fn a_diagonal_arrows_tip_lies_on_the_target_boxes_edge_and_its_head_never_inside_it() {
        let boxes = [("push", rect(0.0, 0.0, 140.0, 50.0)), ("gate", rect(420.0, 0.0, 140.0, 50.0)), ("merge", rect(660.0, 0.0, 140.0, 50.0)), ("checks", rect(0.0, 230.0, 140.0, 50.0)), ("review", rect(420.0, 230.0, 140.0, 50.0)), ("land", rect(660.0, 230.0, 140.0, 50.0))];
        let rects: Vec<(String, Rect)> = boxes.iter().map(|(id, r)| (id.to_string(), *r)).collect();
        let at = |id: &str| boxes.iter().find(|(one, _)| *one == id).map(|(_, r)| *r).expect("a box");
        for (from, to, id) in [("push", "land", "a"), ("checks", "merge", "b"), ("push", "review", "c"), ("land", "push", "d")] {
            let ways = ways_of(&rects, &[edge(id, from, to, ScenePath::Straight, (true, true))]);
            let segs = &ways[0].1;
            let tips = tips(segs).expect("a way");
            assert!(on_edge(at(to), tips.end), "{from} to {to}: the tip {:?} is on {:?}'s edge", tips.end, at(to));
            assert!(on_edge(at(from), tips.start), "{from} to {to}: the start {:?} is on {:?}'s edge", tips.start, at(from));
            for (point, from_point, r) in [(tips.end, tips.before_end, at(to)), (tips.start, tips.after_start, at(from))] {
                let head = head_triangle(point, from_point, 16.0, 8.0).expect("a head");
                assert!(head.iter().all(|corner| !inside(r, *corner)), "{from} to {to}: a head corner {head:?} is inside {r:?}");
            }
        }
    }

    #[test]
    fn an_arrow_from_a_shared_side_to_a_lone_one_is_drawn_as_one_run_with_no_jog() {
        let rects = vec![
            ("a".to_string(), rect(0.0, 0.0, 100.0, 50.0)),
            ("b".to_string(), rect(300.0, 0.0, 100.0, 50.0)),
            ("c".to_string(), rect(300.0, 120.0, 100.0, 50.0)),
        ];
        for path in [ScenePath::Straight, ScenePath::Elbow] {
            let ways = ways_of(&rects, &[edge("ab", "a", "b", path, (false, true)), edge("ac", "a", "c", ScenePath::Elbow, (false, true))]);
            let segs = &ways[0].1;
            assert_eq!(segs.len(), 2, "{path:?}: one segment, no jog: {segs:?}");
            let (Seg::Move(start), Seg::Line(end)) = (segs[0], segs[1]) else { panic!("{segs:?}") };
            assert_eq!(start.y, end.y, "{path:?}: level");
            assert!(start.x == 100.0 && end.x == 300.0 && end.y > 0.0 && end.y < 50.0, "{path:?}: inside the lone box's side: {segs:?}");
            assert_ne!(start.y, 25.0, "the start stays where the spread put it");
        }
    }

    #[test]
    fn where_both_ends_share_their_sides_the_spread_stands() {
        let rects = vec![
            ("a".to_string(), rect(0.0, 0.0, 100.0, 50.0)),
            ("b".to_string(), rect(300.0, 0.0, 100.0, 50.0)),
            ("c".to_string(), rect(0.0, -120.0, 100.0, 50.0)),
            ("d".to_string(), rect(300.0, 120.0, 100.0, 50.0)),
        ];
        let ways = ways_of(&rects, &[edge("ab", "a", "b", ScenePath::Straight, (false, true)), edge("ad", "a", "d", ScenePath::Straight, (false, true)), edge("cb", "c", "b", ScenePath::Straight, (false, true))]);
        let ends = |segs: &[Seg]| match (segs[0], segs[1]) {
            (Seg::Move(a), Seg::Line(b)) => (a.y, b.y),
            _ => panic!("{segs:?}"),
        };
        let (start, end) = ends(&ways[0].1);
        assert_ne!(start, end, "both sides are spread, so the arrow keeps its slope: {:?}", ways[0].1);
    }

    fn run_of(from: Pt, to: Pt) -> Vec<Seg> {
        vec![Seg::Move(from), Seg::Line(to)]
    }

    fn arcs(path: &[Seg]) -> usize {
        path.iter().filter(|seg| matches!(seg, Seg::Arc(..))).count()
    }

    #[test]
    fn a_crossing_is_one_hop_on_the_later_arrow_and_the_earlier_stays_whole() {
        let first = run_of(pt(0.0, 50.0), pt(100.0, 50.0));
        let second = run_of(pt(50.0, 0.0), pt(50.0, 100.0));
        let drawn = hop_all(&[Some(first.clone()), Some(second)], 4.0, &[(0.0, 0.0), (0.0, 0.0)]);
        assert_eq!(drawn[0].as_deref(), Some(first.as_slice()), "the earlier arrow is not broken");
        let later = drawn[1].as_deref().expect("a way");
        assert_eq!(arcs(later), 1, "{later:?}");
        assert_eq!(
            later,
            [Seg::Move(pt(50.0, 0.0)), Seg::Line(pt(50.0, 46.0)), Seg::Arc(pt(50.0, 54.0), 4.0, hop_sweep(pt(50.0, 46.0), pt(50.0, 54.0))), Seg::Line(pt(50.0, 100.0))]
        );
    }

    #[test]
    fn the_arrow_drawn_later_is_the_one_that_hops() {
        let across = run_of(pt(0.0, 50.0), pt(100.0, 50.0));
        let down = run_of(pt(50.0, 0.0), pt(50.0, 100.0));
        let drawn = hop_all(&[Some(down.clone()), Some(across)], 4.0, &[(0.0, 0.0), (0.0, 0.0)]);
        assert_eq!(drawn[0].as_deref(), Some(down.as_slice()));
        assert_eq!(arcs(drawn[1].as_deref().expect("a way")), 1);
    }

    #[test]
    fn a_hop_bulges_up_on_a_run_across_and_right_on_a_run_down() {
        let across = hop(&run_of(pt(0.0, 50.0), pt(100.0, 50.0)), &[vec![pt(50.0, 0.0), pt(50.0, 100.0)]], 4.0, (0.0, 0.0));
        let backwards = hop(&run_of(pt(100.0, 50.0), pt(0.0, 50.0)), &[vec![pt(50.0, 0.0), pt(50.0, 100.0)]], 4.0, (0.0, 0.0));
        let top = |path: &[Seg]| {
            let (Some(&Seg::Line(a)), Some(&Seg::Arc(b, _, sweep))) = (path.get(1), path.get(2)) else { panic!("{path:?}") };
            arc_points(a, b, sweep, 8).iter().map(|p| p.y).fold(f32::INFINITY, f32::min)
        };
        assert!(top(&across) < 50.0 - 3.9 && top(&backwards) < 50.0 - 3.9, "both hop up, whichever way they run");
        let down = hop(&run_of(pt(50.0, 0.0), pt(50.0, 100.0)), &[vec![pt(0.0, 50.0), pt(100.0, 50.0)]], 4.0, (0.0, 0.0));
        let (Some(&Seg::Line(a)), Some(&Seg::Arc(b, _, sweep))) = (down.get(1), down.get(2)) else { panic!("{down:?}") };
        assert!(arc_points(a, b, sweep, 8).iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max) > 50.0 + 3.9);
    }

    #[test]
    fn a_crossing_inside_the_straight_run_of_a_head_is_not_hopped() {
        let way = run_of(pt(0.0, 50.0), pt(100.0, 50.0));
        let under = [vec![pt(90.0, 0.0), pt(90.0, 100.0)]];
        assert_eq!(arcs(&hop(&way, &under, 4.0, (0.0, 32.0))), 0);
        assert_eq!(arcs(&hop(&way, &under, 4.0, (0.0, 0.0))), 1);
        let near_start = [vec![pt(10.0, 0.0), pt(10.0, 100.0)]];
        assert_eq!(arcs(&hop(&way, &near_start, 4.0, (32.0, 0.0))), 0);
    }

    #[test]
    fn a_crossing_on_a_corner_is_not_hopped() {
        let corner = rounded_path(&[pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0)], 8.0);
        let under = [vec![pt(98.0, -10.0), pt(98.0, 10.0)]];
        assert_eq!(arcs(&hop(&corner, &under, 4.0, (0.0, 0.0))), 0);
    }

    #[test]
    fn two_crossings_close_together_do_not_hop_into_each_other() {
        let way = run_of(pt(0.0, 50.0), pt(100.0, 50.0));
        let under = [vec![pt(50.0, 0.0), pt(50.0, 100.0)], vec![pt(54.0, 0.0), pt(54.0, 100.0)]];
        assert_eq!(arcs(&hop(&way, &under, 4.0, (0.0, 0.0))), 1);
    }

    #[test]
    fn a_curve_hops_once_and_still_ends_where_it_did() {
        let curve = vec![Seg::Move(pt(0.0, 0.0)), Seg::Cubic(pt(60.0, 0.0), pt(40.0, 100.0), pt(100.0, 100.0))];
        let hopped = hop(&curve, &[vec![pt(50.0, -20.0), pt(50.0, 120.0)]], 4.0, (0.0, 0.0));
        assert_eq!(arcs(&hopped), 1, "{hopped:?}");
        assert_eq!(hopped.last().map(|seg| end_of(*seg)), Some(pt(100.0, 100.0)));
        assert_eq!(tips(&hopped).map(|t| t.end), Some(pt(100.0, 100.0)));
    }
}
