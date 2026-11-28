//! What the owner settled in a Job, filed for the feature pages. `docs/concepts/feature-catalog.md`, *Decisions*.
//!
//! **Plain Fleet code and no model.** Each place Fleet records one of his answers calls [`Fleet::owner_decision_kept`] with
//! what he was asked and what he chose. The features it belongs to are read when a page is served: the ones an artifact
//! names, otherwise the ones the Job is linked to (`store::feature_decisions`).

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{PartContent, PartState, SKETCH_IN_PLAN, SKETCH_SETTLED, SKETCH_SETTLED_AT, SKETCH_WORDS};
use store::{KeptFeatureDecision, Store};

use crate::daemon::Fleet;
use crate::session_host::artifact_parts::stored;

/// What he chose on a sketch, a page or a part he approved.
pub(crate) const APPROVED: &str = "Approved";
/// What he chose when he sent one back.
pub(crate) const ASKED_FOR_CHANGES: &str = "Asked for changes";
/// What he chose on a Ready to try card.
pub(crate) const LOOKS_GOOD: &str = "Looks good";
/// His Skip on a Ready to try card: the look was approved on the walk, and the change lands untried.
pub(crate) const SKIPPED: &str = "Skipped the preview";

/// One answer of his, before it is kept.
pub(crate) struct Decided<'a> {
    pub job: &'a str,
    /// Names the act, so hearing it again keeps one row.
    pub key: String,
    /// The target of the artifact he answered, empty for a question or a card.
    pub artifact: &'a str,
    pub question: &'a str,
    pub choice: &'a str,
    pub note: Option<&'a str>,
    pub at: &'a str,
}

impl Decided<'_> {
    fn kept(&self) -> Option<KeptFeatureDecision> {
        let (question, choice) = (self.question.trim(), self.choice.trim());
        if self.job.is_empty() || question.is_empty() || choice.is_empty() {
            return None;
        }
        Some(KeptFeatureDecision {
            job: self.job.to_string(),
            key: self.key.clone(),
            artifact: self.artifact.to_string(),
            question: question.to_string(),
            choice: choice.to_string(),
            note: self.note.map(str::trim).unwrap_or_default().to_string(),
            at: self.at.to_string(),
        })
    }
}

/// How he settled an artifact, as its detail says: a sketch or a page he approved or sent back, and each part of it he
/// answered. **One decision a settling**, keyed by when it was settled, so the same detail reported again keeps the
/// same rows, and a redrawn artifact he answers again is a new one.
pub(crate) fn artifact_decisions(job: &str, target: &str, detail: &BTreeMap<String, String>, now: &str) -> Vec<KeptFeatureDecision> {
    let title = detail.get("title").map(String::as_str).unwrap_or(target);
    let swept = detail.contains_key(SKETCH_IN_PLAN);
    let mut found = Vec::new();
    if let (Some(how), false) = (detail.get(SKETCH_SETTLED), swept) {
        let at = detail.get(SKETCH_SETTLED_AT).map(String::as_str).unwrap_or(now);
        let choice = if how == "approved" { APPROVED } else { ASKED_FOR_CHANGES };
        found.extend(Decided { job, key: format!("artifact:{target}:{at}"), artifact: target, question: title, choice, note: detail.get(SKETCH_WORDS).map(String::as_str), at }.kept());
    }
    for part in stored(detail) {
        let Some(answer) = part.answer.as_ref() else { continue };
        let words = answer.words.as_deref();
        let (question, choice) = match (&part.content, part.state) {
            (PartContent::Takes { question, takes, .. }, PartState::Picked) => {
                let Some(take) = answer.take.as_deref() else { continue };
                let named = takes.iter().find(|one| one.id == take).map(|one| one.title.as_str()).unwrap_or(take);
                (question.clone().unwrap_or_else(|| format!("{title}: {}", part.id)), named.to_string())
            }
            (PartContent::Takes { .. }, _) => continue,
            // The plan's Approve settles the words it held without his word on them.
            (_, PartState::Approved) if swept && words.is_none() => continue,
            (_, PartState::Approved) => (format!("{title}: {}", part.id), APPROVED.to_string()),
            (_, PartState::SentBack) => (format!("{title}: {}", part.id), ASKED_FOR_CHANGES.to_string()),
            _ => continue,
        };
        let at = answer.at.as_ref().map(ipc::Instant::as_str).unwrap_or(now);
        found.extend(Decided { job, key: format!("part:{target}#{}:{}", part.id, part.drawn), artifact: target, question: &question, choice: &choice, note: words, at }.kept());
    }
    found
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
    /// Keep one answer of his. **A failure to keep it is not a failure to answer**, so nothing is returned.
    pub(crate) fn owner_decision_kept(&self, store: &mut Store, decided: Decided<'_>) {
        if let Some(kept) = decided.kept() {
            let _ = store.keep_feature_decision(&kept);
        }
    }

    /// Every answer an artifact's detail holds, kept. Called wherever an artifact is reported, so each place that settles
    /// a sketch, a page or a part is heard once.
    pub(crate) fn artifact_decided(&self, store: &mut Store, job: &str, target: &str, detail: &BTreeMap<String, String>, now: &str) {
        for kept in artifact_decisions(job, target, detail, now) {
            let _ = store.keep_feature_decision(&kept);
        }
    }

    /// His Approve of a plan, or the changes he asked for on it. A plan is approved once, and each set of changes he sends
    /// is its own decision.
    pub(crate) async fn plan_decided(&self, job: &str, plan: &store::KeptPlan, choice: &str, note: Option<&str>) {
        let line = plan.summary.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
        let question = format!("Plan v{}: {line}", plan.version);
        let at = self.now().as_str().to_string();
        let key = format!("plan:{}:{choice}:{at}", plan.version);
        self.owner_decision_filed(Decided { job, key, artifact: "", question: &question, choice, note: note.filter(|note| !note.is_empty()), at: &at }).await;
    }

    /// What he chose on each question of an `AskUserQuestion` card. His note on the card goes with the first.
    pub(crate) async fn questions_decided(&self, job: &str, said: &ipc::AnswerSessionAsk) {
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        for (at_question, answer) in said.answers.iter().enumerate() {
            let key = format!("ask:{}:{at}:{}", said.call, answer.question);
            let choice = answer.chosen.join(", ");
            let note = said.note.as_deref().filter(|_| at_question == 0);
            self.owner_decision_kept(&mut store, Decided { job, key, artifact: "", question: &answer.question, choice: &choice, note, at: &at });
        }
    }

    /// How he settled a draft: approved, approved with his fixes, or sent back. A withdrawn one is the Job's own and is no decision.
    pub(crate) async fn draft_decided(&self, job: &str, row: &str, title: &str, how: ipc::DraftHow, note: Option<&str>) {
        let choice = match how {
            ipc::DraftHow::Approved => APPROVED,
            ipc::DraftHow::Fixed => "Approved with fixes",
            ipc::DraftHow::SentBack => ASKED_FOR_CHANGES,
            _ => return,
        };
        // His fixed text is the Job's to use, not a note on the question.
        let note = note.filter(|_| how != ipc::DraftHow::Fixed);
        let artifact = format!("{}{title}", crate::session_host::drafting::DRAFT_PREFIX);
        let at = self.now().as_str().to_string();
        self.owner_decision_filed(Decided { job, key: format!("draft:{row}"), artifact: &artifact, question: title, choice, note, at: &at }).await;
    }

    /// His Looks good on a Ready to try card. **One a branch at a tip**, whichever of the card's press or the build note's
    /// mark heard it.
    pub(crate) async fn looked_good(&self, job: &str, branch: &str, tip: &str, about: &str, at: &str, skipped: bool) {
        let key = format!("ready:{branch}@{tip}");
        let choice = if skipped { SKIPPED } else { LOOKS_GOOD };
        self.owner_decision_filed(Decided { job, key, artifact: "", question: about, choice, note: None, at }).await;
    }

    /// [`Self::owner_decision_kept`] for a caller that does not hold the store.
    pub(crate) async fn owner_decision_filed(&self, decided: Decided<'_>) {
        let mut store = self.store().lock().await;
        self.owner_decision_kept(&mut store, decided);
    }
}
