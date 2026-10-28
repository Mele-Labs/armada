//! How the owner wants a Job's work to leave its branch, asked once when a bug or feature Job starts.
//! `docs/concepts/session/the-delivery-question.md`.
//!
//! **The answer is kept in the Job's thread** as one `delivery` row, replaced by id when the owner
//! answers. A first message that already names a delivery writes the row answered and asks nothing.

use serde::{Deserialize, Serialize};

/// What happens to the work once it is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryChoice {
    /// Open a pull request, watch `ci`, merge it.
    AutoLand,
    /// Stop at the open pull request.
    OpenPr,
    /// Stop at the commit on the Job's branch. Nothing is pushed.
    KeepLocal,
}

impl DeliveryChoice {
    /// The choices in the order the question offers them: `answer_waiting` takes the index.
    pub const OFFERED: [DeliveryChoice; 3] = [DeliveryChoice::AutoLand, DeliveryChoice::OpenPr, DeliveryChoice::KeepLocal];

    pub fn label(self) -> &'static str {
        match self {
            DeliveryChoice::AutoLand => "Auto land",
            DeliveryChoice::OpenPr => "Open PR only",
            DeliveryChoice::KeepLocal => "Keep local",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            DeliveryChoice::AutoLand => "Open a pull request, watch ci, merge it.",
            DeliveryChoice::OpenPr => "Stop at the open pull request.",
            DeliveryChoice::KeepLocal => "Stop at the commit on the job's branch. Nothing is pushed.",
        }
    }
}

/// Where the landing a start card offers came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryPickedFrom {
    /// The one the person most often picks for this kind of work in this repository.
    Usual,
    /// The first message already says how the work lands.
    Named,
    /// Neither: Fleet's own pick, Auto land, so the card is never blank. Bridge marks only the other two.
    Default,
}

/// A landing the start card has ready, and where it came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryPicked {
    pub choice: DeliveryChoice,
    pub from: DeliveryPickedFrom,
}

/// A Job that waits on its start card: the person settles it and presses Start (`begin_session`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartHeld {
    /// The Job's workflow has a step that asks how the work lands, so `begin_session` needs a
    /// `delivery`. False for a custom Job and for a workflow with no such step.
    pub asks_delivery: bool,
    /// Present wherever `asks_delivery` holds: named, else usual, else `default`. Absent elsewhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<DeliveryPicked>,
    /// The first message the agent will read, as it will read it: a pinned note's with Fleet's lines
    /// after his words. Absent until one is kept for the Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Start was pressed while the proposer was still reading: the Job starts once the reading
    /// ends, with the landing the reading picks where the press named none. Left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub queued: bool,
}
