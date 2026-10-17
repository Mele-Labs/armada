//! Whether a pull request is offered for review or parked as a draft.

/// Whether the pull request is offered for review or parked as a draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PrMode {
    #[default]
    Ready,
    Draft,
}

impl PrMode {
    pub const ALL: &'static [PrMode] = &[PrMode::Ready, PrMode::Draft];

    pub fn as_wire(&self) -> &'static str {
        match self {
            PrMode::Ready => "ready",
            PrMode::Draft => "draft",
        }
    }

    pub fn from_wire(value: &str) -> Option<PrMode> {
        PrMode::ALL
            .iter()
            .copied()
            .find(|mode| mode.as_wire() == value)
    }
}
