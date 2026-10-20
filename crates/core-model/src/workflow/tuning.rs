//! How hard a step's agent thinks.

/// How hard a step's Drone thinks. **A step with none is Armada picking**,
/// which is the harness's own default and has no spelling here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Effort {
    pub const ALL: &'static [Effort] = &[
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::XHigh,
        Effort::Max,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
            Effort::Max => "max",
        }
    }

    pub fn from_wire(value: &str) -> Option<Effort> {
        Effort::ALL
            .iter()
            .copied()
            .find(|effort| effort.as_wire() == value)
    }
}
