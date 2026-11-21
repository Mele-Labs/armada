//! A review asked of the person by name: looked for every [`EVERY`], told once a request as a
//! `review_requested` arrival, and standing as a call while he is asked. `docs/practices/protocol/a-jobs-calls.md`,
//! *A review asked of him*.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, ReviewList, ReviewListAsk, Vcs, WorkProduct};
use core_model::Timestamp;
use ipc::{CallArrival, CallArrivalKind, CallTone, Instant, PullInbox, PullReviewRow, ReviewAsked, StandingCall, StandingCallKind, MOST_PER_LIST};
use ipc::ManifestId;
use store::{KeptPullOpened, KeptReviewRequest};
use tokio::task::JoinHandle;

use crate::clock::{elapsed, rfc3339_utc};
use crate::daemon::Fleet;
use crate::pull_review_lists::row_of;

/// How often the forge is asked.
pub const EVERY: Duration = Duration::from_secs(5 * 60);

/// How long after his review a listing of him is not read as asked again: the forge's search can
/// list him for a while after he reviewed.
const SETTLING: Duration = EVERY;

/// The call's key: the repository lowercase, so the forge's casing of it does not make two.
fn key_of(repository: &str, number: u64) -> String {
    format!("review:{}#{number}", repository.to_lowercase())
}

/// Whether a page read at `read` was of this request: any read for a first one, else a read after his
/// review before it. An instant that does not parse reads as not since.
pub(crate) fn read_since(after: &str, read: &str) -> bool {
    let millis = |at: &str| Timestamp::from_rfc3339(at.to_string()).epoch_millis();
    after.is_empty() || matches!((millis(read), millis(after)), (Some(read), Some(after)) if read > after)
}

/// Each row the forge gave no time he was asked, the time Fleet told him of the request: the latest
/// told for that pull request, the request standing. **Only as old as the look after the ask**, so
/// the forge's own time is kept where it gave one.
fn asked_told_of(rows: &mut [PullReviewRow], kept: &[KeptReviewRequest]) {
    for row in rows.iter_mut().filter(|row| row.asked_at.is_none()) {
        let repository = row.repository.to_lowercase();
        row.asked_at = kept
            .iter()
            .filter(|it| it.repository == repository && it.number == row.number)
            .map(|it| &it.told_at)
            .max()
            .cloned();
    }
}

/// The call on him for one pull request, standing while he is asked. **Opened once he has opened it
/// since he was asked**: its page read since the request. A first request is opened too by a seen
/// head kept or an inbox other than a first look; one made again after his review is not, since his
/// review and his visits were before it.
fn call_of(row: &PullReviewRow, since: Instant, read: bool, first: bool) -> StandingCall {
    let key = key_of(&row.repository, row.number);
    StandingCall {
        owner: key.clone(),
        key,
        kind: StandingCallKind::ReviewRequested,
        session_id: None,
        title: row.title.clone(),
        text: format!("Review requested on {}#{}, opened by {}", row.repository, row.number, row.author),
        since,
        tone: CallTone::Waiting,
        opened: read || (first && (row.seen.is_some() || row.inbox != PullInbox::NeedsFirstLook)),
        mac: None,
        review: Some(ReviewAsked {
            manifest_id: row.manifest_id.clone(),
            repository: row.repository.clone(),
            number: row.number,
            title: row.title.clone(),
            author: row.author.clone(),
            address: row.address.clone(),
        }),
    }
}

/// The arrival a request is told as. `session_id` carries the key for readers that do not read `key`.
fn arrival_of(call: &StandingCall) -> CallArrival {
    CallArrival {
        session_id: ipc::SessionId::carried(call.key.clone()),
        key: Some(call.key.clone()),
        title: call.title.clone(),
        text: call.text.clone(),
        tone: call.tone,
        kind: CallArrivalKind::ReviewRequested,
        item: None,
    }
}

/// Look every `every`. `said` hears why a tick was skipped, once each time the reason moves.
pub fn keep_asking_for_reviews<H, V, W>(fleet: Arc<Fleet<H, V, W>>, every: Duration, said: impl Fn(&str) + Send + Sync + 'static) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last: Option<String> = None;
        loop {
            ticker.tick().await;
            let note = fleet.review_requests_looked().await;
            if note != last {
                if let Some(note) = &note {
                    said(note);
                }
                last = note;
            }
        }
    })
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// One look: the list that asks him by name, each request not told before told once and kept as
    /// told, and every one standing as a call. **A note for the log where the tick was skipped**, and
    /// then what stood stands and nothing is told.
    pub(crate) async fn review_requests_looked(&self) -> Option<String> {
        let skipped = |why: &str| Some(format!("reviews asked of you were not read: {why}"));
        let Ok(from) = self.served_named(None) else {
            return skipped("no repository is served");
        };
        let Some(me) = self.mine_in(from.root()).await.login().map(str::to_string) else {
            return skipped("nobody is signed in to the forge");
        };
        let served = self.named_repositories().await;
        let repositories: Vec<String> = served.iter().map(|it| it.slug.clone()).collect();
        if repositories.is_empty() {
            return skipped("no repository served has a remote the forge could be asked about");
        }
        let asked = self
            .forge_asked(from.root(), move |vcs: &V, root: &str| {
                vcs.pull_review_list(root, &ReviewListAsk { list: ReviewList::Requested, login: &me, repositories: &repositories, most: MOST_PER_LIST })
            })
            .await;
        let Ok(pulls) = asked else {
            return skipped("the forge did not answer");
        };
        // The forge's own latest review of his names the request; the one Fleet wrote is only for the inbox.
        let (mut rows, after): (Vec<PullReviewRow>, Vec<String>) = pulls
            .into_iter()
            .filter_map(|pull| {
                let after = pull.my_latest.as_ref().and_then(|it| it.submitted_at.clone()).unwrap_or_default();
                Some((row_of(pull, &served)?, after))
            })
            .unzip();
        self.written_onto(&mut rows).await;
        self.inbox_onto(&mut rows).await;

        let now = self.clock().now();
        let at = rfc3339_utc(now.epoch_millis().unwrap_or(0));
        let (mut calls, mut news, mut unkept) = (Vec::new(), Vec::new(), None);
        {
            let mut store = self.store().lock().await;
            let kept = store.review_requests_told().unwrap_or_default();
            let opened = store.pull_requests_opened().unwrap_or_default();
            for (row, after) in rows.iter().zip(&after) {
                let repository = row.repository.to_lowercase();
                let before = kept.iter().filter(|it| it.repository == repository && it.number == row.number);
                let told = before.clone().find(|it| it.after_review == *after).map(|it| it.told_at.clone());
                let settling = !after.is_empty() && elapsed(&Timestamp::from_rfc3339(after.clone()), &now) < SETTLING;
                let since = match told {
                    Some(told) => told,
                    // Not read as asked again yet: the call stands from the request before, if there was one.
                    None if settling => before.map(|it| it.told_at.clone()).max().unwrap_or_else(|| at.clone()),
                    None => {
                        let request = KeptReviewRequest { repository: repository.clone(), number: row.number, after_review: after.clone(), told_at: at.clone() };
                        // Told unkept, it would be told again every tick.
                        match store.keep_review_request_told(&request) {
                            Ok(()) => news.push(calls.len()),
                            Err(why) => unkept = Some(format!("a review request could not be kept as told: {why:?}")),
                        }
                        at.clone()
                    }
                };
                let read = opened
                    .iter()
                    .find(|it| it.manifest_id == row.manifest_id.as_str() && it.number == row.number)
                    .is_some_and(|it| read_since(after, &it.opened_at));
                calls.push(call_of(row, Instant::carried(since), read, after.is_empty()));
            }
            // The list read whole: what it no longer holds was taken back, reviewed or closed.
            let still: Vec<(String, u64)> = rows.iter().map(|row| (row.repository.to_lowercase(), row.number)).collect();
            let pages: Vec<(String, u64)> = rows.iter().map(|row| (row.manifest_id.as_str().to_string(), row.number)).collect();
            if let Err(why) = store.forget_review_requests_but(&still).and_then(|()| store.forget_pull_requests_opened_but(&pages)) {
                unkept = Some(format!("review requests no longer asked could not be forgotten: {why:?}"));
            }
        }
        let told = news.into_iter().map(|at| arrival_of(&calls[at])).collect();
        self.calls_reviewed(calls, told);
        unkept
    }

    /// [`asked_told_of`] for the rows of the list that asks him by name. Unreadable, the rows keep
    /// what the forge gave.
    pub(crate) async fn asked_told_onto(&self, rows: &mut [PullReviewRow]) {
        let kept = self.store().lock().await.review_requests_told().unwrap_or_default();
        asked_told_of(rows, &kept);
    }

    /// He opened a pull request's page in Bridge (`PullRequestPanel::pull_request_opened`). Kept for the
    /// next look, and a review call on it opened now.
    pub(crate) async fn review_pull_opened(&self, manifest_id: &ManifestId, number: u64) {
        let kept = KeptPullOpened { manifest_id: manifest_id.as_str().to_string(), number, opened_at: self.clock().now().as_str().to_string() };
        // Unkept, the next look reads it unopened again until his next read; the page is served either way.
        let _ = self.store().lock().await.keep_pull_request_opened(&kept);
        self.calls_review_opened(manifest_id.as_str(), number);
    }
}
