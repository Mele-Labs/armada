//! Whether an agent's waiting item repeats a question the person has not answered, which Bridge
//! keeps in Needs you as the question that lapsed (`lapsed_question`, `handoff_answers.rs`).
//!
//! **A question nobody answered in time stays in his Needs you, kept whole**, and his answer comes
//! to the agent as a message. An agent told to say what it waited on lists it again, so
//! `waiting_for` refuses the repeat and a read leaves off one kept before this rule.

use std::collections::BTreeSet;

use ipc::AskedQuestion;

/// Whether an item repeats one of `questions`: the same set of at least two option labels, or the
/// question's own words as its text.
pub(crate) fn repeats_lapsed(text: &str, labels: &[&str], questions: &[AskedQuestion]) -> bool {
    let labels: BTreeSet<&str> = labels.iter().map(|label| label.trim()).collect();
    questions.iter().any(|asked| {
        let offered: BTreeSet<&str> = asked.options.iter().map(|option| option.label.trim()).collect();
        text.trim() == asked.question.trim() || (labels.len() >= 2 && labels == offered)
    })
}

/// What the agent reads when its list is refused for repeating a question that lapsed.
pub(crate) fn lapsed_item_refusal(id: &str) -> String {
    format!(
        "Refused: item \"{id}\" repeats the question you asked with AskUserQuestion that nobody \
         answered in time. It is already in his Needs you, kept from that call, and his answer comes \
         to you as a message. Do not list it in waiting_for. Call waiting_for again without the item."
    )
}
