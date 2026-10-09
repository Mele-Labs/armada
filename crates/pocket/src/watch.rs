//! What pushes: Fleet's `/events`, narrowed to Jobs that stopped.
//!
//! **Only Blocked.** "Escalations interrupt, approvals queue": an approval, a
//! review or a Session question is a decision waiting, and none of them is
//! here, so no event of theirs can reach a phone as a push.

use std::collections::HashMap;
use std::time::Duration;

use ipc::{Event, JobDetail, StreamMessage};
use serde::Serialize;

use crate::fleet_events::Messages;
use crate::phone::PhoneJob;
use crate::webpush::{deliver, Delivery};
use crate::{fleet_client, Gateway};

/// The escalation reasons that mean work stopped, each with the words
/// `enum-verbs.toml` gives it. Nothing else pushes.
const BLOCKED: [(&str, &str); 6] = [
    ("stalled", "stalled"),
    // Rendered "stalled" in the registry too, so a Job the owner sees as stalled buzzes.
    ("silent", "stalled"),
    ("hatch_unbidden", "stalled"),
    ("thrashing", "churning"),
    ("fan_out", "hit the sub-dispatch cap"),
    ("interrupted", "interrupted"),
];

fn verb(reason: &str) -> Option<&'static str> {
    BLOCKED.iter().find(|(name, _)| *name == reason).map(|(_, verb)| *verb)
}

#[derive(Serialize)]
struct At {
    at: usize,
    of: usize,
}

/// The whole payload. The phone's service worker opens `/jobs/<job_id>`.
#[derive(Serialize)]
struct Alert {
    job_id: String,
    title: String,
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    step: Option<At>,
}

pub struct Watch {
    gateway: Gateway,
    /// The reason each Job is held for and has already been pushed for. A Job
    /// leaves when it moves on, so the next time it stops it pushes again.
    sent: HashMap<String, &'static str>,
    /// How long to wait before each retry of a push that failed.
    pub(crate) retries: Vec<Duration>,
    pushes: tokio::task::JoinSet<()>,
}

impl Watch {
    pub fn new(gateway: Gateway) -> Watch {
        Watch {
            gateway,
            sent: HashMap::new(),
            pushes: tokio::task::JoinSet::new(),
            retries: vec![Duration::from_secs(5), Duration::from_secs(30), Duration::from_secs(120)],
        }
    }

    /// One text message from Fleet's `/events`.
    pub async fn observe(&mut self, text: &str) {
        let Ok(StreamMessage::Event(delivered)) = ipc::decode::<StreamMessage>("a stream message", text.as_bytes()) else {
            return;
        };
        match delivered.event {
            Event::JobStateChanged(change) => {
                let id = change.job_id.as_str().to_string();
                let held = (change.to.domain() == core_model::JobStatus::Escalated)
                    .then(|| change.reason.and_then(|reason| reason.named))
                    .flatten()
                    .and_then(|name| verb(&name));
                match held {
                    Some(reason) if self.sent.get(&id) != Some(&reason) => {
                        // Marked in flight now, so a repeat of this event sends nothing
                        // while the push retries on its own task.
                        self.sent.insert(id.clone(), reason);
                        self.pushes.spawn(push(self.gateway.clone(), self.retries.clone(), id, reason));
                    }
                    Some(_) => {}
                    None => {
                        self.sent.remove(&id);
                    }
                }
            }
            Event::JobLanded(event) => {
                self.sent.remove(event.job.id.as_str());
            }
            Event::JobForgotten(event) => {
                self.sent.remove(event.job_id.as_str());
            }
            _ => {}
        }
    }

    /// Wait for every push started so far, retries included. Only tests need it:
    /// `follow` never waits on a push.
    #[cfg(test)]
    pub(crate) async fn settle(&mut self) {
        while self.pushes.join_next().await.is_some() {}
    }
}

async fn alert(gateway: &Gateway, id: &str, reason: &'static str) -> Alert {
    let read = async {
        let port = (gateway.fleet)().ok()?;
        let answer = fleet_client::send(port, "GET", &format!("/jobs/{id}"), None).await.ok()?;
        ipc::decode::<JobDetail>("a job", &answer.body).ok()
    };
    let phone = read.await.map(|detail| PhoneJob::of_detail(&detail, None));
    Alert {
        job_id: id.to_string(),
        title: phone.as_ref().map_or_else(|| id.to_string(), |job| job.title.clone()),
        reason,
        step: phone.and_then(|job| job.step).map(|step| At { at: step.at, of: step.of }),
    }
}

/// One Job's push to every subscription, each on its own task so a phone that
/// is failing does not hold up another.
async fn push(gateway: Gateway, retries: Vec<Duration>, id: String, reason: &'static str) {
    let Ok(payload) = ipc::encode(&alert(&gateway, &id, reason).await) else {
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
    let mut waits = retries.iter();
    loop {
        let now = (gateway.pairing.clock)();
        match deliver(&push.client, &push.vapid, &subject, &sub, payload.as_bytes(), now).await {
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
