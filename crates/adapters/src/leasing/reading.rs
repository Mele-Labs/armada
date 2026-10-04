//! The pool as Fleet serves it to Bridge: `--status`'s reading, plus whether
//! each slot is warm and how far behind the base it is.

use std::path::Path;

use adapter_traits::{SlotHeld, SlotReading};

use super::{git, Holder, Pool, Slot, SlotState};

impl Pool {
    /// Every slot, read the way [`Pool::status`] reads it.
    pub fn readings(&self) -> Vec<SlotReading> {
        self.status()
            .into_iter()
            .map(|slot| self.reading(slot))
            .collect()
    }

    fn reading(&self, slot: Slot) -> SlotReading {
        let made = !matches!(slot.state, SlotState::Unmade | SlotState::NotACheckout);
        let (held, branch, since) = match slot.state {
            SlotState::Unmade => (SlotHeld::Unmade, None, 0),
            SlotState::NotACheckout => (SlotHeld::NotACheckout, None, 0),
            SlotState::Busy => (SlotHeld::Busy, None, 0),
            // A free slot sits detached, so the branch it last held is not the
            // one checked out.
            SlotState::Free | SlotState::Abandoned { .. } => (SlotHeld::Free, None, 0),
            SlotState::Held {
                branch,
                holder,
                since,
                ..
            } => {
                let held = match &holder {
                    Holder::Job(id) => SlotHeld::Job(id.clone()),
                    Holder::Process { .. } => SlotHeld::Session(holder.said()),
                };
                (held, Some(branch), since)
            }
            SlotState::Stranded { branch, since, why } => {
                (SlotHeld::Stranded(why), Some(branch), since)
            }
        };
        let branch = branch.filter(|name| !name.is_empty());
        SlotReading {
            closed: slot.closed,
            slot: slot.number as u32,
            warm: made && self.warm(&slot.path),
            behind: made.then(|| self.behind(&slot.path)).flatten(),
            path: slot.path.to_string_lossy().into_owned(),
            held,
            branch,
            since: (since > 0).then_some(since),
        }
    }

    fn warm(&self, at: &Path) -> bool {
        !self.seeds.is_empty() && self.seeds.iter().all(|path| at.join(path).is_dir())
    }

    fn behind(&self, at: &Path) -> Option<u32> {
        let range = format!("HEAD..{}", self.base_ref());
        git::git(at, &["rev-list", "--count", &range])
            .ok()?
            .parse()
            .ok()
    }
}
