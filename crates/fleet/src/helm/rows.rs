//! What a conversation's session says, as rows on its thread. `#939`, `#2166`.
//!
//! **One reading of a session's stream for every conversation**: Helm's thread
//! and a Job's Commander's are told the same of what a session wrote, and what
//! it injected into its own input is dropped from both.

use std::sync::Arc;

use adapter_traits::{DroneEvent, Speaker};
use core_model::StepId;
use ipc::{HelmChangedCheckout, HelmMessage, Instant, ManifestId, Shown};

use super::conversation::Conversation;
use super::hosting::Heard;
use crate::clock::Clock;

/// What a session says, as rows in its thread — and, where it wrote a file in
/// the checkout, as an event of Helm's own.
pub(super) struct Rows {
    pub(super) conversation: Arc<Conversation>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) manifest_id: ManifestId,
    pub(super) root: String,
    pub(super) events: api::Broadcaster,
}

impl Rows {
    /// `helm.changed_checkout`, where this call was one of the built-ins that
    /// edits a file. **A path and never the contents**: what was written is on
    /// the thread, and this stream is for a client that is not watching the
    /// dock. `docs/concepts/helm.md`, *Audit trail*.
    fn published_as_a_write(&self, event: &DroneEvent, at: &core_model::Timestamp) {
        let DroneEvent::Called { tool, detail, .. } = event else {
            return;
        };
        if !adapters::wrote_the_checkout(tool) {
            return;
        }
        let path = adapters::path_written(detail.whole().unwrap_or(detail.shown()));
        if path.is_empty() {
            return;
        }
        let inside = format!("{}/", self.root.trim_end_matches('/'));
        self.events
            .publish(ipc::Event::HelmChangedCheckout(HelmChangedCheckout {
                manifest_id: self.manifest_id.clone(),
                tool: tool.clone(),
                path: path.strip_prefix(&inside).unwrap_or(path).to_string(),
                at: Instant::from(at),
            }));
    }
}

impl Heard for Rows {
    fn commands(&self, names: Vec<String>) {
        super::facts::commands_changed(&self.conversation, names);
    }

    fn heard(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        for event in events {
            if injected(event) {
                continue;
            }
            self.published_as_a_write(event, &at);
            if let Some(shown) = shown(&at, event) {
                self.conversation.thread.say(HelmMessage::Row(shown));
            }
        }
    }
}

/// Whether prose arrived on the session's input channel, and so is never the
/// session's reply. A skill the session calls is injected as a user turn
/// carrying its whole body, and it reads as text exactly as the brief coming
/// back does; the thread draws every `said` row as the session's own, so it is
/// dropped where `Speaker` is still known. **One reading for every
/// conversation.**
pub(super) fn injected(event: &DroneEvent) -> bool {
    matches!(
        event,
        DroneEvent::Said {
            by: Speaker::Armada,
            ..
        }
    )
}

/// What one decoded event is on a conversation's thread, or `None` where it is
/// not a row.
pub(super) fn shown(at: &core_model::Timestamp, event: &DroneEvent) -> Option<Shown> {
    // `seen` labels a row with a Drone's step, and a conversation has none, so
    // the label is taken off again.
    let mut row = crate::transcript::seen(at, &StepId::new(""), event);
    row.step = None;
    Shown::of(row)
}
