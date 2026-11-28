//! What an Operator's system prompt is made of, and in what order: his words, its role's rules, the Operator's rules
//! and his own addition. `docs/contracts/agent-prompt/`.

use ipc::OperatorRole;

use crate::hosted_session::{operator_rules, OPERATOR_STYLE};

const COPILOT: &str = include_str!("operator_role_copilot.md");
const DISPATCHER: &str = include_str!("operator_role_dispatcher.md");
const REVIEWER: &str = include_str!("operator_role_reviewer.md");

/// The rules block of `role`.
pub fn role_rules(role: OperatorRole) -> &'static str {
    match role {
        OperatorRole::Copilot => COPILOT,
        OperatorRole::Dispatcher => DISPATCHER,
        OperatorRole::Reviewer => REVIEWER,
    }
}

/// What one project's Operator is started with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorPrompt {
    /// His words about what it is for, first in the prompt. Empty where he has none.
    pub words: String,
    pub role: OperatorRole,
    /// His own addition, last in the prompt, after the rules. Empty where he has none.
    pub extra: String,
    /// The output style it runs in.
    pub style: String,
}

impl Default for OperatorPrompt {
    fn default() -> OperatorPrompt {
        OperatorPrompt { words: String::new(), role: OperatorRole::Copilot, extra: String::new(), style: OPERATOR_STYLE.to_string() }
    }
}

impl OperatorPrompt {
    /// The system prompt, in order: his words, the role's block, the Operator's rules, his addition. A part that is empty is left out.
    pub fn told(&self) -> String {
        let rules = operator_rules();
        [self.words.trim(), role_rules(self.role).trim(), rules.trim(), self.extra.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
