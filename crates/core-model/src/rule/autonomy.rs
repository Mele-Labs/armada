//! How far a rule goes on its own: Acts, Asks or Off, set for a kind of act by a Workspace or a
//! repository and overridden by one rule.
//!
//! **What a plan can say is narrower than what a person can set.** An [`Autonomy`] has Off, and a
//! [`Course`] does not, so a planned act that is not to be done is not a value the engine can hand
//! back and a doer can forget to check.

use alloc::collections::BTreeMap;

use super::act::ActKind;

/// What the owner sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Autonomy {
    /// Do it, and draw it on the lane.
    Acts,
    /// Put a one-press item in the Job's Needs you list.
    Asks,
    /// Keep the rule and run nothing.
    Off,
}

impl Autonomy {
    pub const ALL: &'static [Autonomy] = &[Autonomy::Acts, Autonomy::Asks, Autonomy::Off];

    pub fn as_wire(self) -> &'static str {
        match self {
            Autonomy::Acts => "acts",
            Autonomy::Asks => "asks",
            Autonomy::Off => "off",
        }
    }

    pub fn from_wire(value: &str) -> Option<Autonomy> {
        Autonomy::ALL.iter().copied().find(|autonomy| autonomy.as_wire() == value)
    }

    /// What a planned act does, where it is planned at all.
    pub fn course(self) -> Option<Course> {
        match self {
            Autonomy::Acts => Some(Course::Acts),
            Autonomy::Asks => Some(Course::Asks),
            Autonomy::Off => None,
        }
    }
}

/// What becomes of a planned act: it is done, or it is asked about first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Course {
    Acts,
    Asks,
}

/// The defaults one Workspace or one repository sets, each for a kind of act. **Only what was set
/// is held**, so a kind nobody spoke of falls through to the next layer and in the end to the
/// kind's own default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutonomyDefaults {
    set: BTreeMap<ActKind, Autonomy>,
}

impl AutonomyDefaults {
    /// Nothing set.
    pub fn new() -> AutonomyDefaults {
        AutonomyDefaults::default()
    }

    /// With `kind` set to `autonomy`.
    pub fn setting(mut self, kind: ActKind, autonomy: Autonomy) -> AutonomyDefaults {
        self.set.insert(kind, autonomy);
        self
    }

    pub fn get(&self, kind: ActKind) -> Option<Autonomy> {
        self.set.get(&kind).copied()
    }

    /// What was set, in the order of the kinds. For Fleet to keep.
    pub fn iter(&self) -> impl Iterator<Item = (ActKind, Autonomy)> + '_ {
        self.set.iter().map(|(kind, autonomy)| (*kind, *autonomy))
    }

    /// This layer laid over a wider one: the repository's over its Workspace's. A kind both set
    /// takes this layer's.
    pub fn laid_over(&self, wider: &AutonomyDefaults) -> AutonomyDefaults {
        let mut set = wider.set.clone();
        set.extend(self.set.iter().map(|(kind, autonomy)| (*kind, *autonomy)));
        AutonomyDefaults { set }
    }

    /// What a rule does for an act of `kind`: the rule's own word, else these defaults, else the
    /// kind's own.
    pub fn resolved(&self, kind: ActKind, rule: Option<Autonomy>) -> Autonomy {
        rule.or_else(|| self.get(kind)).unwrap_or_else(|| kind.default_autonomy())
    }
}
