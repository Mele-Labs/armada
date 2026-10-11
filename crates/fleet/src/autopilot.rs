//! Autopilot: a repository works its own backlog from one plain-language instruction until a
//! person pauses or stops it. #2092, `docs/capabilities/autopilot.md`, `docs/concepts/autopilot.md`.
//!
//! **A Fleet record that starts Jobs, and neither a Session nor a Job.** Fleet does what is
//! deterministic here: the room, the caps, Pause and Stop. The judgement is two model calls made
//! the way the Job proposer's is, each of which ends: one reads the sentence, one picks an issue.
//! Neither is a Drone, so neither holds a place a Job needs.
//!
//! | Part | Where |
//! |---|---|
//! | The sentence read as settings | [`sentence`] |
//! | The picker's question and its answer | [`picking`] |
//! | Room, caps, and the three states | [`bounds`] |
//! | The pass on the noticing interval, and the Job a sure pick starts | `pass` |
//! | The acts a person takes, and the wire | `acts`, `wire` |
//!
//! **A sure pick skips the dispatch gate, and nothing else does.** [`picking::SurePick`] has no
//! public constructor: only reading the picker's answer makes one, and only a sure pick reaches
//! the queue unasked. The owner's call, `.claude/decisions/2026-10-11-autopilots-clear-picks-start-unasked.md`.

mod acts;
pub mod bounds;
mod pass;
pub mod picking;
pub mod sentence;
mod wire;

use std::sync::Arc;

use adapter_traits::{Environment, Model, ModelClient};

use crate::judging::JudgeBudget;

pub use bounds::{room, Cap, Caps, Settings, Spent, Standing, State, Unsettable};
pub use pass::keep_autopiloting;
pub use picking::{candidates, picked, read_pick, Candidate, NotPicked, Pick, SetAside, SurePick};
pub use sentence::{read_settings, settings_from, NotRead, ProposedSettings};

/// Everything one call needs in order to ask: the Job proposer's client, budget, model and
/// environment. **Its prompt is not here**: each call writes its own question.
#[derive(Clone)]
pub struct Asking {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    pub model: Model,
    pub environment: Environment,
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
}

impl Remembered {
    pub(crate) fn declined(&self, manifest: &str, offered: &[String]) -> bool {
        held(&self.declined)
            .get(manifest)
            .is_some_and(|last| last == offered)
    }

    pub(crate) fn decline(&self, manifest: &str, offered: Vec<String>) {
        held(&self.declined).insert(manifest.to_string(), offered);
    }

    /// Clear what the picker declined, for a start whose instruction may cover it.
    pub(crate) fn forget(&self, manifest: &str) {
        held(&self.declined).remove(manifest);
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
