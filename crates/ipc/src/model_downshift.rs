//! The Session model stepping down while the weekly plan window runs out. `docs/concepts/machine.md`, *Quota*.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// Where the step down stands. **Absent from [`ModelDownshiftState`] is none**: nothing offered, or the owner
/// has settled this window. The phase is the tag, so a client matches on it and never on a field being there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum ModelDownshift {
    /// The weekly window is at or over the line and the Session model, `was`, is not yet changed. `to` is what
    /// accepting makes the Session model.
    Offered { percent: u32, was: String, to: String },
    /// Accepted: the Session model is `to` until `until`, the window's reset. `was` is what restoring puts back.
    Shifted { was: String, to: String, until: Instant },
    /// The window reset and the Session model is still `to`. Restoring puts `was` back; keeping leaves `to`.
    Reset { was: String, to: String },
}

/// `get_model_downshift`, what the four commands answer, and `model_downshift.changed` whole.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDownshiftState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downshift: Option<ModelDownshift>,
}
