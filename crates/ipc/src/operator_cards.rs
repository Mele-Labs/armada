//! The Operator's cards and its per-project values: what only the Operator's own session raises,
//! and what it reads and sets across every project. `crates/ipc/operations/raise_card.toml`.
//!
//! **A card is one decision for the person**, two to four options with the first the Operator's pick and a
//! cost on each. It lives in the page's Your turn as an `OperatorTurnItem` carrying an [`OperatorCard`].

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

/// What a card is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorCardKind {
    /// A decision the Operator brings with its read of it.
    Deal,
    /// A thing the Operator raises for him to look at and decide.
    Raise,
}

/// What a card carries beyond the item every row of Your turn is. Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCard {
    pub kind: OperatorCardKind,
    /// The card this one carries on from: the person talked about that card and the Operator raised this after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk_of: Option<String>,
    /// The Jobs it is about, each live or ended as Fleet kept them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<SessionId>,
    /// The option he pressed and when, on a card in the page's `working` and on one that came back. Absent
    /// before his press. Since 14 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<OperatorCardPicked>,
    /// What the Operator reported doing on it since his press, oldest first. Since 14 Nov 2026.
    #[serde(default)]
    pub steps: Vec<OperatorCardStep>,
    /// Why a picked card came back to the front, and when. Absent until it does, and cleared by his next
    /// press. Since 14 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned: Option<OperatorCardReturned>,
}

/// His press on a card. Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCardPicked {
    /// The option's label.
    pub option: String,
    pub at: Instant,
}

/// One thing the Operator did on a picked card, as it reported it. Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCardStep {
    pub at: Instant,
    pub said: String,
}

/// Why a picked card came back to the front: the Operator reported it failed, or nothing settled it in time.
/// Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCardReturned {
    pub why: String,
    pub at: Instant,
}

/// One option of a card, and what taking it spends or risks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardOption {
    pub label: String,
    pub cost: String,
}

/// `raise_card`: one decision for the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaiseCard {
    pub kind: OperatorCardKind,
    pub title: String,
    /// The facts he needs to decide.
    pub read: String,
    /// Two to four, the Operator's pick first.
    pub options: Vec<CardOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<SessionId>,
    /// The repository's name, where the card is about one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// A card id this one carries on from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk_of: Option<String>,
}

/// What `raise_card` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardRaised {
    /// `turn:<n>`, the id every act on the page names.
    pub card: String,
}

/// `revise_card`: change what an open card says. The title and the Jobs it is about stay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviseCard {
    pub card: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<CardOption>>,
}

/// What `revise_card` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardRevised {
    pub card: String,
}

/// `settle_card`: the card is done, however it came to be, and `said` is how.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleCard {
    pub card: String,
    pub said: String,
}

/// `report_card`: what the Operator did on a picked card, or why it cannot be done. Exactly one of `step` and
/// `failed`. Answered with [`CardRevised`]. Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportCard {
    pub card: String,
    /// One real step, such as a Job started, a pull request opened or a merge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Why the pick cannot be carried out. The card comes back to the front.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
}

/// What `settle_card` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCardSettled {
    pub card: String,
}

/// What the Operator may do in a project without being asked. Unset is a real state: it is `None`
/// on the wire, and the Operator asks once and saves his answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorActs {
    /// Act on what it sees, then tell him.
    ActThenTell,
    /// Do what it can, but ask before it starts a Job.
    AskBeforeStarting,
    /// Do nothing until he says.
    OnlyWhenTold,
}

impl OperatorActs {
    pub fn as_wire(self) -> &'static str {
        match self {
            OperatorActs::ActThenTell => "act_then_tell",
            OperatorActs::AskBeforeStarting => "ask_before_starting",
            OperatorActs::OnlyWhenTold => "only_when_told",
        }
    }

    /// The value a kept word names, `None` for any other word.
    pub fn from_wire(word: &str) -> Option<OperatorActs> {
        [OperatorActs::ActThenTell, OperatorActs::AskBeforeStarting, OperatorActs::OnlyWhenTold].into_iter().find(|one| one.as_wire() == word)
    }
}

/// One project's value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOperatorActs {
    pub project: String,
    pub name: String,
    /// Absent where he has not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acts: Option<OperatorActs>,
}

/// `list_operator_acts`: every project's value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorActsList {
    pub projects: Vec<ProjectOperatorActs>,
}

/// `set_operator_acts`: one project's value, or unset with none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetOperatorActs {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acts: Option<OperatorActs>,
}
