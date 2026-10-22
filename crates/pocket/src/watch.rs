//! What pushes: Fleet's `/events`, narrowed to Sessions that went quiet.
//!
//! **Only Stuck.** "Escalations interrupt, approvals queue": a Session question
//! or a plan to approve is a decision waiting, and neither is here, so no event
//! of theirs can reach a phone as a push.

use std::collections::HashSet;
use std::time::Duration;

use ipc::{Event, SessionState, SessionStanding, StreamMessage};
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
    reason: &'static str,
    /// This Mac's `https://<tailnet name>`, the address the phone paired with.
    /// Absent while Tailscale is down; the app then finds the Mac holding the Job.
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
}

pub struct Watch {
    gateway: Gateway,
    /// The Sessions already pushed for. A Session leaves when it moves on, so
    /// the next time it goes quiet it pushes again.
    sent: HashSet<String>,
    /// How long to wait before each retry of a push that failed.
    pub(crate) retries: Vec<Duration>,
    pushes: tokio::task::JoinSet<()>,
}

impl Watch {
    pub fn new(gateway: Gateway) -> Watch {
        Watch {
            gateway,
            sent: HashSet::new(),
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
            let quiet = record.state == SessionState::Live && record.standing == Some(SessionStanding::Stuck);
            if !quiet {
                self.sent.remove(&id);
            } else if self.sent.insert(id.clone()) {
                // Marked in flight now, so a repeat of this event sends nothing
                // while the push retries on its own task.
                let title = record.title.unwrap_or_else(|| id.clone());
                let alert = Alert { session_id: id, title, reason: "stalled", origin: None };
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
    // The Gateway's own tailnet address says who is sending; with no
    // Tailscale there is no phone to reach, so the fallback is rarely used.
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
