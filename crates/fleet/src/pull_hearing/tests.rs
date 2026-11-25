//! The subscriptions against a fake forge whose streams a test plays by hand.

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{
    PullEvents, PullEventsFailure, PullStreamAction, PullStreamEnd, PullStreamSignal, PullSubscription,
};

use super::engine::{Hearing, Live, Timing, Want};

/// Every wait in milliseconds, none long enough to be waited on for real.
const QUICK: Timing = Timing {
    tick: Duration::from_millis(20),
    coalesce: Duration::from_millis(60),
    renew_before: Duration::from_secs(3600),
    off_for: Duration::from_millis(80),
    probe_again: Duration::from_millis(20),
    reopen_after: Duration::from_millis(10),
    backoff_first: Duration::from_millis(200),
    backoff_longest: Duration::from_millis(400),
    limit_wait: Duration::from_millis(200),
    // Longer than any test, so no safety read joins what a test counts unless it asks for one.
    safety: Duration::from_secs(3600),
};

enum Play {
    Signal(PullStreamSignal),
    End(PullStreamEnd),
}

#[derive(Default)]
struct Calls {
    subscribed: Vec<u64>,
    unsubscribed: Vec<String>,
    probed: usize,
    listened: usize,
}

/// A forge that answers what a test queued, and plays each stream from a channel.
#[derive(Default)]
struct Forge {
    calls: Mutex<Calls>,
    subscribe_answers: Mutex<VecDeque<Result<Duration, PullEventsFailure>>>,
    probe_answers: Mutex<VecDeque<Result<(), PullEventsFailure>>>,
    streams: Mutex<VecDeque<Receiver<Play>>>,
}

impl Forge {
    fn stream(&self) -> Sender<Play> {
        let (play, stream) = channel();
        self.streams.lock().unwrap().push_back(stream);
        play
    }

    fn subscribed(&self) -> Vec<u64> {
        self.calls.lock().unwrap().subscribed.clone()
    }

    fn unsubscribed(&self) -> Vec<String> {
        self.calls.lock().unwrap().unsubscribed.clone()
    }

    fn listened(&self) -> usize {
        self.calls.lock().unwrap().listened
    }
}

impl PullEvents for Forge {
    fn subscribe(&self, _: &str, number: u64) -> Result<PullSubscription, PullEventsFailure> {
        let mut calls = self.calls.lock().unwrap();
        calls.subscribed.push(number);
        let left = self.subscribe_answers.lock().unwrap().pop_front().unwrap_or(Ok(Duration::from_secs(86_400)))?;
        Ok(PullSubscription {
            id: format!("sub-{}", calls.subscribed.len()),
            events_url: format!("https://forge.invalid/event-subscriptions/{number}/events"),
            left,
        })
    }

    fn unsubscribe(&self, subscription: &str) -> Result<(), PullEventsFailure> {
        self.calls.lock().unwrap().unsubscribed.push(subscription.to_string());
        Ok(())
    }

    fn probe(&self) -> Result<(), PullEventsFailure> {
        self.calls.lock().unwrap().probed += 1;
        self.probe_answers.lock().unwrap().pop_front().unwrap_or(Ok(()))
    }

    fn listen(&self, _: &str, signal: &mut dyn FnMut(PullStreamSignal) -> PullStreamAction) -> PullStreamEnd {
        self.calls.lock().unwrap().listened += 1;
        let stream = self.streams.lock().unwrap().pop_front();
        loop {
            let played = match &stream {
                Some(stream) => stream.recv_timeout(Duration::from_millis(5)),
                None => {
                    std::thread::sleep(Duration::from_millis(5));
                    Err(RecvTimeoutError::Timeout)
                }
            };
            match played {
                Ok(Play::Signal(heard)) => {
                    if signal(heard) == PullStreamAction::Stop {
                        return PullStreamEnd::Stopped;
                    }
                }
                Ok(Play::End(end)) => return end,
                Err(RecvTimeoutError::Timeout) => {
                    if signal(PullStreamSignal::Beat) == PullStreamAction::Stop {
                        return PullStreamEnd::Stopped;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return PullStreamEnd::Dropped,
            }
        }
    }
}

fn silent() -> Arc<dyn Fn(&str) + Send + Sync> {
    Arc::new(|_| {})
}

fn want(number: u64) -> Want {
    Want { manifest: "armada".into(), root: "/repo".into(), number }
}

async fn until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..1000 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("never happened: {what}");
}

/// Hearing for `wanted`, with the pull requests read again collected in the second value.
fn running(forge: &Arc<Forge>, wanted: Vec<Want>, over: bool) -> (Arc<Mutex<Vec<u64>>>, tokio::task::JoinHandle<()>) {
    let (read, task, _) = running_on(forge, wanted, over, QUICK);
    (read, task)
}

/// As [`running`], on `timing`, with the set the sweep reads.
fn running_on(
    forge: &Arc<Forge>,
    wanted: Vec<Want>,
    over: bool,
    timing: Timing,
) -> (Arc<Mutex<Vec<u64>>>, tokio::task::JoinHandle<()>, Live) {
    let live = Live::default();
    let (hearing, heard) = Hearing::opened(Arc::clone(forge) as Arc<dyn PullEvents>, timing, silent(), live.clone());
    let read = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&read);
    let task = tokio::spawn(hearing.run(
        heard,
        move || {
            let wanted = wanted.clone();
            async move { wanted }
        },
        move |one: Want| {
            let seen = Arc::clone(&seen);
            async move {
                seen.lock().unwrap().push(one.number);
                over
            }
        },
    ));
    (read, task, live)
}

#[tokio::test]
async fn an_event_reads_that_pull_request_again() {
    let forge = Arc::new(Forge::default());
    let play = forge.stream();
    let (read, task) = running(&forge, vec![want(7)], false);
    // Attaching reads once, as nothing is replayed after a disconnect.
    play.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("the attach read", || read.lock().unwrap().len() == 1).await;
    play.send(Play::Signal(PullStreamSignal::Changed)).unwrap();
    until("the event read", || read.lock().unwrap().len() == 2).await;
    assert_eq!(*read.lock().unwrap(), vec![7, 7]);
    assert_eq!(forge.subscribed(), vec![7]);
    task.abort();
}

#[tokio::test]
async fn a_burst_of_events_is_one_read() {
    let forge = Arc::new(Forge::default());
    let play = forge.stream();
    let (read, task) = running(&forge, vec![want(7)], false);
    for _ in 0..5 {
        play.send(Play::Signal(PullStreamSignal::Changed)).unwrap();
    }
    until("a read", || !read.lock().unwrap().is_empty()).await;
    tokio::time::sleep(QUICK.coalesce * 3).await;
    assert_eq!(*read.lock().unwrap(), vec![7]);
    task.abort();
}

#[tokio::test]
async fn a_stream_the_forge_ended_with_reconnect_is_opened_again_and_read_once() {
    let forge = Arc::new(Forge::default());
    let first = forge.stream();
    let second = forge.stream();
    let (read, task) = running(&forge, vec![want(7)], false);
    first.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("the first read", || read.lock().unwrap().len() == 1).await;
    first.send(Play::End(PullStreamEnd::Reconnect)).unwrap();
    until("the stream opened again", || forge.listened() == 2).await;
    second.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("the second read", || read.lock().unwrap().len() == 2).await;
    // The same subscription, not a second one.
    assert_eq!(forge.subscribed(), vec![7]);
    task.abort();
}

#[tokio::test]
async fn a_stream_that_dropped_without_saying_so_is_opened_again() {
    let forge = Arc::new(Forge::default());
    let first = forge.stream();
    let (_read, task) = running(&forge, vec![want(7)], false);
    until("the stream opened", || forge.listened() == 1).await;
    first.send(Play::End(PullStreamEnd::Dropped)).unwrap();
    until("the stream opened again", || forge.listened() == 2).await;
    task.abort();
}

#[tokio::test]
async fn an_ended_subscription_is_made_again_while_the_pull_request_is_wanted() {
    let forge = Arc::new(Forge::default());
    let first = forge.stream();
    let (_read, task) = running(&forge, vec![want(7)], false);
    until("subscribed", || forge.subscribed().len() == 1).await;
    first.send(Play::End(PullStreamEnd::SubscriptionEnded)).unwrap();
    until("subscribed again", || forge.subscribed().len() == 2).await;
    // The forge ended it, so there was nothing to delete.
    assert!(forge.unsubscribed().is_empty());
    task.abort();
}

#[tokio::test]
async fn a_pull_request_found_over_is_unsubscribed() {
    let forge = Arc::new(Forge::default());
    let play = forge.stream();
    let (read, task) = running(&forge, vec![want(7)], true);
    play.send(Play::Signal(PullStreamSignal::Changed)).unwrap();
    until("the read", || !read.lock().unwrap().is_empty()).await;
    until("the subscription ended", || forge.unsubscribed() == vec!["sub-1".to_string()]).await;
    task.abort();
}

#[tokio::test]
async fn a_pull_request_nobody_holds_any_more_is_unsubscribed() {
    let forge = Arc::new(Forge::default());
    let (mut hearing, _heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), Live::default());
    hearing.pass(&[want(7), want(8)]).await;
    assert_eq!(hearing.held(), vec![want(7), want(8)]);
    hearing.pass(&[want(8)]).await;
    assert_eq!(hearing.held(), vec![want(8)]);
    // The newest was subscribed first, so 7 is the second.
    assert_eq!(forge.unsubscribed(), vec!["sub-2".to_string()]);
}

#[tokio::test]
async fn an_account_with_events_off_holds_nothing_and_asks_again_later() {
    let forge = Arc::new(Forge::default());
    forge.subscribe_answers.lock().unwrap().push_back(Err(PullEventsFailure::NotEnabled));
    forge.probe_answers.lock().unwrap().push_back(Err(PullEventsFailure::NotEnabled));
    let said = Arc::new(Mutex::new(Vec::<String>::new()));
    let kept = Arc::clone(&said);
    let (mut hearing, _heard) = Hearing::opened(
        Arc::clone(&forge) as Arc<dyn PullEvents>,
        QUICK,
        Arc::new(move |line| kept.lock().unwrap().push(line.to_string())),
        Live::default(),
    );
    hearing.pass(&[want(7), want(8)]).await;
    assert!(hearing.is_off());
    assert!(hearing.held().is_empty());
    // One refusal stops the pass: the second pull request was not asked about.
    assert_eq!(forge.subscribed(), vec![8]);
    assert_eq!(forge.listened(), 0);
    // Off, a pass does not ask anything.
    hearing.pass(&[want(7), want(8)]).await;
    assert_eq!(forge.subscribed(), vec![8]);
    assert_eq!(forge.calls.lock().unwrap().probed, 0);
    // After the wait it probes, and a forge still saying no keeps it off.
    tokio::time::sleep(QUICK.off_for + Duration::from_millis(20)).await;
    hearing.pass(&[want(7), want(8)]).await;
    assert!(hearing.is_off());
    assert_eq!(forge.calls.lock().unwrap().probed, 1);
    assert_eq!(forge.subscribed(), vec![8]);
    // And one that now says yes turns events back on.
    tokio::time::sleep(QUICK.off_for + Duration::from_millis(20)).await;
    hearing.pass(&[want(7), want(8)]).await;
    assert!(!hearing.is_off());
    assert_eq!(hearing.held(), vec![want(7), want(8)]);
    // Said once going off, and once coming back: not at each probe.
    let said = said.lock().unwrap();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].contains("off") && said[1].contains("on"), "{said:?}");
}

#[tokio::test]
async fn at_the_limit_the_newest_pull_requests_are_subscribed() {
    let forge = Arc::new(Forge::default());
    let (mut hearing, _heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), Live::default());
    let wanted: Vec<Want> = (1..=105).map(want).collect();
    hearing.pass(&wanted).await;
    let held = hearing.held();
    assert_eq!(held.len(), 100);
    assert!(held.iter().all(|one| one.number > 5), "the oldest five are the sweep's");
}

#[tokio::test]
async fn a_subscription_close_to_its_end_is_made_again() {
    let forge = Arc::new(Forge::default());
    // Left for `renew_before` and a moment: due again almost at once.
    forge.subscribe_answers.lock().unwrap().push_back(Ok(QUICK.renew_before + Duration::from_millis(30)));
    let (mut hearing, _heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), Live::default());
    hearing.pass(&[want(7)]).await;
    assert_eq!(forge.subscribed(), vec![7]);
    tokio::time::sleep(Duration::from_millis(60)).await;
    hearing.pass(&[want(7)]).await;
    // The old one is deleted first: asking for a pull request already subscribed returns it as it was.
    assert_eq!(forge.unsubscribed(), vec!["sub-1".to_string()]);
    assert_eq!(forge.subscribed(), vec![7, 7]);
    assert_eq!(hearing.held(), vec![want(7)]);
}

#[tokio::test]
async fn a_refusal_that_may_pass_is_not_asked_again_at_once() {
    let forge = Arc::new(Forge::default());
    forge.subscribe_answers.lock().unwrap().push_back(Err(PullEventsFailure::Transient));
    let (mut hearing, _heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), Live::default());
    hearing.pass(&[want(7)]).await;
    hearing.pass(&[want(7)]).await;
    assert_eq!(forge.subscribed(), vec![7]);
    tokio::time::sleep(QUICK.backoff_first + Duration::from_millis(30)).await;
    hearing.pass(&[want(7)]).await;
    assert_eq!(forge.subscribed(), vec![7, 7]);
    assert_eq!(hearing.held(), vec![want(7)]);
}

#[tokio::test]
async fn a_pull_request_the_forge_refuses_is_left_to_the_sweep() {
    let forge = Arc::new(Forge::default());
    forge.subscribe_answers.lock().unwrap().push_back(Err(PullEventsFailure::Forbidden));
    let (mut hearing, _heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), Live::default());
    hearing.pass(&[want(7), want(8)]).await;
    hearing.pass(&[want(7), want(8)]).await;
    // The newest is asked first, refused once, and not asked again while it is wanted.
    assert_eq!(forge.subscribed(), vec![8, 7]);
    assert_eq!(hearing.held(), vec![want(7)]);
}

#[tokio::test]
async fn an_attached_stream_is_heard_live_until_it_stops() {
    let forge = Arc::new(Forge::default());
    let first = forge.stream();
    let second = forge.stream();
    let (_read, task, live) = running_on(&forge, vec![want(7)], false, QUICK);
    until("the stream opened", || forge.listened() == 1).await;
    assert!(!live.contains("/repo", 7), "subscribed is not heard: nothing has attached");
    first.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("heard live", || live.contains("/repo", 7)).await;
    // Dropped: the sweep reads it every minute again until the stream is back.
    first.send(Play::End(PullStreamEnd::Dropped)).unwrap();
    until("left to the sweep", || !live.contains("/repo", 7)).await;
    second.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("heard live again", || live.contains("/repo", 7)).await;
    task.abort();
}

#[tokio::test]
async fn a_pull_request_nobody_wants_any_more_is_not_heard_live() {
    let forge = Arc::new(Forge::default());
    let play = forge.stream();
    let wanted = Arc::new(Mutex::new(vec![want(7)]));
    let asked = Arc::clone(&wanted);
    let live = Live::default();
    let (hearing, heard) = Hearing::opened(Arc::clone(&forge) as Arc<dyn PullEvents>, QUICK, silent(), live.clone());
    let task = tokio::spawn(hearing.run(
        heard,
        move || {
            let wanted = asked.lock().unwrap().clone();
            async move { wanted }
        },
        |_| async { false },
    ));
    play.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("heard live", || live.contains("/repo", 7)).await;
    wanted.lock().unwrap().clear();
    until("not heard once unwanted", || !live.contains("/repo", 7)).await;
    task.abort();
}

#[tokio::test]
async fn a_pull_request_heard_live_is_still_read_after_the_safety_wait() {
    let forge = Arc::new(Forge::default());
    let play = forge.stream();
    let timing = Timing { safety: Duration::from_millis(100), ..QUICK };
    let (read, task, _live) = running_on(&forge, vec![want(7)], false, timing);
    play.send(Play::Signal(PullStreamSignal::Attached)).unwrap();
    until("the attach read", || read.lock().unwrap().len() == 1).await;
    // No event after the attach: a comment says nothing on the stream, so the safety read finds it.
    until("two safety reads", || read.lock().unwrap().len() >= 3).await;
    task.abort();
}

#[tokio::test]
async fn a_stream_that_never_attached_gets_no_safety_read() {
    let forge = Arc::new(Forge::default());
    let _play = forge.stream();
    let timing = Timing { safety: Duration::from_millis(20), ..QUICK };
    let (read, task, live) = running_on(&forge, vec![want(7)], false, timing);
    until("the stream opened", || forge.listened() == 1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    // The sweep reads it every minute, so the engine adds nothing.
    assert!(read.lock().unwrap().is_empty());
    assert!(!live.contains("/repo", 7));
    task.abort();
}
