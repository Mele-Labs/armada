//! Ported from `classic-final:packages/components/src/compositions/SketchScene/edge-labels.ts`.

use super::geometry::{Pt, Rect};
use super::route::Seg;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct LabelMetrics {
    pub char_w: f32,
    pub line_h: f32,
    pub pad: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelPlace {
    pub lines: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub line_h: f32,
    pub pad: f32,
}

/// Characters to a line, and lines to a label. What does not fit is cut and the full text stays in the accessible name.
#[allow(dead_code)]
pub(crate) const LABEL_CHARS: usize = 20;
#[allow(dead_code)]
pub(crate) const LABEL_LINES: usize = 2;

#[allow(dead_code)]
pub(crate) fn wrap_label(text: &str) -> Vec<String> {
    let words: Vec<String> = text
        .split_whitespace()
        .flat_map(|word| {
            word.chars()
                .collect::<Vec<char>>()
                .chunks(LABEL_CHARS)
                .map(|part| part.iter().collect::<String>())
                .collect::<Vec<String>>()
        })
        .collect();
    let mut lines: Vec<String> = Vec::new();
    let mut cut = false;
    for word in words {
        let fits = lines
            .last()
            .is_some_and(|last| last.chars().count() + 1 + word.chars().count() <= LABEL_CHARS);
        if fits {
            if let Some(last) = lines.last_mut() {
                last.push(' ');
                last.push_str(&word);
            }
        } else if lines.len() < LABEL_LINES {
            lines.push(word);
        } else {
            cut = true;
            break;
        }
    }
    if cut {
        if let Some(last) = lines.last_mut() {
            let kept: String = last.chars().take(LABEL_CHARS - 1).collect();
            *last = format!("{}…", kept.trim_end());
        }
    }
    lines
}

/// The point `t` of the way along a routed way: its runs end where each segment ends. A rounded corner is short,
/// so it is read as its chord.
#[allow(dead_code)]
pub(crate) fn path_at(path: &[Seg], t: f32) -> Option<Pt> {
    let points: Vec<Pt> = super::route::polyline(path);
    if points.is_empty() {
        return None;
    }
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .collect();
    let mut left = lengths.iter().fold(0.0_f32, |sum, one| sum + one) * t;
    for (at, &length) in lengths.iter().enumerate() {
        if left <= length && length > 0.0 {
            let a = points[at];
            let b = points[at + 1];
            return Some(Pt {
                x: a.x + ((b.x - a.x) * left) / length,
                y: a.y + ((b.y - a.y) * left) / length,
            });
        }
        left -= length;
    }
    points.last().copied()
}

fn overlap(a: &Rect, b: &Rect) -> f32 {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    if w > 0.0 && h > 0.0 {
        w * h
    } else {
        0.0
    }
}

/// Middle first, then outwards either side of it.
const ALONG: [f32; 13] = [
    0.5, 0.44, 0.56, 0.38, 0.62, 0.32, 0.68, 0.26, 0.74, 0.2, 0.8, 0.14, 0.86,
];

/// How big an arrow's words are drawn, on their backing.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelSize {
    pub lines: Vec<String>,
    pub w: f32,
    pub h: f32,
}

#[allow(dead_code)]
pub(crate) fn label_size(label: &str, m: LabelMetrics) -> LabelSize {
    let lines = wrap_label(label);
    let widest = lines
        .iter()
        .map(|one| one.chars().count() as f32)
        .fold(f32::NEG_INFINITY, f32::max);
    LabelSize {
        w: widest * m.char_w + m.pad * 2.0,
        h: lines.len() as f32 * m.line_h + m.pad * 2.0,
        lines,
    }
}

/// An arrow whose words are to be placed: its way as drawn, and its words if it has any.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct LabelEdge<'a> {
    pub id: &'a str,
    pub label: Option<&'a str>,
    pub path: &'a [Seg],
}

/// A straight run of a way: where it starts and ends, and whether it goes across.
#[derive(Clone, Copy, Debug)]
pub(super) struct Straight {
    pub(super) from: Pt,
    pub(super) to: Pt,
    pub(super) across: bool,
}

impl Straight {
    pub(super) fn len(&self) -> f32 {
        (self.to.x - self.from.x).hypot(self.to.y - self.from.y)
    }

    fn at(&self, along: f32) -> Pt {
        let k = along / self.len().max(f32::EPSILON);
        Pt { x: self.from.x + (self.to.x - self.from.x) * k, y: self.from.y + (self.to.y - self.from.y) * k }
    }
}

/// The straight runs of a way, as drawn: what lies between its rounded corners, its hops and its curves.
pub(super) fn straights(path: &[Seg]) -> Vec<Straight> {
    let mut runs = Vec::new();
    let mut here = Pt::default();
    for seg in path {
        match *seg {
            Seg::Move(p) => here = p,
            Seg::Line(p) => {
                let run = Straight { from: here, to: p, across: (p.x - here.x).abs() >= (p.y - here.y).abs() };
                if run.len() > 1.0 && ((p.x - here.x).abs() < 0.01 || (p.y - here.y).abs() < 0.01) {
                    runs.push(run);
                }
                here = p;
            }
            Seg::Quad(_, p) | Seg::Cubic(_, _, p) | Seg::Arc(p, _, _) => here = p,
        }
    }
    runs
}

/// What keeps words off the corners: the room kept at each end of a run, beyond half the words.
const RUN_MARGIN: f32 = 6.0;

/// Whether any run of `way` goes through `rect`.
fn goes_through(way: &[Seg], rect: &Rect) -> bool {
    let points = super::route::polyline(way);
    points.windows(2).any(|pair| {
        pair[0].x.max(pair[1].x) > rect.x && pair[0].x.min(pair[1].x) < rect.x + rect.w && pair[0].y.max(pair[1].y) > rect.y && pair[0].y.min(pair[1].y) < rect.y + rect.h
    })
}

/// Each arrow's words, on a straight run of its line where they fit and beside it where they do not, by id in edge order.
/// `heads` are where arrows end, which the words keep off as they keep off boxes, and the words keep off the lines of other arrows.
#[allow(dead_code)]
pub(crate) fn place_labels(
    drawn: &[LabelEdge<'_>],
    solid: &[Rect],
    m: LabelMetrics,
    heads: &[Rect],
) -> Vec<(String, LabelPlace)> {
    let mut placed: Vec<(String, LabelPlace)> = Vec::new();
    let hard: Vec<Rect> = solid.iter().chain(heads.iter()).copied().collect();
    let mut taken: Vec<Rect> = Vec::new();
    for edge in drawn {
        let Some(label) = edge.label else { continue };
        if label.trim().is_empty() {
            continue;
        }
        let LabelSize { lines, w, h } = label_size(label, m);
        // Where the words may stand: the middle of each straight run they fit in, and either side of it, the longest run first.
        let mut spots: Vec<(Pt, bool)> = Vec::new();
        let mut runs = straights(edge.path);
        runs.sort_by(|a, b| b.len().total_cmp(&a.len()));
        for run in runs {
            let need = if run.across { w } else { h } + 2.0 * RUN_MARGIN;
            if run.len() < need {
                continue;
            }
            let room = run.len() - need;
            for share in [0.0_f32, 0.25, -0.25, 0.5, -0.5] {
                spots.push((run.at(run.len() / 2.0 + share * room), run.across));
            }
        }
        // No run is long enough for them: the middle of the longest, with the words past its ends, rather than on a corner.
        if spots.is_empty() {
            if let Some(run) = straights(edge.path).into_iter().max_by(|a, b| a.len().total_cmp(&b.len())) {
                spots.push((run.at(run.len() / 2.0), run.across));
            }
        }
        // A way with no straight run at all, a curve, has them along it as before.
        if spots.is_empty() {
            for t in ALONG {
                let (at, ahead, behind) = (path_at(edge.path, t), path_at(edge.path, (t + 0.02).min(1.0)), path_at(edge.path, (t - 0.02).max(0.0)));
                if let (Some(at), Some(ahead), Some(behind)) = (at, ahead, behind) {
                    spots.push((at, (ahead.x - behind.x).abs() >= (ahead.y - behind.y).abs()));
                }
            }
        }
        let mut best: Option<LabelPlace> = None;
        let mut best_cost = f32::INFINITY;
        let mut tried = 0_u32;
        for (at, across) in spots {
            // Across the line, by half the words and a little: above or below a run across, beside a run down.
            let step = if across { Pt { x: 0.0, y: h / 2.0 + m.pad } } else { Pt { x: w / 2.0 + m.pad, y: 0.0 } };
            for side in [0.0_f32, 1.0, -1.0] {
                let x = at.x + step.x * side;
                let y = at.y + step.y * side;
                let rect = Rect { x: x - w / 2.0, y: y - h / 2.0, w, h };
                // A hit on a box or a head costs more than one on other words, and another arrow's line through them more than a
                // little; distance from the middle breaks ties.
                let cost = hard.iter().fold(0.0_f32, |sum, one| sum + overlap(&rect, one) * 4.0)
                    + taken.iter().fold(0.0_f32, |sum, one| sum + overlap(&rect, one))
                    + drawn.iter().filter(|other| other.id != edge.id && goes_through(other.path, &rect)).count() as f32 * 400.0
                    + tried as f32;
                tried += 1;
                if cost < best_cost {
                    best_cost = cost;
                    best = Some(LabelPlace { lines: lines.clone(), x, y, w, h, line_h: m.line_h, pad: m.pad });
                }
            }
            if best_cost < tried as f32 {
                break;
            }
        }
        let Some(best) = best else { continue };
        taken.push(Rect { x: best.x - best.w / 2.0, y: best.y - best.h / 2.0, w: best.w, h: best.h });
        placed.push((edge.id.to_string(), best));
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: LabelMetrics = LabelMetrics {
        char_w: 6.0,
        line_h: 12.0,
        pad: 4.0,
    };

    #[test]
    fn a_short_label_is_one_line() {
        assert_eq!(wrap_label("  send   now "), vec!["send now".to_string()]);
    }

    #[test]
    fn a_label_past_two_lines_is_cut_with_an_ellipsis() {
        assert_eq!(
            wrap_label("aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii"),
            vec!["aaaa bbbb cccc dddd".to_string(), "eeee ffff gggg hhhh…".to_string()]
        );
    }

    #[test]
    fn a_word_longer_than_a_line_is_split_and_then_cut() {
        let long = "x".repeat(50);
        assert_eq!(
            wrap_label(&long),
            vec!["x".repeat(20), format!("{}…", "x".repeat(19))]
        );
    }

    #[test]
    fn words_sit_in_the_middle_of_a_clear_line() {
        let path = [
            Seg::Move(Pt { x: 0.0, y: 0.0 }),
            Seg::Line(Pt { x: 200.0, y: 0.0 }),
        ];
        let edges = [LabelEdge {
            id: "e",
            label: Some("go"),
            path: &path,
        }];
        let placed = place_labels(&edges, &[], METRICS, &[]);
        assert_eq!(
            placed,
            vec![(
                "e".to_string(),
                LabelPlace {
                    lines: vec!["go".to_string()],
                    x: 100.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                    line_h: 12.0,
                    pad: 4.0,
                }
            )]
        );
    }
}
