//! The pool as Fleet serves it to Bridge: `--status`'s reading, plus whether
//! each slot is warm and how far behind the base it is.

use std::path::Path;

use adapter_traits::{SlotHeld, SlotReading};

use crate::concurrently::concurrently;

use super::{git, Holder, Pool, Slot, SlotState};

impl Pool {
    /// Every slot, read the way [`Pool::status`] reads it.
    ///
    /// **Slot by slot, side by side**: each is its own checkout and its own
    /// handful of git processes, and the base is asked for once rather than by
    /// each.
    pub fn readings(&self) -> Vec<SlotReading> {
        let shape = self.shape();
        let base = self.base_ref();
        concurrently(&self.bays(), |&number| {
            let (state, status) = self.probe_of(number);
            self.reading(
                Slot {
                    number,
                    path: self.path_of(number),
                    state,
                    closed: shape.closed.contains(&number),
                },
                &base,
                status,
            )
        })
    }

    fn reading(
        &self,
        slot: Slot,
        base: &str,
        status: Option<Result<Vec<String>, String>>,
    ) -> SlotReading {
        let made = !matches!(slot.state, SlotState::Unmade | SlotState::NotACheckout);
        let (kept, completed) = match &slot.state {
            SlotState::Held {
                kept, completed, ..
            } => (kept.clone(), *completed),
            _ => (None, false),
        };
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
            behind: made.then(|| self.behind(&slot.path, base)).flatten(),
            path: slot.path.to_string_lossy().into_owned(),
            held,
            branch,
            since: (since > 0).then_some(since),
            kept,
            completed,
            work: None,
        }
        // What the slot holds, from the state and the `git status` already read.
        .with_work(|_| self.work_at(&slot.path, status).ok())
    }

    fn warm(&self, at: &Path) -> bool {
        !self.seeds.is_empty() && self.seeds.iter().all(|path| at.join(path).is_dir())
    }

    fn behind(&self, at: &Path, base: &str) -> Option<u32> {
        let range = format!("HEAD..{base}");
        git::read(at, &["rev-list", "--count", &range])
            .ok()?
            .parse()
            .ok()
    }
}
