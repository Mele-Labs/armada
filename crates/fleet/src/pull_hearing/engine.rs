//! Which pull requests the forge is subscribed to for Fleet, and what its streams say. **No Fleet
//! in it**: it is told what is wanted and answers what to read again, so a recorded stream and a
//! fake forge drive it in a test.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{PullEvents, PullEventsFailure, PullStreamAction, PullStreamEnd, PullStreamSignal};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Instant;

/// How often what is wanted is read and the subscriptions made to match it.
const TICK: Duration = Duration::from_secs(15);
/// Events for one pull request this close together are one read: a push sets off a head change, a
/// check suite and a mergeability change within a moment of each other.
const COALESCE: Duration = Duration::from_secs(2);
/// A subscription is made again this long before it ends, so none ever lapses under a stream.
const RENEW_BEFORE: Duration = Duration::from_secs(3600);
/// How long events stay off once the account says it has them off. The sweep reads meanwhile.
const OFF_FOR: Duration = Duration::from_secs(6 * 3600);
/// Another try at the account's switch after a failure that was not the switch.
const PROBE_AGAIN: Duration = Duration::from_secs(60);
/// A pull request heard live is still read in full this often, for what no event says: a plain
/// comment, and any event the stream missed. The owner chose ten minutes on 25 Nov 2026.
const SAFETY: Duration = Duration::from_secs(600);
/// The forge's limit for one token. The newest pull requests are subscribed and the rest wait for the sweep.
const MOST: usize = 100;
/// The wait before a stream the forge asked to have reopened is reopened.
const REOPEN_AFTER: Duration = Duration::from_secs(5);
/// A refused subscription is asked again after this, doubling for each in a row.
const BACKOFF_FIRST: Duration = Duration::from_secs(30);
const BACKOFF_LONGEST: Duration = Duration::from_secs(900);
/// The wait when the token already holds every subscription the forge allows.
const LIMIT_WAIT: Duration = Duration::from_secs(300);

/// The waits, so a test can run them in milliseconds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Timing {
    pub(crate) tick: Duration,
    pub(crate) coalesce: Duration,
    pub(crate) renew_before: Duration,
    pub(crate) off_for: Duration,
    pub(crate) probe_again: Duration,
    pub(crate) reopen_after: Duration,
    pub(crate) backoff_first: Duration,
    pub(crate) backoff_longest: Duration,
    pub(crate) limit_wait: Duration,
    pub(crate) safety: Duration,
}

impl Timing {
    pub(crate) const SHIPPED: Timing = Timing {
        tick: TICK,
        coalesce: COALESCE,
        renew_before: RENEW_BEFORE,
        off_for: OFF_FOR,
        probe_again: PROBE_AGAIN,
        reopen_after: REOPEN_AFTER,
        backoff_first: BACKOFF_FIRST,
        backoff_longest: BACKOFF_LONGEST,
        limit_wait: LIMIT_WAIT,
        safety: SAFETY,
    };
}

/// A pull request Fleet wants to hear about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Want {
    pub(crate) manifest: String,
    pub(crate) root: String,
    pub(crate) number: u64,
}

/// The pull requests whose stream is attached now, by root and number: **the minute sweep leaves
/// these to their events** and the engine's own safety read. Written only by the engine.
#[derive(Clone, Debug, Default)]
pub(crate) struct Live(Arc<Mutex<BTreeSet<(String, u64)>>>);

impl Live {
    pub(crate) fn contains(&self, root: &str, number: u64) -> bool {
        self.0.lock().is_ok_and(|live| live.contains(&(root.to_string(), number)))
    }

    /// A stream attached or stopped, as a test of the sweep stands in for the engine.
    #[cfg(test)]
    pub(crate) fn mark(&self, root: &str, number: u64, live: bool) {
        if let Ok(mut held) = self.0.lock() {
            match live {
                true => held.insert((root.to_string(), number)),
                false => held.remove(&(root.to_string(), number)),
            };
        }
    }

    fn set(&self, now: BTreeSet<(String, u64)>) {
        if let Ok(mut live) = self.0.lock() {
            *live = now;
        }
    }
}

/// What a stream's reader tells the engine.
#[derive(Debug)]
pub(crate) enum Heard {
    /// The pull request changed.
    Changed(Want),
    /// The subscription's stream attached: it is read once, as nothing is replayed after a gap.
    Attached(Want, String),
    /// The subscription's stream stopped, for whatever reason; until it attaches again the sweep reads it.
    Detached(Want, String),
    /// The subscription is over, as the forge said it.
    Ended(Want, String),
    /// The forge will not stream it, and asking again changes nothing.
    Refused(Want, String),
    /// The account has events switched off.
    Off,
}

enum Mode {
    On,
    Off { until: Instant },
}

struct Held {
    id: String,
    renew_at: Instant,
    attached: bool,
    /// When it was last read on the engine's say, an event's or the safety read's.
    read_at: Option<Instant>,
    stop: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl Held {
    fn stopped(self) {}
}

/// **Dropping a subscription's hold stops its stream**: the reader answers `Stop` at its next beat.
impl Drop for Held {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.task.abort();
    }
}

pub(crate) struct Hearing {
    source: Arc<dyn PullEvents>,
    timing: Timing,
    /// Where a change of state is said: a line for whoever reads Fleet's log.
    told: Arc<dyn Fn(&str) + Send + Sync>,
    mode: Mode,
    held: BTreeMap<Want, Held>,
    /// What the forge will not stream, until Fleet stops wanting it.
    refused: BTreeSet<Want>,
    /// No new subscription before this, after a refusal that may pass.
    paused_until: Option<Instant>,
    failures: u32,
    due: BTreeMap<Want, Instant>,
    heard: mpsc::UnboundedSender<Heard>,
    live: Live,
}

impl Hearing {
    pub(crate) fn opened(
        source: Arc<dyn PullEvents>,
        timing: Timing,
        told: Arc<dyn Fn(&str) + Send + Sync>,
        live: Live,
    ) -> (Hearing, mpsc::UnboundedReceiver<Heard>) {
        let (heard, rx) = mpsc::unbounded_channel();
        let hearing = Hearing {
            source,
            timing,
            told,
            mode: Mode::On,
            held: BTreeMap::new(),
            refused: BTreeSet::new(),
            paused_until: None,
            failures: 0,
            due: BTreeMap::new(),
            heard,
            live,
        };
        (hearing, rx)
    }

    /// What a stream is open for.
    #[cfg(test)]
    pub(crate) fn held(&self) -> Vec<Want> {
        self.held.keys().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn is_off(&self) -> bool {
        matches!(self.mode, Mode::Off { .. })
    }

    /// Make the subscriptions match `wanted`: end those no longer wanted or about to lapse,
    /// and make the newest of the rest, up to the forge's limit.
    pub(crate) async fn pass(&mut self, wanted: &[Want]) {
        let now = Instant::now();
        self.refused.retain(|one| wanted.contains(one));
        if let Mode::Off { until } = self.mode {
            if now < until || !self.probed(now).await {
                return;
            }
        }
        let gone: Vec<Want> = self
            .held
            .iter()
            .filter(|(want, held)| !wanted.contains(want) || now >= held.renew_at)
            .map(|(want, _)| want.clone())
            .collect();
        for want in gone {
            self.ended(&want).await;
        }
        if self.paused_until.is_some_and(|until| now < until) {
            return;
        }
        let mut missing: Vec<&Want> =
            wanted.iter().filter(|want| !self.held.contains_key(*want) && !self.refused.contains(*want)).collect();
        missing.sort_by(|one, other| other.number.cmp(&one.number));
        let room = MOST.saturating_sub(self.held.len());
        for want in missing.into_iter().take(room) {
            if !self.subscribed(want, now).await {
                return;
            }
        }
    }

    /// Whether the account has events on, after the wait. Says so, and turns them back on, if it does.
    async fn probed(&mut self, now: Instant) -> bool {
        let source = Arc::clone(&self.source);
        match tokio::task::spawn_blocking(move || source.probe()).await {
            Ok(Ok(())) => {
                (self.told)("pull request events are on for this account");
                self.mode = Mode::On;
                true
            }
            Ok(Err(PullEventsFailure::NotEnabled)) => {
                self.mode = Mode::Off { until: now + self.timing.off_for };
                false
            }
            _ => {
                self.mode = Mode::Off { until: now + self.timing.probe_again };
                false
            }
        }
    }

    /// Subscribe to one. Says whether to go on to the next.
    async fn subscribed(&mut self, want: &Want, now: Instant) -> bool {
        let (source, root, number) = (Arc::clone(&self.source), want.root.clone(), want.number);
        let made = tokio::task::spawn_blocking(move || source.subscribe(&root, number))
            .await
            .unwrap_or(Err(PullEventsFailure::Transient));
        match made {
            Ok(subscription) => {
                self.failures = 0;
                // One with less left than the margin, as one made earlier is, is made again halfway.
                let renew_in = match subscription.left.checked_sub(self.timing.renew_before) {
                    Some(early) if !early.is_zero() => early,
                    _ => subscription.left / 2,
                };
                let renew_at = now + renew_in;
                let stop = Arc::new(AtomicBool::new(false));
                let task = tokio::spawn(listening(
                    Arc::clone(&self.source),
                    want.clone(),
                    subscription.id.clone(),
                    subscription.events_url,
                    Arc::clone(&stop),
                    self.heard.clone(),
                    self.timing,
                ));
                self.held.insert(want.clone(), Held { id: subscription.id, renew_at, attached: false, read_at: None, stop, task });
                true
            }
            Err(PullEventsFailure::NotEnabled) => {
                self.turned_off(now);
                false
            }
            Err(PullEventsFailure::Forbidden | PullEventsFailure::NotOnThisForge | PullEventsFailure::Other(_)) => {
                self.refused.insert(want.clone());
                true
            }
            Err(PullEventsFailure::LimitReached) => self.paused(now, self.timing.limit_wait),
            Err(PullEventsFailure::RetryAfter(wait)) => self.paused(now, wait),
            Err(PullEventsFailure::Unauthorized | PullEventsFailure::Transient) => {
                self.failures = self.failures.saturating_add(1);
                let wait = self.timing.backoff_first * 2u32.saturating_pow(self.failures.min(6) - 1);
                self.paused(now, wait.min(self.timing.backoff_longest))
            }
        }
    }

    fn paused(&mut self, now: Instant, wait: Duration) -> bool {
        self.paused_until = Some(now + wait);
        false
    }

    /// The account says events are off: nothing is held, nothing opens, and the probe asks again
    /// after [`OFF_FOR`]. **No subscription is deleted**: they lapse in a day, and none is wanted.
    fn turned_off(&mut self, now: Instant) {
        if matches!(self.mode, Mode::On) {
            (self.told)("pull request events are off for this account; the sweep reads pull requests meanwhile");
        }
        for (_, held) in std::mem::take(&mut self.held) {
            held.stopped();
        }
        self.due.clear();
        self.mode = Mode::Off { until: now + self.timing.off_for };
    }

    /// End one subscription at the forge, and stop reading it.
    async fn ended(&mut self, want: &Want) {
        self.due.remove(want);
        let Some(held) = self.held.remove(want) else { return };
        let (source, id) = (Arc::clone(&self.source), held.id.clone());
        held.stopped();
        // A subscription that would not end lapses in a day, so a failure is not retried.
        let _ = tokio::task::spawn_blocking(move || source.unsubscribe(&id)).await;
    }

    /// What a stream's reader said.
    pub(crate) fn took(&mut self, heard: Heard) {
        match heard {
            Heard::Changed(want) => {
                if self.held.contains_key(&want) {
                    self.due.entry(want).or_insert_with(|| Instant::now() + self.timing.coalesce);
                }
            }
            Heard::Attached(want, id) => {
                if let Some(held) = self.held.get_mut(&want).filter(|held| held.id == id) {
                    held.attached = true;
                    self.due.entry(want).or_insert_with(|| Instant::now() + self.timing.coalesce);
                }
            }
            Heard::Detached(want, id) => {
                if let Some(held) = self.held.get_mut(&want).filter(|held| held.id == id) {
                    held.attached = false;
                }
            }
            Heard::Ended(want, id) => self.released(&want, &id, false),
            Heard::Refused(want, id) => self.released(&want, &id, true),
            Heard::Off => self.turned_off(Instant::now()),
        }
    }

    /// A subscription its reader says is over, without deleting it. **Only the one still held**: a
    /// stream of a subscription since replaced says nothing about the new one.
    fn released(&mut self, want: &Want, id: &str, refused: bool) {
        if !self.held.get(want).is_some_and(|held| held.id == id) {
            return;
        }
        if let Some(held) = self.held.remove(want) {
            held.stopped();
        }
        self.due.remove(want);
        if refused {
            self.refused.insert(want.clone());
        }
    }

    /// Each attached pull request not read for [`SAFETY`] is due now: no event covers every change.
    fn safety_due(&mut self, now: Instant) {
        for (want, held) in &self.held {
            let stale = held.read_at.is_none_or(|at| now.duration_since(at) >= self.timing.safety);
            if held.attached && stale {
                self.due.entry(want.clone()).or_insert(now);
            }
        }
    }

    /// Tell the sweep which pull requests are heard live, as of now.
    fn publish(&self) {
        let live = self.held.iter().filter(|(_, held)| held.attached).map(|(want, _)| (want.root.clone(), want.number));
        self.live.set(live.collect());
    }

    /// When the next pull request is due to be read.
    pub(crate) fn next_due(&self) -> Option<Instant> {
        self.due.values().min().copied()
    }

    /// The pull requests whose wait is over, each once.
    pub(crate) fn due_now(&mut self) -> Vec<Want> {
        let now = Instant::now();
        let due: Vec<Want> = self.due.iter().filter(|(_, at)| **at <= now).map(|(want, _)| want.clone()).collect();
        for want in &due {
            self.due.remove(want);
            if let Some(held) = self.held.get_mut(want) {
                held.read_at = Some(now);
            }
        }
        due
    }

    /// A pull request that merged or closed: its subscription is ended.
    pub(crate) async fn over(&mut self, want: &Want) {
        self.ended(want).await;
    }

    /// Run until Fleet stops: read what is wanted every [`TICK`], and read a pull request again
    /// when its streams have said it changed. `reread` says whether the pull request is over.
    pub(crate) async fn run<W, WF, R, RF>(
        mut self,
        mut heard: mpsc::UnboundedReceiver<Heard>,
        wanted: W,
        reread: R,
    ) where
        W: Fn() -> WF,
        WF: Future<Output = Vec<Want>>,
        R: Fn(Want) -> RF,
        RF: Future<Output = bool>,
    {
        let mut ticker = tokio::time::interval(self.timing.tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            let due = self.next_due();
            tokio::select! {
                _ = ticker.tick() => {
                    let wanted = wanted().await;
                    self.pass(&wanted).await;
                    self.safety_due(Instant::now());
                }
                Some(said) = heard.recv() => self.took(said),
                _ = async {
                    match due {
                        Some(at) => tokio::time::sleep_until(at).await,
                        None => std::future::pending().await,
                    }
                } => {
                    for want in self.due_now() {
                        if reread(want.clone()).await {
                            self.over(&want).await;
                        }
                    }
                }
            }
            self.publish();
        }
    }
}

/// One subscription's stream, read until it ends, and opened again where it should be.
async fn listening(
    source: Arc<dyn PullEvents>,
    want: Want,
    id: String,
    events_url: String,
    stop: Arc<AtomicBool>,
    told: mpsc::UnboundedSender<Heard>,
    timing: Timing,
) {
    let mut failures = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let (source, url, flag, out, one, sub) =
            (Arc::clone(&source), events_url.clone(), Arc::clone(&stop), told.clone(), want.clone(), id.clone());
        let opened = Instant::now();
        let read = tokio::task::spawn_blocking(move || {
            let mut attached = false;
            let end = source.listen(&url, &mut |signal| {
                if flag.load(Ordering::SeqCst) {
                    return PullStreamAction::Stop;
                }
                match signal {
                    PullStreamSignal::Attached => {
                        attached = true;
                        let _ = out.send(Heard::Attached(one.clone(), sub.clone()));
                    }
                    PullStreamSignal::Changed => {
                        let _ = out.send(Heard::Changed(one.clone()));
                    }
                    PullStreamSignal::Beat => {}
                }
                PullStreamAction::Keep
            });
            (end, attached)
        })
        .await;
        let Ok((end, attached)) = read else { return };
        if attached {
            let _ = told.send(Heard::Detached(want.clone(), id.clone()));
        }
        // A stream that held for a while was a good one, whatever ended it.
        if attached && opened.elapsed() >= timing.backoff_first {
            failures = 0;
        }
        let wait = match end {
            PullStreamEnd::Stopped => return,
            PullStreamEnd::SubscriptionEnded => {
                let _ = told.send(Heard::Ended(want, id));
                return;
            }
            PullStreamEnd::Reconnect => {
                failures = 0;
                timing.reopen_after
            }
            PullStreamEnd::Dropped => timing.reopen_after * 2u32.saturating_pow(failures.min(6)),
            PullStreamEnd::Failed(PullEventsFailure::NotEnabled) => {
                let _ = told.send(Heard::Off);
                return;
            }
            PullStreamEnd::Failed(PullEventsFailure::Forbidden | PullEventsFailure::NotOnThisForge | PullEventsFailure::Other(_)) => {
                let _ = told.send(Heard::Refused(want, id));
                return;
            }
            PullStreamEnd::Failed(PullEventsFailure::RetryAfter(wait)) => wait,
            PullStreamEnd::Failed(_) => timing.backoff_first * 2u32.saturating_pow(failures.min(4)),
        };
        failures = failures.saturating_add(1);
        tokio::time::sleep(wait.min(timing.backoff_longest)).await;
    }
}
