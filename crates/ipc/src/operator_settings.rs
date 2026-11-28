//! What each project's Operator is set to: its switch, model, role and words, and the tools it may use.
//! `crates/ipc/operations/list_operator_settings.toml`.
//!
//! **A project's Operator is one session**, so each value is the project's and never the machine's. The machine's
//! `features.operator` stays the master over every project's own `on`.

use serde::{Deserialize, Serialize};

use crate::operator_cards::OperatorActs;
use crate::SessionId;

/// What a project's Operator is for. Each preset is a block of rules and a default for each tool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRole {
    /// Works beside him: answers, starts Jobs and lands on his word.
    #[default]
    Partner,
    /// Turns arrivals and his words into Jobs and keeps them moving. Lands nothing.
    Dispatcher,
    /// Reads what waits on him and gives its read and pick. Starts, ends and lands nothing.
    Reviewer,
}

impl OperatorRole {
    pub const ALL: [OperatorRole; 3] = [OperatorRole::Partner, OperatorRole::Dispatcher, OperatorRole::Reviewer];

    pub fn as_wire(self) -> &'static str {
        match self {
            OperatorRole::Partner => "partner",
            OperatorRole::Dispatcher => "dispatcher",
            OperatorRole::Reviewer => "reviewer",
        }
    }

    /// The role a kept word names, `None` for any other word.
    pub fn from_wire(word: &str) -> Option<OperatorRole> {
        Self::ALL.into_iter().find(|one| one.as_wire() == word)
    }

    /// The tools this role has on until he says otherwise.
    pub fn tools(self) -> OperatorTools {
        match self {
            OperatorRole::Partner => OperatorTools { start_jobs: true, merge: true, end_jobs: true, file_issues: true },
            OperatorRole::Dispatcher => OperatorTools { start_jobs: true, merge: false, end_jobs: true, file_issues: true },
            OperatorRole::Reviewer => OperatorTools { start_jobs: false, merge: false, end_jobs: false, file_issues: false },
        }
    }
}

/// Where an Operator asks its decisions: as cards in front of him, or as questions in the talk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorDecisions {
    #[default]
    Cards,
    Chat,
}

impl OperatorDecisions {
    pub const ALL: [OperatorDecisions; 2] = [OperatorDecisions::Cards, OperatorDecisions::Chat];

    pub fn as_wire(self) -> &'static str {
        match self {
            OperatorDecisions::Cards => "cards",
            OperatorDecisions::Chat => "chat",
        }
    }

    pub fn from_wire(word: &str) -> Option<OperatorDecisions> {
        Self::ALL.into_iter().find(|one| one.as_wire() == word)
    }
}

/// The four things an Operator may be switched off from: starting Jobs, merging pull requests, ending Jobs and filing issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTools {
    pub start_jobs: bool,
    pub merge: bool,
    pub end_jobs: bool,
    pub file_issues: bool,
}

/// The tools he set, each absent where he has not: that tool follows the role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorToolsSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_jobs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_jobs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_issues: Option<bool>,
}

impl OperatorToolsSet {
    /// `role`'s tools, each overridden where he set it.
    pub fn over(&self, role: OperatorRole) -> OperatorTools {
        let preset = role.tools();
        OperatorTools {
            start_jobs: self.start_jobs.unwrap_or(preset.start_jobs),
            merge: self.merge.unwrap_or(preset.merge),
            end_jobs: self.end_jobs.unwrap_or(preset.end_jobs),
            file_issues: self.file_issues.unwrap_or(preset.file_issues),
        }
    }
}

/// One project's Operator, every value resolved: his own where he set it, the default otherwise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSettings {
    /// Whether this project has an Operator. It runs only while the machine's switch is on too.
    pub on: bool,
    pub model: String,
    pub effort: String,
    pub role: OperatorRole,
    /// His words about what this Operator is for, put before the role's rules. Empty where he has none.
    pub words: String,
    /// His own addition to the rules, put after them. Empty where he has none.
    pub prompt: String,
    /// The output style the session runs in.
    pub output_style: String,
    pub tools: OperatorTools,
    pub decisions: OperatorDecisions,
    /// What the Operator is called where a name is drawn.
    pub name: String,
    /// A `#rrggbb` colour, absent where Bridge draws its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// What it may do unasked, absent where he has not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acts: Option<OperatorActs>,
}

/// What he set for one project, each value absent where he has not.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSettingsSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<OperatorRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<OperatorToolsSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decisions: Option<OperatorDecisions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acts: Option<OperatorActs>,
}

/// One project's Operator: what it is set to, what he set, and the session that runs it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOperatorSettings {
    pub project: String,
    pub project_name: String,
    pub settings: OperatorSettings,
    pub set: OperatorSettingsSet,
    /// The live session, absent while none runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `list_operator_settings`: every project's Operator, and the output styles he may pick.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSettingsList {
    pub projects: Vec<ProjectOperatorSettings>,
    pub styles: Vec<OperatorStyle>,
    /// What each role puts at the top of the prompt, in `OperatorRole::ALL`'s order, so Settings can show it. Since 28 Nov 2026.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<OperatorRoleWords>,
}

/// An output style he may pick, with its file's own words for it, so a picker can say what each changes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorStyle {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// One short reply written in the style, from its file's `sample`, so Settings shows the voice. Since 28 Nov 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
}

/// A role and the words its preset puts at the top of the Operator's prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRoleWords {
    pub role: OperatorRole,
    pub words: String,
}

/// `set_operator_settings`: what he sets for one project, and the values he gives back to their defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetOperatorSettings {
    pub project: String,
    #[serde(default)]
    pub set: OperatorSettingsSet,
    /// Field names to return to their default: `on`, `model`, `effort`, `role`, `words`, `prompt`, `output_style`,
    /// `decisions`, `name`, `accent`, `acts`, `tools` for all four tools, or `tools.start_jobs`, `tools.merge`,
    /// `tools.end_jobs`, `tools.file_issues`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unset: Vec<String>,
}

/// Whose Operator a card, a To review item or a feed line is: the project's, with the name and accent it draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorTag {
    pub project: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}
