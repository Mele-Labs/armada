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
const BLOCKED: [(&str, &str); 4] = [
    ("stalled", "stalled"),
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
}

impl Watch {
    pub fn new(gateway: Gateway) -> Watch {
        Watch { gateway, sent: HashMap::new() }
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
                        self.sent.insert(id.clone(), reason);
                        self.push(&id, reason).await;
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

    async fn alert(&self, id: &str, reason: &'static str) -> Alert {
        let read = async {
            let port = (self.gateway.fleet)().ok()?;
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

    async fn push(&self, id: &str, reason: &'static str) {
        let Ok(payload) = ipc::encode(&self.alert(id, reason).await) else {
            return;
        };
        let subs = self.gateway.pairing.store.lock().unwrap().push_subscriptions().unwrap_or_default();
        let push = &self.gateway.push;
        for sub in subs {
            if !push.plain && !sub.endpoint.starts_with("https://") {
                continue;
            }
            let now = (self.gateway.pairing.clock)();
            match deliver(&push.client, &push.vapid, &push.subject, &sub, payload.as_bytes(), now).await {
                Delivery::Sent => {}
                Delivery::Gone => {
                    let _ = self
                        .gateway
                        .pairing
                        .store
                        .lock()
                        .unwrap()
                        .remove_push_subscription(&sub.device_id, &sub.endpoint);
                }
                // The Gateway has no log to write to yet; the condition stays marked sent.
                Delivery::Failed(_) => {}
            }
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
