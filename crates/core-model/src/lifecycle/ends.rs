//! What a step's `ends_on` says of a lifecycle: `pr: approved`, or `any ticket: closed`.
//!
//! **An id pair and a count, nothing more.** Whether the lifecycle and the state exist is a fact
//! about the Job's lifecycles and is checked where they are known; here they are only written
//! the way `ids` says an id is written. A state is the world's to reach, so an end on one is not
//! a thing the person does and raises no wait for him.

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

use super::ids::{LifecycleId, NotAnId, StateId};

/// How many of a lifecycle's instances a step waits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Waits {
    /// Every one the Job holds: two tickets both have to close. What `pr: approved` means.
    Every,
    /// The first to get there. Written `any pr: approved`.
    Any,
}

/// A step ends when instances of a lifecycle reach a state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LifecycleEnd {
    lifecycle: LifecycleId,
    state: StateId,
    waits: Waits,
}

impl LifecycleEnd {
    pub fn new(lifecycle: LifecycleId, state: StateId, waits: Waits) -> LifecycleEnd {
        LifecycleEnd { lifecycle, state, waits }
    }

    /// `pr: approved` or `any ticket: closed`, as written in a step's `ends_on`. Anything that is
    /// not that shape is refused, so a word of today's set is not mistaken for one.
    pub fn from_wire(written: &str) -> Result<LifecycleEnd, NotALifecycleEnd> {
        let Some((left, right)) = written.split_once(':') else {
            return Err(NotALifecycleEnd::NoState);
        };
        let (waits, lifecycle) = match left.trim().strip_prefix("any ") {
            Some(rest) => (Waits::Any, rest.trim()),
            None => (Waits::Every, left.trim()),
        };
        let lifecycle = LifecycleId::new(lifecycle).map_err(NotALifecycleEnd::Lifecycle)?;
        let state = StateId::new(right.trim()).map_err(NotALifecycleEnd::State)?;
        Ok(LifecycleEnd { lifecycle, state, waits })
    }

    pub fn lifecycle(&self) -> &LifecycleId {
        &self.lifecycle
    }

    pub fn state(&self) -> &StateId {
        &self.state
    }

    pub fn waits(&self) -> Waits {
        self.waits
    }

    /// As [`from_wire`](Self::from_wire) reads it.
    pub fn as_wire(&self) -> String {
        match self.waits {
            Waits::Every => format!("{}: {}", self.lifecycle, self.state),
            Waits::Any => format!("any {}: {}", self.lifecycle, self.state),
        }
    }
}

impl fmt::Display for LifecycleEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_wire())
    }
}

/// Written as a lifecycle end and not one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotALifecycleEnd {
    /// No colon: `approved` alone names no lifecycle.
    NoState,
    Lifecycle(NotAnId),
    State(NotAnId),
}

impl fmt::Display for NotALifecycleEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotALifecycleEnd::NoState => f.write_str("a lifecycle end is a lifecycle and its state, such as `pr: approved`"),
            NotALifecycleEnd::Lifecycle(why) | NotALifecycleEnd::State(why) => f.write_str(&why.to_string()),
        }
    }
}

impl core::error::Error for NotALifecycleEnd {}
