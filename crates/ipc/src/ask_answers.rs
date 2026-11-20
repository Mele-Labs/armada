//! His answer to one ask, kept apart from what the Job is told. **The thread card of an ask draws this**: the
//! question, how he answered, his own words and what to open, with none of the instruction the Job reads.
//! An answered item leaves `waiting_for` at once; its answer stays here.
//! `.claude/decisions/2026-11-19-one-ask-one-item-one-place.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// How he answered an ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerHow {
    /// He picked one of its choices or takes, `pick` naming it.
    Picked,
    /// He approved it.
    Approved,
    /// He asked for changes, in `words`.
    AskedForChanges,
    /// He put it away with no verdict; `words` is his reason, where he gave one.
    Dismissed,
    /// He answered in his own words alone.
    Answered,
}

impl AnswerHow {
    /// The wire's spelling, which the ledger keeps too.
    pub fn as_str(self) -> &'static str {
        match self {
            AnswerHow::Picked => "picked",
            AnswerHow::Approved => "approved",
            AnswerHow::AskedForChanges => "asked_for_changes",
            AnswerHow::Dismissed => "dismissed",
            AnswerHow::Answered => "answered",
        }
    }

    /// The spelling read back, `None` for one this build does not know.
    pub fn of(said: &str) -> Option<AnswerHow> {
        [AnswerHow::Picked, AnswerHow::Approved, AnswerHow::AskedForChanges, AnswerHow::Dismissed, AnswerHow::Answered]
            .into_iter()
            .find(|how| how.as_str() == said)
    }
}

/// One answer of his to one ask, as the Job's thread card shows it. Further fields ride here as they come,
/// each defaulted, so an older Bridge reads a newer answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnsweredAsk {
    /// The `waiting_for` item it answered: `sketch:<title>`, `draft:<row>`, `plan:<version>`, `ask:<call>` or the
    /// agent's own id.
    pub item: String,
    /// The ask in plain words, as the item was titled when he answered.
    pub question: String,
    pub how: AnswerHow,
    /// What he typed, as he typed it. Absent where he typed nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The choice or take he picked, by its label. Set where `how` is `picked`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<String>,
    /// The artifact it was asked on, by its `JobArtifact` id, so the card's Open goes there. Absent for an ask
    /// with no artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    /// The part of the artifact it answered, where the ask was a part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// His notes on a part, keyed by what each is on, as he left them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub notes: BTreeMap<String, String>,
    pub at: Instant,
}
