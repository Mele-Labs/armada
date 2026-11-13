//! Plain scene geometry the router, the settler and the arrow words share, in scene units.

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct Pt {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
