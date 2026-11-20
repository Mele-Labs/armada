//! GitHub's event streams for pull requests: Fleet subscribes to a pull request a Job owns and
//! hears when it changes, so it reads that one at once. `fleet::pull_hearing`.
//!
//! **Over the client and token Fleet already has**, not `gh`. A subscription lasts 24 hours and a
//! token holds 100; asking for one that exists returns it, so a restart finds its own again.
//! `frames` reads the stream, and `failure_of` is how each refusal is told apart.

pub(crate) mod frames;

use std::io::Read;
use std::time::Duration;

use adapter_traits::{
    PullEvents, PullEventsFailure, PullStreamAction, PullStreamEnd, PullStreamSignal, PullSubscription,
};
use core_model::Timestamp;
use serde::{Deserialize, Serialize};

use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository, GITHUB_HOST};
use frames::{Frame, Frames, Outcome};

const SUBSCRIPTIONS: &str = "/event-subscriptions";
/// All five, because any of them means the pull request is worth reading again.
const EVENT_TYPES: [&str; 5] = [
    "com.github.pull_request.review.submitted.v1",
    "com.github.pull_request.state.changed.v1",
    "com.github.pull_request.head.changed.v1",
    "com.github.pull_request.merge_conflict.v1",
    "com.github.pull_request.check_suite.status.changed.v1",
];
/// What a stream is read in at a time.
const READ_AT_A_TIME: usize = 4096;
/// A subscription whose end cannot be read is taken to have a day, which is what the forge gives.
const A_DAY: Duration = Duration::from_secs(24 * 3600);
/// A wait for a refusal that named none, for a caller that has no backoff of its own.
const BRIEF_WAIT: Duration = Duration::from_secs(30);

/// The forge's events, over the process's one client.
#[derive(Clone)]
pub struct ApiPullEvents {
    api: GitHubApi,
}

impl ApiPullEvents {
    /// Through the client every reader of the forge shares.
    pub fn shared() -> ApiPullEvents {
        ApiPullEvents { api: GitHubApi::shared() }
    }

    #[cfg(test)]
    pub(crate) fn through(api: GitHubApi) -> ApiPullEvents {
        ApiPullEvents { api }
    }
}

#[derive(Serialize)]
struct Subscribing<'a> {
    resource_url: String,
    event_types: &'a [&'a str],
}

#[derive(Deserialize)]
struct Subscribed {
    id: String,
    events_url: String,
        expires_at: Option<String>,
}

impl PullEvents for ApiPullEvents {
    fn subscribe(&self, in_repo: &str, number: u64) -> Result<PullSubscription, PullEventsFailure> {
        let repository =
            GitHubRepository::of_checkout(std::path::Path::new(in_repo)).map_err(|_| PullEventsFailure::NotOnThisForge)?;
        let resource_url = format!("https://{GITHUB_HOST}/{}/pull/{number}", repository.address().slug());
        let answer = self
            .api
            .post(SUBSCRIPTIONS, &Subscribing { resource_url, event_types: &EVENT_TYPES })
            .map_err(|why| failure_of(&why))?;
        let made: Subscribed = answer.decode("an event subscription").map_err(|why| failure_of(&why))?;
        Ok(PullSubscription { left: left(made.expires_at, now_millis()), id: made.id, events_url: made.events_url })
    }

    fn unsubscribe(&self, subscription: &str) -> Result<(), PullEventsFailure> {
        match self.api.delete(&format!("{SUBSCRIPTIONS}/{subscription}")) {
            Ok(_) => Ok(()),
            // Already gone.
            Err(ApiFailure::Refused { status: 404, .. }) => Ok(()),
            Err(why) => Err(failure_of(&why)),
        }
    }

    fn probe(&self) -> Result<(), PullEventsFailure> {
        self.api.get(SUBSCRIPTIONS).map(|_| ()).map_err(|why| failure_of(&why))
    }

    fn listen(
        &self,
        events_url: &str,
        signal: &mut dyn FnMut(PullStreamSignal) -> PullStreamAction,
    ) -> PullStreamEnd {
        let mut response = match self.api.stream_from(events_url) {
            Ok(response) => response,
            Err(why) => return PullStreamEnd::Failed(failure_of(&why)),
        };
        if signal(PullStreamSignal::Attached) == PullStreamAction::Stop {
            return PullStreamEnd::Stopped;
        }
        let mut frames = Frames::default();
        let mut chunk = [0u8; READ_AT_A_TIME];
        loop {
            let read = match response.read(&mut chunk) {
                Ok(0) | Err(_) => return PullStreamEnd::Dropped,
                Ok(read) => read,
            };
            if signal(PullStreamSignal::Beat) == PullStreamAction::Stop {
                return PullStreamEnd::Stopped;
            }
            for frame in frames.feed(&chunk[..read]) {
                match frame {
                    Frame::End(Outcome::SubscriptionEnded) => return PullStreamEnd::SubscriptionEnded,
                    Frame::End(Outcome::Reconnect | Outcome::Unknown(_)) => return PullStreamEnd::Reconnect,
                    Frame::Event { .. } => {
                        if signal(PullStreamSignal::Changed) == PullStreamAction::Stop {
                            return PullStreamEnd::Stopped;
                        }
                    }
                }
            }
        }
    }
}

/// How long a subscription has left, from its end date and `now` in milliseconds since the epoch.
pub(crate) fn left(expires: Option<String>, now: i64) -> Duration {
    match expires.and_then(|at| Timestamp::from_rfc3339(at).epoch_millis()) {
        Some(to) => Duration::from_millis(to.saturating_sub(now).max(0).unsigned_abs()),
        None => A_DAY,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |it| i64::try_from(it.as_millis()).unwrap_or(i64::MAX))
}

/// What a refusal means for events. **403 says two things**: the account has events off, which
/// no retry changes, or something else was refused.
pub(crate) fn failure_of(failure: &ApiFailure) -> PullEventsFailure {
    match failure {
        ApiFailure::NoToken(_) => PullEventsFailure::Unauthorized,
        ApiFailure::Unreachable(_) => PullEventsFailure::Transient,
        ApiFailure::HeldBack { seconds } | ApiFailure::RateLimited { seconds, .. } => {
            PullEventsFailure::RetryAfter(Duration::from_secs(*seconds))
        }
        ApiFailure::Refused { status, said, limit } => match status {
            401 => PullEventsFailure::Unauthorized,
            403 if said.to_ascii_lowercase().contains("not enabled") => PullEventsFailure::NotEnabled,
            403 | 404 => match limit.and_then(|it| it.retry_after) {
                Some(seconds) => PullEventsFailure::RetryAfter(Duration::from_secs(seconds)),
                None => PullEventsFailure::Forbidden,
            },
            422 if said.to_ascii_lowercase().contains("limit") => PullEventsFailure::LimitReached,
            429 | 503 => match limit.and_then(|it| it.retry_after) {
                Some(seconds) => PullEventsFailure::RetryAfter(Duration::from_secs(seconds)),
                None => PullEventsFailure::RetryAfter(BRIEF_WAIT),
            },
            500..=599 => PullEventsFailure::Transient,
            _ => PullEventsFailure::Other(format!("{status}: {said}")),
        },
        ApiFailure::GraphQl(_) | ApiFailure::Malformed(_) => PullEventsFailure::Other(failure.to_string()),
    }
}
