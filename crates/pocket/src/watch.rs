//! What pushes: Fleet's `/events`, narrowed to what a Job newly needs from the person.
//!
//! **Only Needs you.** A push goes out when a Job's `waiting_for` gains an item id it
//! did not have and it is still open [`SETTLE`] later. A Stuck Job sends nothing: Fleet
//! repairs it without the person.
//!
//! **A new item settles first.** Several items one Job raises inside the window go out as
//! one push, saying the newest that is still open.
//!
//! **A push is taken back.** When nothing pushed for a Job is open any more, one clear
//! message goes out for it and the phone closes that Job's notification. A phone on Apple's
//! push service is sent none: iOS removes the subscription after a few pushes that show
//! nothing, and a clear shows nothing.
//!
//! **A snoozed Job pushes nothing until its time.** What it raises meanwhile is held, and one
//! push goes out for it when `snoozed_until` clears, if any of it is still open.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use ipc::{Event, StreamMessage, WaitingItem, WaitingSource};
use serde::Serialize;
use tokio::time::Instant;

use crate::fleet_events::Messages;
use crate::vapid::Vapid;
use crate::webpush::{deliver, Delivery};
use crate::{fleet_client, Gateway};

/// How long a new item is held before it is pushed. A permission prompt, which the Mac or an
/// agent's mode answers within seconds, is gone by then, so the phone is never told of
/// something that no longer needs the person.
pub(crate) const SETTLE: Duration = Duration::from_secs(60);

/// The whole payload. Tapping it opens the Job on the Mac `origin` names.
#[derive(Serialize)]
struct Alert {
    session_id: String,
    title: String,
    reason: String,
    /// This Mac's `https://<tailnet name>`, the address the phone paired with.
    /// Absent while the Gateway cannot say its address; the app then finds the Mac holding the Job.
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    /// Set on a message that takes the Job's notification back instead of showing one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    clear: bool,
}

/// What one Job raised that waits out [`SETTLE`], oldest first as `(id, text)`.
struct Pending {
    due: Instant,
    title: String,
    items: Vec<(String, String)>,
}

pub struct Watch {
    gateway: Gateway,
    /// Per Job, the item ids already seen: pushed, waiting to be, or open when Pocket first saw it.
    /// An id leaves when its item does, so an item raised again pushes again.
    sent: HashMap<String, HashSet<String>>,
    /// Per Job, the item ids a push went out for and that are still open. When it empties, the push is taken back.
    pushed: HashMap<String, HashSet<String>>,
    /// Per Job, what was raised and is waiting out its hold.
    pending: HashMap<String, Pending>,
    /// Per Job, the item ids it raised while snoozed, oldest first, pushed as one when it wakes.
    held: HashMap<String, Vec<String>>,
    /// How long a new item is held. [`SETTLE`] outside a test.
    pub(crate) hold: Duration,
    /// How long to wait before each retry of a push that failed.
    pub(crate) retries: Vec<Duration>,
    pushes: tokio::task::JoinSet<()>,
}

impl Watch {
    pub fn new(gateway: Gateway) -> Watch {
        Watch {
            gateway,
            sent: HashMap::new(),
            pushed: HashMap::new(),
            pending: HashMap::new(),
            held: HashMap::new(),
            pushes: tokio::task::JoinSet::new(),
            hold: SETTLE,
            retries: vec![Duration::from_secs(5), Duration::from_secs(30), Duration::from_secs(120)],
        }
    }

    /// One text message from Fleet's `/events`.
    pub async fn observe(&mut self, text: &str) {
        let Ok(StreamMessage::Event(delivered)) = ipc::decode::<StreamMessage>("a stream message", text.as_bytes()) else {
            return;
        };
        if let Event::SessionChanged(record) = delivered.event {
            let id = record.id.as_str().to_string();
            let open: HashSet<String> = record.waiting_for.iter().map(|item| item.id.clone()).collect();
            // The first event for a Job only records what is already open, so a restart
            // of Pocket does not push every item again.
            let Some(known) = self.sent.get_mut(&id) else {
                self.sent.insert(id, open);
                return;
            };
            known.retain(|item| open.contains(item));
            let title = record.title.clone().unwrap_or_else(|| id.clone());
            // Marked seen now, so a repeat of this event queues nothing
            // while the first is held or its push retries on its own task.
            let raised: Vec<&WaitingItem> = record.waiting_for.iter().filter(|item| known.insert(item.id.clone())).collect();
            // What left before its hold ended is never pushed.
            if let Some(waiting) = self.pending.get_mut(&id) {
                waiting.items.retain(|(item, _)| open.contains(item));
                waiting.title = title.clone();
                if waiting.items.is_empty() {
                    self.pending.remove(&id);
                }
            }
            if let Some(pushed) = self.pushed.get_mut(&id) {
                pushed.retain(|item| open.contains(item));
                if pushed.is_empty() {
                    self.pushed.remove(&id);
                    let alert = Alert { session_id: id.clone(), title: title.clone(), reason: String::new(), origin: None, clear: true };
                    self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), alert));
                }
            }
            if record.snoozed_until.is_some() {
                let held = self.held.entry(id.clone()).or_default();
                // What was still waiting out its hold is held for the wake instead.
                if let Some(waiting) = self.pending.remove(&id) {
                    held.extend(waiting.items.into_iter().map(|(item, _)| item));
                }
                held.extend(raised.iter().map(|item| item.id.clone()));
                return;
            }
            let mut held = self.held.remove(&id).unwrap_or_default();
            held.retain(|item| open.contains(item));
            if !held.is_empty() {
                // Back from a snooze: one push for all it raised, saying the newest.
                let newest = raised.last().copied().or_else(|| record.waiting_for.iter().rfind(|item| held.contains(&item.id)));
                if let Some(newest) = newest {
                    let covered = self.pushed.entry(id.clone()).or_default();
                    covered.extend(held);
                    covered.extend(raised.iter().map(|item| item.id.clone()));
                    let alert = Alert { session_id: id.clone(), title, reason: body(newest), origin: None, clear: false };
                    self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), alert));
                }
                return;
            }
            if !raised.is_empty() {
                let due = Instant::now() + self.hold;
                let waiting = self.pending.entry(id).or_insert_with(|| Pending { due, title, items: Vec::new() });
                waiting.items.extend(raised.iter().map(|item| (item.id.clone(), body(item))));
            }
        }
        self.flush_due();
    }

    /// When the earliest held item is due, for the loop to wake at.
    pub(crate) fn next_due(&self) -> Option<Instant> {
        self.pending.values().map(|waiting| waiting.due).min()
    }

    /// Pushes each Job whose hold has ended: one push, saying the newest of what is still open.
    pub(crate) fn flush_due(&mut self) {
        let now = Instant::now();
        let due: Vec<String> = self.pending.iter().filter(|(_, waiting)| waiting.due <= now).map(|(id, _)| id.clone()).collect();
        for id in due {
            let Some(waiting) = self.pending.remove(&id) else { continue };
            let Some((_, reason)) = waiting.items.last().cloned() else { continue };
            self.pushed.entry(id.clone()).or_default().extend(waiting.items.into_iter().map(|(item, _)| item));
            let alert = Alert { session_id: id, title: waiting.title, reason, origin: None, clear: false };
            self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), alert));
        }
    }

    /// How many pushes have been started and not yet waited for.
    #[cfg(test)]
    pub(crate) fn pushes_started(&self) -> usize {
        self.pushes.len()
    }

    /// Wait for every push started so far, retries included. Only tests need it:
    /// `follow` never waits on a push.
    #[cfg(test)]
    pub(crate) async fn settle(&mut self) {
        while self.pushes.join_next().await.is_some() {}
    }
}

/// What the push says: the item's text, except where it is not a sentence.
fn body(item: &WaitingItem) -> String {
    let text = item.text.trim();
    if item.source == WaitingSource::Walk && (text.is_empty() || text.contains("://")) {
        "A walk is ready".to_string()
    } else {
        text.to_string()
    }
}

/// Where a push to an iPhone goes. iOS ends a subscription whose pushes show nothing, so a clear skips it.
const APPLE_PUSH: &str = "https://web.push.apple.com/";

/// Whether this message goes to this endpoint: everything does, except a clear to an iPhone.
fn is_sent(alert: &Alert, endpoint: &str) -> bool {
    !(alert.clear && endpoint.starts_with(APPLE_PUSH))
}

/// One Session's push to every subscription, each on its own task so a phone
/// that is failing does not hold up another.
async fn push(gateway: Gateway, retries: Vec<Duration>, mut alert: Alert) {
    let pairing = gateway.pairing.clone();
    alert.origin = tokio::task::spawn_blocking(move || pairing.address().ok()).await.ok().flatten();
    let Ok(payload) = ipc::encode(&alert) else {
        return;
    };
    let subs = gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap_or_default();
    let mut each = tokio::task::JoinSet::new();
    for sub in subs {
        if !gateway.push.plain && !sub.endpoint.starts_with("https://") {
            continue;
        }
        if !is_sent(&alert, &sub.endpoint) {
            continue;
        }
        each.spawn(send(gateway.clone(), retries.clone(), sub, payload.clone()));
    }
    while each.join_next().await.is_some() {}
}

async fn send(gateway: Gateway, retries: Vec<Duration>, sub: store::PushSubscription, payload: String) {
    let push = &gateway.push;
    // The Gateway's own address says who is sending; with none
    // there is no phone to reach, so the fallback is rarely used.
    let subject = gateway.pairing.address().unwrap_or_else(|_| push.subject.clone());
    // A subscription made with another Mac's key is pushed to under that key,
    // or the push service refuses it. One whose key will not read cannot be sent.
    let carried = match sub.vapid_private.as_ref().map(|key| Vapid::from_private_key(key.expose())) {
        Some(Ok(vapid)) => Some(vapid),
        Some(Err(_)) => return,
        None => None,
    };
    let vapid = carried.as_ref().unwrap_or(&push.vapid);
    let mut waits = retries.iter();
    loop {
        let now = (gateway.pairing.clock)();
        match deliver(&push.client, vapid, &subject, &sub, payload.as_bytes(), now).await {
            Delivery::Sent => break,
            Delivery::Gone => {
                let _ = gateway.pairing.store.lock().unwrap().remove_push_subscription(&sub.device_id, &sub.endpoint);
                break;
            }
            Delivery::Failed => match waits.next() {
                Some(wait) => tokio::time::sleep(*wait).await,
                None => break,
            },
        }
    }
}

/// Follow Fleet for as long as the Gateway runs, finding it again after it restarts.
pub async fn follow(gateway: Gateway) {
    let mut watch = Watch::new(gateway.clone());
    loop {
        if let Ok(port) = (gateway.fleet)() {
            if let Ok(io) = fleet_client::upgrade(port, "/events").await {
                // Messages are read on a task of their own: a read cut off by a timer would lose its half-read frame.
                let (sender, mut received) = tokio::sync::mpsc::channel::<String>(64);
                let mut messages = Messages::new(io);
                let reader = tokio::spawn(async move {
                    while let Some(text) = messages.next().await {
                        if sender.send(text).await.is_err() {
                            break;
                        }
                    }
                });
                loop {
                    let due = async {
                        match watch.next_due() {
                            Some(at) => tokio::time::sleep_until(at).await,
                            None => std::future::pending().await,
                        }
                    };
                    tokio::select! {
                        text = received.recv() => match text {
                            Some(text) => watch.observe(&text).await,
                            None => break,
                        },
                        () = due => watch.flush_due(),
                    }
                }
                reader.abort();
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(clear: bool) -> Alert {
        Alert { session_id: "s".into(), title: String::new(), reason: String::new(), origin: None, clear }
    }

    #[test]
    fn a_clear_is_not_sent_to_an_iphone_and_a_push_is() {
        let apple = "https://web.push.apple.com/abc";
        let other = "https://fcm.example.test/abc";
        assert!(!is_sent(&alert(true), apple));
        assert!(is_sent(&alert(false), apple));
        assert!(is_sent(&alert(true), other));
    }
}
