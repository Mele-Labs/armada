//! Whether an agent's waiting item is a wait on a merge, which is not a Needs you item: Fleet follows the
//! merge and tells the Job when it lands, and the person has nothing to do for it.
//!
//! **A merge wait is one of this Job's pull requests that is queued, has auto-merge on or has merged**,
//! named by the item's `approve_pr` act, or in its text by number (`#12`, `PR 12`, `pull request 12`,
//! `/pull/12`) or by link. `waiting_for` refuses it when the agent writes it, so the agent rewords; one
//! already kept is never taken off.

use ipc::WaitingAct;
use ipc::WaitingActKind;

use crate::pull_requesting::pull_merged;

/// A pull request of the Job that will land or has landed, as the ledger's `pr` row says.
pub struct Merging {
    pub number: u64,
    /// The forge's address for it, empty where the row has none.
    pub address: String,
}

/// The Job's pull requests that are queued, have auto-merge on or have merged, from its `pr` rows.
pub fn merging_of(rows: &[store::KeptAttachment]) -> Vec<Merging> {
    rows.iter()
        .filter(|one| one.kind == crate::pull_requesting::PR)
        .filter_map(|one| {
            let number = one.target.trim().parse().ok()?;
            let on = |key: &str| one.detail.get(key).map(String::as_str) == Some("true");
            let spent = one.state == store::AttachmentState::Spent;
            (on("queued") || on("auto_merge") || pull_merged(&one.detail, spent))
                .then(|| Merging { number, address: one.detail.get("address").cloned().unwrap_or_default() })
        })
        .collect()
}

/// The pull request an item with this `act` and `text` waits the merge of, if it does.
pub fn merge_wait_of(act: Option<&WaitingAct>, text: &str, pulls: &[Merging]) -> Option<u64> {
    pulls
        .iter()
        .find(|pull| {
            let by_act = act.is_some_and(|act| act.kind == WaitingActKind::ApprovePr && crate::pull_waits::number_of(&act.target) == Some(pull.number));
            by_act || names(text, pull.number) || (!pull.address.is_empty() && text.contains(&pull.address))
        })
        .map(|pull| pull.number)
}

/// Whether `text` names pull request `number`: `#12`, `/pull/12`, or `PR 12` and `pull request 12` in any case.
fn names(text: &str, number: u64) -> bool {
    let lower = text.to_lowercase();
    let mut digits = lower.char_indices().filter(|(_, c)| c.is_ascii_digit()).peekable();
    while let Some((start, _)) = digits.next() {
        let mut end = start + 1;
        while let Some((at, _)) = digits.next_if(|(at, _)| *at == end) {
            end = at + 1;
        }
        if lower[start..end].parse::<u64>() != Ok(number) {
            continue;
        }
        let before = &lower[..start];
        let word = before.trim_end();
        let ends_in = |what: &str| {
            word.strip_suffix(what).is_some_and(|rest| !rest.ends_with(|c: char| c.is_alphanumeric()))
        };
        if before.ends_with('#') || before.ends_with("/pull/") || ends_in("pr") || ends_in("pull request") || ends_in("pull") {
            return true;
        }
    }
    false
}

/// What the agent reads when its list is refused for an item that is a merge wait.
pub fn merge_item_refusal(id: &str, number: u64) -> String {
    format!(
        "Refused: item \"{id}\" waits on the merge of #{number}, and a merge wait is not a Needs you item. \
         The pull request is queued, has auto-merge on or has merged, and Fleet tells you when it merges. \
         Call waiting_for again without the item, or reword it as what he must do himself."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(number: u64, address: &str) -> Merging {
        Merging { number, address: address.to_string() }
    }

    fn act(kind: WaitingActKind, target: &str) -> WaitingAct {
        WaitingAct { kind, target: target.to_string() }
    }

    #[test]
    fn an_approve_pr_act_naming_the_pull_request_is_a_merge_wait() {
        let pulls = [pull(12, "")];
        assert_eq!(merge_wait_of(Some(&act(WaitingActKind::ApprovePr, "#12")), "Review it", &pulls), Some(12));
        assert_eq!(merge_wait_of(Some(&act(WaitingActKind::ApprovePr, "https://forge.invalid/a/pull/12")), "Review it", &pulls), Some(12));
        assert_eq!(merge_wait_of(Some(&act(WaitingActKind::ApprovePr, "#13")), "Review it", &pulls), None);
        assert_eq!(merge_wait_of(Some(&act(WaitingActKind::Run, "#12")), "Run it", &pulls), None);
    }

    #[test]
    fn a_text_naming_the_number_or_the_link_is_a_merge_wait_and_other_numbers_are_not() {
        let pulls = [pull(12, "https://forge.invalid/a/pull/12")];
        for text in ["Merge #12 when ci is green", "land PR 12", "Pull request 12 is queued", "see https://forge.invalid/a/pull/12", "PR #12 waits"] {
            assert_eq!(merge_wait_of(None, text, &pulls), Some(12), "{text}");
        }
        for text in ["Merge #123", "Restart Fleet onto build 12", "Pick one of 12 names", "Review #112", "upr 12"] {
            assert_eq!(merge_wait_of(None, text, &pulls), None, "{text}");
        }
    }

    #[test]
    fn only_a_queued_auto_merging_or_merged_pull_request_counts() {
        let row = |target: &str, state: store::AttachmentState, detail: &[(&str, &str)]| store::KeptAttachment {
            holder: store::Holder::session("a"),
            kind: "pr".into(),
            manifest_id: "m".into(),
            target: target.into(),
            state,
            detail: detail.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            since: "2026-11-05T00:00:00Z".into(),
            changed_at: "2026-11-05T00:00:00Z".into(),
        };
        let rows = [
            row("1", store::AttachmentState::Standing, &[("state", "open"), ("queued", "true")]),
            row("2", store::AttachmentState::Standing, &[("state", "open"), ("auto_merge", "true")]),
            row("3", store::AttachmentState::Spent, &[("state", "merged")]),
            row("4", store::AttachmentState::Standing, &[("state", "open"), ("auto_merge", "false"), ("queued", "false")]),
            row("5", store::AttachmentState::Spent, &[("state", "closed")]),
        ];
        let numbers: Vec<u64> = merging_of(&rows).into_iter().map(|one| one.number).collect();
        assert_eq!(numbers, [1, 2, 3]);
    }
}
