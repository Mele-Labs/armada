//! Whether a Session's wait is about a pull request, so a merge or an auto-merge settles it. **By
//! rules, no model**, as `session_host/asks.rs` reads a reply: the item's own act names the pull
//! request, or its words do, by `#number`, "pull request N", "PR N", its link, its branch or its title.
//!
//! Only the agent's own items and the asks Fleet reads off a reply. A question card, a permission, a
//! walk or a judge's concern is answered by its own press, never by a merge.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{WaitingActKind, WaitingItem, WaitingSource};

use crate::daemon::Fleet;

/// A pull request as a wait might name it.
pub(crate) struct PullNamed<'a> {
    pub(crate) number: u64,
    pub(crate) branch: &'a str,
    pub(crate) title: Option<&'a str>,
}

/// A title shorter than this is too plain to tell one pull request's wait from another's.
const TITLE_LEAST: usize = 12;

/// Whether `item` waits on the person for `pull`.
pub(crate) fn waits_on(item: &WaitingItem, pull: &PullNamed<'_>) -> bool {
    if !matches!(item.source, WaitingSource::Agent | WaitingSource::Replied) {
        return false;
    }
    if let Some(act) = &item.act {
        if act.kind == WaitingActKind::ApprovePr {
            return number_of(&act.target) == Some(pull.number);
        }
    }
    names(&item.text, pull)
}

/// The words name the pull request: its number by `#N`, "pull request N", "PR N" or a link ending
/// `/pull/N`, never followed by a digit; its branch; or its title whole.
fn names(text: &str, pull: &PullNamed<'_>) -> bool {
    let number = format!("#{}", pull.number);
    let by_hash = text.match_indices(&number).any(|(at, _)| {
        !text[at + number.len()..].starts_with(|c: char| c.is_ascii_digit())
    });
    let by_number = by_hash || spelled(text, pull.number);
    let by_branch = !pull.branch.is_empty() && text.contains(pull.branch);
    let by_title = pull.title.is_some_and(|title| {
        let title = title.trim();
        title.len() >= TITLE_LEAST && text.to_lowercase().contains(&title.to_lowercase())
    });
    by_number || by_branch || by_title
}

/// "pull request N", "PR N" (as a word, not the end of "SPR") or a link ending `/pull/N`, whatever the
/// case, with N the whole run of digits.
fn spelled(text: &str, number: u64) -> bool {
    let text = text.to_lowercase();
    let number = number.to_string();
    [("pull request ", true), ("pr ", true), ("/pull/", false)].into_iter().any(|(lead, word)| {
        text.match_indices(lead).any(|(at, _)| {
            let after = &text[at + lead.len()..];
            let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let alone = !word || !text[..at].ends_with(|c: char| c.is_alphanumeric());
            alone && after[..digits] == number
        })
    })
}

/// `#2589`, `2589` or a pull request's address, as an `approve_pr` act's target is written.
fn number_of(target: &str) -> Option<u64> {
    let tail = target.trim().trim_end_matches('/').rsplit(['/', '#']).next()?;
    tail.parse().ok()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Take off a Session's own items, and the asks read off its reply, waiting on `pull`, as [`Self::run_item_settled`] does for a
    /// command: the notice Fleet tells next says why, in Fleet's voice, and the turn it starts clears
    /// an ask read off a reply. The texts of the items taken off.
    pub(crate) async fn waits_taken_off(&self, session: &str, pull: &PullNamed<'_>) -> Vec<String> {
        let record = {
            let mut store = self.store().lock().await;
            let Ok(Some(kept)) = store.session(session) else { return Vec::new() };
            let Ok(reads) = self.ledger_reads(&store, &kept) else { return Vec::new() };
            let taken: Vec<(String, String)> = self
                .ledger_waiting(&kept, reads)
                .into_iter()
                .filter(|item| waits_on(item, pull))
                .map(|item| (item.id, item.text))
                .collect();
            if taken.is_empty() || taken.iter().any(|(id, _)| store.dismiss_waiting(session, id).is_err()) {
                return Vec::new();
            }
            let Ok(record) = self.ledger_row(&store, &kept) else { return Vec::new() };
            (record, taken)
        };
        let (row, taken) = record;
        self.publish(ipc::Event::SessionChanged(row));
        taken.into_iter().map(|(_, text)| text).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{Instant, WaitingAct};

    fn item(source: WaitingSource, text: &str, act: Option<(WaitingActKind, &str)>) -> WaitingItem {
        WaitingItem {
            id: "one".to_string(),
            text: text.to_string(),
            since: Instant::carried("2026-10-19T00:00:00Z"),
            source,
            act: act.map(|(kind, target)| WaitingAct { kind, target: target.to_string() }),
            options: Vec::new(),
            steps: Vec::new(),
            place: None,
            concern: None,
            cites: Vec::new(),
            confirmed: false,
        }
    }

    const PULL: PullNamed<'static> = PullNamed {
        number: 2589,
        branch: "armada/session-a6f77301",
        title: Some("Ask the forge for fewer open pull requests"),
    };

    #[test]
    fn an_approve_pr_act_names_the_pull_request_by_its_target_alone() {
        for target in ["#2589", "2589", "https://github.com/o/r/pull/2589"] {
            assert!(waits_on(&item(WaitingSource::Agent, "Review it", Some((WaitingActKind::ApprovePr, target))), &PULL), "{target}");
        }
        assert!(!waits_on(&item(WaitingSource::Agent, "Merge #2589", Some((WaitingActKind::ApprovePr, "#2590"))), &PULL));
    }

    #[test]
    fn words_name_it_by_number_branch_or_title() {
        for text in [
            "Merge #2589 when ci is green",
            "Merge armada/session-a6f77301 once green",
            "Approve 'ask the forge for fewer open pull requests'",
        ] {
            assert!(waits_on(&item(WaitingSource::Agent, text, None), &PULL), "{text}");
        }
        assert!(waits_on(&item(WaitingSource::Replied, "Can you merge #2589?", None), &PULL));
    }

    #[test]
    fn another_number_that_starts_the_same_is_not_it() {
        assert!(!waits_on(&item(WaitingSource::Agent, "Merge #25890 and #258", None), &PULL));
        assert!(!waits_on(&item(WaitingSource::Agent, "Restart Fleet", Some((WaitingActKind::Run, "scripts/restart"))), &PULL));
    }

    #[test]
    fn words_name_it_as_pull_request_n_pr_n_or_its_link() {
        let live = PullNamed { number: 2825, branch: "armada/session-865cbe01", title: None };
        for text in [
            "Pull request 2825 is waiting on ci and the merge queue, with auto-merge on",
            "Fleet tells me when https://github.com/Mele-Labs/armada/pull/2825 merges, and then I release slot-3",
            "pr 2825 is queued",
            "Waiting on PR 2825.",
        ] {
            assert!(waits_on(&item(WaitingSource::Agent, text, None), &live), "{text}");
            assert!(waits_on(&item(WaitingSource::Replied, text, None), &live), "{text}");
        }
    }

    #[test]
    fn another_number_or_word_that_looks_like_pull_request_n_is_not_it() {
        let live = PullNamed { number: 2825, branch: "armada/session-865cbe01", title: None };
        for text in [
            "Pull request 28250 is waiting on ci",
            "Pull request 282 is waiting on ci",
            "See https://github.com/Mele-Labs/armada/pull/28250",
            "SPR 2825 is not a pull request",
            "Pull request 2826 and PR 7",
        ] {
            assert!(!waits_on(&item(WaitingSource::Agent, text, None), &live), "{text}");
        }
    }

    #[test]
    fn a_question_card_or_a_judge_is_never_settled_by_a_merge() {
        for source in [WaitingSource::AskCard, WaitingSource::Permission, WaitingSource::Walk, WaitingSource::Judge] {
            assert!(!waits_on(&item(source, "Merge #2589?", None), &PULL));
        }
    }

    #[test]
    fn a_short_title_is_too_plain_to_match() {
        let plain = PullNamed { title: Some("Fix tests"), ..PULL };
        assert!(!waits_on(&item(WaitingSource::Agent, "Fix tests in the cockpit", None), &plain));
    }
}
