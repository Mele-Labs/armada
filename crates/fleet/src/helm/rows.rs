//! What a conversation's session says, as rows on its thread. `#939`, `#2166`.
//!
//! **One reading of a session's stream for a Job's Commander**: what it wrote is told as rows, and
//! what it injected into its own input is dropped. Helm's thread is a Session's and is written by
//! `crate::session_host::hearing`, so Helm no longer reads a stream here.

use adapter_traits::{DroneEvent, Speaker};
use core_model::StepId;
use ipc::Shown;

/// Whether prose arrived on the session's input channel, and so is never the
/// session's reply. A skill the session calls is injected as a user turn
/// carrying its whole body, and it reads as text exactly as the brief coming
/// back does; the thread draws every `said` row as the session's own, so it is
/// dropped where `Speaker` is still known.
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
