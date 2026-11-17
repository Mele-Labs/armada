//! The lists last read, and the wait the forge's rate limit puts on reading them again.
//! In memory: a Fleet that starts has read none, and the Reviews tab asks.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use adapter_traits::ListFailure;
use api::Refusal;
use ipc::{PullReviewLists, PullReviewRateLimited, WireValue};

use crate::clock::rfc3339_utc;

/// Where the forge named no time, Fleet asks again after this long.
const UNNAMED_WAIT_MILLIS: i64 = 180_000;

/// One wait. `until` is what the forge said, in milliseconds since the epoch; absent where it said none.
#[derive(Clone, Copy)]
pub(crate) struct Hold {
    pub(crate) until: Option<i64>,
    pub(crate) asks_again_at: i64,
}

/// The last read of each project's lists (`None` is every project's), and the wait in force.
#[derive(Default)]
pub(crate) struct Kept {
    lists: Mutex<BTreeMap<Option<String>, PullReviewLists>>,
    hold: Mutex<Option<Hold>>,
}

impl Kept {
    /// A read that reached the forge: kept, and any wait lifted.
    pub(crate) fn keep(&self, project: Option<&str>, lists: &PullReviewLists) {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner).insert(project.map(str::to_string), lists.clone());
        *self.hold.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// The forge refused for its rate limit: nothing is asked until `until`, or for a few minutes.
    pub(crate) fn hold(&self, now: i64, until_epoch_secs: Option<u64>) -> Hold {
        let until = until_epoch_secs.map(|secs| secs as i64 * 1_000);
        let hold = Hold { until, asks_again_at: until.unwrap_or(now + UNNAMED_WAIT_MILLIS) };
        *self.hold.lock().unwrap_or_else(PoisonError::into_inner) = Some(hold);
        hold
    }

    /// The wait still in force at `now`.
    pub(crate) fn held(&self, now: i64) -> Option<Hold> {
        (*self.hold.lock().unwrap_or_else(PoisonError::into_inner)).filter(|hold| now < hold.asks_again_at)
    }

    fn last(&self, project: Option<&str>) -> Option<PullReviewLists> {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner).get(&project.map(str::to_string)).cloned()
    }

    /// What a rate-limited read answers: the lists last read, marked, else the refusal.
    pub(crate) fn limited(&self, project: Option<&str>, hold: Hold) -> Result<PullReviewLists, (Option<String>, String)> {
        let until = hold.until.map(rfc3339_utc);
        match self.last(project) {
            Some(mut lists) => {
                lists.rate_limited = Some(PullReviewRateLimited { until: until.map(ipc::Instant::carried) });
                Ok(lists)
            }
            None => Err((until, "GitHub is rate-limiting us".to_string())),
        }
    }
}

/// The sentence for a list that could not be read, said so a person can act on it.
pub(crate) fn failure_said(failure: &ListFailure) -> String {
    match failure {
        ListFailure::RateLimited { .. } => "GitHub is rate-limiting us".to_string(),
        ListFailure::NoToken => "GitHub has no token for this machine. Run `gh auth login`, then open Reviews again.".to_string(),
        ListFailure::Unreachable => "GitHub did not answer. Check the network, then open Reviews again.".to_string(),
        ListFailure::Other(said) => said.clone(),
    }
}

/// The worst of the lists' failures: the rate limit, which names a time, then no token, then silence.
pub(crate) fn worst(failures: Vec<ListFailure>) -> Option<ListFailure> {
    let rank = |failure: &ListFailure| match failure {
        ListFailure::RateLimited { .. } => 0,
        ListFailure::NoToken => 1,
        ListFailure::Unreachable => 2,
        ListFailure::Other(_) => 3,
    };
    // Where two are rate limits, the later time is the one that clears them both.
    let latest = failures.iter().filter_map(|it| matches!(it, ListFailure::RateLimited { until: Some(_) }).then_some(it)).cloned().max_by_key(|it| match it {
        ListFailure::RateLimited { until } => *until,
        _ => None,
    });
    latest.or_else(|| failures.into_iter().min_by_key(rank))
}

/// The refusal for a rate limit with nothing kept to show: its own code, the time to try again in `until`.
pub(crate) fn rate_limited_refusal(error: impl FnOnce(&'static str, String) -> ipc::WireError, until: Option<String>, said: String) -> Refusal {
    let mut wire = error(ipc::REVIEW_LISTS_RATE_LIMITED, said);
    if let Some(until) = until {
        wire.fields.insert("until".to_string(), WireValue::Str(until));
    }
    Refusal::Fault(wire)
}
