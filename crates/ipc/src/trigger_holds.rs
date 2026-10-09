//! A Trigger that holds its Job, and what the owner does about it.
//! `docs/concepts/trigger.md`, *A failed Trigger with `block` on*.
//! **Since 23.68.**
//!
//! A hold is a firing in `held` (or `repairing`, `rerunning` and `fix_ready`
//! where `repair` is also on) whose Trigger blocks. The firing's row says so on
//! its own; what this adds is the two acts and the alert a Board row carries.

use serde::{Deserialize, Serialize};

use crate::ids::StepId;
use crate::triggers::{TriggerFiringState, TriggerMoment};

/// Why a Job's row carries the bell. **One reason, the most pressing**: a hold
/// before a destructive Command asking to run before a fix waiting on a choice
/// before a failure nobody can repair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobAlertKind {
    /// A Trigger that blocks failed, and the Job waits on the owner.
    Held,
    /// A Trigger on a destructive Command waits on the owner's `rerun_trigger`
    /// (run it once) or `skip_trigger`. With `block` on it holds the Job too.
    Asks,
    /// A repair's fix passes and waits on the owner's `choose_trigger_fix`.
    FixReady,
    /// A Trigger failed after its repair tries, and nothing waits on it.
    Failed,
}

/// The Trigger a Board row's bell is about. Present on `JobSummary.alert`.
/// **The row draws a mark and a tooltip from this** and composes no sentence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobAlert {
    pub kind: JobAlertKind,
    /// The Trigger's name, or the added step's command, skill or brief.
    pub trigger: String,
    pub when: TriggerMoment,
    /// The step it fired at. For `pr_opened`, the delivering one.
    pub step: StepId,
}

/// The body of `rerun_trigger` and `skip_trigger`: which hold. **Exactly one**
/// of the two fields is set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldAct {
    /// The Trigger's name. The latest firing of it that holds the Job is the one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    /// An added step's id, where it is an added step that holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addition: Option<String>,
}

/// What `rerun_trigger` and `skip_trigger` came to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldSettled {
    /// Where the firing stands: `passed` for a rerun that passed, `skipped` for
    /// a skip, `held` for a rerun that failed again. A Run on a destructive
    /// Command that asks ends as any firing does: `passed`, `held` where it
    /// blocks, `repairing` where it repairs, otherwise `failed`.
    pub state: TriggerFiringState,
    /// Whether the Job holds nothing now. False where a rerun failed again, and
    /// where another Trigger still holds it.
    pub released: bool,
}
