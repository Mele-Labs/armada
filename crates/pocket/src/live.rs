//! `/api/live`: Fleet's event stream narrowed to Job changes.
//!
//! Every message Fleet sends is read as the type it is, and only a Job change
//! is written back out, as the fields of [`Change`] and nothing else.

use ipc::{Event, StreamMessage};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Change {
    /// `created`, `status`, `step`, `landed`, `forgotten`, or `resync` when the
    /// phone should read its lists again.
    pub change: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn of_summary(change: &'static str, job: &ipc::JobSummary) -> Change {
    Change {
        change,
        job_id: Some(job.id.as_str().to_string()),
        title: Some(job.title.clone()),
        status: Some(job.status.as_wire().to_string()),
        reason: job.reason.as_ref().and_then(|reason| reason.named.clone()),
    }
}

fn resync() -> Change {
    Change {
        change: "resync",
        job_id: None,
        title: None,
        status: None,
        reason: None,
    }
}

/// The Job change a Fleet message comes to, or `None` for anything else.
pub fn change(text: &str) -> Option<Change> {
    match ipc::decode::<StreamMessage>("a stream message", text.as_bytes()).ok()? {
        // A resync or a drop means the phone's lists may be behind.
        StreamMessage::Resync(_) | StreamMessage::Missed(_) => Some(resync()),
        StreamMessage::Event(delivered) => match delivered.event {
            Event::JobCreated(event) => Some(of_summary("created", &event.job)),
            Event::JobStepAdvanced(event) => Some(of_summary("step", &event.job)),
            Event::JobLanded(event) => Some(of_summary("landed", &event.job)),
            Event::JobStateChanged(event) => Some(Change {
                change: "status",
                job_id: Some(event.job_id.as_str().to_string()),
                title: None,
                status: Some(event.to.as_wire().to_string()),
                reason: event.reason.and_then(|reason| reason.named),
            }),
            Event::JobForgotten(event) => Some(Change {
                change: "forgotten",
                job_id: Some(event.job_id.as_str().to_string()),
                title: None,
                status: None,
                reason: None,
            }),
            _ => None,
        },
    }
}

/// One Server-Sent Event.
pub fn frame(change: &Change) -> Option<String> {
    Some(format!("data: {}\n\n", ipc::encode(change).ok()?))
}
