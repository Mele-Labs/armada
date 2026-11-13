//! Ported from `packages/components/src/compositions/SketchScene/settle.ts`.

use std::cmp::Ordering;

use super::geometry::Rect;

/// A box as the agent placed it, at its measured size. `ground` is a group or a lane, which holds boxes.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct SettleBox {
    pub id: String,
    pub rect: Rect,
    pub ground: bool,
}

/// How far apart two agent coordinates may be and still be one column or row.
const SAME_LINE: f32 = 40.0;

/// Two boxes an arrow with words joins, and the size of its words: the room the arrow needs between them.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct Join {
    pub from: String,
    pub to: String,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy)]
enum Axis {
    Rows,
    Columns,
}

impl Axis {
    fn at(self, r: &Rect) -> f32 {
        match self {
            Axis::Rows => r.y,
            Axis::Columns => r.x,
        }
    }

    fn size(self, r: &Rect) -> f32 {
        match self {
            Axis::Rows => r.h,
            Axis::Columns => r.w,
        }
    }

    fn across(self, r: &Rect) -> f32 {
        match self {
            Axis::Rows => r.x,
            Axis::Columns => r.y,
        }
    }

    fn span(self, r: &Rect) -> f32 {
        match self {
            Axis::Rows => r.w,
            Axis::Columns => r.h,
        }
    }

    fn shift(self, r: &mut Rect, by: f32) {
        match self {
            Axis::Rows => r.y += by,
            Axis::Columns => r.x += by,
        }
    }
}

fn order(a: f32, b: f32) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// The agent's lines along one axis: boxes whose starts are within `SAME_LINE` of the line's first.
fn lines_of(boxes: &[SettleBox], axis: Axis) -> Vec<Vec<SettleBox>> {
    let mut sorted: Vec<SettleBox> = boxes.to_vec();
    sorted.sort_by(|a, b| order(axis.at(&a.rect), axis.at(&b.rect)));
    let mut lines: Vec<Vec<SettleBox>> = Vec::new();
    for one in sorted {
        let same = lines
            .last()
            .and_then(|line| line.first())
            .is_some_and(|head| axis.at(&one.rect) - axis.at(&head.rect) <= SAME_LINE);
        match lines.last_mut() {
            Some(line) if same => line.push(one),
            _ => lines.push(vec![one]),
        }
    }
    lines
}

fn meets(a: &Rect, b: &Rect, axis: Axis, gap: f32) -> bool {
    axis.across(a) < axis.across(b) + axis.span(b) + gap
        && axis.across(b) < axis.across(a) + axis.span(a) + gap
}

/// Each line moved on as far as the boxes before it that it would meet need, and the whole line together.
fn spread(boxes: &[SettleBox], axis: Axis, gap: f32, joins: &[Join]) -> Vec<SettleBox> {
    // Two joined boxes keep the arrow's words and a gap either side of them between them.
    let room = |a: &str, b: &str| {
        joins
            .iter()
            .filter(|one| (one.from == a && one.to == b) || (one.from == b && one.to == a))
            .map(|one| {
                (match axis {
                    Axis::Rows => one.h,
                    Axis::Columns => one.w,
                }) + gap
            })
            .fold(0.0_f32, f32::max)
    };
    let mut done: Vec<SettleBox> = Vec::new();
    for line in lines_of(boxes, axis) {
        let mut shift = 0.0_f32;
        for one in &line {
            for before in &done {
                if meets(&one.rect, &before.rect, axis, gap) {
                    shift = shift.max(
                        axis.at(&before.rect)
                            + axis.size(&before.rect)
                            + gap
                            + room(&one.id, &before.id)
                            - axis.at(&one.rect),
                    );
                }
            }
        }
        for mut one in line {
            axis.shift(&mut one.rect, shift);
            done.push(one);
        }
    }
    done
}

fn least(values: impl Iterator<Item = f32>) -> f32 {
    values.fold(f32::INFINITY, f32::min)
}

fn most(values: impl Iterator<Item = f32>) -> f32 {
    values.fold(f32::NEG_INFINITY, f32::max)
}

fn touching(a: &Rect, b: &Rect, gap: f32) -> bool {
    a.x < b.x + b.w + gap && b.x < a.x + a.w + gap && a.y < b.y + b.h + gap && b.y < a.y + a.h + gap
}

/// Every box's settled rectangle, by id, solid boxes first and then the grounds. A group or a lane is grown round
/// the boxes it held.
#[allow(dead_code)]
pub(crate) fn settle(boxes: &[SettleBox], gap: f32, joins: &[Join]) -> Vec<(String, Rect)> {
    let solid: Vec<SettleBox> = boxes.iter().filter(|one| !one.ground).cloned().collect();
    let mut settled: Vec<(String, Rect)> = Vec::new();

    // Rows down first, then columns across, then anything still touching goes below what it touches.
    let mut moved = spread(&spread(&solid, Axis::Rows, gap, joins), Axis::Columns, gap, joins);
    moved.sort_by(|a, b| {
        order(a.rect.y, b.rect.y).then_with(|| order(a.rect.x, b.rect.x))
    });
    let mut down: Vec<Rect> = Vec::new();
    for mut one in moved {
        while let Some(clash) = down.iter().find(|other| touching(&one.rect, other, gap)) {
            one.rect.y = clash.y + clash.h + gap;
        }
        down.push(one.rect);
        settled.push((one.id, one.rect));
    }

    for ground in boxes.iter().filter(|one| one.ground) {
        let g = ground.rect;
        let held: Vec<&SettleBox> = solid
            .iter()
            .filter(|one| {
                let cx = one.rect.x + one.rect.w / 2.0;
                let cy = one.rect.y + one.rect.h / 2.0;
                cx >= g.x && cx <= g.x + g.w && cy >= g.y && cy <= g.y + g.h
            })
            .collect();
        if held.is_empty() {
            settled.push((ground.id.clone(), g));
            continue;
        }
        // The margins it had round what it held, kept; what it held now decides where it is and how big.
        let now: Vec<Rect> = held
            .iter()
            .map(|one| {
                settled
                    .iter()
                    .find(|(id, _)| *id == one.id)
                    .map_or(one.rect, |(_, rect)| *rect)
            })
            .collect();
        let left = least(held.iter().map(|one| one.rect.x - g.x));
        let top = least(held.iter().map(|one| one.rect.y - g.y));
        let right = (gap / 2.0).max(least(
            held.iter().map(|one| g.x + g.w - one.rect.x - one.rect.w),
        ));
        let bottom = (gap / 2.0).max(least(
            held.iter().map(|one| g.y + g.h - one.rect.y - one.rect.h),
        ));
        let x = least(now.iter().map(|one| one.x)) - left;
        let y = least(now.iter().map(|one| one.y)) - top;
        let w = g.w.max(most(now.iter().map(|one| one.x + one.w)) + right - x);
        let h = g.h.max(most(now.iter().map(|one| one.y + one.h)) + bottom - y);
        settled.push((ground.id.clone(), Rect { x, y, w, h }));
    }
    settled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(id: &str, x: f32, y: f32, w: f32, h: f32) -> SettleBox {
        SettleBox {
            id: id.to_string(),
            rect: Rect { x, y, w, h },
            ground: false,
        }
    }

    fn rect_of<'a>(settled: &'a [(String, Rect)], id: &str) -> &'a Rect {
        &settled.iter().find(|(one, _)| one == id).unwrap().1
    }

    #[test]
    fn overlapping_boxes_are_pushed_apart_by_the_gap() {
        let settled = settle(
            &[solid("a", 0.0, 0.0, 100.0, 50.0), solid("b", 50.0, 0.0, 100.0, 50.0)],
            20.0,
            &[],
        );
        assert_eq!(*rect_of(&settled, "a"), Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 });
        assert_eq!(*rect_of(&settled, "b"), Rect { x: 120.0, y: 0.0, w: 100.0, h: 50.0 });
    }

    #[test]
    fn boxes_that_already_have_room_stay_put() {
        let settled = settle(
            &[solid("a", 0.0, 0.0, 100.0, 50.0), solid("b", 200.0, 0.0, 100.0, 50.0)],
            20.0,
            &[],
        );
        assert_eq!(*rect_of(&settled, "a"), Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 });
        assert_eq!(*rect_of(&settled, "b"), Rect { x: 200.0, y: 0.0, w: 100.0, h: 50.0 });
    }

    #[test]
    fn an_arrows_words_keep_room_between_the_boxes_it_joins() {
        let join = Join {
            from: "a".to_string(),
            to: "b".to_string(),
            w: 60.0,
            h: 30.0,
        };
        let settled = settle(
            &[solid("a", 0.0, 0.0, 100.0, 50.0), solid("b", 50.0, 0.0, 100.0, 50.0)],
            20.0,
            &[join],
        );
        assert_eq!(rect_of(&settled, "b").x, 200.0);
    }
}
