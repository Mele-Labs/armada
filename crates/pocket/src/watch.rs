//! What pushes: Fleet's `/events`, narrowed to what a Job newly needs from the person.
//!
//! **Only Needs you.** A push goes out when a Job's `waiting_for` gains an item id it
//! did not have. A Stuck Job sends nothing: Fleet repairs it without the person.
//!
//! **A snoozed Job pushes nothing until its time.** What it raises meanwhile is held, and one
//! push goes out for it when `snoozed_until` clears, if any of it is still open.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use ipc::{Event, StreamMessage, WaitingItem, WaitingSource};
use serde::Serialize;

use crate::fleet_events::Messages;
use crate::vapid::Vapid;
use crate::webpush::{deliver, Delivery};
use crate::{fleet_client, Gateway};

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
}

pub struct Watch {
    gateway: Gateway,
    /// Per Job, the item ids already pushed for or already open when Pocket first saw it.
    /// An id leaves when its item does, so an item raised again pushes again.
    sent: HashMap<String, HashSet<String>>,
    /// Per Job, the item ids it raised while snoozed, oldest first, pushed as one when it wakes.
    held: HashMap<String, Vec<String>>,
    /// How long to wait before each retry of a push that failed.
    pub(crate) retries: Vec<Duration>,
    pushes: tokio::task::JoinSet<()>,
}

impl Watch {
    pub fn new(gateway: Gateway) -> Watch {
        Watch {
            gateway,
            sent: HashMap::new(),
            held: HashMap::new(),
            pushes: tokio::task::JoinSet::new(),
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
            // Marked in flight now, so a repeat of this event sends nothing
            // while a push retries on its own task.
            let raised: Vec<&WaitingItem> = record.waiting_for.iter().filter(|item| known.insert(item.id.clone())).collect();
            if record.snoozed_until.is_some() {
                self.held.entry(id).or_default().extend(raised.iter().map(|item| item.id.clone()));
                return;
            }
            let mut held = self.held.remove(&id).unwrap_or_default();
            held.retain(|item| open.contains(item));
            if !held.is_empty() {
                // Back from a snooze: one push for all it raised, saying the newest.
                let newest = raised.last().copied().or_else(|| record.waiting_for.iter().rfind(|item| held.contains(&item.id)));
                if let Some(newest) = newest {
                    let alert = Alert { session_id: id.clone(), title, reason: body(newest), origin: None };
                    self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), alert));
                }
                return;
            }
            for item in raised {
                let alert = Alert { session_id: id.clone(), title: title.clone(), reason: body(item), origin: None };
                self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), alert));
            }
        }
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
                let mut messages = Messages::new(io);
                while let Some(text) = messages.next().await {
                    watch.observe(&text).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
