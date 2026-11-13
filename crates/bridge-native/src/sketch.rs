//! The composer's "Draw sketch": what is on the pad (`packages/screens/src/draft/sketch.ts`) and the drawing a message carries (`DrawnSketch`, `sessions.tsx`). Plain data; the pad draws it and `sketch_png` turns it into the upload.

/// One box: where a person put it, and the words in it. It has no size; a box is one width and grows down the page with its words.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Shape {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub body: String,
}

/// One box joined to another, in the direction a person drew it.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Join {
    pub id: String,
    pub from: String,
    pub to: String,
}

/// One place on the pad, in the coordinates a box's `x` and `y` are in.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

/// One line drawn freehand, kept as its points so the pad can redraw, undo and scale it.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Stroke {
    pub id: String,
    pub points: Vec<Point>,
}

/// What is on the pad. Electron's version also carries pasted pictures; its `DrawnSketch` drops them, so this one never has any.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Drawing {
    pub shapes: Vec<Shape>,
    pub joins: Vec<Join>,
    pub strokes: Vec<Stroke>,
}

/// A drawing attached to a message, with the title its chip shows.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct DrawnSketch {
    pub id: String,
    pub title: String,
    pub drawing: Drawing,
}

/// The pad as it goes onto a message: titled by its first words, as `SketchSheet.attach` does. The id is the caller's, since Electron mints it from the clock.
pub(crate) fn attach(drawing: &Drawing, id: String) -> DrawnSketch {
    let words = drawing.shapes.iter().map(|one| one.body.trim()).find(|one| !one.is_empty());
    let title = match words {
        Some(words) => words.chars().take(32).collect(),
        None => "Sketch".to_string(),
    };
    DrawnSketch { id, title, drawing: drawing.clone() }
}

/// The lowest `<prefix><n>` that none of `held` carries, so adding and removing all afternoon does not end on `b214`.
fn first_free<'a>(prefix: &str, held: impl Iterator<Item = &'a str>) -> String {
    let held: Vec<&str> = held.collect();
    let mut at = 1usize;
    while held.contains(&format!("{prefix}{at}").as_str()) {
        at += 1;
    }
    format!("{prefix}{at}")
}

impl Drawing {
    /// Whether anything is on the pad, which decides the chip. A stroke counts on its own; a join does not, because it needs the two boxes it hangs on.
    pub(crate) fn is_drawn(&self) -> bool {
        !self.shapes.is_empty() || !self.strokes.is_empty()
    }

    /// The id the next box takes: the lowest `b<n>` nothing on the pad holds.
    pub(crate) fn next_shape_id(&self) -> String {
        first_free("b", self.shapes.iter().map(|one| one.id.as_str()))
    }

    /// The same rule for strokes, with `s` so the two never collide.
    fn next_stroke_id(&self) -> String {
        first_free("s", self.strokes.iter().map(|one| one.id.as_str()))
    }

    /// A box put down. An id already on the pad replaces the box holding it.
    pub(crate) fn with_shape(&mut self, shape: Shape) {
        match self.shapes.iter_mut().find(|one| one.id == shape.id) {
            Some(held) => *held = shape,
            None => self.shapes.push(shape),
        }
    }

    /// Boxes taken off, and every join that hung on one of them: a join to a box that is gone would draw as a line into nothing.
    pub(crate) fn without_shapes(&mut self, ids: &[String]) {
        self.shapes.retain(|one| !ids.contains(&one.id));
        self.joins.retain(|one| !ids.contains(&one.from) && !ids.contains(&one.to));
    }

    /// A line drawn, on top of what is there. It mints the id, because nothing reads a stroke's id but Undo's order. Fewer than two points is a click and not a line, so it is dropped.
    pub(crate) fn with_stroke(&mut self, points: Vec<Point>) {
        if points.len() < 2 {
            return;
        }
        let id = self.next_stroke_id();
        self.strokes.push(Stroke { id, points });
    }

    /// The last line drawn, taken back. Strokes only: a box comes off under Remove.
    pub(crate) fn without_last_stroke(&mut self) {
        self.strokes.pop();
    }

    /// Two boxes joined, once. A pair already joined stays as it was, whichever way round, and a box does not join itself. Like the TS, it does not check that either box exists.
    pub(crate) fn with_join(&mut self, from: &str, to: &str) {
        if from == to {
            return;
        }
        let already = self.joins.iter().any(|one| (one.from == from && one.to == to) || (one.from == to && one.to == from));
        if already {
            return;
        }
        self.joins.push(Join { id: format!("{from}-{to}"), from: from.to_string(), to: to.to_string() });
    }

    /// The words in one box, rewritten. A box the pad does not hold changes nothing.
    pub(crate) fn with_body(&mut self, id: &str, body: &str) {
        if let Some(one) = self.shapes.iter_mut().find(|one| one.id == id) {
            one.body = body.to_string();
        }
    }

    /// A box put down somewhere new. A box the pad does not hold changes nothing.
    pub(crate) fn with_place(&mut self, id: &str, x: f32, y: f32) {
        if let Some(one) = self.shapes.iter_mut().find(|one| one.id == id) {
            one.x = x;
            one.y = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A line of three points, the smallest one worth curving.
    fn line() -> Vec<Point> {
        vec![Point { x: 0.0, y: 0.0 }, Point { x: 40.0, y: 20.0 }, Point { x: 80.0, y: 0.0 }]
    }

    fn ids<'a>(shapes: &'a [Shape]) -> Vec<&'a str> {
        shapes.iter().map(|one| one.id.as_str()).collect()
    }

    #[test]
    fn a_box_counts_as_drawn_and_a_join_alone_does_not() {
        assert!(!Drawing::default().is_drawn());
        assert!(pad().is_drawn());
        let only_a_join = Drawing { joins: pad().joins, ..Drawing::default() };
        assert!(!only_a_join.is_drawn());
    }

    #[test]
    fn a_stroke_counts_as_drawn_on_its_own() {
        let mut drawing = Drawing::default();
        drawing.with_stroke(line());
        assert!(drawing.is_drawn());
    }

    #[test]
    fn the_next_box_takes_the_first_id_free() {
        assert_eq!(Drawing::default().next_shape_id(), "b1");
        assert_eq!(pad().next_shape_id(), "b3");
        let mut drawing = pad();
        drawing.without_shapes(&["b1".to_string()]);
        assert_eq!(drawing.next_shape_id(), "b1");
    }

    #[test]
    fn a_box_goes_down_at_the_end() {
        let mut drawing = pad();
        drawing.with_shape(shape("b3", 0.0, 200.0, ""));
        assert_eq!(ids(&drawing.shapes), ["b1", "b2", "b3"]);
    }

    #[test]
    fn a_box_whose_id_is_down_is_replaced_not_drawn_twice() {
        let mut drawing = pad();
        drawing.with_shape(shape("b2", 9.0, 9.0, "moved"));
        assert_eq!(drawing.shapes.len(), 2);
        assert_eq!(drawing.shapes[1], shape("b2", 9.0, 9.0, "moved"));
    }

    #[test]
    fn a_box_takes_every_join_off_with_it() {
        let mut drawing = pad();
        drawing.without_shapes(&["b2".to_string()]);
        assert_eq!(ids(&drawing.shapes), ["b1"]);
        assert!(drawing.joins.is_empty());
    }

    #[test]
    fn words_are_rewritten_in_one_box_only() {
        let mut drawing = pad();
        drawing.with_body("b1", "the Drones stat");
        assert_eq!(drawing.shapes[0].body, "the Drones stat");
        assert_eq!(drawing.shapes[1].body, "the panel it opens");
    }

    #[test]
    fn a_box_the_pad_does_not_hold_changes_nothing() {
        let mut drawing = pad();
        drawing.with_body("b9", "nowhere");
        drawing.with_place("b9", 1.0, 1.0);
        assert_eq!(drawing, pad());
    }

    #[test]
    fn a_box_is_put_down_somewhere_new() {
        let mut drawing = pad();
        drawing.with_place("b1", 40.0, 80.0);
        assert_eq!(drawing.shapes[0], shape("b1", 40.0, 80.0, "the stat"));
    }

    #[test]
    fn lines_stack_in_the_order_they_were_drawn() {
        let mut drawing = Drawing::default();
        drawing.with_stroke(line());
        drawing.with_stroke(line());
        assert_eq!(drawing.strokes.iter().map(|one| one.points.len()).collect::<Vec<_>>(), [3, 3]);
    }

    #[test]
    fn a_line_of_fewer_than_two_points_is_not_kept() {
        let mut drawing = Drawing::default();
        drawing.with_stroke(vec![]);
        drawing.with_stroke(vec![Point { x: 4.0, y: 4.0 }]);
        assert_eq!(drawing, Drawing::default());
    }

    #[test]
    fn the_last_line_is_taken_back_and_no_other() {
        let mut drawing = Drawing::default();
        drawing.with_stroke(line());
        drawing.with_stroke(line());
        drawing.without_last_stroke();
        assert_eq!(drawing.strokes.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["s1"]);
    }

    #[test]
    fn taking_back_a_line_where_none_was_drawn_changes_nothing() {
        let mut drawing = pad();
        drawing.without_last_stroke();
        assert_eq!(drawing, pad());
    }

    #[test]
    fn lines_and_boxes_leave_each_other_alone() {
        let mut drawing = pad();
        drawing.with_stroke(line());
        assert_eq!(drawing.shapes, pad().shapes);
        drawing.without_shapes(&["b1".to_string(), "b2".to_string()]);
        assert_eq!(drawing.strokes.len(), 1);
    }

    #[test]
    fn a_stroke_id_is_the_first_free_so_undo_does_not_count_up() {
        let mut drawing = Drawing::default();
        drawing.with_stroke(line());
        drawing.with_stroke(line());
        assert_eq!(drawing.strokes.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["s1", "s2"]);
        drawing.without_last_stroke();
        drawing.with_stroke(line());
        assert_eq!(drawing.strokes[1].id, "s2");
    }

    #[test]
    fn a_join_runs_in_the_direction_it_was_made() {
        let mut drawing = Drawing { joins: vec![], ..pad() };
        drawing.with_join("b2", "b1");
        assert_eq!(drawing.joins, [Join { id: "b2-b1".into(), from: "b2".into(), to: "b1".into() }]);
    }

    #[test]
    fn a_pair_already_joined_stays_as_it_was_whichever_way_round() {
        let mut drawing = pad();
        drawing.with_join("b1", "b2");
        drawing.with_join("b2", "b1");
        assert_eq!(drawing, pad());
    }

    #[test]
    fn a_box_does_not_join_itself() {
        let mut drawing = pad();
        drawing.with_join("b1", "b1");
        assert_eq!(drawing, pad());
    }

    #[test]
    fn attaching_titles_the_sketch_by_its_first_words() {
        let drawing = Drawing { shapes: vec![shape("b1", 0.0, 0.0, "  "), shape("b2", 0.0, 0.0, "  the panel it opens  ")], ..Drawing::default() };
        let sketch = attach(&drawing, "d1".into());
        assert_eq!(sketch.id, "d1");
        assert_eq!(sketch.title, "the panel it opens");
        assert_eq!(sketch.drawing, drawing);
    }

    #[test]
    fn attaching_cuts_a_long_title_and_names_a_wordless_one_sketch() {
        let long = Drawing { shapes: vec![shape("b1", 0.0, 0.0, &"é".repeat(40))], ..Drawing::default() };
        assert_eq!(attach(&long, "d1".into()).title.chars().count(), 32);
        assert_eq!(attach(&Drawing::default(), "d2".into()).title, "Sketch");
        let blank = Drawing { shapes: vec![shape("b1", 0.0, 0.0, "")], ..Drawing::default() };
        assert_eq!(attach(&blank, "d3".into()).title, "Sketch");
    }
}
