//! Ported from `packages/components/src/compositions/SketchScene/route.ts`.

use std::collections::{HashMap, HashSet};

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
#[allow(dead_code)]
pub(crate) fn route(
    from: Rect,
    to: Rect,
    others: &[Rect],
    gap: f32,
    ends: Option<&Ends>,
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
            let a = Pt {
                x: start.x + out_of(out).x * stub,
                y: start.y + out_of(out).y * stub,
            };
            let b = Pt {
                x: end.x + out_of(into).x * stub,
                y: end.y + out_of(into).y * stub,
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
                let mut cost = 0.0_f32;
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
    best.unwrap_or_else(|| vec![side_of(from, Side::Right), side_of(to, Side::Left)])
}

/// One command of a drawn way: a move, a run, or a rounded corner as its control point and its end.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Seg {
    Move(Pt),
    Line(Pt),
    Quad(Pt, Pt),
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

/// An arrow to route: its id and the ids of the two boxes it joins.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct RouteEdge<'a> {
    pub id: &'a str,
    pub from: &'a str,
    pub to: &'a str,
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
#[allow(dead_code)]
pub(crate) fn route_all(
    rects: &[(String, Rect)],
    edges: &[RouteEdge<'_>],
    gap: f32,
    ground_ids: &HashSet<String>,
    pointed_ids: &HashSet<String>,
) -> Vec<(String, Vec<Pt>)> {
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
    let first: Vec<Vec<Pt>> = joined
        .iter()
        .map(|(edge, from, to)| route(*from, *to, &others_of(edge), gap, None))
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
    joined
        .iter()
        .enumerate()
        .map(|(k, (edge, from, to))| {
            let way = route(*from, *to, &others_of(edge), gap, Some(&moved[k]));
            (edge.id.to_string(), way)
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

    #[test]
    fn route_all_gives_a_lone_pair_the_straight_run() {
        let rects = vec![
            ("a".to_string(), rect(0.0, 0.0, 100.0, 50.0)),
            ("b".to_string(), rect(200.0, 0.0, 100.0, 50.0)),
        ];
        let edges = [RouteEdge {
            id: "e",
            from: "a",
            to: "b",
        }];
        let ways = route_all(&rects, &edges, 40.0, &HashSet::new(), &HashSet::new());
        assert_eq!(
            ways,
            vec![("e".to_string(), vec![pt(100.0, 25.0), pt(200.0, 25.0)])]
        );
    }
}
