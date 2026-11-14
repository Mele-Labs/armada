//! A page's votes: takes the agent declares with `<armada-vote>` and `<armada-take>`, and `cast_page_vote`, his
//! vote on one. **Read off the kept revision he voted on, never off a call**, and sent to the Job with every note.
//! `crates/ipc/operations/cast_page_vote.toml` is the contract, `docs/concepts/artifact.md`, *Page votes*, the rules.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{CastPageVote, Instant, VoteCast, VoteForm, VoteResult};
use store::KeptVote;

use super::document_page::{votes_in, PageTake, PageVote};
use super::documents::revisions_named;
use super::page_actions::PAGE_ARCHIVED;
use crate::daemon::Fleet;

const PAGE_VOTE_UNKNOWN: &str = "fleet.page_vote_unknown";
const PAGE_VOTE_FORM: &str = "fleet.page_vote_form";
const PAGE_VOTE_TAKES: &str = "fleet.page_vote_takes";
/// The most vote ids a refusal names.
const MOST_NAMED: usize = 20;

/// The one line that tells a Job which votes of its page will not count, or `None` where all will. The page is
/// never refused.
pub(crate) fn votes_that_will_not_count(html: &str, path: &str) -> Option<String> {
    let mut seen = BTreeSet::new();
    let failing: Vec<String> = votes_in(html)
        .iter()
        .enumerate()
        .filter_map(|(at, vote)| {
            let why = if vote.id.is_empty() {
                "it has no id".to_string()
            } else if !seen.insert(vote.id.clone()) {
                format!("its id \"{}\" is an earlier vote's too", vote.id)
            } else if VoteForm::named(&vote.form).is_none() {
                format!("form \"{}\" is not one a vote takes. A vote is pick, rank or score", vote.form)
            } else if vote.takes.len() < 2 {
                format!("it holds {} take, and a vote needs two or more", vote.takes.len())
            } else if let Some(why) = take_fault(&vote.takes) {
                why
            } else if VoteForm::named(&vote.form) == Some(VoteForm::Score) && vote.parts.is_empty() {
                "a score vote names the parts it marks in `parts`, and it names none".to_string()
            } else {
                return None;
            };
            Some(format!("In the page {path}, vote \"{}\" will not count: {why}.", named(vote, at)))
        })
        .collect();
    (!failing.is_empty()).then(|| failing.join(" "))
}

/// What is wrong with a vote's takes, where something is.
fn take_fault(takes: &[PageTake]) -> Option<String> {
    let mut seen = BTreeSet::new();
    takes.iter().enumerate().find_map(|(at, take)| match take.id.is_empty() {
        true => Some(format!("take {} has no id", at + 1)),
        false => (!seen.insert(take.id.as_str())).then(|| format!("take id \"{}\" is an earlier take's too", take.id)),
    })
}

/// A vote as a notice and a message name it: its question, else its id, else its place on the page.
fn named(vote: &PageVote, at: usize) -> String {
    [&vote.question, &vote.id].into_iter().find(|one| !one.is_empty()).cloned().unwrap_or_else(|| format!("number {}", at + 1))
}

/// The vote's takes, the first with each id, in the page's order.
fn takes_of(vote: &PageVote) -> Vec<&PageTake> {
    let mut seen = BTreeSet::new();
    vote.takes.iter().filter(|take| !take.id.is_empty() && seen.insert(take.id.as_str())).collect()
}

/// A take as the message names it: its title, else its id.
fn title_of<'a>(takes: &[&'a PageTake], id: &'a str) -> &'a str {
    match takes.iter().copied().find(|take| take.id == id) {
        Some(take) if !take.title.is_empty() => &take.title,
        Some(take) => &take.id,
        None => id,
    }
}

/// `result` with its notes trimmed and the empty ones left out.
fn trimmed(result: VoteResult) -> VoteResult {
    let clean = |notes: BTreeMap<String, String>| -> BTreeMap<String, String> {
        notes.into_iter().map(|(take, words)| (take.trim().to_string(), words.trim().to_string())).filter(|(_, words)| !words.is_empty()).collect()
    };
    match result {
        VoteResult::Pick { take, notes } => VoteResult::Pick { take: take.trim().to_string(), notes: clean(notes) },
        VoteResult::Rank { order, notes } => VoteResult::Rank { order: order.iter().map(|take| take.trim().to_string()).collect(), notes: clean(notes) },
        VoteResult::Score { marks, notes } => VoteResult::Score {
            marks: marks
                .into_iter()
                .map(|mark| ipc::VoteMark { take: mark.take.trim().to_string(), part: mark.part.trim().to_string(), keep: mark.keep })
                .collect(),
            notes: clean(notes),
        },
    }
}

/// Why `result` does not fit the vote's takes and parts, where it does not.
fn takes_refused(vote: &PageVote, result: &VoteResult) -> Option<String> {
    let takes = takes_of(vote);
    let known = |id: &str| takes.iter().any(|take| take.id == id);
    let holds = || takes.iter().map(|take| take.id.as_str()).collect::<Vec<_>>().join(", ");
    let unknown = |id: &str| Some(format!("vote `{}` holds no take `{id}`. Takes it holds: {}", vote.id, holds()));
    if let Some(take) = result.notes().keys().find(|take| !known(take)) {
        return unknown(take);
    }
    match result {
        VoteResult::Pick { take, .. } => (!known(take)).then(|| unknown(take)).flatten(),
        VoteResult::Rank { order, .. } => {
            if let Some(take) = order.iter().find(|take| !known(take)) {
                return unknown(take);
            }
            let mut seen = BTreeSet::new();
            if let Some(take) = order.iter().find(|take| !seen.insert(take.as_str())) {
                return Some(format!("the order names take `{take}` twice, and names each take once"));
            }
            let skipped: Vec<&str> = takes.iter().map(|take| take.id.as_str()).filter(|id| !seen.contains(id)).collect();
            (!skipped.is_empty()).then(|| format!("the order skips {}, and names each take once", skipped.join(", ")))
        }
        VoteResult::Score { marks, .. } => {
            if let Some(mark) = marks.iter().find(|mark| !known(&mark.take)) {
                return unknown(&mark.take);
            }
            let part = marks.iter().find(|mark| !vote.parts.contains(&mark.part))?;
            Some(format!("vote `{}` names no part `{}`. Parts it names: {}", vote.id, part.part, vote.parts.join(", ")))
        }
    }
}

/// What the Job is sent for his vote, with every note.
fn voted_told(vote: &PageVote, title: &str, result: &VoteResult) -> String {
    let takes = takes_of(vote);
    let name = |id: &str| title_of(&takes, id).to_string();
    let notes = result.notes();
    let note_lines = || takes.iter().filter_map(|take| notes.get(&take.id).map(|words| format!("{}: {words}", name(&take.id)))).collect::<Vec<_>>();
    let mut lines = vec![format!("He voted on \"{}\" in \"{title}\".", named(vote, 0))];
    match result {
        VoteResult::Pick { take, .. } => {
            lines.push(format!("Picked: {}.", name(take)));
            lines.extend(note_lines());
        }
        VoteResult::Rank { order, .. } => lines.extend(order.iter().enumerate().map(|(at, take)| match notes.get(take) {
            Some(words) => format!("{}. {}: {words}", at + 1, name(take)),
            None => format!("{}. {}", at + 1, name(take)),
        })),
        VoteResult::Score { marks, .. } => {
            for take in &takes {
                // The last mark on a part is the one that counts.
                let marked = |part: &str| marks.iter().rev().find(|mark| mark.take == take.id && mark.part == part).map(|mark| mark.keep);
                let with = |keep: bool| vote.parts.iter().filter(|part| marked(part.as_str()) == Some(keep)).cloned().collect::<Vec<_>>().join(", ");
                let halves: Vec<String> = [("keeps", with(true)), ("drops", with(false))]
                    .into_iter()
                    .filter(|(_, parts)| !parts.is_empty())
                    .map(|(word, parts)| format!("{word} {parts}"))
                    .collect();
                if !halves.is_empty() {
                    lines.push(format!("{}: {}", name(&take.id), halves.join("; ")));
                }
            }
            lines.extend(note_lines());
        }
    }
    lines.join("\n")
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
    /// `cast_page_vote`: the vote read off the kept revision, checked, sent to the Job, then kept.
    pub(crate) async fn page_vote_cast(self: Arc<Self>, cast: CastPageVote) -> Result<VoteCast, Refusal> {
        let id = cast.session_id.as_str().trim().to_string();
        let artifact = self.artifact_held(&id, &cast.artifact).await?;
        if artifact.form != ipc::ArtifactForm::Page {
            return Err(self.hosted_refusal(PAGE_VOTE_UNKNOWN, &format!("`{}` is not a page, and only a page declares votes", artifact.title)));
        }
        if artifact.archived.is_some() {
            return Err(self.hosted_refusal(PAGE_ARCHIVED, &format!("`{}` is archived. Restore it to vote on it", artifact.title)));
        }
        let vote = self.vote_kept(&id, &artifact.id, &artifact.title, cast.revision, &cast.vote).await?;
        let result = trimmed(cast.result);
        let form = result.form();
        if VoteForm::named(&vote.form) != Some(form) {
            let declared = match vote.form.is_empty() {
                true => "no form",
                false => vote.form.as_str(),
            };
            let why = format!("vote `{}` is declared {declared}, and this is a {} result", vote.id, form.word());
            return Err(self.hosted_refusal(PAGE_VOTE_FORM, &why));
        }
        if let Some(why) = takes_refused(&vote, &result) {
            return Err(self.hosted_refusal(PAGE_VOTE_TAKES, &why));
        }
        // Strings and a bool, which always encode.
        let encoded = ipc::encode(&result).unwrap_or_default();
        // Sent first, so a send that fails keeps nothing.
        self.told_as_him(&id, voted_told(&vote, &artifact.title, &result)).await?;
        let at = self.instant();
        let kept = KeptVote {
            artifact: artifact.id.clone(),
            revision: cast.revision,
            vote: vote.id.clone(),
            form: form.word().to_string(),
            result: encoded,
            at: at.as_str().to_string(),
        };
        self.store().lock().await.keep_page_vote(&id, &kept).map_err(|why| self.ledger_fault(why))?;
        self.published_hosted(&id).await?;
        Ok(VoteCast { revision: cast.revision, vote: vote.id, form, result, at })
    }

    /// The first vote with id `named` in revision `revision` of the page at `path`, as Fleet kept it.
    async fn vote_kept(&self, id: &str, path: &str, title: &str, revision: u32, named: &str) -> Result<PageVote, Refusal> {
        let (_, held) = self.document_held(id, path).await?;
        let file = held.map(|page| revisions_named(&page.detail)).and_then(|kept| kept.get((revision as usize).checked_sub(1)?).cloned());
        let Some(file) = file else {
            return Err(self.hosted_refusal(PAGE_VOTE_UNKNOWN, &format!("revision {revision} of `{title}` is not kept")));
        };
        let html = tokio::task::spawn_blocking(move || std::fs::read_to_string(file)).await.ok().and_then(Result::ok).unwrap_or_default();
        let votes = votes_in(&html);
        let named = named.trim();
        if let Some(vote) = votes.iter().find(|one| !named.is_empty() && one.id == named) {
            return Ok(vote.clone());
        }
        let ids: Vec<&str> = votes.iter().map(|one| one.id.as_str()).filter(|one| !one.is_empty()).take(MOST_NAMED).collect();
        let holds = match ids.is_empty() {
            true => String::from("none"),
            false => ids.join(", "),
        };
        Err(self.hosted_refusal(PAGE_VOTE_UNKNOWN, &format!("revision {revision} of `{title}` holds no vote `{named}`. Votes it holds: {holds}")))
    }
}

/// The newest cast of each vote kept for a page, in the order each vote was first cast. One whose result no
/// longer reads is left out.
pub(crate) fn votes_of(kept: &[KeptVote], artifact: &str) -> Vec<VoteCast> {
    let mut votes: Vec<VoteCast> = Vec::new();
    for one in kept.iter().filter(|one| one.artifact == artifact) {
        let Ok(result) = ipc::decode::<VoteResult>("a kept vote", one.result.as_bytes()) else { continue };
        let cast = VoteCast { revision: one.revision, vote: one.vote.clone(), form: result.form(), result, at: Instant::carried(one.at.clone()) };
        match votes.iter_mut().find(|earlier| earlier.vote == one.vote) {
            Some(earlier) => *earlier = cast,
            None => votes.push(cast),
        }
    }
    votes
}
