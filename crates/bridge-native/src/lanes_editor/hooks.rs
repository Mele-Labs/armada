//! What the editor asks of the canvas that the canvas does not give yet: where the picked node stands, and the gaps
//! between steps. Stubs with the shapes Jobs screens commits (methods of `LanesState` there, so the swap is one line).
//! Both read the last frame's cache, keyed by the id passed to `lanes_canvas()`.

#![allow(dead_code)]

use gpui::{Bounds, Pixels, SharedString};

use crate::lanes_canvas::LanesState;

/// One gap between two steps of the workflow's lane: the node ids either side, and where it stands in window points.
#[derive(Clone, Debug, PartialEq)]
pub struct StepGap {
    pub after: String,
    pub before: String,
    pub bounds: Bounds<Pixels>,
}

/// The picked node or rule chip in window points, as last drawn with pan and zoom applied. None when nothing is picked.
pub fn picked_bounds(_state: &LanesState, _id: &SharedString) -> Option<Bounds<Pixels>> {
    None
}

/// One gap for each pair of steps in the workflow's lane, ordered by column, Start and End left out.
pub fn step_gaps(_state: &LanesState, _id: &SharedString) -> Vec<StepGap> {
    Vec::new()
}
