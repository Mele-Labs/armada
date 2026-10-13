//! The Session model's step down as the weekly plan window runs out: the states, and what a reading
//! of the window does to them. `docs/concepts/machine.md`, *Quota*. [`crate::model_downshifting`] is
//! Fleet's side, which reads settings, writes `models.session` and publishes.
//!
//! **Nothing here reads a clock or a file**, so the whole machine is tested by feeding it readings.

use ipc::{ModelDownshift, ModelDownshiftState};
use store::KeptDownshift;

/// Where the step down stands, with what each phase must remember.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Offered { percent: u32, was: String, to: String },
    /// `until` is the reset of the window it was accepted in; `restore_to` is what settings.json held
    /// for `models.session`, `None` being not saved, so restoring removes the key.
    Shifted { was: String, to: String, until: String, restore_to: Option<String> },
    Reset { was: String, to: String, restore_to: Option<String> },
}

/// The state, and the window an offer was last made for. **The window outlives the state**: a settled
/// or restored offer is not made again before the window resets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Downshift {
    pub(crate) phase: Option<Phase>,
    pub(crate) offered_for: Option<String>,
}

/// The weekly window as a session last read it.
pub(crate) struct Weekly<'a> {
    pub(crate) percent: u32,
    pub(crate) resets_at: &'a str,
}

/// What the owner's settings say a reading is judged by.
pub(crate) struct Terms<'a> {
    pub(crate) line: u32,
    pub(crate) to: &'a str,
    /// `models.session` as it stands.
    pub(crate) current: &'a str,
    /// An environment variable names the Session model, so writing the setting would change nothing.
    pub(crate) pinned: bool,
}

impl Downshift {
    /// A reading of the weekly window. **True where the state or the window remembered moved.**
    pub(crate) fn read(&mut self, weekly: &Weekly<'_>, terms: &Terms<'_>) -> bool {
        let mut moved = false;
        match self.phase.take() {
            Some(Phase::Shifted { was, to, until, restore_to }) if until != weekly.resets_at => {
                self.phase = Some(Phase::Reset { was, to, restore_to });
                return true;
            }
            held @ Some(Phase::Shifted { .. } | Phase::Reset { .. }) => {
                self.phase = held;
                return false;
            }
            Some(Phase::Offered { percent, was, to }) => {
                if self.offered_for.as_deref() == Some(weekly.resets_at) {
                    let percent_now = weekly.percent;
                    self.phase = Some(Phase::Offered { percent: percent_now, was, to });
                    return percent != percent_now;
                }
                // A new window while only offered: the offer is withdrawn and the reading judged afresh.
                moved = true;
            }
            None => {}
        }
        let crossed = weekly.percent >= terms.line;
        let made = self.offered_for.as_deref() == Some(weekly.resets_at);
        if !crossed || made || terms.pinned || terms.current == terms.to {
            return moved;
        }
        self.phase = Some(Phase::Offered {
            percent: weekly.percent,
            was: terms.current.to_string(),
            to: terms.to.to_string(),
        });
        self.offered_for = Some(weekly.resets_at.to_string());
        true
    }

    /// `models.session` was changed by somebody. **True where that ends the state**: while offered,
    /// any value but the one it was offered over; while shifted or reset, any value but `to`.
    pub(crate) fn edited_to(&mut self, current: &str) -> bool {
        let unchanged = match &self.phase {
            None => return false,
            Some(Phase::Offered { was, .. }) => was == current,
            Some(Phase::Shifted { to, .. } | Phase::Reset { to, .. }) => to == current,
        };
        if !unchanged {
            self.phase = None;
        }
        !unchanged
    }

    pub(crate) fn wire(&self) -> ModelDownshiftState {
        ModelDownshiftState {
            downshift: self.phase.as_ref().map(|phase| match phase {
                Phase::Offered { percent, was, to } => {
                    ModelDownshift::Offered { percent: *percent, was: was.clone(), to: to.clone() }
                }
                Phase::Shifted { was, to, until, .. } => ModelDownshift::Shifted {
                    was: was.clone(),
                    to: to.clone(),
                    until: ipc::Instant::carried(until.clone()),
                },
                Phase::Reset { was, to, .. } => ModelDownshift::Reset { was: was.clone(), to: to.clone() },
            }),
        }
    }

    pub(crate) fn kept(&self) -> KeptDownshift {
        let mut kept = KeptDownshift { offered_for: self.offered_for.clone(), ..KeptDownshift::default() };
        match &self.phase {
            None => {}
            Some(Phase::Offered { percent, was, to }) => {
                kept.phase = Some("offered".into());
                kept.percent = *percent;
                kept.was = was.clone();
                kept.to = to.clone();
            }
            Some(Phase::Shifted { was, to, until, restore_to }) => {
                kept.phase = Some("shifted".into());
                kept.was = was.clone();
                kept.to = to.clone();
                kept.until = Some(until.clone());
                kept.restore_to = restore_to.clone();
            }
            Some(Phase::Reset { was, to, restore_to }) => {
                kept.phase = Some("reset".into());
                kept.was = was.clone();
                kept.to = to.clone();
                kept.restore_to = restore_to.clone();
            }
        }
        kept
    }

    /// The state as the store kept it. A phase this build does not know, or a shifted row with no
    /// window, reads as none: the default stays what settings.json says, and nothing is offered over it.
    pub(crate) fn from_kept(kept: KeptDownshift) -> Downshift {
        let phase = match (kept.phase.as_deref(), kept.until) {
            (Some("offered"), _) => Some(Phase::Offered { percent: kept.percent, was: kept.was, to: kept.to }),
            (Some("shifted"), Some(until)) => {
                Some(Phase::Shifted { was: kept.was, to: kept.to, until, restore_to: kept.restore_to })
            }
            (Some("reset"), _) => Some(Phase::Reset { was: kept.was, to: kept.to, restore_to: kept.restore_to }),
            _ => None,
        };
        Downshift { phase, offered_for: kept.offered_for }
    }
}
