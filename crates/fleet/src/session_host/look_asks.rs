//! Whether an agent's waiting item asks the person to try, look at or approve this Job's own change. The
//! Ready to try card of the Job's preview already asks him that, and his Looks good there closes the plan's
//! `you_approved` gate, so the item only says it twice.
//!
//! **A look ask is an item about one of this Job's pull requests or offered branches**: an `approve_pr` act
//! on one of its pull requests, `approves: true` with words naming one, or words that open with *Try*,
//! *Look at* or *Test* and name one. A decision with options is never one. In a repository with a preview,
//! `waiting_for` refuses it as the agent writes it, so the agent offers the branch; one already kept comes
//! off Needs you once the Job's branch is on the preview list.

use std::collections::BTreeMap;

use ipc::WaitingAct;
use ipc::WaitingActKind;

use crate::session_host::merge_waits::names;

/// This Job's change as the person would try it.
#[derive(Default)]
pub(crate) struct OwnChange {
    /// Its pull requests, each with the forge's address for it, empty where the row has none.
    pub pulls: Vec<(u64, String)>,
    /// The branches it offered for the preview.
    pub branches: Vec<String>,
}

/// The pull request an attachment row is, if it is one.
pub(crate) fn pull_of(kind: &str, target: &str, detail: &BTreeMap<String, String>) -> Option<(u64, String)> {
    let number = (kind == crate::pull_requesting::PR).then(|| target.trim().parse().ok()).flatten()?;
    Some((number, detail.get("address").cloned().unwrap_or_default()))
}

/// What of the Job's change an item asks him to try, look at or approve, if it does: `#12` or a branch.
pub(crate) fn look_ask_of(approves: bool, options: usize, act: Option<&WaitingAct>, text: &str, change: &OwnChange) -> Option<String> {
    if !approves && options > 0 {
        return None;
    }
    let by_act = act
        .filter(|act| act.kind == WaitingActKind::ApprovePr)
        .and_then(|act| crate::pull_waits::number_of(&act.target))
        .filter(|number| change.pulls.iter().any(|(pull, _)| pull == number));
    if let Some(number) = by_act {
        return Some(format!("#{number}"));
    }
    let named = change
        .pulls
        .iter()
        .find(|(number, address)| names(text, *number) || (!address.is_empty() && text.contains(address.as_str())))
        .map(|(number, _)| format!("#{number}"))
        .or_else(|| change.branches.iter().find(|branch| names_branch(text, branch)).cloned())?;
    (approves || opens_with_a_look(text)).then_some(named)
}

/// Whether `text` opens by asking him to try, look at or test something.
fn opens_with_a_look(text: &str) -> bool {
    let lower = text.trim_start().to_lowercase();
    ["try ", "try:", "look at ", "test "].iter().any(|opening| lower.starts_with(opening))
}

/// Whether `text` names `branch` whole, not as part of a longer name.
fn names_branch(text: &str, branch: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || matches!(c, '-' | '_' | '/' | '.');
    !branch.is_empty()
        && text.match_indices(branch).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + branch.len()..].chars().next();
            !before.is_some_and(part) && !after.is_some_and(|c| part(c) && c != '.')
        })
}

/// What the agent reads when its list is refused for an item that asks him to try its own change.
pub(crate) fn look_item_refusal(id: &str, named: &str) -> String {
    format!(
        "Refused: item \"{id}\" asks him to try, look at or approve {named}, this Job's own change, and the Ready to try \
         card of your preview already asks him that. Call offer_preview with about, look and steps instead: his Looks good \
         on that card closes your plan's you_approved gate. Then call waiting_for again without the item."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change() -> OwnChange {
        OwnChange { pulls: vec![(3832, "https://forge.invalid/a/pull/3832".into())], branches: vec!["armada/session-1".into()] }
    }

    fn act(kind: WaitingActKind, target: &str) -> WaitingAct {
        WaitingAct { kind, target: target.to_string(), on: None }
    }

    #[test]
    fn an_approve_pr_act_on_the_jobs_pull_request_is_a_look_ask_and_on_another_is_not() {
        let change = change();
        assert_eq!(look_ask_of(false, 0, Some(&act(WaitingActKind::ApprovePr, "#3832")), "Approve it", &change).as_deref(), Some("#3832"));
        assert_eq!(look_ask_of(false, 0, Some(&act(WaitingActKind::ApprovePr, "#3834")), "Approve it", &change), None);
    }

    #[test]
    fn words_opening_with_a_look_or_marked_approves_and_naming_the_change_are_a_look_ask() {
        let change = change();
        for text in [
            "Try page notes on a Job's Browser tab with ⌥⌘A, then approve PR #3832",
            "Look at armada/session-1 in the preview",
            "test the new row on armada/session-1.",
        ] {
            assert!(look_ask_of(false, 0, None, text, &change).is_some(), "{text}");
        }
        assert_eq!(look_ask_of(true, 2, None, "Approve the Site settings change in #3832", &change).as_deref(), Some("#3832"));
        for text in ["Restart Fleet so #3832 is in your build", "Try page notes on #3834", "Look at armada/session-10", "Try it"] {
            assert_eq!(look_ask_of(false, 0, None, text, &change), None, "{text}");
        }
    }

    #[test]
    fn a_decision_with_options_is_never_a_look_ask() {
        assert_eq!(look_ask_of(false, 2, Some(&act(WaitingActKind::ApprovePr, "#3832")), "Try #3832: which layout?", &change()), None);
    }
}
