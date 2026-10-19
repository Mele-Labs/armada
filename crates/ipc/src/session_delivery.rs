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
