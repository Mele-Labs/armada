//! A Fleet notice a person turned off — `set_notice_mute` in `crates/ipc/operations/`.

use serde::{Deserialize, Serialize};

/// A notice Fleet writes to a Session. The word is what a thread row's `notice` carries and what
/// the store keeps a mute under. **Every notice carries one**, so a person can name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notice {
    /// "slot-N is N commits behind the base."
    BehindBase,
    /// "#N failed its checks at <commit>."
    PrChecksFailed,
    /// "#N conflicts with <base>."
    PrConflicts,
    /// "#N left the merge queue."
    PrDropped,
    /// "#N is unmergeable in the merge queue."
    PrUnmergeable,
    /// "#N merged into <base>", with what to do before `all_done`.
    PrMerged,
    /// "Auto-merge is on for #N."
    PrAutoMerge,
    /// "#N is in the merge queue."
    PrQueued,
    /// A pull request Fleet opened is this Job's to see merged.
    PrHanded,
    /// The branch a Session's slot is leased on merged, so the slot is free to reuse.
    SlotFree,
    /// What keeps a Session's slot after its branch merged.
    SlotKept,
    /// The build Fleet and Bridge run moved.
    BuildMoved,
    /// Two Jobs, or a Job and main, wrote the same lines.
    WriteClash,
    /// A slot the Session held was left with work in it.
    SlotStranded,
    /// Main went red and the Session has it to fix.
    MainRed,
    /// A step of the workflow was skipped.
    RailNudge,
    /// The instructions of the workflow step the Session entered.
    RailStep,
    /// A usage rule tripped: context size, idle time or the plan's limit.
    UsageTrip,
    /// The plan's limit ended and the Session is woken.
    LimitWake,
    /// The owner approved the plan.
    PlanApproved,
    /// Auto land is on and the forge would not turn on auto-merge.
    AutoLandRefused,
    /// Fleet asks for the Job's retro note.
    RetroNote,
    /// A reminder that a reply claimed something it had no call for.
    ReplyClaim,
    /// The context handoff: the brief asked of the old Job, and the carry-on of the new.
    ContextHandoff,
    /// A walk window could not show its page.
    WalkFailed,
    /// The owner moved the Session to another workflow.
    WorkflowChanged,
    /// How the work is delivered is being asked.
    DeliveryAsked,
    /// How the work is delivered was chosen.
    DeliveryChosen,
    /// A watch Fleet runs for the Job saw what it was watching for.
    WatchSaw,
    /// The owner taught Armada something the Session should follow from now on.
    Taught,
    /// The backlog gave an issue the Job claimed to someone else.
    IssueLost,
}

impl Notice {
    pub fn word(self) -> &'static str {
        match self {
            Notice::BehindBase => "behind_base",
            Notice::PrChecksFailed => "pr_checks_failed",
            Notice::PrConflicts => "pr_conflicts",
            Notice::PrDropped => "pr_dropped",
            Notice::PrUnmergeable => "pr_unmergeable",
            Notice::PrMerged => "pr_merged",
            Notice::PrAutoMerge => "pr_auto_merge",
            Notice::PrQueued => "pr_queued",
            Notice::PrHanded => "pr_handed",
            Notice::SlotFree => "slot_free",
            Notice::SlotKept => "slot_kept",
            Notice::BuildMoved => "build_moved",
            Notice::WriteClash => "write_clash",
            Notice::SlotStranded => "slot_stranded",
            Notice::MainRed => "main_red",
            Notice::RailNudge => "rail_nudge",
            Notice::RailStep => "rail_step",
            Notice::UsageTrip => "usage_trip",
            Notice::LimitWake => "limit_wake",
            Notice::PlanApproved => "plan_approved",
            Notice::AutoLandRefused => "auto_land_refused",
            Notice::RetroNote => "retro_note",
            Notice::ReplyClaim => "reply_claim",
            Notice::ContextHandoff => "context_handoff",
            Notice::WalkFailed => "walk_failed",
            Notice::WorkflowChanged => "workflow_changed",
            Notice::DeliveryAsked => "delivery_asked",
            Notice::DeliveryChosen => "delivery_chosen",
            Notice::WatchSaw => "watch_saw",
            Notice::Taught => "taught",
            Notice::IssueLost => "issue_lost",
        }
    }
}

/// What a mute covers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoticeScope {
    /// One Job, until a person lifts it.
    Job { session_id: String },
    /// Every Job in a repository, until a person lifts it.
    Repo { manifest_id: String },
    /// One Job, for an hour from the press. Lifting it lifts the Job's mute too.
    Hour { session_id: String },
}

/// `set_notice_mute`' body and answer: mute the notice, or lift the mute.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetNoticeMute {
    pub notice: Notice,
    pub scope: NoticeScope,
    pub muted: bool,
}
