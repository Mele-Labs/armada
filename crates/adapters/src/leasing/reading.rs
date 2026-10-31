//! The pool as Fleet serves it to Bridge: `--status`'s reading, plus whether
//! each slot is warm and how far behind the base it is.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{SlotHeld, SlotReading};

use crate::concurrently::concurrently;

use super::record::Record;
use super::{git, Holder, Pool, Slot, SlotState};

/// One slot's reading, and the `git status` it was made from.
pub(super) struct Read {
    pub(super) reading: SlotReading,
    /// `None` where the slot is held, or not a checkout: nothing asked.
    pub(super) status: Option<Result<Vec<String>, String>>,
    /// Git was asked for `status`, rather than it being trusted.
    pub(super) asked_git: bool,
}

impl Pool {
    /// Every slot, read the way [`Pool::status`] reads it.
    ///
    /// **Slot by slot, side by side**: each is its own checkout and its own
    /// handful of git processes, and the base is asked for once rather than by
    /// each.
    pub fn readings(&self) -> Vec<SlotReading> {
        self.readings_of(&self.bays())
    }

    /// [`readings`](Pool::readings) of the slots in `numbers` alone.
    pub(super) fn readings_of(&self, numbers: &[usize]) -> Vec<SlotReading> {
        self.readings_trusting(numbers, &BTreeMap::new())
            .into_iter()
            .map(|read| read.reading)
            .collect()
    }

    /// [`readings_of`](Pool::readings_of), asking git for `status` of a slot
    /// nobody holds only where `trusted` has no answer for it, and handing
    /// back the answer each slot's reading was made from.
    pub(super) fn readings_trusting(
        &self,
        numbers: &[usize],
        trusted: &BTreeMap<usize, Vec<String>>,
    ) -> Vec<Read> {
        let shape = self.shape();
        let base = self.base_ref();
        concurrently(numbers, |&number| {
            // **A take or a release holds the slot's lock while it works**, and a slot read
            // in the middle of one has files and no record: stranded, its holder gone.
            // `take` reads under its own lock and asks `probe_trusting` itself.
            let (state, status) = match self.under_way(number) {
                true => (SlotState::Busy, None),
                false => self.probe_trusting(number, trusted.get(&number)),
            };
            let kept = status.clone();
            let reading = self.reading(
                Slot {
                    number,
                    path: self.path_of(number),
                    state,
                    closed: shape.closed.contains(&number),
                },
                &base,
                status,
            );
            Read {
                reading,
                status: kept,
                asked_git: !trusted.contains_key(&number),
            }
        })
    }

    fn reading(
        &self,
        slot: Slot,
        base: &str,
        status: Option<Result<Vec<String>, String>>,
    ) -> SlotReading {
        let made = !matches!(slot.state, SlotState::Unmade | SlotState::NotACheckout);
        let apart = made.then(|| self.apart(&slot.path, base)).flatten();
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
                    Holder::Hosted(id) => SlotHeld::Hosted(id.clone()),
                    Holder::Process { .. } => SlotHeld::Session(holder.said()),
                };
                (held, Some(branch), since)
            }
            SlotState::Stranded { branch, since, why } => {
                (SlotHeld::Stranded(why), Some(branch), since)
            }
        };
        let branch = branch.filter(|name| !name.is_empty());
        let last_session = match &held {
            SlotHeld::Stranded(_) => {
                Record::read(&self.record_path(slot.number)).and_then(|record| record.session)
            }
            _ => None,
        };
        SlotReading {
            closed: slot.closed,
            slot: slot.number as u32,
            warm: made && self.warm(&slot.path),
            behind: apart.map(|(_, behind)| behind),
            ahead: apart.map(|(ahead, _)| ahead),
            path: slot.path.to_string_lossy().into_owned(),
            held,
            branch,
            since: (since > 0).then_some(since),
            last_session,
            work: None,
        }
        // What the slot holds, from the state and the `git status` already read.
        .with_work(|_| self.work_at(&slot.path, status).ok())
    }

    fn warm(&self, at: &Path) -> bool {
        !self.seeds.is_empty() && self.seeds.iter().all(|path| at.join(path).is_dir())
    }

    /// Commits HEAD has that the base does not, and commits the base has that HEAD does not.
    fn apart(&self, at: &Path, base: &str) -> Option<(u32, u32)> {
        if let Some(answer) = git::apart_in_process(at, base) {
            return answer;
        }
        let range = format!("HEAD...{base}");
        let both = git::read(at, &["rev-list", "--left-right", "--count", &range]).ok()?;
        let (ahead, behind) = both.split_once('\t')?;
        Some((ahead.parse().ok()?, behind.parse().ok()?))
    }
}
