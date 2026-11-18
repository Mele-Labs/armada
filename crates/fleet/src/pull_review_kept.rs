//! The lists last read, the wait the forge's rate limit puts on reading them again, and the read going now.
//! The wait and the read going are in memory. The lists are kept in the store too (`crate::pull_review_refresh`),
//! so a Fleet that starts answers the Reviews tab at once.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::watch;

use adapter_traits::ListFailure;
use api::Refusal;
use ipc::{PullReviewLists, PullReviewRateLimited, WireValue};

use crate::clock::rfc3339_utc;

/// Where the forge named no time, Fleet asks again after this long.
const UNNAMED_WAIT_MILLIS: i64 = 180_000;

/// One wait. `until` is what the forge said, in milliseconds since the epoch; absent where it said none.
#[derive(Clone)]
pub(crate) struct Hold {
    pub(crate) until: Option<i64>,
    /// The forge's sentence for the limit, without a time.
    pub(crate) said: String,
    pub(crate) asks_again_at: i64,
}

type Going = Arc<Mutex<BTreeMap<Option<String>, watch::Receiver<()>>>>;

/// The last read of each project's lists (`None` is every project's), the wait in force, and the reads going.
#[derive(Default)]
pub(crate) struct Kept {
    lists: Mutex<BTreeMap<Option<String>, PullReviewLists>>,
    hold: Mutex<Option<Hold>>,
    going: Going,
}

/// The one read of a project's lists going. Dropped, the project may be read again and whoever joined is woken.
pub(crate) struct Reader {
    project: Option<String>,
    going: Going,
    _done: watch::Sender<()>,
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.going.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.project);
    }
}

/// Whether the asker is the one to read, or a read is going already.
pub(crate) enum Reading {
    Began(Reader),
    /// Ends when that read does.
    Going(watch::Receiver<()>),
}

impl Kept {
    /// A read that reached the forge: kept, and any wait lifted.
    pub(crate) fn keep(&self, project: Option<&str>, lists: &PullReviewLists) {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner).insert(project.map(str::to_string), lists.clone());
        *self.hold.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// The forge refused for its rate limit: nothing is asked until `until`, or for a few minutes.
    pub(crate) fn hold(&self, now: i64, until_epoch_secs: Option<u64>, said: String) -> Hold {
        let until = until_epoch_secs.map(|secs| secs as i64 * 1_000);
        let hold = Hold { until, said, asks_again_at: until.unwrap_or(now + UNNAMED_WAIT_MILLIS) };
        *self.hold.lock().unwrap_or_else(PoisonError::into_inner) = Some(hold.clone());
        hold
    }

    /// The wait still in force at `now`.
    pub(crate) fn held(&self, now: i64) -> Option<Hold> {
        self.hold.lock().unwrap_or_else(PoisonError::into_inner).clone().filter(|hold| now < hold.asks_again_at)
    }

    pub(crate) fn last(&self, project: Option<&str>) -> Option<PullReviewLists> {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner).get(&project.map(str::to_string)).cloned()
    }

    /// Hold the lists as they were read, without lifting a wait: they came from the store.
    pub(crate) fn recalled(&self, project: Option<&str>, lists: &PullReviewLists) {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner).insert(project.map(str::to_string), lists.clone());
    }

    /// Begin reading a project's lists, or find the read going. **One a project**, so a second ask joins it.
    pub(crate) fn begin(&self, project: Option<&str>) -> Reading {
        let key = project.map(str::to_string);
        let mut going = self.going.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(wait) = going.get(&key) {
            return Reading::Going(wait.clone());
        }
        let (done, wait) = watch::channel(());
        going.insert(key.clone(), wait);
        Reading::Began(Reader { project: key, going: Arc::clone(&self.going), _done: done })
    }
}

/// What a rate-limited read answers: the lists last read, marked, else the refusal.
pub(crate) fn limited(kept: Option<PullReviewLists>, hold: Hold) -> Result<PullReviewLists, (Option<String>, String)> {
    let until = hold.until.map(rfc3339_utc);
    match kept {
        Some(mut lists) => {
            lists.rate_limited = Some(PullReviewRateLimited { until: until.map(ipc::Instant::carried), said: hold.said });
            Ok(lists)
        }
        None => Err((until, hold.said)),
    }
}

/// The worst of the lists' failures: the rate limit, which names a time, then no token, then silence.
pub(crate) fn worst(failures: Vec<ListFailure>) -> Option<ListFailure> {
    let rank = |failure: &ListFailure| match failure {
        ListFailure::RateLimited { .. } => 0,
        ListFailure::NoToken(_) => 1,
        ListFailure::Unreachable(_) => 2,
        ListFailure::Other(_) => 3,
    };
    // Where two are rate limits, the later time is the one that clears them both.
    let latest = failures.iter().filter_map(|it| matches!(it, ListFailure::RateLimited { until: Some(_), .. }).then_some(it)).cloned().max_by_key(|it| match it {
        ListFailure::RateLimited { until, .. } => *until,
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
