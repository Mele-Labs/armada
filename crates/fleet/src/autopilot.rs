//! Autopilot: a repository works its own backlog from one instruction until a person pauses or
//! stops it. #2092, `docs/concepts/autopilot.md`, which has the parts and the rules.
//!
//! **A Fleet record that starts Sessions, and neither a Session nor a Job.** Fleet does what is
//! deterministic: the room, the caps, Pause and Stop. The judgement is two model calls made the
//! way the Session proposer's is, each of which ends, so neither holds a place a Session needs.
//!
//! **Only a sure pick starts unasked.** [`picking::SurePick`] has no public constructor: only
//! reading the picker's answer makes one, and only a sure pick starts a Session with no person
//! pressing. The owner's call of 11 Oct 2026, `2026-10-11-autopilots-clear-picks-start-unasked.md`.

mod acts;
pub mod bounds;
mod editing;
mod pass;
pub mod picking;
pub mod sentence;
mod starting;
mod wire;

use std::sync::Arc;

use adapter_traits::{Environment, Model, ModelClient};

use crate::judging::JudgeBudget;
use crate::prompts::Prompts;

pub use bounds::{room, Cap, Caps, NotResumed, Settings, Spent, Standing, State, Unsettable};
pub use pass::keep_autopiloting;
pub use picking::{candidates, picked, read_pick, Candidate, NotPicked, Pick, SetAside, SurePick};
pub use sentence::{read_settings, settings_from, NotRead, ProposedSettings, Quoted};

/// Everything one call needs in order to ask: the proposer's client, budget, model and
/// environment, and the prompts in force when the call was put together. **Each call fills its
/// own**: `prompts.autopilotSentence` for the reader and `prompts.autopilotPicker` for the picker.
#[derive(Clone)]
pub struct Asking {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    pub model: Model,
    pub environment: Environment,
    pub prompts: Prompts,
}

/// What a pass remembers between intervals, and nothing a restart needs. Held on
/// `crate::tracking::Tracking`, beside the issue lists it is about.
#[derive(Default)]
pub(crate) struct Remembered {
    /// Per repository, the issues the picker last said none of fit. **Not asked again until the
    /// list changes**, or every interval would pay for the same answer.
    declined: std::sync::Mutex<std::collections::BTreeMap<String, Vec<String>>>,
    /// Per repository, the Autopilot last published, so a pass publishes only what moved.
    published: std::sync::Mutex<std::collections::BTreeMap<String, ipc::Autopilot>>,
    /// Per repository, why the last pass picked nothing. **In memory**: the next pass says again.
    faults: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
    /// Per repository, the picker calls failed in a row and the epoch millisecond before which it
    /// is not asked again. A failing call was asked again every pass, a call every few seconds.
    resting: std::sync::Mutex<std::collections::BTreeMap<String, (u32, i64)>>,
}

/// How long the picker rests after its first failure in a row; each further one doubles it.
const FIRST_REST_MS: i64 = 30_000;
/// The longest rest, so a picker that comes back is asked again within this.
const LONGEST_REST_MS: i64 = 15 * 60_000;

impl Remembered {
    pub(crate) fn declined(&self, manifest: &str, offered: &[String]) -> bool {
        held(&self.declined)
            .get(manifest)
            .is_some_and(|last| last == offered)
    }

    pub(crate) fn decline(&self, manifest: &str, offered: Vec<String>) {
        held(&self.declined).insert(manifest.to_string(), offered);
    }

    /// Keep why a pass failed, or clear it where it went through.
    pub(crate) fn faulted(&self, manifest: &str, why: Option<String>) {
        match why {
            Some(why) => held(&self.faults).insert(manifest.to_string(), why),
            None => held(&self.faults).remove(manifest),
        };
    }

    pub(crate) fn fault(&self, manifest: &str) -> Option<String> {
        held(&self.faults).get(manifest).cloned()
    }

    /// Clear what the picker declined, for a start whose instruction may cover it.
    pub(crate) fn forget(&self, manifest: &str) {
        held(&self.declined).remove(manifest);
    }

    /// True while the picker rests after failing.
    pub(crate) fn picker_resting(&self, manifest: &str, now_ms: i64) -> bool {
        held(&self.resting)
            .get(manifest)
            .is_some_and(|(_, until)| now_ms < *until)
    }

    /// A picker call failed: rest, twice as long as the last time where the one before failed too.
    pub(crate) fn picker_failed(&self, manifest: &str, now_ms: i64) {
        let mut resting = held(&self.resting);
        let failed = resting.get(manifest).map_or(1, |(failed, _)| failed + 1);
        let rest = FIRST_REST_MS
            .saturating_mul(1 << (failed - 1).min(10))
            .min(LONGEST_REST_MS);
        resting.insert(manifest.to_string(), (failed, now_ms.saturating_add(rest)));
    }

    pub(crate) fn picker_answered(&self, manifest: &str) {
        held(&self.resting).remove(manifest);
    }

    /// True where `now` differs from what was last published for its repository.
    pub(crate) fn moved(&self, now: &ipc::Autopilot) -> bool {
        let mut published = held(&self.published);
        let key = now.manifest_id.as_str().to_string();
        if published.get(&key) == Some(now) {
            return false;
        }
        published.insert(key, now.clone());
        true
    }
}

fn held<T>(lock: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
